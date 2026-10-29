use crate::PeerSync::{PeerSyncMethod, NODE_SYNC_TARGET};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CorePushRequest, CoreValue};
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceStore};
use operit_store::NetworkControlStore::NetworkControlStore;
use operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use operit_store::RuntimeFileSyncStore::{RuntimeFileSyncReference, RuntimeFileSyncStore};
use operit_store::SyncOperationStore::{subscribeSyncMutations, syncMutationRevision, SyncMutationSubscription, SyncOperation};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use operit_store::PreferencesDataStore::StateFlow;
use crate::SpaceSyncProgress::{SpaceSyncProgress, SyncProgressTracker};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
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
/// Fast workspace-scan polling while scans keep finding changes. App writes and
/// incoming batches scan inline, so this only bounds discovery latency for
/// writes made by terminals and external editors while activity continues.
const WORKSPACE_SCAN_ACTIVE_INTERVAL_MS: u64 = 2_000;
/// Idle ceiling for the adaptive workspace-scan backoff. Steady-state scans are
/// metadata-only (unchanged sizes reuse inventory references), so one listing
/// comparison per minute is the worst idle cost.
const WORKSPACE_SCAN_IDLE_INTERVAL_MS: u64 = 60_000;

/// Per-peer scheduling state behind the change-gated synchronization floor.
///
/// Every cycle probes each storage-granted member with the cheap `syncClockDigest`
/// RPC and runs the expensive exchanges (Space projection, control log,
/// business replication, blobs) only when something actually changed for that
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
    /// Local synchronized-state summary captured right after the last
    /// completed pass (journal clock, projected operations and device preferences).
    lastLocalStateSummary: Option<String>,
    /// Probe clock from the most recent decision, promoted on pass completion.
    probedClockSummary: Option<String>,
    /// Consecutive failed full passes; drives the exponential retry delay.
    consecutiveFailures: u32,
    /// Earliest wall-clock time the next full pass may run after a failure.
    nextFullSyncAllowedAt: i64,
    /// Business failures keep receiving new data without arming connection failure backoff.
    pendingWork: bool,
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
    workspaceScanStop: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    peerGates: Mutex<BTreeMap<String, PeerSyncGate>>,
    progress: SyncProgressTracker,
}

/// 通过 Router 与可达的 Space 成员同步持久化数据，不拥有配对、发现或传输。
#[derive(Clone)]
pub struct SpacePersistenceSyncService {
    state: Arc<SpacePersistenceSyncState>,
}

#[derive(Deserialize)]
struct SyncOperationOrder {
    opId: String,
    originDeviceId: String,
    sequence: i64,
    createdAt: i64,
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
            if let Err(error) = service.scheduleSynchronization() {
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

        if let Err(error) = self.scheduleSynchronization() {
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
        defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
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
                    // A productive round pulls the scanner back to fast polling;
                    // quiet rounds double the interval up to the idle ceiling.
                    intervalMs = if changes > 0 {
                        WORKSPACE_SCAN_ACTIVE_INTERVAL_MS
                    } else {
                        (intervalMs * 2).min(WORKSPACE_SCAN_IDLE_INTERVAL_MS)
                    };
                    // Also retry interrupted transfers even when no new edits occur.
                    let _ = service.scheduleSynchronization();
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
                    if let Err(error) = (SpacePersistenceSyncService { state }).scheduleSynchronization() {
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
        if let Err(error) = crate::fault_isolation::run("scan workspace files", || self.workspaceFiles().scan().map(|_| ())) {
            errors.push(error);
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
        for peerNodeId in services.peers().outboundPeerNodeIds().map_err(|error| error.to_string())? {
            if !activePeers.contains(&peerNodeId) { continue; }
            match control.nodeIsDisconnected(&peerNodeId) {
                Ok(false) => {},
                Ok(true) => continue,
                Err(error) => { errors.push(format!("Peer {peerNodeId} connection policy: {error}")); continue; }
            }
            if !self
                .peerPassRequired(&peerNodeId, &localStateSummary, &mut cycleDecisions)
                .await
            {
                continue;
            }
            if let Err(error) = crate::fault_isolation::operation(&format!("Peer {peerNodeId} Space projection"),
                self.exchangePairedDeviceSpaceProjection(&peerNodeId)).await {
                self.recordPeerPassFailed(&peerNodeId);
                cycleDecisions.insert(peerNodeId.clone(), false);
                errors.push(format!("CoreNode {peerNodeId} Space projection exchange: {error}"));
                continue;
            }
            // A paired peer outside the Space membership has no business
            // replication pass; the projection exchange was its whole pass.
            if !membershipBeforeExchange.contains(&peerNodeId) {
                self.recordPeerPassCompleted(&peerNodeId, false).await?;
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
                continue;
            }
            if !self
                .peerPassRequired(&targetNodeId, &localStateSummary, &mut cycleDecisions)
                .await
            {
                continue;
            }
            if let Err(error) = self
                .synchronizeReachablePeer(targetNodeId.clone(), 512)
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

    /// All entry points share serialization, progress, completion and retry state.
    pub(crate) async fn synchronizeReachablePeer(
        &self,
        targetNodeId: String,
        limit: usize,
    ) -> Result<(), String> {
        if limit == 0 { return Err("sync limit must be greater than 0".into()); }
        if targetNodeId == self.state.nodeRouter.localNodeId() { return Ok(()); }
        let execution = self.state.peerGates.lock().map_err(|error| error.to_string())?
            .entry(targetNodeId.clone()).or_default().execution.clone();
        let _pass = execution.lock().await;
        let space = self.state.spaceStore.space()?;
        self.state.progress.retain(&space.spaceId, &space.members);
        self.state.progress.begin(&targetNodeId, &space.spaceId);
        let context = format!("Peer {targetNodeId} persistence synchronization");
        let pass = crate::fault_isolation::operation(&context, self.synchronizePeer(&targetNodeId, limit));
        tokio::pin!(pass);
        let result = if let Some(delayMs) = self.state.progress.revealDelayMs(&targetNodeId) {
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
                if *allowed {
                    let pending = self.state.peerGates.lock().map_err(|error| error.to_string())?
                        .get(&targetNodeId).is_some_and(|gate| gate.pendingWork);
                    self.state.progress.finish(&targetNodeId, pending, None, None);
                } else {
                    self.state.progress.remove(&targetNodeId);
                }
            }
            Err(error) => {
                self.recordPeerPassFailed(&targetNodeId);
                let retryAt = self.state.peerGates.lock().map_err(|error| error.to_string())?
                    .get(&targetNodeId).map(|gate| gate.nextFullSyncAllowedAt);
                self.state.progress.finish(&targetNodeId, false, retryAt, Some(error.clone()));
            }
        }
        // A pass can revoke membership or storage authority while it is running.
        let space = self.state.spaceStore.space()?;
        self.state.progress.retain(&space.spaceId, &space.members);
        result.map(|_| ())
    }

    /// Executes authority, preferences and authorized business replication in order.
    async fn synchronizePeer(&self, targetNodeId: &str, limit: usize) -> Result<bool, String> {
        if !self.state.nodeRouter.nodeIsReachable(targetNodeId)? {
            return Err(format!(
                "synchronization target is not reachable in the current device space: {targetNodeId}"
            ));
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
            let remote = crate::NodeSpaceService::decodeControlExchange(response.result)?;
            control.projectPeerControlOperations(targetNodeId, &remote)?;
            self.applyRemovedMemberEffects(&remote)?;
        }
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
            let preferences: operit_store::DevicePreferencesStore::DevicePreferences = decodeCoreResponse(response.result.map_err(|e| e.to_string())?)?;
            if preferences.spaceId != preferenceScope || preferences.nodeId != preferenceNodeId
                || self.state.spaceStore.space()?.spaceId != preferenceScope
                || !control.currentState()?.memberNodeIds.iter().any(|node| node == targetNodeId)
                || control.nodeIsDisconnected(targetNodeId)? {
                return Err("Device preference response scope changed".into());
            }
            preferenceStore.merge(&preferences)?;
        }
        // Admission is not a storage grant. Business replication requires both endpoints to provide storage.
        if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
        self.validateReachableDeviceSpace(targetNodeId).await?;
        self.synchronizeNodeOperations(targetNodeId, limit).await
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

    /// Summarizes this device's synchronized state for the local-change
    /// trigger: the replication journal clock (journal plus chat storage,
    /// mirroring the application clock), projected operations and preferences.
    /// Projected operations advance neither clock, so their count is what
    /// makes this device push relayed authority to its peers. Device
    /// preferences have their own revision. The process mutation revision is
    /// deliberately excluded: opening and closing a storage handle publishes
    /// it without writing any data, which would schedule a pass per plugin
    /// read. Read through local stores directly: gating must never enter the
    /// application backend, which non-storage peers are not permitted to reach.
    async fn localStateSummary(&self) -> Result<String, String> {
        let storageHost = self.state.localRuntime.runtimeStorageHost();
        let syncStore = operit_store::SyncOperationStore::SyncOperationStore::new(
            storageHost.clone(),
            RUNTIME_SYNC_DIR_PATH.to_string(),
        );
        let localDeviceId = syncStore.localDeviceId().map_err(|error| error.to_string())?;
        let mut clock = syncStore
            .localClock()
            .map_err(|error| error.to_string())?;
        // The chat storage belongs to the process default store root. A root
        // that was never registered means this process holds no chat state
        // at all, so an empty chat clock is the correct contribution; a real
        // storage failure is logged but must not disable the gate.
        if operit_util::RuntimeStoreRoot::default_runtime_store_root_config().is_some() {
            match operit_store::sync::SqlChatSyncStore::SqlChatSyncStore::default() {
                Ok(sqlStore) => match sqlStore.localClock() {
                    Ok(sqlClock) => {
                        for (deviceId, sequence) in sqlClock.sequences {
                            if sequence > clock.sequenceFor(&deviceId) {
                                clock.setSequence(deviceId, sequence);
                            }
                        }
                    }
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::w(
                            "SpacePersistenceSyncService",
                            &format!("chat sync clock unavailable for gate summary: {error}"),
                        );
                    }
                },
                Err(error) => {
                    operit_util::AppLogger::AppLogger::w(
                        "SpacePersistenceSyncService",
                        &format!("chat sync storage unavailable for gate summary: {error}"),
                    );
                }
            }
        }
        // Operations applied from peers advance their origins in this clock.
        // Counting that as local progress would open a full pass for every
        // member whenever any other device writes, even though this device may
        // have nothing to send. Members fetch another member's operations over
        // routed links, so a partition catches up when the route returns.
        clock.retainAuthoredBy(&localDeviceId);
        let control = NetworkControlStore::new(storageHost)?;
        Ok(format!(
            "{}#{}#{}",
            serde_json::to_string(&clock).map_err(|error| error.to_string())?,
            control.projectionEntryCount()?,
            operit_store::DevicePreferencesStore::devicePreferencesMutationRevision()
        ))
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
        let decision = if !gate.backoffElapsed(currentTimeMillis()) {
            false
        } else if gate.pendingWork {
            true
        } else if gate.consecutiveFailures > 0 {
            // Retry the failed exchange after the backoff has elapsed.
            true
        } else if !self
            .storageReplicationAllowed(targetNodeId)
            .unwrap_or(false)
        {
            // Non-storage members never enter replication traffic: probing
            // them would only draw a denial, so keep exchanging directly.
            true
        } else {
            match self.probeRemoteClockSummary(targetNodeId).await {
                Ok(summary) => {
                    if let Ok(mut gates) = self.state.peerGates.lock() {
                        gates
                            .entry(targetNodeId.to_string())
                            .or_default()
                            .probedClockSummary = Some(summary.clone());
                    }
                    gate.lastRemoteClockSummary.as_deref() != Some(summary.as_str())
                        || gate.lastLocalStateSummary.as_deref() != Some(localStateSummary)
                }
                // The clock is unobservable (the probe was lost). Forcing the
                // full exchange from here made every cycle re-export the whole
                // operation journal against an unreachable peer — seconds of
                // serialization per attempt, indefinitely. Record the failed
                // pass instead so the exponential backoff governs the retry;
                // once the peer answers again, the retry converges and the
                // gate returns to probing.
                Err(_) => {
                    self.recordPeerPassFailed(targetNodeId);
                    false
                }
            }
        };
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

    /// Records one completed full pass: the freshly probed clock and the
    /// current local-state summary become the pass baseline.
    async fn recordPeerPassCompleted(&self, targetNodeId: &str, businessReplication: bool) -> Result<(), String> {
        let localStateSummary = if businessReplication { Some(self.localStateSummary().await?) } else { None };
        let mut gates = self.state.peerGates.lock().map_err(|error| error.to_string())?;
        let gate = gates.entry(targetNodeId.to_string()).or_default();
        if let Some(probed) = gate.probedClockSummary.take() {
            gate.lastRemoteClockSummary = Some(probed);
        }
        gate.lastLocalStateSummary = localStateSummary;
        gate.consecutiveFailures = 0;
        gate.nextFullSyncAllowedAt = 0;
        Ok(())
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

    /// Exchanges persistent operations with one already reachable CoreNode.
    #[allow(non_snake_case)]
    async fn synchronizeNodeOperations(
        &self,
        targetNodeId: &str,
        limit: usize,
    ) -> Result<bool, String> {
        let localVersion: String = self.callLocal("coreVersion", Value::Null).await?;
        let remoteVersion: String = callRemote(
            &self.state.nodeRouter,
            targetNodeId,
            PeerSyncMethod::CoreVersion,
            Value::Null,
        )
        .await?;
        if localVersion != remoteVersion {
            return Err(format!(
                "core version mismatch: local={localVersion}, remote={remoteVersion}. sync blocked"
            ));
        }

        self.synchronizeDomains(targetNodeId, limit, &SYNC_DOMAINS).await
    }

    /// Keeps partially materialized work eligible without delaying healthy data behind it.
    fn recordPendingWork(&self, targetNodeId: &str, stage: &str, report: &Value) {
        let pending = report.get("pending").and_then(Value::as_u64).unwrap_or(0);
        let errors = report.get("errors").and_then(Value::as_array);
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
        // Pages are bounded by bytes as well as by operation count, so a page
        // can end before the backlog does. The pass therefore keeps paging until
        // one exchange finds nothing left; the cap only stops a device that is
        // writing continuously from holding this pass open forever.
        let mut pages = 0usize;
        let mut seenOperationIds = BTreeSet::new();
        loop {
            if !self.storageReplicationAllowed(targetNodeId)? {
                return Ok(false);
            }
            let (localClock, remoteClock) = tokio::join!(
                self.callLocal::<Value>("syncClock", Value::Null),
                callRemote::<Value>(
                    &self.state.nodeRouter,
                    targetNodeId,
                    PeerSyncMethod::SyncClock,
                    Value::Null,
                ),
            );
            let localClock = localClock?;
            validateDomainClock(&localClock, domains, "local")?;
            let remoteClock = remoteClock?;
            validateDomainClock(&remoteClock, domains, "remote")?;
            if let Ok(mut gates) = self.state.peerGates.lock() {
                gates.entry(targetNodeId.to_owned()).or_default().pendingWork = false;
            }
            self.recordPendingWork(targetNodeId, "local receipt", &localClock);
            self.recordPendingWork(targetNodeId, "remote receipt", &remoteClock);
            let mut pendingBlobs: Vec<RuntimeFileSyncReference> =
                serde_json::from_value(localClock["pendingBlobs"].clone())
                    .map_err(|error| format!("invalid local pending blobs: {error}"))?;
            pendingBlobs.extend(
                serde_json::from_value::<Vec<RuntimeFileSyncReference>>(
                    remoteClock["pendingBlobs"].clone(),
                )
                .map_err(|error| format!("invalid remote pending blobs: {error}"))?,
            );
            if let Err(error) = self
                .synchronizeBlobReferences(targetNodeId, pendingBlobs)
                .await
            {
                self.recordPendingWork(targetNodeId, "pending blobs", &json!({"errors": [error]}));
            }
            operit_util::AppLogger::AppLogger::trace(
                "CoreSyncTrace",
                &format!(
                    "sync_exchange.clocks target={} local={} remote={}",
                    targetNodeId,
                    summarizeSyncClock(&localClock),
                    summarizeSyncClock(&remoteClock)
                ),
            );
            let (localOperations, remoteOperations) = tokio::join!(
                self.callLocal::<Value>(
                    "syncOperationsSince",
                    json!({
                        "clock": remoteClock,
                        "domains": domains,
                        "limit": limit,
                    }),
                ),
                callRemote::<Value>(
                    &self.state.nodeRouter,
                    targetNodeId,
                    PeerSyncMethod::SyncOperationsSince,
                    json!({
                        "clock": localClock,
                        "domains": domains,
                        "limit": limit,
                    }),
                ),
            );
            let localOperations = localOperations?;
            let remoteOperations = remoteOperations?;
            if !self.storageReplicationAllowed(targetNodeId)? {
                return Ok(false);
            }
            self.recordPendingWork(targetNodeId, "local export", &localOperations);
            self.recordPendingWork(targetNodeId, "remote export", &remoteOperations);
            let operations = mergeSyncOperations(
                localOperations["operations"].clone(),
                remoteOperations["operations"].clone(),
            )?;
            if operations.is_empty() {
                if !self
                    .applySynchronizedOperationBatch(targetNodeId, &operations, &[])
                    .await?
                {
                    return Ok(false);
                }
                operit_util::AppLogger::AppLogger::trace(
                    "CoreSyncTrace",
                    &format!(
                        "sync_exchange.idle target={} reason=no_exported_operations pages={}",
                        targetNodeId, pages
                    ),
                );
                break;
            }
            // A domain whose acknowledgement never advances the clock returns
            // the same page forever. Stop after the first page that adds no new
            // operation instead of burning the whole pass cap.
            if !pageAddsNewOperations(&mut seenOperationIds, &operations) {
                self.recordPendingWork(targetNodeId, "remaining pages", &json!({"pending": 1}));
                operit_util::AppLogger::AppLogger::trace(
                    "CoreSyncTrace",
                    &format!(
                        "sync_exchange.no_progress target={} pages={} count={}",
                        targetNodeId,
                        pages,
                        operations.len()
                    ),
                );
                break;
            }
            if pages >= MAX_SYNC_PAGES_PER_PASS {
                self.recordPendingWork(targetNodeId, "remaining pages", &json!({"pending": 1}));
                operit_util::AppLogger::AppLogger::trace(
                    "CoreSyncTrace",
                    &format!(
                        "sync_exchange.pages_capped target={} pages={} remaining=unknown",
                        targetNodeId, pages
                    ),
                );
                break;
            }
            pages += 1;
            // Measure once: the page is chunked by these sizes and the trace
            // records the same number, so an oversized import stays diagnosable
            // without replaying the encoding after the fact.
            let sizes = syncOperationWireSizes(&operations)?;
            operit_util::AppLogger::AppLogger::trace(
                "CoreSyncTrace",
                &format!(
                    "sync_exchange.batch target={} bytes={} {}",
                    targetNodeId,
                    sizes.iter().sum::<usize>(),
                    summarizeSyncOperations(&operations)
                ),
            );
            if !self
                .applySynchronizedOperationBatch(targetNodeId, &operations, &sizes)
                .await?
            {
                return Ok(false);
            }
            operit_util::AppLogger::AppLogger::trace(
                "CoreSyncTrace",
                &format!(
                    "sync_exchange.batch_received target={} {}",
                    targetNodeId,
                    summarizeSyncOperations(&operations)
                ),
            );
        }
        Ok(true)
    }

    /// Applies one transport page on both peers, split into messages that fit
    /// the connection ceiling, preserving every original operation boundary.
    #[allow(non_snake_case)]
    async fn applySynchronizedOperationBatch(
        &self,
        targetNodeId: &str,
        operations: &[Value],
        sizes: &[usize],
    ) -> Result<bool, String> {
        if let Err(error) = self
            .synchronizeRequiredBlobs(targetNodeId, operations)
            .await
        {
            self.recordPendingWork(targetNodeId, "blobs", &json!({"errors": [error]}));
        }
        if !self.storageReplicationAllowed(targetNodeId)? {
            return Ok(false);
        }
        self.state.progress.records(targetNodeId, 0);
        // Empty pages retry durable pending operations on both peers.
        let chunks = if operations.is_empty() {
            vec![operations]
        } else {
            syncPageChunks(
                operations,
                sizes,
                self.state.nodeRouter.maxPeerMessageBytes(),
            )?
        };
        for chunk in chunks.iter().copied() {
            let (remote, local) = tokio::join!(
                callRemote::<Value>(
                    &self.state.nodeRouter,
                    targetNodeId,
                    PeerSyncMethod::SyncApplyOperations,
                    json!({ "operations": chunk }),
                ),
                self.callLocal::<Value>(
                    "syncApplyOperations",
                    json!({ "operations": chunk }),
                ),
            );
            if !self.storageReplicationAllowed(targetNodeId)? {
                return Ok(false);
            }
            for (side, result) in [("remote apply", &remote), ("local apply", &local)] {
                if let Ok(report) = result {
                    self.recordPendingWork(targetNodeId, side, report);
                }
            }
            let errors = [("remote", remote), ("local", local)]
                .into_iter()
                .filter_map(|(side, result)| result.err().map(|error| format!("{side} apply: {error}")))
                .collect::<Vec<_>>();
            if !errors.is_empty() {
                return Err(errors.join("; "));
            }
            self.state.progress.records(targetNodeId, chunk.len());
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
        let mut referenceRevision = None;
        let remote = if protocol.as_ref().is_some_and(|protocol| protocol.snapshotPaging) {
            use crate::NodeSpaceService::space_projection::{SnapshotHeader, SnapshotCollector};
            let header: SnapshotHeader = callRemoteService(&self.state.nodeRouter, peerNodeId,
                crate::NodeSpaceService::NODE_SPACE_TARGET, "snapshotHeader", Value::Null).await?;
            referenceRevision = Some(header.controlRevision.clone());
            let mut collector = SnapshotCollector::new(header)?;
            loop {
                let page = callRemoteService(&self.state.nodeRouter, peerNodeId,
                    crate::NodeSpaceService::NODE_SPACE_TARGET, "snapshotControlPage",
                    serde_json::to_value(collector.request()).map_err(|error| error.to_string())?).await?;
                if collector.push(page)? { break; }
            }
            let current: SnapshotHeader = callRemoteService(&self.state.nodeRouter, peerNodeId,
                crate::NodeSpaceService::NODE_SPACE_TARGET, "snapshotHeader", Value::Null).await?;
            collector.finish(current)?
        } else {
            callRemoteService(&self.state.nodeRouter, peerNodeId,
                crate::NodeSpaceService::NODE_SPACE_TARGET, "snapshot", Value::Null).await?
        };
        let remoteSpaceId = remote.space.spaceId.clone();
        let remoteHasDepartures = !remote.departures.is_empty();
        let snapshotControlOperations = remote.controlOperations.clone();
        let service = crate::RuntimeRemoteLinkService::RuntimeRemoteLinkService::newWithRouter(
            (*self.state.localRuntime).clone(), self.state.nodeRouter.clone());
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
        let remoteSpace: CoreSpace = decodeCoreResponse(response.result.map_err(|error| error.to_string())?)?;
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
        operations: &[Value],
    ) -> Result<(), String> {
        let mut references = Vec::new();
        let mut errors = Vec::new();
        for value in operations {
            let result = serde_json::from_value::<SyncOperation>(value.clone())
                .map_err(|error| error.to_string())
                .and_then(|operation| RuntimeFileSyncStore::requiredBlob(&operation));
            match result {
                Ok(Some(reference)) => references.push(reference),
                Ok(None) => {}
                Err(error) => errors.push(error),
            }
        }
        if let Err(error) = self
            .synchronizeBlobReferences(targetNodeId, references)
            .await
        {
            errors.push(error);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// Transfers independent content hashes without withholding durable receipt of another operation.
    async fn synchronizeBlobReferences(
        &self,
        targetNodeId: &str,
        references: Vec<RuntimeFileSyncReference>,
    ) -> Result<(), String> {
        let references = references
            .into_iter()
            .map(|reference| ((reference.contentHash.clone(), reference.size), reference))
            .collect::<BTreeMap<_, _>>();
        let mut errors = Vec::new();
        for reference in references.into_values() {
            let result = async {
                self.state
                    .nodeRouter
                    .requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                    .map_err(|error| error.to_string())?;
                let local = self.localHasBlob(&reference).await?;
                let remote = remoteHasBlob(&self.state.nodeRouter, targetNodeId, &reference).await?;
                match (local, remote) {
                    (true, true) => Ok(()),
                    (true, false) => self.transferBlob(targetNodeId, &reference, true).await,
                    (false, true) => self.transferBlob(targetNodeId, &reference, false).await,
                    (false, false) => Err(format!(
                        "blob {} is missing from both direct peers",
                        reference.contentHash
                    )),
                }
            }
            .await;
            if let Err(error) = result {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    /// Reports whether the local CoreNode owns one verified synchronization blob.
    async fn localHasBlob(&self, reference: &RuntimeFileSyncReference) -> Result<bool, String> {
        self.callLocal(
            "syncBlobExists",
            json!({
                "contentHash": reference.contentHash,
                "size": reference.size,
            }),
        )
        .await
    }

    /// Both directions share authorization, chunk validation, progress and final verification.
    async fn transferBlob(
        &self,
        targetNodeId: &str,
        reference: &RuntimeFileSyncReference,
        localSource: bool,
    ) -> Result<(), String> {
        self.state.progress.beginFile(targetNodeId, reference.size);
        let mut request = blobPushRequest(&self.state.nodeRouter, reference)?;
        let mut push = if localSource {
            request.target = NODE_SYNC_TARGET.into();
            self.state.nodeRouter.openPushNode(targetNodeId.to_string(), request).await
                .map_err(|error| error.to_string())?
        } else {
            self.state.localRuntime.openPush(request).map_err(|error| error.to_string())?
        };
        let mut offset = 0;
        let result = async {
            while offset < reference.size {
                self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                    .map_err(|error| error.to_string())?;
                let args = json!({ "contentHash": reference.contentHash, "offset": offset, "length": SYNC_BLOB_CHUNK_BYTES });
                let chunk = if localSource {
                    self.callLocal::<Vec<u8>>("syncReadBlobChunk", args).await?
                } else {
                    callRemote::<Vec<u8>>(&self.state.nodeRouter, targetNodeId, PeerSyncMethod::SyncReadBlobChunk, args).await?
                };
                self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                    .map_err(|error| error.to_string())?;
                offset = sendBlobChunk(&mut push, reference, offset, chunk).await?;
                self.state.progress.fileOffset(targetNodeId, offset, false);
            }
            self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                .map_err(|error| error.to_string())?;
            push.close().await.map_err(|error| error.to_string())?;
            let persisted = if localSource {
                remoteHasBlob(&self.state.nodeRouter, targetNodeId, reference).await?
            } else { self.localHasBlob(reference).await? };
            if !persisted {
                return Err(format!("CoreNode did not persist synchronization blob {}", reference.contentHash));
            }
            Ok(())
        }.await;
        // Preserve the exact sent offset even if a later chunk or verification fails.
        self.state.progress.fileOffset(targetNodeId, offset, true);
        result
    }

    /// Coalesces join, storage and connectivity changes into the existing worker.
    pub(crate) fn scheduleSynchronization(&self) -> Result<(), String> {
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
                    let synchronizedRevision = syncMutationRevision();
                    let syncStartedAt = currentTimeMillis();
                    let localNodeId = service.state.nodeRouter.localNodeId();
                    operit_util::AppLogger::AppLogger::v_with_level(
                        "SpacePersistenceSyncService",
                        &format!(
                            "sync_cycle.start local={} revision={}",
                            localNodeId, synchronizedRevision
                        ),
                        operit_util::AppLogger::VERBOSE_LEVEL_5,
                    );
                    match crate::fault_isolation::operation("Space synchronization pass", service.synchronizeOnce()).await {
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
                        if let Err(error) = service.scheduleSynchronization() {
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
    async fn callLocal<T>(&self, methodName: &str, args: Value) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let request = applicationCallRequest(&self.state.nodeRouter, methodName, args)?;
        let startedAt = currentTimeMillis();
        let response = self.state.localRuntime.callApplication(request).await;
        recordSlowSyncCall("local", methodName, startedAt);
        decodeCoreResponse(response.result.map_err(|error| {
            format!("local {methodName}: {error}")
        })?)
    }
}

/// Reports whether the target CoreNode owns one verified synchronization blob.
async fn remoteHasBlob(
    nodeRouter: &CoreNodeRouter,
    targetNodeId: &str,
    reference: &RuntimeFileSyncReference,
) -> Result<bool, String> {
    callRemote(
        nodeRouter,
        targetNodeId,
        PeerSyncMethod::SyncBlobExists,
        json!({
            "contentHash": reference.contentHash,
            "size": reference.size,
        }),
    )
    .await
}

/// Builds one service reverse-stream request for a complete synchronization blob.
#[allow(non_snake_case)]
fn blobPushRequest(
    nodeRouter: &CoreNodeRouter,
    reference: &RuntimeFileSyncReference,
) -> Result<CorePushRequest, String> {
    let target = nodeRouter
        .targetForSchema("services.syncBlobTransferManager")
        .ok_or_else(|| "unknown Core schema key: services.syncBlobTransferManager".to_string())?;
    Ok(CorePushRequest::new(
        operit_link::nextCoreRouteRequestId("space-persistence-blob"),
        target,
        "syncReceiveBlob",
    )
    .withArgs(
        toCoreValue(json!({
            "contentHash": reference.contentHash,
            "size": reference.size,
        }))
        .map_err(|error| error.to_string())?,
    ))
}

/// Sends one non-empty blob chunk and returns the next absolute offset.
#[allow(non_snake_case)]
async fn sendBlobChunk(
    push: &mut Box<dyn operit_link::CoreLinkPushSession>,
    reference: &RuntimeFileSyncReference,
    offset: i64,
    chunk: Vec<u8>,
) -> Result<i64, String> {
    if chunk.is_empty() {
        return Err(format!(
            "synchronization blob {} ended before its declared size",
            reference.contentHash
        ));
    }
    let chunkLength = i64::try_from(chunk.len())
        .map_err(|_| "synchronization blob chunk length does not fit i64".to_string())?;
    let nextOffset = offset
        .checked_add(chunkLength)
        .ok_or_else(|| "synchronization blob offset overflow".to_string())?;
    if nextOffset > reference.size {
        return Err(format!(
            "synchronization blob {} exceeded its declared size",
            reference.contentHash
        ));
    }
    push.send(toCoreValue(chunk).map_err(|error| error.to_string())?)
        .await
        .map_err(|error| error.to_string())?;
    Ok(nextOffset)
}

/// Weak registration shares one worker across facades without extending its lifetime.
fn persistenceServices() -> &'static Mutex<BTreeMap<String, Weak<SpacePersistenceSyncState>>> {
    SPACE_SYNC_SERVICES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Rejects only the requested domains whose durable clocks could not be read.
#[allow(non_snake_case)]
fn validateDomainClock(clock: &Value, _domains: &[&str], side: &str) -> Result<(), String> {
    if clock.get("protocolVersion").and_then(Value::as_u64) != Some(1) {
        return Err(format!("{side} synchronization receipt protocol mismatch"));
    }
    let receipt: operit_store::SyncOperationStore::SyncReceipt =
        serde_json::from_value(clock.clone())
            .map_err(|error| format!("{side} synchronization receipt: {error}"))?;
    receipt
        .validate()
        .map_err(|error| format!("{side} synchronization receipt: {error}"))
}

/// Invokes one application method through the Router.
async fn callRemote<T>(
    nodeRouter: &CoreNodeRouter,
    targetNodeId: &str,
    method: PeerSyncMethod,
    args: Value,
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
    decodeCoreResponse(response.result.map_err(|error| {
        format!("remote {targetNodeId} {methodName}: {error}")
    })?)
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
    let mut protocol = crate::NodeSpaceService::space_projection::decodeProtocolResponse(peer, response.result)?;
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
    decodeCoreResponse(response.result.map_err(|error| {
        format!("remote {targetNodeId} {targetPath}.{methodName}: {error}")
    })?)
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

/// Builds one Link request for an application-level runtime operation.
fn applicationCallRequest(
    nodeRouter: &CoreNodeRouter,
    methodName: &str,
    args: Value,
) -> Result<CoreCallRequest, String> {
    Ok(CoreCallRequest::new(
        operit_link::nextCoreRouteRequestId(&format!("space-persistence-{methodName}")),
        nodeRouter
            .targetForSchema("application")
            .ok_or_else(|| "unknown Core schema key: application".to_string())?,
        methodName,
        toCoreValue(args).map_err(|error| error.to_string())?,
    ))
}

/// Builds one Link request for a generated service operation.
#[allow(non_snake_case)]
fn serviceCallRequest(
    nodeRouter: &CoreNodeRouter,
    targetPath: &str,
    methodName: &str,
    args: Value,
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

/// Reads only the ordering fields of one encoded operation. Cloning each
/// operation just to deserialize it would duplicate the whole page.
fn syncOperationOrder(value: &Value) -> Result<SyncOperationOrder, String> {
    let field = |name: &str| {
        value
            .get(name)
            .ok_or_else(|| format!("invalid sync operation: missing {name}"))
    };
    Ok(SyncOperationOrder {
        opId: field("opId")?
            .as_str()
            .ok_or("invalid sync operation: opId is not a string")?
            .to_string(),
        originDeviceId: field("originDeviceId")?
            .as_str()
            .ok_or("invalid sync operation: originDeviceId is not a string")?
            .to_string(),
        sequence: field("sequence")?
            .as_i64()
            .ok_or("invalid sync operation: sequence is not an integer")?,
        createdAt: field("createdAt")?
            .as_i64()
            .ok_or("invalid sync operation: createdAt is not an integer")?,
    })
}

/// Merges two operation pages into their deterministic application order.
fn mergeSyncOperations(left: Value, right: Value) -> Result<Vec<Value>, String> {
    let mut byId = BTreeMap::<String, (SyncOperationOrder, Value)>::new();
    for operation in syncOperations(left)?
        .into_iter()
        .chain(syncOperations(right)?)
    {
        let key = syncOperationOrder(&operation)?;
        if let Some((_, existing)) = byId.get(&key.opId) {
            if existing == &operation { continue; }
            let existing = serde_json::from_value::<SyncOperation>(existing.clone())
                .map_err(|error| format!("invalid sync operation {}: {error}", key.opId))?;
            let incoming = serde_json::from_value::<SyncOperation>(operation.clone())
                .map_err(|error| format!("invalid sync operation {}: {error}", key.opId))?;
            if !operit_store::SyncOperationStore::incomingSyncOperationWins(&existing, &incoming)
                .map_err(|error| error.to_string())? { continue; }
        }
        byId.insert(key.opId.clone(), (key, operation));
    }
    let mut operations = byId.into_values().collect::<Vec<_>>();
    operations.sort_by(|left, right| {
        (
            left.0.createdAt,
            &left.0.originDeviceId,
            left.0.sequence,
            &left.0.opId,
        )
            .cmp(&(
                right.0.createdAt,
                &right.0.originDeviceId,
                right.0.sequence,
                &right.0.opId,
            ))
    });
    Ok(operations
        .into_iter()
        .map(|(_, operation)| operation)
        .collect())
}

/// Decodes one runtime sync operation page into its ordered operation array.
fn syncOperations(value: Value) -> Result<Vec<Value>, String> {
    serde_json::from_value(value).map_err(|error| format!("invalid sync operations: {error}"))
}

/// Encoded MessagePack size of every operation, measured once per page.
/// The same sizes drive the chunking decision and the batch trace.
#[allow(non_snake_case)]
fn syncOperationWireSizes(operations: &[Value]) -> Result<Vec<usize>, String> {
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
    operations: &'a [Value],
    sizes: &[usize],
    maxMessageBytes: usize,
) -> Result<Vec<&'a [Value]>, String> {
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
            let opId = operations[index].get("opId").and_then(Value::as_str).unwrap_or("unknown");
            let domain = operations[index].get("domain").and_then(Value::as_str).unwrap_or("unknown");
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

/// Summarizes one synchronization clock without logging the complete vector.
#[allow(non_snake_case)]
fn summarizeSyncClock(value: &Value) -> String {
    format!(
        "ranges={}#pending={}#errors={}",
        value["ranges"], value["pending"], value["errors"]
    )
}

/// Records one page's operation ids and reports whether it added a new identity.
///
/// A page without complete ids is treated as progress so malformed data can
/// never terminate a synchronization pass early.
#[allow(non_snake_case)]
fn pageAddsNewOperations(
    seenOperationIds: &mut BTreeSet<String>,
    operations: &[Value],
) -> bool {
    let pageOperationIds = operations
        .iter()
        .filter_map(|operation| operation.get("opId").and_then(Value::as_str))
        .map(str::to_string)
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

/// Summarizes synchronization operations by domain and addressed chat entities.
#[allow(non_snake_case)]
fn summarizeSyncOperations(operations: &[Value]) -> String {
    let mut domainCounts = BTreeMap::<String, usize>::new();
    let mut chatEntities = BTreeSet::new();
    let mut createdAt = Vec::new();
    let mut operationIds = Vec::new();
    for value in operations {
        // Read the fields the summary needs directly: cloning each operation to
        // deserialize it would copy the whole page once per trace line.
        let Some(domain) = value.get("domain").and_then(Value::as_str) else {
            continue;
        };
        *domainCounts.entry(domain.to_string()).or_default() += 1;
        if domain == "chat" {
            let entityType = value.get("entityType").and_then(Value::as_str).unwrap_or_default();
            let entityId = value.get("entityId").and_then(Value::as_str).unwrap_or_default();
            chatEntities.insert(format!("{entityType}/{entityId}"));
        }
        createdAt.push(value.get("createdAt").and_then(Value::as_i64).unwrap_or_default());
        operationIds.push(value.get("opId").and_then(Value::as_str).unwrap_or_default().to_string());
    }
    createdAt.sort();
    operationIds.sort();
    let domains = domainCounts
        .into_iter()
        .map(|(domain, count)| format!("{domain}:{count}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "count={} domains={} chatEntities={} createdAt={}..{} opIds={}",
        operations.len(),
        if domains.is_empty() { "none" } else { &domains },
        if chatEntities.is_empty() {
            "none".to_string()
        } else {
            chatEntities.into_iter().collect::<Vec<_>>().join(",")
        },
        createdAt.first().copied().unwrap_or_default(),
        createdAt.last().copied().unwrap_or_default(),
        if operationIds.is_empty() {
            "none".to_string()
        } else {
            operationIds.join(",")
        }
    )
}

#[cfg(test)]
mod page_budget_tests {
    use super::*;

    /// Keeps core clocks usable while a plugin clock is explicitly unavailable.
    #[test]
    fn plugin_clock_failures_are_isolated_from_core_domains() {
        let clock = json!({
            "protocolVersion": 1, "ranges": {"core": [[1,7]]},
            "errors": ["invalid plugin catalog"],
        });
        assert!(validateDomainClock(&clock, &SYNC_DOMAINS, "local").is_ok());
        let durable: operit_store::SyncOperationStore::SyncReceipt = serde_json::from_value(clock).unwrap();
        assert_eq!(durable.ranges["core"], vec![[1,7]]);
    }

    /// Clears a plugin clock error only when the actual clock can be read again.
    #[test]
    fn recovered_plugin_clock_can_synchronize_again() {
        let clock = json!({"protocolVersion": 1, "ranges": {"plugin": [[1,2]]}, "errors": []});
        assert!(validateDomainClock(&clock, &SYNC_DOMAINS, "local").is_ok());
    }



    /// Constructs one transport-sized plugin operation for page budget tests.
    fn operation(opId: &str, payloadBytes: usize) -> Value {
        json!({
            "opId": opId,
            "domain": "plugin_storage",
            "payload": "x".repeat(payloadBytes),
        })
    }

    /// Extracts operation identities from one planned message chunk.
    fn ids(chunk: &[Value]) -> Vec<String> {
        chunk
            .iter()
            .map(|value| value["opId"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// Resolves same-ID SQL revisions identically regardless of which peer supplied each page.
    #[test]
    fn merge_pages_keeps_newest_chat_revision_in_both_orders() {
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
            assert_eq!(mergeSyncOperations(json!([left]), json!([right])).unwrap(), vec![new.clone()]);
        }
        let mut control = old.clone(); control["domain"] = json!("network_control");
        let mut conflict = control.clone(); conflict["payload"] = json!({"permission":"admin"});
        assert!(mergeSyncOperations(json!([control]), json!([conflict])).is_err());
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
    fn operation_order_is_read_without_cloning_the_payload() {
        let value = json!({
            "opId": "device:7",
            "originDeviceId": "device",
            "sequence": 7,
            "createdAt": 1234,
            "payload": "x".repeat(4096),
        });
        let order = syncOperationOrder(&value).unwrap();
        assert_eq!(order.opId, "device:7");
        assert_eq!(order.originDeviceId, "device");
        assert_eq!(order.sequence, 7);
        assert_eq!(order.createdAt, 1234);
        assert!(syncOperationOrder(&json!({"opId": "device:7"})).is_err());
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
        assert!(pageAddsNewOperations(&mut seen, &[json!({"domain": "chat"})]));
    }
}
