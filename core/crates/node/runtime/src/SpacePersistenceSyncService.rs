use crate::PeerSync::{PeerSyncMethod, NODE_SYNC_TARGET};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CorePushRequest, CoreValue};
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceStore};
use operit_store::NetworkControlStore::NetworkControlStore;
use operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use operit_store::PreferencesDataStore::{CoreNodeStateStore, stringPreferencesKey};
use operit_store::RuntimeFileSyncStore::{RuntimeFileSyncReference, RuntimeFileSyncStore};
use operit_store::SyncOperationStore::{subscribeSyncMutations, syncMutationRevision, SyncMutationSubscription, SyncOperation};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::CoreNodeRouter::{CoreNodeLocalRuntime, CoreNodeRouter};

const SYNC_DOMAINS: [&str; 6] = [
    "preferences",
    "chat",
    "binding",
    "runtime_file",
    "network_control",
    "plugin_storage",
];
const SPACE_SYNC_PREPARATION_DELAY_MS: u64 = 0;
const SYNC_BLOB_CHUNK_BYTES: i64 = 64 * 1024;
/// Delay before the first retry of a failed full synchronization pass with one peer.
const PEER_FULL_SYNC_BACKOFF_BASE_MS: i64 = 2_000;
/// Ceiling for the exponential retry delay of one peer's full synchronization pass.
const PEER_FULL_SYNC_BACKOFF_MAX_MS: i64 = 60_000;

/// Per-peer scheduling state behind the change-gated synchronization floor.
///
/// Every cycle probes each storage-granted member with the cheap `syncClock`
/// RPC and runs the expensive exchanges (Space projection, control log,
/// business replication, blobs) only when something actually changed for that
/// peer: the remote clock moved, the local synchronized-state summary moved
/// since the last completed pass, or a previous pass failed and its backoff
/// elapsed. Space records advance the journal clock, so every snapshot
/// ingredient is visible to the probe. Third-party control operations learned
/// by projection advance neither clock; the local summary counts them, so the
/// projecting device pushes relayed authority through its own next full pass.
#[derive(Clone, Default)]
struct PeerSyncGate {
    /// Summarized remote clock observed at the last completed full pass.
    lastRemoteClockSummary: Option<String>,
    /// Local synchronized-state summary captured right after the last
    /// completed pass (journal clock plus projected-operation count).
    lastLocalStateSummary: Option<String>,
    /// Probe clock from the most recent decision, promoted on pass completion.
    probedClockSummary: Option<String>,
    /// Consecutive failed full passes; drives the exponential retry delay.
    consecutiveFailures: u32,
    /// Earliest wall-clock time the next full pass may run after a failure.
    nextFullSyncAllowedAt: i64,
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

static SPACE_SYNC_SERVICES: OnceLock<Mutex<BTreeMap<String, Arc<SpacePersistenceSyncState>>>> =
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
        Self {
            state: Arc::new(SpacePersistenceSyncState {
                localRuntime,
                nodeRouter,
                spaceStore,
                synchronizationScheduled: AtomicBool::new(false),
                active: AtomicBool::new(false),
                mutationSubscription: Mutex::new(None),
                peerChangesStop: Mutex::new(None),
                synchronizationRequested: AtomicBool::new(false),
                workspaceScanStop: Mutex::new(None),
                peerGates: Mutex::new(BTreeMap::new()),
            }),
        }
    }

    /// Starts the unique change-triggered persistence synchronizer for this CoreNode.
    pub fn start(&self) -> Result<(), String> {
        let startedAt = currentTimeMillis();
        self.state.nodeRouter.nodeServices()?;
        self.state.spaceStore.initialize()?;
        let localNodeId = self.state.nodeRouter.localNodeId();
        operit_util::AppLogger::AppLogger::i(
            "SpacePersistenceSyncService",
            &format!(
                "space store initialize done elapsedMs={}",
                currentTimeMillis() - startedAt
            ),
        );
        {
            let mut services = persistenceServices()
                .lock()
                .map_err(|error| format!("Space sync registry lock poisoned: {error}"))?;
            if services.contains_key(&localNodeId) {
                return Ok(());
            }
            self.state.active.store(true, Ordering::Release);
            services.insert(localNodeId.clone(), self.state.clone());
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

        if let Err(error) = self.startWorkspaceScanner().and_then(|_| self.startPeerChangeWatcher()) {
            let _ = self.stop();
            return Err(error);
        }
        operit_util::AppLogger::AppLogger::i(
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
        operit_util::AppLogger::AppLogger::i(
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
        let localNodeId = self.state.nodeRouter.localNodeId();
        let mut services = persistenceServices()
            .lock()
            .map_err(|error| format!("Space sync registry lock poisoned: {error}"))?;
        if services
            .get(&localNodeId)
            .is_some_and(|state| Arc::ptr_eq(state, &self.state))
        {
            services.remove(&localNodeId);
        }
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
                loop {
                    tokio::select! {
                        _ = &mut stopped => break,
                        result = defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(2000) => {
                            if result.is_err() { break; }
                        }
                    }
                    let Some(state) = weakState.upgrade() else { break; };
                    if !state.active.load(Ordering::Acquire) { break; }
                    let service = SpacePersistenceSyncService { state };
                    if let Err(error) = service.workspaceFiles().scan() {
                        operit_util::AppLogger::AppLogger::w("SpacePersistenceSyncService",
                            &format!("Workspace scan failed; no missing-file deletions published: {error}"));
                    }
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
        self.state.spaceStore.initialize()?;
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
        self.workspaceFiles().scan()?;
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
                    operit_util::AppLogger::VERBOSE_LEVEL_1,
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
        let mut errors = Vec::new();
        let localStateSummary = self.localStateSummary().await?;
        let mut cycleDecisions = BTreeMap::<String, bool>::new();
        let membershipBeforeExchange = self.state.spaceStore.space()?.members.into_iter().collect::<BTreeSet<_>>();
        for peerNodeId in services.peers().outboundPeerNodeIds().map_err(|error| error.to_string())? {
            if !activePeers.contains(&peerNodeId) || control.nodeIsDisconnected(&peerNodeId)? {
                continue;
            }
            if !self
                .peerPassRequired(&peerNodeId, &localStateSummary, &mut cycleDecisions)
                .await
            {
                continue;
            }
            if let Err(error) = self.exchangePairedDeviceSpaceProjection(&peerNodeId).await {
                self.recordPeerPassFailed(&peerNodeId);
                errors.push(format!("CoreNode {peerNodeId} Space projection exchange: {error}"));
                continue;
            }
            // A paired peer outside the Space membership has no business
            // replication pass; the projection exchange was its whole pass.
            if !membershipBeforeExchange.contains(&peerNodeId) {
                self.recordPeerPassCompleted(&peerNodeId).await;
            }
        }
        let space = self.state.spaceStore.space()?;
        for targetNodeId in space.members {
            if targetNodeId == localNodeId {
                continue;
            }
            let reachable = self.state.nodeRouter.nodeIsReachable(&targetNodeId)?;
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
                .synchronizeReachablePeer(targetNodeId.clone(), 512, false)
                .await
            {
                self.recordPeerPassFailed(&targetNodeId);
                errors.push(format!(
                    "CoreNode {} reachable persistence sync: {error}",
                    targetNodeId
                ));
                continue;
            }
            self.recordPeerPassCompleted(&targetNodeId).await;
        }
        if !errors.is_empty() {
            return Err(errors.join(" | "));
        }
        Ok(())
    }

    /// Exchanges persisted operations with one reachable Space member selected by node id.
    #[allow(non_snake_case)]
    pub(crate) async fn synchronizeReachablePeer(
        &self,
        targetNodeId: String,
        limit: usize,
        bootstrap: bool,
    ) -> Result<(), String> {
        if limit == 0 {
            return Err("sync limit must be greater than 0".to_string());
        }
        if targetNodeId == self.state.nodeRouter.localNodeId() {
            return Ok(());
        }
        if !self.state.nodeRouter.nodeIsReachable(&targetNodeId)? {
            return Err(format!(
                "synchronization target is not reachable in the current device space: {targetNodeId}"
            ));
        }
        // Authority updates must still reach non-storage members (including
        // grants/revocations); they do not advance business replication clocks.
        let control = NetworkControlStore::new(self.state.localRuntime.runtimeStorageHost())?;
        let request = serviceCallRequest(&self.state.nodeRouter,
            crate::NodeSpaceService::NODE_SPACE_CONTROL_TARGET, "exchange",
            serde_json::to_value(control.currentSpaceOperations()?).map_err(|error| error.to_string())?)?;
        let response = self.state.nodeRouter.callNode(targetNodeId.clone(), request).await;
        let remote = crate::NodeSpaceService::decodeControlExchange(response.result)?;
        control.projectPeerControlOperations(&targetNodeId, &remote)?;
        self.applyRemovedMemberEffects(&remote)?;
        // Admission is not a storage grant. No business RPC, blob transfer or
        // bootstrap reset may run unless both endpoints currently have it.
        if !self.storageReplicationAllowed(&targetNodeId)? {
            // Silence here cost a real debugging session: routed chats on the
            // target then fail with missing model config and nobody can tell
            // why the replica never arrived.
            operit_util::AppLogger::AppLogger::w(
                "SpacePersistenceSync",
                &format!(
                    "business replica sync skipped for {targetNodeId}: either endpoint lacks the storage.provide capability, advertises no core version, or is disconnected"
                ),
            );
            return Ok(());
        }
        self.validateReachableDeviceSpace(&targetNodeId).await?;
        let spaceId = self.state.spaceStore.space()?.spaceId;
        let completed = self.synchronizeNodeOperations(&targetNodeId, limit,
            bootstrap || self.replicaBootstrapRequired(&spaceId)?).await?;
        if completed && self.storageReplicationAllowed(&targetNodeId)? && self.state.spaceStore.space()?.spaceId == spaceId {
            let marker = self.replicaMarker();
            let mut preferences = marker.data().map_err(|error| error.to_string())?;
            preferences.set(&stringPreferencesKey("spaceId"), spaceId);
            marker.replaceRecoverably(preferences).map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn replicaMarker(&self) -> CoreNodeStateStore {
        CoreNodeStateStore::newWithStorage(self.state.localRuntime.runtimeStorageHost(),
            "runtime/link_access/storage_replica.preferences.json")
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

    pub(crate) fn replicaBootstrapRequired(&self, spaceId: &str) -> Result<bool, String> {
        let preferences = self.replicaMarker().data().map_err(|error| error.to_string())?;
        if preferences.get(&stringPreferencesKey("spaceId")).is_some_and(|ready| ready == spaceId) { return Ok(false); }
        // A Space creator's existing local data belongs to this Space. A joined
        // node without a completed replica must bootstrap when later authorized,
        // even if it could not sync at the moment admission was approved.
        let local = self.state.nodeRouter.localNodeId();
        let policy = NetworkControlStore::new(self.state.localRuntime.runtimeStorageHost())?;
        Ok(!policy.currentSpaceOperations()?.iter().any(|operation| {
            serde_json::from_value::<operit_store::NetworkControlStore::NetworkControlCommandRecord>(operation.payload.clone())
                .is_ok_and(|record| record.spaceId == spaceId && matches!(record.command,
                    operit_store::NetworkControlStore::NetworkControlCommand::Bootstrap { initialAdminNodeId } if initialAdminNodeId == local))
        }))
    }

    pub(crate) fn storageReplicationAllowed(&self, targetNodeId: &str) -> Result<bool, String> {
        match self.state.nodeRouter.requirePeerStorageProviders(
            &self.state.nodeRouter.localNodeId(), targetNodeId,
        ) {
            Ok(()) => Ok(true),
            Err(error) if error.code == "SPACE_STORAGE_PERMISSION_DENIED" => Ok(false),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Reads whether route changes may auto-push this device's model config.
    #[allow(non_snake_case)]
    pub fn autoModelSyncEnabled(&self) -> Result<bool, String> {
        let store = operit_store::PreferencesDataStore::CoreNodeStateStore::newWithStorage(
            self.state.localRuntime.runtimeStorageHost(),
            "runtime/link_access/auto_model_sync.preferences.json",
        );
        let preferences = store.data().map_err(|error| error.to_string())?;
        Ok(preferences
            .get(&operit_store::PreferencesDataStore::stringPreferencesKey("enabled"))
            .is_some_and(|value| value == "true"))
    }

    /// Toggles auto-pushing this device's model config on route changes.
    #[allow(non_snake_case)]
    pub fn setAutoModelSyncEnabled(&self, enabled: bool) -> Result<(), String> {
        let store = operit_store::PreferencesDataStore::CoreNodeStateStore::newWithStorage(
            self.state.localRuntime.runtimeStorageHost(),
            "runtime/link_access/auto_model_sync.preferences.json",
        );
        let mut preferences = store.data().map_err(|error| error.to_string())?;
        preferences.set(
            &operit_store::PreferencesDataStore::stringPreferencesKey("enabled"),
            if enabled { "true".to_string() } else { "false".to_string() },
        );
        store
            .replaceRecoverably(preferences)
            .map_err(|error| error.to_string())
    }

    /// Queries one node's chat model readiness over the control plane.
    #[allow(non_snake_case)]
    async fn targetChatModelStatus(
        &self,
        targetNodeId: &str,
    ) -> Result<serde_json::Value, String> {
        if targetNodeId == self.state.nodeRouter.localNodeId() {
            self.callLocal("chatModelStatus", serde_json::Value::Null).await
        } else {
            callRemote(
                &self.state.nodeRouter,
                targetNodeId,
                PeerSyncMethod::ChatModelStatus,
                serde_json::Value::Null,
            )
            .await
        }
    }

    /// Pushes this device's model provider configuration to one node.
    #[allow(non_snake_case)]
    async fn pushLocalModelConfig(
        &self,
        targetNodeId: &str,
    ) -> Result<serde_json::Value, String> {
        let providersJson: String = self
            .callLocal("exportModelProviders", serde_json::Value::Null)
            .await?;
        callRemote(
            &self.state.nodeRouter,
            targetNodeId,
            PeerSyncMethod::SyncModelConfig,
            serde_json::json!({ "providersJson": providersJson }),
        )
        .await
    }

    /// Verifies that one route-change target can serve a chat turn once this
    /// node's pre-switch sync has run. Storage replication carries the model
    /// configuration itself; when it is blocked, the target must already hold
    /// a usable chat model — or the auto-sync opt-in pushes this device's
    /// configuration over the control plane before the switch commits.
    #[allow(non_snake_case)]
    pub(crate) async fn routeTargetReadyForChat(&self, targetNodeId: &str) -> Result<(), String> {
        if self.storageReplicationAllowed(targetNodeId)? {
            return Ok(());
        }
        let usable = |status: &serde_json::Value| {
            status
                .get("hasUsableChatModel")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        };
        let status = self.targetChatModelStatus(targetNodeId).await?;
        if !usable(&status) && targetNodeId != self.state.nodeRouter.localNodeId() {
            if self.autoModelSyncEnabled()? {
                self.pushLocalModelConfig(targetNodeId).await?;
                let refreshed = self.targetChatModelStatus(targetNodeId).await?;
                if usable(&refreshed) {
                    operit_util::AppLogger::AppLogger::i(
                        "SpacePersistenceSync",
                        &format!("model config auto-synced to {targetNodeId} before route change"),
                    );
                    return Ok(());
                }
            }
        }
        if usable(&status) || usable(&self.targetChatModelStatus(targetNodeId).await?) {
            return Ok(());
        }
        let reason = status
            .get("reason")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string();
        Err(format!(
            "ROUTE_TARGET_NOT_READY: 目标设备 {targetNodeId} 缺少可用的聊天模型配置（{reason}），\
             且业务同步被存储能力阻止；可在本设备开启「远程缺少模型时自动同步」、\
             将其身份设为执行节点(Runner)或在该设备上配置模型后重试"
        ))
    }

    /// Summarizes this device's synchronized state for the local-change
    /// trigger: the replication journal clock (journal plus chat storage,
    /// mirroring the application clock) plus the projected-operation count.
    /// Projected operations advance neither clock, so their count is what
    /// makes this device push relayed authority to its peers. Read through
    /// local stores directly: gating must never enter the application
    /// backend, which non-storage peers are not permitted to reach.
    async fn localStateSummary(&self) -> Result<String, String> {
        let storageHost = self.state.localRuntime.runtimeStorageHost();
        let mut clock = operit_store::SyncOperationStore::SyncOperationStore::new(
            storageHost.clone(),
            RUNTIME_SYNC_DIR_PATH.to_string(),
        )
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
        let control = NetworkControlStore::new(storageHost)?;
        Ok(format!(
            "{}#{}",
            summarizeSyncClock(
                &serde_json::to_value(&clock).map_err(|error| error.to_string())?
            ),
            control.projectionEntryCount()?
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
        } else if gate.consecutiveFailures > 0 {
            // The previous failure may sit on this side (bootstrap reset,
            // missing blob); retry the full pass now that the backoff elapsed.
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
                // The clock is unobservable (the probe was lost): fall back to
                // exchanging, as before.
                Err(_) => true,
            }
        };
        cycleDecisions.insert(targetNodeId.to_string(), decision);
        decision
    }

    /// Reads one peer's synchronization clock through the cheap probe RPC and
    /// returns its comparable summary.
    async fn probeRemoteClockSummary(&self, targetNodeId: &str) -> Result<String, String> {
        let clock: Value = callRemote(
            &self.state.nodeRouter,
            targetNodeId,
            PeerSyncMethod::SyncClock,
            Value::Null,
        )
        .await?;
        Ok(summarizeSyncClock(&clock))
    }

    /// Records one completed full pass: the freshly probed clock and the
    /// current local-state summary become the pass baseline.
    async fn recordPeerPassCompleted(&self, targetNodeId: &str) {
        let localStateSummary = self.localStateSummary().await.unwrap_or_default();
        if let Ok(mut gates) = self.state.peerGates.lock() {
            let gate = gates.entry(targetNodeId.to_string()).or_default();
            if let Some(probed) = gate.probedClockSummary.take() {
                gate.lastRemoteClockSummary = Some(probed);
            }
            gate.lastLocalStateSummary = Some(localStateSummary);
            gate.consecutiveFailures = 0;
            gate.nextFullSyncAllowedAt = 0;
        }
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
        bootstrap: bool,
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

        if bootstrap {
            let _: Value = self
                .callLocal(
                    "syncApplyOperations",
                    json!({
                        "operations": {
                            "operations": [],
                            "forceApply": true,
                        }
                    }),
                )
                .await?;
        }

        let mut bootstrapClock = operit_store::SyncOperationStore::SyncClock::empty();
        loop {
            if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
            let localClock: Value = if bootstrap {
                serde_json::to_value(&bootstrapClock).map_err(|error| error.to_string())?
            } else { self.callLocal("syncClock", Value::Null).await? };
            let remoteClock: Value = callRemote(
                &self.state.nodeRouter,
                targetNodeId,
                PeerSyncMethod::SyncClock,
                Value::Null,
            )
            .await?;
            operit_util::AppLogger::AppLogger::trace(
                "CoreSyncTrace",
                &format!(
                    "sync_exchange.clocks target={} local={} remote={}",
                    targetNodeId,
                    summarizeSyncClock(&localClock),
                    summarizeSyncClock(&remoteClock)
                ),
            );
            let localOperations: Value = self
                .callLocal(
                    "syncOperationsSince",
                    json!({
                        "clock": remoteClock,
                        "domains": SYNC_DOMAINS,
                        "limit": limit,
                    }),
                )
                .await?;
            let remoteOperations: Value = callRemote(
                &self.state.nodeRouter,
                targetNodeId,
                PeerSyncMethod::SyncOperationsSince,
                json!({
                    "clock": localClock,
                    "domains": SYNC_DOMAINS,
                    "limit": limit,
                }),
            )
            .await?;
            if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
            if bootstrap {
                let operations = syncOperations(remoteOperations.clone())?;
                if operations.is_empty() {
                    break;
                }
                self.synchronizeRequiredBlobs(targetNodeId, &operations)
                    .await?;
                if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
                let _: Value = self
                    .callLocal(
                        "syncApplyOperations",
                        json!({
                            "operations": {
                                "operations": remoteOperations,
                                "forceApply": true,
                            }
                        }),
                    )
                    .await?;
                for operation in &operations {
                    let operation: SyncOperationOrder = serde_json::from_value(operation.clone()).map_err(|error| error.to_string())?;
                    if operation.sequence > bootstrapClock.sequenceFor(&operation.originDeviceId) {
                        bootstrapClock.setSequence(operation.originDeviceId, operation.sequence);
                    }
                }
                if operations.len() < limit {
                    break;
                }
                continue;
            }
            let operations = mergeSyncOperations(localOperations, remoteOperations)?;
            if operations.is_empty() {
                operit_util::AppLogger::AppLogger::trace(
                    "CoreSyncTrace",
                    &format!(
                        "sync_exchange.idle target={} reason=clocks_equal",
                        targetNodeId
                    ),
                );
                break;
            }
            operit_util::AppLogger::AppLogger::trace(
                "CoreSyncTrace",
                &format!(
                    "sync_exchange.batch target={} {}",
                    targetNodeId,
                    summarizeSyncOperations(&operations)
                ),
            );
            self.synchronizeRequiredBlobs(targetNodeId, &operations)
                .await?;
            if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
            let _: Value = callRemote(
                &self.state.nodeRouter,
                targetNodeId,
                PeerSyncMethod::SyncApplyOperations,
                json!({ "operations": operations.clone() }),
            )
            .await?;
            if !self.storageReplicationAllowed(targetNodeId)? { return Ok(false); }
            let _: Value = self
                .callLocal(
                    "syncApplyOperations",
                    json!({ "operations": operations.clone() }),
                )
                .await?;
            operit_util::AppLogger::AppLogger::trace(
                "CoreSyncTrace",
                &format!(
                    "sync_exchange.batch_applied target={} {}",
                    targetNodeId,
                    summarizeSyncOperations(&operations)
                ),
            );
            if operations.len() < limit {
                break;
            }
        }
        Ok(true)
    }

    /// Exchanges authenticated Device Space projections and reports whether business sync is allowed.
    #[allow(non_snake_case)]
    pub(crate) async fn exchangePairedDeviceSpaceProjection(
        &self,
        peerNodeId: &str,
    ) -> Result<bool, String> {
        let remote: crate::RuntimeRemoteLinkService::PeerSpaceSnapshot = callRemoteService(
            &self.state.nodeRouter, peerNodeId,
            crate::RuntimeRemoteLinkService::NODE_SPACE_TARGET, "snapshot", Value::Null,
        ).await?;
        let snapshotControlOperations = remote.controlOperations.clone();
        let service = crate::RuntimeRemoteLinkService::RuntimeRemoteLinkService::newWithRouter(
            (*self.state.localRuntime).clone(), self.state.nodeRouter.clone());
        service.observePeerSpaceSnapshot(peerNodeId, remote)?;
        self.applyRemovedMemberEffects(&snapshotControlOperations)?;
        let local = service.peerSpaceSnapshot()?;
        let remoteSpace: CoreSpace = callRemoteService(
            &self.state.nodeRouter, peerNodeId,
            crate::RuntimeRemoteLinkService::NODE_SPACE_TARGET, "observeSpaceSnapshot",
            serde_json::to_value(local).map_err(|error| error.to_string())?,
        ).await?;
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
        let mut references = BTreeMap::<String, RuntimeFileSyncReference>::new();
        for operation in operations {
            let operation: SyncOperation = serde_json::from_value(operation.clone())
                .map_err(|error| format!("invalid sync operation: {error}"))?;
            let Some(reference) = RuntimeFileSyncStore::requiredBlob(&operation)? else {
                continue;
            };
            if let Some(existing) = references.get(&reference.contentHash) {
                if existing.size != reference.size {
                    return Err(format!(
                        "synchronization blob {} has conflicting declared sizes",
                        reference.contentHash
                    ));
                }
            }
            references.insert(reference.contentHash.clone(), reference);
        }
        for reference in references.into_values() {
            self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                .map_err(|error| error.to_string())?;
            let localHasBlob = self.localHasBlob(&reference).await?;
            let remoteHasBlob =
                remoteHasBlob(&self.state.nodeRouter, targetNodeId, &reference).await?;
            match (localHasBlob, remoteHasBlob) {
                (true, true) => {}
                (true, false) => self.pushLocalBlobToRemote(targetNodeId, &reference).await?,
                (false, true) => self.pushRemoteBlobToLocal(targetNodeId, &reference).await?,
                (false, false) => {
                    return Err(format!(
                        "synchronization blob {} is missing from both direct peers",
                        reference.contentHash
                    ));
                }
            }
        }
        Ok(())
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

    /// Pushes one locally available blob into the target CoreNode.
    async fn pushLocalBlobToRemote(
        &self,
        targetNodeId: &str,
        reference: &RuntimeFileSyncReference,
    ) -> Result<(), String> {
        let mut push = self
            .state
            .nodeRouter
            .openPushNode(
                targetNodeId.to_string(),
                {
                    let mut request = blobPushRequest(&self.state.nodeRouter, reference)?;
                    request.target = NODE_SYNC_TARGET.into();
                    request
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        let mut offset = 0i64;
        while offset < reference.size {
            self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                .map_err(|error| error.to_string())?;
            let chunk: Vec<u8> = self
                .callLocal(
                    "syncReadBlobChunk",
                    json!({
                        "contentHash": reference.contentHash,
                        "offset": offset,
                        "length": SYNC_BLOB_CHUNK_BYTES,
                    }),
                )
                .await?;
            let nextOffset = sendBlobChunkValue(reference, offset, chunk)?;
            push.send(nextOffset.1)
                .await
                .map_err(|error| error.to_string())?;
            offset = nextOffset.0;
        }
        self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
            .map_err(|error| error.to_string())?;
        push.close().await.map_err(|error| error.to_string())?;
        if !remoteHasBlob(&self.state.nodeRouter, targetNodeId, reference).await? {
            return Err(format!(
                "remote CoreNode did not persist synchronization blob {}",
                reference.contentHash
            ));
        }
        Ok(())
    }

    /// Pushes one remotely available blob into the local CoreNode.
    async fn pushRemoteBlobToLocal(
        &self,
        targetNodeId: &str,
        reference: &RuntimeFileSyncReference,
    ) -> Result<(), String> {
        let mut push = self
            .state
            .localRuntime
            .openPush(blobPushRequest(&self.state.nodeRouter, reference)?)
            .map_err(|error| error.to_string())?;
        let mut offset = 0i64;
        while offset < reference.size {
            self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                .map_err(|error| error.to_string())?;
            let chunk: Vec<u8> = callRemote(
                &self.state.nodeRouter,
                targetNodeId,
                PeerSyncMethod::SyncReadBlobChunk,
                json!({
                    "contentHash": reference.contentHash,
                    "offset": offset,
                    "length": SYNC_BLOB_CHUNK_BYTES,
                }),
            )
            .await?;
            self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
                .map_err(|error| error.to_string())?;
            offset = sendBlobChunk(&mut push, reference, offset, chunk).await?;
        }
        self.state.nodeRouter.requirePeerStorageProviders(&self.state.nodeRouter.localNodeId(), targetNodeId)
            .map_err(|error| error.to_string())?;
        push.close().await.map_err(|error| error.to_string())?;
        if !self.localHasBlob(reference).await? {
            return Err(format!(
                "local CoreNode did not persist synchronization blob {}",
                reference.contentHash
            ));
        }
        Ok(())
    }

    /// Schedules one fixed coalescing window without restarting an existing timer.
    fn scheduleSynchronization(&self) -> Result<(), String> {
        self.state.synchronizationRequested.store(true, Ordering::Release);
        if self
            .state
            .synchronizationScheduled
            .swap(true, Ordering::AcqRel)
        {
            return Ok(());
        }
        let service = self.clone();
        let scheduleResult = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
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
                    let _ = defaultHostRuntimeTaskSchedulerHost()
                        .waitForHostRuntimeDelay(SPACE_SYNC_PREPARATION_DELAY_MS)
                        .await;
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
                        operit_util::AppLogger::VERBOSE_LEVEL_1,
                    );
                    match service.synchronizeOnce().await {
                        Ok(()) => {
                            operit_util::AppLogger::AppLogger::v_with_level(
                                "SpacePersistenceSyncService",
                                &format!(
                                    "sync_cycle.done local={} revision={} elapsedMs={}",
                                    localNodeId,
                                    synchronizedRevision,
                                    currentTimeMillis() - syncStartedAt
                                ),
                                operit_util::AppLogger::VERBOSE_LEVEL_1,
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
        let response = self
            .state
            .localRuntime
            .callApplication(applicationCallRequest(
                &self.state.nodeRouter,
                methodName,
                args,
            )?)
            .await;
        decodeCoreResponse(response.result.map_err(|error| error.to_string())?)
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
        format!("space-persistence-blob-{}", currentTimeMillis()),
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

/// Validates one synchronization chunk and converts it to one Link value.
fn sendBlobChunkValue(
    reference: &RuntimeFileSyncReference,
    offset: i64,
    chunk: Vec<u8>,
) -> Result<(i64, CoreValue), String> {
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
    Ok((
        nextOffset,
        toCoreValue(chunk).map_err(|error| error.to_string())?,
    ))
}

/// Returns the process registry of active per-CoreNode persistence workers.
fn persistenceServices() -> &'static Mutex<BTreeMap<String, Arc<SpacePersistenceSyncState>>> {
    SPACE_SYNC_SERVICES.get_or_init(|| Mutex::new(BTreeMap::new()))
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
    let response = nodeRouter
        .callNode(
            targetNodeId.to_string(),
            method.request(operit_link::nextCoreRouteRequestId("space-persistence"), toCoreValue(args).map_err(|error| error.to_string())?),
        )
        .await;
    decodeCoreResponse(response.result.map_err(|error| error.to_string())?)
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
    let response = nodeRouter
        .callNode(
            targetNodeId.to_string(),
            serviceCallRequest(nodeRouter, targetPath, methodName, args)?,
        )
        .await;
    decodeCoreResponse(response.result.map_err(|error| error.to_string())?)
}

/// Builds one Link request for an application-level runtime operation.
fn applicationCallRequest(
    nodeRouter: &CoreNodeRouter,
    methodName: &str,
    args: Value,
) -> Result<CoreCallRequest, String> {
    Ok(CoreCallRequest::new(
        format!("space-persistence-{methodName}-{}", currentTimeMillis()),
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
        format!("space-persistence-{methodName}-{}", currentTimeMillis()),
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

/// Merges two operation pages into their deterministic application order.
fn mergeSyncOperations(left: Value, right: Value) -> Result<Vec<Value>, String> {
    let mut byId = BTreeMap::new();
    for operation in syncOperations(left)?
        .into_iter()
        .chain(syncOperations(right)?)
    {
        let key: SyncOperationOrder = serde_json::from_value(operation.clone())
            .map_err(|error| format!("invalid sync operation: {error}"))?;
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

/// Summarizes one synchronization clock without logging the complete vector.
#[allow(non_snake_case)]
fn summarizeSyncClock(value: &Value) -> String {
    value
        .get("sequences")
        .and_then(Value::as_object)
        .map(|sequences| {
            let mut entries = sequences
                .iter()
                .filter_map(|(device, sequence)| {
                    sequence
                        .as_i64()
                        .map(|sequence| format!("{device}:{sequence}"))
                })
                .collect::<Vec<_>>();
            entries.sort();
            entries.join(",")
        })
        .unwrap_or_else(|| "none".to_string())
}

/// Summarizes synchronization operations by domain and addressed chat entities.
#[allow(non_snake_case)]
fn summarizeSyncOperations(operations: &[Value]) -> String {
    let mut domainCounts = BTreeMap::<String, usize>::new();
    let mut chatEntities = BTreeSet::new();
    let mut createdAt = Vec::new();
    let mut operationIds = Vec::new();
    for value in operations {
        let Ok(operation) = serde_json::from_value::<SyncOperation>(value.clone()) else {
            continue;
        };
        *domainCounts.entry(operation.domain.clone()).or_default() += 1;
        if operation.domain == "chat" {
            chatEntities.insert(format!("{}/{}", operation.entityType, operation.entityId));
        }
        createdAt.push(operation.createdAt);
        operationIds.push(operation.opId);
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
