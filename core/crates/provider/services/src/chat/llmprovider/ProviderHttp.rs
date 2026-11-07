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
        }
    }

    /// Applies validated provider headers without opening a transport.
    pub(super) fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
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
        let mut body = ResponseBody::open(host, self.request)?;
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

enum Event {
    Head(HttpResponseHead),
    Bytes(Vec<u8>),
    Closed(Result<(), String>),
}

/// Releases the Host request on completion, cancellation, parsing failure or consumer drop.
pub(super) struct ResponseBody {
    host: Arc<dyn HttpHost>,
    id: String,
    events: tokio::sync::mpsc::UnboundedReceiver<Event>,
    finished: bool,
}

impl ResponseBody {
    /// Binds ordered Host callbacks to one asynchronous response consumer.
    fn open(host: Arc<dyn HttpHost>, request: HttpRequestData) -> Result<Self, AiServiceError> {
        let id = format!("provider-http-{}", uuid::Uuid::new_v4());
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        let head_sender = sender.clone();
        let chunk_sender = sender.clone();
        let body = Self {
            host,
            id,
            events,
            finished: false,
        };
        body.host
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
                Poll::Ready(None)
            }
            Poll::Ready(event) => {
                self.finished = true;
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
        let _ = self.host.closeHttpByteStream(&self.id);
    }
}

#[cfg(test)]
#[path = "../../../../tests/ProviderHttpTests.rs"]
mod tests;
