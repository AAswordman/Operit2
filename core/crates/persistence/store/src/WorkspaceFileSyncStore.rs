//! Replicates managed workspace files using the existing file/blob protocol.
//! External mounts are never traversed. The durable inventory detects edits made
//! by terminals/editors and deletions made while the application was stopped.
#![allow(non_snake_case)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::RuntimeFileSyncStore::{
    RuntimeFileSyncReference, RuntimeFileSyncStore, RUNTIME_FILE_SYNC_DOMAIN,
};
use crate::SyncOperationStore::{SyncOperation, SyncOperationStore};

type Inventory = BTreeMap<String, RuntimeFileSyncReference>;
type WorkspaceLocks = Vec<(Weak<dyn RuntimeStorageHost>, String, Arc<Mutex<()>>)>;
static LOCKS: OnceLock<Mutex<WorkspaceLocks>> = OnceLock::new();

/// Steady-state scans reuse inventory references while a file's size is unchanged.
/// This interval forces a full content-hash round so same-size rewrites made by
/// terminals and external editors are eventually discovered.
const FULL_HASH_INTERVAL_MS: i64 = 300_000;

#[derive(Serialize, Deserialize, Default)]
struct ScanState {
    files: Inventory,
    // Written before materializing incoming bytes. After a crash the next scan
    // compares with the intended remote content, not the obsolete local content.
    pending: Option<(String, Option<RuntimeFileSyncReference>)>,
    // Milliseconds timestamp of the last full-content-hash round. Missing (zero)
    // in inventories written by older builds, which forces the next scan to
    // hash every file before the shortcut may be trusted.
    #[serde(default)]
    lastFullHashAt: i64,
}

pub struct WorkspaceFileSyncStore {
    host: Arc<dyn RuntimeStorageHost>,
    root: String,
    lock: Arc<Mutex<()>>,
}

impl WorkspaceFileSyncStore {
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

    pub fn owns(operation: &SyncOperation) -> bool {
        operation.domain == RUNTIME_FILE_SYNC_DOMAIN
            && operation.entityId.starts_with("workspaces/")
    }

    /// Scans only the managed root; mounted VFS directories are not consulted.
    pub fn scan(&self) -> Result<usize, String> {
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        self.scanUnlocked(&[])
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

    /// Like track, but re-hashes the given workspace file paths after the change
    /// even when their size is unchanged, so structured same-size rewrites publish
    /// immediately instead of waiting for the next full-content-hash round.
    pub fn trackPaths<T, P: AsRef<str>>(
        &self,
        paths: &[P],
        change: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        for path in paths {
            validateFilePath(path.as_ref())?;
        }
        let forcedPaths: Vec<&str> = paths.iter().map(|path| path.as_ref()).collect();
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        self.scanUnlocked(&[])?;
        let result = change();
        let scanned = self.scanUnlocked(&forcedPaths);
        match result {
            Err(error) => Err(error),
            Ok(value) => {
                scanned?;
                Ok(value)
            }
        }
    }

    fn statePath(&self) -> String {
        format!("{}/workspace_inventory.json", self.root)
    }
    fn files(&self) -> RuntimeFileSyncStore {
        RuntimeFileSyncStore::new(self.host.clone(), self.root.clone())
    }
    fn operations(&self) -> SyncOperationStore {
        SyncOperationStore::new(self.host.clone(), self.root.clone())
    }

    fn load(&self) -> Result<ScanState, String> {
        if !self
            .host
            .exists(&self.statePath())
            .map_err(|e| e.to_string())?
        {
            return Ok(ScanState::default());
        }
        let bytes = self
            .host
            .readBytes(&self.statePath())
            .map_err(|e| e.to_string())?;
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid workspace inventory: {e}"))
    }
    fn save(&self, state: &ScanState) -> Result<(), String> {
        self.host
            .writeBytes(
                &self.statePath(),
                &serde_json::to_vec(state).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())
    }

    fn scanUnlocked(&self, forcedPaths: &[&str]) -> Result<usize, String> {
        let mut state = self.load()?;
        self.recoverPending(&mut state)?;
        // A clock moving backwards is treated as an overdue full-hash round, and
        // a failed clock read (nowMs == 0) degrades to hashing every scan.
        let nowMs = operit_host_api::TimeUtils::tryCurrentTimeMillis().unwrap_or(0);
        let fullHashDue = nowMs.saturating_sub(state.lastFullHashAt) >= FULL_HASH_INTERVAL_MS
            || nowMs < state.lastFullHashAt;
        let forced: BTreeSet<&str> = forcedPaths.iter().copied().collect();
        let mut current = Inventory::new();
        if !state.files.is_empty() && !self.host.exists("workspaces").map_err(|e| e.to_string())? {
            return Err("Workspace root is unavailable; refusing to publish mass deletions".into());
        }
        // Complete a successful traversal before producing deletion tombstones.
        // One unreadable directory must never be mistaken for mass deletion.
        self.collect("workspaces", &state.files, fullHashDue, &forced, &mut current)?;
        let files = self.files();
        let mut changes = 0;
        for path in state
            .files
            .keys()
            .filter(|path| !current.contains_key(*path))
        {
            files.recordDeletion(path)?;
            changes += 1;
        }
        for (path, reference) in &current {
            if state.files.get(path) != Some(reference) {
                // collect already stored the immutable blob. Do not rewrite the live file.
                let bytes = files.readBlob(reference)?;
                files.recordSnapshot(path, &bytes, false)?;
                changes += 1;
            }
        }
        if changes > 0 || fullHashDue {
            state.files = current;
            state.lastFullHashAt = nowMs;
            self.save(&state)?;
        }
        Ok(changes)
    }

    fn collect(
        &self,
        directory: &str,
        previous: &Inventory,
        fullHash: bool,
        forced: &BTreeSet<&str>,
        files: &mut Inventory,
    ) -> Result<(), String> {
        for entry in self.host.list(directory).map_err(|e| e.to_string())? {
            validatePath(&entry.path)?;
            entry
                .path
                .strip_prefix(&format!("{directory}/"))
                .ok_or_else(|| format!("Workspace entry escaped directory: {}", entry.path))?;
            if entry.isDirectory {
                self.collect(&entry.path, previous, fullHash, forced, files)?;
            } else {
                // Collection-root housekeeping files are not part of any workspace.
                if entry.path.split('/').count() < 3 {
                    continue;
                }
                // Size is the cheap change signal: an unchanged size reuses the
                // inventory reference without reading or hashing, so a steady
                // workspace costs only directory listings. Same-size rewrites wait
                // for the next full-hash round or a trackPaths re-hash.
                if !fullHash
                    && !forced.contains(entry.path.as_str())
                    && previous
                        .get(&entry.path)
                        .is_some_and(|reference| reference.size == entry.size)
                {
                    let reference = previous[entry.path.as_str()].clone();
                    files.insert(entry.path, reference);
                    continue;
                }
                let bytes = self
                    .host
                    .readBytes(&entry.path)
                    .map_err(|e| e.to_string())?;
                let reference = reference(&bytes);
                let store = self.files();
                let blobPath = store.blobStoragePath(&reference)?;
                if !self.host.exists(&blobPath).map_err(|e| e.to_string())? {
                    store.writeBlob(&reference, &bytes)?;
                }
                files.insert(entry.path, reference);
            }
        }
        Ok(())
    }

    /// Serializes conflict decisions with scans and app writes. Incoming changes update
    /// the inventory so the scanner never republishes them as fresh local edits.
    pub fn applyOperation(
        &self,
        operation: &SyncOperation,
        bootstrap: bool,
    ) -> Result<bool, String> {
        let applied = self.materializeOperation(operation, bootstrap)?;
        self.operations()
            .appendOperation(operation)
            .map_err(|e| e.to_string())?;
        Ok(applied)
    }

    /// Materialize without advancing the vector clock ahead of the application's batch.
    pub fn materializeOperation(
        &self,
        operation: &SyncOperation,
        bootstrap: bool,
    ) -> Result<bool, String> {
        Ok(self.materializeOperations(&[operation], bootstrap)?[0])
    }

    /// Scan once per received batch, not once per file in a large workspace.
    pub fn materializeOperations(
        &self,
        batch: &[&SyncOperation],
        bootstrap: bool,
    ) -> Result<Vec<bool>, String> {
        if batch.is_empty() {
            return Ok(Vec::new());
        }
        for operation in batch {
            validateFilePath(&operation.entityId)?;
            if !Self::owns(operation) || operation.entityType != "file" {
                return Err("Invalid workspace operation".into());
            }
        }
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;
        self.scanUnlocked(&[])?;
        let operations = self.operations();
        let mut decisions = Vec::new();
        for operation in batch {
            let apply = bootstrap
                || operations
                    .shouldApplyOperation(operation)
                    .map_err(|e| e.to_string())?;
            if apply {
                self.applyUnlocked(
                    &operation.entityId,
                    &operation.operation,
                    operation.payload.clone(),
                )?;
                if bootstrap {
                    operations.recordBootstrapAppliedOperations(std::slice::from_ref(*operation))
                } else {
                    operations.recordAppliedOperation(operation)
                }
                .map_err(|e| e.to_string())?;
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
        self.scanUnlocked(&[])?;
        self.applyUnlocked(path, operation, payload)
    }

    fn applyUnlocked(
        &self,
        path: &str,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        self.checkDestination(path)?;
        let next = match operation {
            "upsert" => {
                let reference: RuntimeFileSyncReference =
                    serde_json::from_value(payload).map_err(|e| e.to_string())?;
                self.files().readBlob(&reference)?; // verify before touching live files
                Some(reference)
            }
            "delete" => None,
            other => return Err(format!("Unsupported workspace file operation: {other}")),
        };
        let mut state = self.load()?;
        state.pending = Some((path.to_string(), next));
        self.save(&state)?;
        self.recoverPending(&mut state)
    }

    fn recoverPending(&self, state: &mut ScanState) -> Result<(), String> {
        let Some((path, next)) = state.pending.clone() else {
            return Ok(());
        };
        validateFilePath(&path)?;
        self.checkDestination(&path)?;
        match next {
            Some(reference) => {
                let content = self.files().readBlob(&reference)?;
                self.host
                    .writeBytes(&path, &content)
                    .map_err(|e| e.to_string())?;
                state.files.insert(path, reference);
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

    // Remove empty descendants after tombstones, keeping the workspace root.
    fn removeEmptyParents(&self, path: &str) -> Result<(), String> {
        let mut parent = path.rsplit_once('/').map(|(parent, _)| parent);
        while let Some(directory) = parent {
            if directory.split('/').count() < 3 {
                break;
            }
            if !self.host.exists(directory).map_err(|e| e.to_string())? {
                break;
            }
            if !self
                .host
                .list(directory)
                .map_err(|e| e.to_string())?
                .is_empty()
            {
                break;
            }
            // Never recurse: hidden links or newly-created files keep the directory intact.
            if self.host.delete(directory, false).is_err() {
                break;
            }
            parent = directory.rsplit_once('/').map(|(parent, _)| parent);
        }
        Ok(())
    }

    // list() omits links on native hosts. An existing but unlisted component is
    // therefore not a writable sync destination (including ancestor links).
    fn checkDestination(&self, path: &str) -> Result<(), String> {
        let parts: Vec<_> = path.split('/').collect();
        let mut parent = "workspaces".to_string();
        for (index, part) in parts.iter().enumerate().skip(1) {
            let child = format!("{parent}/{part}");
            let entry = self
                .host
                .list(&parent)
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|entry| entry.path == child);
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

fn validatePath(path: &str) -> Result<(), String> {
    if !path.starts_with("workspaces/")
        || path.split('/').count() < 2
        || runtimeStorageOwnership(path)? != RuntimeStorageOwnership::Space
    {
        return Err(format!("Not a managed workspace path: {path}"));
    }
    Ok(())
}

fn validateFilePath(path: &str) -> Result<(), String> {
    validatePath(path)?;
    if path.split('/').count() < 3 {
        return Err("Workspace file must belong to a workspace directory".into());
    }
    Ok(())
}

fn reference(bytes: &[u8]) -> RuntimeFileSyncReference {
    RuntimeFileSyncReference {
        contentHash: format!("{:x}", Sha256::digest(bytes)),
        size: bytes.len() as i64,
    }
}
