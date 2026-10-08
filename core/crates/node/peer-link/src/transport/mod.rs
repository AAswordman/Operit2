//! 只按 transport 选择 Host 适配，不处理鉴权或配对。
mod bluetooth;
mod http;
mod inbox;
mod serial;
mod stream;
mod tcp;
mod websocket;

use crate::{
    PeerConnection, PeerEndpoint, PeerLink, PeerListener, PeerListenerCapabilities, PeerTransport,
};
use async_trait::async_trait;
use operit_host_api::HostManager::HostManager;
use std::sync::Arc;

pub struct HostPeerLink {
    webServers: http::ServerRegistry,
    maxMessageBytes: usize,
}

impl Default for HostPeerLink {
    fn default() -> Self { Self { webServers: Default::default(), maxMessageBytes: crate::DEFAULT_MAX_PEER_MESSAGE_BYTES } }
}

impl HostPeerLink {
    /// Resource policy belongs to the embedding app, not the OS or transport.
    pub fn withMaxMessageBytes(maxMessageBytes: usize) -> Result<Self, String> {
        if maxMessageBytes == 0 || maxMessageBytes > (u32::MAX as usize).saturating_sub(12) {
            return Err("Peer message byte limit is outside the framing range".into());
        }
        Ok(Self { maxMessageBytes, ..Self::default() })
    }

    /// Derives each inbound transport from its installed Host operation contract.
    pub fn listenerCapabilities(host: &HostManager) -> PeerListenerCapabilities {
        let mut transports = Vec::new();
        if let Some(server) = &host.httpServerHost {
            transports.push(PeerTransport::Http);
            if server.supportsWebSocketUpgrade() {
                transports.push(PeerTransport::WebSocket);
            }
        }
        if host.tcpHost.is_some() {
            transports.push(PeerTransport::Tcp);
        }
        // A serial endpoint is a single point-to-point Link carrier.
        // Host operation opens it for outbound connections and for the device
        // side's long-lived accept loop.
        if host.serialPortHost.is_some() {
            transports.push(PeerTransport::Serial);
        }
        if host
            .bluetoothHost
            .as_ref()
            .is_some_and(|host| host.supportsClassicListening())
        {
            transports.push(PeerTransport::Bluetooth);
        }
        PeerListenerCapabilities {
            transports,
            discoveryAdvertisement: host
                .serviceDiscoveryHost
                .as_ref()
                .is_some_and(|host| host.supportsAdvertisement()),
        }
    }
}

#[async_trait]
impl PeerLink for HostPeerLink {
    fn exclusiveEndpointKey(&self, target: &PeerEndpoint, transport: PeerTransport) -> Option<String> {
        match transport {
            PeerTransport::Serial => Some(format!("serial:{}", serial::endpointKey(&target.address))),
            _ => None,
        }
    }

    async fn connect(
        &self,
        host: Arc<HostManager>,
        source: PeerEndpoint,
        target: PeerEndpoint,
        transport: PeerTransport,
    ) -> Result<Arc<dyn PeerConnection>, String> {
        match transport {
            PeerTransport::Tcp => tcp::connect(&host, source, target, self.maxMessageBytes).await,
            PeerTransport::Serial => serial::connect(&host, source, target, self.maxMessageBytes).await,
            PeerTransport::Http => http::connect(&host, source, target, self.maxMessageBytes).await,
            PeerTransport::WebSocket => websocket::connect(&host, source, target, self.maxMessageBytes).await,
            PeerTransport::Bluetooth => bluetooth::connect(&host, source, target, self.maxMessageBytes).await,
        }
    }
    /// Rejects unsupported inbound transports before acquiring any Host resource.
    async fn listen(
        &self,
        host: Arc<HostManager>,
        source: PeerEndpoint,
        transport: PeerTransport,
    ) -> Result<Arc<dyn PeerListener>, String> {
        if !Self::listenerCapabilities(&host)
            .transports
            .contains(&transport)
        {
            return Err(format!(
                "Host does not support {transport:?} peer listeners"
            ));
        }
        match transport {
            PeerTransport::Tcp => tcp::listen(&host, source, self.maxMessageBytes).await,
            PeerTransport::Serial => serial::listen(&host, source, self.maxMessageBytes).await,
            PeerTransport::Http => http::listen(&self.webServers, &host, source, self.maxMessageBytes).await,
            PeerTransport::WebSocket => websocket::listen(&self.webServers, &host, source, self.maxMessageBytes).await,
            PeerTransport::Bluetooth => bluetooth::listen(&host, source, self.maxMessageBytes).await,
        }
    }
}

pub(crate) use stream::serialFrameCounters;
