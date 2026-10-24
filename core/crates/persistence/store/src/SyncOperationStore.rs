use operit_host_api::AtomicCounter::AtomicU64;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::tryCurrentTimeMillis;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_PREFERENCES_PAYLOADS_DIR_PATH;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::NetworkControlStore::NETWORK_CONTROL_SYNC_DOMAIN;
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

/// Largest serialized operation page one synchronization exchange materializes.
/// The page stays far below the Link message ceiling so peak memory is bounded
/// by policy instead of by how much a device has pending.
pub const DEFAULT_SYNC_PAGE_BYTES: usize = 4 * 1024 * 1024;

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

/// Supplies operation membership independently of an application's materialization clock.
pub trait SyncOperationKnowledge {
    /// Returns the prefix below which every source sequence is already known.
    fn sequenceFor(&self, origin: &str) -> i64;
    /// Reports whether this exact source operation is already durably known.
    fn containsSequence(&self, origin: &str, sequence: i64) -> bool;
    /// Finds the next missing sequence after a cursor, skipping known intervals.
    fn nextMissingSequence(&self, origin: &str, after: i64) -> Option<i64>;
}

impl SyncOperationKnowledge for SyncClock {
    /// Reads the contiguous observation boundary used by store callers.
    fn sequenceFor(&self, origin: &str) -> i64 {
        SyncClock::sequenceFor(self, origin)
    }
    /// Tests membership in a complete observed prefix.
    fn containsSequence(&self, origin: &str, sequence: i64) -> bool {
        sequence <= self.sequenceFor(origin)
    }
    /// Skips the complete observed prefix before scanning source records.
    fn nextMissingSequence(&self, origin: &str, after: i64) -> Option<i64> {
        after.max(self.sequenceFor(origin)).checked_add(1)
    }
}

/// Advertises exact durable receipts, including gaps between independent stores.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncReceipt {
    pub ranges: BTreeMap<String, Vec<[i64; 2]>>,
}

impl SyncReceipt {
    /// Rejects malformed ranges rather than acknowledging an ambiguous interval.
    pub fn validate(&self) -> Result<(), String> {
        for (origin, ranges) in &self.ranges {
            if origin.is_empty() || origin.len() > 256 {
                return Err("Invalid synchronization receipt origin".into());
            }
            let mut previous = 0i64;
            for [start, end] in ranges {
                if *start <= 0 || end < start || *start <= previous {
                    return Err(format!(
                        "Invalid synchronization receipt ranges for {origin}"
                    ));
                }
                previous = *end;
            }
        }
        Ok(())
    }
    /// Adds one committed operation without acknowledging any missing predecessor.
    pub fn observe(&mut self, origin: &str, sequence: i64) {
        self.observeRange(origin, sequence, sequence);
    }
    /// Unions one durable source interval with the existing exact intervals.
    pub fn observeRange(&mut self, origin: &str, start: i64, end: i64) {
        let ranges = self.ranges.entry(origin.to_owned()).or_default();
        let first = ranges.partition_point(|range| range[1].saturating_add(1) < start);
        let mut merged = [start, end];
        let mut last = first;
        while last < ranges.len() && ranges[last][0] <= merged[1].saturating_add(1) {
            merged[0] = merged[0].min(ranges[last][0]);
            merged[1] = merged[1].max(ranges[last][1]);
            last += 1;
        }
        ranges.splice(first..last, [merged]);
    }
    /// Combines receipts from journals and transactional storage engines.
    pub fn merge(&mut self, other: Self) {
        for (origin, ranges) in other.ranges {
            for [start, end] in ranges {
                self.observeRange(&origin, start, end);
            }
        }
    }
}

impl SyncOperationKnowledge for SyncReceipt {
    /// Returns only an actually received prefix, leaving sparse gaps requestable.
    fn sequenceFor(&self, origin: &str) -> i64 {
        match self.ranges.get(origin).and_then(|ranges| ranges.first()) {
            Some([1, end]) => *end,
            _ => 0,
        }
    }
    /// Tests exact membership even when later operations have already arrived.
    fn containsSequence(&self, origin: &str, sequence: i64) -> bool {
        let Some(ranges) = self.ranges.get(origin) else {
            return false;
        };
        let index = ranges.partition_point(|range| range[0] <= sequence);
        index > 0 && sequence <= ranges[index - 1][1]
    }
    /// Skips an exact known interval without skipping its missing predecessor.
    fn nextMissingSequence(&self, origin: &str, after: i64) -> Option<i64> {
        let next = after.checked_add(1)?;
        let Some(ranges) = self.ranges.get(origin) else {
            return Some(next);
        };
        let index = ranges.partition_point(|range| range[0] <= next);
        if index > 0 && next <= ranges[index - 1][1] {
            ranges[index - 1][1].checked_add(1)
        } else {
            Some(next)
        }
    }
}

/// Declares whether one operation is a replaceable entity state or an ordered transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperationSemantics {
    /// Replaces one declared entity state according to its owner's conflict order.
    EntityState,
    /// Preserves an immutable ordered mutation for replay and commits the owner's resolved plan atomically.
    /// Conflict resolution belongs to the owner; this flag does not declare cross-dataset atomicity.
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

/// One corrupt operation-log line found while reading an origin journal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationLogLineFinding {
    pub deviceId: String,
    pub lineNumber: u64,
    pub byteOffset: u64,
    pub byteLength: u64,
    pub error: String,
}

/// Raw classification of one origin journal: decodable lines versus corruption.
/// `operations` carries the decoded lines so diagnostics can inspect them
/// without a second read; payload envelopes stay untouched (not decrypted).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OperationLogScan {
    pub deviceId: String,
    pub exists: bool,
    pub totalLines: u64,
    pub decodedLines: u64,
    pub highestSequence: i64,
    pub findings: Vec<OperationLogLineFinding>,
    pub operations: Vec<SyncOperation>,
}

/// One journal line moved to the quarantine sidecar by an explicit repair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantinedOperationLine {
    pub deviceId: String,
    pub byteOffset: u64,
    pub byteLength: u64,
    pub lineHash: String,
    pub quarantinedAt: i64,
    pub line: String,
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
            createdAt: match (
                operation.domain.as_str(),
                operation
                    .payload
                    .get("contractVersion")
                    .and_then(Value::as_u64),
            ) {
                ("preferences", Some(1)) => operation.payload["logicalTime"]
                    .as_i64()
                    .expect("validated preference logical time"),
                ("runtime_file", _) if operation.payload["initial"] == true => 1,
                _ => operation.createdAt,
            },
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
    receiveState: Mutex<SyncOperationCachedValue<SyncReceiveState>>,
    application: Mutex<()>,
    preferenceOutboxes: Mutex<SyncOperationCachedValue<BTreeMap<String, bool>>>,
}

/// Keeps durable receipt membership after completed native history is compacted.
#[derive(Clone, Default, Serialize, Deserialize)]
struct SyncReceiveState {
    receipt: SyncReceipt,
    pending: BTreeMap<String, SyncOperation>,
    #[serde(default)]
    fingerprints: BTreeMap<String, String>,
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
    domainOperations: BTreeMap<String, Vec<SyncOperation>>,
    operationIds: BTreeSet<String>,
    sequenceOffsets: BTreeMap<i64, u64>,
    highestSequence: i64,
    encodedByteLength: u64,
    needsSeparator: bool,
    findings: Vec<OperationLogLineFinding>,
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

    /// Registers a dataset before its first durable state-and-outbox commit.
    pub(crate) fn registerPreferenceOutbox(
        &self,
        path: &str,
        encrypted: bool,
    ) -> Result<(), SyncOperationStoreError> {
        let mut cached =
            lockSyncState(&self.sharedState.preferenceOutboxes, "preference outboxes")?;
        let registryPath = format!("{}/preference_outboxes.json", self.rootPath);
        let mut registry = self.loadCachedJson(&mut cached, &registryPath)?.clone();
        match registry.get(path) {
            Some(previous) if *previous != encrypted => {
                return Err(SyncOperationStoreError::Message(
                    "Conflicting preference outbox encryption policy".into(),
                ))
            }
            Some(_) => return Ok(()),
            None => {}
        }
        registry.insert(path.into(), encrypted);
        self.storageHost
            .writeBytesAtomically(&registryPath, &serde_json::to_vec(&registry)?)?;
        cached.value = registry;
        Ok(())
    }

    /// Publishes only immutable operations whose owning business snapshot has already committed.
    pub fn publishPreferenceOutboxes(&self) -> Result<Vec<String>, SyncOperationStoreError> {
        let registry = {
            let mut cached =
                lockSyncState(&self.sharedState.preferenceOutboxes, "preference outboxes")?;
            self.loadCachedJson(
                &mut cached,
                &format!("{}/preference_outboxes.json", self.rootPath),
            )?
            .clone()
        };
        let mut errors = Vec::new();
        for (path, encrypted) in registry {
            let store = if encrypted {
                crate::PreferencesDataStore::PreferencesDataStore::newEncryptedWithStorage(
                    self.storageHost.clone(),
                    path.clone(),
                )
            } else {
                crate::PreferencesDataStore::PreferencesDataStore::newWithStorage(
                    self.storageHost.clone(),
                    path.clone(),
                )
            };
            if let Err(error) = store.publishOutbox(self) {
                let error = format!("preference outbox path={path}: {error}");
                operit_util::AppLogger::AppLogger::w("CoreSyncTrace", &error);
                errors.push(error);
            }
        }
        Ok(errors)
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
        let operationOffset = operationLog.encodedByteLength + u64::from(operationLog.needsSeparator);
        let writtenBytes = self.appendOperationLine(&op, &mut operationLog)?;
        if let Some(operations) = operationLog.domainOperations.get_mut(&op.domain) {
            operations.push(op.clone());
        }
        operationLog.operationIds.insert(op.opId.clone());
        operationLog
            .sequenceOffsets
            .insert(op.sequence, operationOffset);
        operationLog.highestSequence = sequence;
        operationLog.encodedByteLength += writtenBytes;
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

    /// Completes metadata after an acknowledged log append but failed later
    /// writes. Never allocates a second sequence or appends a duplicate line.
    #[allow(non_snake_case)]
    pub fn recoverAppendedOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        self.appendOperation(operation)?;
        self.registerDevice(&operation.originDeviceId)?;
        self.recordAppliedOperation(operation)?;
        publishSyncMutation();
        Ok(())
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
        // Hosts commonly use the same relative sync root and may initialize
        // within the same millisecond. Their operation origins must be unique.
        let deviceId = format!("core-{now}-{}", uuid::Uuid::new_v4().simple());
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
        let mut identities = BTreeMap::new();
        for operation in operations {
            if let Some(previous) = identities.insert(&operation.opId, operation) {
                if previous != operation {
                    return Err(SyncOperationStoreError::Message(format!(
                        "Conflicting immutable operation {}",
                        operation.opId
                    )));
                }
            }
            self.containsIdenticalOperation(operation)?;
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
        self.registerDevices(&operationsByOrigin.keys().cloned().collect())?;
        let mut clockChanged = false;
        let mut appendedOrigins = BTreeSet::new();
        for (originDeviceId, originOperations) in operationsByOrigin {
            let operationLog = self.operationLog(&originDeviceId)?;
            let mut operationLog = lockSyncState(&operationLog, "operation log")?;
            self.loadOperationLogIndex(&originDeviceId, &mut operationLog)?;
            let mut content = Vec::new();
            let mut appendedOperations = Vec::new();
            let mut appendedOperationIds = BTreeSet::new();
            let mut nextOffset = operationLog.encodedByteLength + u64::from(operationLog.needsSeparator);
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
                let storedOperation =
                    self.encodeOperationPayloadWithEncryption(operation, encryption.as_ref())?;
                let mut line = serde_json::to_vec(&storedOperation)?;
                line.push(b'\n');
                let lineLength = u64::try_from(line.len()).map_err(|_| {
                    SyncOperationStoreError::Message(
                        "sync operation line length does not fit u64".to_string(),
                    )
                })?;
                content.extend_from_slice(&line);
                appendedOperationIds.insert(operation.opId.clone());
                appendedOperations.push((operation, nextOffset, lineLength));
                nextOffset += lineLength;
                if operation.sequence > clock.sequenceFor(&originDeviceId) {
                    clock.setSequence(originDeviceId.clone(), operation.sequence);
                    clockChanged = true;
                }
            }
            if !content.is_empty() {
                self.appendOperationBytes(&originDeviceId, content, &mut operationLog)?;
                for (operation, offset, _) in appendedOperations {
                    if let Some(operations) =
                        operationLog.domainOperations.get_mut(&operation.domain)
                    {
                        operations.push(operation.clone());
                    }
                    operationLog.operationIds.insert(operation.opId.clone());
                    operationLog
                        .sequenceOffsets
                        .insert(operation.sequence, offset);
                    operationLog.highestSequence =
                        operationLog.highestSequence.max(operation.sequence);
                }
                operationLog.encodedByteLength = nextOffset;
                appendedOrigins.insert(originDeviceId);
            }
        }
        if clockChanged {
            self.writeLocalClockValue(&clock)?;
            clockState.value = clock;
        }
        drop(clockState);
        let didMutate = clockChanged || !appendedOrigins.is_empty();
        if didMutate {
            publishSyncMutation();
        }
        Ok(())
    }

    /// Flushes original journals before a committed preference outbox can be acknowledged.
    pub(crate) fn flushOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        let origins = operations.iter().map(|operation| &operation.originDeviceId).collect::<BTreeSet<_>>();
        for origin in origins {
            let log = self.operationLog(origin)?;
            let _log = lockSyncState(&log, "operation log")?;
            self.storageHost.flushFile(&self.operationsPath(origin))?;
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
        let operationOffset = operationLog.encodedByteLength + u64::from(operationLog.needsSeparator);
        let writtenBytes = self.appendOperationLine(operation, &mut operationLog)?;
        if let Some(operations) = operationLog.domainOperations.get_mut(&operation.domain) {
            operations.push(operation.clone());
        }
        operationLog.operationIds.insert(operation.opId.clone());
        operationLog
            .sequenceOffsets
            .insert(operation.sequence, operationOffset);
        operationLog.highestSequence = operationLog.highestSequence.max(operation.sequence);
        operationLog.encodedByteLength += writtenBytes;
        drop(operationLog);
        self.registerDevice(&operation.originDeviceId)?;
        publishSyncMutation();
        Ok(())
    }

    /// Returns operations whose sequence is newer than the provided clock.
    pub fn operationsSince<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        domains: &[String],
        limit: usize,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.operationsSinceWithBudget(clock, domains, limit, usize::MAX, |_| true)
    }

    /// Exports eligible missing operations, filtering before compaction and page limits.
    /// Keeps the first operation even when it exceeds the byte budget so transport can report it.
    pub fn operationsSinceWithBudget<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        domains: &[String],
        limit: usize,
        maxBytes: usize,
        retain: impl Fn(&SyncOperation) -> bool,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.operationsSinceWithExportPolicy(clock, domains, limit, true, maxBytes, retain)
    }

    /// Reads local preference provenance without treating an export boundary as a conflict boundary.
    pub fn preferenceHistory(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.operationsSinceWithExportPolicy(
            &SyncClock::empty(),
            &["preferences".to_owned()],
            usize::MAX,
            false,
            usize::MAX,
            |_| true,
        )
    }

    /// Reads only cross-dataset merge context; each owner reads its own business history under its lock.
    pub fn preferenceMergeHistory(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.operationsSinceWithExportPolicy(
            &SyncClock::empty(),
            &["preferences".to_owned()],
            usize::MAX,
            false,
            usize::MAX,
            crate::PreferenceMerge::isMergeMetadata,
        )
    }

    /// Uses the shared journal cache for exports and internal deterministic state replay.
    fn operationsSinceWithExportPolicy<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        domains: &[String],
        limit: usize,
        applyExportFloors: bool,
        maxBytes: usize,
        retain: impl Fn(&SyncOperation) -> bool,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.publishPreferenceOutboxes()?;
        self.archivedOperationsSinceWithExportPolicy(
            clock,
            domains,
            limit,
            applyExportFloors,
            maxBytes,
            None,
            retain,
        )
    }

    /// Reads archived originals without publishing outboxes or acquiring any preference dataset lock.
    pub(crate) fn archivedPreferenceHistory(
        &self,
        path: &str,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.archivedOperationsSinceWithExportPolicy(
            &SyncClock::empty(),
            &["preferences".to_owned()],
            usize::MAX,
            false,
            usize::MAX,
            Some(path),
            |operation| {
                operation
                    .payload
                    .get("storagePath")
                    .and_then(serde_json::Value::as_str)
                    == Some(path)
            },
        )
    }

    /// Reads committed journal entries without invoking dataset publication during locked replay.
    fn archivedOperationsSinceWithExportPolicy<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        domains: &[String],
        limit: usize,
        applyExportFloors: bool,
        maxBytes: usize,
        preferencePath: Option<&str>,
        retain: impl Fn(&SyncOperation) -> bool,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let domainSet = domains.iter().cloned().collect::<BTreeSet<_>>();
        let exportFloors = if applyExportFloors {
            let mut state = lockSyncState(&self.sharedState.exportFloors, "export floors")?;
            self.loadCachedJson(&mut state, &self.exportFloorsPath())?
                .clone()
        } else {
            BTreeMap::new()
        };
        let mut out = Vec::new();
        for deviceId in self.devices()? {
            let exportFloor = exportFloors.get(&deviceId).copied().unwrap_or(0);
            let businessMinimum = clock.sequenceFor(&deviceId).max(exportFloor);
            // A complete source prefix covers every domain; control acknowledgements cover only control operations.
            let controlMinimum = clock
                .sequenceFor(&networkControlClockKey(&deviceId))
                .max(businessMinimum);
            // Checks the appropriate domain floor and exact receipt membership for both export paths.
            let isMissing = |operation: &SyncOperation| {
                let minimum = if operation.domain == NETWORK_CONTROL_SYNC_DOMAIN {
                    controlMinimum
                } else {
                    businessMinimum
                };
                operation.sequence > minimum
                    && !clock.containsSequence(&deviceId, operation.sequence)
                    && preferencePath.map_or(true, |path| {
                        // Encrypted history keeps the dataset in its canonical entity identity.
                        // Select that dataset before opening any encrypted business payload.
                        if storedEncryptedSyncPayload(&operation.payload) {
                            serde_json::from_str::<Vec<Value>>(&operation.entityId)
                                .is_ok_and(|identity| identity.first().and_then(Value::as_str) == Some(path))
                        } else {
                            operation.payload.get("storagePath").and_then(Value::as_str) == Some(path)
                        }
                    })
            };
            let operationLog = self.operationLog(&deviceId)?;
            if !domainSet.is_empty() {
                let selected = {
                    let mut index = lockSyncState(&operationLog, "operation log")?;
                    self.loadOperationLogIndex(&deviceId, &mut index)?;
                    let missing = domainSet
                        .iter()
                        .filter(|domain| !index.domainOperations.contains_key(*domain))
                        .cloned()
                        .collect::<BTreeSet<_>>();
                    if !missing.is_empty() {
                        let content = self.readOperationLog(&deviceId)?;
                        let mut domains = missing
                            .iter()
                            .map(|domain| (domain.clone(), Vec::new()))
                            .collect::<BTreeMap<_, _>>();
                        let mut skipped = 0usize;
                        for line in content.lines().filter(|line| !line.trim().is_empty()) {
                            match serde_json::from_str::<SyncOperation>(line) {
                                Ok(operation) => {
                                    if let Some(operations) = domains.get_mut(&operation.domain) {
                                        operations.push(operation);
                                    }
                                }
                                Err(_) => skipped += 1,
                            }
                        }
                        if skipped > 0 {
                            operit_util::AppLogger::AppLogger::w(
                                "SyncOperationStore",
                                &format!(
                                    "skipped {skipped} corrupt operation-log line(s) for {deviceId}"
                                ),
                            );
                        }
                        index.domainOperations.extend(domains);
                    }
                    domainSet
                        .iter()
                        .flat_map(|domain| index.domainOperations[domain].iter())
                        .filter(|operation| isMissing(operation))
                        .filter(|operation| storedEncryptedSyncPayload(&operation.payload) || retain(operation))
                        .cloned()
                        .collect::<Vec<_>>()
                };
                for operation in selected {
                    out.push(self.decodeOperationPayload(operation)?);
                }
                continue;
            }
            let content = {
                let mut operationLog = lockSyncState(&operationLog, "operation log")?;
                self.loadOperationLogIndex(&deviceId, &mut operationLog)?;
                if operationLog.highestSequence <= businessMinimum {
                    String::new()
                } else {
                    let startOffset = operationLog
                        .sequenceOffsets
                        .range((businessMinimum + 1)..)
                        .map(|(_, offset)| *offset)
                        .min()
                        .ok_or_else(|| {
                            SyncOperationStoreError::Message(format!(
                                "sync operation log index is missing sequence after {businessMinimum} for {deviceId}"
                            ))
                        })?;
                    let byteLength = usize::try_from(
                        operationLog
                            .encodedByteLength
                            .checked_sub(startOffset)
                            .ok_or_else(|| {
                                SyncOperationStoreError::Message(format!(
                                    "sync operation log offset exceeds length for {deviceId}"
                                ))
                            })?,
                    )
                    .map_err(|_| {
                        SyncOperationStoreError::Message(
                            "sync operation log range length does not fit usize".to_string(),
                        )
                    })?;
                    self.readOperationLogRange(&deviceId, startOffset, byteLength)?
                }
            };
            out.extend(
                self.decodeOperationLog(&content)?
                    .into_iter()
                    .filter(isMissing),
            );
        }
        out.retain(retain);
        out.sort_by_key(SyncOperationOrder::fromOperation);
        let mut out = compactSyncOperations(out);
        out.truncate(limit);
        Ok(boundSyncOperationPage(out, maxBytes))
    }

    /// Serializes receipt and materialization so concurrent peers cannot replay one transaction.
    pub fn lockApplication(&self) -> Result<MutexGuard<'_, ()>, SyncOperationStoreError> {
        lockSyncState(&self.sharedState.application, "sync application")
    }

    /// Persists originals and exact receipt membership before acknowledging delivery.
    pub fn receiveOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
        let mut state = self
            .loadCachedJson(&mut cached, &self.receiveStatePath())?
            .clone();
        let mut changed = false;
        for operation in operations {
            if operation.originDeviceId.is_empty()
                || operation.originDeviceId.len() > 256
                || operation.sequence <= 0
                || operation.opId != format!("{}:{}", operation.originDeviceId, operation.sequence)
            {
                return Err(SyncOperationStoreError::Message(
                    "Invalid received operation identity".into(),
                ));
            }
            use sha2::{Digest, Sha256};
            let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(operation)?));
            if let Some(previous) = state.fingerprints.get(&operation.opId) {
                if previous != &fingerprint {
                    return Err(SyncOperationStoreError::Message(format!(
                        "Conflicting immutable operation {}",
                        operation.opId
                    )));
                }
            } else {
                state
                    .fingerprints
                    .insert(operation.opId.clone(), fingerprint);
                changed = true;
            }
            if let Some(existing) = state.pending.get(&operation.opId) {
                if self.decodeOperationPayload(existing.clone())? != *operation {
                    return Err(SyncOperationStoreError::Message(format!(
                        "Conflicting immutable operation {}",
                        operation.opId
                    )));
                }
                continue;
            }
            let journaled = self.containsIdenticalOperation(operation)?;
            if state
                .receipt
                .containsSequence(&operation.originDeviceId, operation.sequence)
            {
                continue;
            }
            if !journaled {
                state.pending.insert(
                    operation.opId.clone(),
                    self.encodeOperationPayload(operation)?,
                );
            }
            state
                .receipt
                .observe(&operation.originDeviceId, operation.sequence);
            changed = true;
        }
        if changed {
            self.writeJson(&self.receiveStatePath(), &state)?;
            cached.value = state;
        }
        drop(cached);
        if changed {
            publishSyncMutation();
        }
        Ok(())
    }

    /// Reads unapplied original operations after restarts or partial application.
    pub fn pendingOperations(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
        let mut operations = self
            .loadCachedJson(&mut cached, &self.receiveStatePath())?
            .pending
            .values()
            .cloned()
            .map(|operation| self.decodeOperationPayload(operation))
            .collect::<Result<Vec<_>, _>>()?;
        operations.sort_by(|left, right| {
            (&left.originDeviceId, left.sequence).cmp(&(&right.originDeviceId, right.sequence))
        });
        Ok(operations)
    }

    /// Clears completed replay intent while retaining its receipt independently of native logs.
    pub fn completeOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
        let mut state = self
            .loadCachedJson(&mut cached, &self.receiveStatePath())?
            .clone();
        let mut changed = false;
        for operation in operations {
            changed |= state.pending.remove(&operation.opId).is_some();
        }
        if changed {
            self.writeJson(&self.receiveStatePath(), &state)?;
            cached.value = state;
        }
        Ok(())
    }

    /// Combines retained receipts with exact journal sequences without acknowledging gaps.
    pub fn receipt(&self) -> Result<SyncReceipt, SyncOperationStoreError> {
        let mut receipt = {
            let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
            self.loadCachedJson(&mut cached, &self.receiveStatePath())?
                .receipt
                .clone()
        };
        for origin in self.devices()? {
            let log = self.operationLog(&origin)?;
            let mut log = lockSyncState(&log, "operation log")?;
            self.loadOperationLogIndex(&origin, &mut log)?;
            for sequence in log.sequenceOffsets.keys() {
                receipt.observe(&origin, *sequence);
            }
        }
        Ok(receipt)
    }

    /// Verifies immutable content before recognizing an already journaled operation.
    fn containsIdenticalOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<bool, SyncOperationStoreError> {
        let log = self.operationLog(&operation.originDeviceId)?;
        let mut log = lockSyncState(&log, "operation log")?;
        self.loadOperationLogIndex(&operation.originDeviceId, &mut log)?;
        let Some(offset) = log.sequenceOffsets.get(&operation.sequence).copied() else {
            return Ok(false);
        };
        let end = log
            .sequenceOffsets
            .values()
            .copied()
            .filter(|value| *value > offset)
            .min()
            .unwrap_or(log.encodedByteLength);
        let length = usize::try_from(end - offset)
            .map_err(|_| SyncOperationStoreError::Message("Operation length overflows usize".into()))?;
        let content = self.readOperationLogRange(&operation.originDeviceId, offset, length)?;
        let existing = self.decodeOperationPayload(serde_json::from_str(content.lines().next().unwrap_or("").trim())?)?;
        if existing != *operation {
            return Err(SyncOperationStoreError::Message(format!(
                "Conflicting immutable operation {}",
                operation.opId
            )));
        }
        Ok(true)
    }

    /// Locates durable receipts and replay intents beside the operation journal.
    fn receiveStatePath(&self) -> String {
        format!("{}/receive_state.json", self.rootPath)
    }

    /// Reads the local vector clock from storage.
    pub fn localClock(&self) -> Result<SyncClock, SyncOperationStoreError> {
        self.publishPreferenceOutboxes()?;
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
        let clock = self.localClock()?;
        for (origin, sequence) in &clock.sequences {
            if origin == &deviceId || origin.starts_with(&format!("{deviceId}:preferences:")) {
                self.setExportFloor(origin, *sequence)?;
            }
        }
        Ok(())
    }

    /// Sets the highest sequence excluded from future synchronization exports.
    #[allow(non_snake_case)]
    pub fn setExportFloor(
        &self,
        originDeviceId: &str,
        sequence: i64,
    ) -> Result<(), SyncOperationStoreError> {
        let mut state = lockSyncState(&self.sharedState.exportFloors, "export floors")?;
        let floors = self
            .loadCachedJson(&mut state, &self.exportFloorsPath())?
            .clone();
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

    /// Reports known entity state, including synchronized deletion tombstones.
    pub(crate) fn hasEntityVersion(&self, domain: &str, entityType: &str, entityId: &str) -> Result<bool, SyncOperationStoreError> {
        let mut state = lockSyncState(&self.sharedState.entityVersions, "entity versions")?;
        let key = serde_json::to_string(&(domain, entityType, entityId))?;
        Ok(self.loadCachedEntityVersions(&mut state)?.contains_key(&key))
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

    /// Acknowledges one received control operation on its own per-origin clock.
    ///
    /// Control operations are replayed into every peer but must not advance the
    /// origin's business sequence: a later authority command would otherwise
    /// hide an earlier chat or file operation that this device still needs.
    pub fn observeNetworkControlOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        if operation.domain != NETWORK_CONTROL_SYNC_DOMAIN {
            return Err(SyncOperationStoreError::Message(format!(
                "network-control acknowledgement requires a {NETWORK_CONTROL_SYNC_DOMAIN} operation"
            )));
        }
        let mut clockState = lockSyncState(&self.sharedState.clock, "clock")?;
        let mut clock = self
            .loadCachedJson(&mut clockState, &self.clockPath())?
            .clone();
        let key = networkControlClockKey(&operation.originDeviceId);
        if operation.sequence <= clock.sequenceFor(&key) {
            return Ok(());
        }
        clock.setSequence(key, operation.sequence);
        self.writeLocalClockValue(&clock)?;
        clockState.value = clock;
        Ok(())
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

    /// Reads one indexed byte range from an origin operation log.
    #[allow(non_snake_case)]
    fn readOperationLogRange(
        &self,
        deviceId: &str,
        offset: u64,
        length: usize,
    ) -> Result<String, SyncOperationStoreError> {
        let path = self.operationsPath(deviceId);
        if length == 0 {
            return Ok(String::new());
        }
        String::from_utf8(self.storageHost.readBytesRange(&path, offset, length)?)
            .map_err(|error| SyncOperationStoreError::Message(error.to_string()))
    }

    /// Decodes one immutable operation-log snapshot without holding its file lock.
    /// Corrupt lines are skipped and reported, not propagated as a whole-file failure.
    #[allow(non_snake_case)]
    fn decodeOperationLog(
        &self,
        content: &str,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let mut operations = Vec::new();
        let mut skipped = 0usize;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<SyncOperation>(trimmed) {
                Ok(operation) => match self.decodeOperationPayload(operation) {
                    Ok(operation) => operations.push(operation),
                    Err(error) => {
                        skipped += 1;
                        operit_util::AppLogger::AppLogger::w(
                            "SyncOperationStore",
                            &format!("skipped undecodable sync operation payload: {error}"),
                        );
                    }
                },
                Err(_) => skipped += 1,
            }
        }
        if skipped > 0 {
            operit_util::AppLogger::AppLogger::w(
                "SyncOperationStore",
                &format!("skipped {skipped} corrupt operation-log line(s) during decode"),
            );
        }
        Ok(operations)
    }

    /// Loads the persisted operation IDs once for one locked origin log.
    /// Corrupt lines are recorded as findings and skipped: one damaged line
    /// must not make every read of the journal fail.
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
        index.needsSeparator = !content.is_empty() && !content.ends_with('\n');
        let mut byteOffset = 0u64;
        let mut lineNumber = 0u64;
        for line in content.split_inclusive('\n') {
            lineNumber += 1;
            let trimmed = line.trim();
            let lineLength = u64::try_from(line.len()).map_err(|_| {
                SyncOperationStoreError::Message(
                    "sync operation line length does not fit u64".to_string(),
                )
            })?;
            if trimmed.is_empty() {
                byteOffset += lineLength;
                continue;
            }
            match serde_json::from_str::<SyncOperation>(trimmed) {
                Ok(operation) => {
                    index.highestSequence = index.highestSequence.max(operation.sequence);
                    index.sequenceOffsets.insert(operation.sequence, byteOffset);
                    index.operationIds.insert(operation.opId);
                }
                Err(error) => index.findings.push(OperationLogLineFinding {
                    deviceId: deviceId.to_string(),
                    lineNumber,
                    byteOffset,
                    byteLength: lineLength,
                    error: error.to_string(),
                }),
            }
            byteOffset += lineLength;
        }
        index.encodedByteLength = u64::try_from(content.len()).map_err(|_| {
            SyncOperationStoreError::Message(
                "sync operation log length does not fit u64".to_string(),
            )
        })?;
        if !index.findings.is_empty() {
            operit_util::AppLogger::AppLogger::w(
                "SyncOperationStore",
                &format!(
                    "origin {deviceId} journal carries {} corrupt line(s); reads skip them",
                    index.findings.len()
                ),
            );
        }
        index.loaded = true;
        Ok(())
    }

    /// Appends one encoded operation as a complete JSON line.
    #[allow(non_snake_case)]
    fn appendOperationLine(
        &self,
        operation: &SyncOperation,
        index: &mut SyncOperationLogIndex,
    ) -> Result<u64, SyncOperationStoreError> {
        let storedOperation = self.encodeOperationPayload(operation)?;
        let mut content = serde_json::to_vec(&storedOperation)?;
        content.push(b'\n');
        self.appendOperationBytes(&operation.originDeviceId, content, index)
    }

    /// Separates an interrupted tail from the next intact operation and reloads after uncertain writes.
    fn appendOperationBytes(
        &self,
        origin: &str,
        mut content: Vec<u8>,
        index: &mut SyncOperationLogIndex,
    ) -> Result<u64, SyncOperationStoreError> {
        if index.needsSeparator { content.insert(0, b'\n'); }
        let writtenBytes = u64::try_from(content.len()).map_err(|_| {
            SyncOperationStoreError::Message(
                "sync operation line length does not fit u64".to_string(),
            )
        })?;
        if let Err(error) = self.storageHost.appendBytes(&self.operationsPath(origin), &content) {
            *index = SyncOperationLogIndex::default();
            return Err(error.into());
        }
        index.needsSeparator = false;
        Ok(writtenBytes)
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
            .map_err(|error| SyncOperationStoreError::Message(format!(
                "Cannot decrypt preference history op={} entity={}: {error}",
                operation.opId, operation.entityId,
            )))?;
        let mut decodedOperation = operation;
        decodedOperation.payload = serde_json::from_slice(&decryptedPayload)?;
        Ok(decodedOperation)
    }

    /// Reads a snapshot of registered origin devices under its metadata lock.
    pub fn devices(&self) -> Result<Vec<String>, SyncOperationStoreError> {
        let mut devicesState = lockSyncState(&self.sharedState.devices, "devices")?;
        Ok(self
            .loadCachedJson(&mut devicesState, &self.devicesPath())?
            .clone())
    }

    /// Scans one origin journal raw, classifying every line as decodable or
    /// corruption. Diagnostics use this; it never fails on damaged content,
    /// only on host storage errors. Payload envelopes are not decrypted here.
    #[allow(non_snake_case)]
    pub fn scanOperationLog(
        &self,
        deviceId: &str,
    ) -> Result<OperationLogScan, SyncOperationStoreError> {
        let mut scan = OperationLogScan {
            deviceId: deviceId.to_string(),
            ..Default::default()
        };
        let content = self.readOperationLog(deviceId)?;
        if content.is_empty() {
            return Ok(scan);
        }
        scan.exists = true;
        let mut byteOffset = 0u64;
        let mut lineNumber = 0u64;
        for line in content.split_inclusive('\n') {
            lineNumber += 1;
            let lineLength = u64::try_from(line.len()).map_err(|_| {
                SyncOperationStoreError::Message(
                    "sync operation line length does not fit u64".to_string(),
                )
            })?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                match serde_json::from_str::<SyncOperation>(trimmed) {
                    Ok(operation) => {
                        scan.decodedLines += 1;
                        scan.highestSequence = scan.highestSequence.max(operation.sequence);
                        scan.operations.push(operation);
                    }
                    Err(error) => scan.findings.push(OperationLogLineFinding {
                        deviceId: deviceId.to_string(),
                        lineNumber,
                        byteOffset,
                        byteLength: lineLength,
                        error: error.to_string(),
                    }),
                }
            }
            scan.totalLines += 1;
            byteOffset += lineLength;
        }
        Ok(scan)
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
        self.storageHost.writeBytesAtomically(path, &content)?;
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

    /// Returns the JSONL operation-log path for one origin device. Public for
    /// diagnostics that copy or back up the raw journal beside the store APIs.
    #[allow(non_snake_case)]
    pub fn operationsPath(&self, deviceId: &str) -> String {
        format!(
            "{}/operations/{}.jsonl",
            self.rootPath,
            storageSafeId(deviceId)
        )
    }

    /// Returns the reversible quarantine sidecar path for one origin device.
    #[allow(non_snake_case)]
    fn quarantinePath(&self, deviceId: &str) -> String {
        format!(
            "{}/operations/{}.jsonl.quarantine",
            self.rootPath,
            storageSafeId(deviceId)
        )
    }

    /// Moves the journal lines at the given byte offsets out of one origin
    /// log into a reversible quarantine sidecar, holding the same per-origin
    /// lock as appends, then invalidates every cached view of the journal.
    /// Nothing is deleted: each moved line is preserved verbatim with its
    /// original offset, a stable hash and a timestamp. Offsets must come from
    /// scanOperationLog findings; the byte accounting is identical.
    #[allow(non_snake_case)]
    pub fn quarantineOperationLines(
        &self,
        deviceId: &str,
        byteOffsets: &[u64],
    ) -> Result<Vec<QuarantinedOperationLine>, SyncOperationStoreError> {
        let operationLog = self.operationLog(deviceId)?;
        let mut index = lockSyncState(&operationLog, "operation log")?;
        let content = self.readOperationLog(deviceId)?;
        let targets: BTreeSet<u64> = byteOffsets.iter().copied().collect();
        let mut kept = String::new();
        let mut moved = Vec::new();
        let mut byteOffset = 0u64;
        let now = tryCurrentTimeMillis().map_err(SyncOperationStoreError::Message)?;
        for line in content.split_inclusive('\n') {
            let lineLength = u64::try_from(line.len()).map_err(|_| {
                SyncOperationStoreError::Message(
                    "sync operation line length does not fit u64".to_string(),
                )
            })?;
            if targets.contains(&byteOffset) {
                let mut hasher = DefaultHasher::new();
                line.hash(&mut hasher);
                moved.push(QuarantinedOperationLine {
                    deviceId: deviceId.to_string(),
                    byteOffset,
                    byteLength: lineLength,
                    lineHash: format!("{:016x}", hasher.finish()),
                    quarantinedAt: now,
                    line: line.trim_end_matches('\n').to_string(),
                });
            } else {
                kept.push_str(line);
            }
            byteOffset += lineLength;
        }
        if moved.is_empty() {
            return Ok(moved);
        }
        let sidecarPath = self.quarantinePath(deviceId);
        let mut sidecar = if self.storageHost.exists(&sidecarPath)? {
            String::from_utf8(self.storageHost.readBytes(&sidecarPath)?)
                .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?
        } else {
            String::new()
        };
        for entry in &moved {
            let encoded = serde_json::to_vec(entry)?;
            sidecar.push_str(
                std::str::from_utf8(&encoded)
                    .map_err(|error| SyncOperationStoreError::Message(error.to_string()))?,
            );
            sidecar.push('\n');
        }
        self.storageHost
            .writeBytes(&self.operationsPath(deviceId), kept.as_bytes())?;
        self.storageHost
            .writeBytes(&sidecarPath, sidecar.as_bytes())?;
        index.domainOperations.clear();
        index.operationIds.clear();
        index.sequenceOffsets.clear();
        index.highestSequence = 0;
        index.encodedByteLength = 0;
        index.findings.clear();
        index.loaded = false;
        publishSyncMutation();
        Ok(moved)
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
        && matches!(operation.operation.as_str(), "set" | "transaction")
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

/// Identifies one independently compactable state within its origin's operation log.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_snake_case)]
struct SyncEntityKey {
    originDeviceId: String,
    domain: String,
    entityType: String,
    entityId: String,
}

/// Cuts one ordered operation page at a serialized-byte budget.
///
/// Callers pass an already compacted, count-bounded page; this bounds what one
/// store contributes to a merged synchronization page. The first operation is
/// always kept so one oversized record cannot stall the export.
pub fn boundSyncOperationPage(    operations: Vec<SyncOperation>,
    maxBytes: usize,
) -> Vec<SyncOperation> {
    let mut page = Vec::new();
    let mut bytes = 0usize;
    for operation in operations {
        let size = serde_json::to_vec(&operation)
            .map(|encoded| encoded.len())
            .unwrap_or(0);
        if !page.is_empty() && bytes.saturating_add(size) > maxBytes {
            break;
        }
        bytes = bytes.saturating_add(size);
        page.push(operation);
    }
    page
}

/// Preserves preference history for identity replay and compacts other replaceable entity states.
#[allow(non_snake_case)]
pub fn compactSyncOperations(operations: Vec<SyncOperation>) -> Vec<SyncOperation> {
    let mut latestSequences = BTreeMap::<SyncEntityKey, i64>::new();
    for operation in &operations {
        if operation.semantics == SyncOperationSemantics::EntityState {
            latestSequences
                .entry(syncEntityKey(operation))
                .and_modify(|sequence| *sequence = (*sequence).max(operation.sequence))
                .or_insert(operation.sequence);
        }
    }
    operations
        .into_iter()
        .filter(|operation| {
            operation.domain == "preferences"
                || operation.semantics == SyncOperationSemantics::Transaction
                || latestSequences.get(&syncEntityKey(operation)) == Some(&operation.sequence)
        })
        .collect()
}

/// Builds the origin-scoped identity used for operation-log compaction.
#[allow(non_snake_case)]
fn syncEntityKey(operation: &SyncOperation) -> SyncEntityKey {
    SyncEntityKey {
        originDeviceId: operation.originDeviceId.clone(),
        domain: operation.domain.clone(),
        entityType: operation.entityType.clone(),
        entityId: operation.entityId.clone(),
    }
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

/// Builds the per-origin clock key that acknowledges network-control operations.
#[allow(non_snake_case)]
fn networkControlClockKey(originDeviceId: &str) -> String {
    format!("{originDeviceId}:control")
}

#[cfg(test)]
#[path = "tests/SyncOperationStoreTests.rs"]
mod SyncOperationStoreTests;
