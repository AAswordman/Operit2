//! Current control state, maintained when authority changes rather than on reads.
//!
//! The registry retains only control records and states, never a storage host or
//! the business journal. Reopening a short-lived store reuses the same state;
//! dropping the host makes its entry eligible for removal.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};

use operit_host_api::RuntimeStorageHost;

use crate::NetworkControlStore::{
    decodeControlOperation, emptyControlState, replayControlCommands, NetworkControlState,
    NETWORK_CONTROL_SYNC_DOMAIN,
};
use crate::SyncOperationStore::{SyncOperation, SyncOperationOrder, SyncOperationStoreError};

pub(crate) type SharedControlReadModel = Arc<Mutex<NetworkControlReadModel>>;

#[derive(Default)]
pub(crate) struct NetworkControlReadModel {
    journal: BTreeMap<String, SyncOperation>,
    received: BTreeMap<String, SyncOperation>,
    projected: BTreeMap<String, SyncOperation>,
    commands: BTreeMap<String, Vec<SyncOperation>>,
    states: BTreeMap<String, NetworkControlState>,
    errors: BTreeMap<String, String>,
    recovery_error: Option<String>,
    pub(crate) projection_revision: u64,
}

impl NetworkControlReadModel {
    pub(crate) fn new(
        journal: Vec<SyncOperation>,
        received: Vec<SyncOperation>,
        projected: Vec<SyncOperation>,
    ) -> Self {
        let mut model = Self {
            journal: journal
                .into_iter()
                .map(|op| (op.opId.clone(), op))
                .collect(),
            received: received
                .into_iter()
                .map(|op| (op.opId.clone(), op))
                .collect(),
            projected: projected
                .into_iter()
                .map(|op| (op.opId.clone(), op))
                .collect(),
            ..Self::default()
        };
        model.rebuild();
        model
    }

    pub(crate) fn state(&self, space_id: &str) -> Result<NetworkControlState, String> {
        self.check(space_id)?;
        match self.states.get(space_id) {
            Some(state) => Ok(state.clone()),
            None => Ok(emptyControlState(space_id)),
        }
    }

    pub(crate) fn commands(&self, space_id: &str) -> Result<Vec<SyncOperation>, String> {
        self.check(space_id)?;
        Ok(self.commands.get(space_id).cloned().unwrap_or_default())
    }

    pub(crate) fn unavailable(&mut self, error: String) {
        self.recovery_error = Some(error);
    }

    fn check(&self, space_id: &str) -> Result<(), String> {
        if let Some(error) = &self.recovery_error { return Err(error.clone()); }
        match self.errors.get(space_id) {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    pub(crate) fn append_journal<'a>(
        &mut self,
        operations: impl IntoIterator<Item = &'a SyncOperation>,
    ) {
        let mut changed = false;
        for operation in operations {
            if operation.domain != NETWORK_CONTROL_SYNC_DOMAIN {
                continue;
            }
            if self.journal.get(&operation.opId) != Some(operation) {
                self.journal
                    .insert(operation.opId.clone(), operation.clone());
                changed = true;
            }
        }
        if changed {
            self.rebuild();
        }
    }

    pub(crate) fn receive<'a>(&mut self, operations: impl IntoIterator<Item = &'a SyncOperation>) {
        let mut changed = false;
        for operation in operations {
            if operation.domain != NETWORK_CONTROL_SYNC_DOMAIN {
                continue;
            }
            if self.received.get(&operation.opId) != Some(operation) {
                self.received
                    .insert(operation.opId.clone(), operation.clone());
                changed = true;
            }
        }
        if changed {
            self.rebuild();
        }
    }

    pub(crate) fn complete(&mut self, operations: &[SyncOperation]) {
        let mut changed = false;
        for operation in operations {
            changed |= self.received.remove(&operation.opId).is_some();
        }
        if changed {
            self.rebuild();
        }
    }

    pub(crate) fn replace_projected(&mut self, operations: Vec<SyncOperation>) {
        self.projected = operations
            .into_iter()
            .map(|op| (op.opId.clone(), op))
            .collect();
        self.rebuild();
    }

    pub(crate) fn replace_journal_origin(&mut self, origin: &str, operations: &[SyncOperation]) {
        self.journal
            .retain(|_, operation| operation.originDeviceId != origin);
        for operation in operations {
            if operation.domain == NETWORK_CONTROL_SYNC_DOMAIN {
                self.journal
                    .insert(operation.opId.clone(), operation.clone());
            }
        }
        self.rebuild();
    }

    pub(crate) fn retire_journal(&mut self) {
        self.journal.clear();
        self.rebuild();
    }

    /// Late operations may precede commands whose authorization they change.
    /// Replaying the small control set on mutations preserves the existing order
    /// and rejection semantics without putting replay in the query path.
    fn rebuild(&mut self) {
        self.commands.clear();
        self.states.clear();
        self.errors.clear();
        let mut unique = BTreeMap::new();
        for operation in self.journal.values().chain(self.received.values()).chain(self.projected.values()) {
            if let Some(previous) = unique.insert(&operation.opId, operation) {
                if previous != operation {
                    // Fail closed for both affected Spaces, without poisoning
                    // authorization reads for every other Space on this node.
                    for conflicting in [previous, operation] {
                        if let Some(space) = conflicting.payload.get("spaceId").and_then(serde_json::Value::as_str) {
                            self.errors.insert(space.into(), format!("Space {space}: Conflicting immutable control operation {}", operation.opId));
                        }
                    }
                }
            }
        }
        for operation in unique.into_values() {
            match decodeControlOperation(operation) {
                Ok(record) => self.commands.entry(record.spaceId).or_default().push(operation.clone()),
                Err(error) => {
                    let error = format!("Space control operation {} (origin {}): {error}", operation.opId, operation.originDeviceId);
                    if let Some(space) = operation.payload.get("spaceId").and_then(serde_json::Value::as_str).filter(|id| !id.trim().is_empty()) {
                        self.errors.insert(space.into(), error.clone());
                    }
                    // An undecodable operation grants no authority. Keep its
                    // diagnostic, but do not make unrelated Spaces unreadable.
                    operit_util::AppLogger::AppLogger::w("NetworkControlReadModel", &error);
                }
            }
        }
        for (space_id, commands) in &mut self.commands {
            commands.sort_by_key(SyncOperationOrder::fromOperation);
            match replayControlCommands(space_id, commands.iter(), None) {
                Ok((state, _)) => { self.states.insert(space_id.clone(), state); }
                Err(error) => { self.errors.insert(space_id.clone(), format!("Space {space_id} control replay: {error}")); }
            }
        }
    }

}

struct Entry {
    host: Weak<dyn RuntimeStorageHost>,
    root: String,
    model: SharedControlReadModel,
}

#[derive(Default)]
pub(crate) struct ControlReadModels {
    entries: Vec<Entry>,
}

impl ControlReadModels {
    pub(crate) fn get(
        &mut self,
        host: &Arc<dyn RuntimeStorageHost>,
        root: &str,
    ) -> Option<SharedControlReadModel> {
        self.entries.retain(|entry| entry.host.strong_count() > 0);
        let host = Arc::downgrade(host);
        self.entries
            .iter()
            .find(|entry| entry.root == root && Weak::ptr_eq(&entry.host, &host))
            .map(|entry| entry.model.clone())
    }

    pub(crate) fn insert(
        &mut self,
        host: &Arc<dyn RuntimeStorageHost>,
        root: &str,
        model: SharedControlReadModel,
    ) {
        self.entries.push(Entry {
            host: Arc::downgrade(host),
            root: root.into(),
            model,
        });
    }
}

/// Initialization and control writes take this before any journal/receipt lock.
/// Business writes never take it. Readers only lock their already-built model.
pub(crate) fn lockControlReadModels(
) -> Result<MutexGuard<'static, ControlReadModels>, SyncOperationStoreError> {
    static MODELS: OnceLock<Mutex<ControlReadModels>> = OnceLock::new();
    MODELS
        .get_or_init(|| Mutex::new(ControlReadModels::default()))
        .lock()
        .map_err(|_| {
            SyncOperationStoreError::Message("control read model registry mutex is poisoned".into())
        })
}

pub(crate) fn lockControlReadModel(
    model: &SharedControlReadModel,
) -> Result<MutexGuard<'_, NetworkControlReadModel>, SyncOperationStoreError> {
    model.lock().map_err(|_| {
        SyncOperationStoreError::Message("control read model mutex is poisoned".into())
    })
}

/// Keeps initialization from racing a durable control write. Publication is
/// explicit so callers can release this before notifying sync subscribers.
pub(crate) struct ControlWrite {
    _registry: MutexGuard<'static, ControlReadModels>,
    model: Option<SharedControlReadModel>,
}

impl ControlWrite {
    pub(crate) fn update(
        &self,
        update: impl FnOnce(&mut NetworkControlReadModel),
    ) -> Result<(), SyncOperationStoreError> {
        if let Some(model) = &self.model {
            let mut model = lockControlReadModel(model)?;
            update(&mut model);
        }
        Ok(())
    }
}

pub(crate) fn beginControlWrite(
    host: &Arc<dyn RuntimeStorageHost>,
    root: &str,
    controls: bool,
) -> Result<Option<ControlWrite>, SyncOperationStoreError> {
    if !controls {
        return Ok(None);
    }
    let mut registry = lockControlReadModels()?;
    let model = registry.get(host, root);
    Ok(Some(ControlWrite {
        _registry: registry,
        model,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SyncOperationStore::SyncOperationSemantics;

    fn bootstrap(space: &str) -> SyncOperation {
        SyncOperation {
            opId: format!("bootstrap-{space}"), originDeviceId: "local".into(), sequence: 1,
            domain: NETWORK_CONTROL_SYNC_DOMAIN.into(), entityType: "command".into(),
            entityId: format!("bootstrap-{space}"), operation: "apply".into(),
            semantics: SyncOperationSemantics::Transaction,
            payload: serde_json::to_value(crate::NetworkControlStore::NetworkControlCommandRecord {
                commandId: format!("bootstrap-{space}"), spaceId: space.into(), issuerNodeId: "local".into(),
                command: crate::NetworkControlStore::NetworkControlCommand::Bootstrap { initialAdminNodeId: "local".into() },
            }).unwrap(),
            createdAt: 1, schemaVersion: 1,
        }
    }

    #[test]
    fn a_broken_space_does_not_poison_other_spaces_or_a_fresh_local_space() {
        let good = bootstrap("healthy");
        let mut broken = bootstrap("broken");
        broken.payload["command"] = serde_json::Value::Null;
        let mut model = NetworkControlReadModel::new(vec![good, broken], Vec::new(), Vec::new());
        assert!(model.state("healthy").unwrap().initialized);
        assert!(model.state("broken").unwrap_err().contains("bootstrap-broken"));
        let fresh = bootstrap("fresh-after-exit");
        model.append_journal([&fresh]);
        assert!(model.state("fresh-after-exit").unwrap().initialized);
        assert!(model.state("healthy").unwrap().initialized);
    }

    #[test]
    fn a_conflicting_operation_blocks_only_its_own_spaces() {
        let good = bootstrap("healthy");
        let first = bootstrap("broken");
        let mut conflicting = first.clone();
        conflicting.createdAt += 1;
        let model = NetworkControlReadModel::new(vec![good, first], Vec::new(), vec![conflicting]);
        assert!(model.state("healthy").unwrap().initialized);
        assert!(model.state("broken").unwrap_err().contains("Conflicting immutable"));
    }
}
