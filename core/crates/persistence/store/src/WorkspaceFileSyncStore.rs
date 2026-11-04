//! Replicates managed workspace files using the existing file/blob protocol.
//! External mounts are never traversed. The durable inventory detects edits made
//! by terminals/editors and deletions made while the application was stopped.
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership, RUNTIME_CHAT_OUTPUTS_DIR_PATH, LEGACY_CHAT_ATTACHMENTS_DIR_PATH};
use crate::FileSyncPolicy::{FileSyncPolicy, FileReference, POLICY_PATH, REFERENCE_SUFFIX, MAX_POLICY_BYTES, readLimited, ignored, readIgnore, isControl};
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
    #[serde(default)]
    references: Inventory,
    #[serde(default)]
    deferred: BTreeMap<String, Option<RuntimeFileSnapshot>>,
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
            && Self::manages(&operation.entityId)
    }

    /// Shared roots use one scanner, policy evaluator and transport protocol.
    pub fn manages(path: &str) -> bool {
        ["workspaces", RUNTIME_CHAT_OUTPUTS_DIR_PATH, LEGACY_CHAT_ATTACHMENTS_DIR_PATH].iter()
            .any(|root| path.starts_with(&format!("{root}/")))
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

    /// Completes traversal and validates all policies before publishing any state.
    fn scanUnlocked(&self) -> Result<usize, String> {
        let mut state = self.load()?;
        self.recoverPending(&mut state)?;
        // Received states outside the current policy remain durable. Re-enabling
        // a path restores the latest received version instead of losing it.
        for (path, next) in state.deferred.clone() {
            if self.accepts(&path, next.as_ref())? {
                if let Some(snapshot) = &next {
                    if !self.files().hasBlob(&snapshot.reference)? { continue; }
                }
                let content = next.as_ref().map(|s| self.files().readBlob(&s.reference)).transpose()?;
                state.deferred.remove(&path);
                state.pending = Some((path, next));
                self.save(&state)?;
                self.finishPending(&mut state, content.as_deref())?;
            }
        }
        let mut current = Inventory::new();
        let mut skipped = Vec::new();
        let mut large = Inventory::new();
        for root in ["workspaces", RUNTIME_CHAT_OUTPUTS_DIR_PATH, LEGACY_CHAT_ATTACHMENTS_DIR_PATH] {
            if !self.host.exists(root).map_err(|e| e.to_string())? {
                if state.files.keys().any(|path| path.starts_with(&format!("{root}/"))) {
                    return Err(format!("Managed root {root} is unavailable; refusing mass deletions"));
                }
                continue;
            }
            for entry in self.host.list(root).map_err(|e| e.to_string())? {
                if !entry.isDirectory { continue; }
                let policy = FileSyncPolicy::load(self.host.as_ref(), &entry.path)?;
                let mut rules = policy.rules(self.host.as_ref(), &entry.path)?;
                let mut candidates = Inventory::new();
                self.collect(&entry.path, &entry.path, &policy, &mut rules, &mut candidates, &mut skipped)?;
                let mut total = 0u64;
                for (path, stamp) in candidates {
                    let control = isControl(&entry.path, &path);
                    let reference = path.ends_with(REFERENCE_SUFFIX);
                    let size = u64::try_from(stamp.size).map_err(|_| format!("Invalid file size: {path}"))?;
                    if control || reference || (size <= policy.maxFileBytes && size <= policy.maxTotalBytes.saturating_sub(total)) {
                        if !control && !reference { total += size; }
                        current.insert(path, stamp);
                    } else { large.insert(path, stamp); }
                }
            }
        }
        // Sidecars are small synchronized files. The large original stays on its source node.
        for (path, stamp) in &large {
            let sidecar = format!("{path}{REFERENCE_SUFFIX}");
            // A formerly small file exceeding the limit is no longer uploaded,
            // but changing a limit must not erase the peer's existing copy.
            skipped.push(path.clone());
            let missing = !self.host.exists(&sidecar).map_err(|e| e.to_string())?;
            if state.references.get(path) != Some(stamp) || missing {
                let reference = FileReference::new(self.host.clone(), managedVfsPath(path)?, path.rsplit('/').next().unwrap().into(), stamp.size, stamp.modifiedAt)?;
                let bytes = reference.bytes()?;
                // A user-authored file occupying the reserved suffix must never be overwritten.
                if !missing {
                    let old: FileReference = serde_json::from_slice(&readLimited(self.host.as_ref(), &sidecar, MAX_POLICY_BYTES)?).map_err(|_| format!("Reference path is occupied: {sidecar}"))?;
                    if old.format != "operit_file_reference" || old.version != 1 || old.path != reference.path { return Err(format!("Reference path is occupied: {sidecar}")); }
                }
                self.host.writeBytesAtomically(&sidecar, &bytes).map_err(|e| e.to_string())?;
            }
            current.insert(sidecar.clone(), FileStamp { size: self.host.list(path.rsplit_once('/').unwrap().0).map_err(|e| e.to_string())?.into_iter().find(|e| e.path == sidecar).ok_or("Missing file reference")?.size, modifiedAt: Some(self.host.modifiedAt(&sidecar).map_err(|e| e.to_string())?) });
        }
        for path in state.references.keys().filter(|path| !large.contains_key(*path)) {
            if skipped.iter().any(|prefix| path.as_str() == prefix || path.starts_with(&format!("{prefix}/"))) { continue; }
            let sidecar = format!("{path}{REFERENCE_SUFFIX}");
            if self.host.exists(&sidecar).map_err(|e| e.to_string())? {
                let bytes = readLimited(self.host.as_ref(), &sidecar, MAX_POLICY_BYTES)?;
                if let Ok(reference) = serde_json::from_slice::<FileReference>(&bytes) {
                    let local = crate::CoreNodeIdentityStore::CoreNodeIdentityStore::new(self.host.clone()).identity()?.nodeId;
                    if reference.nodeId == local && reference.path == managedVfsPath(path)? {
                        self.host.delete(&sidecar, false).map_err(|e| e.to_string())?;
                        current.remove(&sidecar);
                    }
                }
            }
        }
        // Ignoring data is not a deletion. Keep its last observed stamp dormant.
        for (path, stamp) in &state.files {
            if skipped.iter().any(|prefix| path == prefix || path.starts_with(&format!("{prefix}/"))) { current.insert(path.clone(), stamp.clone()); }
        }
        let files = self.files();
        let mut changes = 0;
        for path in state.files.keys().filter(|path| !current.contains_key(*path)) { files.recordDeletion(path)?; changes += 1; }
        for (path, stamp) in &current {
            if state.files.get(path) != Some(stamp) {
                let root = managedDirectory(path)?;
                let policy = FileSyncPolicy::load(self.host.as_ref(), &root)?;
                let limit = if path.ends_with(REFERENCE_SUFFIX) || isControl(&root, path) { MAX_POLICY_BYTES } else { usize::try_from(policy.maxFileBytes).unwrap_or(usize::MAX) };
                let bytes = readLimited(self.host.as_ref(), path, limit)?;
                if bytes.len() as i64 != stamp.size || Some(self.host.modifiedAt(path).map_err(|e| e.to_string())?) != stamp.modifiedAt { return Err(format!("File changed during synchronization: {path}")); }
                files.recordSnapshot(path, &bytes, false)?;
                changes += 1;
            }
        }
        let inventoryChanged = state.files != current;
        state.files = current;
        // Preserve references hidden by a newly added ignore rule too.
        for (path, stamp) in &state.references {
            if skipped.iter().any(|prefix| path == prefix || path.starts_with(&format!("{prefix}/"))) { large.insert(path.clone(), stamp.clone()); }
        }
        let referencesChanged = state.references != large;
        state.references = large;
        if inventoryChanged || referencesChanged { self.save(&state)?; }
        Ok(changes)
    }

    fn collect(&self, root: &str, directory: &str, policy: &FileSyncPolicy, rules: &mut Vec<ignore::gitignore::Gitignore>, files: &mut Inventory, skipped: &mut Vec<String>) -> Result<(), String> {
        let previous = rules.len();
        if directory != root && policy.ignoreFiles.iter().any(|p| p == ".gitignore") {
            if let Some(rule) = readIgnore(self.host.as_ref(), &format!("{directory}/.gitignore"))? { rules.push(rule); }
        }
        for entry in self.host.list(directory).map_err(|e| e.to_string())? {
            validatePath(&entry.path)?;
            entry.path.strip_prefix(&format!("{directory}/")).ok_or_else(|| format!("Managed entry escaped directory: {}", entry.path))?;
            if !isControl(root, &entry.path) && ignored(rules, &entry.path, entry.isDirectory) { skipped.push(entry.path); continue; }
            if entry.isDirectory { self.collect(root, &entry.path, policy, rules, files, skipped)?; }
            else { files.insert(entry.path.clone(), FileStamp { size: entry.size, modifiedAt: Some(self.host.modifiedAt(&entry.path).map_err(|e| e.to_string())?) }); }
        }
        rules.truncate(previous);
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
        let next: Option<RuntimeFileSnapshot> = match operation {
            "upsert" => Some(serde_json::from_value(payload).map_err(|e| e.to_string())?),
            "delete" => None,
            other => return Err(format!("Unsupported managed file operation: {other}")),
        };
        let mut state = self.load()?;
        if !self.accepts(path, next.as_ref())? {
            state.deferred.insert(path.to_string(), next);
            return self.save(&state);
        }
        self.checkDestination(path)?;
        state.deferred.remove(path);
        let content = next.as_ref().map(|snapshot: &RuntimeFileSnapshot| self.files().readBlob(&snapshot.reference)).transpose()?;
        state.pending = Some((path.to_string(), next));
        self.save(&state)?;
        self.finishPending(&mut state, content.as_deref())
    }

    /// Policy decisions precede content transfer as well as materialization.
    pub fn wantsBlob(&self, operation: &SyncOperation) -> Result<bool, String> {
        let snapshot = if operation.operation == "upsert" { Some(serde_json::from_value::<RuntimeFileSnapshot>(operation.payload.clone()).map_err(|e| e.to_string())?) } else { None };
        self.accepts(&operation.entityId, snapshot.as_ref())
    }

    /// Files deferred by a policy become eligible for transfer when it changes.
    pub fn deferredBlobs(&self) -> Result<Vec<crate::RuntimeFileSyncStore::RuntimeFileSyncReference>, String> {
        let mut result = Vec::new();
        for (path, snapshot) in self.load()?.deferred {
            if let Some(snapshot) = snapshot {
                if self.accepts(&path, Some(&snapshot))? { result.push(snapshot.reference); }
            }
        }
        Ok(result)
    }

    pub(crate) fn captureLimit(&self, path: &str) -> Result<Option<usize>, String> {
        let state = self.load()?;
        if state.references.contains_key(path) { return Ok(None); }
        let root = managedDirectory(path)?;
        let policy = FileSyncPolicy::load(self.host.as_ref(), &root)?;
        if !policy.includes(self.host.as_ref(), &root, path, false)? { return Ok(None); }
        Ok(Some(if isControl(&root, path) || path.ends_with(REFERENCE_SUFFIX) { MAX_POLICY_BYTES } else { usize::try_from(policy.maxFileBytes.min(policy.maxTotalBytes)).unwrap_or(usize::MAX) }))
    }

    fn accepts(&self, path: &str, snapshot: Option<&RuntimeFileSnapshot>) -> Result<bool, String> {
        let root = managedDirectory(path)?;
        let policy = FileSyncPolicy::load(self.host.as_ref(), &root)?;
        if !policy.includes(self.host.as_ref(), &root, path, false)? { return Ok(false); }
        let control = isControl(&root, path) || path.ends_with(REFERENCE_SUFFIX);
        let limit = if control { MAX_POLICY_BYTES as u64 } else { policy.maxFileBytes };
        let Some(snapshot) = snapshot else { return Ok(true); };
        if snapshot.reference.size < 0 || snapshot.reference.size as u64 > limit { return Ok(false); }
        if control { return Ok(true); }
        let mut total = snapshot.reference.size as u64;
        let state = self.load()?;
        for (other, stamp) in &state.files {
            if other != path && !state.references.contains_key(other) && other.starts_with(&format!("{root}/"))
                && !isControl(&root, other) && !other.ends_with(REFERENCE_SUFFIX)
                && policy.includes(self.host.as_ref(), &root, other, false)? {
                total = total.saturating_add(stamp.size.max(0) as u64);
            }
        }
        Ok(total <= policy.maxTotalBytes)
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
        let root = managedDirectory(path)?;
        let mut parent = path.rsplit_once('/').map(|(parent, _)| parent);
        while let Some(directory) = parent {
            if directory == root
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
        let mut parent = parts[0].to_string();
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

/// Returns the stable app-managed directory owning one file.
pub fn managedDirectory(path: &str) -> Result<String, String> {
    for root in ["workspaces", RUNTIME_CHAT_OUTPUTS_DIR_PATH, LEGACY_CHAT_ATTACHMENTS_DIR_PATH] {
        if let Some(relative) = path.strip_prefix(&format!("{root}/")) {
            let name = relative.split('/').next().filter(|s| !s.is_empty()).ok_or("Missing managed directory")?;
            return Ok(format!("{root}/{name}"));
        }
    }
    Err(format!("Not a managed file path: {path}"))
}
pub fn managedVfsPath(path: &str) -> Result<String, String> {
    if let Some(rest) = path.strip_prefix("workspaces/") { return Ok(format!("/operitapp/workspaces/{rest}")); }
    if let Some(rest) = path.strip_prefix(&format!("{RUNTIME_CHAT_OUTPUTS_DIR_PATH}/")) { return Ok(format!("/operitapp/outputs/{rest}")); }
    if let Some(rest) = path.strip_prefix("runtime/") { return Ok(format!("/operitapp/data/{rest}")); }
    Err("Not a managed VFS file".into())
}
fn validatePath(path: &str) -> Result<(), String> {
    if !WorkspaceFileSyncStore::manages(path) || path.split('/').any(|p| p.is_empty() || p == "." || p == ".." || p.contains(['\\', '\0'])) || runtimeStorageOwnership(path)? != RuntimeStorageOwnership::Space {
        return Err(format!("Not a managed shared path: {path}"));
    }
    Ok(())
}
fn validateFilePath(path: &str) -> Result<(), String> {
    validatePath(path)?;
    if managedDirectory(path)? == path { return Err("Managed file must belong to a directory".into()); }
    Ok(())
}
