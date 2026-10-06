#![allow(non_snake_case)]
//! Transport-only contract for carrying standard Link operations between runtimes.
//! No pairing, authorization, routing, persistence or global Host lookup.

use async_trait::async_trait;
use operit_host_api::HostManager::HostManager;
use operit_link::{CoreLinkRequest, CoreLinkResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
mod transport;
pub use transport::HostPeerLink;

/// Default resource ceiling for ordinary application embeddings.
pub const DEFAULT_MAX_PEER_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// Selects a Host transport, not a different application protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PeerTransport {
    Http,
    WebSocket,
    Tcp,
    Serial,
    Bluetooth,
}

/// Enumerates independently available inbound peer transports and LAN discovery.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerListenerCapabilities {
    pub transports: Vec<PeerTransport>,
    pub discoveryAdvertisement: bool,
}

/// A runtime-supplied identity and transport address (URL, socket address or device address).
/// The node ID is addressing metadata, never proof of identity or authorization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerEndpoint {
    pub nodeId: String,
    pub address: String,
}

/// Local API union only: untagged serialization preserves the existing Link operation envelope.
/// Requests and responses retain the existing Call / Watch / Push protocol.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum PeerMessage {
    Request(CoreLinkRequest),
    Response(CoreLinkResponse),
}

/// One bidirectional connection. Runtime consumes messages and decides how to dispatch them.
/// Implementations must preserve message order and partial input across receive cancellation.
#[async_trait]
pub trait PeerConnection: Send + Sync {
    fn source(&self) -> &PeerEndpoint;
    fn target(&self) -> &PeerEndpoint;
    fn transport(&self) -> PeerTransport;
    /// Exclusive carriers need one reusable authenticated session, not a new
    /// connection for each operation. This is an I/O property, not permission.
    fn requiresSessionReuse(&self) -> bool { false }
    /// 实际接入来源；不是客户端自报身份/地址。
    fn remoteAddress(&self) -> Option<std::net::SocketAddr> { None }

    async fn send(&self, message: PeerMessage) -> Result<(), String>;
    /// None means the connection has ended, not that no message is currently available.
    async fn receive(&self) -> Result<Option<PeerMessage>, String>;
    /// Idempotent; wakes pending I/O. Dropping a connection must also release Host resources.
    async fn close(&self);
}

/// Accepts connections only; accepting never implies successful authentication or pairing.
#[async_trait]
pub trait PeerListener: Send + Sync {
    /// Accepted connections use source = local endpoint, target = remote endpoint.
    /// None means the listener has closed. Remote identity remains untrusted runtime input.
    async fn accept(&self) -> Result<Option<Arc<dyn PeerConnection>>, String>;
    /// Stops acceptance; already accepted connections have their own lifetime.
    /// Dropping a listener must also release its Host resources.
    async fn close(&self);
}

/// 传输契约；I/O 使用显式传入的 Host。
#[async_trait]
pub trait PeerLink: Send + Sync {
    /// Connections sharing this key compete for the same exclusive endpoint.
    /// None preserves independent connections for ordinary network transports.
    fn exclusiveEndpointKey(&self, _target: &PeerEndpoint, _transport: PeerTransport) -> Option<String> { None }

    async fn connect(
        &self,
        host: Arc<HostManager>,
        source: PeerEndpoint,
        target: PeerEndpoint,
        transport: PeerTransport,
    ) -> Result<Arc<dyn PeerConnection>, String>;

    /// Only the local endpoint is needed to listen; the remote endpoint arrives on accept.
    async fn listen(
        &self,
        host: Arc<HostManager>,
        source: PeerEndpoint,
        transport: PeerTransport,
    ) -> Result<Arc<dyn PeerListener>, String>;
}

/// Valid frames, CRC errors, noise bytes and expired partial UART frames.
pub fn serialFrameCounters() -> [u32; 4] { transport::serialFrameCounters() }
