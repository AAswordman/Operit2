//! TCP 连接和监听适配；socket I/O 由传入的 Host 提供。
use super::stream::{ByteConnection, FramedPeerConnection};
use crate::{PeerConnection, PeerEndpoint, PeerListener, PeerTransport};
use async_trait::async_trait;
use operit_host_api::HostManager::HostManager;
use std::sync::Arc;

struct TcpBytes(Arc<dyn operit_host_api::Tcp::TcpConnection>);
#[async_trait]
impl ByteConnection for TcpBytes {
    fn remoteAddress(&self) -> Option<std::net::SocketAddr> { self.0.remote_address() }
    async fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.0.write(bytes).await.map_err(|e| e.to_string())
    }
    async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        self.0.read().await.map_err(|e| e.to_string())
    }
    async fn close(&self) {
        self.0.close().await;
    }
}
struct TcpPeerListener {
    listener: Arc<dyn operit_host_api::Tcp::TcpListener>,
    source: PeerEndpoint,
    maxMessageBytes: usize,
}
#[async_trait]
impl PeerListener for TcpPeerListener {
    async fn accept(&self) -> Result<Option<Arc<dyn PeerConnection>>, String> {
        let connection = self.listener.accept().await.map_err(|e| e.to_string())?;
        Ok(Some(FramedPeerConnection::new(
            self.source.clone(),
            PeerEndpoint {
                nodeId: String::new(),
                address: String::new(),
            },
            PeerTransport::Tcp,
            Arc::new(TcpBytes(connection)),
            self.maxMessageBytes,
        )))
    }
    async fn close(&self) {
        self.listener.close().await;
    }
}

pub(super) async fn connect(
    host: &HostManager,
    source: PeerEndpoint,
    target: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerConnection>, String> {
    let provider = host.tcpHost.as_ref().ok_or("TCP Host is not installed")?;
    let connection = provider
        .connect(&target.address)
        .await
        .map_err(|e| e.to_string())?;
    Ok(FramedPeerConnection::new(
        source,
        target,
        PeerTransport::Tcp,
        Arc::new(TcpBytes(connection)),
        maxMessageBytes,
    ))
}
pub(super) async fn listen(
    host: &HostManager,
    source: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerListener>, String> {
    let provider = host.tcpHost.as_ref().ok_or("TCP Host is not installed")?;
    let listener = provider
        .bind(&source.address)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(TcpPeerListener { listener, source, maxMessageBytes }))
}
