#[path = "SyncConsistency.rs"]
mod consistency;

use crate::PeerSync::{PeerSyncMethod, SyncRequest, NODE_SYNC_TARGET};
use futures_util::{stream, StreamExt};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CorePushRequest, CoreValue};
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceStore};
use operit_store::NetworkControlStore::NetworkControlStore;
use operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use operit_store::RuntimeFileSyncStore::{RuntimeFileSyncReference, RuntimeFileSyncStore};
use operit_store::SyncOperationStore::{subscribeSyncMutations, syncMutationRevision, SyncMutationSubscription, SyncOperation, SyncOperationKnowledge};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use operit_store::PreferencesDataStore::StateFlow;
use crate::SpaceSyncProgress::{SpaceSyncProgress, SyncProgressTracker};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::CoreNodeRouter::{CoreNodeLocalRuntime, CoreNodeRouter};

const SYNC_DOMAINS: [&str; 6] = [
    "preferences",
    "chat",
    "binding",
    "runtime_file",
    "network_control",
    "plugin_storage",
];
const SYNC_BLOB_CHUNK_BYTES: i64 = 64 * 1024;
/// Bounds active file streams to eight chunks (512 KiB of raw payload).
const SYNC_BLOB_TRANSFER_CONCURRENCY: usize = 8;
/// Wire reservation for the Link request envelope, AEAD tag and frame prefix.
/// One operation above this reservation cannot travel in any single message.
const SYNC_PAGE_ENVELOPE_BYTES: usize = 4 * 1024;
/// Slack kept below the connection ceiling so MessagePack container headers
/// never push a measured page over the transport limit.
const SYNC_PAGE_HEADROOM_BYTES: usize = 64 * 1024;
/// Upper bound on pages one synchronization pass exchanges before yielding.
/// A device that writes continuously must not hold one pass open forever.
const MAX_SYNC_PAGES_PER_PASS: usize = 64;
/// Delay before the first retry of a failed full synchronization pass with one peer.
const PEER_FULL_SYNC_BACKOFF_BASE_MS: i64 = 2_000;
/// Ceiling for the exponential retry delay of one peer's full synchronization pass.
const PEER_FULL_SYNC_BACKOFF_MAX_MS: i64 = 60_000;
/// Polls for app and external-editor changes without putting discovery on app writes.
const WORKSPACE_SCAN_ACTIVE_INTERVAL_MS: u64 = 2_000;
/// Bound idle polling for local edits. Unchanged files
/// reuse metadata and blob references; the five-second ceiling leaves time to apply.
const WORKSPACE_SCAN_IDLE_INTERVAL_MS: u64 = 5_000;
const WORKSPACE_CHANGED: &str = "workspace_changed";
const WORKSPACE_RETRY_POLL: &str = "workspace_retry_poll";

/// Only verified exchange boundaries may become a caller's scheduling baseline.
#[derive(Serialize, Deserialize)]
pub(crate) struct PeerSyncStatus {
    local: Option<String>,
    remote: Option<String>,
    pending: bool,
    progress: Option<SpaceSyncProgress>,
}

/// Per-peer scheduling state behind the change-gated synchronization floor.
///
/// Direct pairings first probe their bounded, pairing-authorized snapshot header.
/// Only matching Space authority may proceed to the cheap `syncClockDigest`
/// RPC. Expensive exchanges (Space projection, control log,
/// business replication, blobs) run only when something actually changed for that
/// peer: the remote clock moved, the local synchronized-state summary moved
/// since the last completed pass, or a previous pass failed and its backoff
/// elapsed. Space records advance the journal clock, so every snapshot
/// ingredient is visible to the probe. Third-party control operations learned
/// by projection advance neither clock; the local summary counts them, so the
/// projecting device pushes relayed authority through its own next full pass.
/// Device preference changes have a separate revision and wake the same scheduler
/// without creating business operations.
#[derive(Clone, Default)]
struct PeerSyncGate {
    /// Every caller uses the same peer pass, including route changes.
    execution: Arc<tokio::sync::Mutex<()>>,
    /// Summarized remote clock observed at the last completed full pass.
    lastRemoteClockSummary: Option<String>,
    /// Local summary captured before the last verified empty-export boundary.
    lastLocalStateSummary: Option<String>,
    /// Consecutive failed full passes; drives the exponential retry delay.
    consecutiveFailures: u32,
    /// Earliest wall-clock time the next full pass may run after a failure.
    nextFullSyncAllowedAt: i64,
    /// Business failures keep receiving new data without arming connection failure backoff.
    pendingWork: bool,
    /// A correction error stays visible without suppressing the next pass.
    correctionError: Option<String>,
}

impl PeerSyncGate {
    fn backoffElapsed(&self, now: i64) -> bool {
        if self.consecutiveFailures == 0 {
            return true;
        }
        now >= self.nextFullSyncAllowedAt
    }
}

/// Computes the exponential retry delay for the given consecutive failure count.
fn peerFullSyncBackoffMs(consecutiveFailures: u32) -> i64 {
    let doubling = consecutiveFailures.saturating_sub(1).min(5);
    std::cmp::min(
        PEER_FULL_SYNC_BACKOFF_BASE_MS.saturating_mul(1_i64 << doubling),
        PEER_FULL_SYNC_BACKOFF_MAX_MS,
    )
}

static SPACE_SYNC_SERVICES: OnceLock<Mutex<BTreeMap<String, Weak<SpacePersistenceSyncState>>>> =
    OnceLock::new();

tokio::task_local! { static SYNC_NETWORK_FAILURE: std::cell::Cell<bool>; }

/// Keeps transport failure classification scoped to one asynchronous attempt.
async fn classifySync<T>(operation: impl std::future::Future<Output = Result<T, String>>) -> (Result<T, String>, bool) {
    SYNC_NETWORK_FAILURE.scope(std::cell::Cell::new(false), async {
        let result = operation.await;
        (result, SYNC_NETWORK_FAILURE.with(std::cell::Cell::get))
    }).await
}

/// Retains transport provenance before the existing application API formats the error.
fn syncLinkError(error: operit_link::CoreLinkError) -> String {
    recordSyncTransportError(&error);
    error.to_string()
}

/// Marks transport failures before shared Space helpers format a routed response.
pub(crate) fn recordSyncTransportError(error: &operit_link::CoreLinkError) {
    if matches!(error.code.as_str(), "CORE_NODE_UNREACHABLE" | "PEER_LINK_CLOSED" | "PEER_RESPONSE_CLOSED"
        | "PEER_SEND_FAILED" | "PEER_CONNECTION_CLOSED" | "PEER_CONNECT_TIMEOUT" | "PEER_HEARTBEAT_TIMEOUT"
        | "PEER_SYNC_NETWORK_FAILED" | "PEER_SECURITY") {
        let _ = SYNC_NETWORK_FAILURE.try_with(|failure| failure.set(true));
    }
}

/// Stores the runtime state owned by one CoreNode persistence worker.
struct SpacePersistenceSyncState {
    localRuntime: Arc<CoreNodeLocalRuntime>,
    nodeRouter: CoreNodeRouter,
    spaceStore: CoreSpaceStore,
    synchronizationScheduled: AtomicBool,
    active: AtomicBool,
    mutationSubscription: Mutex<Option<SyncMutationSubscription>>,
    peerChangesStop: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    synchronizationRequested: AtomicBool,
    synchronizationReasons: Mutex<BTreeSet<&'static str>>,
    workspaceScanStop: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    peerGates: Mutex<BTreeMap<String, PeerSyncGate>>,
    progress: SyncProgressTracker,
}

/// 通过 Router 与可达的 Space 成员同步持久化数据，不拥有配对、发现或传输。
#[derive(Clone)]
pub struct SpacePersistenceSyncService {
    state: Arc<SpacePersistenceSyncState>,
}

impl SpacePersistenceSyncService {
    /// Creates one persistence service over a concrete local CoreNode.
    pub fn new(
        localRuntime: Arc<CoreNodeLocalRuntime>,
        nodeRouter: CoreNodeRouter,
            spaceStore: CoreSpaceStore,
    ) -> Self {
        let mut services = persistenceServices().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let nodeId = nodeRouter.localNodeId();
        if let Some(state) = services.get(&nodeId).and_then(Weak::upgrade)
            .filter(|state| state.localRuntime.sharesApplicationWith(&localRuntime)) {
            return Self { state };
        }
        services.retain(|_, state| state.strong_count() > 0);
        let state = Arc::new(SpacePersistenceSyncState {
            localRuntime, nodeRouter, spaceStore,
            synchronizationScheduled: AtomicBool::new(false),
            active: AtomicBool::new(false),
            mutationSubscription: Mutex::new(None),
            peerChangesStop: Mutex::new(None),
            synchronizationRequested: AtomicBool::new(false),
            synchronizationReasons: Mutex::new(BTreeSet::new()),
            workspaceScanStop: Mutex::new(None),
            peerGates: Mutex::new(BTreeMap::new()),
            progress: SyncProgressTracker::default(),
        });
        services.insert(nodeId, Arc::downgrade(&state));
        Self { state }
    }

    /// Returns the shared local progress without rebuilding topology projections.
    pub(crate) fn progressFlow(&self) -> StateFlow<Vec<SpaceSyncProgress>> {
        self.state.progress.flow()
    }

    /// Returns this peer's current visible coordinator session.
    pub(crate) fn peerProgress(&self, peer: &str) -> Option<SpaceSyncProgress> {
        self.state.progress.peer(peer)
    }

    /// Starts the unique change-triggered persistence synchronizer for this CoreNode.
    pub fn start(&self) -> Result<(), String> {
        let startedAt = currentTimeMillis();
        self.state.nodeRouter.nodeServices()?;
        self.state.spaceStore.initialize()?;
        operit_util::AppLogger::AppLogger::v(
            "SpacePersistenceSyncService",
            &format!(
                "space store initialize done elapsedMs={}",
                currentTimeMillis() - startedAt
            ),
        );
        if self.state.active.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let registrationStartedAt = currentTimeMillis();

        let weakState = Arc::downgrade(&self.state);
        let subscription = subscribeSyncMutations(move || {
            let Some(state) = weakState.upgrade() else {
                return;
            };
            let service = SpacePersistenceSyncService { state };
            if let Err(error) = service.scheduleSynchronization("sync_mutation") {
                operit_util::AppLogger::AppLogger::e(
                    "SpacePersistenceSyncService",
                    &format!("Space persistence sync scheduling failed: {error}"),
                );
            }
        });
        *self
            .state
            .mutationSubscription
            .lock()
            .map_err(|error| format!("Space sync subscription lock poisoned: {error}"))? =
            Some(subscription);

        for (context, result) in [
            ("workspace scanner", self.startWorkspaceScanner()),
            ("peer change watcher", self.startPeerChangeWatcher()),
        ] {
            if let Err(error) = result {
                operit_util::AppLogger::AppLogger::w("SpacePersistenceSyncService", &format!("{context}: {error}"));
            }
        }
        operit_util::AppLogger::AppLogger::v(
            "SpacePersistenceSyncService",
            &format!(
                "peer availability watcher started elapsedMs={}",
                currentTimeMillis() - registrationStartedAt
            ),
        );
        let synchronizationStartedAt = currentTimeMillis();

        if let Err(error) = self.scheduleSynchronization("service_start") {
            let _ = self.stop();
            return Err(error);
        }
        operit_util::AppLogger::AppLogger::v(
            "SpacePersistenceSyncService",
            &format!(
                "space sync scheduling done elapsedMs={}",
                currentTimeMillis() - synchronizationStartedAt
            ),
        );
        Ok(())
    }

    /// Stops this CoreNode's persistence synchronizer and detaches its mutation listener.
    pub fn stop(&self) -> Result<(), String> {
        self.state.active.store(false, Ordering::Release);
        if let Some(stop) = self.state.workspaceScanStop.lock().map_err(|e| e.to_string())?.take() {
            let _ = stop.send(());
        }
        if let Some(stop) = self.state.peerChangesStop.lock()
            .map_err(|error| format!("peer change subscription lock poisoned: {error}"))?.take() {
            let _ = stop.send(());
        }
        self.state
            .mutationSubscription
            .lock()
            .map_err(|error| format!("Space sync subscription lock poisoned: {error}"))?
            .take();
        self.state.progress.clear();
        Ok(())
    }

    /// Observe shell/editor changes even while peers are offline. Uses the Host scheduler
    /// so desktop, mobile and Web share one lifecycle and stop mechanism.
    fn startWorkspaceScanner(&self) -> Result<(), String> {
        let weakState = Arc::downgrade(&self.state);
        let (sender, mut stopped) = tokio::sync::oneshot::channel();
        defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeBlockingAsyncTask(
            "core-node-workspace-file-scan",
            Box::new(move || Box::pin(async move {
                let mut intervalMs = WORKSPACE_SCAN_ACTIVE_INTERVAL_MS;
                loop {
                    tokio::select! {
                        _ = &mut stopped => break,
                        result = defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(intervalMs) => {
                            if result.is_err() { break; }
                        }
                    }
                    let Some(state) = weakState.upgrade() else { break; };
                    if !state.active.load(Ordering::Acquire) { break; }
                    let service = SpacePersistenceSyncService { state };
                    let mut changes = 0;
                    if let Err(error) = crate::fault_isolation::run("workspace file scanner", || service.workspaceFiles().scan().map(|scanned| changes = scanned)) {
                        operit_util::AppLogger::AppLogger::w("SpacePersistenceSyncService",
                            &format!("Workspace scan failed; no missing-file deletions published: {error}"));
                    }
                    // Reclamation belongs to background maintenance, never a file commit.
                    operit_store::SyncOperationStore::SyncOperationStore::new(
                        service.state.localRuntime.runtimeStorageHost(), RUNTIME_SYNC_DIR_PATH,
                    ).collectBlobs(None);
                    // A productive round pulls the scanner back to fast polling;
                    // quiet rounds double the interval up to the idle ceiling.
                    intervalMs = if changes > 0 {
                        WORKSPACE_SCAN_ACTIVE_INTERVAL_MS
                    } else {
                        (intervalMs * 2).min(WORKSPACE_SCAN_IDLE_INTERVAL_MS)
                    };
                    // Also retry interrupted transfers even when no new edits occur.
                    let _ = service.scheduleSynchronization(if changes > 0 { WORKSPACE_CHANGED } else { WORKSPACE_RETRY_POLL });
                }
            })),
        ).map_err(|e| e.to_string())?;
        *self.state.workspaceScanStop.lock().map_err(|e| e.to_string())? = Some(sender);
        Ok(())
    }

    fn workspaceFiles(&self) -> WorkspaceFileSyncStore {
        WorkspaceFileSyncStore::new(self.state.localRuntime.runtimeStorageHost(), RUNTIME_SYNC_DIR_PATH)
    }

    /// 只消费 runtime 的连接变化；不订阅发现端点，不读取或修改配对记录。
    fn startPeerChangeWatcher(&self) -> Result<(), String> {
        let mut stop = self.state.peerChangesStop.lock()
            .map_err(|error| format!("peer change subscription lock poisoned: {error}"))?;
        let mut changes = self.state.nodeRouter.nodeServices()?.peers().subscribePeerChanges();
        let weakState = Arc::downgrade(&self.state);
        let (sender, mut stopped) = tokio::sync::oneshot::channel();
        defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "core-node-space-peer-changes",
            Box::new(move || Box::pin(async move {
                loop {
                    tokio::select! {
                        _ = &mut stopped => break,
                        event = changes.recv() => {
                            if matches!(event, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                                break;
                            }
                        }
                    }
                    let Some(state) = weakState.upgrade() else { break; };
                    if !state.active.load(Ordering::Acquire) { break; }
                    if let Err(error) = (SpacePersistenceSyncService { state }).scheduleSynchronization("peer_connection_changed") {
                        operit_util::AppLogger::AppLogger::w(
                            "SpacePersistenceSyncService", &format!("Peer change sync scheduling failed: {error}"));
                    }
                }
            })),
        ).map_err(|error| error.to_string())?;
        *stop = Some(sender);
        Ok(())
    }

    /// Exchanges Space projections through direct pairings, then synchronizes every reachable member.
    pub async fn synchronizeOnce(&self) -> Result<(), String> {
        self.synchronizeCycle(true).await
    }

    async fn synchronizeCycle(&self, scanWorkspace: bool) -> Result<(), String> {
        let space = self.state.spaceStore.initialize()?;
        self.state.progress.retain(&space.spaceId, &space.members);
        // A shared removal takes effect before any exchange: when the replayed
        // policy no longer admits this device it retires itself here, and a
        // duplicate delivery of the same command is a no-op.
        {
            let service = crate::RuntimeRemoteLinkService::RuntimeRemoteLinkService::newWithRouter(
                (*self.state.localRuntime).clone(), self.state.nodeRouter.clone());
            if let Err(error) = service.retireRemovedDevice().await {
                operit_util::AppLogger::AppLogger::w(
                    "SpacePersistenceSyncService",
                    &format!("Removed-device retirement check failed: {error}"),
                );
            }
        }
        let mut errors = Vec::new();
        if scanWorkspace {
            if let Err(error) = crate::fault_isolation::run("scan workspace files", || self.workspaceFiles().scan().map(|_| ())) {
                errors.push(error);
            }
        }
        let services = self.state.nodeRouter.nodeServices()?;
        let activePeers = services.peers().activePeerNodeIds().map_err(|error| error.to_string())?;
        let localNodeId = self.state.nodeRouter.localNodeId();
        let control = NetworkControlStore::new(self.state.localRuntime.runtimeStorageHost())?;
        // Publish this device's live direct-link adjacency before any snapshot exchange so
        // every member can resolve routes through it. The announcements carry no measured
        // quality yet, which keeps route selection at hop count.
        match crate::NodeSpaceService::space_topology::publishLocalTopology(
            &self.state.spaceStore,
            &control,
            &localNodeId,
            &activePeers,
        ) {
            Ok(true) => {
                operit_util::AppLogger::AppLogger::v_with_level(
                    "SpacePersistenceSyncService",
                    &format!("space_topology.published local={localNodeId}"),
                    operit_util::AppLogger::VERBOSE_LEVEL_5,
                );
            }
            Ok(false) => {}
            Err(error) => {
                operit_util::AppLogger::AppLogger::w(
                    "SpacePersistenceSyncService",
                    &format!(
                        "Local Space topology publish failed; multi-hop routing may degrade: {error}"
                    ),
                );
            }
        }
        let service = crate::RuntimeRemoteLinkService::RuntimeRemoteLinkService::newWithRouter(
            (*self.state.localRuntime).clone(), self.state.nodeRouter.clone());
        if let Err(error) = crate::fault_isolation::operation("deliver Space departures",
            crate::NodeSpaceService::space_departure::synchronize(&service)).await { errors.push(error); }
        let localStateSummary = self.localStateSummary().await?;
        let mut cycleDecisions = BTreeMap::<String, bool>::new();
        let membershipBeforeExchange = self.state.spaceStore.space()?.members.into_iter().collect::<BTreeSet<_>>();
        // An authenticated carrier is duplex. The listener must reconcile its
        // inbound peers too before using the old member projection for routing.
        for peerNodeId in services.peers().pairedPeers().map_err(|error| error.to_string())?.into_iter().map(|peer| peer.nodeId) {
            if !activePeers.contains(&peerNodeId) { continue; }
            match control.nodeIsDisconnected(&peerNodeId) {
                Ok(false) => {},
                Ok(true) => continue,
                Err(error) => { errors.push(format!("Peer {peerNodeId} connection policy: {error}")); continue; }
            }
            let (projection, networkFailure) = classifySync(crate::fault_isolation::operation(&format!("Peer {peerNodeId} Space projection"),
                async {
                    if !self.pairedPeerPassRequired(&peerNodeId, &localStateSummary, &mut cycleDecisions).await? {
                        return Ok(None);
                    }
                    self.exchangePairedDeviceSpaceProjection(&peerNodeId).await.map(Some)
                })).await;
            let sameSpace = match projection {
                Ok(Some(sameSpace)) => sameSpace,
                Ok(None) => continue,
                Err(error) => {
                    self.recordPeerFailure(&peerNodeId, networkFailure, &error).await;
                    cycleDecisions.insert(peerNodeId.clone(), false);
                    errors.push(format!("CoreNode {peerNodeId} Space projection exchange: {error}"));
                    continue;
                }
            };
            // A paired peer outside the Space membership has no business
            // replication pass; the projection exchange was its whole pass.
            if !sameSpace || !membershipBeforeExchange.contains(&peerNodeId) {
                self.recordPeerPassCompleted(&peerNodeId, false).await?;
                cycleDecisions.insert(peerNodeId.clone(), false);
            }
        }
        let space = self.state.spaceStore.space()?;
        self.state.progress.retain(&space.spaceId, &space.members);
        for targetNodeId in space.members {
            if targetNodeId == localNodeId {
                continue;
            }
            let reachable = match self.state.nodeRouter.nodeIsReachable(&targetNodeId) {
                Ok(reachable) => reachable,
                Err(error) => { errors.push(format!("Peer {targetNodeId} reachability: {error}")); continue; }
            };
            if !reachable {
                self.state.progress.remove(&targetNodeId);
                continue;
            }
            if !self
                .peerPassRequired(&targetNodeId, &localStateSummary, &mut cycleDecisions)
                .await
            {
                continue;
            }
            if let Err(error) = self
                .synchronizeReachablePeerForReason(targetNodeId.clone(), 512, "change_gate")
                .await
            {
                errors.push(format!(
                    "CoreNode {} reachable persistence sync: {error}",
                    targetNodeId
                ));
                continue;
            }
        }
        if !errors.is_empty() {
            return Err(errors.join(" | "));
        }
        Ok(())
    }

    /// Test entry point for an explicit peer pass.
    #[cfg(test)]
    pub(crate) async fn synchronizeReachablePeer(
        &self,
        targetNodeId: String,
        limit: usize,
    ) -> Result<(), String> {
        self.synchronizeReachablePeerForReason(targetNodeId, limit, "direct_request").await
    }

    /// All entry points share serialization, progress, completion and retry state.
    /// Keeps the caller visible even when it bypasses the background change gate.
    pub(crate) async fn synchronizeReachablePeerForReason(
        &self,
        targetNodeId: String,
        limit: usize,
        trigger: &str,
    ) -> Result<(), String> {
        if limit == 0 { return Err("sync limit must be greater than 0".into()); }
        if targetNodeId == self.state.nodeRouter.localNodeId() { return Ok(()); }
        let execution = self.state.peerGates.lock().map_err(|error| error.to_string())?
            .entry(targetNodeId.clone()).or_default().execution.clone();
        let requestedAt = currentTimeMillis();
        let _pass = execution.lock().await;
        if matches!(trigger, "change_gate" | "peer_request") && !self.peerPassRequired(
            &targetNodeId, &self.localStateSummary().await?, &mut BTreeMap::new()).await {
            return Ok(());
        }
        let startedAt = currentTimeMillis();
        operit_util::AppLogger::AppLogger::i("SpacePersistenceSyncService", &format!(
            "sync_pass.start target={targetNodeId} trigger={trigger} limit={limit} queuedMs={}",
            startedAt - requestedAt));
        let space = self.state.spaceStore.space()?;
        self.state.progress.retain(&space.spaceId, &space.members);
        self.state.progress.begin(&targetNodeId, &space.spaceId);
        let context = format!("Peer {targetNodeId} persistence synchronization");
        let pass = classifySync(crate::fault_isolation::operation(&context, self.synchronizePeer(&targetNodeId, limit,
            !matches!(trigger, "change_gate" | "peer_request"))));
        tokio::pin!(pass);
        let (result, networkFailure) = if let Some(delayMs) = self.state.progress.revealDelayMs(&targetNodeId) {
            tokio::select! {
                biased;
                result = &mut pass => result,
                delay = defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(delayMs) => {
                    if delay.is_ok() { self.state.progress.reveal(&targetNodeId); }
                    pass.await
                }
            }
        } else { pass.await };
        match &result {
            Ok(allowed) => {
                self.recordPeerPassCompleted(&targetNodeId, *allowed).await?;
                let pending = self.state.peerGates.lock().map_err(|error| error.to_string())?
                    .get(&targetNodeId).is_some_and(|gate| gate.pendingWork);
                operit_util::AppLogger::AppLogger::i("SpacePersistenceSyncService", &format!(
                    "sync_pass.done target={targetNodeId} trigger={trigger} businessReplication={allowed} pending={pending} elapsedMs={}",
                    currentTimeMillis() - startedAt));
                if *allowed && pending {
                    self.state.progress.finish(&targetNodeId, true, None, None);
                } else {
                    self.state.progress.remove(&targetNodeId);
                }
            }
            Err(error) => {
                self.recordPeerFailure(&targetNodeId, networkFailure, error).await;
                let retryAt = self.state.peerGates.lock().map_err(|error| error.to_string())?
                    .get(&targetNodeId).filter(|gate| gate.consecutiveFailures > 0).map(|gate| gate.nextFullSyncAllowedAt);
                operit_util::AppLogger::AppLogger::w("SpacePersistenceSyncService", &format!(
                    "sync_pass.failed target={targetNodeId} trigger={trigger} retryAt={retryAt:?} elapsedMs={} error={error}",
                    currentTimeMillis() - startedAt));
                self.state.progress.finish(&targetNodeId, false, retryAt, Some(error.clone()));
            }
        }
        // A pass can revoke membership or storage authority while it is running.
        let space = self.state.spaceStore.space()?;
        self.state.progress.retain(&space.spaceId, &space.members);
        result.map(|_| ())
    }

    /// Exchanges preferences after the business receipts so those receipts cannot acknowledge unsent edits.
    async fn synchronizePeer(&self, targetNodeId: &str, limit: usize, force: bool) -> Result<bool, String> {
        if !self.state.nodeRouter.nodeIsReachable(targetNodeId)? {
            return Err(syncLinkError(operit_link::CoreLinkError::new("CORE_NODE_UNREACHABLE", format!(
                "synchronization target is not reachable in the current device space: {targetNodeId}"
            ))));
        }
        let storageReplicationAllowed = self.storageReplicationAllowed(targetNodeId)?;
        if storageReplicationAllowed {
            let protocol: crate::PeerProtocol::PeerProtocol = callRemote(&self.state.nodeRouter,
                targetNodeId, PeerSyncMethod::Protocol, Value::Null).await?;
            protocol.requireCompatible().map_err(|error| error.to_string())?;
        }
        if storageReplicationAllowed && self.state.nodeRouter.localNodeId().as_str() > targetNodeId {
            let request = callRemote::<PeerSyncStatus>(&self.state.nodeRouter, targetNodeId,
                PeerSyncMethod::SyncRequest, json!(SyncRequest { limit, force }));
            tokio::pin!(request);
            let mut progress_interval_ms = 3_000;
            let status = loop {
                tokio::select! { biased;
                    result = &mut request => break result?,
                    delay = defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(progress_interval_ms) => {
                        delay.map_err(|e|e.to_string())?;
                        let progress = tokio::select! { biased;
                            result = &mut request => break result?,
                            progress = callRemote::<Option<SpaceSyncProgress>>(&self.state.nodeRouter,
                                targetNodeId,PeerSyncMethod::SyncProgress,Value::Null) => progress?,
                        };
                        if let Some(progress) = progress {
                            // Hidden/short passes stay quiet; visible work gets responsive feedback.
                            progress_interval_ms = 500;
                            self.state.progress.mirror(targetNodeId,&progress);
                        }
                    }
                }
            };
            if let Some(progress) = status.progress { self.state.progress.mirror(targetNodeId, &progress); }
            let mut gates = self.state.peerGates.lock().map_err(|e|e.to_string())?;
            let gate = gates.entry(targetNodeId.to_owned()).or_default();
            gate.lastLocalStateSummary = status.remote;
            gate.lastRemoteClockSummary = status.local;
            gate.pendingWork = status.pending;
            drop(gates);
            return self.storageReplicationAllowed(targetNodeId);
        }
        // Authority updates must still reach non-storage members (including
        // grants/revocations); they do not advance business replication clocks.
        let protocol = projectionProtocol(&self.state.nodeRouter, targetNodeId).await?;
        let control = NetworkControlStore::new(self.state.localRuntime.runtimeStorageHost())?;
        if protocol.as_ref().is_some_and(|protocol| protocol.snapshotPaging) {
            // The endpoint already provides a bounded authority export. Use it
            // for reachable-member passes too; requesting the entire log again
            // forces a second full reply into its constrained heap. Projection
            // observation still validates all authority before preference sync.
            Box::pin(self.exchangePairedDeviceSpaceProjection(targetNodeId)).await?;
        } else {
            let request = serviceCallRequest(&self.state.nodeRouter,
                crate::NodeSpaceService::NODE_SPACE_CONTROL_TARGET, "exchange",
                serde_json::to_value(control.currentSpaceOperations()?).map_err(|error| error.to_string())?)?;
            if let Some(protocol) = &protocol {
                crate::NodeSpaceService::space_projection::requireRoutedRequestFits(&request,
                    &self.state.spaceStore.space()?.spaceId, &self.state.nodeRouter.localNodeId(),
                    targetNodeId, 4, protocol.maxMessageBytes)?;
            }
            let response = self.state.nodeRouter.callNode(targetNodeId.to_string(), request).await;
            let remote = crate::NodeSpaceService::decodeControlExchange(response.result.map_err(|error| {
                syncLinkError(error.clone()); error
            }))?;
            control.projectPeerControlOperations(targetNodeId, &remote)?;
            self.applyRemovedMemberEffects(&remote)?;
        }
        // Business receipts include the preference mutation boundary. Capture
        // them before exchanging preferences: an edit counted by a receipt must
        // still reach the peer, while later edits reopen the existing gate.
        let businessReplication = if self.storageReplicationAllowed(targetNodeId)? {
            self.validateReachableDeviceSpace(targetNodeId).await?;
            Box::pin(self.synchronizeDomains(targetNodeId, limit, &SYNC_DOMAINS)).await?
        } else { false };
        // Small device-owned preferences are available to every admitted member;
        // their clocks and payloads are separate from business replica clocks.
        let preferenceStore = operit_store::DevicePreferencesStore::DevicePreferencesStore::new(self.state.localRuntime.runtimeStorageHost());
        let preferenceScope = self.state.spaceStore.space()?.spaceId;
        // Every member receives its own configuration. Storage providers also
        // exchange the local device's configuration as a backup; lightweight
        // endpoints must not accumulate other devices' preference files.
        let mut preferenceNodes = vec![targetNodeId.to_string()];
        if self.storageReplicationAllowed(targetNodeId)? {
            preferenceNodes.push(self.state.nodeRouter.localNodeId());
        }
        for preferenceNodeId in preferenceNodes {
            let preferences = preferenceStore.read(&preferenceScope, &preferenceNodeId)?;
            let request = serviceCallRequest(&self.state.nodeRouter,
                crate::NodeSpaceService::NODE_SPACE_CONTROL_TARGET, "exchangePreferences",
                serde_json::to_value(preferences).map_err(|e| e.to_string())?)?;
            let response = self.state.nodeRouter.callNode(targetNodeId.to_string(), request).await;
            let preferences: operit_store::DevicePreferencesStore::DevicePreferences = decodeCoreResponse(response.result.map_err(syncLinkError)?)?;
            if preferences.spaceId != preferenceScope || preferences.nodeId != preferenceNodeId
                || self.state.spaceStore.space()?.spaceId != preferenceScope
                || !control.currentState()?.memberNodeIds.iter().any(|node| node == targetNodeId)
                || control.nodeIsDisconnected(targetNodeId)? {
                return Err("Device preference response scope changed".into());
            }
            preferenceStore.merge(&preferences)?;
        }
        Ok(businessReplication)
    }

    /// Applies the local effects of removals that just reached this device
    /// through any path: the member projection shrinks for devices the
    /// replayed policy still excludes, and each removal operation forgets the
    /// pairing credentials it revoked exactly once, whichever delivery
    /// carried it here.
    fn applyRemovedMemberEffects(&self, operations: &[SyncOperation]) -> Result<(), String> {
        let service = crate::RuntimeRemoteLinkService::RuntimeRemoteLinkService::newWithRouter(
            (*self.state.localRuntime).clone(), self.state.nodeRouter.clone());
        crate::NodeSpaceService::space_join::applyRemovedMemberLocalEffects(&service, operations)
    }

    /// Checks whether both endpoints are authorized storage providers.
    pub(crate) fn storageReplicationAllowed(&self, targetNodeId: &str) -> Result<bool, String> {
        match self.state.nodeRouter.requirePeerStorageProviders(
            &self.state.nodeRouter.localNodeId(), targetNodeId,
        ) {
            Ok(()) => Ok(true),
            Err(error) if error.code == "SPACE_STORAGE_PERMISSION_DENIED" => Ok(false),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Publishes local outboxes before comparing the same boundary advertised to peers.
    async fn localStateSummary(&self) -> Result<String, String> {
        let store = operit_store::SyncOperationStore::SyncOperationStore::new(
            self.state.localRuntime.runtimeStorageHost(), RUNTIME_SYNC_DIR_PATH);
        store.publishPreferenceOutboxes().map_err(|e|e.to_string())?;
        store.synchronizationSummary().map(|digest|format!("digest:{digest}")).map_err(|e|e.to_string())
    }

    /// Checks pairing-authorized authority metadata before any Space-scoped clock probe.
    async fn pairedPeerPassRequired(
        &self,
        peer: &str,
        localStateSummary: &str,
        cycleDecisions: &mut BTreeMap<String, bool>,
    ) -> Result<bool, String> {
        if self.state.peerGates.lock().map_err(|error| error.to_string())?
            .get(peer).is_some_and(|gate| !gate.backoffElapsed(currentTimeMillis())) {
            cycleDecisions.insert(peer.to_owned(), false);
            return Ok(false);
        }
        let header: crate::NodeSpaceService::space_projection::SnapshotHeader = callRemoteService(
            &self.state.nodeRouter, peer, crate::NodeSpaceService::NODE_SPACE_TARGET,
            "snapshotHeader", Value::Null).await?;
        let local = self.state.spaceStore.space()?;
        let manifest = NetworkControlStore::new(self.state.localRuntime.runtimeStorageHost())?
            .spaceControlManifest(&local.spaceId)?;
        if header.space != local || !header.departures.is_empty() || header.controlRevision != manifest.revision {
            // A changed authority boundary must be reconciled through the pairing.
            // A scoped clock RPC here would reject the very exit that repairs it.
            cycleDecisions.insert(peer.to_owned(), true);
            return Ok(true);
        }
        Ok(self.peerPassRequired(peer, localStateSummary, cycleDecisions).await)
    }

    /// Decides once per cycle whether this cycle must run a full synchronization
    /// pass with one peer, probing its clock when no cheaper signal decides.
    async fn peerPassRequired(
        &self,
        targetNodeId: &str,
        localStateSummary: &str,
        cycleDecisions: &mut BTreeMap<String, bool>,
    ) -> bool {
        if let Some(decision) = cycleDecisions.get(targetNodeId) {
            return *decision;
        }
        let gate = self
            .state
            .peerGates
            .lock()
            .map_err(|error| format!("peer sync gate lock poisoned: {error}"))
            .and_then(|gates| {
                gates
                    .get(targetNodeId)
                    .cloned()
                    .ok_or_else(|| "peer sync gate absent".to_string())
            })
            .unwrap_or_default();
        let mut reasons = Vec::new();
        let decision = if !gate.backoffElapsed(currentTimeMillis()) {
            false
        } else if gate.pendingWork {
            reasons.push("pending_work");
            true
        } else if gate.consecutiveFailures > 0 {
            // Retry the failed exchange after the backoff has elapsed.
            reasons.push("failure_retry");
            true
        } else if !self
            .storageReplicationAllowed(targetNodeId)
            .unwrap_or(false)
        {
            // Non-storage members never enter replication traffic: probing
            // them would only draw a denial, so keep exchanging directly.
            reasons.push("authority_only");
            true
        } else {
            let (probe, networkFailure) = classifySync(self.probeRemoteClockSummary(targetNodeId)).await;
            match probe {
                Ok(summary) => {
                    if gate.lastRemoteClockSummary.is_none() { reasons.push("no_remote_baseline"); }
                    else if gate.lastRemoteClockSummary.as_deref() != Some(summary.as_str()) { reasons.push("remote_clock_changed"); }
                    if gate.lastLocalStateSummary.is_none() { reasons.push("no_local_baseline"); }
                    else if gate.lastLocalStateSummary.as_deref() != Some(localStateSummary) { reasons.push("local_state_changed"); }
                    !reasons.is_empty()
                }
                Err(error) => {
                    self.recordPeerFailure(targetNodeId, networkFailure, &error).await;
                    operit_util::AppLogger::AppLogger::w("SpacePersistenceSyncService", &format!(
                        "sync_gate.probe_failed target={targetNodeId} networkFailure={networkFailure} retryDelayMs={} error={error}",
                        if networkFailure { peerFullSyncBackoffMs(gate.consecutiveFailures.saturating_add(1)) } else { 0 }));
                    !networkFailure
                }
            }
        };
        if decision {
            operit_util::AppLogger::AppLogger::i("SpacePersistenceSyncService", &format!(
                "sync_gate.required target={targetNodeId} reasons={} failures={} pending={}",
                reasons.join(","), gate.consecutiveFailures, gate.pendingWork));
        }
        cycleDecisions.insert(targetNodeId.to_string(), decision);
        decision
    }

    /// Reads the constant-size clock digest used by the peer change gate.
    async fn probeRemoteClockSummary(&self, targetNodeId: &str) -> Result<String, String> {
        let value: Value = callRemote(
            &self.state.nodeRouter,
            targetNodeId,
            PeerSyncMethod::SyncClockDigest,
            Value::Null,
        ).await?;
        value.get("digest")
            .and_then(Value::as_str)
            .filter(|digest| !digest.is_empty())
            .map(|digest| format!("digest:{digest}"))
            .ok_or_else(|| "remote synchronization clock digest is missing".to_string())
    }

    /// Resets retry state without promoting an unverified or pre-pass change probe.
    async fn recordPeerPassCompleted(&self, targetNodeId: &str, businessReplication: bool) -> Result<(), String> {
        let mut gates = self.state.peerGates.lock().map_err(|error| error.to_string())?;
        let gate = gates.entry(targetNodeId.to_string()).or_default();
        if !businessReplication { gate.lastLocalStateSummary = None; gate.lastRemoteClockSummary = None; }
        gate.consecutiveFailures = 0;
        gate.nextFullSyncAllowedAt = 0;
        gate.correctionError = None;
        Ok(())
    }

    /// Coalesces an authenticated peer's wakeup on the same pair coordinator and local gate.
    pub(crate) async fn synchronizeRequestedPeer(&self, target: String, limit: usize, force: bool) -> Result<PeerSyncStatus, operit_link::CoreLinkError> {
        use operit_link::CoreLinkError;
        if self.state.nodeRouter.localNodeId() >= target { return Err(CoreLinkError::internal("Synchronization request targets the wrong coordinator")); }
        let result = Box::pin(self.synchronizeReachablePeerForReason(target.clone(), limit,
            if force { "peer_request_force" } else { "peer_request" })).await;
        let gates = self.state.peerGates.lock().map_err(|e|CoreLinkError::internal(e.to_string()))?;
        let gate = gates.get(&target).ok_or_else(||CoreLinkError::internal("Synchronization gate missing"))?;
        if let Err(error) = result { return Err(CoreLinkError::new(if gate.consecutiveFailures > 0 { "PEER_SYNC_NETWORK_FAILED" } else { "PEER_SYNC_CORRECTION_FAILED" }, error)); }
        if gate.consecutiveFailures > 0 { return Err(CoreLinkError::new("PEER_SYNC_NETWORK_FAILED", "Peer synchronization is waiting for connection")); }
        if let Some(error) = &gate.correctionError { return Err(CoreLinkError::new("PEER_SYNC_CORRECTION_FAILED", error.clone())); }
        Ok(PeerSyncStatus { local: gate.lastLocalStateSummary.clone(), remote: gate.lastRemoteClockSummary.clone(), pending: gate.pendingWork,
            progress: self.peerProgress(&target) })
    }

    /// Records one failed full pass and arms the exponential retry backoff.
    fn recordPeerPassFailed(&self, targetNodeId: &str) {
        if let Ok(mut gates) = self.state.peerGates.lock() {
            let gate = gates.entry(targetNodeId.to_string()).or_default();
            gate.consecutiveFailures = gate.consecutiveFailures.saturating_add(1);
            gate.nextFullSyncAllowedAt =
                currentTimeMillis() + peerFullSyncBackoffMs(gate.consecutiveFailures);
        }
    }

    /// Only transport faults back off; correction failures remain eligible immediately.
    async fn recordPeerFailure(&self, target: &str, network: bool, error: &str) {
        if network { self.recordPeerPassFailed(target); return; }
        if let Ok(mut gates) = self.state.peerGates.lock() {
            let gate = gates.entry(target.to_owned()).or_default();
            gate.correctionError = Some(error.to_owned());
            gate.pendingWork = true;
            gate.consecutiveFailures = 0;
            gate.nextFullSyncAllowedAt = 0;
        }
    }

    /// Test support: clears one peer's failure backoff so the retry path can
    /// be exercised without waiting out the real delay.
    #[cfg(test)]
    pub(crate) fn testExpirePeerSyncBackoff(&self, targetNodeId: &str) {
        if let Ok(mut gates) = self.state.peerGates.lock() {
            if let Some(gate) = gates.get_mut(targetNodeId) {
                gate.nextFullSyncAllowedAt = 0;
            }
        }
    }

    /// Keeps partially materialized work eligible without delaying healthy data behind it.
    fn recordPendingWork(&self, targetNodeId: &str, stage: &str, report: &Value) {
        let pending = report.get("pending").and_then(Value::as_u64).unwrap_or(0);
        let errors = report.get("errors").and_then(Value::as_array);
        let side = match stage {
            "local receipt" | "local apply" => "local",
            "remote receipt" | "remote apply" => "remote",
            _ => stage,
        };
        self.state.progress.report(targetNodeId, side,
            pendingOperationIds(report), if side == "local" || side == "remote" { pending } else { 0 },
            errors.into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect());
        if pending > 0 || errors.is_some_and(|errors| !errors.is_empty()) {
            if let Ok(mut gates) = self.state.peerGates.lock() {
                gates
                    .entry(targetNodeId.to_owned())
                    .or_default()
                    .pendingWork = true;
            }
            operit_util::AppLogger::AppLogger::w(
                "SpacePersistenceSyncService",
                &format!(
                    "sync_exchange.partial target={targetNodeId} stage={stage} pending={pending} errors={errors:?}"
                ),
            );
        }
    }

    /// Advances one independent group without acknowledging failures in another group.
    #[allow(non_snake_case)]
    async fn synchronizeDomains(
        &self,
        targetNodeId: &str,
        limit: usize,
        domains: &[&str],
    ) -> Result<bool, String> {
        let mut seen = BTreeSet::new();
        let mut boundary = None;
        let mut failed = BTreeMap::<String, BTreeSet<String>>::new();
        loop {
            for page in 0..MAX_SYNC_PAGES_PER_PASS {
                if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
                let exchange: Result<bool, String> = async {
                    let (remote, local) = tokio::join!(
                        callRemote::<Value>(&self.state.nodeRouter, targetNodeId, PeerSyncMethod::SyncClock, Value::Null),
                        self.callLocal::<Value>("syncClock", Value::Null),
                    );
                    let (mut local, mut remote) = (local?, remote?);
                    validateSyncReceipt(&local, "local")?;
                    validateSyncReceipt(&remote, "remote")?;
                    boundary = Some((local.clone(), remote.clone()));
                    self.recordPendingWork(targetNodeId, "local receipt", &local);
                    self.recordPendingWork(targetNodeId, "remote receipt", &remote);
                    let mut blobs: Vec<RuntimeFileSyncReference> = serde_json::from_value(local["pendingBlobs"].clone()).map_err(|e|e.to_string())?;
                    blobs.extend(serde_json::from_value::<Vec<RuntimeFileSyncReference>>(remote["pendingBlobs"].clone()).map_err(|e|e.to_string())?);
                    if let Err(error) = self.synchronizeBlobReferences(targetNodeId, blobs, &BTreeMap::new()).await {
                        if SYNC_NETWORK_FAILURE.try_with(std::cell::Cell::get).unwrap_or(false) { return Err(error); }
                        self.recordPendingWork(targetNodeId, "pending blobs", &json!({"errors":[error]}));
                    }
                    local["includeProgress"] = json!(page == 0);
                    remote["includeProgress"] = json!(page == 0);
                    let (remotePage, localPage) = tokio::join!(
                        callRemote::<SyncOperationPage>(&self.state.nodeRouter, targetNodeId, PeerSyncMethod::SyncOperationsSince,
                            json!({"clock":local,"domains":domains,"limit":limit})),
                        self.callLocal::<SyncOperationPage>("syncOperationsSince", json!({"clock":remote,"domains":domains,"limit":limit})),
                    );
                    let (localPage, remotePage) = (localPage?, remotePage?);
                    if page == 0 {
                        let remaining = localPage.report["remaining"].as_u64().zip(remotePage.report["remaining"].as_u64())
                            .map(|(a,b)|a.saturating_add(b));
                        self.state.progress.backlog(targetNodeId, remaining, 0);
                    }
                    self.recordPendingWork(targetNodeId, "local export", &localPage.report);
                    self.recordPendingWork(targetNodeId, "remote export", &remotePage.report);
                    let mut owners = BTreeMap::<String, String>::new();
                    for report in [&localPage.report, &remotePage.report] {
                        if let Some(routes) = report.get("owners") { owners.extend(serde_json::from_value::<BTreeMap<String,String>>(routes.clone()).map_err(|e|e.to_string())?); }
                    }
                    let localExported = exportedOperationIds(&localPage.operations);
                    let remoteExported = exportedOperationIds(&remotePage.operations);
                    let operations = mergeSyncOperations(localPage.operations, remotePage.operations)?;
                    let done = operations.is_empty() || !pageAddsNewOperations(&mut seen, &operations);
                    if !done || operations.is_empty() {
                        let sizes = syncOperationWireSizes(&operations)?;
                        if !self.applySynchronizedOperationBatch(targetNodeId, &operations, &sizes, &owners, &localExported, &remoteExported, &mut failed).await? { return Ok(true); }
                    }
                    boundary = done.then_some((local, remote));
                    Ok(done)
                }.await;
                match exchange {
                    Ok(false) => continue,
                    Ok(true) => break,
                    Err(error) => {
                        if SYNC_NETWORK_FAILURE.try_with(std::cell::Cell::get).unwrap_or(false) { return Err(error); }
                        self.recordPendingWork(targetNodeId, "incremental exchange", &json!({"errors":[error]}));
                        break;
                    }
                }
            }
            // Repeated/capped pages and replay failures all reach the same final correction.
            if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
            self.state.progress.checking(targetNodeId);
            if !self.synchronizeConsistency(targetNodeId, boundary.as_ref().map(|(a,b)|(a,b)), &mut failed).await? {
                boundary = None;
                seen.clear();
                continue;
            }
            let mut gates = self.state.peerGates.lock().map_err(|e|e.to_string())?;
            let gate = gates.entry(targetNodeId.to_owned()).or_default();
            gate.pendingWork = false;
            if let Some((local, remote)) = &boundary {
                gate.lastLocalStateSummary = Some(clockStateSummary(local)?);
                gate.lastRemoteClockSummary = Some(clockStateSummary(remote)?);
            }
            return Ok(true);
        }
    }

    /// Applies one transport page on both peers, split into messages that fit
    /// the connection ceiling, preserving every original operation boundary.
    #[allow(non_snake_case)]
    async fn applySynchronizedOperationBatch(
        &self,
        targetNodeId: &str,
        operations: &[SyncOperation],
        sizes: &[usize],
        owners: &BTreeMap<String, String>,
        localExported: &BTreeSet<String>,
        remoteExported: &BTreeSet<String>,
        failed: &mut BTreeMap<String, BTreeSet<String>>,
    ) -> Result<bool, String> {
        if let Err(error) = self.synchronizeRequiredBlobs(targetNodeId, operations).await {
            if SYNC_NETWORK_FAILURE.try_with(std::cell::Cell::get).unwrap_or(false) { return Err(error); }
            self.recordPendingWork(targetNodeId, "blobs", &json!({"errors":[error]}));
        }
        if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
        self.state.progress.records(targetNodeId, 0);
        let chunks = if operations.is_empty() { vec![operations] }
            else { syncPageChunks(operations, sizes, self.state.nodeRouter.maxPeerMessageBytes())? };
        for chunk in chunks {
            let toRemote = exportedOperations(chunk, localExported);
            let toLocal = exportedOperations(chunk, remoteExported);
            // Empty calls still replay each side's existing pending journal.
            let (remote, local) = tokio::join!(
                callRemote::<Value>(&self.state.nodeRouter, targetNodeId, PeerSyncMethod::SyncApplyOperations, SyncOperationBatch { operations: &toRemote }),
                self.callLocal::<Value>("syncApplyOperations", SyncOperationBatch { operations: &toLocal }),
            );
            if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
            let mut errors = Vec::new();
            let mut pending = BTreeSet::new();
            for (side, result, delivered) in [("remote", remote, &toRemote), ("local", local, &toLocal)] {
                let failedIds = match result {
                    Ok(report) => {
                        self.recordPendingWork(targetNodeId, &format!("{side} apply"), &report);
                        pending.extend(pendingOperationIds(&report));
                        errors.extend(report["errors"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned));
                        report["failedOperationIds"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()
                    }
                    Err(error) => {
                        if SYNC_NETWORK_FAILURE.try_with(std::cell::Cell::get).unwrap_or(false) { return Err(error); }
                        errors.push(format!("{side} apply: {error}"));
                        delivered.iter().map(|op|op.opId.clone()).collect()
                    },
                };
                for id in failedIds {
                    let owner = owners.get(&id).ok_or_else(||format!("Failed transaction has no correction owner: {id}"))?;
                    failed.entry(owner.clone()).or_default().insert(id);
                }
            }
            if !errors.is_empty() { return Err(errors.join("; ")); }
            let confirmed = chunk.iter().map(|operation| operation.opId.as_str())
                .filter(|id| !pending.contains(*id)).map(str::to_owned).collect::<Vec<_>>();
            self.state.progress.records(targetNodeId, confirmed.len());
            self.state.progress.confirmRecords(targetNodeId, &confirmed);
        }
        Ok(true)
    }

    /// Exchanges authenticated Device Space projections and reports whether business sync is allowed.
    #[allow(non_snake_case)]
    pub(crate) async fn exchangePairedDeviceSpaceProjection(
        &self,
        peerNodeId: &str,
    ) -> Result<bool, String> {
        let protocol = projectionProtocol(&self.state.nodeRouter, peerNodeId).await?;
        let service = crate::RuntimeRemoteLinkService::RuntimeRemoteLinkService::newWithRouter(
            (*self.state.localRuntime).clone(), self.state.nodeRouter.clone());
        let mut referenceRevision = None;
        let remote = if protocol.as_ref().is_some_and(|protocol| protocol.snapshotPaging) {
            let (revision, snapshot) = crate::NodeSpaceService::space_projection::readSnapshot(&service, peerNodeId).await?;
            referenceRevision = Some(revision);
            snapshot
        } else {
            callRemoteService(&self.state.nodeRouter, peerNodeId,
                crate::NodeSpaceService::NODE_SPACE_TARGET, "snapshot", Value::Null).await?
        };
        let remoteSpaceId = remote.space.spaceId.clone();
        let remoteHasDepartures = !remote.departures.is_empty();
        let snapshotControlOperations = remote.controlOperations.clone();
        service.observePeerSpaceSnapshot(peerNodeId, remote)?;
        self.applyRemovedMemberEffects(&snapshotControlOperations)?;
        let local = crate::NodeSpaceService::peerSpaceSnapshotForPeer(&service, peerNodeId)?;
        let useReference = protocol.as_ref().is_some_and(|protocol| protocol.projectionReference)
            && referenceRevision.is_some() && local.space.spaceId == remoteSpaceId
            && !remoteHasDepartures && local.departures.is_empty()
            && crate::NodeSpaceService::space_projection::sameControlOperations(
                &local.controlOperations, &snapshotControlOperations);
        drop(snapshotControlOperations);
        let (method, arguments) = if useReference {
            use crate::NodeSpaceService::space_projection::ProjectionObservation;
            let observation = ProjectionObservation { controlRevision: referenceRevision.unwrap(),
                space: local.space, deviceProfiles: local.deviceProfiles, topology: local.topology };
            ("observeSpaceProjection", serde_json::to_value(observation))
        } else {
            ("observeSpaceSnapshot", serde_json::to_value(local))
        };
        let request = serviceCallRequest(&self.state.nodeRouter,
            crate::NodeSpaceService::NODE_SPACE_TARGET, method,
            arguments.map_err(|error| error.to_string())?)?;
        if let Some(protocol) = &protocol {
            crate::NodeSpaceService::space_projection::requireRoutedRequestFits(&request,
                &self.state.spaceStore.space()?.spaceId, &self.state.nodeRouter.localNodeId(),
                peerNodeId, 0, protocol.maxMessageBytes)?;
        }
        let response = self.state.nodeRouter.callNode(peerNodeId.to_owned(), request).await;
        let remoteSpace: CoreSpace = decodeCoreResponse(response.result.map_err(syncLinkError)?)?;
        Ok(self.state.spaceStore.space()?.spaceId == remoteSpace.spaceId)
    }

    /// Validates that one reachable CoreNode belongs to the same synchronized Device Space.
    #[allow(non_snake_case)]
    async fn validateReachableDeviceSpace(&self, targetNodeId: &str) -> Result<(), String> {
        let localSpace = self.state.spaceStore.initialize()?;
        let remoteSpace: CoreSpace = callRemote(
            &self.state.nodeRouter,
            targetNodeId,
            PeerSyncMethod::DeviceSpace,
            Value::Null,
        )
        .await?;
        if !remoteSpace
            .members
            .iter()
            .any(|member| member == targetNodeId)
        {
            return Err(
                "route synchronization target is not present in its announced device space"
                    .to_string(),
            );
        }
        if !remoteSpace
            .members
            .iter()
            .any(|member| member == &self.state.nodeRouter.localNodeId())
        {
            return Err(
                "route synchronization source is not present in the target device space"
                    .to_string(),
            );
        }
        if localSpace.spaceId != remoteSpace.spaceId {
            return Err(format!(
                "route synchronization space mismatch: local={}, target={}",
                localSpace.spaceId, remoteSpace.spaceId
            ));
        }
        Ok(())
    }

    /// Transfers every content-addressed blob required by a synchronization page.
    async fn synchronizeRequiredBlobs(
        &self,
        targetNodeId: &str,
        operations: &[SyncOperation],
    ) -> Result<(), String> {
        let mut errors = Vec::new();
        let mut references = Vec::new();
        let mut sources = BTreeMap::<String, Vec<String>>::new();
        for value in operations {
            match RuntimeFileSyncStore::requiredBlob(value) {
                Ok(Some(reference)) => {
                    let ids = sources.entry(reference.contentHash.clone()).or_default();
                    if ids.len() < 2 { ids.push(value.opId.clone()); }
                    references.push(reference);
                }
                Ok(None) => {}
                Err(error) => { errors.push(error); }
            }
        }
        if let Err(error) = self.synchronizeBlobReferences(targetNodeId, references, &sources).await { errors.push(error); }
        if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
    }

    /// Overlaps independent blob transfers on the existing multiplexed channel with bounded buffers.
    /// The attribution map names the operations requiring each hash so a full
    /// transfer logs which original or correction snapshot demanded it.
    async fn synchronizeBlobReferences(
        &self,
        targetNodeId: &str,
        references: Vec<RuntimeFileSyncReference>,
        attribution: &BTreeMap<String, Vec<String>>,
    ) -> Result<(), String> {
        let references = references.into_iter()
            .map(|reference|((reference.contentHash.clone(),reference.size),reference))
            .collect::<BTreeMap<_,_>>().into_values().collect::<Vec<_>>();
        if references.is_empty() { return Ok(()); }
        let timing = std::time::Instant::now();
        let (remote,local) = tokio::join!(biased;
            self.blobPresence(targetNodeId,&references,false), self.blobPresence(targetNodeId,&references,true));
        let mut errors = Vec::new();
        let mut pending = Vec::new();
        for ((reference,local),remote) in references.iter().zip(local?).zip(remote?) {
            match local.and_then(|local| remote.map(|remote| (local, remote))) {
                Ok((true,true)) => {},
                Ok((false,false)) => errors.push(format!("blob {} is missing from both direct peers",reference.contentHash)),
                Ok((local,_)) => {
                    let ops = attribution.get(&reference.contentHash)
                        .map(|ids| ids.join(",")).unwrap_or_else(|| "receipt".to_string());
                    operit_util::AppLogger::AppLogger::i("CoreSyncTrace", &format!(
                        "sync_blobs.required target={targetNodeId} hash={} bytes={} holder={} ops={}",
                        reference.contentHash.get(..12).unwrap_or(&reference.contentHash), reference.size, if local {"local"} else {"remote"}, ops));
                    pending.push((reference,local));
                },
                Err(error) => errors.push(error),
            }
        }
        let presence_ms = timing.elapsed().as_millis();
        let missing = pending.len();
        let mut transferred_bytes = 0;
        if !pending.is_empty() {
            let size = pending.iter().try_fold(0i64, |size, (reference,_)|
                size.checked_add(reference.size).ok_or("Synchronization blob batch size overflow"))?;
            transferred_bytes = size;
            RuntimeFileSyncStore::new(self.state.localRuntime.runtimeStorageHost(), RUNTIME_SYNC_DIR_PATH)
                .prepareBlobReads(&pending.iter().filter(|(_, local)| *local).map(|(reference, _)| *reference).collect::<Vec<_>>())?;
            let sent = AtomicI64::new(0);
            self.state.progress.beginFile(targetNodeId, size);
            errors.extend(stream::iter(pending)
                .map(|(reference,source)| self.transferBlob(targetNodeId,reference,source,&sent))
                .buffer_unordered(SYNC_BLOB_TRANSFER_CONCURRENCY)
                .filter_map(|result| async move { result.err() }).collect::<Vec<_>>().await);
            self.state.progress.fileOffset(targetNodeId, sent.load(Ordering::Relaxed), true);
        }
        if timing.elapsed().as_millis() >= 100 {
            operit_util::AppLogger::AppLogger::trace("CoreSyncTrace", &format!(
                "sync_blobs.stages references={} missing={missing} bytes={transferred_bytes} presenceMs={presence_ms} transferMs={} totalMs={}",
                references.len(), timing.elapsed().as_millis()-presence_ms, timing.elapsed().as_millis()));
        }
        if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
    }

    /// Checks one page in one dispatch per endpoint, retaining individual blob failures.
    async fn blobPresence(&self, target: &str, references: &[RuntimeFileSyncReference], local: bool) -> Result<Vec<Result<bool,String>>,String> {
        let args = json!({"references":references});
        let presence: Vec<Result<bool,String>> = if local { self.callLocal("syncBlobExists",args).await? }
            else { callRemote(&self.state.nodeRouter,target,PeerSyncMethod::SyncBlobExists,args).await? };
        if presence.len() != references.len() { return Err("Blob presence response length differs".into()); }
        Ok(presence)
    }

    /// Both directions share authorization, chunk validation, progress and final verification.
    async fn transferBlob(
        &self,
        targetNodeId: &str,
        reference: &RuntimeFileSyncReference,
        localSource: bool,
        sent: &AtomicI64,
    ) -> Result<(), String> {
        self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
            .map_err(|error| error.to_string())?;
        let target = if localSource { NODE_SYNC_TARGET } else {
            self.state.nodeRouter.targetForSchema("services.syncBlobTransferManager")
                .ok_or("unknown Core schema key: services.syncBlobTransferManager")?
        };
        let request = CorePushRequest::new(operit_link::nextCoreRouteRequestId("space-persistence-blob"), target, "syncReceiveBlob")
            .withArgs(toCoreValue(reference).map_err(|error| error.to_string())?);
        let mut push = if localSource {
            self.state.nodeRouter.openPushNode(targetNodeId.to_string(), request).await
                .map_err(syncLinkError)?
        } else {
            self.state.localRuntime.openPush(request).map_err(|error| error.to_string())?
        };
        let localFiles = localSource.then(|| RuntimeFileSyncStore::new(
            self.state.localRuntime.runtimeStorageHost(), RUNTIME_SYNC_DIR_PATH));
        let mut offset = 0;
        while offset < reference.size {
            self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                .map_err(|error| error.to_string())?;
            let chunk = if let Some(files) = &localFiles {
                files.readBlobChunk(&reference.contentHash, offset, SYNC_BLOB_CHUNK_BYTES)?
            } else {
                let args = json!({ "contentHash": reference.contentHash, "offset": offset, "length": SYNC_BLOB_CHUNK_BYTES });
                callRemote::<Vec<u8>>(&self.state.nodeRouter, targetNodeId, PeerSyncMethod::SyncReadBlobChunk, args).await?
            };
            self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                .map_err(|error| error.to_string())?;
            let length = i64::try_from(chunk.len()).map_err(|error| error.to_string())?;
            if length == 0 || length > reference.size - offset {
                return Err(format!("Invalid synchronization blob chunk: hash={} offset={offset} length={length} size={}", reference.contentHash, reference.size));
            }
            // Link already accepts binary as Vec<u8>; avoid allocating one value per byte.
            push.send(operit_link::CoreValue::Bytes(chunk.into())).await.map_err(syncLinkError)?;
            offset += length;
            self.state.progress.fileOffset(targetNodeId, sent.fetch_add(length, Ordering::Relaxed) + length, false);
        }
        self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
            .map_err(|error| error.to_string())?;
        push.close().await.map_err(syncLinkError)?;
        let persisted = self.blobPresence(targetNodeId,std::slice::from_ref(reference),!localSource).await?
            .pop().ok_or("Blob presence response missing")??;
        if !persisted { return Err(format!("CoreNode did not persist synchronization blob {}", reference.contentHash)); }
        Ok(())
    }

    /// Coalesces join, storage and connectivity changes into the existing worker.
    pub(crate) fn scheduleSynchronization(&self, reason: &'static str) -> Result<(), String> {
        self.state.synchronizationReasons.lock().unwrap_or_else(|error| error.into_inner()).insert(reason);
        self.state.synchronizationRequested.store(true, Ordering::Release);
        if self
            .state
            .synchronizationScheduled
            .swap(true, Ordering::AcqRel)
        {
            return Ok(());
        }
        let service = self.clone();
        // The sync pass performs synchronous storage work between its remote awaits.
        let scheduleResult = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeBlockingAsyncTask(
            "core-node-space-persistence-sync",
            Box::new(move || {
                Box::pin(async move {
                    if !service.state.active.load(Ordering::Acquire) {
                        service
                            .state
                            .synchronizationScheduled
                            .store(false, Ordering::Release);
                        return;
                    }
                    service.state.synchronizationRequested.store(false, Ordering::Release);
                    let reasons = std::mem::take(&mut *service.state.synchronizationReasons.lock()
                        .unwrap_or_else(|error| error.into_inner()));
                    // A workspace tick has already scanned (and reported any failure).
                    // Other wakeups keep the immediate scan before exchanging with peers.
                    let scanWorkspace = !reasons.contains(WORKSPACE_CHANGED) && !reasons.contains(WORKSPACE_RETRY_POLL);
                    let reasons = reasons.into_iter().collect::<Vec<_>>().join(",");
                    let synchronizedRevision = syncMutationRevision();
                    let syncStartedAt = currentTimeMillis();
                    let localNodeId = service.state.nodeRouter.localNodeId();
                    operit_util::AppLogger::AppLogger::v_with_level(
                        "SpacePersistenceSyncService",
                        &format!(
                            "sync_cycle.start local={} revision={} triggers={}",
                            localNodeId, synchronizedRevision, reasons
                        ),
                        operit_util::AppLogger::VERBOSE_LEVEL_5,
                    );
                    match crate::fault_isolation::operation("Space synchronization pass", service.synchronizeCycle(scanWorkspace)).await {
                        Ok(()) => {
                            operit_util::AppLogger::AppLogger::v_with_level(
                                "SpacePersistenceSyncService",
                                &format!(
                                    "sync_cycle.done local={} revision={} elapsedMs={}",
                                    localNodeId,
                                    synchronizedRevision,
                                    currentTimeMillis() - syncStartedAt
                                ),
                                operit_util::AppLogger::VERBOSE_LEVEL_5,
                            );
                        }
                        Err(error) => {
                            operit_util::AppLogger::AppLogger::w(
                                "SpacePersistenceSyncService",
                                &format!(
                                    "sync_cycle.failed local={} revision={} elapsedMs={} error={}",
                                    localNodeId,
                                    synchronizedRevision,
                                    currentTimeMillis() - syncStartedAt,
                                    error
                                ),
                            );
                        }
                    }
                    service
                        .state
                        .synchronizationScheduled
                        .store(false, Ordering::Release);
                    if service.state.active.load(Ordering::Acquire)
                        && (service.state.synchronizationRequested.load(Ordering::Acquire)
                            || syncMutationRevision() != synchronizedRevision)
                    {
                        if let Err(error) = service.scheduleSynchronization("changes_during_pass") {
                            operit_util::AppLogger::AppLogger::e(
                                "SpacePersistenceSyncService",
                                &format!("Space persistence sync rescheduling failed: {error}"),
                            );
                        }
                    }
                })
            }),
        );
        if let Err(error) = scheduleResult {
            self.state
                .synchronizationScheduled
                .store(false, Ordering::Release);
            self.state.active.store(false, Ordering::Release);
            return Err(error.to_string());
        }
        Ok(())
    }

    /// Invokes one local application method through the active in-process Core.
    async fn callLocal<T>(&self, methodName: &str, args: impl Serialize) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let request = serviceCallRequest(&self.state.nodeRouter, "application", methodName, args)?;
        let startedAt = currentTimeMillis();
        let response = self.state.localRuntime.callApplication(request).await;
        recordSlowSyncCall("local", methodName, startedAt);
        decodeCoreResponse(response.result.map_err(|error| {
            format!("local {methodName}: {error}")
        })?)
    }
}

/// Serializes directly into Link values without a second owned JSON operation page.
#[derive(Serialize)]
struct SyncOperationBatch<'a, T = Value> {
    operations: &'a [T],
}

/// Weak registration shares one worker across facades without extending its lifetime.
fn persistenceServices() -> &'static Mutex<BTreeMap<String, Weak<SpacePersistenceSyncState>>> {
    SPACE_SYNC_SERVICES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Validates receipt intervals after the unified peer protocol has been admitted.
#[allow(non_snake_case)]
fn validateSyncReceipt(clock: &Value, side: &str) -> Result<(), String> {
    let receipt: operit_store::SyncOperationStore::SyncReceipt =
        serde_json::from_value(clock.clone())
            .map_err(|error| format!("{side} synchronization receipt: {error}"))?;
    receipt
        .validate()
        .map_err(|error| format!("{side} synchronization receipt: {error}"))
}

/// Refuses a scheduling acknowledgement that is not tied to the verified exchange.
fn clockStateSummary(clock: &Value) -> Result<String, String> {
    clock.get("stateSummary").and_then(Value::as_str).filter(|summary| !summary.is_empty())
        .map(str::to_owned).ok_or_else(|| "Synchronization state summary missing".into())
}

/// Invokes one application method through the Router.
async fn callRemote<T>(
    nodeRouter: &CoreNodeRouter,
    targetNodeId: &str,
    method: PeerSyncMethod,
    args: impl Serialize,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let request = method.request(
        operit_link::nextCoreRouteRequestId("space-persistence"),
        toCoreValue(args).map_err(|error| error.to_string())?,
    );
    let methodName = request.methodName.clone();
    let startedAt = currentTimeMillis();
    let response = nodeRouter.callNode(targetNodeId.to_string(), request).await;
    recordSlowSyncCall(targetNodeId, &methodName, startedAt);
    decodeCoreResponse(response.result.map_err(syncLinkError)?)
}

/// Negotiates bounded transport with direct authenticated peers; relayed Core routes use their existing contract.
async fn projectionProtocol(
    router: &CoreNodeRouter, peer: &str,
) -> Result<Option<crate::NodeSpaceService::space_projection::ProjectionProtocol>, String> {
    let peers = router.nodeServices()?.peers();
    // Endpoint negotiation belongs to a direct authenticated neighbor. Ordinary
    // Core control/business synchronization must still work across relay hops.
    if !peers.activePeerNodeIds().map_err(|error| error.to_string())?.contains(peer)
        || !peers.hasPairing(peer, crate::NodeServices::PairingDirection::Either).map_err(|error| error.to_string())? {
        return Ok(None);
    }
    let request = serviceCallRequest(router, crate::NodeSpaceService::NODE_SPACE_TARGET,
        "projectionProtocol", json!({"version": crate::NodeSpaceService::space_projection::VERSION}))?;
    let response = router.callNode(peer.to_owned(), request).await;
    let mut protocol = crate::NodeSpaceService::space_projection::decodeProtocolResponse(peer, response.result.map_err(|error| {
        syncLinkError(error.clone()); error
    }))?;
    protocol.maxMessageBytes = protocol.maxMessageBytes.min(peers.maxMessageBytes());
    Ok(Some(protocol))
}

/// Invokes one generated service method through the Router.
#[allow(non_snake_case)]
async fn callRemoteService<T>(
    nodeRouter: &CoreNodeRouter,
    targetNodeId: &str,
    targetPath: &str,
    methodName: &str,
    args: Value,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let request = serviceCallRequest(nodeRouter, targetPath, methodName, args)?;
    let startedAt = currentTimeMillis();
    let response = nodeRouter.callNode(targetNodeId.to_string(), request).await;
    recordSlowSyncCall(targetNodeId, methodName, startedAt);
    decodeCoreResponse(response.result.map_err(syncLinkError)?)
}

/// Reads exact unapplied identities from the existing receipt and application reports.
fn pendingOperationIds(report: &Value) -> BTreeSet<String> {
    report.get("pendingOperationIds").and_then(Value::as_array).into_iter().flatten()
        .filter_map(Value::as_str).map(str::to_owned).collect()
}

/// Reports slow synchronization steps without logging business payloads or adding another dispatch path.
fn recordSlowSyncCall(target: &str, method: &str, startedAt: i64) {
    let elapsedMs = currentTimeMillis() - startedAt;
    if elapsedMs >= 250 {
        operit_util::AppLogger::AppLogger::trace("CoreSyncTrace", &format!(
            "sync_call.slow target={target} method={method} elapsedMs={elapsedMs}"
        ));
    }
}

/// Builds one Link request for a generated service operation.
#[allow(non_snake_case)]
fn serviceCallRequest(
    nodeRouter: &CoreNodeRouter,
    targetPath: &str,
    methodName: &str,
    args: impl Serialize,
) -> Result<CoreCallRequest, String> {
    let target = if targetPath == crate::RuntimeRemoteLinkService::NODE_SPACE_TARGET
        || targetPath == crate::NodeSpaceService::NODE_SPACE_CONTROL_TARGET {
        targetPath
    } else {
        nodeRouter.targetForSchema(targetPath)
            .ok_or_else(|| format!("unknown Core schema key: {targetPath}"))?
    };
    Ok(CoreCallRequest::new(
        operit_link::nextCoreRouteRequestId(&format!("space-persistence-{methodName}")),
        target,
        methodName,
        toCoreValue(args).map_err(|error| error.to_string())?,
    ))
}

/// Decodes one successful Link response into its declared transport type.
#[allow(non_snake_case)]
fn decodeCoreResponse<T>(value: CoreValue) -> Result<T, String>
where
    T: DeserializeOwned,
{
    fromCoreValue(value).map_err(|error| error.to_string())
}

/// Decode originals directly; only small reporting fields use the generic JSON map.
#[derive(Deserialize)]
struct SyncOperationPage {
    operations: Vec<SyncOperation>,
    #[serde(flatten)]
    report: Value,
}

/// An exporter already durably owns its page; only the opposite side receives it.
fn exportedOperationIds(page: &[SyncOperation]) -> BTreeSet<String> {
    page.iter().map(|op| op.opId.clone()).collect()
}

fn exportedOperations<'a>(operations: &'a [SyncOperation], exported: &BTreeSet<String>) -> Vec<&'a SyncOperation> {
    operations.iter().filter(|op| exported.contains(&op.opId)).collect()
}

/// Merge originals once, preserving the shared identity checks and business order.
fn mergeSyncOperations(left: Vec<SyncOperation>, right: Vec<SyncOperation>) -> Result<Vec<SyncOperation>, String> {
    operit_store::Transaction::ordered(left.into_iter().chain(right), |op| &op.opId,
        operit_store::Transaction::Revision::fromOperation)
}

/// Encoded MessagePack size of every operation, measured once per page.
/// The same sizes drive the chunking decision and the batch trace.
#[allow(non_snake_case)]
fn syncOperationWireSizes(operations: &[SyncOperation]) -> Result<Vec<usize>, String> {
    operations
        .iter()
        .map(|operation| {
            operit_link::encodeLink(operation)
                .map(|bytes| bytes.len())
                .map_err(|error| error.to_string())
        })
        .collect()
}

/// Largest single encoded operation the Link can carry with its request envelope.
fn syncOperationCeiling(maxMessageBytes: usize) -> usize {
    maxMessageBytes.saturating_sub(SYNC_PAGE_ENVELOPE_BYTES)
}

/// Wire budget for one synchronization page message under a connection ceiling.
/// Slack stays well below the ceiling so the envelope and MessagePack container
/// headers never push a measured page over the transport limit.
fn syncPageBudget(maxMessageBytes: usize) -> usize {
    (maxMessageBytes.saturating_mul(3) / 4).saturating_sub(SYNC_PAGE_HEADROOM_BYTES)
}



/// Splits one ordered, deduplicated page into contiguous slices that each fit
/// the Link ceiling. Operations stay whole, so a transaction is never divided.
/// A single operation wider than the page budget still travels alone; one wider
/// than the ceiling itself is reported here instead of failing at the socket.
#[allow(non_snake_case)]
fn syncPageChunks<'a>(
    operations: &'a [SyncOperation],
    sizes: &[usize],
    maxMessageBytes: usize,
) -> Result<Vec<&'a [SyncOperation]>, String> {
    if operations.len() != sizes.len() {
        return Err("synchronization page size table is incomplete".into());
    }
    let ceiling = syncOperationCeiling(maxMessageBytes);
    let budget = syncPageBudget(maxMessageBytes);
    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut bytes = 0usize;
    for (index, size) in sizes.iter().copied().enumerate() {
        if size > ceiling {
            let opId = &operations[index].opId;
            let domain = &operations[index].domain;
            return Err(format!(
                "SYNC_OPERATION_TOO_LARGE opId={opId} domain={domain} bytes={size} limit={ceiling}"
            ));
        }
        if index > start && bytes.saturating_add(size) > budget {
            chunks.push(&operations[start..index]);
            start = index;
            bytes = 0;
        }
        bytes = bytes.saturating_add(size);
    }
    if start < operations.len() {
        chunks.push(&operations[start..]);
    }
    Ok(chunks)
}

/// Records one page's operation ids and reports whether it added a new identity.
///
/// Malformed operation envelopes are rejected while decoding the page.
#[allow(non_snake_case)]
fn pageAddsNewOperations(
    seenOperationIds: &mut BTreeSet<String>,
    operations: &[SyncOperation],
) -> bool {
    let pageOperationIds = operations
        .iter()
        .map(|operation| operation.opId.clone())
        .collect::<BTreeSet<_>>();
    if pageOperationIds.len() == operations.len()
        && pageOperationIds
            .iter()
            .all(|opId| seenOperationIds.contains(opId))
    {
        return false;
    }
    seenOperationIds.extend(pageOperationIds);
    true
}

#[cfg(test)]
mod page_budget_tests {
    use super::*;

    #[test]
    fn borrowed_application_batch_preserves_wire_arguments() {
        let operations = vec![json!({"opId":"peer:1","payload":{"unicode":"配置","bytes":[0,255],"null":null}})];
        assert_eq!(toCoreValue(SyncOperationBatch { operations: &operations }).unwrap(),
            toCoreValue(json!({"operations":operations})).unwrap());
        assert_eq!(toCoreValue(SyncOperationBatch::<Value> { operations: &[] }).unwrap(),
            toCoreValue(json!({"operations":[]})).unwrap());
    }

    #[test]
    fn typed_export_page_preserves_originals_and_reporting_fields() {
        let operations = vec![operation("device:1", 4096)];
        let report = json!({"operations":operations, "remaining":3, "pending":1,
            "pendingOperationIds":["retry:1"], "owners":{"device:1":"runtime/data/test.json"},
            "errors":["retry file"], "pendingBlobs":[{"contentHash":"abc","size":4096}]});
        let page: SyncOperationPage = fromCoreValue(toCoreValue(&report).unwrap()).unwrap();
        assert_eq!(page.operations, operations);
        let mut expected = report;
        expected.as_object_mut().unwrap().remove("operations");
        assert_eq!(page.report, expected);
    }

    #[test]
    fn directional_pages_keep_order_overlap_and_empty_replay() {
        let operations = vec![operation("left:1", 0), operation("both:1", 0), operation("right:1", 0)];
        let left = exportedOperationIds(&[operations[0].clone(), operations[1].clone()]);
        let right = exportedOperationIds(&[operations[2].clone(), operations[1].clone()]);
        let to_remote = exportedOperations(&operations, &left);
        let to_local = exportedOperations(&operations, &right);
        assert_eq!(to_remote.iter().map(|op|op.opId.as_str()).collect::<Vec<_>>(), vec!["left:1","both:1"]);
        assert_eq!(to_local.iter().map(|op|op.opId.as_str()).collect::<Vec<_>>(), vec!["both:1","right:1"]);
        assert!(exportedOperations(&operations, &BTreeSet::new()).is_empty());
        assert_eq!(toCoreValue(SyncOperationBatch { operations: &to_remote }).unwrap(),
            toCoreValue(json!({"operations":[operations[0], operations[1]]})).unwrap());
    }

    /// Identical messages cannot turn application corruption into a connection retry.
    #[tokio::test]
    async fn retry_classification_uses_transport_codes() {
        for (code, expected) in [("PEER_SEND_FAILED",true), ("PEER_RESPONSE_CLOSED",true),
            ("PEER_SYNC_NETWORK_FAILED",true), ("PEER_SECURITY",true), ("INTERNAL_ERROR",false), ("PEER_SYNC_CORRECTION_FAILED",false)] {
            let (_, network) = classifySync(async {
                Err::<(),_>(syncLinkError(operit_link::CoreLinkError::new(code,"database is corrupt")))
            }).await;
            assert_eq!(network,expected,"{code}");
        }
    }

    /// Keeps core clocks usable while a plugin clock is explicitly unavailable.
    #[test]
    fn plugin_clock_failures_are_isolated_from_core_domains() {
        let clock = json!({
            "ranges": {"core": [[1,7]]},
            "errors": ["invalid plugin catalog"],
        });
        assert!(validateSyncReceipt(&clock, "local").is_ok());
        let durable: operit_store::SyncOperationStore::SyncReceipt = serde_json::from_value(clock).unwrap();
        assert_eq!(durable.ranges["core"], vec![[1,7]]);
    }

    /// Clears a plugin clock error only when the actual clock can be read again.
    #[test]
    fn recovered_plugin_clock_can_synchronize_again() {
        let clock = json!({"ranges": {"plugin": [[1,2]]}, "errors": []});
        assert!(validateSyncReceipt(&clock, "local").is_ok());
    }



    /// Constructs one transport-sized plugin operation for page budget tests.
    fn operation(opId: &str, payloadBytes: usize) -> SyncOperation {
        serde_json::from_value(json!({
            "opId": opId, "originDeviceId":"device", "sequence":1,
            "domain": "plugin_storage", "entityType":"test", "entityId":"entity",
            "operation":"set", "semantics":"transaction", "createdAt":1, "schemaVersion":1,
            "payload": "x".repeat(payloadBytes),
        })).unwrap()
    }

    /// Extracts operation identities from one planned message chunk.
    fn ids(chunk: &[SyncOperation]) -> Vec<String> {
        chunk
            .iter()
            .map(|value| value.opId.clone())
            .collect()
    }

    fn mergeValues(left: Value, right: Value) -> Result<Vec<SyncOperation>, String> {
        super::mergeSyncOperations(serde_json::from_value(left).map_err(|e| e.to_string())?,
            serde_json::from_value(right).map_err(|e| e.to_string())?)
    }

    /// Transport rejects a changed original regardless of arrival direction.
    #[test]
    fn merge_pages_rejects_changed_immutable_chat_original_in_both_orders() {
        let old = json!({
            "opId":"device:1", "originDeviceId":"device", "sequence":1,
            "domain":"chat", "entityType":"message", "entityId":"chat:1",
            "operation":"upsert", "semantics":"entity_state",
            "payload":{"text":"old"}, "createdAt":1, "schemaVersion":7,
        });
        let mut new = old.clone();
        new["schemaVersion"] = json!(8);
        new["payload"] = json!({"text":"new"});
        for (left, right) in [(old.clone(), new.clone()), (new.clone(), old.clone())] {
            assert!(mergeValues(json!([left]), json!([right])).is_err());
        }
        let mut control = old.clone(); control["domain"] = json!("network_control");
        let mut conflict = control.clone(); conflict["payload"] = json!({"permission":"admin"});
        assert!(mergeValues(json!([control]), json!([conflict])).is_err());
    }

    #[test]
    fn page_chunks_stay_within_the_budget_and_keep_order() {
        let operations = (0..5)
            .map(|index| operation(&format!("op-{index}"), 50_000))
            .collect::<Vec<_>>();
        let sizes = syncOperationWireSizes(&operations).unwrap();
        let ceiling = 256 * 1024;
        let chunks = syncPageChunks(&operations, &sizes, ceiling).unwrap();
        assert!(chunks.len() > 1);
        let ordered = chunks
            .iter()
            .flat_map(|chunk| ids(chunk))
            .collect::<Vec<_>>();
        assert_eq!(
            ordered,
            vec!["op-0", "op-1", "op-2", "op-3", "op-4"]
        );
        for chunk in &chunks {
            let bytes: usize = syncOperationWireSizes(chunk).unwrap().iter().sum();
            // A chunk either holds several operations inside the page budget,
            // or exactly one operation wider than the budget itself.
            assert!(bytes <= syncPageBudget(ceiling) || chunk.len() == 1);
        }
    }

    #[test]
    fn single_operation_wider_than_the_budget_travels_alone() {
        let operations = vec![
            operation("small", 10_000),
            operation("wide", 200_000),
            operation("tail", 10_000),
        ];
        let sizes = syncOperationWireSizes(&operations).unwrap();
        let chunks = syncPageChunks(&operations, &sizes, 256 * 1024).unwrap();
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| ids(chunk))
                .collect::<Vec<_>>(),
            vec![
                vec!["small".to_string()],
                vec!["wide".to_string()],
                vec!["tail".to_string()],
            ]
        );
    }

    #[test]
    fn operation_above_the_ceiling_is_reported_not_sent() {
        let operations = vec![operation("huge", 5_000)];
        let sizes = syncOperationWireSizes(&operations).unwrap();
        let error = syncPageChunks(&operations, &sizes, 8 * 1024).unwrap_err();
        assert!(error.starts_with("SYNC_OPERATION_TOO_LARGE"), "{error}");
        assert!(error.contains("opId=huge"), "{error}");
        assert!(error.contains("domain=plugin_storage"), "{error}");
    }



    #[test]
    fn an_empty_page_has_no_chunks() {
        assert!(syncPageChunks(&[], &[], 4 * 1024 * 1024).unwrap().is_empty());
        assert!(syncOperationWireSizes(&[]).unwrap().is_empty());
    }

    #[test]
    fn page_union_uses_the_common_business_order_and_rejects_invalid_originals() {
        let mut first = operation("device:1", 4096);
        first.createdAt = 20;
        let mut second = first.clone();
        second.opId = "device:2".into(); second.sequence = 2; second.createdAt = 10;
        assert_eq!(mergeValues(json!([second.clone(), first.clone()]), json!([first.clone()])).unwrap(), vec![first,second]);
        assert!(mergeValues(json!([{"opId":"device:7"}]), json!([])).is_err());
    }

    #[test]
    fn repeated_page_without_new_ids_ends_the_pass() {
        let mut seen = BTreeSet::new();
        let page = vec![operation("op-a", 1), operation("op-b", 1)];
        assert!(pageAddsNewOperations(&mut seen, &page));
        assert!(!pageAddsNewOperations(&mut seen, &page));
        let progressed = vec![operation("op-b", 1), operation("op-c", 1)];
        assert!(pageAddsNewOperations(&mut seen, &progressed));
        assert!(!pageAddsNewOperations(&mut seen, &progressed));
        // A page without complete identities must never stop a pass.
        assert!(fromCoreValue::<SyncOperationPage>(toCoreValue(json!({"operations":[{"domain":"chat"}]})).unwrap()).is_err());
    }
    #[test]
    fn blob_chunks_use_existing_binary_wire_and_decode_as_legacy_byte_vectors() {
        let chunk = (0..65536).map(|index| (index % 256) as u8).collect::<Vec<_>>();
        let encoded = operit_link::encodeLink(operit_link::CoreValue::Bytes(chunk.clone().into())).unwrap();
        assert!(encoded.len() < chunk.len()+16);
        let wire: operit_link::CoreValue = operit_link::decodeLink(&encoded).unwrap();
        assert_eq!(operit_link::fromCoreValue::<Vec<u8>>(wire).unwrap(), chunk);
        let legacy = operit_link::toCoreValue(&chunk).unwrap();
        assert_eq!(operit_link::fromCoreValue::<Vec<u8>>(legacy).unwrap(), chunk);
    }

}
