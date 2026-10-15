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
use crypto::{Channel, ChannelLease, MultiplexedChannel, error};
#[path = "peer/heartbeat.rs"] mod heartbeat;
#[path = "peer/live_channel.rs"] mod live_channel;
use live_channel::LiveChannel;
#[path = "peer/dispatch.rs"] mod dispatch;
#[path = "peer/duplex.rs"] mod duplex;
#[path = "peer/space_channel.rs"] mod space_channel;
#[path = "peer/availability.rs"] mod availability;
const HANDSHAKE: &str = "$peer.pairing";

const PAIRING_LIFETIME_MS: i64 = 300_000;
/// Advertises that this device accepts calls over a connection the peer opened,
/// so a peer behind NAT or a port-remapping proxy can be called back without
/// anyone dialing an address that is not routable from this side. Both peers
/// must advertise support; an older peer defaults this field to false.
const DUPLEX_CHANNELS: bool = true;
/// Consecutive failed dials after which a peer that can also be called over the
/// connection it opened stops being dialed: reaching it by dialing is not the
/// only way, so nobody should keep paying the connect timeout for it.
const ACTIVE_DIAL_FAILURE_LIMIT: u32 = 3;
/// How long that pause lasts before dialing is tried again, so a peer can always
/// recover on its own instead of staying unreachable forever.
const ACTIVE_DIAL_SUPPRESSION_MS: i64 = 60_000;
const MAX_POOLED_CHANNELS_PER_PEER: usize = 8;
const MAX_LOGICAL_LEASES_PER_CHANNEL: usize = 16;
/// Establishing one authenticated connection is bounded here rather than by the
/// caller, so a dial to an unreachable peer becomes an error for every business
/// call, watch and push instead of holding whoever asked for it.
const CONNECT_DEADLINE_MS: u64 = 30_000;
/// Replaces the host of one dial endpoint, keeping its scheme, recorded port and
/// path. The port is the peer's listener: a TCP source port is ephemeral.
fn endpointWithHost(endpoint: &str, host: &str) -> Option<String> {
    let (scheme, rest) = endpoint.split_once("://").map_or(("", endpoint), |(scheme, rest)| (scheme, rest));
    let (authority, path) = rest.split_once('/').map_or((rest, None), |(authority, path)| (authority, Some(path)));
    let (_, port) = authority.rsplit_once(':')?;
    let prefix = if scheme.is_empty() { String::new() } else { format!("{scheme}://") };
    let suffix = path.map_or(String::new(), |path| format!("/{path}"));
    Some(format!("{prefix}{host}:{port}{suffix}"))
}

/// Resolves the pairing carrier when the caller left it open: an explicit URL
/// scheme implies its carrier, a bare `ip:port` is classified by one throwaway
/// probe connection (see `operit_peer_link::detectTransport`).
async fn resolvePairingTransport(
    host: &std::sync::Arc<operit_host_api::HostManager::HostManager>,
    address: &str,
) -> Result<PeerTransport, String> {
    if let Some((scheme, _)) = address.split_once("://") {
        return match scheme {
            "http" => Ok(PeerTransport::Http),
            "ws" => Ok(PeerTransport::WebSocket),
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

/// Last transport/session failure only; no credentials or message contents.
pub fn peerSessionDiagnostic() -> Option<String> { duplex::diagnostic() }

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
    #[serde(default)] duplex: bool,
}
#[derive(Serialize, Deserialize)]
struct HelloReply { transcript: crypto::Transcript, info: LinkDeviceInfo, #[serde(default)] duplex: bool }
#[derive(Serialize, Deserialize)]
struct Authorization { keyProof: Vec<u8> }
#[derive(Serialize, Deserialize)]
struct Authorized { proof: Vec<u8>, pairingId: String }

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

/// Active dialing state for one peer: the failure streak that decides whether a
/// freshly observed address is worth adopting, and until when dialing is paused
/// because the peer proved reachable over the connection it opened instead.
#[derive(Clone, Default)]
struct PeerDialState {
    consecutiveFailures: u32,
    suppressedUntil: i64,
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
    pooledChannels: Mutex<BTreeMap<String, Arc<AsyncMutex<Vec<Arc<MultiplexedChannel>>>>>>,
    liveChannels: Mutex<BTreeMap<String, Vec<Weak<LiveChannel>>>>,
    /// Channels a peer opened to this device that may also carry calls back to
    /// it, kept weak so a closed channel never stays usable.
    callableInbound: Mutex<BTreeMap<String, Vec<Weak<MultiplexedChannel>>>>,
    /// Per-peer active-dialing state: failure streaks and paused dialing.
    dialStates: Mutex<BTreeMap<String, PeerDialState>>,
    advertisements: Mutex<Vec<Box<dyn operit_host_api::ServiceDiscovery::DiscoveryAdvertisement>>>,
    slots: Arc<tokio::sync::Semaphore>,
    active: Mutex<BTreeSet<String>>, changes: broadcast::Sender<()>,
    availability: Mutex<Option<availability::AvailabilityWorker>>,
    lifecycle: AsyncMutex<()>,
    connectLocks: Mutex<BTreeMap<String, Weak<AsyncMutex<()>>>>,
    sharedSessions: Mutex<BTreeMap<String, Arc<Channel>>>,
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
                pooledChannels: Mutex::new(BTreeMap::new()),
                liveChannels: Mutex::new(BTreeMap::new()),
                callableInbound: Mutex::new(BTreeMap::new()),
                dialStates: Mutex::new(BTreeMap::new()),
                advertisements: Mutex::new(Vec::new()),
                slots: Arc::new(tokio::sync::Semaphore::new(limits.incomingSessions)),
                active: Mutex::new(BTreeSet::new()),
                changes: broadcast::channel(32).0,
                availability: Mutex::new(None),
                lifecycle: AsyncMutex::new(()),
                connectLocks: Mutex::new(BTreeMap::new()),
                sharedSessions: Mutex::new(BTreeMap::new()),
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
        let mut locks = self.state.connectLocks.lock().unwrap();
        locks.retain(|_, lock| lock.strong_count() != 0);
        if let Some(lock) = locks.get(key).and_then(Weak::upgrade) { return lock; }
        let lock = Arc::new(AsyncMutex::new(()));
        locks.insert(key.to_owned(), Arc::downgrade(&lock));
        lock
    }
    fn changed(&self) { let _ = self.state.changes.send(()); }
    fn track(&self, node: &str, raw: &Arc<dyn PeerConnection>) {
        let mut connections = self.state.connections.lock().unwrap();
        let list = connections.entry(node.into()).or_default();
        list.retain(|c| c.strong_count() != 0); list.push(Arc::downgrade(raw));
    }
    fn pending(&self, inbound: bool, id: &str) -> Result<Pending, CoreLinkError> {
        let path = if inbound { RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH } else { RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH };
        let p = self.pendingRecords(path)?.remove(id)
            .ok_or_else(|| error("Pairing transaction not found"))?;
        if p.expires <= currentTimeMillis() { self.state.store.deleteRecord(path, id).map_err(error)?; return Err(error("Pairing transaction expired")); }
        Ok(p)
    }
    fn inboundCredentials(&self) -> Result<BTreeMap<String, StoredInbound>, CoreLinkError> {
        self.state.store.records(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH).map_err(error)
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
            self.removeSpaceChannels(nodeId)?;
        }
        Ok(())
    }
    async fn raw(&self, target: PeerEndpoint, transport: PeerTransport) -> Result<Arc<dyn PeerConnection>, CoreLinkError> {
        self.state.link.connect(self.state.host.clone(), PeerEndpoint { nodeId: self.state.nodeId.clone(), address: String::new() }, target, transport).await.map_err(error)
    }
    /// Exchanges ephemeral keys and verifies identity-bound proofs over every transport.
    async fn handshake(&self, raw: Arc<dyn PeerConnection>, purpose: &str, sessionId: &str,
        saved: Option<&[u8]>,
    ) -> Result<(Arc<Channel>, String, LinkDeviceInfo, Vec<u8>, String), CoreLinkError> {
        let (private, public) = crypto::ephemeral()?;
        let hello = Hello { version: PAIRING_SERVICE_VERSION, nodeId: self.state.nodeId.clone(),
            expectedNodeId: raw.target().nodeId.clone(),
            info: self.state.store.announcedDeviceInfo(&self.state.info), public: public.clone(),
            purpose: purpose.into(), sessionId: sessionId.into(), duplex: DUPLEX_CHANNELS };
        let reply: HelloReply = rawCall(&raw, "hello", &hello).await?;
        let t = &reply.transcript;
        if t.version != PAIRING_SERVICE_VERSION || t.clientNodeId != self.state.nodeId || t.clientPublic != public
            || (!raw.target().nodeId.is_empty() && t.serverNodeId != raw.target().nodeId)
            || t.serverNodeId == self.state.nodeId || t.serverPublic.len() != 32 || t.challenge.len() != 32
            || (!sessionId.is_empty() && t.sessionId != sessionId) {
            return Err(error("Handshake identity/transcript mismatch"));
        }
        let context = crypto::transcript(t)?;
        let dh = crypto::agree(private, &t.serverPublic)?;
        let root = crypto::derive(&dh, &context, b"operit-pairing-root-v1")?;
        let key = match saved { Some(saved) => crypto::derive(&dh, saved, &context)?, None => root };
        let accepted: Authorized = rawCall(&raw, "authorize", &Authorization {
            keyProof: crypto::proof(&key, &context, b"client"),
        }).await?;
        crypto::verify(&key, &context, b"server", &accepted.proof)?;
        if accepted.pairingId != t.sessionId { return Err(error("Pairing correlation mismatch")); }
        let id = t.sessionId.clone(); let peerNodeId = t.serverNodeId.clone();
        let peerDuplex = reply.duplex; let info = reply.info;
        let channel = Channel::new(raw, &key, &context, true)?;
        channel.setPeerDuplex(peerDuplex);
        channel.setSpaceScoped(purpose == "space");
        Ok((channel, id, info, root.to_vec(), peerNodeId))
    }
    /// Gets the per-peer pool lock without retaining a global lock during network I/O.
    fn channelPool(&self, node: &str) -> Arc<AsyncMutex<Vec<Arc<MultiplexedChannel>>>> {
        self.state.pooledChannels.lock().unwrap().entry(node.into())
            .or_insert_with(|| Arc::new(AsyncMutex::new(Vec::new()))).clone()
    }

    /// Tracks an authenticated connection generation and its terminal availability changes.
    fn startLiveChannel(&self, node: &str, channel: Arc<Channel>) -> Result<Arc<LiveChannel>, CoreLinkError> {
        let state = Arc::downgrade(&self.state);
        let peer = node.to_string();
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.clone()
            .ok_or_else(|| error("Host scheduler is not installed"))?;
        let channel = LiveChannel::start(channel, scheduler, move || {
            if let Some(state) = state.upgrade() {
                HostRuntimePeerService { state }.refreshPeerAvailability(&peer);
            }
        })?;
        {
            let mut channels = self.state.liveChannels.lock().unwrap();
            let channels = channels.entry(node.into()).or_default();
            channels.retain(|channel| channel.strong_count() != 0);
            channels.push(Arc::downgrade(&channel));
        }
        self.refreshPeerAvailability(node);
        Ok(channel)
    }

    /// Derives peer availability from all current authenticated connection generations.
    fn refreshPeerAvailability(&self, node: &str) {
        let sharedAvailable = self.cachedSession(node).is_some();
        let channels = self.state.liveChannels.lock().unwrap();
        let available = self.requirePeerConnectionAllowed(node).is_ok()
            && (sharedAvailable || channels.get(node).is_some_and(|channels| channels.iter()
                .filter_map(Weak::upgrade).any(|channel| channel.isAvailable())));
        let mut active = self.state.active.lock().unwrap();
        let changed = if available { active.insert(node.into()) } else { active.remove(node) };
        drop(active);
        drop(channels);
        if changed { self.changed(); }
    }

    /// Reports whether any current authenticated channel can still carry peer traffic.
    fn hasLiveChannel(&self, node: &str) -> bool {
        if self.cachedSession(node).is_some() { return true; }
        self.state.liveChannels.lock().unwrap().get(node).is_some_and(|channels| channels.iter()
            .filter_map(Weak::upgrade).any(|channel| channel.isAvailable()))
    }

    /// Registers one channel a peer opened that may also carry calls back to it.
    fn registerInboundCallableChannel(&self, node: &str, channel: &Arc<MultiplexedChannel>) {
        let mut registry = self.state.callableInbound.lock().unwrap();
        let entries = registry.entry(node.into()).or_default();
        entries.retain(|entry| entry.strong_count() > 0);
        entries.push(Arc::downgrade(channel));
    }

    /// Per-peer active dialing: a working dial forgets the streak, a failing one
    /// builds it, and a long enough streak beside a usable return connection
    /// pauses dialing so no caller keeps paying the connect timeout.
    fn recordDialFailed(&self, node: &str) {
        let paused = {
            let Ok(mut states) = self.state.dialStates.lock() else { return; };
            let state = states.entry(node.into()).or_default();
            state.consecutiveFailures = state.consecutiveFailures.saturating_add(1);
            if state.consecutiveFailures >= ACTIVE_DIAL_FAILURE_LIMIT && self.inboundCallableChannel(node).is_some() {
                state.suppressedUntil = currentTimeMillis() + ACTIVE_DIAL_SUPPRESSION_MS;
                true
            } else { false }
        };
        if paused {
            operit_util::AppLogger::AppLogger::i("RuntimePeerService", &format!(
                "Active dialing paused peer={node}: reachable over the connection it opened"));
        }
    }
    fn recordDialSucceeded(&self, node: &str) {
        if let Ok(mut states) = self.state.dialStates.lock() { states.remove(node); }
    }
    fn dialSuppressed(&self, node: &str) -> bool {
        let Ok(states) = self.state.dialStates.lock() else { return false; };
        states.get(node).is_some_and(|state| state.suppressedUntil > currentTimeMillis())
    }
    fn dialFailureCount(&self, node: &str) -> u32 {
        self.state.dialStates.lock().map(|states| states.get(node).map_or(0, |state| state.consecutiveFailures)).unwrap_or(0)
    }

    /// Re-points one peer's dial endpoint at the address its own authenticated
    /// connection came from. Only while the recorded address is failing (a working
    /// one is never replaced), and only the host moves: the port stays the recorded
    /// listening port, because a TCP source port is ephemeral and says nothing
    /// about where the peer listens.
    fn learnObservedEndpoint(&self, node: &str, observed: Option<std::net::SocketAddr>) {
        let skip = |reason: String| operit_util::AppLogger::AppLogger::trace("RuntimePeerService", &format!("Endpoint refresh skipped peer={node}: {reason}"));
        let Some(address) = observed else { skip("the carrier reports no address".into()); return; };
        let host = address.ip();
        if host.is_loopback() || host.is_unspecified() || host.is_multicast() {
            skip(format!("the observed address {host} is not routable"));
            return;
        }
        if self.dialFailureCount(node) == 0 {
            skip("the recorded address is not failing".into());
            return;
        }
        let host = match host { std::net::IpAddr::V6(_) => format!("[{host}]"), _ => host.to_string() };
        // Pairing writes this record under the same lock: never read-modify-write it beside one.
        let _guard = self.state.mutation.lock().unwrap();
        let records = match self.state.store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH) {
            Ok(records) => records,
            Err(error) => {
                operit_util::AppLogger::AppLogger::w("RuntimePeerService", &format!("Endpoint refresh failed peer={node}: {error}"));
                return;
            }
        };
        let Some((id, record)) = records.iter().find(|(_, record)|
            record.peerNodeId == node && record.pairingServiceVersion == PAIRING_SERVICE_VERSION) else {
            skip("no dial endpoint is recorded for this peer".into());
            return;
        };
        let Some(endpoint) = endpointWithHost(&record.endpoint, &host) else {
            operit_util::AppLogger::AppLogger::w("RuntimePeerService", &format!(
                "Endpoint refresh failed peer={node}: cannot re-host the recorded endpoint {}", record.endpoint));
            return;
        };
        if endpoint == record.endpoint {
            skip(format!("the observed address is already recorded ({endpoint})"));
            return;
        }
        let mut record = record.clone();
        record.endpoint = endpoint.clone();
        if let Err(error) = self.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, id, &record) {
            operit_util::AppLogger::AppLogger::w("RuntimePeerService", &format!("Endpoint refresh failed peer={node}: {error}"));
            return;
        }
        // A failing streak is what admitted this address, so let its trial start clean.
        self.recordDialSucceeded(node);
        operit_util::AppLogger::AppLogger::i("RuntimePeerService", &format!(
            "Dial endpoint refreshed from the peer's own connection peer={node} endpoint={endpoint}"));
    }

    /// Returns the least loaded channel that carries calls to a peer which dialed
    /// us. Such a peer can be called without dialing an address of its own, which
    /// is the only route available when that address is not routable from here.
    fn inboundCallableChannel(&self, node: &str) -> Option<Arc<MultiplexedChannel>> {
        let mut registry = self.state.callableInbound.lock().unwrap();
        let entries = registry.get_mut(node)?;
        entries.retain(|entry| entry.strong_count() > 0);
        entries.iter().filter_map(Weak::upgrade).filter(|channel| !channel.isFailed())
            .min_by_key(|channel| channel.activeLeases())
    }
    /// Acquires one channel for the peer within the connect deadline. The bound
    /// belongs to the acquisition itself: only the availability worker used to
    /// wrap it, so every other caller could wait on an unreachable address for
    /// as long as the transport took to give up.
    pub(super) async fn acquirePooledChannel(&self, node: &str) -> Result<ChannelLease, CoreLinkError> {
        match tokio::time::timeout(
            std::time::Duration::from_millis(CONNECT_DEADLINE_MS),
            self.acquirePooledChannelUnbounded(node),
        ).await {
            Ok(lease) => lease,
            Err(_) => Err(CoreLinkError::new("PEER_CONNECT_TIMEOUT", "Authenticated connection establishment timed out")),
        }
    }

    /// Acquires a lazily created multiplexed channel from the fixed per-peer pool.
    /// Call acquirePooledChannel instead: this form waits without a deadline.
    async fn acquirePooledChannelUnbounded(&self, node: &str) -> Result<ChannelLease, CoreLinkError> {
        self.requirePeerConnectionAllowed(node)?;
        if let Some(channel) = self.cachedSession(node) {
            if self.outbound(node).is_err() && !self.sessionScopeAllowed(node, &channel)? {
                return Err(error("Session return route is not admitted"));
            }
            self.offerSessionReturn(node, &channel).await?;
            return Ok(ChannelLease::Shared(channel));
        }
        let pool = self.channelPool(node);
        let mut channels = pool.lock().await;
        channels.retain(|channel| !channel.isFailed());
        if let Some(channel) = channels.iter().min_by_key(|channel| channel.activeLeases()) {
            if channel.activeLeases() < MAX_LOGICAL_LEASES_PER_CHANNEL || channels.len() >= MAX_POOLED_CHANNELS_PER_PEER {
                match self.outbound(node) {
                    Ok(record) => {
                        if let Some(spaceId) = self.router()?.spaceChannelScope(node)? {
                            if !channel.hasSpaceOffer(&record.sessionId, &spaceId).await
                                && self.offerSpaceChannelMultiplexed(node, &record.sessionId, channel).await? {
                                channel.markSpaceOffer(&record.sessionId, &spaceId).await;
                            }
                        }
                    }
                    Err(error) if error.code == "PEER_OUTBOUND_NOT_AUTHORIZED" => {}
                    Err(error) => return Err(error),
                }
                return Ok(channel.lease());
            }
        }
        // A peer that dialed us can be called over the connection it opened, which
        // is the only route that exists when its own address is not routable here.
        if self.router()?.spaceChannelScope(node)?.is_some() {
            if let Some(channel) = self.inboundCallableChannel(node) {
                operit_util::AppLogger::AppLogger::trace("RuntimePeerService", &format!("Calling {node} over the connection it opened"));
                return Ok(channel.lease());
            }
        }
        if self.dialSuppressed(node) {
            return Err(CoreLinkError::new("PEER_CALL_BACK_ONLY", "Active dialing is paused: the peer is reachable over the connection it opened"));
        }
        let raw = match self.connectAuthorized(node).await {
            Ok(raw) => { self.recordDialSucceeded(node); raw }
            Err(error) => { self.recordDialFailed(node); return Err(error); }
        };
        if raw.duplex().is_some() { return Ok(ChannelLease::Shared(raw)); }
        // Pairing and scoped Space channels both support reverse calls after admission.
        // Every request rechecks the original credential and current Space membership.
        let inbound = if DUPLEX_CHANNELS && raw.peerDuplex() && isNetworkTransport(raw.raw.transport()) {
            let (sessionId, authorization) = if raw.spaceScoped() {
                (self.spaceOutbound(node)?.pairingId, dispatch::Authorization::SpaceDuplex)
            } else {
                (self.outbound(node)?.sessionId, dispatch::Authorization::PairingDuplex)
            };
            Some(dispatch::InboundSide::new(self.clone(), node.into(), sessionId, authorization))
        } else { None };
        let live = self.startLiveChannel(node, raw.clone())?;
        let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.clone()
            .ok_or_else(|| error("Host scheduler is not installed"))?;
        let channel = MultiplexedChannel::start(live, scheduler, inbound)?;
        // Start the response reader before advertising a return route: the peer
        // may use that route as soon as it receives the offer.
        if let Ok(record) = self.outbound(node) {
            if let Some(spaceId) = self.router()?.spaceChannelScope(node)? {
                match self.offerSpaceChannelMultiplexed(node, &record.sessionId, &channel).await {
                    Ok(true) => channel.markSpaceOffer(&record.sessionId, &spaceId).await,
                    Ok(false) => {},
                    Err(error) => { raw.close().await; return Err(error); }
                }
            }
        }
        channels.push(channel.clone());
        Ok(channel.lease())
    }

    /// Returns the number of live pooled channels for one peer in test builds.
    #[cfg(test)]
    pub(crate) async fn pooledChannelCount(&self, node: &str) -> usize {
        self.channelPool(node).lock().await.len()
    }

    async fn connectAuthorized(&self, node: &str) -> Result<Arc<Channel>, CoreLinkError> {
        // An inbound session can supply a scoped return route without inventing
        // a reverse pairing credential or opening the exclusive endpoint again.
        if self.outbound(node).is_err() {
            if let Some(channel) = self.cachedSession(node) {
                if !self.sessionScopeAllowed(node, &channel)? { return Err(error("Session return route is not admitted")); }
                return Ok(channel);
            }
        }
        self.requirePeerConnectionAllowed(node)?;
        let (purpose, sessionId, secret, endpoint, transport) = match self.outbound(node) {
            Ok(record) => ("session", record.sessionId, record.sessionSecret, record.endpoint, parseTransport(&record.transport)?),
            Err(e) if e.code == "PEER_OUTBOUND_NOT_AUTHORIZED" => {
                let grant = self.spaceOutbound(node)?;
                ("space", grant.id, grant.secret, grant.endpoint, grant.transport)
            }
            Err(e) => return Err(e),
        };
        let target = PeerEndpoint { nodeId: node.into(), address: endpoint };
        let lock = self.state.link.exclusiveEndpointKey(&target, transport).map(|key| self.connectionLock(&key));
        let _connect = match &lock { Some(lock) => Some(lock.lock().await), None => None };
        if let Some(channel) = self.cachedSession(node) {
            self.requirePeerConnectionAllowed(node)?;
            self.offerSessionReturn(node, &channel).await?;
            return Ok(channel);
        }
        let root = BASE64.decode(&secret).map_err(|_| error("Invalid stored credential"))?;
        let raw = self.raw(target, transport).await?;
        let result = self.handshake(raw.clone(), purpose, &sessionId, Some(&root)).await;
        let (channel, _, _, _, _) = match result { Ok(result) => result, Err(e) => { raw.close().await; return Err(e); } };
        let reusable = if purpose == "session" && raw.requiresSessionReuse() {
            match self.supportsSessionReturn(&channel).await {
                Ok(supported) => supported,
                Err(e) => { channel.close().await; return Err(e); }
            }
        } else { false };
        if reusable {
            self.startDuplex(&channel, node)?;
            self.state.sharedSessions.lock().unwrap().insert(node.into(), channel.clone());
            let service = self.clone(); let incoming = channel.clone(); let peer = node.to_owned(); let pairing = sessionId.clone();
            let scheduled = self.state.host.hostRuntimeTaskSchedulerHost.as_ref().unwrap().scheduleHostRuntimeAsyncTask("session-return-dispatch", Box::new(move || Box::pin(async move {
                if let Err(e) = dispatch::serveAuthorized(service, incoming.clone(), peer, pairing, dispatch::Authorization::SessionReturn).await {
                    if let Some(d) = incoming.duplex() { d.fail(e); }
                }
                incoming.close().await;
            })));
            if let Err(e) = scheduled {
                channel.close().await;
                return Err(error(e.to_string()));
            }
            if let Err(e) = self.offerSessionReturn(node, &channel).await { channel.close().await; return Err(e); }
        }
        if let Err(e) = self.requirePeerConnectionAllowed(node) { raw.close().await; return Err(e); }
        self.track(node, &raw);
        if channel.duplex().is_some() && self.state.active.lock().unwrap().insert(node.into()) { self.changed(); }
        Ok(channel)
    }
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
        if hello.version != PAIRING_SERVICE_VERSION
            || hello.nodeId.is_empty()
            || hello.nodeId == self.state.nodeId
            || (!hello.expectedNodeId.is_empty() && hello.expectedNodeId != self.state.nodeId)
            || !matches!(hello.purpose.as_str(), "start" | "finish" | "session" | "space") {
            return sendError(&raw, helloRequest.requestId, error("Invalid pairing identity/purpose")).await;
        }
        if matches!(hello.purpose.as_str(), "session" | "space") {
            if let Err(revoked) = self.requirePeerConnectionAllowed(&hello.nodeId) {
                // 回报撤销原因而不是静默断开：静默关闭会被客户端误报成
                // PEER_SECURITY correlation mismatch，无法与传输故障区分。
                return sendError(&raw, helloRequest.requestId, revoked).await;
            }
        }
        // 沿用持久化的 sessionId/sessionSecret；配对事务仍受过期时间和尝试上限保护。
        let saved = match hello.purpose.as_str() {
            "start" => None,
            "finish" => { let p = self.pending(true, &hello.sessionId)?;
                if p.clientDeviceId != hello.nodeId { return Err(error("Pairing identity mismatch")); }
                Some(p.root) },
            "space" => Some(BASE64.decode(&self.spaceInbound(&hello.sessionId, &hello.nodeId)?.secret)
                .map_err(|_| error("Invalid Space credential"))?),
            _ => {
                let records = self.inboundCredentials()?;
                let record = records.get(&hello.sessionId).ok_or_else(|| error("Inbound authorization not found"))?;
                if record.deviceId != hello.nodeId || record.pairingServiceVersion != PAIRING_SERVICE_VERSION { return Err(error("Inbound identity/version mismatch")); }
                Some(BASE64.decode(&record.sessionSecret).map_err(|_| error("Invalid stored credential"))?)
            },
        };
        let (private, public) = crypto::ephemeral()?;
        let id = if hello.purpose == "start" { uuid::Uuid::new_v4().to_string() } else { hello.sessionId.clone() };
        let transcript = crypto::Transcript { version: PAIRING_SERVICE_VERSION, sessionId: id.clone(),
            clientNodeId: hello.nodeId.clone(), serverNodeId: self.state.nodeId.clone(),
            clientPublic: hello.public, serverPublic: public, challenge: crypto::random()?.to_vec() };
        let context = crypto::transcript(&transcript)?;
        let dh = crypto::agree(private, &transcript.clientPublic)?;
        let root = crypto::derive(&dh, &context, b"operit-pairing-root-v1")?;
        let key = match &saved { Some(saved) => crypto::derive(&dh, saved, &context)?, None => root };
        sendValue(&raw, helloRequest.requestId, HelloReply { transcript, duplex: DUPLEX_CHANNELS,
            info: self.state.store.announcedDeviceInfo(&self.state.info) }).await?;
        let request = self.readHandshake(&raw, "authorize").await?;
        let authorization: Authorization = fromCoreValue(request.args).map_err(|e| error(e.to_string()))?;
        let admission = (|| {
            crypto::verify(&key, &context, b"client", &authorization.keyProof)?;
            if hello.purpose == "start" {
                let _guard = self.state.mutation.lock().unwrap();
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
            Ok(())
        })();
        if let Err(e) = admission { return sendError(&raw, request.requestId, e).await; }
        sendValue(&raw, request.requestId, Authorized { proof: crypto::proof(&key, &context, b"server"), pairingId: id.clone() }).await?;
        if hello.purpose == "start" { return Ok(()); }
        let channel = Channel::new(raw.clone(), &key, &context, false)?;
        channel.setPeerDuplex(hello.duplex);
        if hello.purpose == "finish" {
            let Some(PeerMessage::Request(CoreLinkRequest::Call(request))) = channel.receive().await? else { return Err(error("Encrypted confirmation Call required")); };
            if request.target != HANDSHAKE || request.methodName != "finish" { return Err(error("Confirmation required before business")); }
            let proof: Vec<u8> = fromCoreValue(request.args).map_err(|e| error(e.to_string()))?;
            let mut replacing = false;
            let result = (|| {
                let _guard = self.state.mutation.lock().unwrap();
                let mut pending = self.pending(true, &id)?;
                if pending.attempts >= 5 { return Err(error("Confirmation attempt limit reached")); }
                pending.attempts += 1;
                self.state.store.putRecord(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH, &id, &pending).map_err(error)?;
                let code = pending.confirmationCode.as_deref().ok_or_else(|| error("Receiver confirmation missing"))?;
                crypto::verify(&pending.root, id.as_bytes(), code.as_bytes(), &proof)?;
                let record = StoredInbound { deviceId: hello.nodeId.clone(), deviceInfo: hello.info.clone(),
                    pairingServiceVersion: PAIRING_SERVICE_VERSION, sessionSecret: BASE64.encode(&pending.root) };
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
                self.state.active.lock().unwrap().insert(hello.nodeId.clone());
                self.changed();
            }
            channel.send(PeerMessage::Response(CoreLinkResponse::Call(CoreCallResponse { requestId: request.requestId, result }))).await?;
            return Ok(());
        }
        self.learnObservedEndpoint(&hello.nodeId, raw.remoteAddress());
        self.track(&hello.nodeId, &raw);
        if raw.requiresSessionReuse() && hello.purpose == "session" {
            self.startDuplex(&channel, &hello.nodeId)?;
            self.state.sharedSessions.lock().unwrap().insert(hello.nodeId.clone(), channel.clone());
            let result = dispatch::serve(self.clone(), channel.clone(), hello.nodeId, id, false).await;
            if let Some(d) = channel.duplex() { if let Err(e) = &result { d.fail(e.clone()); } channel.close().await; }
            result
        } else {
            let spaceChannel = hello.purpose == "space";
            channel.setSpaceScoped(spaceChannel);
            let live = self.startLiveChannel(&hello.nodeId, channel.clone())?;
            // One reader handles both directions, including original pairing connections.
            // Reverse routing becomes available only after admission to the same Space.
            if DUPLEX_CHANNELS && channel.peerDuplex() && isNetworkTransport(raw.transport()) {
                let scheduler = self.state.host.hostRuntimeTaskSchedulerHost.clone()
                    .ok_or_else(|| error("Host scheduler is not installed"))?;
                let authorization = if spaceChannel { dispatch::Authorization::SpaceChannel } else { dispatch::Authorization::Pairing };
                let inbound = dispatch::InboundSide::new(self.clone(), hello.nodeId.clone(), id, authorization);
                let multiplexed = MultiplexedChannel::start(live, scheduler, Some(inbound))?;
                self.registerInboundCallableChannel(&hello.nodeId, &multiplexed);
                // The accept path closes the connection as soon as this returns:
                // wait for the reader that now serves the channel instead.
                multiplexed.waitStopped().await;
                return Ok(());
            }
            dispatch::serveLive(self.clone(), live, channel, hello.nodeId, id, spaceChannel).await
        }
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
    fn spaceClient(&self) -> Option<Arc<dyn CoreLinkSharedClient + Send + Sync>> { duplex::spaceClient(self) }
    fn spaceConnection(&self) -> Option<super::NodeServices::SpaceClientConnection> { duplex::spaceConnection(self) }
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
        Ok(discoveredPeers(records, &self.state.nodeId, &pairedNodeIds))
    }
    /// Checks the known identity and the handshake result before creating a pairing transaction.
    async fn startPairing(&self, target: PeerEndpoint, transport: Option<PeerTransport>) -> Result<PendingPairing, CoreLinkError> {
        if !target.nodeId.is_empty() {
            let _guard = self.state.mutation.lock().unwrap();
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
                    self.handshake(raw.clone(), "start", "", None),
                )
                .await
                {
                    Ok(result) => result,
                    Err(_) => Err(error("Serial pairing handshake timed out")),
                }
            } else {
                self.handshake(raw.clone(), "start", "", None).await
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
            let _guard = self.state.mutation.lock().unwrap();
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
    /// Rechecks device uniqueness at each serialized confirmation boundary.
    async fn finishPairing(&self, id: &str, code: &str) -> Result<PairedPeer, CoreLinkError> {
        let pending = {
            let _guard = self.state.mutation.lock().unwrap();
            let mut p = self.pending(false, id)?;
            self.ensureDeviceUnpaired(&p.peerNodeId)?;
            if p.attempts >= 5 { return Err(error("Confirmation attempt limit reached")); }
            p.attempts += 1;
            self.state.store.putRecord(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, id, &p).map_err(error)?;
            // The receiver alone generates the six-digit user code. The encrypted
            // confirmation proof remains bound to this pairing's shared root/id;
            // the receiver checks the code and enforces expiration/attempt limits.
            if code.len() != 6 || !code.bytes().all(|c| c.is_ascii_digit()) {
                return Err(error("Enter the six-digit pairing code shown on the receiving device"));
            }
            p
        };
        let raw = self.raw(PeerEndpoint { nodeId: pending.peerNodeId.clone(), address: pending.endpoint.clone() }, pending.transport).await?;
        let confirmation = async {
            let (channel, _, _, _, _) = self.handshake(raw.clone(), "finish", id, Some(&pending.root)).await?;
            let request = CoreCallRequest::new(uuid::Uuid::new_v4().to_string(), HANDSHAKE, "finish",
                toCoreValue(crypto::proof(&pending.root, id.as_bytes(), code.as_bytes())).map_err(|e| error(e.to_string()))?);
            match channel.exchange(CoreLinkRequest::Call(request.clone())).await? {
                CoreLinkResponse::Call(r) if r.requestId == request.requestId => { r.result?; },
                _ => return Err(error("Confirmation response mismatch")),
            }
            Ok::<_, CoreLinkError>(())
        };
        // A USB UART has no EOF when the remote handshake rejects/closes.
        // Bound confirmation and release the port even if no response arrives.
        let result = if pending.transport == PeerTransport::Serial {
            match tokio::time::timeout(std::time::Duration::from_secs(20), confirmation).await {
                Ok(result) => result,
                Err(_) => Err(error("Serial pairing confirmation timed out")),
            }
        } else { confirmation.await };
        raw.close().await; result?;
        {
            let _guard = self.state.mutation.lock().unwrap();
            self.pending(false, id)?; // 撤销/取消不能被正在完成的网络事务重新授予权限。
            self.ensureDeviceUnpaired(&pending.peerNodeId)?;
            self.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, id, &StoredOutbound {
                endpoint: pending.endpoint, sessionId: id.into(), deviceId: self.state.nodeId.clone(), peerNodeId: pending.peerNodeId.clone(),
                peerDeviceInfo: pending.info.clone(), pairingServiceVersion: PAIRING_SERVICE_VERSION,
                sessionSecret: BASE64.encode(&pending.root), transport: transportName(pending.transport).into(),
            }).map_err(error)?;
            self.state.store.deleteRecord(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, id).map_err(error)?;
            self.state.active.lock().unwrap().insert(pending.peerNodeId.clone());
        }
        self.changed();
        Ok(PairedPeer { nodeId: pending.peerNodeId, displayName: pending.info.displayName(),
            inbound: false, outbound: true, pairingIncarnation: pending.info.pairingIncarnation.clone() })
    }
    async fn cancelPairing(&self, id: &str) -> Result<(), CoreLinkError> {
        let _guard = self.state.mutation.lock().unwrap();
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
                // Choose an address only for the first network listener. HTTP and
                // WS must then attach to the same server, never select two ports.
                let canMove = config.portMode == PeerHostPortMode::Automatic
                    && !listeners.keys().chain(opened.keys()).any(|t| isNetworkTransport(*t));
                let (listener, address) = listenWithPortFallback(&self.state.link, self.state.host.clone(),
                    PeerEndpoint { nodeId: self.state.nodeId.clone(), address: config.bindAddress.clone() },
                    *transport, canMove).await.map_err(error)?;
                config.bindAddress = address;
                opened.insert(*transport, listener);
            }
            let mut advertisements = Vec::new();
            if config.discoveryEnabled && capabilities.discoveryAdvertisement && requested.iter().any(|t| isNetworkTransport(*t)) {
                let host = self.state.host.serviceDiscoveryHost.as_ref().ok_or_else(|| error("Discovery enabled but Host not installed"))?;
                let socket: std::net::SocketAddr = config.bindAddress.parse().map_err(|_| error("Discovery requires a concrete network bindAddress"))?;
                let properties = [("nodeId".into(), self.state.nodeId.clone()), ("displayName".into(), self.state.info.displayName()),
                    ("transports".into(), requested.iter().filter(|t| isNetworkTransport(**t)).map(|t| transportName(*t)).collect::<Vec<_>>().join(","))].into();
                if !opened.is_empty() || self.state.advertisements.lock().unwrap().is_empty() {
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
                operit_util::AppLogger::AppLogger::i("RuntimePeerService", &format!("Listener port occupied; moved from {previousAddress} to {}", config.bindAddress));
            }
            Ok::<_, CoreLinkError>(advertisements)
        }.await;
        match result {
            Ok(advertisements) => {
                listeners.extend(opened);
                if !advertisements.is_empty() { *self.state.advertisements.lock().unwrap() = advertisements; }
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
        self.state.advertisements.lock().unwrap().clear();
        let listeners = std::mem::take(&mut *self.state.listeners.lock().await);
        for listener in listeners.into_values() { listener.close().await; }
        let serial = std::mem::take(&mut *self.state.sharedSessions.lock().unwrap());
        for channel in serial.into_values() { channel.close().await; }
        let connections = std::mem::take(&mut *self.state.connections.lock().unwrap());
        for raw in connections.into_values().flatten().filter_map(|v| v.upgrade()) { raw.close().await; }
        self.state.pooledChannels.lock().unwrap().clear();
        self.state.liveChannels.lock().unwrap().clear();
        self.state.callableInbound.lock().unwrap().clear();
        self.state.dialStates.lock().unwrap().clear();
        self.state.active.lock().unwrap().clear(); self.changed(); Ok(())
    }
    async fn call(&self, node: &str, mut request: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
        let id = request.payload.requestId.clone();
        // Relayed origins may reuse IDs; correlation belongs to this outbound hop.
        let wireId = CoreRequestId(operit_link::nextCoreRouteRequestId("peer-call"));
        request.payload.requestId = wireId.clone();
        let result = async {
            let channel = self.acquirePooledChannel(node).await?;
            let wire = dispatch::routedCall(request)?;
            let result = channel.exchange(CoreLinkRequest::Call(wire)).await;
            match result? { CoreLinkResponse::Call(r) if r.requestId == wireId => Ok(r.result), _ => Err(error("Call response mismatch")) }
        }.await;
        if result.as_ref().err().is_some_and(failureProvesPeerUnavailable) {
            self.refreshPeerAvailability(node);
        }
        // Remote business errors prove a working authenticated link, not an offline peer.
        CoreCallResponse { requestId: id, result: result.and_then(|remote| remote) }
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
    fn outboundPeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> { Ok(self.pairedPeers()?.into_iter().filter(|p| p.outbound).map(|p| p.nodeId).collect()) }
    fn activePeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> { Ok(self.state.active.lock().unwrap().clone()) }
    fn subscribePeerChanges(&self) -> broadcast::Receiver<()> { self.state.changes.subscribe() }
    fn pairingPrompts(&self) -> Result<Vec<PairingPrompt>, CoreLinkError> {
        Ok(self.pendingRecords(RUNTIME_LINK_ACCESS_PENDING_PAIRINGS_PATH)?.into_values()
            .filter(|p| p.expires > currentTimeMillis()).map(|p| PairingPrompt { pairingId: p.id,
                peerNodeId: p.clientDeviceId, displayName: p.info.displayName(), confirmationCode: p.confirmationCode.unwrap_or_default() }).collect())
    }
    async fn disconnectPeer(&self, node: &str) -> Result<(), CoreLinkError> {
        let serial = self.state.sharedSessions.lock().unwrap().remove(node);
        if let Some(channel) = serial { channel.close().await; }
        let connections = self.state.connections.lock().unwrap().remove(node).unwrap_or_default();
        for raw in connections.into_iter().filter_map(|v| v.upgrade()) { raw.close().await; }
        self.state.pooledChannels.lock().unwrap().remove(node);
        self.state.liveChannels.lock().unwrap().remove(node);
        self.state.callableInbound.lock().unwrap().remove(node);
        self.state.dialStates.lock().unwrap().remove(node);
        self.state.active.lock().unwrap().remove(node); self.changed(); Ok(())
    }
    async fn removePairedPeer(&self, node: &str) -> Result<(), CoreLinkError> {
        self.forgetPairingRecords(&[node.to_string()])?;
        self.disconnectPeer(node).await
    }

    fn forgetPairingRecords(&self, nodeIds: &[String]) -> Result<(), CoreLinkError> {
        if nodeIds.is_empty() { return Ok(()); }
        {
            let _guard = self.state.mutation.lock().unwrap();
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
    for record in records {
        let Some(node) = record.properties.get("nodeId").filter(|n| n.as_str() != localNodeId) else { continue; };
        if pairedNodeIds.contains(node) { continue; }
        let modes = record.properties.get("transports").map(String::as_str).unwrap_or("tcp");
        for mode in modes.split(',') {
            for ip in &record.addresses {
                if ip.is_unspecified() || ip.is_multicast()
                    || matches!(ip, std::net::IpAddr::V6(v6) if v6.is_unicast_link_local()) {
                    continue;
                }
                let socket = std::net::SocketAddr::new(*ip, record.port).to_string();
                let address = match mode { "http" => format!("http://{socket}/link"), "ws" => format!("ws://{socket}/link"), "tcp" => socket, _ => continue };
                let candidate = DiscoveredPeer { nodeId: node.clone(), address,
                    displayName: record.properties.get("displayName").cloned().unwrap_or_else(|| node.clone()) };
                // One device, not one row per NIC, IP family and transport.
                let rank = discoveryEndpointRank(*ip, mode);
                if peers.get(node).is_none_or(|(old, _)| rank < *old) {
                    peers.insert(node.clone(), (rank, candidate));
                }
            }
        }
    }
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
}

fn isNetworkTransport(transport: PeerTransport) -> bool {
    matches!(transport, PeerTransport::Http | PeerTransport::WebSocket | PeerTransport::Tcp)
}

// PeerLink currently exposes Host bind errors as strings. Only address-in-use
// errors qualify for retry; permissions, invalid addresses and missing Hosts do not.
fn isAddressInUse(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("address already in use") || message.contains("os error 48")
        || message.contains("os error 98") || message.contains("os error 10048")
}

async fn listenWithPortFallback(
    link: &dyn PeerLink, host: Arc<HostManager>, mut endpoint: PeerEndpoint,
    transport: PeerTransport, automatic: bool,
) -> Result<(Arc<dyn PeerListener>, String), String> {
    let mut socket = if isNetworkTransport(transport) {
        Some(endpoint.address.parse::<std::net::SocketAddr>().map_err(|_| "Listener requires an IP address and port".to_string())?)
    } else { None };
    // Port zero cannot be advertised correctly by the current PeerListener API.
    if let Some(address) = socket.as_mut() {
        if address.port() == 0 {
            if !automatic { return Err("Fixed listener port must be nonzero".into()); }
            address.set_port(37195);
            endpoint.address = address.to_string();
        }
    }
    for attempt in 0..128 {
        match link.listen(host.clone(), endpoint.clone(), transport).await {
            Ok(listener) => return Ok((listener, endpoint.address)),
            Err(e) if automatic && socket.is_some() && isAddressInUse(&e) && attempt < 127 => {
                let address = socket.as_mut().unwrap();
                address.set_port(if address.port() == u16::MAX { 49152 } else { address.port() + 1 });
                endpoint.address = address.to_string();
            }
            Err(e) => return Err(format!("Cannot listen on {}: {e}", endpoint.address)),
        }
    }
    unreachable!()
}

#[cfg(test)]
mod port_fallback_tests {
    use super::*;
    struct Listener;
    #[async_trait]
    impl PeerListener for Listener {
        async fn accept(&self) -> Result<Option<Arc<dyn PeerConnection>>, String> { Ok(None) }
        async fn close(&self) {}
    }
    struct Link {
        addresses: Mutex<Vec<String>>,
        failures: usize,
        error: &'static str,
    }
    #[async_trait]
    impl PeerLink for Link {
        async fn connect(&self, _: Arc<HostManager>, _: PeerEndpoint, _: PeerEndpoint, _: PeerTransport) -> Result<Arc<dyn PeerConnection>, String> { unreachable!() }
        async fn listen(&self, _: Arc<HostManager>, source: PeerEndpoint, _: PeerTransport) -> Result<Arc<dyn PeerListener>, String> {
            let mut addresses = self.addresses.lock().unwrap();
            addresses.push(source.address);
            if addresses.len() <= self.failures { Err(self.error.into()) } else { Ok(Arc::new(Listener)) }
        }
    }
    async fn bind(link: &Link, address: &str, automatic: bool) -> Result<(Arc<dyn PeerListener>, String), String> {
        listenWithPortFallback(link, Arc::new(HostManager::default()), PeerEndpoint { nodeId: "test".into(), address: address.into() }, PeerTransport::Http, automatic).await
    }
    #[tokio::test]
    async fn automatic_skips_multiple_occupied_ports() {
        let link = Link { addresses: Mutex::new(vec![]), failures: 2, error: "Address already in use (os error 48)" };
        assert_eq!(bind(&link, "0.0.0.0:37195", true).await.unwrap().1, "0.0.0.0:37197");
        assert_eq!(*link.addresses.lock().unwrap(), ["0.0.0.0:37195", "0.0.0.0:37196", "0.0.0.0:37197"]);
    }
    #[tokio::test]
    async fn fixed_and_non_collision_errors_do_not_retry() {
        for (automatic, error) in [(false, "Address already in use (os error 98)"), (true, "Permission denied"), (true, "HTTP server Host is not installed")] {
            let link = Link { addresses: Mutex::new(vec![]), failures: 200, error };
            assert!(bind(&link, "127.0.0.1:37195", automatic).await.is_err());
            assert_eq!(link.addresses.lock().unwrap().len(), 1);
        }
    }
    #[tokio::test]
    async fn ipv6_wraps_without_selecting_zero_and_retries_are_bounded() {
        let link = Link { addresses: Mutex::new(vec![]), failures: 1, error: "os error 10048" };
        assert_eq!(bind(&link, "[::]:65535", true).await.unwrap().1, "[::]:49152");
        let link = Link { addresses: Mutex::new(vec![]), failures: 200, error: "os error 98" };
        assert!(bind(&link, "127.0.0.1:37195", true).await.is_err());
        assert_eq!(link.addresses.lock().unwrap().len(), 128);
    }
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
mod dial_route_tests {
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
        async fn close(&self) { unreachable!() }
    }

    #[derive(Default)]
    struct Storage(Mutex<BTreeMap<String, Vec<u8>>>);
    impl operit_host_api::RuntimeStorageHost for Storage {
        fn runtimeRootDir(&self) -> Option<std::path::PathBuf> { None }
        fn workspaceRootDir(&self) -> Option<std::path::PathBuf> { None }
        fn readBytes(&self, p: &str) -> HostResult<Vec<u8>> { self.0.lock().unwrap().get(p).cloned().ok_or_else(|| HostError::new("missing")) }
        fn writeBytes(&self, p: &str, b: &[u8]) -> HostResult<()> { self.0.lock().unwrap().insert(p.into(), b.into()); Ok(()) }
        fn appendBytes(&self, p: &str, b: &[u8]) -> HostResult<()> { self.0.lock().unwrap().entry(p.into()).or_default().extend_from_slice(b); Ok(()) }
        fn delete(&self, p: &str, _: bool) -> HostResult<()> { self.0.lock().unwrap().remove(p); Ok(()) }
        fn exists(&self, p: &str) -> HostResult<bool> { Ok(self.0.lock().unwrap().contains_key(p)) }
        fn list(&self, p: &str) -> HostResult<Vec<operit_host_api::RuntimeStorageEntry>> { Ok(self.0.lock().unwrap().iter().filter(|(k, _)| k.starts_with(p)).map(|(k, v)| operit_host_api::RuntimeStorageEntry { path: k.clone(), isDirectory: false, size: v.len() as i64 }).collect()) }
    }

    fn service() -> HostRuntimePeerService {
        static ROUTER: std::sync::OnceLock<Arc<NoRouter>> = std::sync::OnceLock::new();
        let router: Arc<dyn PeerRouter> = ROUTER.get_or_init(|| Arc::new(NoRouter)).clone();
        let storage = Arc::new(Storage::default());
        let _ = operit_store::CoreSpaceStore::CoreSpaceStore::newNodeLocal(storage.clone()).initialize();
        HostRuntimePeerService { state: Arc::new(State {
            limits: PeerRuntimeLimits::default(),
            host: Arc::new(HostManager { runtimeStorageHost: Some(storage.clone()), ..HostManager::default() }),
            link: HostPeerLink::default(),
            store: PeerStateStore::new(storage),
            nodeId: "self".into(),
            info: LinkDeviceInfo { platform: "test".into(), model: "test".into(), pairingIncarnation: None },
            router: Arc::downgrade(&router),
            listeners: AsyncMutex::default(), connections: Mutex::default(), pooledChannels: Mutex::default(),
            liveChannels: Mutex::default(), callableInbound: Mutex::default(), dialStates: Mutex::default(),
            advertisements: Mutex::default(), slots: Arc::new(tokio::sync::Semaphore::new(2)),
            active: Mutex::default(), changes: broadcast::channel(32).0, availability: Mutex::new(None),
            lifecycle: AsyncMutex::new(()), mutation: Mutex::new(()),
            connectLocks: Mutex::default(), sharedSessions: Mutex::default(),
        })}
    }

    fn storedOutbound(peer: &str, endpoint: &str) -> StoredOutbound {
        StoredOutbound { endpoint: endpoint.into(), sessionId: format!("session-{peer}"), deviceId: "self".into(),
            peerNodeId: peer.into(), peerDeviceInfo: LinkDeviceInfo { platform: "t".into(), model: "t".into(), pairingIncarnation: None },
            pairingServiceVersion: PAIRING_SERVICE_VERSION, sessionSecret: BASE64.encode([9u8; 32]), transport: "ws".into() }
    }

    fn recordedEndpoint(service: &HostRuntimePeerService, peer: &str) -> Option<String> {
        service.state.store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).ok()
            .and_then(|records| records.into_values().find(|record| record.peerNodeId == peer).map(|record| record.endpoint))
    }

    /// 一条登记为可回拨、但永不失联的通道。
    fn callableChannel() -> Arc<MultiplexedChannel> {
        let scheduler = Arc::new(QueuedScheduler::default());
        let encrypted = Channel::new(
            Arc::new(SilentPeer(PeerEndpoint { nodeId: "peer".into(), address: "unused".into() })),
            &[7; 32], b"test", false).unwrap();
        let live = live_channel::LiveChannel::start(encrypted, scheduler.clone(), || {}).unwrap();
        MultiplexedChannel::start(live, scheduler, None).unwrap()
    }

    /// 拨号必然失败的对端（没有任何出站授权），回拨通道存在时调用必须成功：
    /// 这就是"调用一个只会拨进来的对端"。
    #[tokio::test]
    async fn a_return_connection_carries_calls_when_dialing_cannot() {
        let service = service();
        let mux = callableChannel();
        service.registerInboundCallableChannel("p1", &mux);
        let lease = service.acquirePooledChannel("p1").await.unwrap();
        assert!(Arc::ptr_eq(lease.pooled().unwrap(), &mux));
        assert_eq!(mux.activeLeases(), 1);
    }

    /// 端点只在记录地址失败时被观测地址替换，且只替换 host。
    #[test]
    fn endpoint_moves_only_while_the_recorded_address_fails() {
        let service = service();
        service.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "s1",
            &storedOutbound("p1", "ws://10.0.0.1:37195/link")).unwrap();
        // 记录地址没有失败：工作中的地址不被覆盖。
        service.learnObservedEndpoint("p1", Some("10.0.0.2:37195".parse().unwrap()));
        assert_eq!(recordedEndpoint(&service, "p1").as_deref(), Some("ws://10.0.0.1:37195/link"));
        // 失败之后才采纳观测地址，且 scheme、端口、路径原样保留。
        for _ in 0..3 { service.recordDialFailed("p1"); }
        service.learnObservedEndpoint("p1", Some("10.0.0.2:37195".parse().unwrap()));
        assert_eq!(recordedEndpoint(&service, "p1").as_deref(), Some("ws://10.0.0.2:37195/link"));
        // 新地址从干净的一次试用开始。
        assert_eq!(service.dialFailureCount("p1"), 0);
    }

    /// 只有在"能从回拨通道调用"时，连续失败才会暂停主动拨。
    #[test]
    fn three_failed_dials_beside_a_return_connection_pause_dialing() {
        let service = service();
        for _ in 0..3 { service.recordDialFailed("p1"); }
        // 没有回拨通道时，拨号是唯一路径，永不暂停。
        assert!(!service.dialSuppressed("p1"));
        // 登记回拨通道后，再一次失败就触发暂停。
        let mux = callableChannel();
        service.registerInboundCallableChannel("p1", &mux);
        service.recordDialFailed("p1");
        assert!(service.dialSuppressed("p1"));
        // 一次成功的拨号清空计数并解除暂停。
        service.recordDialSucceeded("p1");
        assert!(!service.dialSuppressed("p1"));
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
                result = dispatch::serve(service, server, "p1".into(), "pairing".into(), false) => panic!("dispatcher stopped: {result:?}"),
                _ = calls => {},
            }
        }).await.expect("a forwarding call must not serialize unrelated calls behind it");
    }

}
