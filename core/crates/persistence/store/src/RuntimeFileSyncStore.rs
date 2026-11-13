#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_util::FileUtils::FileUtils;
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::SyncOperationStore::{
    NewSyncOperation, SyncOperation, SyncOperationSemantics, SyncOperationStore,
};

pub const RUNTIME_FILE_SYNC_DOMAIN: &str = "runtime_file";
const RUNTIME_FILE_SYNC_ENTITY_TYPE: &str = "file";

pub use crate::SyncBlobIo::RuntimeFileSyncReference;
pub(crate) use crate::SyncBlobIo::{validateBlobReference,validateContentHash,verifyBlob};

/// File state carries metadata separately from content-addressed transport references.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct RuntimeFileSnapshot {
    #[serde(flatten)]
    pub reference: RuntimeFileSyncReference,
    // Historical originals did not record a source modification time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifiedAt: Option<i64>,
}

/// Records and applies synchronized snapshots for explicitly registered runtime files.
#[derive(Clone)]
pub struct RuntimeFileSyncStore {
    storageHost: Arc<dyn RuntimeStorageHost>,
    syncRootPath: String,
    syncOperationStore: SyncOperationStore,
}

/// The losing side supplies coverage without resending its business contents.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCoverage {
    pub owner: crate::Transaction::Owner,
    pub covered: std::collections::BTreeSet<String>,
}

/// The winning file plus the losing history coverage it supersedes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileCorrectionReference {
    pub source: RuntimeFileSyncReference,
    pub superseded: Option<RuntimeFileSyncReference>,
    /// Exact failed originals accepted by this correction, never a time cutoff.
    #[serde(default, skip_serializing_if = "std::collections::BTreeSet::is_empty")]
    pub skipped: std::collections::BTreeSet<String>,
}

impl RuntimeFileSyncStore {
    /// Ordinary file operations are complete replacement states, including deletion tombstones.
    pub fn isCompleteState(operation: &SyncOperation) -> bool {
        operation.domain == RUNTIME_FILE_SYNC_DOMAIN && operation.entityType == RUNTIME_FILE_SYNC_ENTITY_TYPE
            && operation.semantics == SyncOperationSemantics::EntityState && matches!(operation.operation.as_str(), "upsert" | "delete")
    }

    /// Creates a runtime file synchronization store over one host and operation root.
    pub fn new(storageHost: Arc<dyn RuntimeStorageHost>, syncRootPath: impl Into<String>) -> Self {
        let syncRootPath = syncRootPath.into();
        Self {
            syncOperationStore: SyncOperationStore::new(storageHost.clone(), syncRootPath.clone()),
            storageHost,
            syncRootPath,
        }
    }

    /// Writes a complete Space file snapshot and records its synchronization operation.
    pub fn writeBytes(&self, storagePath: &str, content: &[u8]) -> Result<(), String> {
        self.writeSnapshot(storagePath, content, false)
    }

    /// Seeds an unseen file below explicit edits and deletion tombstones in conflict order.
    pub fn initializeBytes(&self, storagePath: &str, content: &[u8]) -> Result<(), String> {
        self.writeSnapshot(storagePath, content, true)
    }

    /// Reports whether this file has synchronized state, even when it has been deleted.
    pub fn hasSnapshot(&self, storagePath: &str) -> Result<bool, String> {
        self.syncOperationStore.hasEntityVersion(RUNTIME_FILE_SYNC_DOMAIN, RUNTIME_FILE_SYNC_ENTITY_TYPE, storagePath)
            .map_err(|error| error.to_string())
    }

    /// Writes explicit changes and initial seeds through the same file synchronization path.
    fn writeSnapshot(&self, storagePath: &str, content: &[u8], initial: bool) -> Result<(), String> {
        if initial && self.hasSnapshot(storagePath)? {
            return Ok(());
        }
        requireSpaceFile(storagePath)?;
        if crate::PluginStorage::ownsFile(self.storageHost.clone(),storagePath).map_err(|e|e.to_string())? {
            return Err("Registered database files require record-level storage operations".into());
        }
        if crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::manages(storagePath) {
            return crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::new(self.storageHost.clone(), self.syncRootPath.clone())
                .trackPaths(&[storagePath], || self.storageHost.writeBytes(storagePath, content).map_err(|e| e.to_string()));
        }
        self.storageHost
            .writeBytes(storagePath, content)
            .map_err(|error| error.to_string())?;
        if FileUtils::isPortableFilePath(storagePath) { self.recordSnapshot(storagePath, content, initial)?; }
        Ok(())
    }

    /// Records observed bytes without rewriting an externally edited workspace file.
    pub(crate) fn recordSnapshot(&self, storagePath: &str, content: &[u8], initial: bool) -> Result<RuntimeFileSyncReference, String> {
        requireSpaceFile(storagePath)?;
        if crate::PluginStorage::ownsFile(self.storageHost.clone(),storagePath).map_err(|e|e.to_string())? {
            return Err("Registered database files require record-level storage operations".into());
        }
        let reference = RuntimeFileSyncReference { contentHash: contentHash(content), size: i64::try_from(content.len()).map_err(|_| "File is too large")? };
        let mut deviceId = self.syncOperationStore.localDeviceId().map_err(|error| error.to_string())?;
        let modifiedAt = Some(self.storageHost.modifiedAt(storagePath).map_err(|e| e.to_string())?);
        let mut payload = serde_json::to_value(RuntimeFileSnapshot { reference: reference.clone(), modifiedAt }).map_err(|e| e.to_string())?;
        if initial {
            deviceId.push_str(":initial");
            payload["initial"] = true.into();
        }
        self.syncOperationStore.appendLocalOperation(&deviceId, NewSyncOperation {
            domain: RUNTIME_FILE_SYNC_DOMAIN.to_string(),
            entityType: RUNTIME_FILE_SYNC_ENTITY_TYPE.to_string(),
            entityId: storagePath.to_string(),
            operation: "upsert".to_string(),
            semantics: SyncOperationSemantics::EntityState,
            payload,
        }).map_err(|e|e.to_string())?;
        Ok(reference)
    }

    /// Republishes current bytes and tombstones before retiring the captured file history.
    pub fn resetTransactions(&self) -> Result<usize, String> {
        crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::new(self.storageHost.clone(), self.syncRootPath.clone()).track(|| {
            let originals = self.syncOperationStore.withJournal(|journal|journal.headers(None)).map_err(|e|e.to_string())?
                .into_iter().filter(|record|record.domain == RUNTIME_FILE_SYNC_DOMAIN).collect::<Vec<_>>();
            let paths = originals.iter().map(|record|record.entity_id.clone()).collect::<std::collections::BTreeSet<_>>();
            let mut replaced = std::collections::BTreeSet::new();
            let managed = crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::new(self.storageHost.clone(), self.syncRootPath.clone());
            for path in paths {
                if !FileUtils::isPortableFilePath(&path) || requireSpaceFile(&path).is_err() || crate::PluginStorage::ownsFile(self.storageHost.clone(),&path).map_err(|e|e.to_string())? { continue; }
                let limit = if crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::manages(&path) {
                    match managed.captureLimit(&path)? { Some(limit) => Some(limit), None => continue }
                } else { None };
                if self.storageHost.exists(&path).map_err(|e|e.to_string())? {
                    let bytes = match limit {
                        Some(limit) => crate::FileSyncPolicy::readLimited(self.storageHost.as_ref(), &path, limit)?,
                        None => self.storageHost.readBytes(&path).map_err(|e|e.to_string())?,
                    };
                    self.recordSnapshot(&path, &bytes, false)?;
                } else { self.recordDeletion(&path)?; }
                replaced.insert(path);
            }
            self.syncOperationStore.withJournal(|journal|journal.transaction(|tx|crate::Transaction::Journal::retire_ids(tx,
                originals.into_iter().filter(|record| replaced.contains(&record.entity_id)).map(|record|record.op_id)))).map_err(|e|e.to_string())?;
            let count = replaced.len();
            Ok(count)
        })
    }

    /// Appends bytes to a Space file and records the resulting complete snapshot.
    pub fn appendBytes(&self, storagePath: &str, content: &[u8]) -> Result<(), String> {
        requireSpaceFile(storagePath)?;
        if crate::PluginStorage::ownsFile(self.storageHost.clone(),storagePath).map_err(|e|e.to_string())? {
            return Err("Registered database files require record-level storage operations".into());
        }
        let mut next = if self
            .storageHost
            .exists(storagePath)
            .map_err(|error| error.to_string())?
        {
            self.storageHost
                .readBytes(storagePath)
                .map_err(|error| error.to_string())?
        } else {
            Vec::new()
        };
        next.extend_from_slice(content);
        self.writeBytes(storagePath, &next)
    }

    /// Deletes a Space file and records a synchronized tombstone.
    pub fn delete(&self, storagePath: &str) -> Result<(), String> {
        requireSpaceFile(storagePath)?;
        if crate::PluginStorage::ownsFile(self.storageHost.clone(),storagePath).map_err(|e|e.to_string())? {
            return Err("Registered database files require record-level storage operations".into());
        }
        if crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::manages(storagePath) {
            return crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::new(self.storageHost.clone(), self.syncRootPath.clone())
                .track(|| {
                    if self.storageHost.exists(storagePath).map_err(|e| e.to_string())? {
                        self.storageHost.delete(storagePath, false).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                });
        }
        if self.storageHost.exists(storagePath).map_err(|e|e.to_string())? {
            self.storageHost.delete(storagePath, false).map_err(|e|e.to_string())?;
        }
        self.recordDeletion(storagePath)
    }

    /// Records an observed deletion without deleting a possibly recreated local file.
    pub(crate) fn recordDeletion(&self, storagePath: &str) -> Result<(), String> {
        requireSpaceFile(storagePath)?;
        if !FileUtils::isPortableFilePath(storagePath) { return Ok(()); }
        if crate::PluginStorage::ownsFile(self.storageHost.clone(),storagePath).map_err(|e|e.to_string())? {
            return Err("Registered database files require record-level storage operations".into());
        }
        let deviceId = self.syncOperationStore.localDeviceId().map_err(|e|e.to_string())?;
        self.syncOperationStore.appendLocalOperation(&deviceId, NewSyncOperation {
            domain: RUNTIME_FILE_SYNC_DOMAIN.to_string(),
            entityType: RUNTIME_FILE_SYNC_ENTITY_TYPE.to_string(),
            entityId: storagePath.to_string(),
            operation: "delete".to_string(),
            semantics: SyncOperationSemantics::EntityState,
            payload: serde_json::Value::Null,
        }).map(|_|()).map_err(|e|e.to_string())
    }

    /// Tracks VFS mutations using declared storage ownership, without knowledge of file consumers.
    /// Capturing before the operation preserves tombstones for recursive deletion and moves.
    pub fn trackChanges<T>(&self, paths: &[String], change: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let tracked = || {
            let before = self.captureFiles(paths)?;
            let result = change();
            let after = self.captureFiles(paths)?;
            for (path, stamp) in &after {
                if before.get(path) != Some(stamp) {
                    self.recordSnapshot(path, &self.storageHost.readBytes(path).map_err(|e|e.to_string())?, false)?;
                }
            }
            for path in before.keys().filter(|path| !after.contains_key(*path)) { self.recordDeletion(path)?; }
            result
        };
        // Only edits intersecting managed roots acquire their scanner lock and
        // recover pending workspace operations. Unrelated local files must not
        // depend on the availability or recovery state of shared workspaces.
        if paths.iter().any(|path| crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::affectedBy(path)) {
            crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::new(self.storageHost.clone(), self.syncRootPath.clone()).track(tracked)
        } else {
            tracked()
        }
    }

    /// Captures metadata for registered files under the requested VFS paths.
    fn captureFiles(&self, paths: &[String]) -> Result<std::collections::BTreeMap<String, (i64, i64)>, String> {
        let mut files = std::collections::BTreeMap::new();
        for path in paths {
            if !FileUtils::isPortableFilePath(path) { continue; }
            if !self.storageHost.exists(path).map_err(|e|e.to_string())? { continue; }
            let Some((parent, _)) = path.rsplit_once('/') else {
                for entry in self.storageHost.list(path).map_err(|e|e.to_string())? { self.captureEntry(entry, &mut files)?; }
                continue;
            };
            let entry = self.storageHost.list(parent).map_err(|e|e.to_string())?.into_iter()
                .find(|entry| entry.path == *path).ok_or("Missing runtime file entry")?;
            self.captureEntry(entry, &mut files)?;
        }
        Ok(files)
    }

    /// Visits each listed entry once, excluding registered databases and device-local files.
    fn captureEntry(&self, entry: operit_host_api::RuntimeStorageEntry, files: &mut std::collections::BTreeMap<String, (i64, i64)>) -> Result<(), String> {
        let path = &entry.path;
        if !FileUtils::isPortableFilePath(path) { return Ok(()); }
        if crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::manages(path) { return Ok(()); }
        if crate::PluginStorage::ownsFile(self.storageHost.clone(), path).map_err(|e|e.to_string())? { return Ok(()); }
        if entry.isDirectory {
            for child in self.storageHost.list(path).map_err(|e|e.to_string())? {
                if !child.path.starts_with(&format!("{path}/")) { return Err("Storage entry escaped directory".into()); }
                self.captureEntry(child, files)?;
            }
        } else if runtimeStorageOwnership(path) == Ok(RuntimeStorageOwnership::Space) {
            files.insert(path.clone(), (entry.size, self.storageHost.modifiedAt(path).map_err(|e|e.to_string())?));
        }
        Ok(())
    }

    /// Applies one validated runtime file operation without recording another local mutation.
    pub fn applySyncedOperation(
        storageHost: Arc<dyn RuntimeStorageHost>,
        syncRootPath: impl Into<String>,
        storagePath: &str,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        requireSpaceFile(storagePath)?;
        if !FileUtils::isPortableFilePath(storagePath) { return Ok(()); }
        if crate::PluginStorage::ownsFile(storageHost.clone(),storagePath).map_err(|e|e.to_string())? {
            return Err("Registered database files require record-level storage operations".into());
        }
        let syncRootPath = syncRootPath.into();
        if crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::manages(storagePath) {
            return crate::WorkspaceFileSyncStore::WorkspaceFileSyncStore::new(storageHost, syncRootPath)
                .apply(storagePath, operation, payload);
        }
        let store = Self::new(storageHost.clone(), syncRootPath);
        match operation {
            "upsert" => {
                let snapshot: RuntimeFileSnapshot = serde_json::from_value(payload).map_err(|e| e.to_string())?;
                let content = store.readBlob(&snapshot.reference)?;
                storageHost.writeBytes(storagePath, &content).map_err(|e| e.to_string())?;
                if let Some(modifiedAt) = snapshot.modifiedAt { storageHost.setModifiedAt(storagePath, modifiedAt).map_err(|e| e.to_string())?; }
                storageHost.flushFile(storagePath).map_err(|e| e.to_string())?;
                store.indexMaterializedFile(storagePath, Some(&snapshot.reference))
            }
            "delete" => {
                if storageHost.exists(storagePath).map_err(|e|e.to_string())? {
                    storageHost.delete(storagePath, false).map_err(|e|e.to_string())?;
                }
                store.indexMaterializedFile(storagePath, None)
            }
            other => Err(format!("unsupported runtime file sync operation: {other}")),
        }
    }

    /// Returns the content blob required by a transported operation or runtime-file upsert.
    pub fn requiredBlob(
        operation: &SyncOperation,
    ) -> Result<Option<RuntimeFileSyncReference>, String> {
        if operation.domain == RUNTIME_FILE_SYNC_DOMAIN && !FileUtils::isPortableFilePath(&operation.entityId) { return Ok(None); }
        if let Some(reference) = crate::SyncBlobIo::transaction_reference(operation)? {
            return Ok(Some(reference));
        }
        if operation.domain != RUNTIME_FILE_SYNC_DOMAIN {
            return Ok(None);
        }
        if runtimeStorageOwnership(&operation.entityId) != Ok(RuntimeStorageOwnership::Space) { return Ok(None); }
        if operation.entityType != RUNTIME_FILE_SYNC_ENTITY_TYPE {
            return Err(format!(
                "unsupported runtime file entity type: {}",
                operation.entityType
            ));
        }
        match operation.operation.as_str() {
            "upsert" => serde_json::from_value(operation.payload.clone())
                .map(Some)
                .map_err(|error| error.to_string()),
            "delete" => Ok(None),
            other => Err(format!("unsupported runtime file sync operation: {other}")),
        }
    }

    /// Sends large immutable originals through the existing verified blob transport.
    pub fn prepareTransactionForTransport(&self, mut operation: SyncOperation) -> Result<SyncOperation,String> {
        if crate::SyncBlobIo::transaction_reference(&operation)?.is_none() {
            let (hash, size) = crate::Transaction::serialized_digest(&operation)?;
            if size > 32 * 1024 {
                let reference = self.storeJsonBlobWithDigest(&operation, hash, size)?;
                operation.payload = serde_json::json!({"transactionBlob":reference});
            }
        }
        Ok(operation)
    }

    /// Restores the verified original, leaving SQL initializers to their streaming reader.
    pub fn hydrateTransaction(&self, operation: SyncOperation) -> Result<SyncOperation,String> {
        if operation.domain == RUNTIME_FILE_SYNC_DOMAIN && !FileUtils::isPortableFilePath(&operation.entityId) { return Ok(operation); }
        let Some(reference) = crate::SyncBlobIo::transaction_reference(&operation)? else { return Ok(operation); };
        let initialization = operation.operation == "initialize" && operation.entityType == "initialization";
        if initialization && matches!(operation.domain.as_str(), "chat"|"plugin_storage") { return Ok(operation); }
        let mut original: SyncOperation = self.readJsonBlob(&reference)?;
        // Only the tiny transport payload changes. Do not clone the complete transaction to compare headers.
        let payload = std::mem::replace(&mut original.payload, operation.payload.clone());
        let matches = original == operation;
        original.payload = payload;
        if !matches || (initialization && !crate::Transaction::compaction::Initialization::validate_identity(&original)?) {
            return Err("Transaction blob differs from its envelope".into());
        }
        Ok(original)
    }

    pub(crate) fn storeStream(&self,reference:&RuntimeFileSyncReference,reader:&mut dyn std::io::Read)->Result<(),String> {
        crate::SyncBlobIo::write_stream(self.storageHost.as_ref(),&self.blobStoragePath(reference)?,reference,|writer|{
            std::io::copy(reader,writer).map(|_|()).map_err(|e|e.to_string())
        })
    }

    pub fn readStagedJson<T:serde::de::DeserializeOwned>(&self, reference:&RuntimeFileSyncReference) -> Result<(T,std::sync::Arc<crate::Transaction::sql_snapshot::SqlSnapshot>),String> {
        crate::Transaction::sql_snapshot::read_staged(self.storageHost.clone(),&self.blobReadPath(reference)?,reference)
    }

    pub fn readSqlInitialization(&self, operation:&SyncOperation) -> Result<crate::Transaction::compaction::Initialization,String> {
        let reference=Self::requiredBlob(operation)?.ok_or("Missing SQL initialization blob")?;
        let (original,sql)=self.readStagedJson::<SyncOperation>(&reference)?;
        crate::Transaction::sql_snapshot::validate_initialization(operation,original,sql)
    }

    /// Checks local blob presence without reading or hashing its contents.
    pub fn hasBlob(&self, reference: &RuntimeFileSyncReference) -> Result<bool, String> {
        validateBlobReference(reference)?;
        Ok(self.fileSource(&reference.contentHash)?.is_some() || self.storageHost.exists(&self.blobStoragePath(reference)?).map_err(|e|e.to_string())?)
    }

    /// Reads one bounded range from a verified content-addressed blob.
    pub fn readBlobChunk(&self, contentHash: &str, offset: i64, length: i64) -> Result<Vec<u8>, String> {
        let offset = u64::try_from(offset).map_err(|_| "blob chunk offset must not be negative")?;
        let length = usize::try_from(length).map_err(|_| "blob chunk length must not be negative or overflow")?;
        crate::SyncBlobIo::validateContentHash(contentHash)?;
        self.syncOperationStore.withJournal(|journal|journal.lease_blob(contentHash)).map_err(|e|e.to_string())?;
        let source = self.fileSource(contentHash)?;
        let path = source.as_ref().map(|source| source.path.clone()).unwrap_or(self.blobPath(contentHash)?);
        let bytes = self.storageHost.readBytesRange(&path, offset, length).map_err(|e|e.to_string())?;
        if source.is_some_and(|source| !source.current(self.storageHost.as_ref()).unwrap_or(false)) {
            return Err("File changed during synchronization; refresh its index".into());
        }
        Ok(bytes)
    }

    /// Verifies and persists one complete content-addressed blob.
    pub fn writeBlob(
        &self,
        reference: &RuntimeFileSyncReference,
        content: &[u8],
    ) -> Result<(), String> {
        verifyBlob(reference, content)?;
        self.storageHost
            .writeBytes(&self.blobPath(&reference.contentHash)?, content)
            .map_err(|error| error.to_string())
    }

    /// Resolves the verified host storage path for one content-addressed blob.
    pub fn blobStoragePath(&self, reference: &RuntimeFileSyncReference) -> Result<String, String> {
        validateBlobReference(reference)?;
        self.syncOperationStore.withJournal(|journal|journal.lease_blob(&reference.contentHash)).map_err(|e|e.to_string())?;
        self.blobPath(&reference.contentHash)
    }

    /// Persists local bytes and returns their verified content identity.
    pub fn storeBlob(&self, content: &[u8]) -> Result<RuntimeFileSyncReference, String> {
        let size = i64::try_from(content.len())
            .map_err(|_| "runtime file size does not fit i64".to_string())?;
        let reference = RuntimeFileSyncReference {
            contentHash: contentHash(content),
            size,
        };
        // Locally captured bytes already define this hash; remote writes still verify their references.
        self.storageHost.writeBytes(&self.blobPath(&reference.contentHash)?, content).map_err(|e|e.to_string())?;
        Ok(reference)
    }

    /// Encodes directly into a private host file instead of constructing a second JSON buffer.
    pub fn storeJsonBlob<T: serde::Serialize>(&self, value: &T) -> Result<RuntimeFileSyncReference, String> {
        let (hash,size) = crate::Transaction::serialized_digest(value)?;
        self.storeJsonBlobWithDigest(value,hash,size)
    }

    fn storeJsonBlobWithDigest<T: serde::Serialize>(&self, value: &T, contentHash: String, size: u64) -> Result<RuntimeFileSyncReference, String> {
        let reference = RuntimeFileSyncReference { contentHash, size: i64::try_from(size).map_err(|_|"Synchronization blob is too large")? };
        crate::SyncBlobIo::write_json(self.storageHost.as_ref(), &self.blobStoragePath(&reference)?, value, &reference)?;
        Ok(reference)
    }

    /// Verifies all source bytes while decoding; only the decoded object and a fixed I/O buffer coexist.
    pub fn readJsonBlob<T: serde::de::DeserializeOwned>(&self, reference: &RuntimeFileSyncReference) -> Result<T, String> {
        crate::SyncBlobIo::read_json(self.storageHost.as_ref(), &self.blobReadPath(reference)?, reference)
    }

    /// Reads and verifies one complete content-addressed blob.
    pub fn readBlob(&self, reference: &RuntimeFileSyncReference) -> Result<Vec<u8>, String> {
        validateBlobReference(reference)?;
        let content = self
            .storageHost
            .readBytes(&self.blobReadPath(reference)?)
            .map_err(|error| format!("blob {}: {error}", reference.contentHash))?;
        verifyBlob(reference, &content)?;
        Ok(content)
    }

    /// Materialized remote bytes replace their temporary blob with the same live-file index.
    pub(crate) fn indexMaterializedFile(&self, path: &str, reference: Option<&RuntimeFileSyncReference>) -> Result<(), String> {
        requireSpaceFile(path)?;
        let source = reference.map(|reference| self.storageHost.modifiedAt(path).map(|modified_at|
            crate::Transaction::FileSource { path: path.into(), reference: reference.clone(), modified_at })).transpose().map_err(|e|e.to_string())?;
        self.syncOperationStore.withJournal(|journal| journal.transaction(|tx| match &source {
            Some(source) => crate::Transaction::Journal::index_file(tx, source),
            None => crate::Transaction::Journal::unindex_file(tx, path),
        })).map_err(|e|e.to_string())?;
        self.syncOperationStore.collectBlobs(None);
        Ok(())
    }

    fn fileSource(&self, hash: &str) -> Result<Option<crate::Transaction::FileSource>, String> {
        validateContentHash(hash)?;
        Ok(self.syncOperationStore.withJournal(|journal| journal.file_sources(hash)).map_err(|e|e.to_string())?
            .into_iter().find(|source| source.current(self.storageHost.as_ref()).unwrap_or(false)))
    }

    fn blobReadPath(&self, reference: &RuntimeFileSyncReference) -> Result<String, String> {
        validateBlobReference(reference)?;
        self.syncOperationStore.withJournal(|journal|journal.lease_blob(&reference.contentHash)).map_err(|e|e.to_string())?;
        Ok(self.fileSource(&reference.contentHash)?.map(|source| source.path).unwrap_or(self.blobPath(&reference.contentHash)?))
    }

    /// Resolves one validated blob hash below the private synchronization root.
    fn blobPath(&self, contentHash: &str) -> Result<String, String> {
        validateContentHash(contentHash)?;
        Ok(format!("{}/blobs/{contentHash}", self.syncRootPath))
    }
}

/// Computes the lowercase SHA-256 identity of one byte sequence.
fn contentHash(content: &[u8]) -> String {
    format!("{:x}", Sha256::digest(content))
}


/// Rejects an operation whose entity is outside the explicit Space file registry.
pub(crate) fn requireSpaceFile(storagePath: &str) -> Result<(), String> {
    if runtimeStorageOwnership(storagePath)? != RuntimeStorageOwnership::Space {
        return Err(format!(
            "runtime file is not registered for Space synchronization: {storagePath}"
        ));
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    #[test]
    fn local_model_files_cannot_enter_space_file_sync() {
        for path in ["runtime/models/local/chat/llama_cpp/qwen/model.gguf", "runtime/models/local/chat/mnn/qwen/llm.mnn.weight", operit_util::RuntimeStorageLayout::RUNTIME_LOCAL_MODEL_REGISTRY_PATH] {
            assert!(super::requireSpaceFile(path).is_err(), "{path}");
        }
    }

    use std::sync::Arc;
    use operit_util::RuntimeStorageLayout::{
        RUNTIME_CLIENT_LOG_PATH, RUNTIME_SYNC_DIR_PATH, RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH,
    };
    use super::*;
    use crate::SyncOperationStore::SyncClock;

    // Real range reads and atomic staging are part of the blob memory contract.
    struct TestDirectory(std::path::PathBuf);
    impl Drop for TestDirectory {
        fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
    }
    fn storage() -> (TestDirectory, Arc<operit_host_native_storage::NativeRuntimeStorageHost>) {
        let root = std::env::temp_dir().join(format!("operit-file-sync-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        (TestDirectory(root), host)
    }

    #[test]
    fn reset_file_transactions_recaptures_current_bytes_and_deletions() {
        let (_directory, host) = storage();
        let files = RuntimeFileSyncStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
        let deleted = operit_util::RuntimeStorageLayout::RUNTIME_WEBSESSION_BROWSER_BOOKMARKS_PATH;
        files.writeBytes(RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH,b"old").unwrap();
        files.writeBytes(deleted,b"deleted").unwrap();
        files.delete(deleted).unwrap();
        host.writeBytes(RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH,b"current external content").unwrap();
        let journal = SyncOperationStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
        let before = journal.operationsSince(&SyncClock::empty(),&[],usize::MAX).unwrap();
        assert_eq!(files.resetTransactions().unwrap(),2);
        let after = journal.operationsSince(&SyncClock::empty(),&[],usize::MAX).unwrap();
        assert_eq!(after.len(),2);
        assert!(before.iter().all(|old|after.iter().all(|new|old.opId != new.opId)));
        let upsert = after.iter().find(|op|op.operation == "upsert").unwrap();
        assert_eq!(files.readBlob(&RuntimeFileSyncStore::requiredBlob(upsert).unwrap().unwrap()).unwrap(),b"current external content");
        assert!(after.iter().any(|op|op.entityId == deleted && op.operation == "delete"));
        assert_eq!(host.readBytes(RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH).unwrap(),b"current external content");
        journal.receiveOperations(&before).unwrap();
        assert!(journal.pendingOperations().unwrap().is_empty());
    }

    /// Verifies upsert and delete operations reproduce the same registered file on another host.
    #[test]
    fn synchronizes_registered_file_snapshots_and_tombstones() {
        let (_source_directory, source) = storage();
        let (_target_directory, target) = storage();
        let path = RUNTIME_WEBSESSION_BROWSER_HISTORY_PATH;
        let sourceStore = RuntimeFileSyncStore::new(source.clone(), RUNTIME_SYNC_DIR_PATH);
        sourceStore
            .writeBytes(path, b"[{\"url\":\"https://operit.app\"}]")
            .unwrap();

        let operationStore = SyncOperationStore::new(source, RUNTIME_SYNC_DIR_PATH);
        let operations = operationStore
            .operationsSince(
                &SyncClock::empty(),
                &[RUNTIME_FILE_SYNC_DOMAIN.to_string()],
                16,
            )
            .unwrap();
        assert_eq!(operations.len(), 1);
        let reference = RuntimeFileSyncStore::requiredBlob(&operations[0])
            .unwrap()
            .expect("runtime file upsert must reference a blob");
        let content = sourceStore.readBlob(&reference).unwrap();
        RuntimeFileSyncStore::new(target.clone(), RUNTIME_SYNC_DIR_PATH)
            .writeBlob(&reference, &content)
            .unwrap();
        RuntimeFileSyncStore::applySyncedOperation(
            target.clone(),
            RUNTIME_SYNC_DIR_PATH,
            &operations[0].entityId,
            &operations[0].operation,
            operations[0].payload.clone(),
        )
        .unwrap();
        assert_eq!(
            target.readBytes(path).unwrap(),
            b"[{\"url\":\"https://operit.app\"}]"
        );

        sourceStore.delete(path).unwrap();
        let deleteOperation = operationStore
            .operationsSince(
                &SyncClock::empty(),
                &[RUNTIME_FILE_SYNC_DOMAIN.to_string()],
                16,
            )
            .unwrap()
            .into_iter()
            .find(|operation| operation.operation == "delete")
            .expect("runtime file tombstone must be exported");
        RuntimeFileSyncStore::applySyncedOperation(
            target.clone(),
            RUNTIME_SYNC_DIR_PATH,
            &deleteOperation.entityId,
            &deleteOperation.operation,
            deleteOperation.payload,
        )
        .unwrap();
        assert!(!target.exists(path).unwrap());
    }

    /// Verifies a node-local file cannot enter the runtime file synchronization log.
    #[test]
    fn rejects_node_local_files() {
        let (_directory, host) = storage();
        let store = RuntimeFileSyncStore::new(host, RUNTIME_SYNC_DIR_PATH);
        assert!(store.writeBytes(RUNTIME_CLIENT_LOG_PATH, b"log").is_err());
    }
    /// Blob transport preserves ordinary and initialization identities under corrupt or forged delivery.
    #[test]
    fn large_operations_use_verified_blobs_and_preserve_their_immutable_envelope() {
        use crate::Transaction::{Backend,Owner,Data,Snapshot,compaction::Initialization};
        let (_source_directory, source_host) = storage();
        let (_target_directory, target_host) = storage();
        let source = RuntimeFileSyncStore::new(source_host,RUNTIME_SYNC_DIR_PATH);
        let target = RuntimeFileSyncStore::new(target_host,RUNTIME_SYNC_DIR_PATH);
        let mut data = Data::empty(Owner {backend:Backend::DataStore,path:"runtime/config/preferences/blob.preferences.json".into(),format:1,layout:1});
        data.values.insert("large".into(),serde_json::json!("content".repeat(20_000)));
        let original = Initialization::from_snapshot(&Snapshot {damage:None,node:"a".into(),data,covered:Default::default(),generation:0}).operation("preferences",false).unwrap();
        let mut chat = original.clone();
        chat.domain = "chat".into(); chat.entityType = "message".into(); chat.operation = "upsert".into();
        let mut plugin = chat.clone(); plugin.domain = "plugin_storage".into();
        for original in [original, chat, plugin] {
            let wire = source.prepareTransactionForTransport(original.clone()).unwrap();
            assert_eq!(source.prepareTransactionForTransport(wire.clone()).unwrap(), wire);
            assert_eq!(crate::Transaction::operation_digest(&wire).unwrap(), crate::Transaction::operation_digest(&original).unwrap());
            assert!(serde_json::to_vec(&wire).unwrap().len() < 1024);
            let reference = RuntimeFileSyncStore::requiredBlob(&wire).unwrap().unwrap();
            assert!(target.hydrateTransaction(wire.clone()).is_err());
            target.writeBlob(&reference,&source.readBlob(&reference).unwrap()).unwrap();
            assert_eq!(target.hydrateTransaction(wire.clone()).unwrap(),original);
            let path = target.blobStoragePath(&reference).unwrap();
            let bytes = source.readBlob(&reference).unwrap();
            for corrupt in [bytes[..bytes.len()-1].to_vec(), [bytes.as_slice(), b" "].concat(), [bytes.as_slice(), b"{}"].concat()] {
                target.storageHost.writeBytes(&path, &corrupt).unwrap();
                assert!(target.hydrateTransaction(wire.clone()).is_err());
            }
            target.writeBlob(&reference,&bytes).unwrap();
            let mut forged = wire; forged.createdAt += 1;
            assert!(target.hydrateTransaction(forged).is_err());
            target.storageHost.writeBytes(&target.blobStoragePath(&reference).unwrap(), &vec![0; reference.size as usize]).unwrap();
            assert!(target.hasBlob(&reference).unwrap());
            assert!(target.readBlob(&reference).is_err());
        }
    }

    #[test]
    fn changing_json_never_publishes_a_partial_blob() {
        struct Changing(std::cell::Cell<bool>);
        impl serde::Serialize for Changing {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok,S::Error> {
                serializer.serialize_str(&if self.0.replace(true) { "b" } else { "a" }.repeat(128 * 1024))
            }
        }
        let (_directory, host) = storage();
        let files = RuntimeFileSyncStore::new(host.clone(), RUNTIME_SYNC_DIR_PATH);
        assert!(files.storeJsonBlob(&Changing(std::cell::Cell::new(false))).is_err());
        assert!(host.list(&format!("{RUNTIME_SYNC_DIR_PATH}/blobs")).unwrap().is_empty());
    }

}

#[cfg(test)]
#[path = "tests/FileContentIndexTests.rs"]
mod file_content_index_tests;
