use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_link::{
    fromCoreValue, toCoreValue, CoreCallRequest, CoreLinkClient, CoreLinkSharedClient,
    CorePushRequest, CoreValue,
};
use operit_link_access::{
    coreNodeTransportClient,
    CoreNodePeerLink::{isPeerLinkActive, openOutboundPeerLink},
    LinkAccessStore, PairedRemoteSession, PairedRemoteSessionRecord,
};
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceStore};
use operit_store::RuntimeFileSyncStore::{RuntimeFileSyncReference, RuntimeFileSyncStore};
use operit_store::SyncOperationStore::{
    subscribeSyncMutations, syncMutationRevision, SyncMutationSubscription, SyncOperation,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::{CoreNodeRouter::CoreNodeRouter, LocalCoreProxy};

const SYNC_DOMAINS: [&str; 5] = [
    "preferences",
    "chat",
    "binding",
    "objectbox",
    "runtime_file",
];
const SPACE_SYNC_PREPARATION_DELAY_MS: u64 = 1_000;
const SYNC_BLOB_CHUNK_BYTES: i64 = 64 * 1024;

static SPACE_SYNC_SERVICES: OnceLock<Mutex<BTreeMap<String, Arc<SpacePersistenceSyncState>>>> =
    OnceLock::new();

/// Stores the runtime state owned by one CoreNode persistence worker.
struct SpacePersistenceSyncState {
    localCore: LocalCoreProxy,
    nodeRouter: CoreNodeRouter,
    linkAccessStore: LinkAccessStore,
    spaceStore: CoreSpaceStore,
    synchronizationScheduled: AtomicBool,
    mutationSubscription: Mutex<Option<SyncMutationSubscription>>,
}

/// Exchanges coalesced persistent changes with every directly paired Space member.
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
        localCore: LocalCoreProxy,
        nodeRouter: CoreNodeRouter,
        linkAccessStore: LinkAccessStore,
        spaceStore: CoreSpaceStore,
    ) -> Self {
        Self {
            state: Arc::new(SpacePersistenceSyncState {
                localCore,
                nodeRouter,
                linkAccessStore,
                spaceStore,
                synchronizationScheduled: AtomicBool::new(false),
                mutationSubscription: Mutex::new(None),
            }),
        }
    }

    /// Starts the unique change-triggered persistence synchronizer for this CoreNode.
    pub fn start(&self) -> Result<(), String> {
        self.state.spaceStore.initialize()?;
        let localNodeId = self.state.nodeRouter.localNodeId();
        {
            let mut services = persistenceServices()
                .lock()
                .map_err(|error| format!("Space sync registry lock poisoned: {error}"))?;
            if services.contains_key(&localNodeId) {
                return Ok(());
            }
            services.insert(localNodeId.clone(), self.state.clone());
        }

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

        if let Err(error) = self.scheduleSynchronization() {
            self.state
                .mutationSubscription
                .lock()
                .map_err(|lockError| {
                    format!("Space sync subscription lock poisoned: {lockError}")
                })?
                .take();
            persistenceServices()
                .lock()
                .map_err(|lockError| format!("Space sync registry lock poisoned: {lockError}"))?
                .remove(&localNodeId);
            return Err(error);
        }
        Ok(())
    }

    /// Exchanges every pending persistent operation with all direct outbound peers once.
    pub async fn synchronizeOnce(&self) -> Result<(), String> {
        self.state.spaceStore.initialize()?;
        let sessions = self.state.linkAccessStore.outboundSessions()?;
        self.validateDirectPeerSessions(&sessions)?;
        let localNodeId = self.state.nodeRouter.localNodeId();
        let mut errors = Vec::new();
        for (name, record) in sessions {
            if let Err(error) = self.ensurePeerLink(&localNodeId, &record).await {
                errors.push(format!(
                    "CoreNode {} Peer Link: {error}",
                    record.coreDeviceId
                ));
                continue;
            }
            if let Err(error) = self.synchronizePeer(name, 512, false).await {
                errors.push(format!(
                    "CoreNode {} persistence sync: {error}",
                    record.coreDeviceId
                ));
            }
        }
        if !errors.is_empty() {
            return Err(errors.join(" | "));
        }
        Ok(())
    }

    /// Exchanges persisted operations with one directly paired Space member.
    pub(crate) async fn synchronizePeer(
        &self,
        name: String,
        limit: usize,
        bootstrap: bool,
    ) -> Result<(), String> {
        if limit == 0 {
            return Err("sync limit must be greater than 0".to_string());
        }
        let (record, session) = self.pairedSession(&name)?;
        let info = session.sessionInfo().await?;
        ensureRemoteIdentity(&record, &info.coreDeviceId)?;

        let localVersion: String = self.callLocal("coreVersion", Value::Null).await?;
        let remoteVersion: String = callRemote(&session, "coreVersion", Value::Null).await?;
        if localVersion != remoteVersion {
            return Err(format!(
                "core version mismatch: local={localVersion}, remote={remoteVersion}. sync blocked"
            ));
        }

        if !self
            .exchangeDeviceSpaceProjection(&record, &session)
            .await?
        {
            return Ok(());
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

        loop {
            let localClock: Value = self.callLocal("syncClock", Value::Null).await?;
            let remoteClock: Value = callRemote(&session, "syncClock", Value::Null).await?;
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
                &session,
                "syncOperationsSince",
                json!({
                    "clock": localClock,
                    "domains": SYNC_DOMAINS,
                    "limit": limit,
                }),
            )
            .await?;
            if bootstrap {
                let operations = syncOperations(remoteOperations.clone())?;
                if operations.is_empty() {
                    break;
                }
                self.synchronizeRequiredBlobs(&session, &operations).await?;
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
                if operations.len() < limit {
                    break;
                }
                continue;
            }
            let operations = mergeSyncOperations(localOperations, remoteOperations)?;
            if operations.is_empty() {
                break;
            }
            self.synchronizeRequiredBlobs(&session, &operations).await?;
            let _: Value = callRemote(
                &session,
                "syncApplyOperations",
                json!({ "operations": operations.clone() }),
            )
            .await?;
            let _: Value = self
                .callLocal("syncApplyOperations", json!({ "operations": operations }))
                .await?;
            if operations.len() < limit {
                break;
            }
        }
        Ok(())
    }

    /// Exchanges authenticated Device Space projections and reports whether business sync is allowed.
    #[allow(non_snake_case)]
    async fn exchangeDeviceSpaceProjection(
        &self,
        record: &PairedRemoteSessionRecord,
        session: &PairedRemoteSession,
    ) -> Result<bool, String> {
        let localNodeId = self.state.nodeRouter.localNodeId();
        let localSpace = self.state.spaceStore.initialize()?;
        let remoteSpace: CoreSpace = callRemoteService(
            session,
            "runtimeRemoteLinkService",
            "deviceSpace",
            Value::Null,
        )
        .await?;
        if !remoteSpace
            .members
            .iter()
            .any(|member| member == &record.coreDeviceId)
        {
            return Err("Paired device is not present in its announced device space".to_string());
        }
        self.state
            .spaceStore
            .observePairedDeviceSpace(record.coreDeviceId.clone(), remoteSpace)?;
        let _: CoreSpace = callRemoteService(
            session,
            "runtimeRemoteLinkService",
            "observePairedDeviceSpace",
            json!({
                "deviceId": localNodeId,
                "space": localSpace,
            }),
        )
        .await?;
        let currentLocalSpace = self.state.spaceStore.space()?;
        let currentRemoteSpace: CoreSpace = callRemoteService(
            session,
            "runtimeRemoteLinkService",
            "deviceSpace",
            Value::Null,
        )
        .await?;
        Ok(currentLocalSpace.spaceId == currentRemoteSpace.spaceId)
    }

    /// Transfers every content-addressed blob required by a synchronization page.
    async fn synchronizeRequiredBlobs(
        &self,
        session: &PairedRemoteSession,
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
            let localHasBlob = self.localHasBlob(&reference).await?;
            let remoteHasBlob = remoteHasBlob(session, &reference).await?;
            match (localHasBlob, remoteHasBlob) {
                (true, true) => {}
                (true, false) => self.pushLocalBlobToRemote(session, &reference).await?,
                (false, true) => self.pushRemoteBlobToLocal(session, &reference).await?,
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

    /// Pushes one locally available blob into the paired CoreNode.
    async fn pushLocalBlobToRemote(
        &self,
        session: &PairedRemoteSession,
        reference: &RuntimeFileSyncReference,
    ) -> Result<(), String> {
        let mut destination = session.clone();
        let mut push = CoreLinkClient::openPush(&mut destination, blobPushRequest(reference)?)
            .await
            .map_err(|error| error.to_string())?;
        let mut offset = 0i64;
        while offset < reference.size {
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
            offset = sendBlobChunk(&mut push, reference, offset, chunk).await?;
        }
        push.close().await.map_err(|error| error.to_string())?;
        if !remoteHasBlob(session, reference).await? {
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
        session: &PairedRemoteSession,
        reference: &RuntimeFileSyncReference,
    ) -> Result<(), String> {
        let mut destination = self.state.localCore.clone();
        let mut push = CoreLinkClient::openPush(&mut destination, blobPushRequest(reference)?)
            .await
            .map_err(|error| error.to_string())?;
        let mut offset = 0i64;
        while offset < reference.size {
            let chunk: Vec<u8> = callRemote(
                session,
                "syncReadBlobChunk",
                json!({
                    "contentHash": reference.contentHash,
                    "offset": offset,
                    "length": SYNC_BLOB_CHUNK_BYTES,
                }),
            )
            .await?;
            offset = sendBlobChunk(&mut push, reference, offset, chunk).await?;
        }
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
                    defaultHostRuntimeTaskSchedulerHost()
                        .waitForHostRuntimeDelay(SPACE_SYNC_PREPARATION_DELAY_MS)
                        .await;
                    let synchronizedRevision = syncMutationRevision();
                    if let Err(error) = service.synchronizeOnce().await {
                        operit_util::AppLogger::AppLogger::w(
                            "SpacePersistenceSyncService",
                            &format!("Space persistence sync failed: {error}"),
                        );
                    }
                    service
                        .state
                        .synchronizationScheduled
                        .store(false, Ordering::Release);
                    if syncMutationRevision() != synchronizedRevision {
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
            return Err(error.to_string());
        }
        Ok(())
    }

    /// Validates that every direct Space peer has one unambiguous outbound session.
    fn validateDirectPeerSessions(
        &self,
        sessions: &BTreeMap<String, PairedRemoteSessionRecord>,
    ) -> Result<(), String> {
        let mut sessionNameByPeer = BTreeMap::<String, String>::new();
        for (name, record) in sessions {
            if let Some(existingName) =
                sessionNameByPeer.insert(record.coreDeviceId.clone(), name.clone())
            {
                return Err(format!(
                    "multiple direct pairings target CoreNode {}: {}, {}",
                    record.coreDeviceId, existingName, name
                ));
            }
        }
        Ok(())
    }

    /// Opens the bidirectional Peer Link carrier for one direct outbound pairing.
    async fn ensurePeerLink(
        &self,
        localNodeId: &str,
        record: &PairedRemoteSessionRecord,
    ) -> Result<(), String> {
        if isPeerLinkActive(localNodeId, &record.coreDeviceId)? {
            return Ok(());
        }
        let session = PairedRemoteSession::fromRecord(record.clone())?;
        openOutboundPeerLink(
            session,
            coreNodeTransportClient(self.state.nodeRouter.clone()),
            self.state.spaceStore.clone(),
        )
        .await?;
        Ok(())
    }

    /// Resolves a named persisted outbound record into its authenticated remote session.
    fn pairedSession(
        &self,
        name: &str,
    ) -> Result<(PairedRemoteSessionRecord, PairedRemoteSession), String> {
        let sessions = self.state.linkAccessStore.outboundSessions()?;
        let record = sessions
            .get(name)
            .cloned()
            .ok_or_else(|| format!("paired remote runtime does not exist: {name}"))?;
        let session = PairedRemoteSession::fromRecord(record.clone())?;
        Ok((record, session))
    }

    /// Invokes one local application method through the active in-process Core.
    async fn callLocal<T>(&self, methodName: &str, args: Value) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let response = CoreLinkSharedClient::call(
            &self.state.localCore,
            applicationCallRequest(methodName, args)?,
        )
        .await;
        decodeCoreResponse(response.result.map_err(|error| error.to_string())?)
    }
}

/// Reports whether the paired CoreNode owns one verified synchronization blob.
async fn remoteHasBlob(
    session: &PairedRemoteSession,
    reference: &RuntimeFileSyncReference,
) -> Result<bool, String> {
    callRemote(
        session,
        "syncBlobExists",
        json!({
            "contentHash": reference.contentHash,
            "size": reference.size,
        }),
    )
    .await
}

/// Builds one service reverse-stream request for a complete synchronization blob.
#[allow(non_snake_case)]
fn blobPushRequest(reference: &RuntimeFileSyncReference) -> Result<CorePushRequest, String> {
    Ok(CorePushRequest::new(
        format!("space-persistence-blob-{}", currentTimeMillis()),
        "services.syncBlobTransferManager",
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

/// Returns the process registry of active per-CoreNode persistence workers.
fn persistenceServices() -> &'static Mutex<BTreeMap<String, Arc<SpacePersistenceSyncState>>> {
    SPACE_SYNC_SERVICES.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Invokes one application method through an authenticated paired remote session.
async fn callRemote<T>(
    session: &PairedRemoteSession,
    methodName: &str,
    args: Value,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = session
        .call(applicationCallRequest(methodName, args)?)
        .await?;
    decodeCoreResponse(response.result.map_err(|error| error.to_string())?)
}

/// Invokes one generated service method through an authenticated paired remote session.
#[allow(non_snake_case)]
async fn callRemoteService<T>(
    session: &PairedRemoteSession,
    targetPath: &str,
    methodName: &str,
    args: Value,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = session
        .call(serviceCallRequest(targetPath, methodName, args)?)
        .await?;
    decodeCoreResponse(response.result.map_err(|error| error.to_string())?)
}

/// Builds one Link request for an application-level runtime operation.
fn applicationCallRequest(methodName: &str, args: Value) -> Result<CoreCallRequest, String> {
    Ok(CoreCallRequest::new(
        format!("space-persistence-{methodName}-{}", currentTimeMillis()),
        "application",
        methodName,
        toCoreValue(args).map_err(|error| error.to_string())?,
    ))
}

/// Builds one Link request for a generated service operation.
#[allow(non_snake_case)]
fn serviceCallRequest(
    targetPath: &str,
    methodName: &str,
    args: Value,
) -> Result<CoreCallRequest, String> {
    Ok(CoreCallRequest::new(
        format!("space-persistence-{methodName}-{}", currentTimeMillis()),
        targetPath,
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

/// Verifies that the endpoint answered for the paired runtime identity stored locally.
fn ensureRemoteIdentity(
    record: &PairedRemoteSessionRecord,
    coreDeviceId: &str,
) -> Result<(), String> {
    if coreDeviceId != record.coreDeviceId {
        return Err("remote runtime identity changed".to_string());
    }
    Ok(())
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
