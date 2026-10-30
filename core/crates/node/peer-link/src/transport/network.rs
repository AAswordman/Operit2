//! One listener per Host/address; carrier registration does not bind more sockets.
//! Platform sockets and HTTP parsing stay in Host; Core selects the Link carrier.
use crate::{PeerConnection, PeerListener, PeerTransport};
use async_trait::async_trait;
use operit_host_api::{HostManager::HostManager, HostResult, HttpServer::*, TcpConnection};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex as StdMutex, Weak},
    time::Duration,
};
use tokio::sync::{mpsc, watch, Mutex, Semaphore};

pub(super) type ServerRegistry = Mutex<BTreeMap<(usize, usize, String), Weak<SharedServer>>>;
type TcpHandler = Arc<dyn Fn(Arc<dyn TcpConnection>) -> Result<(), String> + Send + Sync>;
#[derive(Clone)]
pub(super) enum Handler {
    Http(HttpServerHandler),
    Tcp(TcpHandler),
}
type Handlers = Arc<StdMutex<BTreeMap<PeerTransport, Handler>>>;
pub(super) struct SharedServer {
    address: String,
    handlers: Handlers,
    stop: watch::Sender<bool>,
    done: watch::Receiver<Option<Result<(), String>>>,
    waitForRelease: bool,
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
    fn localAddress(&self) -> Option<String> {
        Some(self.server.address.clone())
    }
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
                Err(_) => Err("Network Host server task stopped".into()),
            },
            value = rx.recv() => Ok(value),
        }
    }
    async fn close(&self) {
        self.unregister();
        let mut incoming = self.incoming.lock().await;
        incoming.close();
        while let Ok(connection) = incoming.try_recv() {
            connection.close().await;
        }
        drop(incoming);
        // The shared accept loop releases its socket before returning. Legacy
        // HTTP Hosts may still be draining streams that the caller closes next.
        if self.server.waitForRelease && *self.server.stop.borrow() {
            let mut done = self.server.done.clone();
            let _ = done.wait_for(|result| result.is_some()).await;
        }
    }
}
pub(super) fn canShare(host: &HostManager) -> bool {
    host.tcpHost.is_some()
        && host
            .httpServerHost
            .as_ref()
            .is_some_and(|http| http.supportsAcceptedConnections())
}
fn dispatch(handlers: Handlers) -> HttpServerHandler {
    Arc::new(move |request| {
        let transport = if request.extensions().get::<WebSocketUpgrade>().is_some() {
            PeerTransport::WebSocket
        } else {
            PeerTransport::Http
        };
        let handler = handlers.lock().unwrap().get(&transport).cloned();
        Box::pin(async move {
            match handler {
                Some(Handler::Http(handler)) => handler(request).await,
                _ => super::http::response(404),
            }
        })
    })
}

/// Bind once (including port zero), then register carriers against the concrete address.
pub(super) async fn serve(
    registry: &ServerRegistry,
    host: &HostManager,
    address: &str,
    transport: PeerTransport,
    incoming: mpsc::Receiver<Arc<dyn PeerConnection>>,
    makeHandler: impl FnOnce(&str) -> Handler + Send,
) -> Result<Arc<dyn PeerListener>, String> {
    let provider = host
        .httpServerHost
        .as_ref()
        .ok_or("HTTP server Host is not installed")?;
    let scheduler = host
        .hostRuntimeTaskSchedulerHost
        .as_ref()
        .ok_or("Host task scheduler is not installed")?;
    let shared = canShare(host);
    let tcpKey = if shared {
        Arc::as_ptr(host.tcpHost.as_ref().unwrap()) as *const () as usize
    } else {
        0
    };
    let key = (
        Arc::as_ptr(provider) as *const () as usize,
        tcpKey,
        address.to_owned(),
    );
    let mut registry = registry.lock().await;
    registry.retain(|_, value| value.strong_count() != 0);
    let server = match registry.get(&key).and_then(Weak::upgrade) {
        Some(server) if !*server.stop.borrow() => server,
        _ => {
            let (stop, mut stopped) = watch::channel(false);
            let (done, finished) = watch::channel(None);
            let handlers: Handlers = Arc::new(StdMutex::new(BTreeMap::new()));
            let handler = dispatch(handlers.clone());
            let (boundAddress, serving): (_, ServerFuture<Result<(), String>>) = if shared {
                let listener = host
                    .tcpHost
                    .as_ref()
                    .unwrap()
                    .bind(address)
                    .await
                    .map_err(|e| e.to_string())?;
                let bound = listener.local_address().map_err(|e| e.to_string())?;
                let http = provider.clone();
                let tasks = scheduler.clone();
                let handlers = handlers.clone();
                let stopped = stopped.clone();
                (
                    bound,
                    Box::pin(async move {
                        serveShared(listener, http, tasks, handlers, handler, stopped).await
                    }),
                )
            } else {
                let listener = provider.bind(address).await.map_err(|e| e.to_string())?;
                let bound = listener
                    .localAddress()
                    .map_err(|e| e.to_string())?
                    .to_string();
                (
                    bound,
                    Box::pin(async move {
                        listener
                            .serve(
                                handler,
                                Box::pin(async move {
                                    let _ = stopped.wait_for(|stopped| *stopped).await;
                                }),
                            )
                            .await
                            .map_err(|e| e.to_string())
                    }),
                )
            };
            // Install before scheduling: the first accepted connection sees its carrier.
            handlers
                .lock()
                .unwrap()
                .insert(transport, makeHandler(&boundAddress));
            let server = Arc::new(SharedServer {
                address: boundAddress.clone(),
                handlers,
                stop,
                done: finished,
                waitForRelease: shared,
            });
            scheduler
                .scheduleHostRuntimeAsyncTask(
                    "peer-network-server",
                    Box::new(move || {
                        Box::pin(async move {
                            let result = serving.await;
                            done.send_replace(Some(result));
                        })
                    }),
                )
                .map_err(|e| e.to_string())?;
            registry.insert((key.0, key.1, boundAddress), Arc::downgrade(&server));
            return Ok(Arc::new(Listener {
                incoming: Mutex::new(incoming),
                stop: watch::channel(false).0,
                server,
                transport,
            }));
        }
    };
    {
        let mut handlers = server.handlers.lock().unwrap();
        if handlers.contains_key(&transport) {
            return Err(format!(
                "{transport:?} listener already active at {address}"
            ));
        }
        handlers.insert(transport, makeHandler(&server.address));
    }
    Ok(Arc::new(Listener {
        incoming: Mutex::new(incoming),
        stop: watch::channel(false).0,
        server,
        transport,
    }))
}

async fn serveShared(
    listener: Arc<dyn operit_host_api::TcpListener>,
    http: Arc<dyn HttpServerHost>,
    scheduler: Arc<dyn operit_host_api::HostRuntimeTaskSchedulerHost>,
    handlers: Handlers,
    handler: HttpServerHandler,
    mut stopped: watch::Receiver<bool>,
) -> Result<(), String> {
    // Bound classification and unadmitted HTTP sessions; each socket is handled independently.
    let slots = Arc::new(Semaphore::new(128));
    let result = loop {
        let connection = tokio::select! {
            biased;
            _ = stopped.wait_for(|stop| *stop) => break Ok(()),
            accepted = listener.accept() => match accepted {
                Ok(connection) => connection,
                Err(e) => break Err(e.to_string()),
            }
        };
        let tcpOnly = {
            let handlers = handlers.lock().unwrap();
            if handlers.contains_key(&PeerTransport::Http)
                || handlers.contains_key(&PeerTransport::WebSocket)
            {
                None
            } else {
                handlers.get(&PeerTransport::Tcp).cloned()
            }
        };
        if let Some(Handler::Tcp(handler)) = tcpOnly {
            if handler(connection.clone()).is_err() {
                connection.close().await;
            }
            continue;
        }
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            connection.close().await;
            continue;
        };
        let (http, handlers, handler, mut stop) = (
            http.clone(),
            handlers.clone(),
            handler.clone(),
            stopped.clone(),
        );
        let accepted = connection.clone();
        if scheduler.scheduleHostRuntimeAsyncTask("peer-network-connection", Box::new(move || Box::pin(async move {
            let _permit = permit;
            let classification = tokio::select! {
                biased;
                _ = stop.wait_for(|stop| *stop) => None,
                result = tokio::time::timeout(Duration::from_secs(5), classify(accepted.clone())) => result.ok().and_then(Result::ok),
            };
            let Some((isHttp, stream)) = classification else { accepted.close().await; return; };
            if isHttp {
                let enabled = {
                    let handlers = handlers.lock().unwrap();
                    handlers.contains_key(&PeerTransport::Http) || handlers.contains_key(&PeerTransport::WebSocket)
                };
                if !enabled { accepted.close().await; return; }
                let _ = http.serveConnection(stream, handler, Box::pin(async move {
                    let _ = stop.wait_for(|stop| *stop).await;
                })).await;
                accepted.close().await;
            } else {
                let tcp = handlers.lock().unwrap().get(&PeerTransport::Tcp).cloned();
                if !matches!(tcp, Some(Handler::Tcp(handler)) if handler(stream).is_ok()) {
                    accepted.close().await;
                }
            }
        }))).is_err() { connection.close().await; }
    };
    listener.close().await;
    result
}

/// Preserve every classified byte and the actual remote address for authentication policy.
struct PrefixedConnection {
    connection: Arc<dyn TcpConnection>,
    prefix: Mutex<std::collections::VecDeque<Vec<u8>>>,
}
#[async_trait]
impl TcpConnection for PrefixedConnection {
    fn remote_address(&self) -> Option<std::net::SocketAddr> {
        self.connection.remote_address()
    }
    async fn write(&self, bytes: &[u8]) -> HostResult<()> {
        self.connection.write(bytes).await
    }
    async fn read(&self) -> HostResult<Option<Vec<u8>>> {
        if let Some(prefix) = self.prefix.lock().await.pop_front() {
            return Ok(Some(prefix));
        }
        self.connection.read().await
    }
    async fn close(&self) {
        self.connection.close().await;
    }
}
async fn classify(
    connection: Arc<dyn TcpConnection>,
) -> HostResult<(bool, Arc<dyn TcpConnection>)> {
    let mut prefix = Vec::new();
    let mut chunks = std::collections::VecDeque::new();
    let isHttp = loop {
        let Some(bytes) = connection.read().await? else {
            return Err(operit_host_api::HostError::new(
                "Connection closed before protocol header",
            ));
        };
        if bytes.is_empty() || bytes.len() > 4096 {
            return Err(operit_host_api::HostError::new("Invalid Host TCP chunk"));
        }
        prefix.extend_from_slice(&bytes);
        chunks.push_back(bytes);
        if prefix.len() < 4 {
            continue;
        }
        // Recognize a complete HTTP method and the first request-target byte.
        // Checking only four ASCII bytes would misclassify large Link lengths
        // such as 0x504f5354 (POST); their MessagePack payload is not an HTTP target.
        let methodEnd = prefix.iter().position(|byte| !methodByte(*byte));
        match methodEnd {
            Some(end) if end > 0 && end <= 64 && prefix[end] == b' ' => {
                if let Some(target) = prefix.get(end + 1) {
                    break target.is_ascii_graphic();
                }
            }
            Some(_) => break false,
            None if prefix.len() >= 64 => break false,
            None => {}
        }
    };
    Ok((
        isHttp,
        Arc::new(PrefixedConnection {
            connection,
            prefix: Mutex::new(chunks),
        }),
    ))
}
fn methodByte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Chunks(StdMutex<std::collections::VecDeque<Vec<u8>>>);
    #[async_trait]
    impl TcpConnection for Chunks {
        fn remote_address(&self) -> Option<std::net::SocketAddr> {
            Some("127.0.0.1:12345".parse().unwrap())
        }
        async fn read(&self) -> HostResult<Option<Vec<u8>>> {
            Ok(self.0.lock().unwrap().pop_front())
        }
        async fn write(&self, _: &[u8]) -> HostResult<()> {
            unreachable!()
        }
        async fn close(&self) {}
    }
    #[tokio::test]
    async fn classification_preserves_fragmented_headers_payload_and_remote_address() {
        for (bytes, http) in [
            (b"GET /link HTTP/1.1\r\n".as_slice(), true),
            (b"POST /link HTTP/1.1\r\n".as_slice(), true),
            (b"OPTIONS /link HTTP/1.1\r\n".as_slice(), true),
            (b"M-SEARCH * HTTP/1.1\r\n".as_slice(), true),
            (b"\x00\x00\x00\x04\x92\x01\x02\x03".as_slice(), false),
            (b"POST\x92\x01\x02\x03".as_slice(), false),
            (b"GET \x92\x01\x02\x03".as_slice(), false),
        ] {
            for size in 1..=bytes.len() {
                let connection = Arc::new(Chunks(StdMutex::new(
                    bytes.chunks(size).map(|b| b.to_vec()).collect(),
                )));
                let (detected, stream) = classify(connection).await.unwrap();
                assert_eq!(detected, http, "{bytes:?}, chunk size {size}");
                assert_eq!(
                    stream.remote_address(),
                    Some("127.0.0.1:12345".parse().unwrap())
                );
                let mut replayed = Vec::new();
                while let Some(bytes) = stream.read().await.unwrap() {
                    replayed.extend(bytes);
                }
                assert_eq!(replayed, bytes);
            }
        }
    }
    #[tokio::test]
    async fn incomplete_headers_and_invalid_host_chunks_are_rejected() {
        for chunks in [vec![vec![0, 0]], vec![vec![]], vec![vec![0; 4097]]] {
            assert!(classify(Arc::new(Chunks(StdMutex::new(chunks.into()))))
                .await
                .is_err());
        }
    }
}
