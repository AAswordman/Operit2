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

/// Stable transport failure for one encoded message above the connection ceiling.
/// Callers classify this as a capacity limit rather than an authentication fault.
pub const MESSAGE_TOO_LARGE_ERROR: &str = "peer message exceeds limit";

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
#[derive(Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PeerMessage {
    Request(CoreLinkRequest),
    Response(CoreLinkResponse),
}

impl PeerMessage {
    /// Decodes the declared message direction once, without speculative request decoding.
    pub fn decode(bytes: &[u8]) -> Result<Self, operit_link::CoreLinkCodecError> {
        match MessageDirection::read(bytes)? {
            MessageDirection::Request => operit_link::decodeLinkRequest(bytes).map(Self::Request),
            MessageDirection::Response => operit_link::decodeLinkResponse(bytes).map(Self::Response),
        }
    }

    /// Retains binary arguments directly from the single owned transport frame.
    pub fn decodeOwned(
        bytes: operit_link::CoreBytes,
    ) -> Result<Self, operit_link::CoreLinkCodecError> {
        match MessageDirection::read(&bytes)? {
            MessageDirection::Request => operit_link::decodeLinkRequestOwned(bytes).map(Self::Request),
            MessageDirection::Response => operit_link::decodeLinkResponseOwned(bytes).map(Self::Response),
        }
    }
}

/// Selects the existing wire schema from its exact response field without speculative decoding.
enum MessageDirection {
    Request,
    Response,
}

impl MessageDirection {
    /// Reads envelope structure with a constant-stack MessagePack field scan.
    fn read(bytes: &[u8]) -> Result<Self, operit_link::CoreLinkCodecError> {
        let body = operit_link::codec::linkMapField(bytes, "body")?
            .ok_or_else(|| operit_link::CoreLinkCodecError::Decode("Missing Link body".into()))?;
        Ok(match operit_link::codec::linkMapField(body, "result")? {
            Some(_) => Self::Response,
            None => Self::Request,
        })
    }
}

/// One bidirectional connection. Runtime consumes messages and decides how to dispatch them.
/// Implementations must preserve message order and partial input across receive cancellation.
#[async_trait]
pub trait PeerConnection: Send + Sync {
    fn source(&self) -> &PeerEndpoint;
    fn target(&self) -> &PeerEndpoint;
    fn transport(&self) -> PeerTransport;
    /// 实际接入来源；不是客户端自报身份/地址。
    fn remoteAddress(&self) -> Option<std::net::SocketAddr> {
        None
    }

    async fn send(&self, message: PeerMessage) -> Result<(), String>;
    /// None means the connection has ended, not that no message is currently available.
    async fn receive(&self) -> Result<Option<PeerMessage>, String>;
    /// Idempotent; wakes pending I/O. Dropping a connection must also release Host resources.
    async fn close(&self);
}

/// Accepts connections only; accepting never implies successful authentication or pairing.
#[async_trait]
pub trait PeerListener: Send + Sync {
    /// Actual bound socket for network listeners, including the Host-selected port.
    /// Non-network transports have no socket address.
    fn localAddress(&self) -> Option<String> { None }
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
pub fn serialFrameCounters() -> [u32; 4] {
    transport::serialFrameCounters()
}

/// One throwaway TCP connection classifies which Link carrier an endpoint
/// speaks, so pairing can start from a bare `ip:port` without a user-picked
/// transport.
///
/// A single WebSocket-upgrade probe separates all three carriers: HTTP and
/// WS listeners answer `HTTP/…` — `101` only when the upgrade is accepted —
/// while a framed TCP receiver drops the unreadable length header without
/// writing any bytes. The probe socket is closed before the real transport
/// connects; a framed receiver only counts one discarded bogus frame.
pub async fn detectTransport(
    host: std::sync::Arc<HostManager>,
    socket: &str,
) -> Result<PeerTransport, String> {
    let provider = host.tcpHost.as_ref().ok_or("TCP Host is not installed")?;
    let probe = provider.connect(socket).await.map_err(|e| e.to_string())?;
    let verdict = classifyEndpoint(probe.clone(), socket).await;
    probe.close().await;
    verdict
}

/// Reads exactly enough of the probe response to pick a carrier.
async fn classifyEndpoint(
    probe: std::sync::Arc<dyn operit_host_api::Tcp::TcpConnection>,
    socket: &str,
) -> Result<PeerTransport, String> {
    // Fixed RFC 6455 sample key: a conforming server treats the probe as an
    // ordinary handshake candidate; uniqueness is irrelevant here.
    let hostHeader = socket.split('/').next().unwrap_or(socket);
    let request = format!(
        "GET /link HTTP/1.1\r\nHost: {hostHeader}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    probe
        .write(request.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let prefix = b"HTTP/";
    let mut head: Vec<u8> = Vec::new();
    loop {
        let chunk =
            match tokio::time::timeout(std::time::Duration::from_secs(5), probe.read()).await {
                Ok(result) => result.map_err(|e| e.to_string())?,
                Err(_) => return Err(format!("transport detection on {socket} timed out")),
            };
        let Some(bytes) = chunk else {
            // Closed without a byte: an HTTP server always answers, so this
            // is a framed receiver discarding the bogus length header.
            return Ok(PeerTransport::Tcp);
        };
        head.extend_from_slice(&bytes);
        if head.len() < prefix.len() {
            if head.as_slice() != &prefix[..head.len()] {
                return Ok(PeerTransport::Tcp);
            }
            continue;
        }
        if !head.starts_with(prefix) {
            return Ok(PeerTransport::Tcp);
        }
        // "HTTP/1.1 " is nine bytes; three status digits complete the verdict.
        if head.len() >= 12 {
            let status = std::str::from_utf8(&head[9..12]).unwrap_or("");
            return Ok(if status == "101" {
                PeerTransport::WebSocket
            } else {
                PeerTransport::Http
            });
        }
    }
}
#[cfg(test)]
mod message_decode_tests {
    use super::*;
    use operit_link::*;

    /// Preserves each existing request and response envelope, including shared binary payloads.
    #[test]
    fn direct_decode_preserves_call_watch_push_envelopes_and_binary_payloads() {
        let messages = vec![
            PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(
                "cipher",
                "$peer.session",
                "data",
                CoreValue::Bytes(vec![0xa5; 8192].into()),
            ))),
            PeerMessage::Request(CoreLinkRequest::Watch(CoreLinkWatchRequest::Close {
                requestId: CoreRequestId::new("watch"),
            })),
            PeerMessage::Request(CoreLinkRequest::Push(CoreLinkPushRequestMessage::Close {
                pushId: "push".into(),
            })),
            PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse::err(
                CoreRequestId::new("call"),
                CoreLinkError::new("DENIED", "denied"),
            ))),
            PeerMessage::Response(CoreLinkResponse::Watch {
                requestId: CoreRequestId::new("watch"),
                result: Ok(CoreLinkWatchResponse::Opened),
            }),
            PeerMessage::Response(CoreLinkResponse::Push {
                pushId: "push".into(),
                result: Ok(CoreLinkPushResponse::ItemAccepted { sequence: 7 }),
            }),
        ];
        for message in messages {
            let encoded = encodeLink(&message).unwrap();
            assert_eq!(PeerMessage::decode(&encoded).unwrap(), message);
            assert_eq!(PeerMessage::decodeOwned(encoded.into()).unwrap(), message);
        }
        assert!(PeerMessage::decode(&[0xc1]).is_err());
        assert!(PeerMessage::decode(&encodeLink(CoreValue::Null).unwrap()).is_err());
    }
}
