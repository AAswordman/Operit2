//! 生产节点通信：唯一的配对/鉴权入口，所有 I/O 委托 PeerLink 和 Host。
//! 未鉴权只接受 hello/authorize；已鉴权 Call/Watch/Push 交给 Router。
use crate::{
    NodeServices::*,
    PeerRouter::PeerRouter,
    PeerStateStore::{PeerStateStore, StoredInbound, StoredOutbound, PAIRING_SERVICE_VERSION},
    RuntimePeerService::RuntimePeerService,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use operit_host_api::{HostManager::HostManager, TimeUtils::currentTimeMillis};
use operit_link::*;
use operit_peer_link::{HostPeerLink, PeerConnection, PeerLink, PeerListener, PeerMessage};
use operit_util::RuntimeStorageLayout::*;
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, sync::{Arc, Mutex, Weak}};
use tokio::sync::{broadcast, Mutex as AsyncMutex};
#[path = "peer/crypto.rs"] mod crypto;
use crypto::{Channel, MultiplexedChannel, error};
#[path = "peer/heartbeat.rs"] mod heartbeat;
#[path = "peer/dispatch.rs"] mod dispatch;
#[path = "peer/space_client.rs"] mod space_client;
#[path = "peer/availability.rs"] mod availability;
#[path = "peer/pairing_throttle.rs"] mod pairing_throttle;
use pairing_throttle::{pairingSource, PairingThrottle};
const HANDSHAKE: &str = "$peer.pairing";

/// Pairing handshake protocol. Version 2 replaces the old
/// `HMAC(root, id||code)` confirmation, which an active man in the middle could
/// verify offline, with a SPAKE2 exchange keyed by the six-digit code. Stored
/// session credentials stay version 1: only new pairings negotiate this.
const PAIRING_PROTOCOL_VERSION: u32 = 2;
/// Rejects a pairing whose peer still speaks the offline-verifiable protocol.
const PAIRING_PROTOCOL_UPGRADE_REQUIRED: &str =
    "Both devices must run a version with the SPAKE2 pairing protocol";
/// Confirmation guesses allowed for one pending pairing.
const PAIRING_MAX_ATTEMPTS: u8 = 5;

const PAIRING_LIFETIME_MS: i64 = 300_000;
/// Establishing one authenticated connection is bounded here rather than by the
/// caller, so a dial to an unreachable peer becomes an error for every business
/// call, watch and push instead of holding whoever asked for it.
const CONNECT_DEADLINE_MS: u64 = 30_000;
/// Resolves the pairing carrier when the caller left it open: an explicit URL
/// scheme implies its carrier, a bare `ip:port` is classified by one throwaway
/// probe connection (see `operit_peer_link::detectTransport`).
async fn resolvePairingTransport(
    host: &std::sync::Arc<operit_host_api::HostManager::HostManager>,
    address: &str,
) -> Result<PeerTransport, String> {
    if let Some((scheme, _)) = address.split_once("://") {
        return match scheme {
            // TLS is selected by the URL passed to the Host, not a separate carrier.
            "http" | "https" => Ok(PeerTransport::Http),
            "ws" | "wss" => Ok(PeerTransport::WebSocket),
            "tcp" => Ok(PeerTransport::Tcp),
            other => Err(format!("unsupported link address scheme: {other}")),
        };
    }
    let socket = address.split('/').next().unwrap_or(address);
    operit_peer_link::detectTransport(host.clone(), socket).await
}

/// Upgrades a bare socket address to the carrier's URL form; explicit URLs and
/// non-network carriers (serial, bluetooth) pass through unchanged.
fn pairingAddressFor(address: &str, transport: PeerTransport) -> String {
    if address.contains("://") {
        return address.to_string();
    }
    match transport {
        PeerTransport::Http => format!("http://{address}/link"),
        PeerTransport::WebSocket => format!("ws://{address}/link"),
        _ => address.to_string(),
    }
}

/// One operator-supplied dial address in its stored endpoint form.
struct OperatorEndpoint {
    rendered: String,
    /// A scheme implies its carrier; a bare `host:port` keeps the record's.
    transport: Option<PeerTransport>,
}

/// Parses an operator-supplied network endpoint, preserving its scheme, host, port, and path.
fn parseOperatorEndpoint(input: &str) -> Result<OperatorEndpoint, CoreLinkError> {
    let value = input.trim();
    if value.is_empty() {
        return Err(error("Endpoint address is empty"));
    }
    if let Some((scheme, rest)) = value.split_once("://") {
        let transport = match scheme {
            "http" | "https" => PeerTransport::Http,
            "ws" | "wss" => PeerTransport::WebSocket,
            "tcp" => PeerTransport::Tcp,
            other => return Err(error(&format!("Unsupported endpoint scheme '{other}': use ws://, wss://, http://, https:// or tcp://"))),
        };
        if transport == PeerTransport::Tcp {
            // Bare-carrier endpoints carry no path; the authority is the address.
            let (host, port) = splitEndpointHostPort(rest)?;
            return Ok(OperatorEndpoint { rendered: format!("tcp://{host}:{port}"), transport: Some(transport) });
        }
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (rest, "/link".to_string()),
        };
        let (host, port) = splitEndpointHostPort(authority)?;
        return Ok(OperatorEndpoint {
            rendered: format!("{scheme}://{host}:{port}{path}"),
            transport: Some(transport),
        });
    }
    let (host, port) = splitEndpointHostPort(value)?;
    Ok(OperatorEndpoint { rendered: format!("{host}:{port}"), transport: None })
}

/// Splits one `host:port` authority, keeping bracketed IPv6 hosts verbatim.
fn splitEndpointHostPort(authority: &str) -> Result<(String, u16), CoreLinkError> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, rest) = rest.split_once(']')
            .ok_or_else(|| error("Endpoint host is missing a closing bracket"))?;
        let port = rest.strip_prefix(':').ok_or_else(|| error("Endpoint port is missing"))?;
        return Ok((format!("[{host}]"), parseEndpointPort(port)?));
    }
    let (host, port) = authority.rsplit_once(':')
        .ok_or_else(|| error("Endpoint port is missing: use host:port"))?;
    if host.is_empty() {
        return Err(error("Endpoint host is missing"));
    }
    Ok((host.to_string(), parseEndpointPort(port)?))
}

/// Parses a positive port number for an operator-supplied endpoint.
fn parseEndpointPort(port: &str) -> Result<u16, CoreLinkError> {
    let port = port.parse::<u16>().map_err(|_| error("Endpoint port is not a valid port number"))?;
    if port == 0 { return Err(error("Endpoint port must be positive")); }
    Ok(port)
}

#[cfg(test)]
mod pairing_transport_tests {
    use super::*;

    /// Explicit URLs must work on client-only hosts and keep their TLS scheme,
    /// default/custom port, proxy path and query for pairing and later redials.
    #[tokio::test]
    async fn explicit_urls_select_carrier_without_probing_or_rewriting() {
        let host = Arc::new(HostManager::default());
        for (address, expected) in [
            ("http://peer.local:37195/link", PeerTransport::Http),
            ("https://peer.example/link", PeerTransport::Http),
            ("https://peer.example:8443/operit/link?device=remote", PeerTransport::Http),
            ("https://[fd00::1]:8443/link", PeerTransport::Http),
            ("ws://peer.local:37195/link", PeerTransport::WebSocket),
            ("wss://peer.example/link", PeerTransport::WebSocket),
            ("wss://peer.example:8443/operit/link?device=remote", PeerTransport::WebSocket),
            ("wss://[fd00::1]:8443/link", PeerTransport::WebSocket),
            ("tcp://peer.local:37195", PeerTransport::Tcp),
        ] {
            let transport = resolvePairingTransport(&host, address).await.unwrap();
            assert_eq!(transport, expected, "{address}");
            assert_eq!(pairingAddressFor(address, transport), address);
        }
    }

    #[tokio::test]
    async fn unsupported_url_scheme_is_rejected_without_probing() {
        let host = Arc::new(HostManager::default());
        assert_eq!(
            resolvePairingTransport(&host, "ftp://peer.example/link").await.unwrap_err(),
            "unsupported link address scheme: ftp",
        );
    }
}

/// Last transport/session failure only; no credentials or message contents.
pub fn peerSessionDiagnostic() -> Option<String> { crypto::diagnostic() }

#[derive(Clone, Serialize, Deserialize)]
struct Pending {
    version: u32, id: String, clientDeviceId: String, peerNodeId: String,
    endpoint: String, transport: PeerTransport, info: LinkDeviceInfo,
    root: Vec<u8>, expires: i64, attempts: u8,
    #[serde(default)] confirmationCode: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Hello {
    version: u32, nodeId: String, expectedNodeId: String, info: LinkDeviceInfo,
    public: Vec<u8>, purpose: String, sessionId: String,
    duplex: bool,
    /// Pairing handshake protocol. Version 2 requires SPAKE2 confirmation;
    /// older peers omit it and may not create new pairings.
    #[serde(default, skip_serializing_if = "Option::is_none")] pairingProtocol: Option<u32>,
}
#[derive(Serialize, Deserialize)]
struct HelloReply {
    transcript: crypto::Transcript, info: LinkDeviceInfo, duplex: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")] pairingProtocol: Option<u32>,
}
#[derive(Serialize, Deserialize)]
struct Authorization {
    keyProof: Vec<u8>,
    /// SPAKE2 message for a `finish` handshake. It is only processed after the
    /// sender proved knowledge of the pairing root, so unauthenticated traffic
    /// cannot make this device derive a PAKE key.
    #[serde(default, skip_serializing_if = "Option::is_none")] spake: Option<Vec<u8>>,
}
#[derive(Serialize, Deserialize)]
struct Authorized {
    proof: Vec<u8>,
    pairingId: String,
    /// SPAKE2 message for the receiver side of a `finish` handshake.
    #[serde(default, skip_serializing_if = "Option::is_none")] spake: Option<Vec<u8>>,
    /// Proof that the receiver derived the same code-authenticated secret.
    #[serde(default, skip_serializing_if = "Option::is_none")] spakeProof: Option<Vec<u8>>,
}

/// Resource policy chosen by the embedding app, independent of OS/transport.
#[derive(Clone, Copy)]
pub struct PeerRuntimeLimits {
    pub incomingSessions: usize,
    pub pendingPairings: usize,
    pub concurrentProbes: usize,
    pub maxMessageBytes: usize,
}
impl Default for PeerRuntimeLimits {
    fn default() -> Self { Self { incomingSessions: 64, pendingPairings: 32, concurrentProbes: 4, maxMessageBytes: operit_peer_link::DEFAULT_MAX_PEER_MESSAGE_BYTES } }
}
impl PeerRuntimeLimits {
    pub fn constrained() -> Self { Self { incomingSessions: 2, pendingPairings: 2, concurrentProbes: 1, maxMessageBytes: 8 * 1024 } }
}

struct State {
    limits: PeerRuntimeLimits,
    host: Arc<HostManager>,
    link: HostPeerLink,
    store: PeerStateStore,
    nodeId: String,
    info: LinkDeviceInfo,
    router: Weak<dyn PeerRouter>,
    listeners: AsyncMutex<BTreeMap<PeerTransport, Arc<dyn PeerListener>>>,
    connections: Mutex<BTreeMap<String, Vec<Weak<dyn PeerConnection>>>>,
    channels: Mutex<BTreeMap<String, Weak<MultiplexedChannel>>>,
    advertisements: Mutex<Vec<Box<dyn operit_host_api::ServiceDiscovery::DiscoveryAdvertisement>>>,
    slots: Arc<tokio::sync::Semaphore>,
    active: Mutex<BTreeSet<String>>, changes: broadcast::Sender<()>,
    availability: Mutex<Option<availability::AvailabilityWorker>>,
    lifecycle: AsyncMutex<()>,
    connectLocks: Mutex<BTreeMap<String, Weak<AsyncMutex<()>>>>,
    /// Cross-session budgets for unauthenticated pairing work.
    pairingThrottle: Mutex<PairingThrottle>,
    /// 本节点的配对/撤销持久化操作串行化，不持锁执行网络 I/O。
    mutation: Mutex<()>,
}
#[derive(Clone)]
pub struct HostRuntimePeerService { state: Arc<State> }
impl HostRuntimePeerService {
    /// Creates shared peer services and starts availability checks on the supplied Host scheduler.
    pub fn new<R>(
        host: Arc<HostManager>,
        router: &Arc<R>,
        info: LinkDeviceInfo,
    ) -> Result<Arc<Self>, String>
    where
        R: PeerRouter + 'static,
    {
        Self::newWithLimits(host, router, info, PeerRuntimeLimits::default())
    }
    pub fn newWithLimits<R: PeerRouter + 'static>(
        host: Arc<HostManager>, router: &Arc<R>, info: LinkDeviceInfo, limits: PeerRuntimeLimits,
    ) -> Result<Arc<Self>, String> {
        if limits.incomingSessions == 0 || limits.pendingPairings == 0 || limits.concurrentProbes == 0 {
            return Err("Peer runtime limits must be positive".into());
        }
        let link = HostPeerLink::withMaxMessageBytes(limits.maxMessageBytes)?;
        let storage = host
            .runtimeStorageHost
            .clone()
            .ok_or("Runtime storage Host is not installed")?;
        let service = Arc::new(Self {
            state: Arc::new(State {
                limits,
                host,
                link,
                store: PeerStateStore::new(storage),
                nodeId: router.localNodeId(),
                info,
                router: Arc::downgrade(&(router.clone() as Arc<dyn PeerRouter>)),
                listeners: AsyncMutex::new(BTreeMap::new()),
                connections: Mutex::new(BTreeMap::new()),
                channels: Mutex::new(BTreeMap::new()),
                advertisements: Mutex::new(Vec::new()),
                slots: Arc::new(tokio::sync::Semaphore::new(limits.incomingSessions)),
                active: Mutex::new(BTreeSet::new()),
                changes: broadcast::channel(32).0,
                availability: Mutex::new(None),
                lifecycle: AsyncMutex::new(()),
                connectLocks: Mutex::new(BTreeMap::new()),
                pairingThrottle: Mutex::new(PairingThrottle::default()),
                mutation: Mutex::new(()),
            }),
        });
        service.startAvailabilityWorker().map_err(|error| error.to_string())?;
        Ok(service)
    }
    fn router(&self) -> Result<Arc<dyn PeerRouter>, CoreLinkError> {
        self.state.router.upgrade().ok_or_else(|| CoreLinkError::new("PEER_ROUTER_CLOSED", "Peer router was released"))
    }
    fn connectionLock(&self, key: &str) -> Arc<AsyncMutex<()>> {
        let mut locks = self.state.connectLocks.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        locks.retain(|_, lock| lock.strong_count() != 0);
        if let Some(lock) = locks.get(key).and_then(Weak::upgrade) { return lock; }
        let lock = Arc::new(AsyncMutex::new(()));
        locks.insert(key.to_owned(), Arc::downgrade(&lock));
        lock
    }
    fn changed(&self) { let _ = self.state.changes.send(()); }
    fn track(&self, node: &str, raw: &Arc<dyn PeerConnection>) {
        let mut connections = self.state.connections.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let list = connections.entry(node.into()).or_default();
        list.retain(|c| c.strong_count() != 0); list.push(Arc::downgrade(raw));
    }
    /// Loads one owned confirmation transaction without copying it into callers.
    fn pending(&self, inbound: bool, id: &str) -> Result<Box<Pending>, CoreLinkError> {
        let path = if inbound { RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH } else { RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH };
        let p = self.pendingRecords(path)?.remove(id)
            .ok_or_else(|| error("Pairing transaction not found"))?;
        if p.expires <= currentTimeMillis() { self.state.store.deleteRecord(path, id).map_err(error)?; return Err(error("Pairing transaction expired")); }
        Ok(Box::new(p))
    }
    fn inboundCredential(&self, sessionId: &str) -> Result<Option<StoredInbound>, CoreLinkError> {
        self.state.store.record(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, sessionId).map_err(error)
    }
    fn pendingRecords(&self, path: &str) -> Result<BTreeMap<String, Pending>, CoreLinkError> {
        self.state.store.versionedRecords(path, PAIRING_SERVICE_VERSION).map_err(error)
    }

    fn outbound(&self, node: &str) -> Result<StoredOutbound, CoreLinkError> {
        self.state.store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).map_err(error)?
            .into_values().find(|c| c.peerNodeId == node && c.pairingServiceVersion == PAIRING_SERVICE_VERSION)
            .ok_or_else(|| CoreLinkError::new("PEER_OUTBOUND_NOT_AUTHORIZED", "No current outbound authorization for node"))
    }
    /// Rejects an existing device authorization in either pairing direction.
    fn ensureDeviceUnpaired(&self, peerNodeId: &str) -> Result<(), CoreLinkError> {
        if self.state.store.pairedPeers(&self.state.nodeId).map_err(error)?.iter().any(|peer| peer.nodeId == peerNodeId) {
            return Err(CoreLinkError::new("PEER_ALREADY_PAIRED", "Device is already paired"));
        }
        Ok(())
    }
    /// Credential cleanup shared by upstream local unpair and confirmed renewal.
    /// The caller holds mutation, so cancellation cannot interleave with writes.
    fn forgetPairingRecordsLocked(&self, nodeIds: &[String]) -> Result<(), CoreLinkError> {
        for nodeId in nodeIds {
            self.state.store.removePairedPeer(nodeId).map_err(error)?;
        }
        Ok(())
    }
    async fn raw(&self, target: PeerEndpoint, transport: PeerTransport) -> Result<Arc<dyn PeerConnection>, CoreLinkError> {
        self.state.link.connect(self.state.host.clone(), PeerEndpoint { nodeId: self.state.nodeId.clone(), address: String::new() }, target, transport).await.map_err(error)
    }
    /// Exchanges ephemeral keys and verifies identity-bound proofs over every transport.
    ///
    /// `pairingCode` is present only for the `finish` handshake. It turns the
    /// six-digit code into a SPAKE2 password instead of an offline-verifiable
    /// MAC message. On success the returned secret is the SPAKE2-derived session
    /// key; for `start` it remains the X25519 root stored in the pending record.
    async fn handshake(&self, raw: Arc<dyn PeerConnection>, purpose: &str, sessionId: &str,
        saved: Option<&[u8]>, pairingCode: Option<&str>,
    ) -> Result<(Arc<Channel>, String, LinkDeviceInfo, Vec<u8>, String), CoreLinkError> {
        let (private, public) = crypto::ephemeral()?;
        let hello = Hello { version: PAIRING_SERVICE_VERSION, nodeId: self.state.nodeId.clone(),
            expectedNodeId: raw.target().nodeId.clone(),
            info: self.state.store.announcedDeviceInfo(&self.state.info), public: public.clone(),
            purpose: purpose.into(), sessionId: sessionId.into(), duplex: true,
            pairingProtocol: Some(PAIRING_PROTOCOL_VERSION) };
        let reply: HelloReply = rawCall(&raw, "hello", &hello).await?;
        if !reply.duplex { return Err(error("Peer must support one bidirectional Link channel")); }
        let t = &reply.transcript;
        if t.version != PAIRING_SERVICE_VERSION || t.clientNodeId != self.state.nodeId || t.clientPublic != public
            || (!raw.target().nodeId.is_empty() && t.serverNodeId != raw.target().nodeId)
            || t.serverNodeId == self.state.nodeId || t.serverPublic.len() != 32 || t.challenge.len() != 32
            || (!sessionId.is_empty() && t.sessionId != sessionId) {
            return Err(error("Handshake identity/transcript mismatch"));
        }
        if matches!(purpose, "start" | "finish")
            && reply.pairingProtocol != Some(PAIRING_PROTOCOL_VERSION) {
            return Err(CoreLinkError::new(
                "PAIRING_PROTOCOL_UPGRADE_REQUIRED",
                PAIRING_PROTOCOL_UPGRADE_REQUIRED,
            ));
        }
        let context = crypto::transcript(t)?;
        let dh = crypto::agree(private, &t.serverPublic)?;
        let root = crypto::derive(&dh, &context, b"operit-pairing-root-v1")?;
        let key = match saved { Some(saved) => crypto::derive(&dh, saved, &context)?, None => root };
        let finishing = purpose == "finish";
        let spake = if finishing {
            let code = pairingCode.ok_or_else(|| error("Pairing confirmation code is required"))?;
            let startRoot = saved.ok_or_else(|| error("Pairing confirmation root is missing"))?;
            let (clientId, serverId) = crypto::pairingIdentities(&context);
            let (state, message) = crypto::spakeStart(true, code.as_bytes(), &clientId, &serverId)?;
            let state = (state, startRoot.to_vec());
            (Some(state), Some(message))
        } else {
            (None, None)
        };
        let accepted: Authorized = rawCall(&raw, "authorize", &Authorization {
            keyProof: crypto::proof(&key, &context, b"client"),
            spake: spake.1,
        }).await?;
        crypto::verify(&key, &context, b"server", &accepted.proof)?;
        if accepted.pairingId != t.sessionId { return Err(error("Pairing correlation mismatch")); }
        let storedSecret = match spake.0 {
            Some((state, startRoot)) => {
                let serverMessage = accepted.spake.ok_or_else(|| CoreLinkError::new(
                    "PAIRING_PROTOCOL_UPGRADE_REQUIRED",
                    PAIRING_PROTOCOL_UPGRADE_REQUIRED,
                ))?;
                let spakeKey = crypto::spakeFinish(state, &serverMessage)?;
                let sessionSecret = crypto::pairingSessionSecret(&spakeKey, &startRoot, &context)?;
                let proof = accepted.spakeProof.ok_or_else(|| error("Pairing confirmation proof is missing"))?;
                crypto::verify(&sessionSecret, &context, b"server-spake", &proof)?;
                sessionSecret.to_vec()
            }
            None => root.to_vec(),
        };
        let id = t.sessionId.clone(); let peerNodeId = t.serverNodeId.clone();
        let info = reply.info;
        let channelKey = if finishing {
            let secret: [u8; 32] = storedSecret.as_slice().try_into().map_err(|_| error("Invalid pairing secret"))?;
            secret
        } else { key };
        let channel = Channel::new(raw, &channelKey, &context, true)?;
        Ok((channel, id, info, storedSecret, peerNodeId))
    }
    /// Returns the peer's only authenticated connection while it remains usable.
    fn peerChannel(&self, node: &str) -> Option<Arc<MultiplexedChannel>> {
        self.state.channels.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(node).and_then(Weak::upgrade).filter(|channel| !channel.isFailed())
    }

    /// Rejects a second authenticated carrier instead of distributing operations across sockets.
    fn registerChannel(&self, node: &str, channel: &Arc<MultiplexedChannel>) -> Result<(), CoreLinkError> {
        self.requirePeerConnectionAllowed(node)?;
        let mut channels = self.state.channels.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if channels.get(node).and_then(Weak::upgrade).is_some_and(|current| !current.isFailed()) {
            return Err(CoreLinkError::new("PEER_CHANNEL_EXISTS", "Peer already has an authenticated Link channel"));
        }
        channels.insert(node.into(), Arc::downgrade(channel));
        drop(channels);
        self.refreshPeerAvailability(node);
        Ok(())
    }

    /// Derives availability from the same channel used by every business operation.
    fn refreshPeerAvailability(&self, node: &str) {
        let available = self.requirePeerConnectionAllowed(node).is_ok() && self.hasLiveChannel(node);
        let mut active = self.state.active.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let changed = if available { active.insert(node.into()) } else { active.remove(node) };
        drop(active);
        if changed { self.changed(); }
    }

    /// Reports whether the peer's sole carrier can still carry traffic in either direction.
    fn hasLiveChannel(&self, node: &str) -> bool { self.peerChannel(node).is_some() }

    /// Updates the sole outbound pairing record under the caller's mutation lock.
    fn updateOutboundEndpointLocked(
        &self, node: &str, parsed: &OperatorEndpoint, transport: Option<PeerTransport>,
    ) -> Result<(), CoreLinkError> {
        let records = self.state.store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).map_err(error)?;
        let (id, record) = records.iter().find(|(_, record)|
            record.peerNodeId == node && record.pairingServiceVersion == PAIRING_SERVICE_VERSION)
            .ok_or_else(|| CoreLinkError::new("PEER_OUTBOUND_NOT_AUTHORIZED", "No current outbound authorization for node"))?;
        let mut record = record.clone();
        if let Some(transport) = parsed.transport.or(transport) { record.transport = transportName(transport).into(); }
        record.endpoint = parsed.rendered.clone();
        self.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, id, &record).map_err(error)
    }

    /// Starts the same authenticated reader and dispatcher for every Host transport.
    async fn startPeerChannel(&self, node: &str, channel: Arc<Channel>, pairingId: String,
        authorization: dispatch::Authorization,
    ) -> Result<Arc<MultiplexedChannel>, CoreLinkError> {
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.clone()
            .ok_or_else(|| error("Host scheduler is not installed"))?;
        let state = Arc::downgrade(&self.state);
        let peer = node.to_owned();
        let inbound = dispatch::InboundSide::new(self.clone(), node.into(), pairingId, authorization);
        let channel = MultiplexedChannel::start(channel, scheduler, inbound, move || {
            if let Some(state) = state.upgrade() { HostRuntimePeerService { state }.refreshPeerAvailability(&peer); }
        })?;
        if let Err(error) = self.registerChannel(node, &channel) { channel.close().await; return Err(error); }
        Ok(channel)
    }

    /// Completes one outbound pairing on the Host peer worker, preserving transaction boundaries.
    async fn confirmOutboundPairing(&self, id: &str, code: &str) -> Result<PairedPeer, CoreLinkError> {
        let pending = self.beginPairingConfirmation(id, code)?;
        let raw = self.raw(PeerEndpoint {
            nodeId: pending.peerNodeId.clone(), address: pending.endpoint.clone(),
        }, pending.transport).await?;
        let confirmation = self.confirmPairing(&raw, id, code, &pending.root);
        // A USB UART cannot signal EOF after the remote handshake rejects.
        let result = if pending.transport == PeerTransport::Serial {
            match tokio::time::timeout(std::time::Duration::from_secs(20), confirmation).await {
                Ok(result) => result,
                Err(_) => Err(error("Serial pairing confirmation timed out")),
            }
        } else { confirmation.await };
        raw.close().await;
        self.commitPairingConfirmation(id, pending, result?)
    }

    /// Reserves one confirmation attempt under the durable pairing mutation lock.
    fn beginPairingConfirmation(&self, id: &str, code: &str) -> Result<Box<Pending>, CoreLinkError> {
        let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut pending = self.pending(false, id)?;
        self.ensureDeviceUnpaired(&pending.peerNodeId)?;
        if pending.attempts >= PAIRING_MAX_ATTEMPTS {
            return Err(error("Confirmation attempt limit reached"));
        }
        pending.attempts += 1;
        self.state.store.putRecord(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, id, &pending).map_err(error)?;
        if code.len() != 6 || !code.bytes().all(|c| c.is_ascii_digit()) {
            return Err(error("Enter the six-digit pairing code shown on the receiving device"));
        }
        Ok(pending)
    }

    /// Proves the receiver's code and obtains its acknowledgement of the commit marker.
    async fn confirmPairing(&self, raw: &Arc<dyn PeerConnection>, id: &str, code: &str, root: &[u8]) -> Result<Vec<u8>, CoreLinkError> {
        let (channel, _, _, secret, _) = self.handshake(raw.clone(), "finish", id, Some(root), Some(code)).await?;
        let requestId = CoreRequestId::new(uuid::Uuid::new_v4().to_string());
        let request = CoreCallRequest::new(requestId.0.clone(), HANDSHAKE, "finish",
            toCoreValue(crypto::proof(&secret, id.as_bytes(), b"finish")).map_err(|e| error(e.to_string()))?);
        channel.send(PeerMessage::Request(CoreLinkRequest::Call(request))).await?;
        match channel.receive().await? {
            Some(PeerMessage::Response(CoreLinkResponse::Call(response))) if response.requestId == requestId => {
                response.result?;
                Ok(secret)
            }
            _ => Err(error("Confirmation response mismatch")),
        }
    }

    /// Publishes the credential only while the original transaction is still authorized.
    fn commitPairingConfirmation(&self, id: &str, pending: Box<Pending>, secret: Vec<u8>) -> Result<PairedPeer, CoreLinkError> {
        {
            let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            self.pending(false, id)?;
            self.ensureDeviceUnpaired(&pending.peerNodeId)?;
            self.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, id, &StoredOutbound {
                endpoint: pending.endpoint, sessionId: id.into(), deviceId: self.state.nodeId.clone(), peerNodeId: pending.peerNodeId.clone(),
                peerDeviceInfo: pending.info.clone(), pairingServiceVersion: PAIRING_SERVICE_VERSION,
                sessionSecret: BASE64.encode(&secret), transport: transportName(pending.transport).into(),
            }).map_err(error)?;
            self.state.store.deleteRecord(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, id).map_err(error)?;
            self.state.active.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(pending.peerNodeId.clone());
        }
        self.changed();
        Ok(PairedPeer { nodeId: pending.peerNodeId, displayName: pending.info.displayName(),
            inbound: false, outbound: true, pairingIncarnation: pending.info.pairingIncarnation.clone() })
    }

    /// Acquires the shared channel while keeping transport establishment on its Host async task.
    pub(super) async fn acquireChannel(&self, node: &str) -> Result<Arc<MultiplexedChannel>, CoreLinkError> {
        self.requirePeerConnectionAllowed(node)?;
        if let Some(channel) = self.peerChannel(node) { return Ok(channel); }
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.as_ref()
            .ok_or_else(|| error("Host scheduler is not installed"))?;
        let service = self.clone();
        let node = node.to_owned();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        scheduler.scheduleHostRuntimeAsyncTask("peer-connect", Box::new(move || Box::pin(async move {
            let result = service.establishChannel(&node).await;
            let _ = sender.send(result);
        }))).map_err(|e| error(e.to_string()))?;
        receiver.await.map_err(|_| error("Peer connection task ended without a result"))?
    }

    /// Executes authenticated transport work on the Host network worker.
    async fn callOnHost(&self, node: &str, mut request: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
        let id = request.payload.requestId.clone();
        // Relayed origins may reuse IDs; correlation belongs to this outbound hop.
        let wireId = CoreRequestId(operit_link::nextCoreRouteRequestId("peer-call"));
        request.payload.requestId = wireId.clone();
        let result = match self.acquireChannel(node).await {
            Ok(channel) => match dispatch::routedCall(request) {
                Ok(wire) => match channel.exchange(CoreLinkRequest::Call(wire)).await {
                    Ok(CoreLinkResponse::Call(response)) if response.requestId == wireId => Ok(response.result),
                    Ok(_) => Err(error("Call response mismatch")),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        if result.as_ref().err().is_some_and(failureProvesPeerUnavailable) {
            self.refreshPeerAvailability(node);
        }
        // Remote business errors prove a working authenticated link, not an offline peer.
        CoreCallResponse { requestId: id, result: result.and_then(|remote| remote) }
    }

    /// Serializes one authenticated establishment per peer under its connection deadline.
    async fn establishChannel(&self, node: &str) -> Result<Arc<MultiplexedChannel>, CoreLinkError> {
        tokio::time::timeout(std::time::Duration::from_millis(CONNECT_DEADLINE_MS), async {
            let lock = self.connectionLock(node);
            let _connect = lock.lock().await;
            self.requirePeerConnectionAllowed(node)?;
            if let Some(channel) = self.peerChannel(node) { return Ok(channel); }
            let record = self.outbound(node)?;
            let root = BASE64.decode(&record.sessionSecret).map_err(|_| error("Invalid stored credential"))?;
            let raw = self.raw(PeerEndpoint { nodeId: node.into(), address: record.endpoint }, parseTransport(&record.transport)?).await?;
            let result = self.handshake(raw.clone(), "session", &record.sessionId, Some(&root), None).await;
            let (channel, _, _, _, _) = match result { Ok(result) => result, Err(error) => { raw.close().await; return Err(error); } };
            self.requirePeerConnectionAllowed(node)?;
            self.track(node, &raw);
            self.startPeerChannel(node, channel, record.sessionId, dispatch::Authorization::Outbound).await
        }).await.map_err(|_| CoreLinkError::new("PEER_CONNECT_TIMEOUT", "Authenticated connection establishment timed out"))?
    }

    /// Counts the sole authenticated carrier for connection lifecycle tests.
    #[cfg(test)]
    pub(crate) async fn channelCount(&self, node: &str) -> usize { usize::from(self.hasLiveChannel(node)) }

    async fn readHandshake(&self, raw: &Arc<dyn PeerConnection>, method: &str) -> Result<CoreCallRequest, CoreLinkError> {
        let delay = self.state.host.hostRuntimeTaskSchedulerHost.as_ref().ok_or_else(|| error("Host scheduler missing"))?.waitForHostRuntimeDelay(15_000);
        tokio::select! { r = readCall(raw, method) => r, _ = delay => Err(error("Unauthenticated handshake deadline reached")) }
    }
    async fn serve(&self, raw: Arc<dyn PeerConnection>) -> Result<(), CoreLinkError> {
        let helloRequest = match self.readHandshake(&raw, "hello").await {
            Ok(request) => request,
            // Serial initialization may deliberately finish an abandoned
            // stream before sending a fresh handshake on the next UART lease.
            Err(e) if e.code == "PEER_CONNECTION_CLOSED" => return Ok(()),
            Err(e) => return Err(e),
        };
        let hello: Hello =
            fromCoreValue(helloRequest.args).map_err(|e| error(e.to_string()))?;
        if !hello.duplex { return sendError(&raw, helloRequest.requestId, error("Peer must support one bidirectional Link channel")).await; }
        if hello.version != PAIRING_SERVICE_VERSION
            || hello.nodeId.is_empty()
            || hello.nodeId == self.state.nodeId
            || (!hello.expectedNodeId.is_empty() && hello.expectedNodeId != self.state.nodeId)
            || !matches!(hello.purpose.as_str(), "start" | "finish" | "session") {
            return sendError(&raw, helloRequest.requestId, error("Invalid pairing identity/purpose")).await;
        }
        if hello.purpose == "session" {
            if let Err(revoked) = self.requirePeerConnectionAllowed(&hello.nodeId) {
                // 回报明确的撤销原因；静默关闭只会留下连接已结束的错误。
                return sendError(&raw, helloRequest.requestId, revoked).await;
            }
        }
        if matches!(hello.purpose.as_str(), "start" | "finish")
            && hello.pairingProtocol != Some(PAIRING_PROTOCOL_VERSION) {
            return sendError(&raw, helloRequest.requestId, CoreLinkError::new(
                "PAIRING_PROTOCOL_UPGRADE_REQUIRED", PAIRING_PROTOCOL_UPGRADE_REQUIRED)).await;
        }
        // 沿用持久化的 sessionId/sessionSecret；配对事务仍受过期时间和尝试上限保护。
        let saved = match hello.purpose.as_str() {
            "start" => None,
            "finish" => match self.pending(true, &hello.sessionId) {
                Ok(p) if p.clientDeviceId == hello.nodeId => Some(p.root),
                Ok(_) => return sendError(&raw, helloRequest.requestId, error("Pairing identity mismatch")).await,
                Err(e) => return sendError(&raw, helloRequest.requestId, e).await,
            },
            _ => {
                let record = self.inboundCredential(&hello.sessionId)?.ok_or_else(|| error("Inbound authorization not found"))?;
                if record.deviceId != hello.nodeId || record.pairingServiceVersion != PAIRING_SERVICE_VERSION { return Err(error("Inbound identity/version mismatch")); }
                Some(BASE64.decode(&record.sessionSecret).map_err(|_| error("Invalid stored credential"))?)
            },
        };
        // A six-digit code is only safe when guessing it costs an online
        // exchange. A finish token is charged only after its pending record
        // checks out, so random ids cannot drain the global budget.
        let source = pairingSource(&raw);
        let allowed = {
            let mut throttle = self.state.pairingThrottle.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            match hello.purpose.as_str() {
                "start" => throttle.allowCreation(&source, currentTimeMillis()),
                "finish" => throttle.allowConfirmation(&source, currentTimeMillis()),
                _ => true,
            }
        };
        if !allowed {
            return sendError(&raw, helloRequest.requestId, CoreLinkError::new(
                "PAIRING_RATE_LIMITED", "Too many pairing attempts; wait before trying again")).await;
        }
        let (private, public) = crypto::ephemeral()?;
        let id = if hello.purpose == "start" { uuid::Uuid::new_v4().to_string() } else { hello.sessionId.clone() };
        let transcript = crypto::Transcript { version: PAIRING_SERVICE_VERSION, sessionId: id.clone(),
            clientNodeId: hello.nodeId.clone(), serverNodeId: self.state.nodeId.clone(),
            clientPublic: hello.public, serverPublic: public, challenge: crypto::random()?.to_vec() };
        let context = crypto::transcript(&transcript)?;
        let dh = crypto::agree(private, &transcript.clientPublic)?;
        let root = crypto::derive(&dh, &context, b"operit-pairing-root-v1")?;
        let key = match &saved { Some(saved) => crypto::derive(&dh, saved, &context)?, None => root };
        sendValue(&raw, helloRequest.requestId, HelloReply { transcript, duplex: true,
            info: self.state.store.announcedDeviceInfo(&self.state.info),
            pairingProtocol: Some(PAIRING_PROTOCOL_VERSION) }).await?;
        let request = self.readHandshake(&raw, "authorize").await?;
        let authorization: Authorization = fromCoreValue(request.args).map_err(|e| error(e.to_string()))?;
        // The X25519 proof authenticates this channel and costs no guess, so
        // reject it before a confirmation attempt is recorded.
        if let Err(e) = crypto::verify(&key, &context, b"client", &authorization.keyProof) {
            return sendError(&raw, request.requestId, e).await;
        }
        let mut confirmation = None;
        let admission = (|| {
            match hello.purpose.as_str() {
                "start" => {
                    let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    // The initiator may have forgotten this one-way pairing. A new
                    // challenge is allowed, but must not revoke the old credential.
                    self.state.store.putPending(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH, &id, &Pending {
                        version: PAIRING_SERVICE_VERSION, id: id.clone(), clientDeviceId: hello.nodeId.clone(), peerNodeId: self.state.nodeId.clone(),
                        endpoint: String::new(), transport: raw.transport(), info: hello.info.clone(),
                        root: root.to_vec(), expires: currentTimeMillis() + PAIRING_LIFETIME_MS, attempts: 0,
                        confirmationCode: Some(crypto::pairingCode()?),
                    }, currentTimeMillis(), self.state.limits.pendingPairings).map_err(error)?;
                    self.changed();
                }
                "finish" => {
                    let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    let mut pending = self.pending(true, &id)?;
                    if pending.attempts >= PAIRING_MAX_ATTEMPTS {
                        return Err(error("Confirmation attempt limit reached"));
                    }
                    // Persist the guess before running the PAKE so a crash or a
                    // concurrent confirmation cannot refund attempts.
                    pending.attempts += 1;
                    self.state.store.putRecord(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH, &id, &pending).map_err(error)?;
                    let code = pending.confirmationCode.as_deref()
                        .ok_or_else(|| error("Receiver confirmation missing"))?;
                    let clientMessage = authorization.spake.as_deref().ok_or_else(|| CoreLinkError::new(
                        "PAIRING_PROTOCOL_UPGRADE_REQUIRED", PAIRING_PROTOCOL_UPGRADE_REQUIRED))?;
                    let (clientId, serverId) = crypto::pairingIdentities(&context);
                    let (state, message) = crypto::spakeStart(false, code.as_bytes(), &clientId, &serverId)?;
                    let spakeKey = crypto::spakeFinish(state, clientMessage)?;
                    let sessionSecret = crypto::pairingSessionSecret(&spakeKey, &pending.root, &context)?;
                    confirmation = Some((message, sessionSecret));
                }
                _ => {},
            }
            Ok(())
        })();
        if let Err(e) = admission { return sendError(&raw, request.requestId, e).await; }
        let (spakeMessage, sessionSecret) = match confirmation {
            Some((message, secret)) => (Some(message), Some(secret)),
            None => (None, None),
        };
        sendValue(&raw, request.requestId, Authorized {
            proof: crypto::proof(&key, &context, b"server"),
            pairingId: id.clone(),
            spake: spakeMessage,
            spakeProof: sessionSecret.as_ref().map(|secret| crypto::proof(secret, &context, b"server-spake")),
        }).await?;
        if hello.purpose == "start" { return Ok(()); }
        // The confirmation channel is keyed by the PAKE secret bound to the code
        // and transcript, not by the X25519 root an active relay could learn.
        let channelKey = sessionSecret.unwrap_or(key);
        let channel = Channel::new(raw.clone(), &channelKey, &context, false)?;
        if hello.purpose == "finish" {
            let Some(PeerMessage::Request(CoreLinkRequest::Call(request))) = channel.receive().await? else { return Err(error("Encrypted confirmation Call required")); };
            if request.target != HANDSHAKE || request.methodName != "finish" { return Err(error("Confirmation required before business")); }
            let proof: Vec<u8> = fromCoreValue(request.args).map_err(|e| error(e.to_string()))?;
            let sessionSecret = sessionSecret.ok_or_else(|| error("Pairing confirmation secret is missing"))?;
            let mut replacing = false;
            let result = (|| {
                let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                let pending = self.pending(true, &id)?;
                // Only a peer that entered the same code derives this key; a
                // wrong guess fails here after the attempt was already counted.
                crypto::verify(&sessionSecret, id.as_bytes(), b"finish", &proof)?;
                let record = StoredInbound { deviceId: hello.nodeId.clone(), deviceInfo: hello.info.clone(),
                    pairingServiceVersion: PAIRING_SERVICE_VERSION, sessionSecret: BASE64.encode(&sessionSecret) };
                // A first pairing follows the upstream confirmation path.
                // Only a retained pairing needs revocation, after its new code
                // is verified. Reuse the same cleanup as explicit local unpair.
                match self.ensureDeviceUnpaired(&hello.nodeId) {
                    Ok(()) => {},
                    Err(e) if e.code == "PEER_ALREADY_PAIRED" => {
                        replacing = true;
                        self.forgetPairingRecordsLocked(std::slice::from_ref(&hello.nodeId))?;
                    }
                    Err(e) => return Err(e),
                }
                self.state.store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, &id, &record).map_err(error)?;
                self.state.store.deleteRecord(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH, &id).map_err(error)?;
                Ok(CoreValue::Null)
            })();
            if replacing {
                // The confirmation channel is intentionally not tracked yet;
                // disconnect only old business channels before acknowledging.
                self.disconnectPeer(&hello.nodeId).await?;
            }
            if result.is_ok() {
                // A verified confirmation also proves this peer is online.
                self.state.active.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(hello.nodeId.clone());
                self.changed();
            }
            channel.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse { requestId: request.requestId, result }))).await?;
            return Ok(());
        }
        self.requirePeerConnectionAllowed(&hello.nodeId)?;
        self.track(&hello.nodeId, &raw);
        if let Some(previous) = self.peerChannel(&hello.nodeId) { previous.close().await; }
        let channel = self.startPeerChannel(&hello.nodeId, channel, id, dispatch::Authorization::Inbound).await?;
        channel.waitStopped().await;
        Ok(())
    }
}

fn failureProvesPeerUnavailable(error: &CoreLinkError) -> bool {
    error.code != "PEER_OUTBOUND_NOT_AUTHORIZED"
}

async fn readCall(raw: &Arc<dyn PeerConnection>, method: &str) -> Result<CoreCallRequest, CoreLinkError> {
    match raw.receive().await.map_err(error)? {
        Some(PeerMessage::Request(CoreLinkRequest::Call(r))) if r.target == HANDSHAKE && r.methodName == method => Ok(r),
        None => Err(CoreLinkError::new("PEER_CONNECTION_CLOSED", "Peer ended the stream before handshake")),
        _ => Err(error("Unauthenticated operation is not in the pairing whitelist")),
    }
}
async fn rawCall<T: Serialize, R: serde::de::DeserializeOwned>(raw: &Arc<dyn PeerConnection>, method: &str, args: &T) -> Result<R, CoreLinkError> {
    let id = CoreRequestId::new(uuid::Uuid::new_v4().to_string());
    raw.send(PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(id.0.clone(), HANDSHAKE, method,
        toCoreValue(args).map_err(|e| error(e.to_string()))?)))).await.map_err(error)?;
    let response = if raw.transport() == PeerTransport::Serial {
        tokio::time::timeout(std::time::Duration::from_secs(5), raw.receive()).await
            .map_err(|_| error("Serial pairing response timed out"))?
    } else { raw.receive().await };
    match response.map_err(error)? {
        Some(PeerMessage::Response(CoreLinkResponse::Call(r))) if r.requestId == id => fromCoreValue(r.result?).map_err(|e| error(e.to_string())),
        None => Err(CoreLinkError::new("PEER_CONNECTION_CLOSED", "Peer ended the stream before handshake")),
        _ => Err(error("Pairing response correlation mismatch")),
    }
}
async fn sendValue(raw: &Arc<dyn PeerConnection>, id: CoreRequestId, value: impl Serialize) -> Result<(), CoreLinkError> {
    raw.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse::ok(id, toCoreValue(value).map_err(|e| error(e.to_string()))?))))
        .await.map_err(error)
}
async fn sendError(raw: &Arc<dyn PeerConnection>, id: CoreRequestId, e: CoreLinkError) -> Result<(), CoreLinkError> {
    raw.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse::err(id, e)))).await.map_err(error)
}
fn transportName(t: PeerTransport) -> &'static str { match t {
    PeerTransport::Http => "http", PeerTransport::WebSocket => "ws", PeerTransport::Tcp => "tcp",
    PeerTransport::Serial => "serial", PeerTransport::Bluetooth => "bluetooth",
} }
fn parseTransport(t: &str) -> Result<PeerTransport, CoreLinkError> { match t {
    "http" => Ok(PeerTransport::Http), "ws" => Ok(PeerTransport::WebSocket), "tcp" => Ok(PeerTransport::Tcp),
    "serial" => Ok(PeerTransport::Serial), "bluetooth" => Ok(PeerTransport::Bluetooth), _ => Err(error("Unknown transport")),
} }
#[async_trait(?Send)]
impl RuntimePeerService for HostRuntimePeerService {
    fn spaceClient(&self) -> Option<Arc<dyn CoreLinkSharedClient + Send + Sync>> { space_client::spaceClient(self) }
    fn spaceConnection(&self) -> Option<super::NodeServices::SpaceClientConnection> { space_client::spaceConnection(self) }
    /// Reports the ceiling the Host app installed, not a protocol-wide constant.
    fn maxMessageBytes(&self) -> usize { self.state.limits.maxMessageBytes }
    /// Returns only unpaired candidates using the current state after discovery completes.
    async fn discoverPeers(&self, timeoutMs: u64) -> Result<Vec<DiscoveredPeer>, CoreLinkError> {
        let host = self.state.host.serviceDiscoveryHost.clone().ok_or_else(|| error("Discovery Host is not installed"))?;
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.as_ref().ok_or_else(|| error("Host scheduler is not installed"))?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        scheduler.scheduleHostRuntimeTask("peer-discovery", Box::new(move || {
            let result = host.discover("_operit-link._tcp.local.", timeoutMs); let _ = tx.send(result);
        })).map_err(|e| error(e.to_string()))?;
        let records = rx.await.map_err(|_| error("Discovery cancelled"))?.map_err(|e| error(e.to_string()))?;
        let pairedNodeIds = self.state.store.pairedPeers(&self.state.nodeId).map_err(error)?
            .into_iter().map(|peer| peer.nodeId).collect::<BTreeSet<_>>();
        let recordCount = records.len();
        let peers = discoveredPeers(records, &self.state.nodeId, &pairedNodeIds);
        operit_util::AppLogger::AppLogger::i("RuntimePeerService", &format!(
            "LAN discovery completed records={recordCount} paired={} candidates={}", pairedNodeIds.len(), peers.len()));
        Ok(peers)
    }
    async fn pairingAddresses(&self) -> Result<Vec<String>, CoreLinkError> {
        // Snapshot actual listeners, never a saved wildcard or a requested port zero.
        let listeners = self.state.listeners.lock().await;
        let sockets = listeners.iter().filter_map(|(transport, listener)| {
            isNetworkTransport(*transport).then(|| listener.localAddress()
                .and_then(|address| address.parse::<std::net::SocketAddr>().ok())
                .map(|socket| (*transport, socket))).flatten()
        }).collect::<Vec<_>>();
        drop(listeners);
        let interfaces = if sockets.iter().any(|(_, socket)| socket.ip().is_unspecified()) {
            self.state.host.serviceDiscoveryHost.as_ref()
                .ok_or_else(|| error("Host cannot enumerate listener addresses"))?
                .localAddresses().map_err(|failure| error(failure.to_string()))?
        } else { Vec::new() };
        crate::PeerConnectionCode::listenerAddresses(&sockets, &interfaces)
    }

    /// Checks the known identity and the handshake result before creating a pairing transaction.
    async fn startPairing(&self, target: PeerEndpoint, transport: Option<PeerTransport>) -> Result<PendingPairing, CoreLinkError> {
        if !target.nodeId.is_empty() {
            let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            self.ensureDeviceUnpaired(&target.nodeId)?;
        }
        // A bare `ip:port` input pairs without a user-picked carrier: classify
        // the endpoint once, then normalize the address for the winner so the
        // pending record stores the same endpoint a later reconnect would dial.
        let transport = match transport {
            Some(transport) => transport,
            None => resolvePairingTransport(&self.state.host, &target.address)
                .await
                .map_err(error)?,
        };
        let target = PeerEndpoint { address: pairingAddressFor(&target.address, transport), ..target };
        // Opening a USB-UART can reset an ESP32 through the carrier's modem
        // control lines. Retry the serial handshake after that boot window;
        // network transports keep the original single-attempt behavior.
        let attempts = if transport == PeerTransport::Serial { 3 } else { 1 };
        let mut paired = None;
        for attempt in 0..attempts {
            let raw = self.raw(target.clone(), transport).await?;
            let result = if transport == PeerTransport::Serial {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(20),
                    self.handshake(raw.clone(), "start", "", None, None),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(error("Serial pairing handshake timed out")),
                }
            } else {
                self.handshake(raw.clone(), "start", "", None, None).await
            };
            raw.close().await;
            match result {
                Ok(value) => {
                    paired = Some(Ok(value));
                    break;
                }
                Err(error) if attempt + 1 < attempts => continue,
                Err(error) => {
                    paired = Some(Err(error));
                    break;
                }
            }
        }
        let (_, id, info, root, node) = paired.ok_or_else(|| error("Pairing did not start"))??;
        {
            let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            self.ensureDeviceUnpaired(&node)?;
            self.state
                .store
                .putPending(
                    RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH,
                    &id,
                    &Pending {
                        version: PAIRING_SERVICE_VERSION,
                        id: id.clone(),
                        clientDeviceId: self.state.nodeId.clone(),
                        peerNodeId: node.clone(),
                        endpoint: target.address,
                        transport,
                        info: info.clone(),
                        root,
                        expires: currentTimeMillis() + PAIRING_LIFETIME_MS,
                        attempts: 0,
                        confirmationCode: None,
                    },
                    currentTimeMillis(),
                    self.state.limits.pendingPairings,
                )
                .map_err(error)?;
        }
        self.changed(); Ok(PendingPairing { pairingId: id, peerNodeId: node, displayName: info.displayName() })
    }
    /// Runs confirmation on the Host async worker without nesting network polling in a Space task.
    async fn finishPairing(&self, id: &str, code: &str) -> Result<PairedPeer, CoreLinkError> {
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.as_ref()
            .ok_or_else(|| error("Host scheduler is not installed"))?;
        let service = self.clone();
        let id = id.to_owned();
        let code = code.to_owned();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        scheduler.scheduleHostRuntimeAsyncTask("peer-confirm", Box::new(move || Box::pin(async move {
            let result = service.confirmOutboundPairing(&id, &code).await;
            let _ = sender.send(result);
        }))).map_err(|e| error(e.to_string()))?;
        receiver.await.map_err(|_| error("Peer confirmation task ended without a result"))?
    }
    async fn cancelPairing(&self, id: &str) -> Result<(), CoreLinkError> {
        let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        self.state.store.deleteRecord(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, id).map_err(error)?;
        self.changed(); Ok(())
    }
    /// Reports the operation capabilities supplied by this node's actual Host providers.
    fn listenerCapabilities(&self) -> operit_peer_link::PeerListenerCapabilities {
        HostPeerLink::listenerCapabilities(&self.state.host)
    }

    /// Validates every requested transport before opening the selected listeners.
    async fn startListening(&self, transports: &[PeerTransport]) -> Result<(), CoreLinkError> {
        let capabilities = self.listenerCapabilities();
        for transport in transports {
            if !capabilities.transports.contains(transport) {
                return Err(error(format!("Host does not support {transport:?} peer listeners")));
            }
        }
        let _lifecycle = self.state.lifecycle.lock().await;
        use crate::PeerStateStore::PeerHostPortMode;
        let mut config = self.state.store.hostConfig().map_err(error)?.ok_or_else(|| error("Configure bindAddress/transports before listening"))?;
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.clone().ok_or_else(|| error("Host scheduler is not installed"))?;
        let mut listeners = self.state.listeners.lock().await;
        let requested: BTreeSet<_> = listeners.keys().copied().chain(transports.iter().copied()).collect();
        if requested.contains(&PeerTransport::Tcp) && (requested.contains(&PeerTransport::Http) || requested.contains(&PeerTransport::WebSocket)) {
            return Err(error("TCP cannot share a listener port with HTTP/WebSocket"));
        }
        let mut opened = BTreeMap::<PeerTransport, Arc<dyn PeerListener>>::new();
        let previousAddress = config.bindAddress.clone();
        let result = async {
            for transport in transports {
                if listeners.contains_key(transport) || opened.contains_key(transport) { continue; }
                let mut socket = if isNetworkTransport(*transport) {
                    let socket = config.bindAddress.parse::<std::net::SocketAddr>()
                        .map_err(|_| error("Listener requires an IP address and port"))?;
                    if config.portMode == PeerHostPortMode::Fixed && socket.port() == 0 {
                        return Err(error("Fixed listener port must be nonzero"));
                    }
                    Some(socket)
                } else { None };
                let endpoint = PeerEndpoint { nodeId: self.state.nodeId.clone(), address: config.bindAddress.clone() };
                let listener = match self.state.link.listen(self.state.host.clone(), endpoint.clone(), *transport).await {
                    Ok(listener) => listener,
                    Err(message) if config.portMode == PeerHostPortMode::Automatic
                        && socket.is_some_and(|socket| socket.port() != 0)
                        && !listeners.keys().chain(opened.keys()).any(|t| isNetworkTransport(*t))
                        && isAddressInUse(&message) => {
                        // Reuse the last assigned port when available, preserving
                        // reconnects without LAN discovery. A collision delegates
                        // allocation to the Host once, with no port scanning.
                        let socket = socket.as_mut().unwrap();
                        socket.set_port(0);
                        self.state.link.listen(self.state.host.clone(),
                            PeerEndpoint { address: socket.to_string(), ..endpoint }, *transport)
                            .await.map_err(error)?
                    }
                    Err(message) => return Err(error(message)),
                };
                opened.insert(*transport, listener.clone());
                if isNetworkTransport(*transport) {
                    config.bindAddress = listener.localAddress()
                        .ok_or_else(|| error("Network listener did not report its bound address"))?;
                }
            }
            let mut advertisements = Vec::new();
            if config.discoveryEnabled && capabilities.discoveryAdvertisement && requested.iter().any(|t| isNetworkTransport(*t)) {
                let host = self.state.host.serviceDiscoveryHost.as_ref().ok_or_else(|| error("Discovery enabled but Host not installed"))?;
                let socket: std::net::SocketAddr = config.bindAddress.parse().map_err(|_| error("Discovery requires a concrete network bindAddress"))?;
                let properties = [("nodeId".into(), self.state.nodeId.clone()), ("displayName".into(), self.state.info.displayName()),
                    ("transports".into(), requested.iter().filter(|t| isNetworkTransport(**t)).map(|t| transportName(*t)).collect::<Vec<_>>().join(","))].into();
                if !opened.is_empty() || self.state.advertisements.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).is_empty() {
                    advertisements.push(host.advertise(operit_host_api::ServiceDiscovery::ServiceAdvertisement {
                        serviceType: "_operit-link._tcp.local.".into(), instance: self.state.nodeId.clone(),
                        hostname: format!("{}.local.", self.state.nodeId), port: socket.port(), properties,
                    }).map_err(|e| error(e.to_string()))?);
                }
            }
            for listener in opened.values() {
                let service = self.clone(); let accepting = listener.clone(); let tasks = scheduler.clone();
                scheduler.scheduleHostRuntimeAsyncTask("peer-listen", Box::new(move || Box::pin(async move {
                    while let Ok(Some(raw)) = accepting.accept().await {
                        let Ok(permit) = service.state.slots.clone().try_acquire_owned() else { raw.close().await; continue; };
                        service.track("", &raw);
                        let service = service.clone(); let connection = raw.clone();
                        if tasks.scheduleHostRuntimeAsyncTask("peer-connection", Box::new(move || Box::pin(async move {
                            let _permit = permit;
                            let result = service.serve(connection.clone()).await;
                            if let Err(e) = result { operit_util::AppLogger::AppLogger::w("RuntimePeerService", &e.to_string()); }
                            connection.close().await;
                        }))).is_err() { raw.close().await; }
                    }
                }))).map_err(|e| error(e.to_string()))?;
            }
            if config.bindAddress != previousAddress {
                config.updatedAt = currentTimeMillis();
                self.state.store.saveHostConfig(&config).map_err(error)?;
                operit_util::AppLogger::AppLogger::i("RuntimePeerService", &format!("Listener bound to {}", config.bindAddress));
            }
            Ok::<_, CoreLinkError>(advertisements)
        }.await;
        match result {
            Ok(advertisements) => {
                listeners.extend(opened);
                if !advertisements.is_empty() { *self.state.advertisements.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = advertisements; }
                self.startAvailabilityWorker()?;
                self.changed(); Ok(())
            }
            Err(e) => {
                // Do not leave partially started transports behind on failure.
                for listener in opened.into_values() { listener.close().await; }
                Err(e)
            }
        }
    }
    async fn stop(&self) -> Result<(), CoreLinkError> {
        let _lifecycle = self.state.lifecycle.lock().await;
        self.stopAvailabilityWorker().await;
        self.state.advertisements.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear();
        let listeners = std::mem::take(&mut *self.state.listeners.lock().await);
        for listener in listeners.into_values() { listener.close().await; }
        let channels = std::mem::take(&mut *self.state.channels.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
        for channel in channels.into_values().filter_map(|channel| channel.upgrade()) { channel.close().await; }
        let connections = std::mem::take(&mut *self.state.connections.lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
        for raw in connections.into_values().flatten().filter_map(|v| v.upgrade()) { raw.close().await; }
        self.state.active.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clear(); self.changed(); Ok(())
    }
    /// Keeps transport authorization, exchange and reply decoding on the Host async worker.
    async fn call(&self, node: &str, request: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
        let id = request.payload.requestId.clone();
        let result = (|| {
            let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.as_ref()
                .ok_or_else(|| error("Host scheduler is not installed"))?;
            let service = self.clone();
            let node = node.to_owned();
            let (mut sender, receiver) = tokio::sync::oneshot::channel();
            scheduler.scheduleHostRuntimeAsyncTask("peer-call", Box::new(move || Box::pin(async move {
                tokio::select! {
                    biased;
                    _ = sender.closed() => {},
                    result = service.callOnHost(&node, request) => { let _ = sender.send(result); }
                }
            }))).map_err(|e| error(e.to_string()))?;
            Ok::<_, CoreLinkError>(receiver)
        })();
        match result {
            Ok(receiver) => match receiver.await {
                Ok(response) => response,
                Err(_) => CoreCallResponse::err(id, error("Peer call task ended without a result")),
            },
            Err(error) => CoreCallResponse::err(id, error),
        }
    }
    async fn watchSnapshot(&self, node: &str, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent, CoreLinkError> {
        dispatch::watchSnapshot(self, node, request).await
    }
    async fn watch(&self, node: &str, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError> {
        dispatch::watch(self, node, request).await
    }
    async fn openPush(&self, node: &str, request: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        dispatch::openPush(self, node, request).await
    }
    fn pairedPeers(&self) -> Result<Vec<PairedPeer>, CoreLinkError> { self.state.store.pairedPeers(&self.state.nodeId).map_err(error) }
    /// Saves an operator-supplied pairing endpoint and probes the peer's sole channel.
    async fn updatePeerEndpoint(
        &self, node: &str, endpoint: &str, transport: Option<PeerTransport>,
    ) -> Result<(), CoreLinkError> {
        let parsed = parseOperatorEndpoint(endpoint)?;
        {
            let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            self.updateOutboundEndpointLocked(node, &parsed, transport)?;
        }
        operit_util::AppLogger::AppLogger::i("RuntimePeerService", &format!(
            "Dial endpoint updated by operator peer={node} endpoint={}", parsed.rendered));
        let _ = self.acquireChannel(node).await;
        Ok(())
    }
    fn outboundPeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> { Ok(self.pairedPeers()?.into_iter().filter(|p| p.outbound).map(|p| p.nodeId).collect()) }
    fn activePeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> { Ok(self.state.active.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()) }
    fn subscribePeerChanges(&self) -> broadcast::Receiver<()> { self.state.changes.subscribe() }
    fn pairingPrompts(&self) -> Result<Vec<PairingPrompt>, CoreLinkError> {
        Ok(self.pendingRecords(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH)?.into_values()
            .filter(|p| p.expires > currentTimeMillis()).map(|p| PairingPrompt { pairingId: p.id,
                peerNodeId: p.clientDeviceId, displayName: p.info.displayName(), confirmationCode: p.confirmationCode.unwrap_or_default() }).collect())
    }
    async fn disconnectPeer(&self, node: &str) -> Result<(), CoreLinkError> {
        let channel = self.state.channels.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(node).and_then(|channel| channel.upgrade());
        if let Some(channel) = channel { channel.close().await; }
        let connections = self.state.connections.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(node).unwrap_or_default();
        for raw in connections.into_iter().filter_map(|v| v.upgrade()) { raw.close().await; }
        self.state.active.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).remove(node); self.changed(); Ok(())
    }
    async fn removePairedPeer(&self, node: &str) -> Result<(), CoreLinkError> {
        self.forgetPairingRecords(&[node.to_string()])?;
        self.disconnectPeer(node).await
    }

    fn forgetPairingRecords(&self, nodeIds: &[String]) -> Result<(), CoreLinkError> {
        if nodeIds.is_empty() { return Ok(()); }
        {
            let _guard = self.state.mutation.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            self.forgetPairingRecordsLocked(nodeIds)?;
        }
        self.changed();
        Ok(())
    }
}

/// Produces one addable candidate per unpaired node across all interfaces and transports.
fn discoveredPeers(records: Vec<operit_host_api::ServiceDiscovery::DiscoveredService>, localNodeId: &str,
    pairedNodeIds: &BTreeSet<String>,
) -> Vec<DiscoveredPeer> {
    let mut peers = BTreeMap::new();
    let (mut local, mut paired, mut invalid) = (0, 0, 0);
    for record in records {
        let [Some(node), Some(displayName), Some(modes)] = ["nodeId", "displayName", "transports"]
            .map(|key| record.properties.get(key).filter(|value| !value.is_empty())) else {
                invalid += 1;
                operit_util::AppLogger::AppLogger::w("RuntimePeerService", &format!(
                    "Ignoring incomplete LAN advertisement service={}", record.fullName));
                continue;
            };
        if node == localNodeId { local += 1; continue; }
        if pairedNodeIds.contains(node) { paired += 1; continue; }
        if record.port == 0 { invalid += 1; continue; }
        let mut valid = false;
        for mode in modes.split(',') {
            for ip in &record.addresses {
                if ip.is_unspecified() || ip.is_multicast()
                    || matches!(ip, std::net::IpAddr::V6(v6) if v6.is_unicast_link_local()) {
                    continue;
                }
                let socket = std::net::SocketAddr::new(*ip, record.port).to_string();
                let address = match mode { "http" => format!("http://{socket}/link"), "ws" => format!("ws://{socket}/link"), "tcp" => socket, _ => continue };
                let candidate = DiscoveredPeer { nodeId: node.clone(), address, displayName: displayName.clone() };
                valid = true;
                // One device, not one row per NIC, IP family and transport.
                let rank = discoveryEndpointRank(*ip, mode);
                if peers.get(node).is_none_or(|(old, _)| rank < *old) {
                    peers.insert(node.clone(), (rank, candidate));
                }
            }
        }
        if !valid { invalid += 1; }
    }
    operit_util::AppLogger::AppLogger::i("RuntimePeerService", &format!(
        "LAN discovery filtering local={local} paired={paired} invalid={invalid} candidates={}", peers.len()));
    peers.into_values().map(|(_, peer)| peer).collect()
}

/// Prefer LAN IPv4, then routable IPv6, with loopback only as a last resort.
fn discoveryEndpointRank(ip: std::net::IpAddr, mode: &str) -> (u8, u8, String) {
    let family = if ip.is_loopback() { 3 } else if ip.is_ipv4() { 0 } else { 1 };
    let transport = match mode { "http" => 0, "ws" => 1, _ => 2 };
    (family, transport, ip.to_string())
}

#[cfg(test)]
mod discovery_ranking_tests {
    use super::{discoveryEndpointRank, BTreeSet};
    /// Keeps different unpaired nodes distinct even when their display names match.
    #[test]
    fn groups_addresses_and_transports_by_node_not_display_name() {
        use operit_host_api::ServiceDiscovery::DiscoveredService;
        let record = |node: &str, addresses: &[&str]| DiscoveredService {
            fullName: "test".into(), hostname: "test.local.".into(), port: 37195,
            addresses: addresses.iter().map(|ip| ip.parse().unwrap()).collect(),
            properties: [("nodeId".into(), node.into()), ("displayName".into(), "Same name".into()),
                ("transports".into(), "ws,http,tcp".into())].into_iter().collect(),
        };
        let peers = super::discoveredPeers(vec![
            record("one", &["fd00::2", "fd00::3", "192.168.1.2", "fe80::1"]),
            record("one", &["192.168.1.2"]),
            record("two", &["fd00::4"]),
            record("self", &["192.168.1.9"]),
            record("invalid", &["::", "ff02::1", "fe80::2"]),
        ], "self", &BTreeSet::new());
        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].nodeId, "one");
        assert_eq!(peers[0].address, "http://192.168.1.2:37195/link");
        assert_eq!(peers[1].nodeId, "two");
        assert_eq!(peers[1].address, "http://[fd00::4]:37195/link");
    }
    /// Excludes all advertisements of paired nodes without excluding equal display names.
    #[test]
    fn excludes_paired_nodes_from_all_discovery_records() {
        use operit_host_api::ServiceDiscovery::DiscoveredService;
        let record = |node: &str, ip: &str| DiscoveredService {
            fullName: "test".into(), hostname: "test.local.".into(), port: 37195,
            addresses: vec![ip.parse().unwrap()],
            properties: [("nodeId".into(), node.into()), ("displayName".into(), "Same name".into()),
                ("transports".into(), "tcp,http,ws".into())].into_iter().collect(),
        };
        let paired = ["paired".to_string()].into_iter().collect();
        let records = vec![record("paired", "192.168.1.2"), record("paired", "fd00::2"), record("new", "192.168.1.3")];
        let peers = super::discoveredPeers(records.clone(), "self", &paired);
        assert_eq!(peers.iter().map(|peer| peer.nodeId.as_str()).collect::<Vec<_>>(), vec!["new"]);
        assert_eq!(super::discoveredPeers(records, "self", &BTreeSet::new()).len(), 2);
    }

    /// Preserves endpoint ordering for unpaired nodes.
    #[test]
    fn prefers_lan_ipv4_over_ipv6_and_loopback() {
        let rank = |ip: &str| discoveryEndpointRank(ip.parse().unwrap(), "http");
        assert!(rank("192.168.1.2") < rank("fd00::2"));
        assert!(rank("fd00::2") < rank("127.0.0.1"));
    }

    /// Rejects incomplete advertisements rather than inventing a transport or device name.
    #[test]
    fn rejects_incomplete_advertisements() {
        use operit_host_api::ServiceDiscovery::DiscoveredService;
        let record = DiscoveredService {
            fullName: "test".into(), hostname: "test.local.".into(), port: 37195,
            addresses: vec!["192.168.8.2".parse().unwrap()],
            properties: [("nodeId".into(), "peer".into()), ("displayName".into(), "Peer".into()),
                ("transports".into(), "http".into())].into(),
        };
        for key in ["nodeId", "displayName", "transports"] {
            let mut incomplete = record.clone();
            incomplete.properties.remove(key);
            assert!(super::discoveredPeers(vec![incomplete], "self", &BTreeSet::new()).is_empty());
        }
        let mut invalid = record;
        invalid.port = 0;
        assert!(super::discoveredPeers(vec![invalid], "self", &BTreeSet::new()).is_empty());
    }
}

/// Host bind failures currently cross the PeerLink boundary as strings.
fn isAddressInUse(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("address already in use") || message.contains("os error 48")
        || message.contains("os error 98") || message.contains("os error 10048")
}

fn isNetworkTransport(transport: PeerTransport) -> bool {
    matches!(transport, PeerTransport::Http | PeerTransport::WebSocket | PeerTransport::Tcp)
}

#[cfg(test)]
mod online_direction_tests {
    use super::*;
    #[test]
    fn inbound_peer_is_not_offline_just_because_reverse_pairing_is_missing() {
        let missing = CoreLinkError::new("PEER_OUTBOUND_NOT_AUTHORIZED", "No current outbound authorization for node");
        assert!(!failureProvesPeerUnavailable(&missing));
        assert!(failureProvesPeerUnavailable(&error("Connection refused")));
    }
}

#[cfg(test)]
mod serial_response_deadline_tests {
    use super::*;
    struct SilentPeer { endpoint: PeerEndpoint, sends: Arc<std::sync::atomic::AtomicUsize> }
    #[async_trait]
    impl PeerConnection for SilentPeer {
        fn source(&self) -> &PeerEndpoint { &self.endpoint }
        fn target(&self) -> &PeerEndpoint { &self.endpoint }
        fn transport(&self) -> PeerTransport { PeerTransport::Serial }
        async fn send(&self, _: PeerMessage) -> Result<(), String> {
            self.sends.fetch_add(1, std::sync::atomic::Ordering::SeqCst); Ok(())
        }
        async fn receive(&self) -> Result<Option<PeerMessage>, String> { std::future::pending().await }
        async fn close(&self) {}
    }
    #[tokio::test]
    async fn lost_serial_handshake_response_returns_an_error_without_retransmission() {
        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let raw: Arc<dyn PeerConnection> = Arc::new(SilentPeer {
            sends: sends.clone(),
            endpoint: PeerEndpoint { nodeId: "silent-board".into(), address: "test-uart".into() },
        });
        let response = tokio::time::timeout(std::time::Duration::from_secs(6),
            rawCall::<_, CoreValue>(&raw, "hello", &CoreValue::Null)).await
            .expect("handshake has its own bounded response deadline");
        assert!(response.unwrap_err().message.contains("Serial pairing response timed out"));
        assert_eq!(sends.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}


#[cfg(test)]
mod single_channel_tests {
    use super::*;
    use operit_host_api::{
        HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
        HostRuntimeTurnFuture,
    };
    use std::sync::Mutex as StdMutex;

    /// 只入队、不执行：被调度的任务永不运行，通道保持"可用"，测试线程上也不需要
    /// 任何 Tokio 运行时。
    #[derive(Default)]
    struct QueuedScheduler { tasks: StdMutex<Vec<HostRuntimeAsyncTask>> }
    impl operit_host_api::HostRuntimeTaskSchedulerHost for QueuedScheduler {
        fn monotonicTimeMillis(&self) -> HostResult<u64> { Ok(100) }
        fn scheduleHostRuntimeAsyncTask(&self, _: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
            self.tasks.lock().unwrap().push(task); Ok(())
        }
        fn scheduleHostRuntimeCooperativeAsyncTask(&self, _: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
            self.tasks.lock().unwrap().push(task); Ok(())
        }
        fn scheduleHostRuntimeTask(&self, _: &str, _: HostRuntimeTask) -> HostResult<()> { unreachable!() }
        fn scheduleDelayedHostRuntimeTask(&self, _: &str, _: u64, _: HostRuntimeTask) -> HostResult<()> { unreachable!() }
        fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture { unreachable!() }
        fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture { unreachable!() }
    }

    /// 路由在这组测试里不会被真正触达，占位即可。
    struct NoRouter;
    #[async_trait::async_trait(?Send)]
    impl PeerRouter for NoRouter {
        fn localNodeId(&self) -> String { "self".into() }
        async fn routedCall(&self, _: String, r: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
            CoreCallResponse { requestId: r.payload.requestId, result: Err(CoreLinkError::new("UNUSED", "no router in this test")) }
        }
        async fn routedWatchSnapshot(&self, _: String, _: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent, CoreLinkError> { unreachable!() }
        async fn routedWatch(&self, _: String, _: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError> { unreachable!() }
        async fn routedOpenPush(&self, _: String, _: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> { unreachable!() }
        fn spaceChannelScope(&self, _: &str) -> Result<Option<String>, CoreLinkError> { Ok(Some("test-space".into())) }
    }

    /// 永不收发的连接：读循环停在 receive 上，通道不会失败。
    struct SilentPeer(PeerEndpoint);
    #[async_trait::async_trait]
    impl PeerConnection for SilentPeer {
        fn source(&self) -> &PeerEndpoint { &self.0 }
        fn target(&self) -> &PeerEndpoint { &self.0 }
        fn transport(&self) -> PeerTransport { PeerTransport::WebSocket }
        async fn send(&self, _: PeerMessage) -> Result<(), String> { unreachable!() }
        async fn receive(&self) -> Result<Option<PeerMessage>, String> { unreachable!() }
        async fn close(&self) {}
    }

    #[derive(Default)]
    struct Storage(Mutex<BTreeMap<String, Vec<u8>>>);
    impl operit_host_api::RuntimeStorageHost for Storage {
        /// Confirms synchronous in-memory publication without copying or rewriting bytes.
        fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
            if !self.exists(path)? {
                return Err(operit_host_api::HostError::new("runtime storage entry not found"));
            }
            Ok(())
        }

        fn runtimeRootDir(&self) -> Option<std::path::PathBuf> { None }
        fn workspaceRootDir(&self) -> Option<std::path::PathBuf> { None }
        fn readBytes(&self, p: &str) -> HostResult<Vec<u8>> { self.0.lock().unwrap().get(p).cloned().ok_or_else(|| HostError::new("missing")) }
        /// Atomically publishes one complete in-memory test entry.
        fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
            self.writeBytes(path, content)
        }

        fn writeBytes(&self, p: &str, b: &[u8]) -> HostResult<()> { self.0.lock().unwrap().insert(p.into(), b.into()); Ok(()) }
        fn appendBytes(&self, p: &str, b: &[u8]) -> HostResult<()> { self.0.lock().unwrap().entry(p.into()).or_default().extend_from_slice(b); Ok(()) }
        fn delete(&self, p: &str, _: bool) -> HostResult<()> { self.0.lock().unwrap().remove(p); Ok(()) }
        fn exists(&self, p: &str) -> HostResult<bool> { Ok(self.0.lock().unwrap().contains_key(p)) }
        fn list(&self, p: &str) -> HostResult<Vec<operit_host_api::RuntimeStorageEntry>> { Ok(self.0.lock().unwrap().iter().filter(|(k, _)| k.starts_with(p)).map(|(k, v)| operit_host_api::RuntimeStorageEntry { path: k.clone(), isDirectory: false, size: v.len() as i64 }).collect()) }
    }

    /// Creates isolated storage and a router using the native test scheduler.
    pub(super) fn service() -> HostRuntimePeerService {
        serviceWithScheduler(Arc::new(operit_host_native_common::NativeHostRuntimeTaskSchedulerHost))
    }

    /// Creates a peer service whose task queue can be controlled by the caller.
    fn serviceWithScheduler(scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>) -> HostRuntimePeerService {
        static ROUTER: std::sync::OnceLock<Arc<NoRouter>> = std::sync::OnceLock::new();
        let router: Arc<dyn PeerRouter> = ROUTER.get_or_init(|| Arc::new(NoRouter)).clone();
        let storage = Arc::new(Storage::default());
        let _ = operit_store::CoreSpaceStore::CoreSpaceStore::newNodeLocal(storage.clone()).initialize();
        HostRuntimePeerService { state: Arc::new(State {
            limits: PeerRuntimeLimits::default(),
            host: Arc::new(HostManager {
                runtimeStorageHost: Some(storage.clone()),
                hostRuntimeTaskSchedulerHost: Some(scheduler),
                ..HostManager::default()
            }),
            link: HostPeerLink::default(),
            store: PeerStateStore::new(storage),
            nodeId: "self".into(),
            info: LinkDeviceInfo { platform: "test".into(), model: "test".into(), pairingIncarnation: None },
            router: Arc::downgrade(&router),
            listeners: AsyncMutex::default(), connections: Mutex::default(), channels: Mutex::default(),
            advertisements: Mutex::default(), slots: Arc::new(tokio::sync::Semaphore::new(2)),
            active: Mutex::default(), changes: broadcast::channel(32).0, availability: Mutex::new(None),
            lifecycle: AsyncMutex::new(()), mutation: Mutex::new(()),
            connectLocks: Mutex::default(),
            pairingThrottle: Mutex::new(PairingThrottle::default()),
        })}
    }

    /// Creates an idle channel whose Host tasks remain under test control.
    fn callableChannel() -> Arc<MultiplexedChannel> {
        let scheduler = Arc::new(QueuedScheduler::default());
        let encrypted = Channel::new(
            Arc::new(SilentPeer(PeerEndpoint { nodeId: "peer".into(), address: "unused".into() })),
            &[7; 32], b"test", false).unwrap();
        let side = dispatch::InboundSide::new(service(), "peer".into(), "test".into(), dispatch::Authorization::Inbound);
        MultiplexedChannel::start(encrypted, scheduler, side, || {}).unwrap()
    }

    /// A caller cancelled before dispatch cannot open transport or execute its queued request.
    #[tokio::test]
    async fn cancelled_queued_call_releases_host_work_without_delivery() {
        let scheduler = Arc::new(QueuedScheduler::default());
        let service = serviceWithScheduler(scheduler.clone());
        let request = RoutedCoreRequest { spaceId: "test-space".into(), originNodeId: "self".into(),
            targetNodeId: "peer".into(), ttl: 0, routeKind: operit_link::RoutedCoreRequestKind::Target,
            payload: CoreCallRequest::new("cancelled", "native.device", "write", CoreValue::Null) };
        let mut call = Box::pin(service.call("peer", request));
        assert!(futures_util::poll!(&mut call).is_pending());
        assert_eq!(scheduler.tasks.lock().unwrap().len(), 1);
        drop(call);
        let task = scheduler.tasks.lock().unwrap().pop().unwrap();
        task().await;
        assert!(scheduler.tasks.lock().unwrap().is_empty());
        assert!(service.state.connectLocks.lock().unwrap().is_empty());
        assert!(service.state.active.lock().unwrap().is_empty());
    }

    /// Concurrent operations reuse the inbound carrier even beyond the deleted pool threshold.
    #[tokio::test]
    async fn all_operations_share_one_channel_and_a_second_carrier_is_rejected() {
        let service = service();
        let channel = callableChannel();
        service.registerChannel("p1", &channel).unwrap();
        let calls = (0..64).map(|_| service.acquireChannel("p1"));
        for result in futures_util::future::join_all(calls).await {
            assert!(Arc::ptr_eq(&result.unwrap(), &channel));
        }
        assert_eq!(service.channelCount("p1").await, 1);
        let second = callableChannel();
        assert_eq!(service.registerChannel("p1", &second).unwrap_err().code, "PEER_CHANNEL_EXISTS");
        assert!(Arc::ptr_eq(&service.peerChannel("p1").unwrap(), &channel));
    }

    /// A Space client cannot silently move from its original carrier to a replacement.
    #[tokio::test]
    async fn replacing_the_only_channel_invalidates_existing_space_clients() {
        let service = service();
        let first = callableChannel();
        service.registerChannel("p1", &first).unwrap();
        let client = service.spaceClient().unwrap();
        first.close().await;
        let replacement = callableChannel();
        service.registerChannel("p1", &replacement).unwrap();
        let result = client.call(CoreCallRequest::new("old", "test", "read", CoreValue::Null)).await;
        assert_eq!(result.result.unwrap_err().code, "SPACE_ENTRY_CHANGED");
    }

    /// Updates only the existing pairing record and reuses the same authenticated channel.
    #[tokio::test]
    async fn operator_endpoint_update_preserves_the_only_channel() {
        let service = service();
        let record = StoredOutbound {
            endpoint: "ws://10.0.0.1:37195/link".into(), sessionId: "s1".into(),
            deviceId: "self".into(), peerNodeId: "p1".into(), peerDeviceInfo: service.state.info.clone(),
            pairingServiceVersion: PAIRING_SERVICE_VERSION, sessionSecret: BASE64.encode([9u8; 32]),
            transport: "ws".into(),
        };
        service.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "s1", &record).unwrap();
        let channel = callableChannel();
        service.registerChannel("p1", &channel).unwrap();
        service.updatePeerEndpoint("p1", " https://peer.example:8443/operit/link ", Some(PeerTransport::Tcp)).await.unwrap();
        let updated = service.outbound("p1").unwrap();
        assert_eq!(updated.endpoint, "https://peer.example:8443/operit/link");
        assert_eq!(updated.transport, "http");
        assert_eq!(updated.sessionId, record.sessionId);
        assert_eq!(updated.sessionSecret, record.sessionSecret);
        assert!(Arc::ptr_eq(&service.peerChannel("p1").unwrap(), &channel));
        assert_eq!(service.channelCount("p1").await, 1);
        service.updatePeerEndpoint("p1", "peer.example:8444", None).await.unwrap();
        assert_eq!(service.outbound("p1").unwrap().transport, "http");
        assert!(service.updatePeerEndpoint("p1", "host:0", None).await.is_err());
        assert_eq!(service.outbound("p1").unwrap().endpoint, "peer.example:8444");
        assert_eq!(service.updatePeerEndpoint("stranger", "host:37195", None).await.unwrap_err().code,
            "PEER_OUTBOUND_NOT_AUTHORIZED");
        assert_eq!(service.state.store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap().len(), 1);
    }

    /// Publishes inbound channel arrival and loss through the unified availability projection.
    #[tokio::test]
    async fn inbound_channel_announces_reachability() {
        let service = service();
        let mut changes = service.subscribePeerChanges();
        let channel = callableChannel();
        assert!(!service.activePeerNodeIds().unwrap().contains("p1"));
        service.registerChannel("p1", &channel).unwrap();
        assert!(service.activePeerNodeIds().unwrap().contains("p1"));
        changes.try_recv().unwrap();
        service.disconnectPeer("p1").await.unwrap();
        assert!(!service.activePeerNodeIds().unwrap().contains("p1"));
        changes.try_recv().unwrap();
    }

    /// Rejects invalid operator endpoints before any record is written.
    #[test]
    fn operator_endpoint_parsing_rejects_malformed_addresses() {
        for bad in ["", "   ", "ws://", "ws://host", "host:", "host:0", "host:notaport",
            "ftp://host:37195/link", "[fd00::1:37195", "ws://[fd00::1]/link"] {
            assert!(parseOperatorEndpoint(bad).is_err(), "{bad} must be rejected");
        }
    }

    /// Normalizes supported endpoint schemes and bracketed IPv6 authorities.
    #[test]
    fn operator_endpoint_parsing_normalizes_accepted_forms() {
        let parsed = parseOperatorEndpoint("ws://10.0.0.5:37195").unwrap();
        assert_eq!(parsed.rendered, "ws://10.0.0.5:37195/link");
        assert_eq!(parsed.transport, Some(PeerTransport::WebSocket));
        let parsed = parseOperatorEndpoint("10.0.0.5:37195").unwrap();
        assert_eq!(parsed.rendered, "10.0.0.5:37195");
        assert_eq!(parsed.transport, None);
        let parsed = parseOperatorEndpoint("wss://[fd00::1]:8443/operit").unwrap();
        assert_eq!(parsed.rendered, "wss://[fd00::1]:8443/operit");
        assert_eq!(parsed.transport, Some(PeerTransport::WebSocket));
        let parsed = parseOperatorEndpoint("tcp://10.0.0.5:37195").unwrap();
        assert_eq!(parsed.rendered, "tcp://10.0.0.5:37195");
        assert_eq!(parsed.transport, Some(PeerTransport::Tcp));
    }

    /// A forwarding call waits for a later request on the same carrier to complete.
    struct WaitingRouter { entered: tokio::sync::Notify, release: tokio::sync::Notify }
    #[async_trait::async_trait(?Send)]
    impl PeerRouter for WaitingRouter {
        fn localNodeId(&self) -> String { "self".into() }
        async fn routedCall(&self, _: String, r: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
            if r.payload.methodName == "wait" {
                self.entered.notify_one();
                self.release.notified().await;
            } else { self.release.notify_one(); }
            CoreCallResponse::ok(r.payload.requestId, r.payload.args)
        }
        async fn routedWatchSnapshot(&self, _: String, _: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent, CoreLinkError> { unreachable!() }
        async fn routedWatch(&self, _: String, _: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError> {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(CoreEventStream::channel().1)
        }
        async fn routedOpenPush(&self, _: String, _: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> { unreachable!() }
        fn spaceChannelScope(&self, _: &str) -> Result<Option<String>, CoreLinkError> { Ok(None) }
    }

    struct MemoryPeer {
        endpoint: PeerEndpoint,
        tx: tokio::sync::mpsc::UnboundedSender<PeerMessage>,
        rx: AsyncMutex<tokio::sync::mpsc::UnboundedReceiver<PeerMessage>>,
    }
    #[async_trait::async_trait]
    impl PeerConnection for MemoryPeer {
        fn source(&self) -> &PeerEndpoint { &self.endpoint }
        fn target(&self) -> &PeerEndpoint { &self.endpoint }
        fn transport(&self) -> PeerTransport { PeerTransport::Tcp }
        async fn send(&self, message: PeerMessage) -> Result<(), String> { self.tx.send(message).map_err(|e| e.to_string()) }
        async fn receive(&self) -> Result<Option<PeerMessage>, String> { Ok(self.rx.lock().await.recv().await) }
        async fn close(&self) {}
    }

    /// Models synchronous persistence work inside an otherwise asynchronous routed Call.
    struct BlockingRouter {
        entered: tokio::sync::Notify,
        release: Mutex<std::sync::mpsc::Receiver<()>>,
        events: Mutex<Option<CoreEventStream>>,
    }

    #[async_trait::async_trait(?Send)]
    impl PeerRouter for BlockingRouter {
        /// Supplies the local identity required by the routed dispatcher.
        fn localNodeId(&self) -> String { "self".into() }
        /// Holds a synchronous operation until the test explicitly releases it.
        async fn routedCall(&self, _: String, request: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
            self.entered.notify_one();
            self.release.lock().unwrap().recv_timeout(std::time::Duration::from_secs(5)).unwrap();
            CoreCallResponse::ok(request.payload.requestId, CoreValue::Null)
        }
        /// Rejects snapshots because the test only opens a live Watch.
        async fn routedWatchSnapshot(&self, _: String, _: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent, CoreLinkError> { unreachable!() }
        /// Hands the dispatcher the single ordered stream controlled by the test.
        async fn routedWatch(&self, _: String, _: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError> {
            Ok(self.events.lock().unwrap().take().unwrap())
        }
        /// Rejects Push because the test exercises Call and Watch sharing one dispatcher.
        async fn routedOpenPush(&self, _: String, _: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> { unreachable!() }
        /// Leaves storage policy out of this dispatcher scheduling test.
        fn spaceChannelScope(&self, _: &str) -> Result<Option<String>, CoreLinkError> { Ok(None) }
    }

    /// A synchronous Call must not hold queued Watch events or completion on the same connection.
    #[tokio::test]
    async fn blocking_call_does_not_stall_watch_burst() {
        use std::time::Duration;
        let (release, blocked) = std::sync::mpsc::channel();
        let (events, stream) = CoreEventStream::channel();
        let router = Arc::new(BlockingRouter {
            entered: tokio::sync::Notify::new(), release: Mutex::new(blocked), events: Mutex::new(Some(stream)),
        });
        let mut service = service();
        Arc::get_mut(&mut service.state).unwrap().router = Arc::downgrade(&(router.clone() as Arc<dyn PeerRouter>));
        service.state.store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "pairing", &StoredInbound {
            deviceId: "p1".into(), deviceInfo: service.state.info.clone(),
            pairingServiceVersion: PAIRING_SERVICE_VERSION, sessionSecret: BASE64.encode([7; 32]),
        }).unwrap();
        let (requests, incoming) = tokio::sync::mpsc::unbounded_channel();
        let (responses, mut received) = tokio::sync::mpsc::unbounded_channel();
        let server = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
                dispatch::serveIncoming(
                    dispatch::InboundSide::new(service, "p1".into(), "pairing".into(), dispatch::Authorization::Inbound),
                    futures_util::stream::unfold(incoming, |mut incoming| async move {
                        incoming.recv().await.map(|request| (crypto::EncodedMessage::new(PeerMessage::Request(request)), incoming))
                    }),
                    |batch| { responses.send(batch.into_iter().map(|bytes| PeerMessage::decode(&bytes).unwrap()).collect::<Vec<_>>()).unwrap(); std::future::ready(Ok(())) },
                )
            ).unwrap();
        });
        let watch = CoreWatchRequest::new("events", "test", "events", CoreValue::Null);
        requests.send(CoreLinkRequest::Watch(CoreLinkWatchRequest::Open(dispatch::routedWatch(RoutedCoreRequest {
            spaceId: "test".into(), originNodeId: "p1".into(), targetNodeId: "self".into(),
            ttl: 8, routeKind: RoutedCoreRequestKind::Target, payload: watch.clone(),
        }).unwrap()))).unwrap();
        let opened = tokio::time::timeout(Duration::from_secs(5), received.recv()).await.unwrap().unwrap();
        assert!(matches!(opened.as_slice(), [PeerMessage::Response(CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Opened), .. })]));
        requests.send(CoreLinkRequest::Call(dispatch::routedCall(RoutedCoreRequest {
            spaceId: "test".into(), originNodeId: "p1".into(), targetNodeId: "self".into(),
            ttl: 8, routeKind: RoutedCoreRequestKind::Target,
            payload: CoreCallRequest::new("slow", "test", "syncApplyOperations", CoreValue::Null),
        }).unwrap())).unwrap();
        tokio::time::timeout(Duration::from_secs(5), router.entered.notified()).await.unwrap();
        for index in 0..=128 {
            events.send(CoreEvent {
                requestId: Some(watch.requestId.clone()), target: watch.target.clone(), propertyName: watch.propertyName.clone(),
                kind: if index == 128 { CoreEventKind::Completed } else { CoreEventKind::Changed },
                value: CoreValue::String(index.to_string()),
            }).unwrap();
        }
        let drained = tokio::time::timeout(Duration::from_millis(500), async {
            let mut next = 0;
            while next <= 128 {
                for response in received.recv().await.unwrap() {
                    if let PeerMessage::Response(CoreLinkResponse::Watch { result: Ok(CoreLinkWatchResponse::Event(event)), .. }) = response {
                        assert_eq!(event.value, CoreValue::String(next.to_string()));
                        assert_eq!(event.kind, if next == 128 { CoreEventKind::Completed } else { CoreEventKind::Changed });
                        next += 1;
                    }
                }
            }
        }).await;
        // Always release the synchronous operation, including on the failing implementation.
        release.send(()).unwrap();
        let replied = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(batch) = received.recv().await {
                for response in batch {
                    if let PeerMessage::Response(CoreLinkResponse::Call(response)) = response {
                        assert_eq!(response.requestId.0, "slow");
                        assert_eq!(response.result.unwrap(), CoreValue::Null);
                        return;
                    }
                }
            }
            panic!("the completed Call lost its response");
        }).await;
        drop(requests);
        server.join().unwrap();
        assert!(drained.is_ok(), "the synchronous Call held the Watch burst until its own completion");
        replied.unwrap();
    }

    #[tokio::test]
    async fn forwarded_call_does_not_block_the_call_that_releases_it() {
        independentCallWhileWaiting(false).await;
    }

    #[tokio::test]
    async fn opening_watch_does_not_block_the_call_that_releases_it() {
        independentCallWhileWaiting(true).await;
    }

    async fn independentCallWhileWaiting(watch: bool) {
        let router = Arc::new(WaitingRouter { entered: tokio::sync::Notify::new(), release: tokio::sync::Notify::new() });
        let mut service = service();
        Arc::get_mut(&mut service.state).unwrap().router = Arc::downgrade(&(router.clone() as Arc<dyn PeerRouter>));
        service.state.store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "pairing", &StoredInbound {
            deviceId: "p1".into(), deviceInfo: service.state.info.clone(),
            pairingServiceVersion: PAIRING_SERVICE_VERSION, sessionSecret: BASE64.encode([7; 32]),
        }).unwrap();
        let (atx, arx) = tokio::sync::mpsc::unbounded_channel();
        let (btx, brx) = tokio::sync::mpsc::unbounded_channel();
        let channel = |tx, rx, client| Channel::new(Arc::new(MemoryPeer {
            endpoint: PeerEndpoint { nodeId: "test".into(), address: "memory".into() }, tx, rx: AsyncMutex::new(rx),
        }), &[7; 32], b"forwarding", client).unwrap();
        let (client, server) = (channel(atx, brx, true), channel(btx, arx, false));
        let request = |method: &str| {
            let payload = CoreCallRequest::new(method, "test", method, CoreValue::Null);
            let routed = RoutedCoreRequest { spaceId: "test".into(), originNodeId: "p1".into(), targetNodeId: "self".into(),
                ttl: 8, routeKind: RoutedCoreRequestKind::Target, payload };
            PeerMessage::Request(CoreLinkRequest::Call(dispatch::routedCall(routed).unwrap()))
        };
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            let calls = async {
                let first = if watch {
                    let routed = RoutedCoreRequest { spaceId: "test".into(), originNodeId: "p1".into(), targetNodeId: "self".into(),
                        ttl: 8, routeKind: RoutedCoreRequestKind::Target, payload: CoreWatchRequest::new("wait", "test", "wait", CoreValue::Null) };
                    PeerMessage::Request(CoreLinkRequest::Watch(CoreLinkWatchRequest::Open(dispatch::routedWatch(routed).unwrap())))
                } else { request("wait") };
                client.send(first).await.unwrap();
                router.entered.notified().await;
                client.send(request("release")).await.unwrap();
                let mut ids = BTreeSet::new();
                while ids.len() < 2 {
                    let id = match client.receive().await.unwrap() {
                        Some(PeerMessage::Response(CoreLinkResponse::Call(r))) => { assert!(r.result.is_ok()); r.requestId },
                        Some(PeerMessage::Response(CoreLinkResponse::Watch { requestId, result: Ok(CoreLinkWatchResponse::Opened) })) => requestId,
                        Some(PeerMessage::Response(CoreLinkResponse::Watch { .. })) => continue,
                        _ => panic!("operation response"),
                    };
                    ids.insert(id.0);
                }
                assert_eq!(ids, BTreeSet::from(["wait".into(), "release".into()]));
            };
            tokio::select! {
                result = dispatch::serve(service, server, "p1".into(), "pairing".into()) => panic!("dispatcher stopped: {result:?}"),
                _ = calls => {},
            }
        }).await.expect("a forwarding call must not serialize unrelated calls behind it");
    }

}


#[cfg(test)]
mod handshake_response_tests {
    use super::*;
    enum Outcome { Closed, WrongId, Denied, Accepted }
    struct ReplyPeer {
        endpoint: PeerEndpoint,
        outcome: Outcome,
        id: Mutex<Option<CoreRequestId>>,
    }
    #[async_trait]
    impl PeerConnection for ReplyPeer {
        fn source(&self) -> &PeerEndpoint { &self.endpoint }
        fn target(&self) -> &PeerEndpoint { &self.endpoint }
        fn transport(&self) -> PeerTransport { PeerTransport::Tcp }
        async fn send(&self, message: PeerMessage) -> Result<(), String> {
            if let PeerMessage::Request(CoreLinkRequest::Call(request)) = message {
                *self.id.lock().unwrap() = Some(request.requestId);
                Ok(())
            } else { panic!("handshake must send a Call") }
        }
        async fn receive(&self) -> Result<Option<PeerMessage>, String> {
            let id = self.id.lock().unwrap().clone().expect("send before receive");
            let response = match self.outcome {
                Outcome::Closed => return Ok(None),
                Outcome::WrongId => CoreCallResponse::ok(CoreRequestId::new("wrong-response-id"), CoreValue::Null),
                Outcome::Denied => CoreCallResponse::err(id, CoreLinkError::new("PEER_REVOKED", "Authorization revoked")),
                Outcome::Accepted => CoreCallResponse::ok(id, CoreValue::Null),
            };
            Ok(Some(PeerMessage::Response(CoreLinkResponse::Call(response))))
        }
        async fn close(&self) {}
    }
    fn peer(outcome: Outcome) -> Arc<dyn PeerConnection> {
        Arc::new(ReplyPeer {
            endpoint: PeerEndpoint { nodeId: "board".into(), address: "test".into() },
            outcome, id: Mutex::new(None),
        })
    }
    #[tokio::test]
    async fn eof_during_pairing_is_a_closed_connection_not_a_correlation_error() {
        let failure = rawCall::<_, CoreValue>(&peer(Outcome::Closed), "hello", &CoreValue::Null).await.unwrap_err();
        assert_eq!(failure.code, "PEER_CONNECTION_CLOSED");
        assert!(failureProvesPeerUnavailable(&failure));
    }
    #[tokio::test]
    async fn response_ids_remain_strict_and_authorization_errors_are_preserved() {
        let failure = rawCall::<_, CoreValue>(&peer(Outcome::WrongId), "hello", &CoreValue::Null).await.unwrap_err();
        assert_eq!(failure.code, "PEER_SECURITY");
        assert!(failure.message.contains("correlation mismatch"));
        let denied = rawCall::<_, CoreValue>(&peer(Outcome::Denied), "hello", &CoreValue::Null).await.unwrap_err();
        assert_eq!(denied.code, "PEER_REVOKED");
        assert_eq!(rawCall::<_, CoreValue>(&peer(Outcome::Accepted), "hello", &CoreValue::Null).await.unwrap(), CoreValue::Null);
    }
}
