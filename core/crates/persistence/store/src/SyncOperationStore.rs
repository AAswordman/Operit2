use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::tryCurrentTimeMillis;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_PREFERENCES_PAYLOADS_DIR_PATH;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::PreferencesEncryption::PreferencesEncryption;
use crate::RuntimeStorageHost::{defaultRuntimeStorageHost, runtimeStoragePath};
use crate::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Debug, Error)]
pub enum SyncOperationStoreError {
    #[error("host error: {0}")]
    Host(#[from] operit_host_api::HostError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncClock {
    pub sequences: BTreeMap<String, i64>,
}

impl SyncClock {
    /// Creates an empty vector clock with no device sequence entries.
    pub fn empty() -> Self {
        Self {
            sequences: BTreeMap::new(),
        }
    }

    /// Returns the last known sequence for a device.
    pub fn sequenceFor(&self, deviceId: &str) -> i64 {
        match self.sequences.get(deviceId) {
            Some(sequence) => *sequence,
            None => 0,
        }
    }

    /// Records the last known sequence for a device.
    pub fn setSequence(&mut self, deviceId: impl Into<String>, sequence: i64) {
        self.sequences.insert(deviceId.into(), sequence);
    }

    /// Returns whether this clock has observed every sequence required by another clock.
    pub fn includes(&self, required: &Self) -> bool {
        required
            .sequences
            .iter()
            .all(|(deviceId, sequence)| self.sequenceFor(deviceId) >= *sequence)
    }
}

/// Declares whether one operation is a replaceable entity state or an ordered transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperationSemantics {
    EntityState,
    Transaction,
}

impl SyncOperationSemantics {
    /// Returns the canonical SQL storage value for this operation semantic.
    #[allow(non_snake_case)]
    pub const fn storageValue(self) -> &'static str {
        match self {
            Self::EntityState => "entity_state",
            Self::Transaction => "transaction",
        }
    }

    /// Parses one canonical SQL storage value into operation semantics.
    #[allow(non_snake_case)]
    pub fn fromStorageValue(value: &str) -> Result<Self, String> {
        match value {
            "entity_state" => Ok(Self::EntityState),
            "transaction" => Ok(Self::Transaction),
            other => Err(format!("unsupported sync operation semantics: {other}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyncOperation {
    pub opId: String,
    pub originDeviceId: String,
    pub sequence: i64,
    pub domain: String,
    pub entityType: String,
    pub entityId: String,
    pub operation: String,
    pub semantics: SyncOperationSemantics,
    pub payload: Value,
    pub createdAt: i64,
    pub schemaVersion: i32,
}

/// Defines the deterministic total order used to resolve concurrent entity operations.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SyncOperationOrder {
    pub createdAt: i64,
    pub originDeviceId: String,
    pub sequence: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct StoredSyncEntityVersion {
    entityKey: String,
    order: SyncOperationOrder,
}

impl SyncOperationOrder {
    /// Builds the conflict order carried by one synchronization operation.
    pub fn fromOperation(operation: &SyncOperation) -> Self {
        Self {
            createdAt: operation.createdAt,
            originDeviceId: operation.originDeviceId.clone(),
            sequence: operation.sequence,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewSyncOperation {
    pub domain: String,
    pub entityType: String,
    pub entityId: String,
    pub operation: String,
    pub semantics: SyncOperationSemantics,
    pub payload: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredEncryptedSyncPayload {
    format: String,
    envelope: Value,
}

const ENCRYPTED_SYNC_PAYLOAD_FORMAT: &str = "operit.sync.encrypted_payload";

static SYNC_MUTATION_REVISION: AtomicU64 = AtomicU64::new(0);
static SYNC_MUTATION_LISTENER_ID: AtomicUsize = AtomicUsize::new(1);
static SYNC_MUTATION_LISTENERS: OnceLock<Mutex<BTreeMap<usize, Arc<dyn Fn() + Send + Sync>>>> =
    OnceLock::new();

/// Removes one process-local synchronization mutation listener when its owner stops.
pub struct SyncMutationSubscription {
    listenerId: usize,
}

impl Drop for SyncMutationSubscription {
    /// Detaches the synchronization mutation listener owned by this subscription.
    fn drop(&mut self) {
        if let Some(listeners) = SYNC_MUTATION_LISTENERS.get() {
            if let Ok(mut listeners) = listeners.lock() {
                listeners.remove(&self.listenerId);
            }
        }
    }
}

/// Returns the process-local revision of newly recorded synchronization work.
#[allow(non_snake_case)]
pub fn syncMutationRevision() -> u64 {
    SYNC_MUTATION_REVISION.load(Ordering::Acquire)
}

/// Registers one listener invoked whenever persistent synchronization work changes.
#[allow(non_snake_case)]
pub fn subscribeSyncMutations(
    listener: impl Fn() + Send + Sync + 'static,
) -> SyncMutationSubscription {
    let listenerId = SYNC_MUTATION_LISTENER_ID.fetch_add(1, Ordering::Relaxed);
    SYNC_MUTATION_LISTENERS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .expect("sync mutation listener mutex poisoned")
        .insert(listenerId, Arc::new(listener));
    SyncMutationSubscription { listenerId }
}

/// Publishes that persistent synchronization work is ready for exchange.
#[allow(non_snake_case)]
pub(crate) fn publishSyncMutation() {
    SYNC_MUTATION_REVISION.fetch_add(1, Ordering::AcqRel);
    let listeners = SYNC_MUTATION_LISTENERS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .expect("sync mutation listener mutex poisoned")
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for listener in listeners {
        listener();
    }
}

#[derive(Clone)]
pub struct SyncOperationStore {
    storageHost: Arc<dyn RuntimeStorageHost>,
    rootPath: String,
    sharedState: Arc<SyncOperationStoreSharedState>,
}

#[derive(Default)]
struct SyncOperationStoreSharedState {
    clock: Mutex<SyncOperationCachedValue<SyncClock>>,
    devices: Mutex<SyncOperationCachedValue<Vec<String>>>,
    entityVersions: Mutex<SyncOperationCachedValue<BTreeMap<String, SyncOperationOrder>>>,
    exportFloors: Mutex<SyncOperationCachedValue<BTreeMap<String, i64>>>,
    localDeviceId: Mutex<SyncOperationCachedValue<Option<String>>>,
    operationLogs: Mutex<HashMap<String, Arc<Mutex<SyncOperationLogIndex>>>>,
}

struct SyncOperationCachedValue<T> {
    loaded: bool,
    value: T,
}

impl<T> Default for SyncOperationCachedValue<T>
where
    T: Default,
{
    /// Creates one unloaded metadata cache with its type's empty value.
    fn default() -> Self {
        Self {
            loaded: false,
            value: T::default(),
        }
    }
}

#[derive(Default)]
struct SyncOperationLogIndex {
    loaded: bool,
    operationIds: BTreeSet<String>,
    highestSequence: i64,
}

struct SyncOperationStoreRegistryKey {
    storageHost: Arc<dyn RuntimeStorageHost>,
    rootPath: String,
}

impl PartialEq for SyncOperationStoreRegistryKey {
    /// Compares registry keys by storage-host identity and synchronization root.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.storageHost, &other.storageHost) && self.rootPath == other.rootPath
    }
}

impl Eq for SyncOperationStoreRegistryKey {}

impl Hash for SyncOperationStoreRegistryKey {
    /// Hashes registry keys by storage-host identity and synchronization root.
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.storageHost) as *const () as usize).hash(state);
        self.rootPath.hash(state);
    }
}

impl SyncOperationStore {
    /// Creates a sync operation store over an explicit storage host and root path.
    pub fn new(storageHost: Arc<dyn RuntimeStorageHost>, rootPath: impl Into<String>) -> Self {
        let rootPath = rootPath.into();
        let sharedState = syncOperationStoreSharedState(&storageHost, &rootPath);
        Self {
            storageHost,
            rootPath,
            sharedState,
        }
    }

    /// Creates the store inside the canonical runtime sync directory.
    pub fn native(paths: RuntimeStorePaths) -> Self {
        Self::new(
            defaultRuntimeStorageHost(),
            runtimeStoragePath(&paths.sync_dir()),
        )
    }

    /// Creates the store inside the adjacent sync directory beside runtime data.
    #[allow(non_snake_case)]
    pub fn adjacentTo(paths: RuntimeStorePaths) -> Self {
        Self::new(
            defaultRuntimeStorageHost(),
            runtimeStoragePath(&paths.adjacent_sync_dir()),
        )
    }

    /// Appends a local operation and advances the local device sequence.
    pub fn appendLocalOperation(
        &self,
        originDeviceId: &str,
        operation: NewSyncOperation,
    ) -> Result<SyncOperation, SyncOperationStoreError> {
        let mut clockState = lockSyncState(&self.sharedState.clock, "clock")?;
        let operationLog = self.operationLog(originDeviceId)?;
        let mut operationLog = lockSyncState(&operationLog, "operation log")?;
        self.loadOperationLogIndex(originDeviceId, &mut operationLog)?;
        let mut clock = self
            .loadCachedJson(&mut clockState, &self.clockPath())?
            .clone();
        let sequence = clock
            .sequenceFor(originDeviceId)
            .max(operationLog.highestSequence)
            + 1;
        let op = SyncOperation {
            opId: format!("{originDeviceId}:{sequence}"),
            originDeviceId: originDeviceId.to_string(),
            sequence,
            domain: operation.domain,
            entityType: operation.entityType,
            entityId: operation.entityId,
            operation: operation.operation,
            semantics: operation.semantics,
            payload: operation.payload,
            createdAt: currentTimeMillis()?,
            schemaVersion: 1,
        };
        self.appendOperationLine(&op)?;
        operationLog.operationIds.insert(op.opId.clone());
        operationLog.highestSequence = sequence;
        clock.setSequence(originDeviceId.to_string(), sequence);
        self.writeLocalClockValue(&clock)?;
        clockState.value = clock;
        drop(operationLog);
        drop(clockState);
        self.registerDevice(originDeviceId)?;
        self.recordAppliedOperation(&op)?;
        publishSyncMutation();
        Ok(op)
    }

    /// Returns the stable local device identifier for this sync store.
    pub fn localDeviceId(&self) -> Result<String, SyncOperationStoreError> {
        let mut localDeviceIdState =
            lockSyncState(&self.sharedState.localDeviceId, "local device id")?;
        if localDeviceIdState.loaded {
            return localDeviceIdState.value.clone().ok_or_else(|| {
                SyncOperationStoreError::Message(
                    "loaded local device id cache is unexpectedly empty".to_string(),
                )
            });
        }
        let path = self.localDeviceIdPath();
        if self.storageHost.exists(&path)? {
            let content = String::from_utf8(self.storageHost.readBytes(&path)?)
                .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
            let value = content.trim().to_string();
            if !value.is_empty() {
                localDeviceIdState.loaded = true;
                localDeviceIdState.value = Some(value.clone());
                return Ok(value);
            }
        }
        let now = currentTimeMillis()?;
        let mut hasher = DefaultHasher::new();
        self.rootPath.hash(&mut hasher);
        now.hash(&mut hasher);
        let deviceId = format!("core-{now}-{:016x}", hasher.finish());
        self.storageHost.writeBytes(&path, deviceId.as_bytes())?;
        localDeviceIdState.loaded = true;
        localDeviceIdState.value = Some(deviceId.clone());
        Ok(deviceId)
    }

    /// Appends an already materialized sync operation to the operation log.
    pub fn appendOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        self.appendOperations(std::slice::from_ref(operation))
    }

    /// Appends and observes an operation batch with one clock write and one append per origin.
    #[allow(non_snake_case)]
    pub fn appendOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        if operations.is_empty() {
            return Ok(());
        }
        let encryption = if operations.iter().any(encryptedPreferenceSyncOperation) {
            Some(
                PreferencesEncryption::load_or_create(self.storageHost.as_ref())
                    .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?,
            )
        } else {
            None
        };
        let mut operationsByOrigin = BTreeMap::<String, Vec<&SyncOperation>>::new();
        for operation in operations {
            operationsByOrigin
                .entry(operation.originDeviceId.clone())
                .or_default()
                .push(operation);
        }

        let mut clockState = lockSyncState(&self.sharedState.clock, "clock")?;
        let mut clock = self
            .loadCachedJson(&mut clockState, &self.clockPath())?
            .clone();
        let mut clockChanged = false;
        let mut appendedOrigins = BTreeSet::new();
        for (originDeviceId, originOperations) in operationsByOrigin {
            let operationLog = self.operationLog(&originDeviceId)?;
            let mut operationLog = lockSyncState(&operationLog, "operation log")?;
            self.loadOperationLogIndex(&originDeviceId, &mut operationLog)?;
            let mut content = Vec::new();
            let mut appendedOperations = Vec::new();
            let mut appendedOperationIds = BTreeSet::new();
            for operation in originOperations {
                if operationLog.operationIds.contains(&operation.opId)
                    || appendedOperationIds.contains(&operation.opId)
                {
                    if operation.sequence > clock.sequenceFor(&originDeviceId) {
                        clock.setSequence(originDeviceId.clone(), operation.sequence);
                        clockChanged = true;
                    }
                    continue;
                }
                if operation.sequence <= clock.sequenceFor(&originDeviceId) {
                    continue;
                }
                let storedOperation =
                    self.encodeOperationPayloadWithEncryption(operation, encryption.as_ref())?;
                let mut line = serde_json::to_vec(&storedOperation)?;
                line.push(b'\n');
                content.extend_from_slice(&line);
                appendedOperationIds.insert(operation.opId.clone());
                appendedOperations.push(operation);
                clock.setSequence(originDeviceId.clone(), operation.sequence);
                clockChanged = true;
            }
            if !content.is_empty() {
                self.storageHost
                    .appendBytes(&self.operationsPath(&originDeviceId), &content)?;
                for operation in appendedOperations {
                    operationLog.operationIds.insert(operation.opId.clone());
                    operationLog.highestSequence =
                        operationLog.highestSequence.max(operation.sequence);
                }
                appendedOrigins.insert(originDeviceId);
            }
        }
        if clockChanged {
            self.writeLocalClockValue(&clock)?;
            clockState.value = clock;
        }
        drop(clockState);
        let didMutate = clockChanged || !appendedOrigins.is_empty();
        self.registerDevices(&appendedOrigins)?;
        if didMutate {
            publishSyncMutation();
        }
        Ok(())
    }

    /// Persists one operation without claiming that preceding source sequences were received.
    #[allow(non_snake_case)]
    pub fn appendUnobservedOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        let operationLog = self.operationLog(&operation.originDeviceId)?;
        let mut operationLog = lockSyncState(&operationLog, "operation log")?;
        self.loadOperationLogIndex(&operation.originDeviceId, &mut operationLog)?;
        if operationLog.operationIds.contains(&operation.opId) {
            return Ok(());
        }
        self.appendOperationLine(operation)?;
        operationLog.operationIds.insert(operation.opId.clone());
        operationLog.highestSequence = operationLog.highestSequence.max(operation.sequence);
        drop(operationLog);
        self.registerDevice(&operation.originDeviceId)?;
        publishSyncMutation();
        Ok(())
    }

    /// Returns operations whose sequence is newer than the provided clock.
    pub fn operationsSince(
        &self,
        clock: &SyncClock,
        domains: &[String],
        limit: usize,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let domainSet = domains.iter().cloned().collect::<BTreeSet<_>>();
        let exportFloors = {
            let mut state = lockSyncState(&self.sharedState.exportFloors, "export floors")?;
            self.loadCachedJson(&mut state, &self.exportFloorsPath())?.clone()
        };
        let mut out = Vec::new();
        for deviceId in self.devices()? {
            let operationLog = self.operationLog(&deviceId)?;
            let content = {
                let _operationLogGuard = lockSyncState(&operationLog, "operation log")?;
                self.readOperationLog(&deviceId)?
            };
            for operation in self.decodeOperationLog(&content)? {
                if operation.sequence <= clock.sequenceFor(&deviceId) {
                    continue;
                }
                if operation.sequence <= exportFloors.get(&deviceId).copied().unwrap_or(0) {
                    continue;
                }
                if !domainSet.is_empty() && !domainSet.contains(&operation.domain) {
                    continue;
                }
                out.push(operation);
            }
        }
        out.sort_by(|left, right| {
            left.createdAt
                .cmp(&right.createdAt)
                .then(left.originDeviceId.cmp(&right.originDeviceId))
                .then(left.sequence.cmp(&right.sequence))
        });
        let mut out = compactSyncOperations(out);
        out.truncate(limit);
        Ok(out)
    }

    /// Reads the local vector clock from storage.
    pub fn localClock(&self) -> Result<SyncClock, SyncOperationStoreError> {
        let mut clockState = lockSyncState(&self.sharedState.clock, "clock")?;
        Ok(self
            .loadCachedJson(&mut clockState, &self.clockPath())?
            .clone())
    }

    /// Writes the local vector clock to storage.
    pub fn writeLocalClock(&self, clock: &SyncClock) -> Result<(), SyncOperationStoreError> {
        let mut clockState = lockSyncState(&self.sharedState.clock, "clock")?;
        self.writeLocalClockValue(clock)?;
        clockState.loaded = true;
        clockState.value = clock.clone();
        Ok(())
    }

    /// Writes the local vector clock while its metadata lock is held.
    #[allow(non_snake_case)]
    fn writeLocalClockValue(&self, clock: &SyncClock) -> Result<(), SyncOperationStoreError> {
        self.writeJson(&self.clockPath(), clock)
    }

    /// Marks the current local operations as outside the next Space membership.
    #[allow(non_snake_case)]
    pub fn markLocalOperationsUnexportable(&self) -> Result<(), SyncOperationStoreError> {
        let deviceId = self.localDeviceId()?;
        let sequence = self.localClock()?.sequenceFor(&deviceId);
        self.setExportFloor(&deviceId, sequence)
    }

    /// Sets the highest sequence excluded from future synchronization exports.
    #[allow(non_snake_case)]
    pub fn setExportFloor(
        &self,
        originDeviceId: &str,
        sequence: i64,
    ) -> Result<(), SyncOperationStoreError> {
        let mut state = lockSyncState(&self.sharedState.exportFloors, "export floors")?;
        let floors = self.loadCachedJson(&mut state, &self.exportFloorsPath())?.clone();
        if floors.get(originDeviceId).copied().unwrap_or(0) >= sequence {
            return Ok(());
        }
        let mut nextFloors = floors;
        nextFloors.insert(originDeviceId.to_string(), sequence);
        self.writeJson(&self.exportFloorsPath(), &nextFloors)?;
        state.value = nextFloors;
        Ok(())
    }

    /// Returns the highest sequence excluded from future synchronization exports.
    #[allow(non_snake_case)]
    pub fn exportFloorFor(&self, originDeviceId: &str) -> Result<i64, SyncOperationStoreError> {
        let mut state = lockSyncState(&self.sharedState.exportFloors, "export floors")?;
        Ok(self
            .loadCachedJson(&mut state, &self.exportFloorsPath())?
            .get(originDeviceId)
            .copied()
            .unwrap_or(0))
    }

    /// Records bootstrap operations as the current materialized entity versions.
    #[allow(non_snake_case)]
    pub fn recordBootstrapAppliedOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        if operations.is_empty() {
            return Ok(());
        }
        let mut entityVersionsState =
            lockSyncState(&self.sharedState.entityVersions, "entity versions")?;
        self.loadCachedEntityVersions(&mut entityVersionsState)?;
        let mut pendingVersions = BTreeMap::new();
        for operation in operations {
            let key = syncEntityVersionKey(operation)?;
            let incoming = SyncOperationOrder::fromOperation(operation);
            if pendingVersions
                .get(&key)
                .map(|current| incoming > *current)
                .unwrap_or(true)
            {
                pendingVersions.insert(key, incoming);
            }
        }
        let mut content = Vec::new();
        for (entityKey, order) in &pendingVersions {
            content.extend_from_slice(&serde_json::to_vec(&StoredSyncEntityVersion {
                entityKey: entityKey.clone(),
                order: order.clone(),
            })?);
            content.push(b'\n');
        }
        self.storageHost
            .appendBytes(&self.entityVersionsPath(), &content)?;
        for (entityKey, order) in pendingVersions {
            entityVersionsState.value.insert(entityKey, order);
        }
        Ok(())
    }

    /// Returns whether an entity operation is newer than the last operation applied locally.
    #[allow(non_snake_case)]
    pub fn shouldApplyOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<bool, SyncOperationStoreError> {
        Ok(self.shouldApplyOperations(std::slice::from_ref(operation))?[0])
    }

    /// Resolves an ordered operation batch against one in-memory entity-version snapshot.
    #[allow(non_snake_case)]
    pub fn shouldApplyOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<Vec<bool>, SyncOperationStoreError> {
        let mut entityVersionsState =
            lockSyncState(&self.sharedState.entityVersions, "entity versions")?;
        let versions = self.loadCachedEntityVersions(&mut entityVersionsState)?;
        let mut pendingVersions = BTreeMap::new();
        let mut decisions = Vec::with_capacity(operations.len());
        for operation in operations {
            let key = syncEntityVersionKey(operation)?;
            let incoming = SyncOperationOrder::fromOperation(operation);
            let shouldApply = pendingVersions
                .get(&key)
                .or_else(|| versions.get(&key))
                .map(|current| incoming > *current)
                .unwrap_or(true);
            if shouldApply {
                pendingVersions.insert(key, incoming);
            }
            decisions.push(shouldApply);
        }
        Ok(decisions)
    }

    /// Records the newest operation that has been materialized for one synchronized entity.
    #[allow(non_snake_case)]
    pub fn recordAppliedOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        self.recordAppliedOperations(std::slice::from_ref(operation))
    }

    /// Records one applied operation batch with a single entity-version metadata write.
    #[allow(non_snake_case)]
    pub fn recordAppliedOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        if operations.is_empty() {
            return Ok(());
        }
        let mut entityVersionsState =
            lockSyncState(&self.sharedState.entityVersions, "entity versions")?;
        let versions = self.loadCachedEntityVersions(&mut entityVersionsState)?;
        let mut pendingVersions = BTreeMap::new();
        for operation in operations {
            let key = syncEntityVersionKey(operation)?;
            let incoming = SyncOperationOrder::fromOperation(operation);
            if pendingVersions
                .get(&key)
                .or_else(|| versions.get(&key))
                .map(|current| incoming <= *current)
                .unwrap_or(false)
            {
                continue;
            }
            pendingVersions.insert(key, incoming);
        }
        if pendingVersions.is_empty() {
            return Ok(());
        }
        let mut content = Vec::new();
        for (entityKey, order) in &pendingVersions {
            content.extend_from_slice(&serde_json::to_vec(&StoredSyncEntityVersion {
                entityKey: entityKey.clone(),
                order: order.clone(),
            })?);
            content.push(b'\n');
        }
        self.storageHost
            .appendBytes(&self.entityVersionsPath(), &content)?;
        for (entityKey, order) in pendingVersions {
            entityVersionsState.value.insert(entityKey, order);
        }
        Ok(())
    }

    /// Persists a local observation of a sync operation with origin metadata.
    #[allow(non_snake_case)]
    pub fn observeOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        let mut clockState = lockSyncState(&self.sharedState.clock, "clock")?;
        let mut clock = self
            .loadCachedJson(&mut clockState, &self.clockPath())?
            .clone();
        if !self.observeOperationWithClock(operation, &mut clock)? {
            return Ok(());
        }
        clockState.value = clock;
        Ok(())
    }

    /// Advances an already locked vector clock from one operation.
    #[allow(non_snake_case)]
    fn observeOperationWithClock(
        &self,
        operation: &SyncOperation,
        clock: &mut SyncClock,
    ) -> Result<bool, SyncOperationStoreError> {
        if operation.sequence > clock.sequenceFor(&operation.originDeviceId) {
            clock.setSequence(operation.originDeviceId.clone(), operation.sequence);
            self.writeLocalClockValue(clock)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Reads one origin operation-log byte snapshot while its per-origin lock is held.
    #[allow(non_snake_case)]
    fn readOperationLog(&self, deviceId: &str) -> Result<String, SyncOperationStoreError> {
        let path = self.operationsPath(deviceId);
        if !self.storageHost.exists(&path)? {
            return Ok(String::new());
        }
        String::from_utf8(self.storageHost.readBytes(&path)?)
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))
    }

    /// Decodes one immutable operation-log snapshot without holding its file lock.
    #[allow(non_snake_case)]
    fn decodeOperationLog(
        &self,
        content: &str,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let mut operations = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let operation: SyncOperation = serde_json::from_str(trimmed)?;
            operations.push(self.decodeOperationPayload(operation)?);
        }
        Ok(operations)
    }

    /// Loads the persisted operation IDs once for one locked origin log.
    #[allow(non_snake_case)]
    fn loadOperationLogIndex(
        &self,
        deviceId: &str,
        index: &mut SyncOperationLogIndex,
    ) -> Result<(), SyncOperationStoreError> {
        if index.loaded {
            return Ok(());
        }
        let content = self.readOperationLog(deviceId)?;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let operation: SyncOperation = serde_json::from_str(trimmed)?;
            index.highestSequence = index.highestSequence.max(operation.sequence);
            index.operationIds.insert(operation.opId);
        }
        index.loaded = true;
        Ok(())
    }

    /// Appends one encoded operation as a complete JSON line.
    #[allow(non_snake_case)]
    fn appendOperationLine(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        let storedOperation = self.encodeOperationPayload(operation)?;
        let mut content = serde_json::to_vec(&storedOperation)?;
        content.push(b'\n');
        self.storageHost
            .appendBytes(&self.operationsPath(&operation.originDeviceId), &content)?;
        Ok(())
    }

    /// Encodes operation payloads that are stored encrypted in the sync log.
    fn encodeOperationPayload(
        &self,
        operation: &SyncOperation,
    ) -> Result<SyncOperation, SyncOperationStoreError> {
        if !encryptedPreferenceSyncOperation(operation) {
            return Ok(operation.clone());
        }
        let encryption = PreferencesEncryption::load_or_create(self.storageHost.as_ref())
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
        self.encodeOperationPayloadWithEncryption(operation, Some(&encryption))
    }

    /// Encodes one operation using a batch-owned preferences encryption context.
    #[allow(non_snake_case)]
    fn encodeOperationPayloadWithEncryption(
        &self,
        operation: &SyncOperation,
        encryption: Option<&PreferencesEncryption>,
    ) -> Result<SyncOperation, SyncOperationStoreError> {
        if !encryptedPreferenceSyncOperation(operation) {
            return Ok(operation.clone());
        }
        let encryption = encryption.ok_or_else(|| {
            SyncOperationStoreError::Message(
                "encrypted preferences operation requires an encryption context".to_string(),
            )
        })?;
        let payloadBytes = serde_json::to_vec(&operation.payload)?;
        let encryptedPayload = encryption
            .encrypt(&encryptedPreferenceSyncPayloadAad(operation), &payloadBytes)
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
        let mut storedOperation = operation.clone();
        storedOperation.payload = serde_json::to_value(StoredEncryptedSyncPayload {
            format: ENCRYPTED_SYNC_PAYLOAD_FORMAT.to_string(),
            envelope: serde_json::from_slice(&encryptedPayload)?,
        })?;
        Ok(storedOperation)
    }

    /// Decodes operation payloads that are stored encrypted in the sync log.
    fn decodeOperationPayload(
        &self,
        operation: SyncOperation,
    ) -> Result<SyncOperation, SyncOperationStoreError> {
        if !storedEncryptedSyncPayload(&operation.payload) {
            return Ok(operation);
        }
        let encryption = PreferencesEncryption::load_or_create(self.storageHost.as_ref())
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
        let storedPayload: StoredEncryptedSyncPayload =
            serde_json::from_value(operation.payload.clone())?;
        let payloadBytes = serde_json::to_vec(&storedPayload.envelope)?;
        let decryptedPayload = encryption
            .decrypt(
                &encryptedPreferenceSyncPayloadAad(&operation),
                &payloadBytes,
            )
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
        let mut decodedOperation = operation;
        decodedOperation.payload = serde_json::from_slice(&decryptedPayload)?;
        Ok(decodedOperation)
    }

    /// Reads a snapshot of registered origin devices under its metadata lock.
    fn devices(&self) -> Result<Vec<String>, SyncOperationStoreError> {
        let mut devicesState = lockSyncState(&self.sharedState.devices, "devices")?;
        Ok(self
            .loadCachedJson(&mut devicesState, &self.devicesPath())?
            .clone())
    }

    /// Registers one origin device under the dedicated device-list lock.
    #[allow(non_snake_case)]
    fn registerDevice(&self, deviceId: &str) -> Result<(), SyncOperationStoreError> {
        self.registerDevices(&BTreeSet::from([deviceId.to_string()]))
    }

    /// Registers an origin-device batch with one device-list metadata write.
    #[allow(non_snake_case)]
    fn registerDevices(&self, deviceIds: &BTreeSet<String>) -> Result<(), SyncOperationStoreError> {
        if deviceIds.is_empty() {
            return Ok(());
        }
        let mut devicesState = lockSyncState(&self.sharedState.devices, "devices")?;
        let mut devices = self
            .loadCachedJson(&mut devicesState, &self.devicesPath())?
            .clone();
        let previousLength = devices.len();
        for deviceId in deviceIds {
            if !devices.iter().any(|existing| existing == deviceId) {
                devices.push(deviceId.clone());
            }
        }
        if devices.len() == previousLength {
            return Ok(());
        }
        devices.sort();
        self.writeJson(&self.devicesPath(), &devices)?;
        devicesState.value = devices;
        Ok(())
    }

    /// Returns the lock and in-memory index dedicated to one origin operation log.
    #[allow(non_snake_case)]
    fn operationLog(
        &self,
        deviceId: &str,
    ) -> Result<Arc<Mutex<SyncOperationLogIndex>>, SyncOperationStoreError> {
        let mut operationLogs =
            lockSyncState(&self.sharedState.operationLogs, "operation log registry")?;
        Ok(Arc::clone(
            operationLogs
                .entry(deviceId.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(SyncOperationLogIndex::default()))),
        ))
    }

    /// Loads one JSON metadata value once into its process-shared shard cache.
    #[allow(non_snake_case)]
    fn loadCachedJson<'a, T>(
        &self,
        state: &'a mut SyncOperationCachedValue<T>,
        path: &str,
    ) -> Result<&'a T, SyncOperationStoreError>
    where
        T: serde::de::DeserializeOwned + Default,
    {
        if !state.loaded {
            state.value = self.readJson(path)?;
            state.loaded = true;
        }
        Ok(&state.value)
    }

    /// Loads the append-only entity-version journal into its latest-order cache.
    #[allow(non_snake_case)]
    fn loadCachedEntityVersions<'a>(
        &self,
        state: &'a mut SyncOperationCachedValue<BTreeMap<String, SyncOperationOrder>>,
    ) -> Result<&'a BTreeMap<String, SyncOperationOrder>, SyncOperationStoreError> {
        if state.loaded {
            return Ok(&state.value);
        }
        let path = self.entityVersionsPath();
        if self.storageHost.exists(&path)? {
            let content = String::from_utf8(self.storageHost.readBytes(&path)?)
                .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let stored: StoredSyncEntityVersion = serde_json::from_str(trimmed)?;
                if state
                    .value
                    .get(&stored.entityKey)
                    .map(|current| stored.order <= *current)
                    .unwrap_or(false)
                {
                    continue;
                }
                state.value.insert(stored.entityKey, stored.order);
            }
        }
        state.loaded = true;
        Ok(&state.value)
    }

    /// Reads one JSON metadata document while its dedicated metadata lock is held.
    fn readJson<T>(&self, path: &str) -> Result<T, SyncOperationStoreError>
    where
        T: serde::de::DeserializeOwned + Default,
    {
        if !self.storageHost.exists(path)? {
            return Ok(T::default());
        }
        let content = String::from_utf8(self.storageHost.readBytes(path)?)
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
        if content.trim().is_empty() {
            return Ok(T::default());
        }
        Ok(serde_json::from_str(&content)?)
    }

    /// Writes one JSON metadata document while its dedicated metadata lock is held.
    fn writeJson<T>(&self, path: &str, value: &T) -> Result<(), SyncOperationStoreError>
    where
        T: serde::Serialize,
    {
        let content = serde_json::to_vec_pretty(value)?;
        self.storageHost.writeBytes(path, &content)?;
        Ok(())
    }

    /// Returns the vector-clock metadata path.
    #[allow(non_snake_case)]
    fn clockPath(&self) -> String {
        format!("{}/clocks.json", self.rootPath)
    }

    /// Returns the registered-device metadata path.
    #[allow(non_snake_case)]
    fn devicesPath(&self) -> String {
        format!("{}/devices.json", self.rootPath)
    }

    /// Returns the stable local-device identifier path.
    #[allow(non_snake_case)]
    fn localDeviceIdPath(&self) -> String {
        format!("{}/local_device_id", self.rootPath)
    }

    /// Returns the path storing deterministic per-entity conflict versions.
    #[allow(non_snake_case)]
    fn entityVersionsPath(&self) -> String {
        format!("{}/entity_versions.jsonl", self.rootPath)
    }

    /// Returns the local operation export-floor metadata path.
    #[allow(non_snake_case)]
    fn exportFloorsPath(&self) -> String {
        format!("{}/export_floors.json", self.rootPath)
    }

    /// Returns the JSONL operation-log path for one origin device.
    #[allow(non_snake_case)]
    fn operationsPath(&self, deviceId: &str) -> String {
        format!(
            "{}/operations/{}.jsonl",
            self.rootPath,
            storageSafeId(deviceId)
        )
    }
}

/// Returns the lock shards shared by stores using the same host and synchronization root.
#[allow(non_snake_case)]
fn syncOperationStoreSharedState(
    storageHost: &Arc<dyn RuntimeStorageHost>,
    rootPath: &str,
) -> Arc<SyncOperationStoreSharedState> {
    static SHARED_STATES: OnceLock<
        Mutex<HashMap<SyncOperationStoreRegistryKey, Arc<SyncOperationStoreSharedState>>>,
    > = OnceLock::new();
    let states = SHARED_STATES.get_or_init(|| Mutex::new(HashMap::new()));
    let key = SyncOperationStoreRegistryKey {
        storageHost: Arc::clone(storageHost),
        rootPath: rootPath.to_string(),
    };
    let mut states = states
        .lock()
        .expect("SyncOperationStore shared state registry mutex must not be poisoned");
    if let Some(state) = states.get(&key) {
        return Arc::clone(state);
    }
    let state = Arc::new(SyncOperationStoreSharedState::default());
    states.insert(key, Arc::clone(&state));
    state
}

/// Acquires one synchronization-state shard and reports poisoning as a store error.
#[allow(non_snake_case)]
fn lockSyncState<'a, T>(
    state: &'a Mutex<T>,
    name: &str,
) -> Result<MutexGuard<'a, T>, SyncOperationStoreError> {
    state.lock().map_err(|_| {
        SyncOperationStoreError::Message(format!("sync operation store {name} mutex is poisoned"))
    })
}

impl Default for SyncClock {
    /// Creates the default empty vector clock.
    fn default() -> Self {
        Self::empty()
    }
}

/// Returns the current wall-clock timestamp used for operation ordering.
#[allow(non_snake_case)]
fn currentTimeMillis() -> Result<i64, SyncOperationStoreError> {
    tryCurrentTimeMillis().map_err(SyncOperationStoreError::Message)
}

/// Converts one device identifier into a safe operation-log file name.
#[allow(non_snake_case)]
fn storageSafeId(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[allow(non_snake_case)]
/// Reports whether this decoded operation carries encrypted Space preferences.
fn encryptedPreferenceSyncOperation(operation: &SyncOperation) -> bool {
    operation.domain == "preferences"
        && operation.operation == "set"
        && operation.payload.get("encrypted").and_then(Value::as_bool) == Some(true)
}

/// Reports whether one stored operation payload is an encrypted sync envelope.
#[allow(non_snake_case)]
fn storedEncryptedSyncPayload(payload: &Value) -> bool {
    payload.get("format").and_then(Value::as_str) == Some(ENCRYPTED_SYNC_PAYLOAD_FORMAT)
}

#[allow(non_snake_case)]
/// Builds authenticated data for an encrypted preferences sync payload.
fn encryptedPreferenceSyncPayloadAad(operation: &SyncOperation) -> String {
    format!(
        "{RUNTIME_SYNC_PREFERENCES_PAYLOADS_DIR_PATH}/{}",
        operation.entityId
    )
}

/// Compacts replaceable entity states while preserving every transaction operation.
#[allow(non_snake_case)]
pub fn compactSyncOperations(operations: Vec<SyncOperation>) -> Vec<SyncOperation> {
    let mut latestStateOperations = BTreeMap::<(String, String, String, String), i64>::new();
    for operation in &operations {
        if operation.semantics == SyncOperationSemantics::EntityState {
            let key = syncEntityKey(operation);
            let sequence = latestStateOperations
                .entry(key)
                .or_insert(operation.sequence);
            if operation.sequence > *sequence {
                *sequence = operation.sequence;
            }
        }
    }

    let mut compacted = Vec::with_capacity(operations.len());
    for operation in operations {
        if operation.semantics == SyncOperationSemantics::EntityState {
            let key = syncEntityKey(&operation);
            if latestStateOperations.get(&key).copied() != Some(operation.sequence) {
                continue;
            }
        }
        compacted.push(operation);
    }
    compacted
}

#[allow(non_snake_case)]
/// Builds the origin-scoped identity used for operation-log compaction.
fn syncEntityKey(operation: &SyncOperation) -> (String, String, String, String) {
    (
        operation.originDeviceId.clone(),
        operation.domain.clone(),
        operation.entityType.clone(),
        operation.entityId.clone(),
    )
}

/// Encodes one entity identity without relying on path or delimiter parsing.
#[allow(non_snake_case)]
fn syncEntityVersionKey(operation: &SyncOperation) -> Result<String, SyncOperationStoreError> {
    serde_json::to_string(&(
        operation.domain.as_str(),
        operation.entityType.as_str(),
        operation.entityId.as_str(),
    ))
    .map_err(SyncOperationStoreError::from)
}

#[cfg(test)]
#[path = "tests/SyncOperationStoreTests.rs"]
mod SyncOperationStoreTests;
