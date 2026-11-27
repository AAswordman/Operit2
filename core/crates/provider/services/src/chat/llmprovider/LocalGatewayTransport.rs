//! Routes gateway HTTP traffic without moving the conversation or interpreting model output.
use operit_host_api::{HttpRequestData, HttpResponseHead};
use operit_link::{CoreCallRequest, CoreStream, CoreValue, CoreWatchRequest};
use operit_model::ModelConfigData::ApiProviderType;
use serde::{Deserialize, Serialize};

use super::ProviderHttp::Event;

/// Carries the HTTP request fields used by model protocols across a device route.
#[derive(Serialize, Deserialize)]
pub struct GatewayRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// Preserves response metadata, ordered byte chunks, and an explicit terminal outcome.
#[derive(Serialize, Deserialize)]
pub enum GatewayEvent {
    Head {
        url: String,
        status: i32,
        message: String,
        headers: Vec<(String, String)>,
    },
    Bytes(Vec<u8>),
    Closed(Result<(), String>),
}

/// Selects the protocol declared by the endpoint's final path segment.
pub fn protocol(endpoint: &str) -> Result<ApiProviderType, String> {
    let url = url::Url::parse(endpoint.trim().trim_end_matches('#'))
        .map_err(|error| error.to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Local gateway endpoint must use HTTP or HTTPS".into());
    }
    match url.path().trim_end_matches('/').rsplit('/').next() {
        Some("messages") => Ok(ApiProviderType::ANTHROPIC_GENERIC),
        Some("responses") => Ok(ApiProviderType::OPENAI_RESPONSES_GENERIC),
        Some("" | "v1" | "completions") => Ok(ApiProviderType::OPENAI_GENERIC),
        _ => Err(
            "Local gateway endpoint must end in /v1, /chat/completions, /responses, or /messages"
                .into(),
        ),
    }
}

/// Resolves a route result without replacing route or permission failures with local execution.
pub fn call<T: serde::de::DeserializeOwned + Send + 'static>(
    method: &str,
    args: serde_json::Value,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, String>> + Send>> {
    let method = method.to_string();
    let (mut sender, receiver) = tokio::sync::oneshot::channel();
    let scheduled = operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "local-gateway-route",
            Box::new(move || {
                Box::pin(async move {
                    let work = async {
                        let runtime = operit_link::coreRouteRuntime()
                            .ok_or("Local gateway requires an active Core route connection")?;
                        let request = CoreCallRequest::new(
                            operit_link::nextCoreRouteRequestId(&method),
                            operit_link::CORE_INTERNAL_TARGET,
                            method,
                            operit_link::toCoreValue(args).map_err(|error| error.to_string())?,
                        );
                        let value = runtime
                            .call(request)
                            .await
                            .result
                            .map_err(|error| error.to_string())?;
                        operit_link::fromCoreValue(value).map_err(|error| error.to_string())
                    };
                    tokio::select! {
                        biased;
                        _ = sender.closed() => {},
                        result = work => { let _ = sender.send(result); },
                    }
                })
            }),
        );
    Box::pin(async move {
        scheduled.map_err(|error| error.to_string())?;
        receiver.await.map_err(|error| error.to_string())?
    })
}

/// Forwards the remote HTTP lifecycle while dropping its route watch on consumer cancellation.
pub(super) async fn forward_response(
    binding: String,
    request: HttpRequestData,
    sender: &tokio::sync::mpsc::UnboundedSender<Event>,
) -> Result<(), String> {
    let runtime = operit_link::coreRouteRuntime()
        .ok_or("Local gateway requires an active Core route connection")?;
    forward_with_route(runtime, binding, request, sender).await
}

/// Opens the gateway on one captured route runtime for the complete response lifetime.
async fn forward_with_route(
    runtime: std::sync::Arc<dyn operit_link::CoreRouteRuntime>,
    binding: String,
    request: HttpRequestData,
    sender: &tokio::sync::mpsc::UnboundedSender<Event>,
) -> Result<(), String> {
    let request = GatewayRequest {
        url: request.url,
        headers: request.headers,
        body: request.body,
    };
    let work = async {
        let args = operit_link::toCoreValue(serde_json::json!({
            "bindingKey": binding, "requestJson": serde_json::to_string(&request).map_err(|error| error.to_string())?,
        })).map_err(|error| error.to_string())?;
        let value = runtime
            .call(CoreCallRequest::new(
                operit_link::nextCoreRouteRequestId("openLocalGatewayResponse"),
                operit_link::CORE_INTERNAL_TARGET,
                "openLocalGatewayResponse",
                args,
            ))
            .await
            .result
            .map_err(|error| error.to_string())?;
        let descriptor: CoreStream<serde_json::Value> =
            operit_link::fromCoreValue(value).map_err(|error| error.to_string())?;
        let mut stream = runtime
            .watch(CoreWatchRequest::new(
                operit_link::nextCoreRouteRequestId("localGatewayResponse"),
                descriptor.descriptor.target,
                descriptor.descriptor.propertyName,
                descriptor.descriptor.args,
            ))
            .await
            .map_err(|error| error.to_string())?;
        while let Some(event) = stream.recv().await {
            if event.kind == operit_link::CoreEventKind::Completed {
                break;
            }
            let value: GatewayEvent =
                operit_link::fromCoreValue(event.value).map_err(|error| error.to_string())?;
            let (event, done) = match value {
                GatewayEvent::Head {
                    url,
                    status,
                    message,
                    headers,
                } => (
                    Event::Head(HttpResponseHead {
                        finalUrl: url,
                        statusCode: status,
                        statusMessage: message,
                        headers,
                    }),
                    false,
                ),
                GatewayEvent::Bytes(bytes) => (Event::Bytes(bytes), false),
                GatewayEvent::Closed(result) => (Event::Closed(result), true),
            };
            sender
                .send(event)
                .map_err(|_| "Local gateway response was cancelled".to_string())?;
            if done {
                return Ok(());
            }
        }
        Err("Local gateway response disconnected without completion".into())
    };
    tokio::select! {
        biased;
        _ = sender.closed() => Ok(()),
        result = work => result,
    }
}

/// Converts one protocol lifecycle event to the route's binary-safe value format.
pub fn event_value(event: GatewayEvent) -> Result<CoreValue, String> {
    operit_link::toCoreValue(event).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_link::{
        CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkError,
        CoreRouteRuntime, CoreStreamSource,
    };
    use std::{
        future::Future,
        pin::Pin,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
    };

    struct GatewayRoute {
        requests: Mutex<Vec<CoreCallRequest>>,
        events: Mutex<Vec<GatewayEvent>>,
        denied: bool,
        keep_open: bool,
        closed: Arc<AtomicBool>,
    }

    impl CoreRouteRuntime for GatewayRoute {
        /// Keeps test dispatch on the ordinary route call path.
        fn shouldRoute(&self, _: &str, _: &CoreValue) -> Result<bool, CoreLinkError> {
            Ok(true)
        }

        /// Records the gateway owner and returns either a stream descriptor or a route denial.
        fn call(
            &self,
            request: CoreCallRequest,
        ) -> Pin<Box<dyn Future<Output = CoreCallResponse>>> {
            let id = request.requestId.clone();
            self.requests.lock().unwrap().push(request);
            let response = if self.denied {
                CoreCallResponse::err(
                    id,
                    CoreLinkError::new("ROUTE_PERMISSION_DENIED", "gateway denied"),
                )
            } else {
                let descriptor = CoreStream::<serde_json::Value>::fromSource(Arc::new(
                    CoreStreamSource::new(|_| unreachable!("the route runtime owns the watch")),
                ));
                CoreCallResponse::ok(id, operit_link::toCoreValue(descriptor).unwrap())
            };
            Box::pin(async move { response })
        }

        /// Emits the exact remote HTTP lifecycle and records Watch release.
        fn watch(
            &self,
            request: CoreWatchRequest,
        ) -> Pin<Box<dyn Future<Output = Result<CoreEventStream, CoreLinkError>>>> {
            let (sender, stream) = CoreEventStream::channel();
            for event in self.events.lock().unwrap().drain(..) {
                sender
                    .send(CoreEvent {
                        requestId: Some(request.requestId.clone()),
                        target: request.target.clone(),
                        propertyName: request.propertyName.clone(),
                        kind: CoreEventKind::Changed,
                        value: event_value(event).unwrap(),
                    })
                    .unwrap();
            }
            let closed = self.closed.clone();
            let lease = self.keep_open.then(|| sender.clone());
            let stream = stream.withOnClose(move || {
                drop(lease);
                closed.store(true, Ordering::SeqCst);
            });
            Box::pin(async move {
                drop(sender);
                Ok(stream)
            })
        }
    }

    /// Builds a request whose localhost address must be interpreted on the gateway owner.
    fn request() -> HttpRequestData {
        HttpRequestData {
            url: "http://127.0.0.1:8000/v1/responses".into(),
            method: "POST".into(),
            headers: vec![("Authorization".into(), "Bearer gateway-key".into())],
            body: b"payload".to_vec(),
            formFields: vec![],
            fileParts: vec![],
            connectTimeoutSeconds: 60,
            readTimeoutSeconds: 1000,
            followRedirects: true,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        }
    }

    /// Preserves non-success statuses, byte chunk boundaries, terminal errors, and owner identity.
    #[tokio::test]
    async fn routed_gateway_preserves_http_lifecycle_and_releases_watch() {
        let route = Arc::new(GatewayRoute {
            requests: Mutex::new(vec![]),
            denied: false,
            keep_open: false,
            closed: Arc::new(AtomicBool::new(false)),
            events: Mutex::new(vec![
                GatewayEvent::Head {
                    url: request().url,
                    status: 429,
                    message: "rate limited".into(),
                    headers: vec![],
                },
                GatewayEvent::Bytes(vec![0, 255, 17]),
                GatewayEvent::Bytes(vec![9]),
                GatewayEvent::Closed(Err("upstream interrupted".into())),
            ]),
        });
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        forward_with_route(route.clone(), "computer-gateway".into(), request(), &sender)
            .await
            .unwrap();
        assert!(
            matches!(receiver.try_recv().unwrap(), Event::Head(head) if head.statusCode == 429)
        );
        assert!(
            matches!(receiver.try_recv().unwrap(), Event::Bytes(bytes) if bytes == vec![0,255,17])
        );
        assert!(matches!(receiver.try_recv().unwrap(), Event::Bytes(bytes) if bytes == vec![9]));
        assert!(
            matches!(receiver.try_recv().unwrap(), Event::Closed(Err(error)) if error == "upstream interrupted")
        );
        assert!(route.closed.load(Ordering::SeqCst));
        let requests = route.requests.lock().unwrap();
        assert_eq!(requests[0].methodName, "openLocalGatewayResponse");
        let args: serde_json::Value = operit_link::fromCoreValue(requests[0].args.clone()).unwrap();
        assert_eq!(args["bindingKey"], "computer-gateway");
        let sent: GatewayRequest =
            serde_json::from_str(args["requestJson"].as_str().unwrap()).unwrap();
        assert_eq!(sent.url, request().url);
        assert_eq!(sent.headers, request().headers);
        assert_eq!(sent.body, request().body);
    }

    /// Exposes route permission failures without starting any response watch or local HTTP request.
    #[tokio::test]
    async fn denied_gateway_route_returns_its_error() {
        let route = Arc::new(GatewayRoute {
            requests: Mutex::new(vec![]),
            events: Mutex::new(vec![]),
            denied: true,
            keep_open: false,
            closed: Arc::new(AtomicBool::new(false)),
        });
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let error =
            forward_with_route(route.clone(), "computer-gateway".into(), request(), &sender)
                .await
                .unwrap_err();
        assert!(error.contains("gateway denied"));
        assert_eq!(route.requests.lock().unwrap().len(), 1);
        assert!(!route.closed.load(Ordering::SeqCst));
    }

    /// Rejects transport disconnection when the gateway never confirmed a complete HTTP response.
    #[tokio::test]
    async fn disconnected_gateway_does_not_complete_successfully() {
        let route = Arc::new(GatewayRoute {
            requests: Mutex::new(vec![]),
            events: Mutex::new(vec![]),
            denied: false,
            keep_open: false,
            closed: Arc::new(AtomicBool::new(false)),
        });
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        assert!(
            forward_with_route(route.clone(), "computer-gateway".into(), request(), &sender)
                .await
                .unwrap_err()
                .contains("without completion")
        );
        assert!(route.closed.load(Ordering::SeqCst));
    }

    /// Releases an active remote response as soon as its model consumer is cancelled.
    #[tokio::test]
    async fn cancelling_the_consumer_releases_the_gateway_watch() {
        let route = Arc::new(GatewayRoute {
            requests: Mutex::new(vec![]),
            denied: false,
            keep_open: true,
            closed: Arc::new(AtomicBool::new(false)),
            events: Mutex::new(vec![GatewayEvent::Head {
                url: request().url,
                status: 200,
                message: "ok".into(),
                headers: vec![],
            }]),
        });
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let work = forward_with_route(route.clone(), "computer-gateway".into(), request(), &sender);
        tokio::pin!(work);
        tokio::select! {
            result = &mut work => panic!("The active response ended before cancellation: {result:?}"),
            event = receiver.recv() => assert!(matches!(event, Some(Event::Head(_)))),
        }
        assert!(!route.closed.load(Ordering::SeqCst));
        drop(receiver);
        work.await.unwrap();
        assert!(route.closed.load(Ordering::SeqCst));
    }

    /// Selects each supported gateway protocol using the declared endpoint path.
    #[test]
    fn endpoint_selects_the_declared_protocol() {
        for (path, expected) in [
            ("", ApiProviderType::OPENAI_GENERIC),
            ("/v1", ApiProviderType::OPENAI_GENERIC),
            ("/v1/chat/completions", ApiProviderType::OPENAI_GENERIC),
            ("/v1/responses", ApiProviderType::OPENAI_RESPONSES_GENERIC),
            ("/v1/messages", ApiProviderType::ANTHROPIC_GENERIC),
        ] {
            assert_eq!(
                protocol(&format!("http://localhost:8000{path}")).unwrap(),
                expected
            );
        }
        assert!(protocol("file:///v1/messages").is_err());
        assert!(protocol("http://localhost:8000/unknown").is_err());
    }
}
