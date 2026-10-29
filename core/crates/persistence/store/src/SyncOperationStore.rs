use operit_host_api::AtomicCounter::AtomicU64;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};
use std::io::{BufReader, Write};
use crate::SyncReceiveJournal::{ReceiveState, load_receive_state, persist_pending};
#[cfg(test)]
use crate::SyncReceiveJournal::pending_path;
use crate::SyncOperationJournal::{JournalIndex, JournalLines, JournalReader, JournalRecord, JOURNAL_READ_BYTES};

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

/// Orders revisions of one SQL chat operation; equal versions and times converge by content hash.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct ChatOperationRevision {
    schema_version: i32,
    created_at: i64,
    fingerprint: String,
    entity_type: String,
    entity_id: String,
}

impl ChatOperationRevision {
    /// Captures the migrated format, author timestamp and deterministic content identity.
    pub(crate) fn of(operation: &SyncOperation) -> Result<Self, SyncOperationStoreError> {
        use sha2::{Digest, Sha256};
        Ok(Self {
            schema_version: operation.schemaVersion,
            created_at: operation.createdAt,
            fingerprint: format!("{:x}", Sha256::digest(serde_json::to_vec(operation)?)),
            entity_type: operation.entityType.clone(),
            entity_id: operation.entityId.clone(),
        })
    }
}

/// Selects the newer SQL chat revision without relaxing immutable identities in other domains.
pub fn incomingSyncOperationWins(
    existing: &SyncOperation,
    incoming: &SyncOperation,
) -> Result<bool, SyncOperationStoreError> {
    if existing == incoming { return Ok(false); }
    if existing.domain != "chat" || incoming.domain != "chat"
        || existing.opId != incoming.opId
        || existing.originDeviceId != incoming.originDeviceId
        || existing.sequence != incoming.sequence
        || existing.entityType != incoming.entityType
        || existing.entityId != incoming.entityId {
        return Err(SyncOperationStoreError::Message(format!(
            "Conflicting immutable operation {}", incoming.opId
        )));
    }
    Ok(ChatOperationRevision::of(incoming)? > ChatOperationRevision::of(existing)?)
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
    controlRevision: AtomicU64,
    clock: Mutex<SyncOperationCachedValue<SyncClock>>,
    devices: Mutex<SyncOperationCachedValue<Vec<String>>>,
    entityVersions: Mutex<SyncOperationCachedValue<BTreeMap<String, SyncOperationOrder>>>,
    exportFloors: Mutex<SyncOperationCachedValue<BTreeMap<String, i64>>>,
    localDeviceId: Mutex<SyncOperationCachedValue<Option<String>>>,
    operationLogs: Mutex<HashMap<String, Arc<Mutex<JournalIndex>>>>,
    receiveState: Mutex<SyncOperationCachedValue<ReceiveState>>,
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
enum JournalProjection<'a> {
    All,
    PreferenceDatasetHistory,
    MergeMetadata,
    PreferenceDataset(&'a str),
}

impl JournalProjection<'_> {
    fn includes(&self, record: &JournalRecord) -> bool {
        match self {
            Self::All => true,
            Self::PreferenceDatasetHistory => record.domain == "preferences",
            Self::MergeMetadata => record.is_merge_candidate(),
            Self::PreferenceDataset(path) => {
                record.domain == "preferences" && record.preference_path_matches(path)
            }
        }
    }

    fn includes_decoded(&self, operation: &SyncOperation) -> bool {
        match self {
            Self::All => true,
            Self::PreferenceDatasetHistory => operation.domain == "preferences",
            Self::MergeMetadata => crate::PreferenceMerge::isMergeMetadata(operation),
            Self::PreferenceDataset(path) => {
                operation.payload.get("storagePath").and_then(Value::as_str) == Some(path)
            }
        }
    }
}

struct JournalQuery<'a> {
    domains: &'a [String],
    projection: JournalProjection<'a>,
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
        let received = {
            let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
            self.loadReceiveState(&mut cached)?
                .pending
                .values()
                .filter(|pending| pending.record.domain == NETWORK_CONTROL_SYNC_DOMAIN)
                .map(|pending| {
                    self.readRecordAt(
                        &pending.path(&self.rootPath),
                        &pending.record,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?
        };
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
        lockSyncState(&self.sharedState.devices, "devices")?.loaded = false;
        lockSyncState(&self.sharedState.receiveState, "receive state")?.loaded = false;
        let logs = lockSyncState(&self.sharedState.operationLogs, "operation log registry")?
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for log in logs {
            *lockSyncState(&log, "operation log")? = JournalIndex::default();
        }
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
        let controlWrite = beginControlWrite(
            &self.storageHost, &self.rootPath, operation.domain == NETWORK_CONTROL_SYNC_DOMAIN,
        )?;
        let mut clockState = lockSyncState(&self.sharedState.clock, "clock")?;
        let operationLog = self.operationLog(originDeviceId)?;
        let mut operationLog = lockSyncState(&operationLog, "operation log")?;
        self.loadOperationLogIndex(originDeviceId, &mut operationLog)?;
        let mut clock = self
            .loadCachedJson(&mut clockState, &self.clockPath())?
            .clone();
        let sequence = clock
            .sequenceFor(originDeviceId)
            .max(operationLog.highest_sequence)
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
        let operationOffset = operationLog.byte_length + u64::from(operationLog.needs_separator);
        let writtenBytes = self.appendOperationLine(&op, &mut operationLog)?;
        let lineLength = writtenBytes - u64::from(operationOffset > operationLog.byte_length);
        operationLog.appended(&op, operationOffset, lineLength, encryptedPreferenceSyncOperation(&op) || storedEncryptedSyncPayload(&op.payload));
        operationLog.highest_sequence = sequence;
        operationLog.byte_length += writtenBytes;
        if let Some(write) = &controlWrite {
            write.update(|model| model.append_journal([&op]))?;
        }
        clock.setSequence(originDeviceId.to_string(), sequence);
        self.writeLocalClockValue(&clock)?;
        clockState.value = clock;
        drop(operationLog);
        drop(clockState);
        self.registerDevice(originDeviceId)?;
        self.recordAppliedOperation(&op)?;
        drop(controlWrite);
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
        let controlWrite = beginControlWrite(
            &self.storageHost, &self.rootPath,
            operations.iter().any(|operation| operation.domain == NETWORK_CONTROL_SYNC_DOMAIN),
        )?;
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
            let mut nextOffset = operationLog.byte_length + u64::from(operationLog.needs_separator);
            for operation in originOperations {
                if operationLog.operation_ids.contains(&operation.opId)
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
                self.appendOperationBytes(&originDeviceId, content, &mut operationLog,
                    appendedOperations.iter().any(|(operation, _, _)| operation.domain == NETWORK_CONTROL_SYNC_DOMAIN))?;
                if let Some(write) = &controlWrite {
                    write.update(|model| model.append_journal(appendedOperations.iter().map(|(operation, _, _)| *operation)))?;
                }
                for (operation, offset, lineLength) in appendedOperations {
                    operationLog.appended(operation, offset, lineLength, encryptedPreferenceSyncOperation(operation) || storedEncryptedSyncPayload(&operation.payload));
                }
                operationLog.byte_length = nextOffset;
                appendedOrigins.insert(originDeviceId);
            }
        }
        if clockChanged {
            self.writeLocalClockValue(&clock)?;
            clockState.value = clock;
        }
        drop(clockState);
        drop(controlWrite);
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
        let controlWrite = beginControlWrite(
            &self.storageHost, &self.rootPath, operation.domain == NETWORK_CONTROL_SYNC_DOMAIN,
        )?;
        let operationLog = self.operationLog(&operation.originDeviceId)?;
        let mut operationLog = lockSyncState(&operationLog, "operation log")?;
        self.loadOperationLogIndex(&operation.originDeviceId, &mut operationLog)?;
        if operationLog.operation_ids.contains(&operation.opId) {
            return Ok(());
        }
        let operationOffset = operationLog.byte_length + u64::from(operationLog.needs_separator);
        let writtenBytes = self.appendOperationLine(operation, &mut operationLog)?;
        let lineLength = writtenBytes - u64::from(operationOffset > operationLog.byte_length);
        operationLog.appended(operation, operationOffset, lineLength, encryptedPreferenceSyncOperation(operation) || storedEncryptedSyncPayload(&operation.payload));
        operationLog.byte_length += writtenBytes;
        if let Some(write) = &controlWrite {
            write.update(|model| model.append_journal([operation]))?;
        }
        drop(operationLog);
        self.registerDevice(&operation.originDeviceId)?;
        drop(controlWrite);
        publishSyncMutation();
        Ok(())
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
        self.readJournal(
            &SyncClock::empty(),
            JournalQuery {
                domains: &[],
                projection: JournalProjection::PreferenceDataset(path),
                export_floors: false,
                limit: usize::MAX,
                max_bytes: usize::MAX,
                retain: None,
            },
        )
    }

    /// Plans using locations and ordering keys, then opens only the selected page.
    /// All origin guards are acquired in device order. This keeps indexed locations stable
    /// across append/repair and never holds a business dataset lock during export publication.
    fn readJournal<K: SyncOperationKnowledge>(
        &self,
        clock: &K,
        query: JournalQuery<'_>,
    ) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        if query.limit == 0 {
            return Ok(Vec::new());
        }
        let export_floors = if query.export_floors {
            let mut state = lockSyncState(&self.sharedState.exportFloors, "export floors")?;
            self.loadCachedJson(&mut state, &self.exportFloorsPath())?
                .clone()
        } else {
            BTreeMap::new()
        };
        // Receiving takes this ledger lock before an origin lock; every reader
        // uses the same order so a replay/export cannot deadlock an append.
        let mut receive = if query.export_floors {
            Some(lockSyncState(
                &self.sharedState.receiveState,
                "receive state",
            )?)
        } else {
            None
        };
        if let Some(state) = receive.as_mut() {
            self.loadReceiveState(state)?;
        }
        let mut devices = self.devices()?;
        devices.sort();
        devices.dedup();
        let logs = devices
            .iter()
            .map(|device| self.operationLog(device))
            .collect::<Result<Vec<_>, _>>()?;
        let mut indexes = logs
            .iter()
            .map(|log| lockSyncState(log, "operation log"))
            .collect::<Result<Vec<_>, _>>()?;
        for (device, index) in devices.iter().zip(&mut indexes) {
            self.loadOperationLogIndex(device, index)?;
        }
        let mut sources = Vec::new();
        if let Some(receive) = &receive {
            for pending in receive.value.pending.values() {
                let record = &pending.record;
                if !clock.containsSequence(&record.origin_device_id, record.sequence) {
                    sources.push((pending.path(&self.rootPath), record));
                }
            }
        }
        for (device, index) in devices.iter().zip(&indexes) {
            let minimum = clock
                .sequenceFor(device)
                .max(export_floors.get(device).copied().unwrap_or(0));
            let control_minimum = clock
                .sequenceFor(&networkControlClockKey(device))
                .max(minimum);
            let path = self.operationsPath(device);
            for record in &index.records {
                let floor = if record.domain == NETWORK_CONTROL_SYNC_DOMAIN {
                    control_minimum
                } else {
                    minimum
                };
                if record.sequence > floor && !clock.containsSequence(device, record.sequence) {
                    sources.push((path.clone(), record));
                }
            }
        }
        let mut candidates = Vec::new();
        let mut latest = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for (path, record) in sources {
            if (!query.domains.is_empty() && !query.domains.contains(&record.domain))
                || !query.projection.includes(record)
                || !seen.insert(&record.op_id)
            {
                continue;
            }
            let order = if record.route.encrypted || query.retain.is_some() {
                let operation = self.readRecordAt(&path, record)?;
                if !query.projection.includes_decoded(&operation)
                    || query.retain.is_some_and(|retain| !retain(&operation))
                {
                    continue;
                }
                SyncOperationOrder::fromOperation(&operation)
            } else {
                record.order()?
            };
            if record.domain != "preferences"
                && record.semantics == SyncOperationSemantics::EntityState
            {
                let key = (
                    &record.origin_device_id,
                    &record.domain,
                    &record.entity_type,
                    &record.entity_id,
                );
                latest
                    .entry(key)
                    .and_modify(|sequence: &mut i64| *sequence = (*sequence).max(record.sequence))
                    .or_insert(record.sequence);
            }
            candidates.push((path, record, order));
        }
        candidates.retain(|(_, record, _)| {
            record.domain == "preferences"
                || record.semantics == SyncOperationSemantics::Transaction
                || latest.get(&(
                    &record.origin_device_id,
                    &record.domain,
                    &record.entity_type,
                    &record.entity_id,
                )) == Some(&record.sequence)
        });
        candidates.sort_by(|left, right| left.2.cmp(&right.2));
        let mut page = Vec::new();
        let mut bytes = 0usize;
        for (path, record, _) in candidates {
            let operation = self.readRecordAt(&path, record)?;
            let mut size = SerializedByteCount::default();
            serde_json::to_writer(&mut size, &operation)?;
            if !page.is_empty() && bytes.saturating_add(size.0) > query.max_bytes {
                break;
            }
            bytes = bytes.saturating_add(size.0);
            page.push(operation);
            if page.len() == query.limit {
                break;
            }
        }
        Ok(page)
    }

    fn readJournalRecord(
        &self,
        device: &str,
        record: &JournalRecord,
    ) -> Result<SyncOperation, SyncOperationStoreError> {
        let path = self.operationsPath(device);
        self.readRecordAt(&path, record)
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

    fn loadReceiveState<'a>(
        &self,
        cached: &'a mut SyncOperationCachedValue<ReceiveState>,
    ) -> Result<&'a ReceiveState, SyncOperationStoreError> {
        if !cached.loaded {
            cached.value = load_receive_state(self.storageHost.as_ref(), &self.rootPath)?;
            cached.loaded = true;
        }
        Ok(&cached.value)
    }

    /// Serializes receipt and materialization so concurrent peers cannot replay one transaction.
    pub fn lockApplication(&self) -> Result<MutexGuard<'_, ()>, SyncOperationStoreError> {
        lockSyncState(&self.sharedState.application, "sync application")
    }

    /// Receives exact identities and queues newer SQL chat revisions for owner-level reconciliation.
    pub fn receiveOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        let controlWrite = beginControlWrite(
            &self.storageHost, &self.rootPath,
            operations.iter().any(|operation| operation.domain == NETWORK_CONTROL_SYNC_DOMAIN),
        )?;
        let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
        let mut state = self.loadReceiveState(&mut cached)?.clone();
        let mut changed = false;
        let mut staged = BTreeMap::<String, SyncOperation>::new();
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
            let pending = match state.pending.get(&operation.opId) {
                Some(pending) => Some(self.readRecordAt(&pending.path(&self.rootPath), &pending.record)?),
                None => None,
            };
            if let Some(existing) = staged.get(&operation.opId).or(pending.as_ref()) {
                if !incomingSyncOperationWins(existing, operation)? { continue; }
            }
            let revision = ChatOperationRevision::of(operation)?;
            if let Some(previous) = state.chat_revisions.get(&operation.opId) {
                if operation.domain != "chat" || previous.entity_type != operation.entityType
                    || previous.entity_id != operation.entityId {
                    return Err(SyncOperationStoreError::Message(format!(
                        "Conflicting immutable operation {}", operation.opId
                    )));
                }
                if *previous >= revision { continue; }
            }
            let fingerprintChanged = state.fingerprints.get(&operation.opId)
                .is_some_and(|previous| previous != &revision.fingerprint);
            if fingerprintChanged && operation.domain != "chat" {
                return Err(SyncOperationStoreError::Message(format!(
                    "Conflicting immutable operation {}", operation.opId
                )));
            }
            let journaled = match self.journalOperation(operation)? {
                Some(existing) => !incomingSyncOperationWins(&existing, operation)?,
                None => false,
            };
            if !fingerprintChanged
                && state.receipt.containsSequence(&operation.originDeviceId, operation.sequence)
                && (state.fingerprints.contains_key(&operation.opId) || journaled) {
                continue;
            }
            // Legacy receipts retain only a fingerprint. Replaying a changed SQL record lets
            // its owner compare against the migrated database instead of rejecting the page.
            if !journaled { staged.insert(operation.opId.clone(), operation.clone()); }
            state.fingerprints.insert(operation.opId.clone(), revision.fingerprint.clone());
            if operation.domain == "chat" {
                state.chat_revisions.insert(operation.opId.clone(), revision);
            }
            state.receipt.observe(&operation.originDeviceId, operation.sequence);
            changed = true;
        }
        if changed {
            let mut replacedPaths = Vec::new();
            for operation in staged.values() {
                let pending = persist_pending(
                    self.storageHost.as_ref(), &self.rootPath,
                    &self.encodeOperationPayload(operation)?,
                    state.pending.contains_key(&operation.opId),
                )?;
                if let Some(previous) = state.pending.get(&operation.opId) {
                    if previous.path(&self.rootPath) != pending.path(&self.rootPath) {
                        replacedPaths.push(previous.path(&self.rootPath));
                    }
                }
                state.pending.insert(operation.opId.clone(), pending);
            }
            self.writeJson(&self.receiveStatePath(), &state)?;
            cached.value = state;
            if let Some(write) = &controlWrite {
                write.update(|model| model.receive(operations.iter()
                    .filter(|operation| cached.value.pending.contains_key(&operation.opId))))?;
            }
            for path in replacedPaths {
                self.storageHost.delete(&path, false)?;
            }
        }
        drop(cached);
        drop(controlWrite);
        if changed {
            publishSyncMutation();
        }
        Ok(())
    }

    /// Explicit replay input. The durable cache itself holds only record locations.
    pub fn pendingOperations(&self) -> Result<Vec<SyncOperation>, SyncOperationStoreError> {
        let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
        let state = self.loadReceiveState(&mut cached)?;
        let mut pending = state.pending.values().collect::<Vec<_>>();
        pending.sort_by(|left, right| {
            (&left.record.origin_device_id, left.record.sequence)
                .cmp(&(&right.record.origin_device_id, right.record.sequence))
        });
        pending
            .into_iter()
            .map(|pending| {
                self.readRecordAt(
                    &pending.path(&self.rootPath),
                    &pending.record,
                )
            })
            .collect()
    }

    /// Counts work without opening any business record.
    pub fn pendingOperationCount(&self) -> Result<usize, SyncOperationStoreError> {
        let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
        Ok(self.loadReceiveState(&mut cached)?.pending.len())
    }

    /// Visits only the requested owners, retaining at most one decoded payload.
    pub fn visitPendingOperations(
        &self,
        domains: &[&str],
        mut visit: impl FnMut(&SyncOperation),
    ) -> Result<(), SyncOperationStoreError> {
        let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
        for pending in self.loadReceiveState(&mut cached)?.pending.values() {
            if domains.is_empty() || domains.contains(&pending.record.domain.as_str()) {
                let operation = self.readRecordAt(
                    &pending.path(&self.rootPath),
                    &pending.record,
                )?;
                visit(&operation);
            }
        }
        Ok(())
    }

    /// Retains skipped originals durably before ending their replay intent, without applying values.
    pub fn skipOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        self.appendOperations(operations)?;
        self.flushOperations(operations)?;
        self.completeOperations(operations)
    }

    /// Clears completed replay intent while retaining its receipt independently of native logs.
    pub fn completeOperations(
        &self,
        operations: &[SyncOperation],
    ) -> Result<(), SyncOperationStoreError> {
        let controlWrite = beginControlWrite(
            &self.storageHost, &self.rootPath,
            operations.iter().any(|operation| operation.domain == NETWORK_CONTROL_SYNC_DOMAIN),
        )?;
        let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
        let mut state = self.loadReceiveState(&mut cached)?.clone();
        let mut completed = Vec::new();
        for operation in operations {
            if let Some(pending) = state.pending.get(&operation.opId) {
                if self.readRecordAt(&pending.path(&self.rootPath), &pending.record)? == *operation {
                    completed.push(pending.path(&self.rootPath));
                    state.pending.remove(&operation.opId);
                }
            }
        }
        let changed = !completed.is_empty();
        if changed {
            self.writeJson(&self.receiveStatePath(), &state)?;
            cached.value = state;
            if let Some(write) = &controlWrite {
                write.update(|model| model.complete(operations))?;
            }
        }
        if changed {
            for path in completed {
                if self.storageHost.exists(&path)? {
                    self.storageHost.delete(&path, false)?;
                }
            }
        }
        Ok(())
    }

    /// Combines retained receipts with exact journal sequences without acknowledging gaps.
    pub fn receipt(&self) -> Result<SyncReceipt, SyncOperationStoreError> {
        let mut receipt = {
            let mut cached = lockSyncState(&self.sharedState.receiveState, "receive state")?;
            self.loadReceiveState(&mut cached)?
                .receipt
                .clone()
        };
        for origin in self.devices()? {
            let log = self.operationLog(&origin)?;
            let mut log = lockSyncState(&log, "operation log")?;
            self.loadOperationLogIndex(&origin, &mut log)?;
            for sequence in log.sequences.keys() {
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
        let Some(existing) = self.journalOperation(operation)? else { return Ok(false); };
        if existing != *operation {
            return Err(SyncOperationStoreError::Message(format!(
                "Conflicting immutable operation {}",
                operation.opId
            )));
        }
        Ok(true)
    }

    /// Reads the archived content for an exact origin sequence before resolving a delivery.
    fn journalOperation(&self, operation: &SyncOperation) -> Result<Option<SyncOperation>, SyncOperationStoreError> {
        let log = self.operationLog(&operation.originDeviceId)?;
        let mut log = lockSyncState(&log, "operation log")?;
        self.loadOperationLogIndex(&operation.originDeviceId, &mut log)?;
        log.sequences.get(&operation.sequence).map(|position|
            self.readJournalRecord(&operation.originDeviceId, &log.records[*position])
        ).transpose()
    }

    /// Locates durable receipts and replay intents beside the operation journal.
    fn receiveStatePath(&self) -> String {
        format!("{}/receive_state.json", self.rootPath)
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

    /// Opens the single metadata index shared by receipt, replay, export, and append.
    #[allow(non_snake_case)]
    fn loadOperationLogIndex(&self, deviceId: &str, index: &mut JournalIndex) -> Result<(), SyncOperationStoreError> {
        index.load(self.storageHost.as_ref(), &self.operationsPath(deviceId), deviceId)
    }

    /// Appends one encoded operation as a complete JSON line.
    #[allow(non_snake_case)]
    fn appendOperationLine(
        &self,
        operation: &SyncOperation,
        index: &mut JournalIndex,
    ) -> Result<u64, SyncOperationStoreError> {
        let storedOperation = self.encodeOperationPayload(operation)?;
        let mut content = serde_json::to_vec(&storedOperation)?;
        content.push(b'\n');
        self.appendOperationBytes(&operation.originDeviceId, content, index, operation.domain == NETWORK_CONTROL_SYNC_DOMAIN)
    }

    /// Separates an interrupted tail from the next intact operation and reloads after uncertain writes.
    fn appendOperationBytes(
        &self,
        origin: &str,
        mut content: Vec<u8>,
        index: &mut JournalIndex,
        authority: bool,
    ) -> Result<u64, SyncOperationStoreError> {
        let _revision = authority.then(|| ControlJournalMutation::new(&self.sharedState.controlRevision));
        if index.needs_separator { content.insert(0, b'\n'); }
        let writtenBytes = u64::try_from(content.len()).map_err(|_| {
            SyncOperationStoreError::Message(
                "sync operation line length does not fit u64".to_string(),
            )
        })?;
        if let Err(error) = self.storageHost.appendBytes(&self.operationsPath(origin), &content) {
            *index = JournalIndex::default();
            return Err(error.into());
        }
        index.needs_separator = false;
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
        let path = self.operationsPath(deviceId);
        if !self.storageHost.exists(&path)? { return Ok(scan); }
        scan.exists = true;
        let log = self.operationLog(deviceId)?;
        let _guard = lockSyncState(&log, "operation log")?;
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
    ) -> Result<Arc<Mutex<JournalIndex>>, SyncOperationStoreError> {
        let mut operationLogs =
            lockSyncState(&self.sharedState.operationLogs, "operation log registry")?;
        Ok(Arc::clone(
            operationLogs
                .entry(deviceId.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(JournalIndex::default()))),
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
        let controlWrite = beginControlWrite(&self.storageHost, &self.rootPath, true)?;
        let operationLog = self.operationLog(deviceId)?;
        let mut index = lockSyncState(&operationLog, "operation log")?;
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
        *index = JournalIndex::default();
        drop(index);
        if let Some(write) = &controlWrite {
            match self.scanOperationLog(deviceId) {
                Ok(repaired) => write.update(|model| model.replace_journal_origin(deviceId, &repaired.operations))?,
                Err(error) => {
                    write.update(|model| model.unavailable(error.to_string()))?;
                    return Err(error);
                }
            }
        }
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
