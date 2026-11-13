//! Replicates managed workspace files using the existing file/blob protocol.
//! External mounts are never traversed. The durable inventory detects edits made
//! by terminals/editors and deletions made while the application was stopped.
#![allow(non_snake_case)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use operit_util::FileUtils::FileUtils;
use operit_host_api::{HostError, HostErrorKind, HostResult, RuntimeStorageHost};
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership, RUNTIME_CHAT_OUTPUTS_DIR_PATH, LEGACY_CHAT_ATTACHMENTS_DIR_PATH};
use crate::FileSyncPolicy::{FileSyncPolicy, FileReference, POLICY_PATH, REFERENCE_SUFFIX, MAX_POLICY_BYTES, readLimitedForScan, ignored, readIgnoreForScan, isControl};
use serde::{Deserialize, Serialize};

use crate::RuntimeFileSyncStore::{
    RuntimeFileSnapshot, RuntimeFileSyncStore, RUNTIME_FILE_SYNC_DOMAIN,
};
use crate::SyncOperationStore::{SyncOperation, SyncOperationStore};

type Inventory = BTreeMap<String, FileStamp>;
const MANAGED_ROOTS: [&str; 3] = ["workspaces", RUNTIME_CHAT_OUTPUTS_DIR_PATH, LEGACY_CHAT_ATTACHMENTS_DIR_PATH];

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
        MANAGED_ROOTS.iter()
            .any(|root| path.starts_with(&format!("{root}/")))
    }

    /// Includes managed descendants and ancestors whose mutation can change managed files.
    pub fn affectedBy(path: &str) -> bool {
        Self::manages(path) || MANAGED_ROOTS.iter()
            .any(|root| path == *root || root.starts_with(&format!("{path}/")))
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
        let pendingPath = state.pending.as_ref().map(|(path, _)| path.clone());
        self.recoverPending(&mut state)?;
        if let Some(path) = pendingPath.filter(|path| state.deferred.contains_key(path)) {
            return Err(format!("Cannot reset workspace inventory while a pending file is inaccessible: {path}"));
        }
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

    /// Traverses accessible files and retains prior inventory beneath permission failures.
    fn scanUnlocked(&self) -> Result<usize, String> {
        let mut state = self.load()?;
        // Incompatible names are outside sync scope, including old recovery state.
        self.operations().ignoreNonPortableFileOperations().map_err(|e| e.to_string())?;
        let previousCounts = (state.files.len(), state.references.len(), state.deferred.len());
        state.files.retain(|path, _| FileUtils::isPortableFilePath(path));
        state.references.retain(|path, _| FileUtils::isPortableFilePath(path));
        state.deferred.retain(|path, _| FileUtils::isPortableFilePath(path));
        if previousCounts != (state.files.len(), state.references.len(), state.deferred.len()) { self.save(&state)?; }
        self.recoverPending(&mut state)?;
        // Received states outside the current policy remain durable. Re-enabling
        // a path restores the latest received version instead of losing it.
        let mut skipped = Vec::new();
        for (path, next) in state.deferred.clone() {
            if self.accepts(&path, next.as_ref())? {
                if let Some(snapshot) = &next {
                    if !self.files().hasBlob(&snapshot.reference)? { continue; }
                }
                let content = next.as_ref().map(|s| self.files().readBlob(&s.reference)).transpose()?;
                state.deferred.remove(&path);
                state.pending = Some((path.clone(), next));
                self.save(&state)?;
                self.finishPending(&mut state, content.as_deref())?;
                if state.deferred.contains_key(&path) { skipped.push(path); }
            }
        }
        let mut current = Inventory::new();
        let mut large = Inventory::new();
        for root in MANAGED_ROOTS {
            let Some(exists) = self.scanRead(self.host.exists(root), root, &mut skipped)? else { continue; };
            if !exists {
                if state.files.keys().any(|path| path.starts_with(&format!("{root}/"))) {
                    return Err(format!("Managed root {root} is unavailable; refusing mass deletions"));
                }
                continue;
            }
            let Some(entries) = self.scanRead(self.host.list(root), root, &mut skipped)? else { continue; };
            for entry in entries {
                if !entry.isDirectory || !FileUtils::isPortableFilePath(&entry.path) { continue; }
                let Some(policy) = self.scanRead(FileSyncPolicy::loadForScan(self.host.as_ref(), &entry.path), &entry.path, &mut skipped)? else { continue; };
                let Some(mut rules) = self.scanRead(policy.rulesForScan(self.host.as_ref(), &entry.path), &entry.path, &mut skipped)? else { continue; };
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
            if isSkipped(path, &skipped) { continue; }
            let sidecar = format!("{path}{REFERENCE_SUFFIX}");
            if !FileUtils::isPortableFilePath(&sidecar) { skipped.push(path.clone()); continue; }
            // A formerly small file exceeding the limit is no longer uploaded,
            // but changing a limit must not erase the peer's existing copy.
            let Some(referenceStamp) = self.scanRead(self.captureReference(path, stamp, state.references.get(path)), path, &mut skipped)? else {
                skipped.push(sidecar);
                continue;
            };
            current.insert(sidecar, referenceStamp);
        }
        for path in state.references.keys().filter(|path| !large.contains_key(*path)) {
            if isSkipped(path, &skipped) { continue; }
            let sidecar = format!("{path}{REFERENCE_SUFFIX}");
            let Some(removed) = self.scanRead(self.removeLocalReference(path), path, &mut skipped)? else {
                skipped.push(sidecar);
                continue;
            };
            if removed { current.remove(&sidecar); }
        }
        let files = self.files();
        let mut changes = 0;
        for (path, stamp) in &current {
            if isSkipped(path, &skipped) { continue; }
            if state.files.get(path) != Some(stamp) {
                let root = managedDirectory(path)?;
                let Some(policy) = self.scanRead(FileSyncPolicy::loadForScan(self.host.as_ref(), &root), &root, &mut skipped)? else { continue; };
                let limit = if path.ends_with(REFERENCE_SUFFIX) || isControl(&root, path) { MAX_POLICY_BYTES } else { usize::try_from(policy.maxFileBytes).unwrap_or(usize::MAX) };
                let Some(bytes) = self.scanRead(readLimitedForScan(self.host.as_ref(), path, limit), path, &mut skipped)? else { continue; };
                let Some(modifiedAt) = self.scanRead(self.host.modifiedAt(path), path, &mut skipped)? else { continue; };
                if bytes.len() as i64 != stamp.size || Some(modifiedAt) != stamp.modifiedAt { return Err(format!("File changed during synchronization: {path}")); }
                // A verified file can replace an old directory, or a new child can
                // replace an old file. Publish those tombstones before the upsert.
                let replaced: Vec<_> = state.files.keys().filter(|previous|
                    previous.starts_with(&format!("{path}/")) || path.starts_with(&format!("{previous}/"))
                ).cloned().collect();
                for previous in replaced {
                    files.recordDeletion(&previous)?;
                    state.files.remove(&previous);
                    changes += 1;
                }
                files.recordSnapshot(path, &bytes, false)?;
                changes += 1;
            }
        }
        preserveSkipped(&mut large, &state.references, &skipped);
        skipped.extend(large.keys().cloned());
        preserveSkipped(&mut current, &state.files, &skipped);
        for path in state.files.keys().filter(|path| !current.contains_key(*path)) { files.recordDeletion(path)?; changes += 1; }
        let inventoryChanged = state.files != current;
        state.files = current;
        let referencesChanged = state.references != large;
        state.references = large;
        if inventoryChanged || referencesChanged { self.save(&state)?; }
        Ok(changes)
    }

    /// Retains prior inventory when access is denied or a file disappears during scanning.
    fn scanRead<T>(&self, result: HostResult<T>, path: &str, skipped: &mut Vec<String>) -> Result<Option<T>, String> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind == HostErrorKind::NotFound => {
                skipped.push(path.to_string());
                Ok(None)
            }
            Err(error) if error.kind == HostErrorKind::PermissionDenied => {
                operit_util::AppLogger::AppLogger::w("WorkspaceFileSyncStore",
                    &format!("Skipping inaccessible workspace path {path}; retaining prior sync state: {error}"));
                skipped.push(path.to_string());
                Ok(None)
            }
            Err(error) => Err(format!("Workspace path {path}: {error}")),
        }
    }

    /// Updates a large-file locator while preserving typed access failures.
    fn captureReference(&self, path: &str, stamp: &FileStamp, previous: Option<&FileStamp>) -> HostResult<FileStamp> {
        let sidecar = format!("{path}{REFERENCE_SUFFIX}");
        let missing = !self.host.exists(&sidecar)?;
        if previous != Some(stamp) || missing {
            let reference = FileReference::new(self.host.clone(), managedVfsPath(path).map_err(HostError::new)?, path.rsplit('/').next().unwrap().into(), stamp.size, stamp.modifiedAt).map_err(HostError::new)?;
            let bytes = reference.bytes().map_err(HostError::new)?;
            if !missing {
                let old: FileReference = serde_json::from_slice(&readLimitedForScan(self.host.as_ref(), &sidecar, MAX_POLICY_BYTES)?).map_err(|_| HostError::new(format!("Reference path is occupied: {sidecar}")))?;
                if old.format != "operit_file_reference" || old.version != 1 || old.path != reference.path { return Err(HostError::new(format!("Reference path is occupied: {sidecar}"))); }
            }
            self.host.writeBytesAtomically(&sidecar, &bytes)?;
        }
        let size = self.host.list(path.rsplit_once('/').unwrap().0)?.into_iter().find(|entry| entry.path == sidecar).ok_or_else(|| HostError::new("Missing file reference"))?.size;
        Ok(FileStamp { size, modifiedAt: Some(self.host.modifiedAt(&sidecar)?) })
    }

    /// Removes an obsolete local locator without losing access errors or replacing user files.
    fn removeLocalReference(&self, path: &str) -> HostResult<bool> {
        let sidecar = format!("{path}{REFERENCE_SUFFIX}");
        if !self.host.exists(&sidecar)? { return Ok(false); }
        let bytes = readLimitedForScan(self.host.as_ref(), &sidecar, MAX_POLICY_BYTES)?;
        if let Ok(reference) = serde_json::from_slice::<FileReference>(&bytes) {
            let local = crate::CoreNodeIdentityStore::CoreNodeIdentityStore::new(self.host.clone()).identity().map_err(HostError::new)?.nodeId;
            if reference.nodeId == local && reference.path == managedVfsPath(path).map_err(HostError::new)? {
                self.host.delete(&sidecar, false)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Traverses each accessible subtree with its own inherited ignore rules.
    fn collect(&self, root: &str, directory: &str, policy: &FileSyncPolicy, rules: &mut Vec<ignore::gitignore::Gitignore>, files: &mut Inventory, skipped: &mut Vec<String>) -> Result<(), String> {
        let Some(entries) = self.scanRead(self.host.list(directory), directory, skipped)? else { return Ok(()); };
        let previous = rules.len();
        if directory != root && policy.ignoreFiles.iter().any(|p| p == ".gitignore") {
            let Some(rule) = self.scanRead(readIgnoreForScan(self.host.as_ref(), &format!("{directory}/.gitignore")), directory, skipped)? else { return Ok(()); };
            if let Some(rule) = rule { rules.push(rule); }
        }
        for entry in entries {
            if !FileUtils::isPortableFilePath(&entry.path) { continue; }
            validatePath(&entry.path)?;
            entry.path.strip_prefix(&format!("{directory}/")).ok_or_else(|| format!("Managed entry escaped directory: {}", entry.path))?;
            if isSkipped(&entry.path, skipped) { continue; }
            if !isControl(root, &entry.path) && ignored(rules, &entry.path, entry.isDirectory) { skipped.push(entry.path); continue; }
            if entry.isDirectory { self.collect(root, &entry.path, policy, rules, files, skipped)?; }
            else {
                let Some(modifiedAt) = self.scanRead(self.host.modifiedAt(&entry.path), &entry.path, skipped)? else { continue; };
                files.insert(entry.path, FileStamp { size: entry.size, modifiedAt: Some(modifiedAt) });
            }
        }
        rules.truncate(previous);
        Ok(())
    }

    /// Serializes conflict decisions with scans and app writes. Incoming changes update
    /// the inventory so the scanner never republishes them as fresh local edits.
    pub fn applyOperation(&self, operation: &SyncOperation, bootstrap: bool) -> Result<bool, String> {
        let applied = self.materializeOperation(operation, bootstrap)?;
        self.operations().appendOperation(operation).map_err(|e| e.to_string())?;
        if !FileUtils::isPortableFilePath(&operation.entityId) {
            self.operations().ignoreFileOperations(&operation.domain, &operation.entityId).map_err(|e| e.to_string())?;
        }
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
            if !FileUtils::isPortableFilePath(&operation.entityId) {
                decisions.push(false);
                continue;
            }
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
        if !FileUtils::isPortableFilePath(path) { return Ok(()); }
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
        if self.scanRead(self.checkDestination(path), path, &mut Vec::new())?.is_none() {
            state.deferred.insert(path.to_string(), next);
            return self.save(&state);
        }
        state.deferred.remove(path);
        let content = next.as_ref().map(|snapshot: &RuntimeFileSnapshot| self.files().readBlob(&snapshot.reference)).transpose()?;
        state.pending = Some((path.to_string(), next));
        self.save(&state)?;
        self.finishPending(&mut state, content.as_deref())
    }

    /// Policy decisions precede content transfer as well as materialization.
    pub fn wantsBlob(&self, operation: &SyncOperation) -> Result<bool, String> {
        if !FileUtils::isPortableFilePath(&operation.entityId) { return Ok(false); }
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
        if !FileUtils::isPortableFilePath(path) { return Ok(None); }
        let state = self.load()?;
        if state.references.contains_key(path) { return Ok(None); }
        let root = managedDirectory(path)?;
        let policy = FileSyncPolicy::load(self.host.as_ref(), &root)?;
        if !policy.includes(self.host.as_ref(), &root, path, false)? { return Ok(None); }
        Ok(Some(if isControl(&root, path) || path.ends_with(REFERENCE_SUFFIX) { MAX_POLICY_BYTES } else { usize::try_from(policy.maxFileBytes.min(policy.maxTotalBytes)).unwrap_or(usize::MAX) }))
    }

    /// Leaves inaccessible incoming files deferred until their permissions permit access.
    fn accepts(&self, path: &str, snapshot: Option<&RuntimeFileSnapshot>) -> Result<bool, String> {
        Ok(self.scanRead(self.acceptsForScan(path, snapshot), path, &mut Vec::new())?.is_some_and(|accepted| accepted))
    }

    /// Evaluates incoming policy limits while retaining typed errors from local rules.
    fn acceptsForScan(&self, path: &str, snapshot: Option<&RuntimeFileSnapshot>) -> HostResult<bool> {
        if !FileUtils::isPortableFilePath(path) { return Ok(false); }
        let root = managedDirectory(path).map_err(HostError::new)?;
        let policy = FileSyncPolicy::loadForScan(self.host.as_ref(), &root)?;
        if !policy.includesForScan(self.host.as_ref(), &root, path, false)? { return Ok(false); }
        let control = isControl(&root, path) || path.ends_with(REFERENCE_SUFFIX);
        let limit = if control { MAX_POLICY_BYTES as u64 } else { policy.maxFileBytes };
        let Some(snapshot) = snapshot else { return Ok(true); };
        if snapshot.reference.size < 0 || snapshot.reference.size as u64 > limit { return Ok(false); }
        if control { return Ok(true); }
        let mut total = snapshot.reference.size as u64;
        let state = self.load().map_err(HostError::new)?;
        for (other, stamp) in &state.files {
            if other != path && !state.references.contains_key(other) && other.starts_with(&format!("{root}/"))
                && !isControl(&root, other) && !other.ends_with(REFERENCE_SUFFIX)
                && policy.includesForScan(self.host.as_ref(), &root, other, false)? {
                total = total.saturating_add(stamp.size.max(0) as u64);
            }
        }
        Ok(total <= policy.maxTotalBytes)
    }

    /// Completes an interrupted incoming write before any scan can publish local changes.
    fn recoverPending(&self, state: &mut ScanState) -> Result<(), String> {
        if state.pending.as_ref().is_some_and(|(path, _)| !FileUtils::isPortableFilePath(path)) {
            state.pending = None;
            return self.save(state);
        }
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
        let Some(stamp) = self.scanRead(self.writePending(&path, next.as_ref(), content), &path, &mut Vec::new())? else {
            state.deferred.insert(path, next);
            state.pending = None;
            return self.save(state);
        };
        match stamp {
            Some(stamp) => { state.files.insert(path.clone(), stamp); }
            None => {
                state.files.remove(&path);
                self.removeEmptyParents(&path)?;
            }
        }
        self.files().indexMaterializedFile(&path, next.as_ref().map(|snapshot| &snapshot.reference))?;
        state.pending = None;
        self.save(state)
    }

    /// Materializes one pending file while keeping permission failures recoverable.
    fn writePending(&self, path: &str, next: Option<&RuntimeFileSnapshot>, content: Option<&[u8]>) -> HostResult<Option<FileStamp>> {
        validateFilePath(path).map_err(HostError::new)?;
        self.checkDestination(path)?;
        match next {
            Some(reference) => {
                self.host.writeBytes(path, content.ok_or_else(|| HostError::new("Missing verified workspace file bytes"))?)?;
                if let Some(modifiedAt) = reference.modifiedAt {
                    self.host.setModifiedAt(path, modifiedAt)?;
                }
                self.host.flushFile(path)?;
                let modifiedAt = Some(self.host.modifiedAt(path)?);
                Ok(Some(FileStamp { size: reference.reference.size, modifiedAt }))
            }
            None => {
                if self.host.exists(path)? {
                    self.host.delete(path, false)?;
                }
                Ok(None)
            }
        }
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
    fn checkDestination(&self, path: &str) -> HostResult<()> {
        let parts: Vec<_> = path.split('/').collect();
        let mut parent = parts[0].to_string();
        for (index, part) in parts.iter().enumerate().skip(1) {
            let child = format!("{parent}/{part}");
            let entry = self.host.list(&parent)?.into_iter().find(|entry| entry.path == child);
            match entry {
                Some(entry) if entry.isDirectory == (index + 1 < parts.len()) => {}
                Some(_) => return Err(HostError::new(format!("Workspace file/directory conflict: {child}"))),
                None if self.host.exists(&child)? => {
                    return Err(HostError::new(format!(
                        "Workspace synchronization will not follow a link: {child}"
                    )))
                }
                None => return Ok(()),
            }
            parent = child;
        }
        Ok(())
    }
}

/// Matches an inaccessible or excluded path together with all of its descendants.
fn isSkipped(path: &str, skipped: &[String]) -> bool {
    skipped.iter().any(|prefix| path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/')))
}

/// Retains only the previous observations of paths that could not be scanned safely.
fn preserveSkipped(current: &mut Inventory, previous: &Inventory, skipped: &[String]) {
    current.retain(|path, _| !isSkipped(path, skipped));
    for (path, stamp) in previous {
        if isSkipped(path, skipped) { current.insert(path.clone(), stamp.clone()); }
    }
}

/// Returns the stable app-managed directory owning one file.
pub fn managedDirectory(path: &str) -> Result<String, String> {
    for root in MANAGED_ROOTS {
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
