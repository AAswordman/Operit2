use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde::{Deserialize, Serialize};

use crate::CoreNodeIdentityStore::CoreNodeIdentityStore;
use crate::RuntimeStorageHost::defaultRuntimeStorageHost;
use crate::SyncOperationStore::{
    NewSyncOperation, SyncClock, SyncOperation, SyncOperationOrder, SyncOperationSemantics,
    SyncOperationStore,
};

/// Synchronization domain containing opaque CoreNode Binding records.
pub const BINDING_SYNC_DOMAIN: &str = "binding";
const BINDING_SYNC_ENTITY_TYPE: &str = "binding";

static CORE_NODE_BINDING_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static CORE_NODE_BINDING_SHARED_STATES: OnceLock<
    Mutex<HashMap<CoreNodeBindingRegistryKey, Arc<CoreNodeBindingSharedState>>>,
> = OnceLock::new();

/// Holds one Binding cache shared by every store over the same storage host.
struct CoreNodeBindingSharedState {
    records: Arc<Mutex<Option<BTreeMap<String, CoreNodeBindingRecord>>>>,
    observers: Mutex<Vec<Weak<CoreNodeBindingChangeObserver>>>,
}

/// Identifies one process-local Binding cache by storage host and sync root.
struct CoreNodeBindingRegistryKey {
    storageHost: Arc<dyn RuntimeStorageHost>,
    rootPath: String,
}

impl PartialEq for CoreNodeBindingRegistryKey {
    /// Compares registry keys by host identity and synchronization root.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.storageHost, &other.storageHost) && self.rootPath == other.rootPath
    }
}

impl Eq for CoreNodeBindingRegistryKey {}

impl Hash for CoreNodeBindingRegistryKey {
    /// Hashes registry keys by host identity and synchronization root.
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.storageHost) as *const () as usize).hash(state);
        self.rootPath.hash(state);
    }
}

/// Returns the mutation lock used to preserve local compare-and-set semantics.
#[allow(non_snake_case)]
fn coreNodeBindingMutationLock() -> &'static Mutex<()> {
    CORE_NODE_BINDING_MUTATION_LOCK.get_or_init(|| Mutex::new(()))
}

/// Returns the cache shared by stores over one storage host and synchronization root.
#[allow(non_snake_case)]
fn coreNodeBindingSharedState(
    storageHost: &Arc<dyn RuntimeStorageHost>,
) -> Arc<CoreNodeBindingSharedState> {
    let states = CORE_NODE_BINDING_SHARED_STATES.get_or_init(|| Mutex::new(HashMap::new()));
    let key = CoreNodeBindingRegistryKey {
        storageHost: Arc::clone(storageHost),
        rootPath: RUNTIME_SYNC_DIR_PATH.to_string(),
    };
    let mut states = states
        .lock()
        .expect("CoreNode Binding shared state registry mutex must not be poisoned");
    if let Some(state) = states.get(&key) {
        return Arc::clone(state);
    }
    let state = Arc::new(CoreNodeBindingSharedState {
        records: Arc::new(Mutex::new(None)),
        observers: Mutex::new(Vec::new()),
    });
    states.insert(key, Arc::clone(&state));
    state
}

/// Describes one committed change to the Binding entity-state cache.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreNodeBindingChange {
    Upsert(CoreNodeBindingRecord),
    Delete(String),
}

/// Receives one committed Binding cache change.
pub type CoreNodeBindingChangeObserver = dyn Fn(CoreNodeBindingChange) + Send + Sync;

/// Publishes one committed Binding change to the shared host-scoped cache and observers.
#[allow(non_snake_case)]
fn notifyCoreNodeBindingChanged(
    sharedState: &CoreNodeBindingSharedState,
    change: CoreNodeBindingChange,
) {
    {
        let mut cache = sharedState
            .records
            .lock()
            .expect("CoreNode Binding cache mutex must not be poisoned");
        if let Some(records) = cache.as_mut() {
            match &change {
                CoreNodeBindingChange::Upsert(binding) => {
                    records.insert(binding.key.clone(), binding.clone());
                }
                CoreNodeBindingChange::Delete(key) => {
                    records.remove(key);
                }
            }
        }
    }
    let observers = {
        let mut registered = sharedState
            .observers
            .lock()
            .expect("CoreNode Binding observer mutex must not be poisoned");
        let mut observers = Vec::with_capacity(registered.len());
        registered.retain(|observer| match observer.upgrade() {
            Some(observer) => {
                observers.push(observer);
                true
            }
            None => false,
        });
        observers
    };
    for observer in observers {
        observer(change.clone());
    }
}

/// Stores the CoreNode selected by one opaque Binding key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreNodeBindingRecord {
    pub key: String,
    pub nodeId: String,
    pub generation: i64,
}

/// Reports one committed Binding change and the operation that persists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreNodeBindingCommit {
    pub binding: CoreNodeBindingRecord,
    pub operation: SyncOperation,
}

/// Provides persistent opaque Binding records backed by the common Space operation log.
#[derive(Clone)]
pub struct CoreNodeBindingStore {
    syncOperationStore: SyncOperationStore,
    localNodeId: String,
    sharedState: Arc<CoreNodeBindingSharedState>,
}

impl CoreNodeBindingStore {
    /// Opens the Binding store over one CoreNode runtime storage capability.
    pub fn new(storageHost: Arc<dyn RuntimeStorageHost>) -> Result<Self, String> {
        let localNodeId = CoreNodeIdentityStore::new(storageHost.clone())
            .initialize()?
            .nodeId;
        let sharedState = coreNodeBindingSharedState(&storageHost);
        Ok(Self {
            syncOperationStore: SyncOperationStore::new(storageHost, RUNTIME_SYNC_DIR_PATH),
            localNodeId,
            sharedState,
        })
    }

    /// Opens the Binding store over the configured default runtime storage capability.
    pub fn default() -> Result<Self, String> {
        Self::new(defaultRuntimeStorageHost())
    }

    /// Reads the current CoreNode selection for one opaque Binding key.
    pub fn binding(&self, key: &str) -> Result<CoreNodeBindingRecord, String> {
        self.bindingOptional(key)?
            .ok_or_else(|| format!("Binding does not exist: {key}"))
    }

    /// Reads the current CoreNode selection when one opaque Binding key has a record.
    #[allow(non_snake_case)]
    pub fn bindingOptional(&self, key: &str) -> Result<Option<CoreNodeBindingRecord>, String> {
        self.bindingRecord(key)
    }

    /// Reports whether one opaque Binding key has a persisted current record.
    pub fn contains(&self, key: &str) -> Result<bool, String> {
        Ok(self.bindingRecord(key)?.is_some())
    }

    /// Creates the initial Binding for one newly created business key.
    pub fn create(&self, key: &str, nodeId: &str) -> Result<CoreNodeBindingCommit, String> {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        validateBindingKey(key)?;
        validateBindingNodeId(nodeId)?;
        if self.bindingRecord(key)?.is_some() {
            return Err(format!("Binding already exists: {key}"));
        }
        self.appendBindingRecord(CoreNodeBindingRecord {
            key: key.to_string(),
            nodeId: nodeId.to_string(),
            generation: 1,
        })
    }

    /// Creates one Binding that selects the local CoreNode as its initial target.
    #[allow(non_snake_case)]
    pub fn createLocal(&self, key: &str) -> Result<CoreNodeBindingCommit, String> {
        self.create(key, &self.localNodeId)
    }

    /// Deletes one Binding by appending an authoritative entity-state tombstone.
    pub fn delete(&self, key: &str) -> Result<(), String> {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        validateBindingKey(key)?;
        self.binding(key)?;
        self.appendBindingOperation(key, "delete", serde_json::Value::Null)?;
        notifyCoreNodeBindingChanged(
            &self.sharedState,
            CoreNodeBindingChange::Delete(key.to_string()),
        );
        Ok(())
    }

    /// Registers one observer for committed Binding changes.
    #[allow(non_snake_case)]
    pub fn addChangeObserver(&self, observer: Arc<CoreNodeBindingChangeObserver>) {
        self.sharedState
            .observers
            .lock()
            .expect("CoreNode Binding observer mutex must not be poisoned")
            .push(Arc::downgrade(&observer));
    }

    /// Atomically changes one Binding when the expected node remains selected.
    #[allow(non_snake_case)]
    pub fn compareAndSet(
        &self,
        key: &str,
        expectedNodeId: &str,
        targetNodeId: &str,
    ) -> Result<CoreNodeBindingCommit, String> {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        validateBindingKey(key)?;
        validateBindingNodeId(expectedNodeId)?;
        validateBindingNodeId(targetNodeId)?;
        let current = self.binding(key)?;
        if current.nodeId != expectedNodeId {
            return Err(format!(
                "Binding conflict for {key}: expected {expectedNodeId}, actual {}",
                current.nodeId
            ));
        }
        if current.nodeId == targetNodeId {
            return Ok(CoreNodeBindingCommit {
                binding: current,
                operation: self.latestBindingOperation(key)?,
            });
        }
        self.appendBindingRecord(CoreNodeBindingRecord {
            key: key.to_string(),
            nodeId: targetNodeId.to_string(),
            generation: current.generation + 1,
        })
    }

    /// Atomically changes one Binding while both its node and generation remain exact.
    #[allow(non_snake_case)]
    pub fn compareAndSetGeneration(
        &self,
        key: &str,
        expectedNodeId: &str,
        expectedGeneration: i64,
        targetNodeId: &str,
    ) -> Result<CoreNodeBindingCommit, String> {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        validateBindingKey(key)?;
        validateBindingNodeId(expectedNodeId)?;
        validateBindingNodeId(targetNodeId)?;
        if expectedGeneration <= 0 {
            return Err(format!(
                "Binding generation must be positive for {key}: {expectedGeneration}"
            ));
        }
        let current = self.binding(key)?;
        if current.nodeId != expectedNodeId || current.generation != expectedGeneration {
            return Err(format!(
                "Binding conflict for {key}: expected {expectedNodeId}@{expectedGeneration}, actual {}@{}",
                current.nodeId, current.generation
            ));
        }
        let generation = current
            .generation
            .checked_add(1)
            .ok_or_else(|| format!("Binding generation overflow for {key}"))?;
        self.appendBindingRecord(CoreNodeBindingRecord {
            key: key.to_string(),
            nodeId: targetNodeId.to_string(),
            generation,
        })
    }

    /// Commits or joins one exact Binding source transition under the mutation lock.
    #[allow(non_snake_case)]
    pub fn transitionGeneration(
        &self,
        key: &str,
        sourceNodeId: &str,
        sourceGeneration: i64,
        targetNodeId: &str,
    ) -> Result<CoreNodeBindingCommit, String> {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        validateBindingKey(key)?;
        validateBindingNodeId(sourceNodeId)?;
        validateBindingNodeId(targetNodeId)?;
        if sourceGeneration <= 0 {
            return Err(format!(
                "Binding generation must be positive for {key}: {sourceGeneration}"
            ));
        }
        let targetGeneration = sourceGeneration
            .checked_add(1)
            .ok_or_else(|| format!("Binding generation overflow for {key}"))?;
        let current = self.binding(key)?;
        if current.nodeId == targetNodeId && current.generation == targetGeneration {
            return Ok(CoreNodeBindingCommit {
                binding: current,
                operation: self.latestBindingOperation(key)?,
            });
        }
        if current.nodeId != sourceNodeId || current.generation != sourceGeneration {
            return Err(format!(
                "Binding transition conflict for {key}: expected {sourceNodeId}@{sourceGeneration} or {targetNodeId}@{targetGeneration}, actual {}@{}",
                current.nodeId, current.generation
            ));
        }
        self.appendBindingRecord(CoreNodeBindingRecord {
            key: key.to_string(),
            nodeId: targetNodeId.to_string(),
            generation: targetGeneration,
        })
    }

    /// Moves every Binding outside the supplied node set onto the local CoreNode.
    #[allow(non_snake_case)]
    pub fn rebindOutsideNodesToLocal(
        &self,
        allowedNodeIds: &BTreeSet<String>,
    ) -> Result<Vec<CoreNodeBindingCommit>, String> {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        if !allowedNodeIds.contains(&self.localNodeId) {
            return Err("Allowed Binding nodes must include the local CoreNode".to_string());
        }
        for nodeId in allowedNodeIds {
            validateBindingNodeId(nodeId)?;
        }

        let mut latestOperations = BTreeMap::<String, SyncOperation>::new();
        for operation in self.bindingOperations()? {
            validateBindingOperation(&operation)?;
            let replace = latestOperations
                .get(&operation.entityId)
                .map(|current| bindingOperationOrder(current) < bindingOperationOrder(&operation))
                .unwrap_or(true);
            if replace {
                latestOperations.insert(operation.entityId.clone(), operation);
            }
        }

        let mut commits = Vec::new();
        for operation in latestOperations.into_values() {
            if operation.operation != "upsert" {
                continue;
            }
            let binding: CoreNodeBindingRecord = serde_json::from_value(operation.payload)
                .map_err(|error| format!("Binding payload is invalid: {error}"))?;
            if allowedNodeIds.contains(&binding.nodeId) {
                continue;
            }
            let generation = binding
                .generation
                .checked_add(1)
                .ok_or_else(|| format!("Binding generation overflow for {}", binding.key))?;
            commits.push(self.appendBindingRecord(CoreNodeBindingRecord {
                key: binding.key,
                nodeId: self.localNodeId.clone(),
                generation,
            })?);
        }
        Ok(commits)
    }

    /// Applies one synchronized Binding operation without creating another local operation.
    #[allow(non_snake_case)]
    pub fn applySyncedOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        self.applyOperation(operation, false)
    }

    /// Applies one Space bootstrap Binding operation without local conflict filtering.
    #[allow(non_snake_case)]
    pub fn applyBootstrapOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        self.applyOperation(operation, true)
    }

    /// Applies one Binding operation with the selected conflict policy.
    #[allow(non_snake_case)]
    fn applyOperation(
        &self,
        operation: &SyncOperation,
        ignoreConflict: bool,
    ) -> Result<(), String> {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        validateBindingOperation(operation)?;
        let shouldApply = ignoreConflict
            || self
                .syncOperationStore
                .shouldApplyOperation(operation)
                .map_err(|error| error.to_string())?;
        self.syncOperationStore
            .appendOperation(operation)
            .map_err(|error| error.to_string())?;
        if !shouldApply {
            return Ok(());
        }
        if ignoreConflict {
            self.syncOperationStore
                .recordBootstrapAppliedOperations(std::slice::from_ref(operation))
                .map_err(|error| error.to_string())?;
        } else {
            self.syncOperationStore
                .recordAppliedOperation(operation)
                .map_err(|error| error.to_string())?;
        }
        notifyBindingOperationChanged(&self.sharedState, operation)?;
        Ok(())
    }
    /// Applies one directly transported Binding operation without advancing persistence clocks.
    #[allow(non_snake_case)]
    pub fn applyImmediateOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        validateBindingOperation(operation)?;
        let shouldApply = self
            .syncOperationStore
            .shouldApplyOperation(operation)
            .map_err(|error| error.to_string())?;
        self.syncOperationStore
            .appendUnobservedOperation(operation)
            .map_err(|error| error.to_string())?;
        if !shouldApply {
            return Ok(());
        }
        self.syncOperationStore
            .recordAppliedOperation(operation)
            .map_err(|error| error.to_string())?;
        notifyBindingOperationChanged(&self.sharedState, operation)?;
        Ok(())
    }

    /// Executes one local mutation while an exact Binding generation remains selected here.
    #[allow(non_snake_case)]
    pub fn withLocalBindingGeneration<T, F>(
        &self,
        key: &str,
        generation: i64,
        operation: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        let _lock = coreNodeBindingMutationLock()
            .lock()
            .map_err(|error| format!("Binding mutation lock poisoned: {error}"))?;
        let binding = self.binding(key)?;
        if binding.nodeId != self.localNodeId {
            return Err(format!(
                "Binding target mismatch for {key}: expected {}, actual {}",
                self.localNodeId, binding.nodeId
            ));
        }
        if binding.generation != generation {
            return Err(format!(
                "Binding generation mismatch for {key}: expected {generation}, actual {}",
                binding.generation
            ));
        }
        operation()
    }

    /// Returns the record selected by the newest operation for one opaque key.
    fn bindingRecord(&self, key: &str) -> Result<Option<CoreNodeBindingRecord>, String> {
        let mut cache = self
            .sharedState
            .records
            .lock()
            .map_err(|error| format!("CoreNode Binding cache mutex poisoned: {error}"))?;
        if cache.is_none() {
            *cache = Some(self.materializeBindingRecords()?);
        }
        Ok(cache.as_ref().and_then(|records| records.get(key).cloned()))
    }

    /// Materializes every current Binding from the common synchronization log.
    fn materializeBindingRecords(&self) -> Result<BTreeMap<String, CoreNodeBindingRecord>, String> {
        let mut latestOperations = BTreeMap::<String, SyncOperation>::new();
        for operation in self.bindingOperations()? {
            validateBindingOperation(&operation)?;
            let replace = latestOperations
                .get(&operation.entityId)
                .map(|current| bindingOperationOrder(current) < bindingOperationOrder(&operation))
                .unwrap_or(true);
            if replace {
                latestOperations.insert(operation.entityId.clone(), operation);
            }
        }

        let mut records = BTreeMap::new();
        for operation in latestOperations.into_values() {
            if operation.operation != "upsert" {
                continue;
            }
            let binding: CoreNodeBindingRecord = serde_json::from_value(operation.payload)
                .map_err(|error| format!("Binding payload is invalid: {error}"))?;
            records.insert(binding.key.clone(), binding);
        }
        Ok(records)
    }

    /// Returns the newest materialized operation for one Binding key.
    fn latestBindingOperation(&self, key: &str) -> Result<SyncOperation, String> {
        self.bindingOperations()?
            .into_iter()
            .filter(|operation| operation.entityId == key)
            .max_by(|left, right| bindingOperationOrder(left).cmp(&bindingOperationOrder(right)))
            .ok_or_else(|| format!("Binding operation does not exist: {key}"))
    }

    /// Lists every materialized Binding operation from the common synchronization log.
    fn bindingOperations(&self) -> Result<Vec<SyncOperation>, String> {
        self.syncOperationStore
            .operationsSince(
                &SyncClock::empty(),
                &[BINDING_SYNC_DOMAIN.to_string()],
                usize::MAX,
            )
            .map_err(|error| error.to_string())
    }

    /// Appends one authoritative Binding entity-state operation and emits its change.
    fn appendBindingRecord(
        &self,
        binding: CoreNodeBindingRecord,
    ) -> Result<CoreNodeBindingCommit, String> {
        let operation = self.appendBindingOperation(
            &binding.key,
            "upsert",
            serde_json::to_value(&binding).map_err(|error| error.to_string())?,
        )?;
        notifyCoreNodeBindingChanged(
            &self.sharedState,
            CoreNodeBindingChange::Upsert(binding.clone()),
        );
        Ok(CoreNodeBindingCommit { binding, operation })
    }

    /// Appends one local Binding entity-state operation to the shared Space log.
    fn appendBindingOperation(
        &self,
        key: &str,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<SyncOperation, String> {
        self.syncOperationStore
            .appendLocalOperation(
                &self.localNodeId,
                NewSyncOperation {
                    domain: BINDING_SYNC_DOMAIN.to_string(),
                    entityType: BINDING_SYNC_ENTITY_TYPE.to_string(),
                    entityId: key.to_string(),
                    operation: operation.to_string(),
                    semantics: SyncOperationSemantics::EntityState,
                    payload,
                },
            )
            .map_err(|error| error.to_string())
    }
}

/// Converts one applied Binding operation into an incremental cache change.
#[allow(non_snake_case)]
fn notifyBindingOperationChanged(
    sharedState: &CoreNodeBindingSharedState,
    operation: &SyncOperation,
) -> Result<(), String> {
    let change = match operation.operation.as_str() {
        "upsert" => CoreNodeBindingChange::Upsert(
            serde_json::from_value(operation.payload.clone())
                .map_err(|error| format!("Binding payload is invalid: {error}"))?,
        ),
        "delete" => CoreNodeBindingChange::Delete(operation.entityId.clone()),
        _ => {
            return Err(format!(
                "Unsupported Binding operation: {}",
                operation.operation
            ))
        }
    };
    notifyCoreNodeBindingChanged(sharedState, change);
    Ok(())
}

/// Returns the deterministic conflict-resolution order for one Binding operation.
fn bindingOperationOrder(operation: &SyncOperation) -> SyncOperationOrder {
    SyncOperationOrder::fromOperation(operation)
}

/// Validates the exact operation contract owned by the Binding store.
fn validateBindingOperation(operation: &SyncOperation) -> Result<(), String> {
    if operation.domain != BINDING_SYNC_DOMAIN
        || operation.entityType != BINDING_SYNC_ENTITY_TYPE
        || !matches!(operation.operation.as_str(), "upsert" | "delete")
        || operation.semantics != SyncOperationSemantics::EntityState
    {
        return Err("operation is not a Binding entity-state update".to_string());
    }
    validateBindingKey(&operation.entityId)?;
    if operation.operation == "delete" {
        if !operation.payload.is_null() {
            return Err("Binding delete operation must carry a null payload".to_string());
        }
        return Ok(());
    }
    let binding: CoreNodeBindingRecord =
        serde_json::from_value(operation.payload.clone()).map_err(|error| error.to_string())?;
    if binding.key != operation.entityId {
        return Err("Binding operation key does not match its payload".to_string());
    }
    validateBindingNodeId(&binding.nodeId)?;
    if binding.generation <= 0 {
        return Err("Binding generation must be positive".to_string());
    }
    Ok(())
}

/// Validates one opaque Binding key.
fn validateBindingKey(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("Binding key must not be empty".to_string());
    }
    Ok(())
}

/// Validates one CoreNode id recorded by a Binding operation.
fn validateBindingNodeId(nodeId: &str) -> Result<(), String> {
    if nodeId.trim().is_empty() {
        return Err("Binding CoreNode id must not be empty".to_string());
    }
    Ok(())
}
