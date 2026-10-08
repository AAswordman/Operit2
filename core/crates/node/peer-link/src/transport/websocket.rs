//! WebSocket transport: 客户端与 upgrade socket 都由 Host 拥有。
use super::{
    inbox::{hostTask, Inbox},
    stream::{ByteConnection, FramedPeerConnection},
};
use crate::{PeerConnection, PeerEndpoint, PeerListener, PeerTransport};
use async_trait::async_trait;
use operit_host_api::{
    HostManager::HostManager, HostRuntimeTaskSchedulerHost, HttpServer::*, WebSocketHost,
    WebSocketRequestData,
};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{mpsc, oneshot};

struct Client {
    host: Arc<dyn WebSocketHost>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    id: String,
    inbox: Arc<Inbox>,
}
impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.host.closeWebSocket(&self.id);
    }
}
#[async_trait]
impl ByteConnection for Client {
    async fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.inbox.status()?;
        let (host, id, bytes) = (self.host.clone(), self.id.clone(), bytes.to_vec());
        hostTask(&self.scheduler, move || {
            host.sendWebSocketMessage(&id, bytes)
        })
        .await
    }
    async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        self.inbox.read().await
    }
    async fn close(&self) {
        self.inbox.finish(Ok(()));
        let _ = self.host.closeWebSocket(&self.id);
    }
}
pub(super) async fn connect(
    host: &HostManager,
    source: PeerEndpoint,
    target: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerConnection>, String> {
    let provider = host
        .webSocketHost
        .clone()
        .ok_or("WebSocket Host is not installed")?;
    let scheduler = host
        .hostRuntimeTaskSchedulerHost
        .clone()
        .ok_or("Host task scheduler is not installed")?;
    let inbox = Inbox::new(maxMessageBytes);
    let client = Arc::new(Client {
        host: provider.clone(),
        scheduler,
        id: uuid::Uuid::new_v4().to_string(),
        inbox: inbox.clone(),
    });
    let (opened, ready) = oneshot::channel();
    let opened = Arc::new(StdMutex::new(Some(opened)));
    let success = opened.clone();
    let failed = opened.clone();
    let data = inbox.clone();
    provider
        .openWebSocket(
            client.id.clone(),
            WebSocketRequestData {
                url: target.address.clone(),
                headers: vec![],
                connectTimeoutSeconds: 15,
                ignoreSsl: false,
            },
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
                        .unwrap_or("WebSocket closed before opening".into())));
                }
                inbox.finish(result);
            }),
        )
        .map_err(|e| e.to_string())?;
    ready
        .await
        .map_err(|_| "WebSocket opening cancelled".to_string())??;
    Ok(FramedPeerConnection::new(
        source,
        target,
        PeerTransport::WebSocket,
        client,
        maxMessageBytes,
    ))
}
struct Server {
    remote: Option<std::net::SocketAddr>,
    output: mpsc::Sender<(Vec<u8>, oneshot::Sender<Result<(), String>>)>,
    inbox: Arc<Inbox>,
    stop: tokio::sync::watch::Sender<bool>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}
#[async_trait]
impl ByteConnection for Server {
    fn remoteAddress(&self) -> Option<std::net::SocketAddr> { self.remote }
    async fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.inbox.status()?;
        let (tx, rx) = oneshot::channel();
        self.output
            .send((bytes.to_vec(), tx))
            .await
            .map_err(|_| "WebSocket closed".to_string())?;
        rx.await
            .map_err(|_| "WebSocket write cancelled".to_string())?
    }
    async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        self.inbox.read().await
    }
    async fn close(&self) {
        self.stop.send_replace(true);
        self.inbox.finish(Ok(()));
    }
}
pub(super) async fn listen(
    registry: &super::http::ServerRegistry,
    host: &HostManager,
    source: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerListener>, String> {
    let address = source.address.clone();
    let (tx, rx) = mpsc::channel(32);
    let handler: HttpServerHandler = Arc::new(move |mut request| {
        let tx = tx.clone();
        let source = source.clone();
        Box::pin(async move {
            let remote = request.extensions().get::<operit_host_api::HttpServer::RemoteAddress>().map(|v| v.0);
            let Some(upgrade) = request.extensions_mut().remove::<WebSocketUpgrade>() else {
                return super::http::response(400);
            };
            let Ok(slot) = tx.try_reserve_owned() else {
                return super::http::response(503);
            };
            upgrade.accept(Box::new(move |mut socket| Box::pin(async move {
                let (output, mut outgoing) = mpsc::channel::<(Vec<u8>, oneshot::Sender<Result<(), String>>)>(32);
                let (stop, mut stopped) = tokio::sync::watch::channel(false);
                let inbox = Inbox::new(maxMessageBytes);
                let connection = Arc::new(Server { remote, output, inbox: inbox.clone(), stop });
                slot.send(FramedPeerConnection::new(source, PeerEndpoint { nodeId: String::new(), address: String::new() }, PeerTransport::WebSocket, connection, maxMessageBytes) as Arc<dyn PeerConnection>);
                let result: Result<(), String> = async {
                    loop {
                        tokio::select! {
                            biased;
                            _ = stopped.changed() => break,
                            outgoing = outgoing.recv() => match outgoing {
                                Some((bytes, ack)) => {
                                    let result = socket.send(WebSocketMessage::Binary(bytes)).await.map_err(|e| e.to_string());
                                    let _ = ack.send(result.clone()); result?;
                                },
                                None => break,
                            },
                            incoming = socket.recv() => match incoming {
                                Some(Ok(WebSocketMessage::Binary(bytes))) => { inbox.put(bytes); inbox.status()?; },
                                Some(Ok(WebSocketMessage::Ping(bytes))) => socket.send(WebSocketMessage::Pong(bytes)).await.map_err(|e| e.to_string())?,
                                Some(Ok(WebSocketMessage::Pong(_))) => {},
                                Some(Ok(WebSocketMessage::Text(_))) => return Err("Link requires binary WebSocket messages".into()),
                                Some(Err(e)) => return Err(e.to_string()),
                                _ => break,
                            },
                        }
                    }
                    Ok(())
                }.await;
                // Pairing and one-shot calls send their last response and close.
                // Dropping the upgraded socket without a Close frame reports a
                // protocol reset, which can hide that already-buffered response.
                if result.is_ok() {
                    let _ = socket.send(WebSocketMessage::Close(None)).await;
                }
                inbox.finish(result);
            }))).unwrap_or_else(|_| super::http::response(500))
        })
    });
    // bind address belongs to the listener, not a remote websocket URL.
    super::http::serve(
        registry,
        host,
        &address,
        PeerTransport::WebSocket,
        rx,
        handler,
    )
    .await
}
