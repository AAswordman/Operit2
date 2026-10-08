//! HTTP 全双工承载：一个持续响应体接收消息，POST 发送消息；不是 WS 回退。
//! 连接标识仅用于关联 HTTP 请求，不作为节点身份或配对授权。
use super::{
    inbox::{hostTask, Inbox},
    stream::{ByteConnection, FramedPeerConnection},
};
use crate::{PeerConnection, PeerEndpoint, PeerListener, PeerTransport};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::Stream;
use http_body_util::{BodyExt, Full, Limited, StreamBody};
use operit_host_api::{
    HostError, HostManager::HostManager, HostRuntimeTaskSchedulerHost, HttpHost, HttpRequestData,
    HttpServer::*,
};
use std::{
    collections::BTreeMap,
    pin::Pin,
    sync::{Arc, Mutex as StdMutex, Weak},
    task::{Context, Poll},
};
use tokio::sync::{mpsc, oneshot, watch, Mutex};

const CONNECTION_HEADER: &str = "x-operit-link-connection";
fn request(url: String, method: &str, id: &str, body: Vec<u8>) -> HttpRequestData {
    HttpRequestData {
        url,
        method: method.into(),
        headers: vec![
            (CONNECTION_HEADER.into(), id.into()),
            ("content-type".into(), "application/octet-stream".into()),
        ],
        body,
        formFields: vec![],
        fileParts: vec![],
        connectTimeoutSeconds: 15,
        readTimeoutSeconds: if method == "GET" { 0 } else { 15 },
        followRedirects: false,
        ignoreSsl: false,
        proxyHost: String::new(),
        proxyPort: 0,
    }
}
struct Client {
    host: Arc<dyn HttpHost>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    id: String,
    url: String,
    inbox: Arc<Inbox>,
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.host.closeHttpByteStream(&self.id);
    }
}
#[async_trait]
impl ByteConnection for Client {
    async fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.inbox.status()?;
        let host = self.host.clone();
        let request = request(self.url.clone(), "POST", &self.id, bytes.to_vec());
        let result = hostTask(&self.scheduler, move || host.executeHttpRequest(request)).await?;
        if result.statusCode != 204 {
            return Err(format!("HTTP Link send failed: {}", result.statusCode));
        }
        Ok(())
    }
    async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        self.inbox.read().await
    }
    async fn close(&self) {
        self.inbox.finish(Ok(()));
        let _ = self.host.closeHttpByteStream(&self.id);
    }
}
pub(super) async fn connect(
    host: &HostManager,
    source: PeerEndpoint,
    target: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerConnection>, String> {
    let provider = host.httpHost.clone().ok_or("HTTP Host is not installed")?;
    let scheduler = host
        .hostRuntimeTaskSchedulerHost
        .clone()
        .ok_or("Host task scheduler is not installed")?;
    let inbox = Inbox::new(maxMessageBytes);
    let client = Arc::new(Client {
        host: provider.clone(),
        scheduler,
        id: uuid::Uuid::new_v4().to_string(),
        url: target.address.clone(),
        inbox: inbox.clone(),
    });
    let (tx, rx) = oneshot::channel();
    let opened = Arc::new(StdMutex::new(Some(tx)));
    let success = opened.clone();
    let failed = opened.clone();
    let data = inbox.clone();
    provider
        .openHttpByteStream(
            client.id.clone(),
            request(client.url.clone(), "GET", &client.id, vec![]),
            Arc::new(move || {
                if let Some(tx) = success.lock().unwrap().take() {
                    let _ = tx.send(Ok(()));
                }
            }),
            Arc::new(move |bytes| data.put(bytes)),
            Arc::new(move |result| {
                if let Some(tx) = failed.lock().unwrap().take() {
                    let _ = tx.send(Err(result
                        .clone()
                        .err()
                        .unwrap_or("HTTP stream closed before opening".into())));
                }
                inbox.finish(result);
            }),
        )
        .map_err(|e| e.to_string())?;
    rx.await
        .map_err(|_| "HTTP opening cancelled".to_string())??;
    Ok(FramedPeerConnection::new(
        source,
        target,
        PeerTransport::Http,
        client,
        maxMessageBytes,
    ))
}
struct Server {
    remote: Option<std::net::SocketAddr>,
    incoming: Arc<Inbox>,
    outgoing: Mutex<Option<mpsc::Sender<Vec<u8>>>>,
}
#[async_trait]
impl ByteConnection for Server {
    fn remoteAddress(&self) -> Option<std::net::SocketAddr> { self.remote }
    async fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.incoming.status()?;
        let tx = self
            .outgoing
            .lock()
            .await
            .clone()
            .ok_or("HTTP Link closed")?;
        tx.send(bytes.to_vec())
            .await
            .map_err(|_| "HTTP response stream closed".into())
    }
    async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        self.incoming.read().await
    }
    async fn close(&self) {
        self.incoming.finish(Ok(()));
        self.outgoing.lock().await.take();
    }
}
type Connections = Arc<StdMutex<BTreeMap<String, Weak<Server>>>>;
struct ResponseStream {
    rx: mpsc::Receiver<Vec<u8>>,
    id: String,
    connections: Connections,
}
impl Stream for ResponseStream {
    type Item = Result<http_body::Frame<Bytes>, HostError>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx
            .poll_recv(cx)
            .map(|v| v.map(|bytes| Ok(http_body::Frame::data(Bytes::from(bytes)))))
    }
}
impl Drop for ResponseStream {
    fn drop(&mut self) {
        if let Some(connection) = self
            .connections
            .lock()
            .unwrap()
            .remove(&self.id)
            .and_then(|v| v.upgrade())
        {
            connection.incoming.finish(Ok(()));
        }
    }
}
pub(super) fn response(status: u16) -> ServerResponse {
    let mut result = ServerResponse::new(
        Full::new(Bytes::new())
            .map_err(|never| match never {})
            .boxed_unsync(),
    );
    *result.status_mut() = http::StatusCode::from_u16(status).unwrap();
    result
}
pub(super) async fn listen(
    registry: &ServerRegistry,
    host: &HostManager,
    source: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerListener>, String> {
    let address = source.address.clone();
    let connections: Connections = Arc::new(StdMutex::new(BTreeMap::new()));
    let (accept, rx) = mpsc::channel(32);
    let handler: HttpServerHandler = Arc::new(move |request| {
        let connections = connections.clone();
        let accept = accept.clone();
        let source = source.clone();
        Box::pin(async move {
            let Some(id) = request
                .headers()
                .get(CONNECTION_HEADER)
                .and_then(|v| v.to_str().ok())
                .filter(|v| uuid::Uuid::parse_str(v).is_ok())
                .map(str::to_owned)
            else {
                return response(400);
            };
            match *request.method() {
                http::Method::GET => {
                    let Ok(slot) = accept.try_reserve_owned() else {
                        return response(503);
                    };
                    let (tx, rx) = mpsc::channel(32);
                    let connection = Arc::new(Server {
                        remote: request.extensions().get::<operit_host_api::HttpServer::RemoteAddress>().map(|v| v.0),
                        incoming: Inbox::new(maxMessageBytes),
                        outgoing: Mutex::new(Some(tx)),
                    });
                    {
                        let mut entries = connections.lock().unwrap();
                        if entries.contains_key(&id) {
                            return response(409);
                        }
                        if entries.len() >= 128 {
                            return response(503);
                        }
                        entries.insert(id.clone(), Arc::downgrade(&connection));
                    }
                    let body = ResponseStream {
                        rx,
                        id,
                        connections,
                    };
                    slot.send(FramedPeerConnection::new(
                        source,
                        PeerEndpoint {
                            nodeId: String::new(),
                            address: String::new(),
                        },
                        PeerTransport::Http,
                        connection,
                        maxMessageBytes,
                    ) as Arc<dyn PeerConnection>);
                    let mut result = ServerResponse::new(StreamBody::new(body).boxed_unsync());
                    result
                        .headers_mut()
                        .insert("content-type", "application/octet-stream".parse().unwrap());
                    result
                        .headers_mut()
                        .insert("cache-control", "no-store".parse().unwrap());
                    result
                }
                http::Method::POST => {
                    let connection = connections.lock().unwrap().get(&id).and_then(Weak::upgrade);
                    let Some(connection) = connection else {
                        return response(404);
                    };
                    let bytes = match Limited::new(request.into_body(), maxMessageBytes + 4)
                        .collect()
                        .await
                    {
                        Ok(body) => body.to_bytes(),
                        Err(_) => return response(413),
                    };
                    connection.incoming.put(bytes.to_vec());
                    if connection.incoming.status().is_err() {
                        response(410)
                    } else {
                        response(204)
                    }
                }
                _ => response(405),
            }
        })
    });
    serve(registry, host, &address, PeerTransport::Http, rx, browserHttpHandler(handler)).await
}

/// Allows credential-free browser Link requests without bypassing runtime authentication.
fn browserHttpHandler(handler: HttpServerHandler) -> HttpServerHandler {
    Arc::new(move |request| {
        let handler = handler.clone();
        Box::pin(async move {
            let mut result = if request.method() == http::Method::OPTIONS {
                browserPreflight(&request)
            } else {
                handler(request).await
            };
            // Link authentication is carried in encrypted messages, not cookies.
            // CORS only grants access to this transport, never to runtime objects.
            result.headers_mut().insert("access-control-allow-origin", http::HeaderValue::from_static("*"));
            result
        })
    })
}

/// Validates the exact methods and headers used by the binary HTTP Link carrier.
fn browserPreflight(request: &ServerRequest) -> ServerResponse {
    let method = request.headers().get("access-control-request-method").and_then(|value| value.to_str().ok());
    if !matches!(method, Some("GET" | "POST")) {
        return response(405);
    }
    if let Some(headers) = request.headers().get("access-control-request-headers") {
        let Ok(headers) = headers.to_str() else { return response(400); };
        if !headers.split(',').map(str::trim).all(|name| {
            name.eq_ignore_ascii_case("content-type") || name.eq_ignore_ascii_case(CONNECTION_HEADER)
        }) {
            return response(400);
        }
    }
    let mut result = response(204);
    result.headers_mut().insert("access-control-allow-methods", http::HeaderValue::from_static("GET, POST"));
    result.headers_mut().insert("access-control-allow-headers", http::HeaderValue::from_static("content-type, x-operit-link-connection"));
    result
}

// One Host listener for HTTP + WS at the same address. Owned by the PeerLink instance;
// no global server registry and no second socket when exposing both carriers.
pub(super) type ServerRegistry = Mutex<BTreeMap<(usize, String), Weak<SharedServer>>>;
pub(super) struct SharedServer {
    handlers: Arc<StdMutex<BTreeMap<PeerTransport, HttpServerHandler>>>,
    stop: watch::Sender<bool>,
    done: watch::Receiver<Option<Result<(), String>>>,
}
impl Drop for SharedServer {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}
struct Listener {
    incoming: Mutex<mpsc::Receiver<Arc<dyn PeerConnection>>>,
    stop: watch::Sender<bool>,
    server: Arc<SharedServer>,
    transport: PeerTransport,
}
impl Listener {
    fn unregister(&self) {
        if !self.stop.send_replace(true) {
            let mut handlers = self.server.handlers.lock().unwrap();
            handlers.remove(&self.transport);
            if handlers.is_empty() {
                self.server.stop.send_replace(true);
            }
        }
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        self.unregister();
    }
}
#[async_trait]
impl PeerListener for Listener {
    async fn accept(&self) -> Result<Option<Arc<dyn PeerConnection>>, String> {
        let mut rx = self.incoming.lock().await;
        let mut stop = self.stop.subscribe();
        let mut done = self.server.done.clone();
        if *stop.borrow() {
            return Ok(None);
        }
        if let Some(result) = done.borrow().clone() {
            return result.map(|_| None);
        }
        tokio::select! {
            biased;
            _ = stop.wait_for(|closed| *closed) => Ok(None),
            result = done.wait_for(|result| result.is_some()) => match result {
                Ok(state) => state.clone().unwrap_or(Ok(())).map(|_| None),
                Err(_) => Err("HTTP Host server task stopped".into()),
            },
            value = rx.recv() => Ok(value),
        }
    }
    async fn close(&self) {
        self.unregister();
        self.incoming.lock().await.close();
    }
}
pub(super) async fn serve(
    registry: &ServerRegistry,
    host: &HostManager,
    address: &str,
    transport: PeerTransport,
    incoming: mpsc::Receiver<Arc<dyn PeerConnection>>,
    handler: HttpServerHandler,
) -> Result<Arc<dyn PeerListener>, String> {
    let provider = host
        .httpServerHost
        .as_ref()
        .ok_or("HTTP server Host is not installed")?;
    let scheduler = host
        .hostRuntimeTaskSchedulerHost
        .as_ref()
        .ok_or("Host task scheduler is not installed")?;
    let key = (
        Arc::as_ptr(provider) as *const () as usize,
        address.to_string(),
    );
    let mut registry = registry.lock().await;
    registry.retain(|_, value| value.strong_count() != 0);
    let server = match registry.get(&key).and_then(Weak::upgrade) {
        Some(server) if !*server.stop.borrow() => server,
        _ => {
            let socket = provider.bind(address).await.map_err(|e| e.to_string())?;
            let (stop, mut stopped) = watch::channel(false);
            let (done, finished) = watch::channel(None);
            let handlers = Arc::new(StdMutex::new(
                BTreeMap::<PeerTransport, HttpServerHandler>::new(),
            ));
            let dispatchHandlers = handlers.clone();
            let dispatch: HttpServerHandler = Arc::new(move |request| {
                let transport = if request.extensions().get::<WebSocketUpgrade>().is_some() {
                    PeerTransport::WebSocket
                } else {
                    PeerTransport::Http
                };
                let handler = dispatchHandlers.lock().unwrap().get(&transport).cloned();
                Box::pin(async move {
                    match handler {
                        Some(handler) => handler(request).await,
                        None => response(404),
                    }
                })
            });
            scheduler
                .scheduleHostRuntimeAsyncTask(
                    "peer-web-server",
                    Box::new(move || {
                        Box::pin(async move {
                            let result = socket
                                .serve(
                                    dispatch,
                                    Box::pin(async move {
                                        let _ = stopped.wait_for(|stopped| *stopped).await;
                                    }),
                                )
                                .await
                                .map_err(|e| e.to_string());
                            done.send_replace(Some(result));
                        })
                    }),
                )
                .map_err(|e| e.to_string())?;
            let server = Arc::new(SharedServer {
                handlers,
                stop,
                done: finished,
            });
            registry.insert(key, Arc::downgrade(&server));
            server
        }
    };
    {
        let mut handlers = server.handlers.lock().unwrap();
        if handlers.contains_key(&transport) {
            return Err(format!(
                "{transport:?} listener already active at {address}"
            ));
        }
        handlers.insert(transport, handler);
    }
    Ok(Arc::new(Listener {
        incoming: Mutex::new(incoming),
        stop: watch::channel(false).0,
        server,
        transport,
    }))
}


#[cfg(test)]
mod browser_cors_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Creates a browser preflight without opening a Link connection.
    fn preflight(method: &str, headers: &str) -> ServerRequest {
        http::Request::builder()
            .method("OPTIONS")
            .uri("/link")
            .header("origin", "https://operit.example")
            .header("access-control-request-method", method)
            .header("access-control-request-headers", headers)
            .body(Full::new(Bytes::new()).map_err(|never| match never {}).boxed_unsync())
            .unwrap()
    }

    /// Accepts binary Link preflights without invoking runtime or allocating a connection.
    #[tokio::test]
    async fn browser_preflight_does_not_dispatch_pairing() {
        let calls = Arc::new(AtomicUsize::new(0));
        let recorded = calls.clone();
        let handler = browserHttpHandler(Arc::new(move |_| {
            recorded.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { response(401) })
        }));
        for method in ["GET", "POST"] {
            let result = handler(preflight(method, "Content-Type, X-Operit-Link-Connection")).await;
            assert_eq!(result.status(), 204);
            assert_eq!(result.headers()["access-control-allow-origin"], "*");
            assert_eq!(result.headers()["access-control-allow-methods"], "GET, POST");
            assert!(!result.headers().contains_key("access-control-allow-credentials"));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(handler(preflight("DELETE", "content-type")).await.status(), 405);
        assert_eq!(handler(preflight("POST", "authorization")).await.status(), 400);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    /// Preserves the observed peer address and authentication rejection for actual requests.
    #[tokio::test]
    async fn browser_request_keeps_runtime_authentication_and_remote_address() {
        let remote = "127.0.0.1:12456".parse().unwrap();
        let handler = browserHttpHandler(Arc::new(move |request| {
            assert_eq!(request.extensions().get::<RemoteAddress>().unwrap().0, remote);
            Box::pin(async { response(401) })
        }));
        let mut request = preflight("POST", "content-type");
        *request.method_mut() = http::Method::POST;
        request.extensions_mut().insert(RemoteAddress(remote));
        let result = handler(request).await;
        assert_eq!(result.status(), 401);
        assert_eq!(result.headers()["access-control-allow-origin"], "*");
        assert!(!result.headers().contains_key("access-control-allow-credentials"));
    }
}
