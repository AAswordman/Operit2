//! Replicates managed workspace files using the existing file/blob protocol.
//! External mounts are never traversed. The durable inventory detects edits made
//! by terminals/editors and deletions made while the application was stopped.
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership};
use serde::{Deserialize, Serialize};

use crate::RuntimeFileSyncStore::{
    RuntimeFileSnapshot, RuntimeFileSyncStore, RUNTIME_FILE_SYNC_DOMAIN,
};
use crate::SyncOperationStore::{SyncOperation, SyncOperationStore};

type Inventory = BTreeMap<String, FileStamp>;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileStamp {
    size: i64,
    #[serde(default)]
    modifiedAt: Option<i64>,
}
type WorkspaceLocks = Vec<(Weak<dyn RuntimeStorageHost>, String, Arc<Mutex<()>>)>;
static LOCKS: OnceLock<Mutex<WorkspaceLocks>> = OnceLock::new();

#[derive(Serialize, Deserialize, Default)]
struct ScanState {
    files: Inventory,
    // Persist the complete incoming write before touching the live file.
    pending: Option<(String, Option<RuntimeFileSnapshot>)>,
}

pub struct WorkspaceFileSyncStore {
    host: Arc<dyn RuntimeStorageHost>,
    root: String,
    lock: Arc<Mutex<()>>,
}

impl WorkspaceFileSyncStore {
    /// Shares one scan/write lock per host and inventory root.
    pub fn new(host: Arc<dyn RuntimeStorageHost>, root: impl Into<String>) -> Self {
        let root = root.into();
        let mut locks = LOCKS
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .expect("workspace lock registry poisoned");
        locks.retain(|(host, _, _)| host.strong_count() > 0);
        let lock = locks
            .iter()
            .find_map(|(other, otherRoot, lock)| {
                (otherRoot == &root
                    && other
                        .upgrade()
                        .is_some_and(|other| Arc::ptr_eq(&host, &other)))
                .then(|| lock.clone())
            })
            .unwrap_or_else(|| {
                let lock = Arc::new(Mutex::new(()));
                locks.push((Arc::downgrade(&host), root.clone(), lock.clone()));
                lock
            });
        Self { host, root, lock }
    }

    /// Identifies complete states owned by the managed workspace.
    pub fn owns(operation: &SyncOperation) -> bool {
        operation.domain == RUNTIME_FILE_SYNC_DOMAIN
            && operation.entityId.starts_with("workspaces/")
    }

    /// Scans only the managed root; mounted VFS directories are not consulted.
    pub fn scan(&self) -> Result<usize, String> {
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        self.scanUnlocked()
    }

    /// Joining a Space makes old local operation logs unexportable. Retain local
    /// files, but inventory them again so files unique to this device are not stranded.
    pub fn prepareSpaceJoin(&self) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        let mut state = self.load()?;
        self.recoverPending(&mut state)?;
        self.save(&ScanState::default())
    }

    /// Captures app edits immediately; periodic scans also cover other writers.
    pub fn track<T>(&self, change: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        self.trackPaths::<T, &str>(&[], change)
    }

    /// Validates managed paths and captures both sides of an application edit.
    pub fn trackPaths<T, P: AsRef<str>>(&self, paths: &[P], change: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        for path in paths { validateFilePath(path.as_ref())?; }
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        self.scanUnlocked()?;
        let result = change();
        let scanned = self.scanUnlocked();
        match result { Err(error) => Err(error), Ok(value) => { scanned?; Ok(value) } }
    }

    /// Returns the node-local inventory path.
    fn statePath(&self) -> String {
        format!("{}/workspace_inventory.json", self.root)
    }
    /// Opens the shared file/blob carrier.
    fn files(&self) -> RuntimeFileSyncStore {
        RuntimeFileSyncStore::new(self.host.clone(), self.root.clone())
    }
    /// Opens the shared immutable operation journal.
    fn operations(&self) -> SyncOperationStore {
        SyncOperationStore::new(self.host.clone(), self.root.clone())
    }

    /// Reads the local inventory and any unfinished remote write.
    fn load(&self) -> Result<ScanState, String> {
        if !self.host.exists(&self.statePath()).map_err(|e| e.to_string())? { return Ok(ScanState::default()); }
        serde_json::from_slice(&self.host.readBytes(&self.statePath()).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Invalid workspace inventory: {e}"))
    }

    /// Publishes an inventory or recovery intent atomically.
    fn save(&self, state: &ScanState) -> Result<(), String> {
        self.host.writeBytesAtomically(&self.statePath(), &serde_json::to_vec(state).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    /// Compares only modification time and size; unchanged files never need content reads.
    fn scanUnlocked(&self) -> Result<usize, String> {
        let mut state = self.load()?;
        self.recoverPending(&mut state)?;
        if !state.files.is_empty() && !self.host.exists("workspaces").map_err(|e| e.to_string())? {
            return Err("Workspace root is unavailable; refusing to publish mass deletions".into());
        }
        let mut current = Inventory::new();
        self.collect("workspaces", &mut current)?;
        let files = self.files();
        let mut changes = 0;
        // Finish traversal before publishing deletes; deletes precede file/directory replacements.
        for path in state.files.keys().filter(|path| !current.contains_key(*path)) {
            files.recordDeletion(path)?;
            changes += 1;
        }
        for (path, stamp) in &current {
            if state.files.get(path) != Some(stamp) {
                let bytes = self.host.readBytes(path).map_err(|e| e.to_string())?;
                files.recordSnapshot(path, &bytes, false)?;
                changes += 1;
            }
        }
        if changes > 0 { state.files = current; self.save(&state)?; }
        Ok(changes)
    }

    /// Enumerates regular managed files without reading file contents or following links.
    fn collect(&self, directory: &str, files: &mut Inventory) -> Result<(), String> {
        for entry in self.host.list(directory).map_err(|e| e.to_string())? {
            validatePath(&entry.path)?;
            entry.path.strip_prefix(&format!("{directory}/"))
                .ok_or_else(|| format!("Workspace entry escaped directory: {}", entry.path))?;
            if entry.isDirectory { self.collect(&entry.path, files)?; }
            else if entry.path.split('/').count() >= 3 {
                let modifiedAt = Some(self.host.modifiedAt(&entry.path).map_err(|e| e.to_string())?);
                files.insert(entry.path, FileStamp { size: entry.size, modifiedAt });
            }
        }
        Ok(())
    }

    /// Serializes conflict decisions with scans and app writes. Incoming changes update
    /// the inventory so the scanner never republishes them as fresh local edits.
    pub fn applyOperation(&self, operation: &SyncOperation, bootstrap: bool) -> Result<bool, String> {
        let applied = self.materializeOperation(operation, bootstrap)?;
        self.operations().appendOperation(operation).map_err(|e| e.to_string())?;
        Ok(applied)
    }

    /// Materialize without advancing the vector clock ahead of the application's batch.
    pub fn materializeOperation(&self, operation: &SyncOperation, bootstrap: bool) -> Result<bool, String> {
        Ok(self.materializeOperations(&[operation], bootstrap)?[0])
    }

    /// Scans once and records versions only after each complete file is materialized.
    pub fn materializeOperations(&self, batch: &[&SyncOperation], bootstrap: bool) -> Result<Vec<bool>, String> {
        if batch.is_empty() { return Ok(Vec::new()); }
        for operation in batch {
            validateFilePath(&operation.entityId)?;
            if !Self::owns(operation) || operation.entityType != "file" { return Err("Invalid workspace operation".into()); }
        }
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        self.scanUnlocked()?;
        let operations = self.operations();
        let mut decisions = Vec::new();
        for operation in batch {
            let apply = bootstrap || operations.shouldApplyOperation(operation).map_err(|e| e.to_string())?;
            if apply {
                self.applyUnlocked(&operation.entityId, &operation.operation, operation.payload.clone())?;
                if bootstrap { operations.recordBootstrapAppliedOperations(std::slice::from_ref(*operation)) }
                else { operations.recordAppliedOperation(operation) }.map_err(|e| e.to_string())?;
            }
            decisions.push(apply);
        }
        Ok(decisions)
    }

    /// Low-level file materialization used by RuntimeFileSyncStore callers.
    pub fn apply(
        &self,
        path: &str,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        validateFilePath(path)?;
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        self.scanUnlocked()?;
        self.applyUnlocked(path, operation, payload)
    }

    /// Saves a complete recovery intent before materializing the incoming state.
    fn applyUnlocked(&self, path: &str, operation: &str, payload: serde_json::Value) -> Result<(), String> {
        self.checkDestination(path)?;
        let next = match operation {
            "upsert" => Some(serde_json::from_value(payload).map_err(|e| e.to_string())?),
            "delete" => None,
            other => return Err(format!("Unsupported workspace file operation: {other}")),
        };
        let content = next.as_ref().map(|snapshot: &RuntimeFileSnapshot| self.files().readBlob(&snapshot.reference)).transpose()?;
        let mut state = self.load()?;
        state.pending = Some((path.to_string(), next));
        self.save(&state)?;
        self.finishPending(&mut state, content.as_deref())
    }

    /// Completes an interrupted incoming write before any scan can publish local changes.
    fn recoverPending(&self, state: &mut ScanState) -> Result<(), String> {
        let content = match &state.pending {
            Some((_, Some(snapshot))) => Some(self.files().readBlob(&snapshot.reference)?),
            _ => None,
        };
        self.finishPending(state, content.as_deref())
    }

    /// Publishes already verified bytes and commits the corresponding metadata inventory.
    fn finishPending(&self, state: &mut ScanState, content: Option<&[u8]>) -> Result<(), String> {
        let Some((path, next)) = state.pending.clone() else {
            return Ok(());
        };
        validateFilePath(&path)?;
        self.checkDestination(&path)?;
        match next {
            Some(reference) => {
                self.host.writeBytes(&path, content.ok_or("Missing verified workspace file bytes")?).map_err(|e| e.to_string())?;
                if let Some(modifiedAt) = reference.modifiedAt {
                    self.host.setModifiedAt(&path, modifiedAt).map_err(|e| e.to_string())?;
                }
                self.host.flushFile(&path).map_err(|e| e.to_string())?;
                let modifiedAt = Some(self.host.modifiedAt(&path).map_err(|e| e.to_string())?);
                state.files.insert(path, FileStamp { size: reference.reference.size, modifiedAt });
            }
            None => {
                if self.host.exists(&path).map_err(|e| e.to_string())? {
                    self.host.delete(&path, false).map_err(|e| e.to_string())?;
                }
                state.files.remove(&path);
                self.removeEmptyParents(&path)?;
            }
        }
        state.pending = None;
        self.save(state)
    }

    /// Removes empty descendants after tombstones, keeping the workspace root.
    fn removeEmptyParents(&self, path: &str) -> Result<(), String> {
        let mut parent = path.rsplit_once('/').map(|(parent, _)| parent);
        while let Some(directory) = parent {
            if directory.split('/').count() < 3
                || !self.host.exists(directory).map_err(|e| e.to_string())?
                || !self.host.list(directory).map_err(|e| e.to_string())?.is_empty() { break; }
            // Never recurse: hidden links or newly-created files keep the directory intact.
            if self.host.delete(directory, false).is_err() {
                break;
            }
            parent = directory.rsplit_once('/').map(|(parent, _)| parent);
        }
        Ok(())
    }

    /// Rejects links and type conflicts at every existing destination component.
    fn checkDestination(&self, path: &str) -> Result<(), String> {
        let parts: Vec<_> = path.split('/').collect();
        let mut parent = "workspaces".to_string();
        for (index, part) in parts.iter().enumerate().skip(1) {
            let child = format!("{parent}/{part}");
            let entry = self.host.list(&parent).map_err(|e| e.to_string())?.into_iter().find(|entry| entry.path == child);
            match entry {
                Some(entry) if entry.isDirectory == (index + 1 < parts.len()) => {}
                Some(_) => return Err(format!("Workspace file/directory conflict: {child}")),
                None if self.host.exists(&child).map_err(|e| e.to_string())? => {
                    return Err(format!(
                        "Workspace synchronization will not follow a link: {child}"
                    ))
                }
                None => return Ok(()),
            }
            parent = child;
        }
        Ok(())
    }
}

/// Accepts only paths owned by the managed workspace.
fn validatePath(path: &str) -> Result<(), String> {
    if !path.starts_with("workspaces/")
        || path.split('/').count() < 2
        || runtimeStorageOwnership(path)? != RuntimeStorageOwnership::Space
    {
        return Err(format!("Not a managed workspace path: {path}"));
    }
    Ok(())
}

/// Requires a file path below a workspace directory.
fn validateFilePath(path: &str) -> Result<(), String> {
    validatePath(path)?;
    if path.split('/').count() < 3 {
        return Err("Workspace file must belong to a workspace directory".into());
    }
    Ok(())
}
