//! Live files back content hashes; only received or otherwise unmaterialized bytes need a blob.
use super::{journal::{Journal, text}, Revision};
use crate::SyncBlobIo::{RuntimeFileSyncReference, validateBlobReference};
use crate::SyncOperationStore::{SyncOperation, SyncOperationStoreError as Error};
use operit_host_api::{RuntimeSqliteTransaction, RuntimeStorageHost, SqliteValue};
use std::collections::BTreeSet;

pub(super) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sync_file_sources(path TEXT PRIMARY KEY, hash TEXT NOT NULL, size INTEGER NOT NULL, modified_at INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS sync_file_hash ON sync_file_sources(hash);
CREATE TABLE IF NOT EXISTS sync_file_garbage(hash TEXT PRIMARY KEY, after_ms INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS sync_blob_references(hash TEXT NOT NULL, operation_id TEXT NOT NULL, PRIMARY KEY(hash,operation_id));
CREATE INDEX IF NOT EXISTS sync_blob_operations ON sync_blob_references(operation_id);
CREATE TABLE IF NOT EXISTS sync_owner_blob_references(hash TEXT NOT NULL,path TEXT NOT NULL,PRIMARY KEY(hash,path));";
// Allow a page already exported to finish transferring before its old source is collected.
const TRANSFER_GRACE_MS: i64 = 10 * 60 * 1000;

#[derive(Clone)]
pub(crate) struct FileSource {
    pub path: String,
    pub reference: RuntimeFileSyncReference,
    pub modified_at: i64,
}

impl FileSource {
    pub fn current(&self, host: &dyn RuntimeStorageHost) -> Result<bool, Error> {
        crate::RuntimeFileSyncStore::requireSpaceFile(&self.path).map_err(Error::Message)?;
        Ok(operit_util::FileUtils::FileUtils::isPortableFilePath(&self.path)
            && host.exists(&self.path)? && host.modifiedAt(&self.path)? == self.modified_at)
    }
}

impl Journal {
    pub(crate) fn file_sources(&mut self, hash: &str) -> Result<Vec<FileSource>, Error> {
        self.0.query("SELECT path,size,modified_at FROM sync_file_sources WHERE hash=?1", vec![text(hash)])?
            .iter().map(|row| Ok(FileSource { path: row.valueAt(0)?.asString()?,
                reference: RuntimeFileSyncReference { contentHash: hash.into(), size: row.valueAt(1)?.asI64()? },
                modified_at: row.valueAt(2)?.asI64()? })).collect()
    }

    pub(crate) fn index_file(tx: &mut dyn RuntimeSqliteTransaction, source: &FileSource) -> Result<(), Error> {
        validateBlobReference(&source.reference).map_err(Error::Message)?;
        tx.execute("INSERT INTO sync_file_sources VALUES(?1,?2,?3,?4) ON CONFLICT(path) DO UPDATE SET hash=excluded.hash,size=excluded.size,modified_at=excluded.modified_at",
            vec![text(&source.path), text(&source.reference.contentHash), SqliteValue::Integer(source.reference.size), SqliteValue::Integer(source.modified_at)])?;
        tx.execute("INSERT OR IGNORE INTO sync_file_garbage VALUES(?1,0)", vec![text(&source.reference.contentHash)])?;
        Ok(())
    }

    pub(crate) fn unindex_file(tx: &mut dyn RuntimeSqliteTransaction, path: &str) -> Result<(), Error> {
        tx.execute("DELETE FROM sync_file_sources WHERE path=?1", vec![text(path)])?;
        Ok(())
    }

    /// Complete replacement states cover older completed originals, never pending reception.
    pub(crate) fn compact_file(tx: &mut dyn RuntimeSqliteTransaction, operation: &SyncOperation) -> Result<(), Error> {
        if !crate::RuntimeFileSyncStore::RuntimeFileSyncStore::isCompleteState(operation) { return Ok(()); }
        let order = Revision::fromOperation(operation);
        let ids = tx.query("SELECT id FROM originals WHERE domain='runtime_file' AND target=?1 AND pending=0 AND id<>?2
            AND (business_time,origin,sequence)<=(?3,?4,?5)", vec![text(&operation.entityId), text(&operation.opId),
                SqliteValue::Integer(order.createdAt), text(&order.originDeviceId), SqliteValue::Integer(order.sequence)])?
            .iter().map(|row| Ok(row.valueAt(0)?.asString()?)).collect::<Result<Vec<_>,Error>>()?;
        Self::retire_ids(tx, ids)
    }

    /// Publish a local content pointer in the same SQL commit as its immutable operation.
    pub(crate) fn publish_file(tx: &mut dyn RuntimeSqliteTransaction, operation: &SyncOperation) -> Result<(), Error> {
        if !crate::RuntimeFileSyncStore::RuntimeFileSyncStore::isCompleteState(operation) { return Ok(()); }
        if operation.operation == "delete" { Self::unindex_file(tx, &operation.entityId)?; }
        else {
            // Older payload formats stay readable through their original carrier.
            let Ok(snapshot) = serde_json::from_value::<crate::RuntimeFileSyncStore::RuntimeFileSnapshot>(operation.payload.clone()) else { return Ok(()); };
            if let Some(modified_at) = snapshot.modifiedAt {
                Self::index_file(tx, &FileSource { path: operation.entityId.clone(), reference: snapshot.reference, modified_at })?;
            }
        }
        Self::compact_file(tx, operation)
    }

    /// Upgrade old snapshots once. Validate the live winner before discarding any of its copies.
    pub(crate) fn migrate_file_index(&mut self, host: &dyn RuntimeStorageHost) -> Result<(), Error> {
        if !self.0.query("SELECT 1 FROM journal_meta WHERE key='live_file_index'", vec![])?.is_empty() { return Ok(()); }
        self.transaction(|tx| {
            tx.execute("INSERT OR IGNORE INTO sync_blob_references SELECT j.value,o.id FROM originals o,json_tree(o.original) j
                WHERE j.key='contentHash' AND j.type='text' AND length(j.value)=64", vec![])?;
            tx.execute("INSERT OR IGNORE INTO sync_owner_blob_references SELECT j.value,o.path FROM transaction_owners o,json_tree(o.state) j
                WHERE j.key='contentHash' AND j.type='text' AND length(j.value)=64", vec![])?;
            tx.execute("INSERT OR IGNORE INTO sync_file_garbage SELECT json_extract(original,'$.payload.contentHash'),0 FROM originals
                WHERE domain='runtime_file' AND json_type(original,'$.payload.contentHash')='text'", vec![])?;
            Ok(())
        })?;
        // Select one small replacement header per file, without loading historical content.
        let rows = self.0.query("SELECT original FROM originals a WHERE domain='runtime_file' AND pending=0
            AND NOT EXISTS(SELECT 1 FROM originals b WHERE b.domain=a.domain AND b.target=a.target AND b.pending=0
                AND (b.business_time,b.origin,b.sequence)>(a.business_time,a.origin,a.sequence))", vec![])?;
        for row in rows {
            let operation: SyncOperation = serde_json::from_str(&row.valueAt(0)?.asString()?)?;
            if !crate::RuntimeFileSyncStore::RuntimeFileSyncStore::isCompleteState(&operation) { continue; }
            if operation.operation == "delete" {
                self.transaction(|tx| Self::compact_file(tx, &operation))?;
            } else {
                let Ok(snapshot) = serde_json::from_value::<crate::RuntimeFileSyncStore::RuntimeFileSnapshot>(operation.payload.clone()) else { continue; };
                // Permission failures or external edits retain the old blob until the scanner publishes them.
                if let Ok(modified_at) = host.modifiedAt(&operation.entityId) {
                    let source = FileSource { path: operation.entityId.clone(), reference: snapshot.reference, modified_at };
                    if verify_source(host, &source).unwrap_or(false) {
                        self.transaction(|tx| { Self::index_file(tx, &source)?; Self::compact_file(tx, &operation) })?;
                    }
                }
            }
        }
        self.0.execute("INSERT INTO journal_meta VALUES('live_file_index','1')", vec![])?;
        Ok(())
    }

    /// Only known file copies are candidates. Transaction/SQL initialization blobs stay with their owners.
    pub(crate) fn collect_file_blobs(&mut self, host: &dyn RuntimeStorageHost, root: &str, now: i64) -> Result<(), Error> {
        let inventory = format!("{root}/workspace_inventory.json");
        let mut workspace = BTreeSet::new();
        if host.exists(&inventory)? {
            fn hashes(value: &serde_json::Value, result: &mut BTreeSet<String>) {
                match value {
                    serde_json::Value::Object(fields) => for (key, value) in fields {
                        if key == "contentHash" { if let Some(hash) = value.as_str() { result.insert(hash.into()); } }
                        else { hashes(value, result); }
                    },
                    serde_json::Value::Array(items) => for item in items { hashes(item, result); },
                    _ => (),
                }
            }
            hashes(&serde_json::from_slice(&host.readBytes(&inventory)?)?, &mut workspace);
        }
        let candidates = self.0.query("SELECT hash FROM sync_file_garbage WHERE after_ms<=?1", vec![SqliteValue::Integer(now)])?;
        for row in candidates {
            let hash = row.valueAt(0)?.asString()?;
            crate::SyncBlobIo::validateContentHash(&hash).map_err(Error::Message)?;
            if workspace.contains(&hash) { continue; }
            // Pending files, arbitrary transaction payloads and owner recovery state are durable roots.
            let protected = self.0.query("SELECT 1 FROM sync_blob_references r JOIN originals o ON o.id=r.operation_id
                WHERE r.hash=?1 AND (o.pending<>0 OR o.domain<>'runtime_file') UNION ALL
                SELECT 1 FROM sync_owner_blob_references WHERE hash=?1 LIMIT 1", vec![text(&hash)])?;
            if !protected.is_empty() { continue; }
            let referenced = !self.0.query("SELECT 1 FROM sync_blob_references r JOIN originals o ON o.id=r.operation_id WHERE r.hash=?1 LIMIT 1", vec![text(&hash)])?.is_empty();
            let backed = self.file_sources(&hash)?.iter().any(|source| source.current(host).unwrap_or(false));
            if referenced && !backed { continue; }
            let path = format!("{root}/blobs/{hash}");
            if host.exists(&path)? { host.delete(&path, false)?; }
            self.0.execute("DELETE FROM sync_file_garbage WHERE hash=?1", vec![text(&hash)])?;
        }
        Ok(())
    }

    pub(crate) fn index_blob_references(tx: &mut dyn RuntimeSqliteTransaction, id: &str) -> Result<(), Error> {
        tx.execute("DELETE FROM sync_blob_references WHERE operation_id=?1", vec![text(id)])?;
        tx.execute("INSERT OR IGNORE INTO sync_blob_references SELECT j.value,o.id FROM originals o,json_tree(o.original) j
            WHERE o.id=?1 AND j.key='contentHash' AND j.type='text' AND length(j.value)=64", vec![text(id)])?;
        Ok(())
    }

    pub(crate) fn index_owner_blobs(tx: &mut dyn RuntimeSqliteTransaction, path: &str) -> Result<(), Error> {
        tx.execute("DELETE FROM sync_owner_blob_references WHERE path=?1", vec![text(path)])?;
        tx.execute("INSERT OR IGNORE INTO sync_owner_blob_references SELECT j.value,o.path FROM transaction_owners o,json_tree(o.state) j
            WHERE o.path=?1 AND j.key='contentHash' AND j.type='text' AND length(j.value)=64", vec![text(path)])?;
        Ok(())
    }

    pub(crate) fn queue_file_blob(tx: &mut dyn RuntimeSqliteTransaction, id: &str) -> Result<(), Error> {
        let after = operit_host_api::TimeUtils::tryCurrentTimeMillis().map_err(Error::Message)? + TRANSFER_GRACE_MS;
        tx.execute("INSERT OR IGNORE INTO sync_file_garbage SELECT json_extract(original,'$.payload.contentHash'),?2
            FROM originals WHERE id=?1 AND domain='runtime_file' AND json_type(original,'$.payload.contentHash')='text'", vec![text(id), SqliteValue::Integer(after)])?;
        Ok(())
    }
}

fn verify_source(host: &dyn RuntimeStorageHost, source: &FileSource) -> Result<bool, Error> {
    use sha2::{Digest, Sha256};
    validateBlobReference(&source.reference).map_err(Error::Message)?;
    let mut digest = Sha256::new();
    let mut offset = 0;
    loop {
        let bytes = host.readBytesRange(&source.path, offset, 64 * 1024)?;
        if bytes.is_empty() { break; }
        offset += bytes.len() as u64;
        digest.update(&bytes);
    }
    Ok(offset == source.reference.size as u64 && format!("{:x}",digest.finalize()) == source.reference.contentHash && source.current(host)?)
}
