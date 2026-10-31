use operit_host_api::AtomicCounter::AtomicU64;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};
use std::io::{BufReader, Write};
use crate::LegacyTransactionImport::{JournalLines, JournalReader, JournalRecord, JOURNAL_READ_BYTES};

use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::tryCurrentTimeMillis;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_PREFERENCES_PAYLOADS_DIR_PATH;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::NetworkControlStore::NETWORK_CONTROL_SYNC_DOMAIN;
use crate::NetworkControlReadModel::{
    beginControlWrite, lockControlReadModel, lockControlReadModels,
    NetworkControlReadModel, SharedControlReadModel,
};
use crate::PreferencesEncryption::PreferencesEncryption;
use crate::RuntimeStorageHost::{defaultRuntimeStorageHost, runtimeStoragePath};
use crate::RuntimeStorePaths::RuntimeStorePaths;

#[derive(Debug, Error)]
pub enum SyncOperationStoreError {
    #[error("host error: {0}")]
    Host(#[from] operit_host_api::HostError),
    #[error("journal I/O error: {0}")]
    Io(#[from] std::io::Error),
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

    /// Keeps only sequences this device authored. Applied peer operations also
    /// advance their origins here, but they are not work this device still
    /// owes its peers, so change summaries exclude them.
    pub fn retainAuthoredBy(&mut self, deviceId: &str) {
        self.sequences
            .retain(|origin, _| origin.starts_with(deviceId));
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

/// Verifies the same immutable envelope in every domain; migrations derive separate views.
pub fn verifySyncOperation(existing: &SyncOperation, incoming: &SyncOperation) -> Result<(), SyncOperationStoreError> {
    crate::Transaction::verify_original(&incoming.opId, existing, incoming).map_err(SyncOperationStoreError::Message)
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
                    .unwrap_or(operation.createdAt),
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
    controlRevision: AtomicU64,
    clock: Mutex<SyncOperationCachedValue<SyncClock>>,
    journal: Mutex<Option<crate::Transaction::Journal>>,
    legacyFiles: Mutex<()>,
    entityVersions: Mutex<SyncOperationCachedValue<BTreeMap<String, SyncOperationOrder>>>,
    exportFloors: Mutex<SyncOperationCachedValue<BTreeMap<String, i64>>>,
    localDeviceId: Mutex<SyncOperationCachedValue<Option<String>>>,
    application: Mutex<()>,
    preferenceOutboxes: Mutex<SyncOperationCachedValue<BTreeMap<String, bool>>>,
}

/// Invalidates readers before and after a journal write, including uncertain failures.
struct ControlJournalMutation<'a>(&'a AtomicU64);

impl<'a> ControlJournalMutation<'a> {
    /// Opens a publication interval while the caller holds the journal mutation lock.
    fn new(revision: &'a AtomicU64) -> Self {
        revision.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        Self(revision)
    }
}

impl Drop for ControlJournalMutation<'_> {
    /// Invalidates snapshots observed before the publication interval completed.
    fn drop(&mut self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Release);
    }
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

/// An internal read intent; replay never acquires export publication side effects.
enum JournalProjection {
    All,
    PreferenceDatasetHistory,
    MergeMetadata,
}

impl JournalProjection {
    fn includes(&self, record: &JournalRecord) -> bool {
        match self {
            Self::All => true,
            Self::PreferenceDatasetHistory => record.domain == "preferences",
            Self::MergeMetadata => record.is_merge_candidate(),
        }
    }
}

struct JournalQuery<'a> {
    domains: &'a [String],
    projection: JournalProjection,
    export_floors: bool,
    limit: usize,
    max_bytes: usize,
    retain: Option<&'a dyn Fn(&SyncOperation) -> bool>,
}

/// Measures wire size without allocating a second serialized payload buffer.
#[derive(Default)]
struct SerializedByteCount(usize);

impl Write for SerializedByteCount {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct SyncOperationStoreRegistryKey {
    storageHost: Weak<dyn RuntimeStorageHost>,
    rootPath: String,
}

impl PartialEq for SyncOperationStoreRegistryKey {
    /// Compares registry keys by storage-host identity and synchronization root.
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.storageHost, &other.storageHost) && self.rootPath == other.rootPath
    }
}

impl Eq for SyncOperationStoreRegistryKey {}

impl Hash for SyncOperationStoreRegistryKey {
    /// Hashes registry keys by storage-host identity and synchronization root.
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.storageHost.as_ptr() as *const () as usize).hash(state);
        self.rootPath.hash(state);
    }
}

impl SyncOperationStore {
    pub fn devices(&self) -> Result<Vec<String>, SyncOperationStoreError> {
        Ok(self.withJournal(|journal| journal.headers(None))?.into_iter().map(|record| record.origin_device_id)
            .collect::<BTreeSet<_>>().into_iter().collect())
    }

    /// Identifies committed or interrupted changes to the authority journal.
    pub(crate) fn controlRevision(&self) -> u64 {
        self.sharedState.controlRevision.load(std::sync::atomic::Ordering::Acquire)
    }

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

    /// Loads authority once for this host/root. Normal queries use the resulting
    /// state, and every control mutation below maintains it before returning.
    pub(crate) fn networkControlReadModel(
        &self,
    ) -> Result<SharedControlReadModel, SyncOperationStoreError> {
        let mut registry = lockControlReadModels()?;
        if let Some(model) = registry.get(&self.storageHost, &self.rootPath) {
            let projection = crate::NetworkControlStore::NetworkControlStore::projectionStore(
                self.storageHost.clone(),
            );
            let mut state = lockControlReadModel(&model)?;
            if state.projection_revision != projection.revision() {
                let projected = Self::projectedControlOperations(&projection)
                    .map_err(SyncOperationStoreError::Message)?;
                state.replace_projected(projected);
                state.projection_revision = projection.revision();
            }
            drop(state);
            return Ok(model);
        }
        let model = Arc::new(Mutex::new(self.loadNetworkControlReadModel()?));
        registry.insert(&self.storageHost, &self.rootPath, model.clone());
        Ok(model)
    }

    fn loadNetworkControlReadModel(
        &self,
    ) -> Result<NetworkControlReadModel, SyncOperationStoreError> {
        let journal = self.readJournal(
            &SyncClock::empty(),
            JournalQuery {
                domains: &[NETWORK_CONTROL_SYNC_DOMAIN.into()],
                projection: JournalProjection::All,
                export_floors: false,
                limit: usize::MAX,
                max_bytes: usize::MAX,
                retain: None,
            },
        )?;
        let mut received = Vec::new();
        self.visitPendingOperations(&[NETWORK_CONTROL_SYNC_DOMAIN], |operation| received.push(operation.clone()))?;
        let projected = Self::projectedControlOperations(
            &crate::NetworkControlStore::NetworkControlStore::projectionStore(
                self.storageHost.clone(),
            ),
        )
        .map_err(SyncOperationStoreError::Message)?;
        let mut model = NetworkControlReadModel::new(journal, received, projected);
        model.projection_revision = crate::NetworkControlStore::NetworkControlStore::projectionStore(
            self.storageHost.clone(),
        ).revision();
        Ok(model)
    }

    /// Explicit recovery after restoring authority files outside the normal write
    /// APIs. Rebuilds existing readers too; ordinary queries never call this.
    pub fn reloadNetworkControlState(&self) -> Result<(), SyncOperationStoreError> {
        let mut registry = lockControlReadModels()?;
        let Some(model) = registry.get(&self.storageHost, &self.rootPath) else {
            return Ok(());
        };
        lockControlReadModel(&model)?.unavailable("Control state is being recovered".into());
        match self.loadNetworkControlReadModel() {
            Ok(rebuilt) => *lockControlReadModel(&model)? = rebuilt,
            Err(error) => {
                lockControlReadModel(&model)?.unavailable(error.to_string());
                return Err(error);
            }
        }
        Ok(())
    }

    fn projectedControlOperations(
        store: &crate::PreferencesDataStore::CoreNodeStateStore,
    ) -> Result<Vec<SyncOperation>, String> {
        // Keep outer projection values encoded on constrained Hosts. Building
        // a generic Preferences tree here duplicates every nested authority
        // payload and exhausts the device's fixed approval worker stack.
        store.withEncodedEntries(|entries| {
            let mut operations = BTreeMap::<String, SyncOperation>::new();
            for value in entries.values() {
                let operation: SyncOperation = serde_json::from_str(value.get())
                    .map_err(|error| error.to_string())?;
                if let Some(previous) = operations.insert(operation.opId.clone(), operation.clone()) {
                    if previous != operation {
                        return Err("Conflicting immutable Space control operation".into());
                    }
                }
            }
            Ok(operations.into_values().collect())
        }).map_err(|error| error.to_string())?
    }

    /// Called by the durable endpoint/import write path, never by state reads.
    pub(crate) fn replaceProjectedControlOperations(
        &self,
        store: &crate::PreferencesDataStore::CoreNodeStateStore,
        retire_journal: bool,
    ) -> Result<(), String> {
        let mut registry = lockControlReadModels().map_err(|error| error.to_string())?;
        if let Some(model) = registry.get(&self.storageHost, &self.rootPath) {
            let projected = Self::projectedControlOperations(store)?;
            let mut model = lockControlReadModel(&model).map_err(|error| error.to_string())?;
            model.replace_projected(projected);
            model.projection_revision = store.revision();
            if retire_journal {
                model.retire_journal();
            }
        }
        Ok(())
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
        self.withJournal(|journal| journal.register_file(path, encrypted))
    }

    /// Publishes only immutable operations whose owning business snapshot has already committed.
    pub fn publishPreferenceOutboxes(&self) -> Result<Vec<String>, SyncOperationStoreError> {
        let registry = self.preferenceDatasets()?;
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

    /// Completes metadata after an acknowledged log append but failed later
    /// writes. Never allocates a second sequence or appends a duplicate line.
    #[allow(non_snake_case)]
    pub fn recoverAppendedOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<(), SyncOperationStoreError> {
        self.appendOperation(operation)?;
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

    /// Returns missing operations in the shared deterministic journal order.
    pub fn operationsSince<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        domains: &[String],
        limit: usize,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        // Preference outboxes can only contribute to the preferences domain.
        // Policy and other domain reads must not open unrelated encrypted stores.
        if domains.is_empty() || domains.iter().any(|domain| domain == "preferences") {
            self.publishPreferenceOutboxes()?;
        }
        self.readJournal(
            clock,
            JournalQuery {
                domains,
                projection: JournalProjection::All,
                export_floors: true,
                limit,
                max_bytes: usize::MAX,
                retain: None,
            },
        )
    }

    /// Filters before compaction and bounds the combined archived/pending page.
    /// A single oversized first record is retained so the transport can report it.
    pub fn operationsSinceWithBudget<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        domains: &[String],
        limit: usize,
        maxBytes: usize,
        retain: impl Fn(&SyncOperation) -> bool,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        // Preference outboxes can only contribute to the preferences domain.
        // Policy and other domain reads must not open unrelated encrypted stores.
        if domains.is_empty() || domains.iter().any(|domain| domain == "preferences") {
            self.publishPreferenceOutboxes()?;
        }
        self.readJournal(
            clock,
            JournalQuery {
                domains,
                projection: JournalProjection::All,
                export_floors: true,
                limit,
                max_bytes: maxBytes,
                retain: Some(&retain),
            },
        )
    }

    /// Plans the normal bounded export and optionally inventories its compacted missing identities.
    pub fn operationsSinceWithInventory<K: SyncOperationKnowledge>(
        &self, clock: &K, domains: &[String], limit: usize, maxBytes: usize,
        inventory: Option<&mut BTreeSet<String>>, retain: impl Fn(&SyncOperation) -> bool,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        if domains.is_empty() || domains.iter().any(|domain| domain == "preferences") {
            self.publishPreferenceOutboxes()?;
        }
        self.readJournalWithInventory(clock, JournalQuery {
            domains, projection: JournalProjection::All, export_floors: true,
            limit, max_bytes: maxBytes, retain: Some(&retain),
        }, inventory)
    }

    /// Permission replay reads committed authority only. It must not publish
    /// unrelated preference outboxes while checking a pairing or routing a Call.
    /// Preserve the existing control export floors and compaction semantics.
    pub(crate) fn archivedControlOperations(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.readJournal(
            &SyncClock::empty(),
            JournalQuery {
                domains: &[NETWORK_CONTROL_SYNC_DOMAIN.to_owned()],
                projection: JournalProjection::All,
                export_floors: true,
                limit: usize::MAX,
                max_bytes: usize::MAX,
                retain: None,
            },
        )
    }

    /// Explicit exhaustive history for provenance consumers; never retained in a cache.
    pub fn preferenceHistory(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.publishPreferenceOutboxes()?;
        self.readJournal(
            &SyncClock::empty(),
            JournalQuery {
                domains: &[],
                projection: JournalProjection::PreferenceDatasetHistory,
                export_floors: false,
                limit: usize::MAX,
                max_bytes: usize::MAX,
                retain: None,
            },
        )
    }

    /// Loads only cross-dataset declarations and identity decisions, not business history.
    pub fn preferenceMergeHistory(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.publishPreferenceOutboxes()?;
        self.readJournal(
            &SyncClock::empty(),
            JournalQuery {
                domains: &[],
                projection: JournalProjection::MergeMetadata,
                export_floors: false,
                limit: usize::MAX,
                max_bytes: usize::MAX,
                retain: None,
            },
        )
    }

    /// Reads one owner's archived originals without exporting or publishing another dataset.
    pub(crate) fn archivedPreferenceHistory(
        &self,
        path: &str,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let routes = crate::PreferenceLayout::routes(self.storageHost.clone()).map_err(SyncOperationStoreError::Message)?;
        let paths = std::iter::once(path).chain(routes.iter().filter(|(_, target)| target.as_str() == path).map(|(source, _)| source.as_str()));
        let mut history = Vec::new();
        for source in paths {
            let stored = self.withJournal(|journal| journal.preference_originals(source))?;
            history.extend(stored.into_iter().map(|operation| self.decodeOperationPayload(operation))
                .collect::<Result<Vec<_>, _>>()?);
        }
        Ok(history)
    }

    pub(crate) fn preferenceCompactionHistory(&self, path: &str, format: u32) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.withJournal(|journal| journal.preference_compaction_history(path, format))?.into_iter()
            .map(|operation| self.decodeOperationPayload(operation)).collect()
    }

    /// Lists registered owners, including datasets prepared by plugins while their UI is closed.
    pub fn preferenceDatasets(&self) -> Result<BTreeMap<String, bool>, SyncOperationStoreError> {
        self.withJournal(|journal| journal.files())
    }

    /// Plans using locations and ordering keys, then opens only the selected page.
    /// All origin guards are acquired in device order. This keeps indexed locations stable
    /// across append/repair and never holds a business dataset lock during export publication.
    fn readJournal<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        query: JournalQuery<'_>,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.readJournalWithInventory(clock, query, None)
    }

    fn readRecordAt(
        &self,
        path: &str,
        record: &JournalRecord,
    ) -> Result<SyncOperation, SyncOperationStoreError> {
        let reader = BufReader::with_capacity(
            JOURNAL_READ_BYTES,
            JournalReader::new(
                self.storageHost.as_ref(),
                path,
                record.offset,
                record.length,
            ),
        );
        self.decodeOperationPayload(serde_json::from_reader(reader)?)
    }

    /// Serializes receipt and materialization so concurrent peers cannot replay one transaction.
    pub fn lockApplication(&self) -> Result<MutexGuard<'_, ()>, SyncOperationStoreError> {
        lockSyncState(&self.sharedState.application, "sync application")
    }



    /// Reads the local vector clock from storage.
    pub fn localClock(&self) -> Result<SyncClock, SyncOperationStoreError> {
        self.publishPreferenceOutboxes()?;
        self.currentLocalClock()
    }

    /// Reads the cached local vector clock without publishing deferred
    /// preference payloads. Change probes poll this instead of the full
    /// receipt so an idle peer performs no writes and serializes no journal.
    #[allow(non_snake_case)]
    pub fn currentLocalClock(&self) -> Result<SyncClock, SyncOperationStoreError> {
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
            if origin == &deviceId || (origin == &format!("{deviceId}:preferences") || origin.starts_with(&format!("{deviceId}:preferences:"))) {
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
        let _revision = ControlJournalMutation::new(&self.sharedState.controlRevision);
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
        let path = self.operationsPath(deviceId);
        if !self.storageHost.exists(&path)? { return Ok(scan); }
        scan.exists = true;
        let _guard = lockSyncState(&self.sharedState.legacyFiles, "legacy journal diagnostics")?;
        let mut lines = JournalLines::new(self.storageHost.as_ref(), &path);
        while lines.next()? {
            let line = std::str::from_utf8(&lines.bytes).map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
            let byteOffset = lines.offset;
            let lineNumber = lines.line_number;
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
        }
        Ok(scan)
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
        let controlWrite = beginControlWrite(&self.storageHost, &self.rootPath, true)?;
        let legacy = lockSyncState(&self.sharedState.legacyFiles, "legacy journal diagnostics")?;
        let path = self.operationsPath(deviceId);
        if !self.storageHost.exists(&path)? { return Ok(Vec::new()); }
        let targets: BTreeSet<u64> = byteOffsets.iter().copied().collect();
        let mut kept = String::new();
        let mut moved = Vec::new();
        let now = tryCurrentTimeMillis().map_err(SyncOperationStoreError::Message)?;
        let mut lines = JournalLines::new(self.storageHost.as_ref(), &path);
        while lines.next()? {
            let line = std::str::from_utf8(&lines.bytes).map_err(|error| SyncOperationStoreError::Message(error.to_string()))?;
            let byteOffset = lines.offset;
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
        let _revision = ControlJournalMutation::new(&self.sharedState.controlRevision);
        self.storageHost
            .writeBytes(&self.operationsPath(deviceId), kept.as_bytes())?;
        drop(legacy);
        self.storageHost
            .writeBytes(&sidecarPath, sidecar.as_bytes())?;
        drop(controlWrite);
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
        Mutex<HashMap<SyncOperationStoreRegistryKey, Weak<SyncOperationStoreSharedState>>>,
    > = OnceLock::new();
    let states = SHARED_STATES.get_or_init(|| Mutex::new(HashMap::new()));
    let key = SyncOperationStoreRegistryKey {
        storageHost: Arc::downgrade(storageHost),
        rootPath: rootPath.to_string(),
    };
    let mut states = states
        .lock()
        .expect("SyncOperationStore shared state registry mutex must not be poisoned");
    states.retain(|_, state| state.strong_count() > 0);
    if let Some(state) = states.get(&key).and_then(Weak::upgrade) { return state; }
    let state = Arc::new(SyncOperationStoreSharedState::default());
    states.insert(key, Arc::downgrade(&state));
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
        let mut measured = SerializedByteCount::default();
        let _ = serde_json::to_writer(&mut measured, &operation);
        let size = measured.0;
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

impl SyncOperationStore {
    pub(crate) fn withJournal<T>(&self, action: impl FnOnce(&mut crate::Transaction::Journal) -> Result<T, SyncOperationStoreError>) -> Result<T, SyncOperationStoreError> {
        let mut state = lockSyncState(&self.sharedState.journal, "transaction journal")?;
        if state.is_none() {
            let mut journal = crate::Transaction::Journal::open(self.storageHost.as_ref(), &self.rootPath)?;
            self.importLegacyJournal(&mut journal)?;
            journal.reclaim_sql_copies()?;
            *state = Some(journal);
        }
        action(state.as_mut().expect("journal opened"))
    }

    pub(crate) fn hasTransactionJournal(&self) -> Result<bool, SyncOperationStoreError> {
        Ok(lockSyncState(&self.sharedState.journal, "transaction journal")?.is_some()
            || self.storageHost.exists(&format!("{}/transactions.sqlite", self.rootPath))?)
    }

    pub(crate) fn retirementOperations(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.withJournal(|journal| journal.headers(None))?.iter().map(|record| self.original(&record.op_id)).collect()
    }

    pub(crate) fn retireTransactionJournal(&self) -> Result<(), SyncOperationStoreError> {
        self.withJournal(|journal| journal.transaction(|tx| {
            for table in ["originals", "legacy_receipts", "transaction_owners", "transaction_files", "transaction_commits"] {
                tx.execute(&format!("DELETE FROM {table}"), vec![])?;
            }
            Ok(())
        }))
    }

    /// A single atomic import is the only reader of the retired file ledger.
    fn importLegacyJournal(&self, journal: &mut crate::Transaction::Journal) -> Result<(), SyncOperationStoreError> {
        use crate::Transaction::{Journal, LegacyReceiveState, text};
        use sha2::{Digest, Sha256};
        if journal.imported()? { return Ok(()); }
        let _legacy = lockSyncState(&self.sharedState.legacyFiles, "legacy import")?;
        let receivePath = format!("{}/receive_state.json", self.rootPath);
        let received: LegacyReceiveState = if self.storageHost.exists(&receivePath)? {
            serde_json::from_slice(&self.storageHost.readBytes(&receivePath)?)?
        } else { LegacyReceiveState::default() };
        if received.format_version.is_some_and(|version| version != 2) {
            return Err(SyncOperationStoreError::Message("Unsupported legacy receive format".into()));
        }
        let registry_path = format!("{}/preference_outboxes.json",self.rootPath);
        let files: BTreeMap<String,bool> = if self.storageHost.exists(&registry_path)? {
            serde_json::from_slice(&self.storageHost.readBytes(&registry_path)?)?
        } else { BTreeMap::new() };
        for (path, encrypted) in files { journal.register_file(&path, encrypted)?; }
        let devices: Vec<String> = self.readJson(&self.devicesPath())?;
        journal.transaction(|tx| {
            Journal::import_receipts(tx, &received)?;
            for device in devices {
                let path = self.operationsPath(&device);
                if !self.storageHost.exists(&path)? { continue; }
                let mut lines = JournalLines::new(self.storageHost.as_ref(), &path);
                while lines.next()? {
                    if lines.bytes.iter().all(u8::is_ascii_whitespace) { continue; }
                    let stored: SyncOperation = serde_json::from_slice(&lines.bytes)?;
                    let original = self.decodeOperationPayload(stored.clone())?;
                    Journal::put(tx, &original, &stored, false)?;
                }
            }
            for (id, raw) in &received.pending {
                let pending: Value = serde_json::from_str(raw.get())?;
                let stored: SyncOperation = if pending.get("record").is_some() {
                    let record: JournalRecord = serde_json::from_value(pending["record"].clone())?;
                    let key = pending.get("revision").and_then(Value::as_str)
                        .map(|revision| format!("{id}:{revision}")).unwrap_or_else(|| id.clone());
                    let path = format!("{}/pending/{:x}.json", self.rootPath, Sha256::digest(key.as_bytes()));
                    self.readRecordAt(&path, &record)?
                } else { serde_json::from_value(pending.clone())? };
                let original = self.decodeOperationPayload(stored)?;
                if original.opId != *id { return Err(SyncOperationStoreError::Message("Invalid legacy pending identity".into())); }
                let stored = self.encodeOperationPayload(&original)?;
                Journal::put(tx, &original, &stored, true)?;
                tx.execute("UPDATE originals SET pending=1 WHERE id=?1", vec![text(id)])?;
            }
            Journal::finish_import(tx)
        })
    }

    fn original(&self, id: &str) -> Result<SyncOperation, SyncOperationStoreError> {
        let stored = self.withJournal(|journal| journal.get(id))?
            .ok_or_else(|| SyncOperationStoreError::Message(format!("Missing immutable original {id}")))?;
        self.decodeOperationPayload(stored)
    }

    fn storeOriginals(&self, operations: &[SyncOperation], pending: bool) -> Result<(), SyncOperationStoreError> {
        let stored = operations.iter().map(|operation| self.encodeOperationPayload(operation)).collect::<Result<Vec<_>, _>>()?;
        self.withJournal(|journal| journal.transaction(|tx| {
            for (operation, stored) in operations.iter().zip(&stored) {
                crate::Transaction::Journal::put(tx, operation, stored, pending)?;
            }
            Ok(())
        }))
    }

    pub fn appendLocalOperation(&self, originDeviceId: &str, operation: NewSyncOperation) -> Result<SyncOperation, SyncOperationStoreError> {
        let control = beginControlWrite(&self.storageHost, &self.rootPath, operation.domain == NETWORK_CONTROL_SYNC_DOMAIN)?;
        let _revision = control.as_ref().map(|_| ControlJournalMutation::new(&self.sharedState.controlRevision));
        let mut cached = lockSyncState(&self.sharedState.clock, "clock")?;
        let mut clock = self.loadCachedJson(&mut cached, &self.clockPath())?.clone();
        let op = self.withJournal(|journal| {
            let sequence = clock.sequenceFor(originDeviceId).max(journal.highest(originDeviceId)?)
                .checked_add(1).ok_or_else(|| SyncOperationStoreError::Message("Transaction sequence overflow".into()))?;
            let op = SyncOperation {
                opId: format!("{originDeviceId}:{sequence}"), originDeviceId: originDeviceId.into(), sequence,
                domain: operation.domain, entityType: operation.entityType, entityId: operation.entityId,
                operation: operation.operation, semantics: operation.semantics, payload: operation.payload,
                createdAt: currentTimeMillis()?, schemaVersion: 1,
            };
            let stored = self.encodeOperationPayload(&op)?;
            journal.transaction(|tx| crate::Transaction::Journal::put(tx, &op, &stored, false))?;
            Ok(op)
        })?;
        if let Some(control) = &control { control.update(|model| model.append_journal([&op]))?; }
        clock.setSequence(originDeviceId, op.sequence);
        self.writeLocalClockValue(&clock)?;
        cached.value = clock;
        drop(cached);
        self.recordAppliedOperation(&op)?;
        drop(control);
        publishSyncMutation();
        Ok(op)
    }

    pub fn appendOperations(&self, operations: &[SyncOperation]) -> Result<(), SyncOperationStoreError> {
        if operations.is_empty() { return Ok(()); }
        let control = beginControlWrite(&self.storageHost, &self.rootPath, operations.iter().any(|op| op.domain == NETWORK_CONTROL_SYNC_DOMAIN))?;
        let _revision = control.as_ref().map(|_| ControlJournalMutation::new(&self.sharedState.controlRevision));
        let mut cached = lockSyncState(&self.sharedState.clock, "clock")?;
        let mut clock = self.loadCachedJson(&mut cached, &self.clockPath())?.clone();
        self.storeOriginals(operations, false)?;
        if let Some(control) = &control { control.update(|model| model.append_journal(operations))?; }
        for op in operations {
            if op.sequence > clock.sequenceFor(&op.originDeviceId) { clock.setSequence(&op.originDeviceId, op.sequence); }
        }
        self.writeLocalClockValue(&clock)?;
        cached.value = clock;
        drop(cached);
        drop(control);
        publishSyncMutation();
        Ok(())
    }

    pub fn appendUnobservedOperation(&self, operation: &SyncOperation) -> Result<(), SyncOperationStoreError> {
        let control = beginControlWrite(&self.storageHost, &self.rootPath, operation.domain == NETWORK_CONTROL_SYNC_DOMAIN)?;
        let _revision = control.as_ref().map(|_| ControlJournalMutation::new(&self.sharedState.controlRevision));
        self.storeOriginals(std::slice::from_ref(operation), false)?;
        if let Some(control) = &control { control.update(|model| model.append_journal([operation]))?; }
        drop(control);
        publishSyncMutation();
        Ok(())
    }

    pub(crate) fn flushOperations(&self, _: &[SyncOperation]) -> Result<(), SyncOperationStoreError> {
        // Every journal write has already completed a synchronous SQL commit.
        self.withJournal(|_| Ok(()))
    }

    pub fn receiveOperations(&self, operations: &[SyncOperation]) -> Result<(), SyncOperationStoreError> {
        if operations.is_empty() { return Ok(()); }
        let control = beginControlWrite(&self.storageHost, &self.rootPath, operations.iter().any(|op| op.domain == NETWORK_CONTROL_SYNC_DOMAIN))?;
        let _revision = control.as_ref().map(|_| ControlJournalMutation::new(&self.sharedState.controlRevision));
        self.storeOriginals(operations, true)?;
        if let Some(control) = &control {
            let pending = self.pendingOperationIds()?.into_iter().collect::<BTreeSet<_>>();
            control.update(|model| model.receive(operations.iter().filter(|op| pending.contains(&op.opId))))?;
        }
        drop(control);
        publishSyncMutation();
        Ok(())
    }

    /// The caller supplies a committed coverage proof for these exact originals.
    pub(crate) fn retireCoveredOperations(&self, operations: &[SyncOperation]) -> Result<(), SyncOperationStoreError> {
        self.withJournal(|journal|journal.transaction(|tx|crate::Transaction::Journal::retire_ids(tx,
            operations.iter().map(|operation|operation.opId.clone()))))
    }

    pub fn pendingOperationIds(&self) -> Result<Vec<String>, SyncOperationStoreError> {
        self.withJournal(|journal| journal.pending_ids())
    }

    pub fn pendingOperationCount(&self) -> Result<usize, SyncOperationStoreError> { Ok(self.pendingOperationIds()?.len()) }

    /// Retains receipts and identity checks while dropping synchronization work outside declared ownership.
    pub fn ignoreFileOperations(&self, domain: &str, path: &str) -> Result<(), SyncOperationStoreError> {
        let routes = crate::PreferenceLayout::routes(self.storageHost.clone()).map_err(SyncOperationStoreError::Message)?;
        self.withJournal(|journal| journal.transaction(|tx| {
            for path in std::iter::once(path).chain(routes.iter().filter(|(_,target)| target.as_str() == path).map(|(source,_)|source.as_str())) {
                let ids = tx.query("SELECT id FROM originals WHERE domain=?1 AND target=?2", vec![crate::Transaction::text(domain),crate::Transaction::text(path)])?
                    .iter().map(|row|row.valueAt(0)?.asString()).collect::<Result<Vec<_>,_>>()?;
                crate::Transaction::Journal::retire_ids(tx, ids)?;
                tx.execute("DELETE FROM transaction_files WHERE path=?1", vec![crate::Transaction::text(path)])?;
                tx.execute("DELETE FROM transaction_owners WHERE path=?1", vec![crate::Transaction::text(path)])?;
            }
            Ok(())
        }))?;
        operit_util::AppLogger::AppLogger::i("TransactionSync", &format!("sync_file.ignored domain={domain} path={path} reason=unregistered"));
        Ok(())
    }

    /// Discovers committed and pending preference owners without decoding transaction payloads.
    pub fn preferenceOwnerPaths(&self) -> Result<Vec<String>, SyncOperationStoreError> {
        self.withJournal(|journal| journal.owner_paths())
    }

    pub fn pendingOperations(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.pendingOperationIds()?.iter().map(|id| self.original(id)).collect()
    }

    /// Skipped originals still relay normally, but no longer block file correction as pending work.
    pub fn skippedOperations(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        self.withJournal(|journal| journal.skipped_ids())?.iter().map(|id| self.original(id)).collect()
    }

    pub(crate) fn skipOperations(&self, operations: &[SyncOperation]) -> Result<(), SyncOperationStoreError> {
        self.withJournal(|journal| journal.skip(operations))?;
        publishSyncMutation();
        Ok(())
    }

    pub fn visitPendingOperations(&self, domains: &[&str], mut visit: impl FnMut(&SyncOperation)) -> Result<(), SyncOperationStoreError> {
        for header in self.withJournal(|journal| journal.headers(Some(true)))? {
            if domains.is_empty() || domains.contains(&header.domain.as_str()) { visit(&self.original(&header.op_id)?); }
        }
        Ok(())
    }

    pub fn completeOperations(&self, operations: &[SyncOperation]) -> Result<(), SyncOperationStoreError> {
        let control = beginControlWrite(&self.storageHost, &self.rootPath, operations.iter().any(|op| op.domain == NETWORK_CONTROL_SYNC_DOMAIN))?;
        let _revision = control.as_ref().map(|_| ControlJournalMutation::new(&self.sharedState.controlRevision));
        self.withJournal(|journal| journal.complete(operations))?;
        if let Some(control) = &control { control.update(|model| model.complete(operations))?; }
        Ok(())
    }

    pub fn receipt(&self) -> Result<SyncReceipt, SyncOperationStoreError> { self.withJournal(|journal| journal.receipt()) }

    pub fn operationIdsSince<K: SyncOperationKnowledge>(&self, clock: &K, domains: &[String]) -> Result<BTreeSet<String>,SyncOperationStoreError> {
        let mut ids = BTreeSet::new();
        self.readJournalWithInventory(clock, JournalQuery { domains,projection: JournalProjection::All,
            export_floors: true,limit: 0,max_bytes: 0,retain: None },Some(&mut ids))?;
        Ok(ids)
    }

    fn readJournalWithInventory<K: SyncOperationKnowledge>(&self, clock: &K, query: JournalQuery<'_>, mut inventory: Option<&mut BTreeSet<String>>) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        if query.limit == 0 && inventory.is_none() { return Ok(Vec::new()); }
        let floors = if query.export_floors {
            let mut state = lockSyncState(&self.sharedState.exportFloors, "export floors")?;
            self.loadCachedJson(&mut state, &self.exportFloorsPath())?.clone()
        } else { BTreeMap::new() };
        let headers = self.withJournal(|journal| journal.headers(if query.export_floors { None } else { Some(false) }))?;
        let mut selected = Vec::new();
        for record in headers {
            let floor = floors.get(&record.origin_device_id).copied().unwrap_or(0);
            let control = record.domain == NETWORK_CONTROL_SYNC_DOMAIN && record.sequence <= clock.sequenceFor(&networkControlClockKey(&record.origin_device_id));
            if record.sequence <= floor || control || clock.containsSequence(&record.origin_device_id, record.sequence)
                || (!query.domains.is_empty() && !query.domains.contains(&record.domain)) || !query.projection.includes(&record) { continue; }
            selected.push((record.order()?, record));
        }
        // Filter before choosing a replaceable entity's latest original.
        let mut winners = BTreeMap::new();
        for (_, record) in &selected {
            if record.domain != "preferences" && record.semantics == SyncOperationSemantics::EntityState {
                winners.entry((&record.origin_device_id, &record.domain, &record.entity_type, &record.entity_id))
                    .or_insert_with(Vec::new).push(record);
            }
        }
        let mut winnerIds = BTreeSet::new();
        for group in winners.values_mut() {
            group.sort_by_key(|record| std::cmp::Reverse(record.sequence));
            for record in group {
                if query.retain.map(|retain| self.original(&record.op_id).map(|op| retain(&op))).transpose()?.unwrap_or(true) {
                    winnerIds.insert(record.op_id.clone()); break;
                }
            }
        }
        selected.sort_by(|a, b| a.0.cmp(&b.0));
        let mut page = Vec::new();
        let mut bytes = 0usize;
        let mut full = query.limit == 0;
        for (_, record) in selected {
            if record.domain != "preferences" && record.semantics == SyncOperationSemantics::EntityState && !winnerIds.contains(&record.op_id) { continue; }
            if full && inventory.is_none() { break; }
            if query.retain.is_none() {
                if let Some(inventory) = inventory.as_deref_mut() { inventory.insert(record.op_id.clone()); }
                if full { continue; }
            }
            let op = self.original(&record.op_id)?;
            if query.retain.is_some_and(|retain| !retain(&op)) { continue; }
            if let Some(inventory) = inventory.as_deref_mut() { inventory.insert(op.opId.clone()); }
            if full { continue; }
            let mut size = SerializedByteCount::default();
            serde_json::to_writer(&mut size, &op)?;
            if !page.is_empty() && bytes.saturating_add(size.0) > query.max_bytes { full = true; continue; }
            bytes = bytes.saturating_add(size.0);
            page.push(op);
            full = page.len() == query.limit;
        }
        Ok(page)
    }
}
