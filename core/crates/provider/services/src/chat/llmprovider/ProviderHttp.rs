//! Provider request encoding and asynchronous responses over the existing HTTP Host.
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use http::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use http::StatusCode;
use operit_host_api::HostManager::defaultHttpHost;
use operit_host_api::{HttpHost, HttpRequestData, HttpResponseHead};
use serde::{de::DeserializeOwned, Serialize};

use super::AIService::AiServiceError;
use super::ApiKeyProvider::{credential_id, ApiKeyOutcome, KeyGetter};
use std::collections::HashSet;

/// Defines the protocol location of the credential without inspecting endpoint strings.
#[derive(Clone)]
pub enum KeyAuthentication {
    Bearer,
    Header(String),
    Query(String),
}

impl KeyAuthentication {
    /// Treats an explicit authentication header as the request's fixed credential source.
    pub(super) fn getter(&self, request: &HttpRequestData, getter: KeyGetter) -> KeyGetter {
        let name = match self {
            Self::Bearer => "Authorization",
            Self::Header(name) => name,
            Self::Query(_) => return getter,
        };
        match request.headers.iter().find(|(existing, value)| {
            existing.eq_ignore_ascii_case(name) && !value.trim().is_empty()
        }) {
            Some((_, value)) => KeyGetter::fixed(value.clone()),
            None => getter,
        }
    }
}

/// Converts typed HTTP metadata into an actionable credential outcome.
pub fn key_outcome(status: i32, headers: &[(String, String)], now: i64) -> Option<ApiKeyOutcome> {
    match status {
        401 | 403 => Some(ApiKeyOutcome::Authentication),
        402 => Some(ApiKeyOutcome::Quota),
        429 => Some(ApiKeyOutcome::RateLimited {
            retry_after_ms: headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("retry-after"))
                .and_then(|(_, value)| {
                    value
                        .trim()
                        .parse::<i64>()
                        .ok()
                        .map(|seconds| seconds.saturating_mul(1000))
                        .or_else(|| {
                            chrono::DateTime::parse_from_rfc2822(value)
                                .ok()
                                .map(|date| date.timestamp_millis().saturating_sub(now))
                        })
                }),
        }),
        408 | 500..=599 => Some(ApiKeyOutcome::Transient),
        _ => None,
    }
}

/// Writes the selected credential into its protocol-specific authentication location.
pub fn apply_key(
    request: &mut HttpRequestData,
    auth: &KeyAuthentication,
    key: &str,
) -> Result<(), AiServiceError> {
    if key.is_empty() {
        return Ok(());
    }
    match auth {
        KeyAuthentication::Bearer | KeyAuthentication::Header(_) => {
            let (name, value) = match auth {
                KeyAuthentication::Bearer => ("Authorization", {
                    let mut parts = key.split_whitespace();
                    if matches!((parts.next(), parts.next()), (Some(scheme), Some(_)) if scheme.eq_ignore_ascii_case("Bearer"))
                    {
                        key.to_string()
                    } else {
                        format!("Bearer {key}")
                    }
                }),
                KeyAuthentication::Header(name) => (name.as_str(), key.to_string()),
                _ => unreachable!(),
            };
            if request.headers.iter().any(|(existing, value)| {
                existing.eq_ignore_ascii_case(name) && !value.trim().is_empty()
            }) {
                return Ok(());
            }
            HeaderValue::from_str(&value)
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            request
                .headers
                .retain(|(existing, _)| !existing.eq_ignore_ascii_case(name));
            request.headers.push((name.to_string(), value));
        }
        KeyAuthentication::Query(name) => {
            let mut url = url::Url::parse(&request.url)
                .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
            let pairs: Vec<_> = url
                .query_pairs()
                .filter(|(existing, _)| existing != name.as_str())
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect();
            url.set_query(None);
            url.query_pairs_mut()
                .extend_pairs(pairs)
                .append_pair(name, key);
            request.url = url.to_string();
        }
    }
    Ok(())
}

/// Carries one credential's feedback identity until its response body completes.
struct KeyAttempt {
    getter: KeyGetter,
    key: String,
    started_at: i64,
}

impl KeyAttempt {
    /// Persists a typed outcome without logging credential material.
    fn report(&self, outcome: ApiKeyOutcome) -> Result<(), AiServiceError> {
        self.getter
            .report(&self.key, self.started_at, outcome)
            .map_err(AiServiceError::RequestFailed)
    }
}

/// Applies the same affinity and status policy to synchronous model discovery.
pub(super) fn execute_with_key_getter(
    host: Arc<dyn HttpHost>,
    request: HttpRequestData,
    getter: KeyGetter,
    auth: KeyAuthentication,
) -> Result<operit_host_api::HttpResponseData, String> {
    let getter = auth.getter(&request, getter);
    let mut attempted = HashSet::new();
    let mut previous = None;
    loop {
        let key = match getter.acquire() {
            Ok(key) if attempted.insert(credential_id(&key)) => key,
            Ok(_) | Err(_) if previous.is_some() => return previous.unwrap(),
            Ok(_) => return Err("No untried API keys remain".into()),
            Err(error) => return Err(error),
        };
        let mut request = request.clone();
        apply_key(&mut request, &auth, &key).map_err(|error| error.to_string())?;
        let started = operit_host_api::TimeUtils::currentTimeMillis();
        match host.executeHttpRequest(request) {
            Ok(response) => {
                if let Some(outcome) = key_outcome(
                    response.statusCode,
                    &response.headers,
                    operit_host_api::TimeUtils::currentTimeMillis(),
                ) {
                    getter.report(&key, started, outcome)?;
                    previous = Some(Ok(response));
                    continue;
                }
                if (200..300).contains(&response.statusCode) {
                    getter.report(&key, started, ApiKeyOutcome::Success)?;
                }
                return Ok(response);
            }
            Err(error) => {
                getter.report(&key, started, ApiKeyOutcome::Transport)?;
                previous = Some(Err(error.to_string()));
            }
        }
    }
}

/// Encodes a POST without allocating or owning a network client.
pub(super) fn post(url: impl AsRef<str>) -> RequestBuilder {
    RequestBuilder::new("POST", url.as_ref())
}

/// Encodes a GET without allocating or owning a network client.
pub(super) fn get(url: impl AsRef<str>) -> RequestBuilder {
    RequestBuilder::new("GET", url.as_ref())
}

/// Holds protocol data only; sockets, TLS and connection pools belong to the Host.
pub(super) struct RequestBuilder {
    request: HttpRequestData,
    headers: HeaderMap,
    error: Option<AiServiceError>,
    credentials: Option<(KeyGetter, KeyAuthentication)>,
    gateway_binding: Option<String>,
}

impl RequestBuilder {
    /// Applies one shared model transport policy to every provider request.
    fn new(method: &str, url: &str) -> Self {
        Self {
            request: HttpRequestData {
                url: url.to_string(),
                method: method.to_string(),
                headers: Vec::new(),
                body: Vec::new(),
                formFields: Vec::new(),
                fileParts: Vec::new(),
                connectTimeoutSeconds: 60,
                readTimeoutSeconds: 1000,
                followRedirects: true,
                ignoreSsl: false,
                proxyHost: String::new(),
                proxyPort: 0,
            },
            headers: HeaderMap::new(),
            error: None,
            credentials: None,
            gateway_binding: None,
        }
    }

    /// Selects the device Binding that owns this local gateway.
    pub(super) fn gateway_binding(mut self, binding: Option<String>) -> Self {
        self.gateway_binding = binding;
        self
    }

    /// Applies validated provider headers without opening a transport.
    pub(super) fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Attaches explicit credential decisions to every actual HTTP attempt, including provider retries.
    pub(super) fn key_getter(mut self, getter: KeyGetter, auth: KeyAuthentication) -> Self {
        self.credentials = Some((getter, auth));
        self
    }

    /// Encodes one header and preserves validation errors until dispatch.
    pub(super) fn header<V>(mut self, name: HeaderName, value: V) -> Self
    where
        V: TryInto<HeaderValue>,
        V::Error: std::fmt::Display,
    {
        match value.try_into() {
            Ok(value) => {
                self.headers.insert(name, value);
            }
            Err(error) => self.error = Some(AiServiceError::RequestFailed(error.to_string())),
        }
        self
    }

    /// Serializes a JSON body and retains an explicitly configured content type.
    pub(super) fn json<T: Serialize + ?Sized>(mut self, value: &T) -> Self {
        match serde_json::to_vec(value) {
            Ok(bytes) => self.request.body = bytes,
            Err(error) => self.error = Some(AiServiceError::RequestFailed(error.to_string())),
        }
        self.headers
            .entry(CONTENT_TYPE)
            .or_insert(HeaderValue::from_static("application/json"));
        self
    }

    /// Encodes OAuth's URL-encoded form body through the same Host transport.
    pub(super) fn form(mut self, pairs: &[(&str, &str)]) -> Self {
        self.request.body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(pairs.iter().copied())
            .finish()
            .into_bytes();
        self.headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        self
    }

    /// Attaches a provider's already serialized payload.
    pub(super) fn body(mut self, bytes: Vec<u8>) -> Self {
        self.request.body = bytes;
        self
    }

    /// Awaits response metadata while a drop guard owns cancellation from the start.
    pub(super) async fn send(self) -> Result<Response, AiServiceError> {
        self.send_with_host(defaultHttpHost()).await
    }

    /// Dispatches through the selected Host, also allowing isolated transport verification.
    pub(super) async fn send_with_host(
        mut self,
        host: Arc<dyn HttpHost>,
    ) -> Result<Response, AiServiceError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.request.headers = self
            .headers
            .iter()
            .map(|(name, value)| {
                value
                    .to_str()
                    .map(|value| (name.to_string(), value.to_string()))
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
            })
            .collect::<Result<_, _>>()?;
        if let Some((getter, auth)) = self.credentials {
            let getter = auth.getter(&self.request, getter);
            let mut attempted = HashSet::new();
            let mut previous = None;
            loop {
                let key = match getter.acquire() {
                    Ok(key) if attempted.insert(credential_id(&key)) => key,
                    Ok(_) | Err(_) if previous.is_some() => return previous.unwrap(),
                    Ok(_) => {
                        return Err(AiServiceError::RequestFailed(
                            "No untried API keys remain".into(),
                        ))
                    }
                    Err(error) => return Err(AiServiceError::RequestFailed(error)),
                };
                let mut request = self.request.clone();
                apply_key(&mut request, &auth, &key)?;
                let attempt = KeyAttempt {
                    getter: getter.clone(),
                    key,
                    started_at: operit_host_api::TimeUtils::currentTimeMillis(),
                };
                match Self::open_response(host.clone(), request, self.gateway_binding.clone()).await {
                    Ok(mut response) => {
                        if let Some(outcome) = key_outcome(
                            response.head.statusCode,
                            &response.head.headers,
                            operit_host_api::TimeUtils::currentTimeMillis(),
                        ) {
                            attempt.report(outcome)?;
                            previous = Some(Ok(response));
                            continue;
                        }
                        if response.status.is_success() {
                            response.body.key_attempt = Some(attempt);
                        }
                        return Ok(response);
                    }
                    Err(error @ AiServiceError::ConnectionFailed(_)) => {
                        attempt.report(ApiKeyOutcome::Transport)?;
                        previous = Some(Err(error));
                    }
                    Err(error) => return Err(error),
                }
            }
        }
        Self::open_response(host, self.request, self.gateway_binding).await
    }

    /// Opens one request and preserves its response for the decision layer before body consumption.
    async fn open_response(
        host: Arc<dyn HttpHost>,
        request: HttpRequestData,
        gateway_binding: Option<String>,
    ) -> Result<Response, AiServiceError> {
        let mut body = match gateway_binding {
            Some(binding) => ResponseBody::open_gateway(binding, request)?,
            None => ResponseBody::open(host, request)?,
        };
        match body.events.recv().await {
            Some(Event::Head(head)) => {
                let status = StatusCode::from_u16(
                    u16::try_from(head.statusCode)
                        .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?,
                )
                .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
                Ok(Response { head, status, body })
            }
            Some(Event::Closed(Err(error))) => Err(AiServiceError::ConnectionFailed(error)),
            _ => Err(AiServiceError::ConnectionFailed(
                "HTTP Host did not deliver response headers".to_string(),
            )),
        }
    }
}

/// Carries metadata and a live, cancellable Host response body.
pub(super) struct Response {
    head: HttpResponseHead,
    status: StatusCode,
    body: ResponseBody,
}

impl Response {
    /// Returns the upstream HTTP status independently of its body.
    pub(super) fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the response media type without its parameters.
    pub(super) fn media_type(&self) -> &str {
        self.head
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
            .map(|(_, value)| value.split(';').next().unwrap_or_default().trim())
            .unwrap_or_default()
    }

    /// Transfers ownership of cancellation to the body consumer.
    pub(super) fn bytes_stream(self) -> ResponseBody {
        self.body
    }

    /// Reads a nonstreaming response without blocking a runtime worker.
    pub(super) async fn text(self) -> Result<String, AiServiceError> {
        let mut body = self.body;
        let mut bytes = Vec::new();
        while let Some(chunk) = body.next().await {
            bytes.extend_from_slice(&chunk?);
        }
        String::from_utf8(bytes).map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }

    /// Decodes one complete JSON response after the Host confirms a clean close.
    pub(super) async fn json<T: DeserializeOwned>(self) -> Result<T, AiServiceError> {
        serde_json::from_str(&self.text().await?)
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))
    }
}

pub(super) enum Event {
    Head(HttpResponseHead),
    Bytes(Vec<u8>),
    Closed(Result<(), String>),
}

/// Releases the Host request on completion, cancellation, parsing failure or consumer drop.
pub(super) struct ResponseBody {
    host: Option<Arc<dyn HttpHost>>,
    id: String,
    events: tokio::sync::mpsc::UnboundedReceiver<Event>,
    finished: bool,
    key_attempt: Option<KeyAttempt>,
}

impl ResponseBody {
    /// Owns the routed stream until completion or cancellation by the response consumer.
    fn open_gateway(binding: String, request: HttpRequestData) -> Result<Self, AiServiceError> {
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask("local-gateway-response", Box::new(move || Box::pin(async move {
                let result = super::LocalGatewayTransport::forward_response(binding, request, &sender).await;
                if let Err(error) = result {
                    let _ = sender.send(Event::Closed(Err(error)));
                }
            })))
            .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
        Ok(Self { host: None, id: String::new(), events, finished: false, key_attempt: None })
    }

    /// Binds ordered Host callbacks to one asynchronous response consumer.
    fn open(host: Arc<dyn HttpHost>, request: HttpRequestData) -> Result<Self, AiServiceError> {
        let id = format!("provider-http-{}", uuid::Uuid::new_v4());
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        let head_sender = sender.clone();
        let chunk_sender = sender.clone();
        let body = Self {
            host: Some(host.clone()),
            id,
            events,
            finished: false,
            key_attempt: None,
        };
        host
            .openHttpResponseStream(
                body.id.clone(),
                request,
                Arc::new(move |head| {
                    let _ = head_sender.send(Event::Head(head));
                }),
                Arc::new(move |bytes| {
                    let _ = chunk_sender.send(Event::Bytes(bytes));
                }),
                Arc::new(move |result| {
                    let _ = sender.send(Event::Closed(result));
                }),
            )
            .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
        Ok(body)
    }
}

impl Stream for ResponseBody {
    type Item = Result<Vec<u8>, AiServiceError>;

    /// Preserves Host ordering and rejects a missing or failed terminal callback.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }
        match self.events.poll_recv(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Event::Bytes(bytes))) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(Some(Event::Closed(Ok(())))) => {
                self.finished = true;
                if let Some(attempt) = self.key_attempt.take() {
                    if let Err(error) = attempt.report(ApiKeyOutcome::Success) {
                        return Poll::Ready(Some(Err(error)));
                    }
                }
                Poll::Ready(None)
            }
            Poll::Ready(event) => {
                self.finished = true;
                if let Some(attempt) = self.key_attempt.take() {
                    if let Err(error) = attempt.report(ApiKeyOutcome::Transport) {
                        return Poll::Ready(Some(Err(error)));
                    }
                }
                let error = match event {
                    Some(Event::Closed(Err(error))) => error,
                    Some(Event::Head(_)) => "HTTP Host sent duplicate response headers".to_string(),
                    _ => "HTTP Host response disconnected without completion".to_string(),
                };
                Poll::Ready(Some(Err(AiServiceError::ConnectionFailed(error))))
            }
        }
    }
}

impl Drop for ResponseBody {
    /// Cancels header and body reads through the same Host that opened them.
    fn drop(&mut self) {
        if let Some(host) = &self.host {
            let _ = host.closeHttpByteStream(&self.id);
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/ProviderHttpTests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../../tests/ApiKeyPoolTransportTests.rs"]
mod key_pool_tests;
