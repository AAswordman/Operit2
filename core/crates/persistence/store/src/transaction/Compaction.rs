//! Equivalent history replacement. Payloads retire only after their whole atomic effect is covered.
use super::{digest, newer, Data, Edit, Revision, Snapshot, Target, Transaction};
use crate::SyncOperationStore::{SyncOperation, SyncOperationSemantics};
use operit_host_api::{HostError, HostResult, RuntimeSqliteTransaction, SqliteValue};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS _operit_compacted(
 id TEXT PRIMARY KEY, origin TEXT NOT NULL, sequence INTEGER NOT NULL,
 business_time INTEGER NOT NULL, fingerprint TEXT NOT NULL, retired INTEGER NOT NULL DEFAULT 0);
CREATE INDEX IF NOT EXISTS _operit_compacted_origin ON _operit_compacted(origin,sequence);
CREATE INDEX IF NOT EXISTS _operit_compacted_time ON _operit_compacted(business_time);
CREATE TABLE IF NOT EXISTS _operit_compaction_scopes(
 scope TEXT NOT NULL,id TEXT NOT NULL,PRIMARY KEY(scope,id));
CREATE INDEX IF NOT EXISTS _operit_compaction_ids ON _operit_compaction_scopes(id);
CREATE TABLE IF NOT EXISTS _operit_transaction_initialization(id INTEGER PRIMARY KEY CHECK(id=1),
 operation_id TEXT NOT NULL,origin TEXT NOT NULL,sequence INTEGER NOT NULL,original TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS _operit_initialization_coverage(id TEXT PRIMARY KEY);
CREATE TABLE IF NOT EXISTS _operit_compaction_progress(version INTEGER PRIMARY KEY);";

fn text(value: &str) -> SqliteValue {
    SqliteValue::Text(value.into())
}
fn error(value: impl std::fmt::Display) -> HostError {
    HostError::new(value.to_string())
}

/// Replaces a correction's separate replay checkpoint with an actual, exportable initialization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Initialization {
    pub data: Data,
    pub covered: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition: Option<serde_json::Value>,
}
impl Initialization {
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        Self {
            data: snapshot.data.clone(),
            covered: snapshot.covered.clone(),
            definition: None,
        }
    }
    pub fn snapshot(&self, node: &str, generation: u64) -> Snapshot {
        Snapshot {
            node: node.into(),
            generation,
            data: self.data.clone(),
            covered: self.covered.clone(),
        }
    }
    pub fn operation(&self, domain: &str, encrypted: bool) -> super::Result<SyncOperation> {
        let payload = serde_json::json!({"storagePath":self.data.owner.path,"schemaVersion":self.data.owner.format,
            "encrypted":encrypted,"initialization":self});
        let origin = format!("~init:{}", digest(&(domain, &payload))?);
        Ok(SyncOperation {
            opId: format!("{origin}:1"),
            originDeviceId: origin,
            sequence: 1,
            domain: domain.into(),
            entityType: "initialization".into(),
            entityId: self.data.owner.path.clone(),
            operation: "initialize".into(),
            semantics: SyncOperationSemantics::Transaction,
            payload,
            createdAt: self.data.revision.as_ref().map_or(0, |r| r.createdAt),
            schemaVersion: 1,
        })
    }
    pub fn decode(operation: &SyncOperation) -> super::Result<Option<Self>> {
        if operation.operation != "initialize" || operation.entityType != "initialization" {
            return Ok(None);
        }
        let value: Self = serde_json::from_value(operation.payload["initialization"].clone())
            .map_err(|e| e.to_string())?;
        if value.operation(&operation.domain, operation.payload["encrypted"] == true)? != *operation
        {
            return Err("Invalid initialization transaction identity".into());
        }
        Ok(Some(value))
    }
}

/// Exact scopes only. Unsupported effects stay retained until a whole-file initialization covers them.
pub fn scopes(transaction: &Transaction) -> super::Result<Option<BTreeSet<String>>> {
    let Target::Data(owner) = &transaction.target else {
        return Ok(None);
    };
    let mut scopes = BTreeSet::new();
    for edit in &transaction.edits {
        let (key, path) = match edit {
            Edit::Set { key, path, .. } | Edit::Delete { key, path } => (key, path),
            _ => return Ok(None),
        };
        scopes.insert(serde_json::to_string(&(owner, key, path)).map_err(|e| e.to_string())?);
    }
    Ok((!scopes.is_empty()).then_some(scopes))
}

/// Select whole transactions before replay. A partial winner keeps its atomic original.
pub fn superseded<'a>(transactions: impl IntoIterator<Item = &'a Transaction>) -> super::Result<BTreeSet<String>> {
    let mut retired = BTreeSet::new();
    let mut winners = BTreeMap::<String, &Transaction>::new();
    for transaction in transactions {
        if !transaction.edits.iter().all(|edit| matches!(edit,
            Edit::Set { path, .. } | Edit::Delete { path, .. } if path.is_empty())) {
            continue;
        }
        let Some(scopes) = scopes(transaction)? else { continue; };
        retired.insert(transaction.id.clone());
        for scope in scopes {
            let winner = winners.entry(scope).or_insert(transaction);
            if newer(&transaction.revision, &winner.revision) { *winner = transaction; }
        }
    }
    for winner in winners.into_values() { retired.remove(&winner.id); }
    Ok(retired)
}

/// Returns a retired original's verification result without restoring its deleted payload.
pub(crate) fn retired(
    tx: &mut dyn RuntimeSqliteTransaction,
    operation: &SyncOperation,
) -> HostResult<bool> {
    let rows = tx.query(
        "SELECT fingerprint,retired FROM _operit_compacted WHERE id=?1",
        vec![text(&operation.opId)],
    )?;
    let Some(row) = rows.first() else {
        return Ok(false);
    };
    let fingerprint = row.valueAt(0)?.asString()?;
    if !fingerprint.is_empty() && fingerprint != digest(operation).map_err(error)? {
        return Err(error(format!(
            "Conflicting immutable operation {}",
            operation.opId
        )));
    }
    Ok(row.valueAt(1)?.asI64()? != 0)
}

/// Indexed write-time coverage. Every remaining scope of an old transaction must be replaced.
pub(crate) fn record(
    tx: &mut dyn RuntimeSqliteTransaction,
    operation: &SyncOperation,
    scopes: Option<BTreeSet<String>>,
) -> HostResult<Vec<String>> {
    if retired(tx, operation)? { return Ok(vec![]); }
    let revision = Revision::fromOperation(operation);
    let id = &operation.opId;
    let inserted = tx.execute(
        "INSERT OR IGNORE INTO _operit_compacted(id,origin,sequence,business_time,fingerprint) VALUES(?1,?2,?3,?4,?5)",
        vec![text(id), text(&revision.originDeviceId), SqliteValue::Integer(revision.sequence),
            SqliteValue::Integer(revision.createdAt), text(&digest(operation).map_err(error)?)],
    )?;
    if inserted == 0 { return Ok(vec![]); }
    let Some(scopes) = scopes else { return Ok(vec![]); };
    let mut candidates = BTreeSet::from([id.to_owned()]);
    for scope in scopes {
        let rows = tx.query(
            "SELECT c.id,c.origin,c.sequence,c.business_time FROM _operit_compaction_scopes s
            JOIN _operit_compacted c ON c.id=s.id WHERE s.scope=?1",
            vec![text(&scope)],
        )?;
        let mut covered = false;
        for row in rows {
            let id = row.valueAt(0)?.asString()?;
            let previous = Revision {
                originDeviceId: row.valueAt(1)?.asString()?,
                sequence: row.valueAt(2)?.asI64()?,
                createdAt: row.valueAt(3)?.asI64()?,
            };
            if newer(&revision, &previous) {
                tx.execute(
                    "DELETE FROM _operit_compaction_scopes WHERE scope=?1 AND id=?2",
                    vec![text(&scope), text(&id)],
                )?;
                candidates.insert(id);
            } else {
                covered = true;
            }
        }
        if !covered {
            tx.execute(
                "INSERT INTO _operit_compaction_scopes(scope,id) VALUES(?1,?2)",
                vec![text(&scope), text(id)],
            )?;
        }
    }
    let mut retired = Vec::new();
    for id in candidates {
        if tx
            .query(
                "SELECT 1 FROM _operit_compaction_scopes WHERE id=?1 LIMIT 1",
                vec![text(&id)],
            )?
            .is_empty()
        {
            tx.execute(
                "UPDATE _operit_compacted SET retired=1 WHERE id=?1",
                vec![text(&id)],
            )?;
            retired.push(id);
        }
    }
    Ok(retired)
}

pub(crate) fn seal(
    tx: &mut dyn RuntimeSqliteTransaction,
    operation: &SyncOperation,
) -> HostResult<()> {
    record(tx, operation, None)?;
    tx.execute(
        "UPDATE _operit_compacted SET retired=1 WHERE id=?1",
        vec![text(&operation.opId)],
    )?;
    tx.execute(
        "DELETE FROM _operit_compaction_scopes WHERE id=?1",
        vec![text(&operation.opId)],
    )?;
    Ok(())
}

#[cfg(feature = "full")]
pub(crate) fn initialization(
    tx: &mut crate::SqliteStore::SqliteTransaction<'_>,
) -> Result<Option<SyncOperation>, crate::SqliteStore::SqliteStoreError> {
    use crate::SqliteStore::SqliteRowGet;
    tx.queryOne(
        "SELECT original FROM _operit_transaction_initialization WHERE id=1",
        vec![],
    )?
    .map(|row| {
        serde_json::from_str(&row.get::<_, String>(0)?)
            .map_err(|e| crate::SqliteStore::SqliteStoreError::Message(e.to_string()))
    })
    .transpose()
}

#[cfg(feature = "full")]
pub(crate) fn save_initialization(
    tx: &mut crate::SqliteStore::SqliteTransaction<'_>,
    operation: &SyncOperation,
) -> Result<(), crate::SqliteStore::SqliteStoreError> {
    use crate::SqliteStore::SqliteStoreError;
    let fail = |e: String| SqliteStoreError::Message(e);
    let data = Initialization::decode(operation)
        .map_err(fail)?
        .ok_or_else(|| fail("Missing initialization".into()))?;
    if let Some(previous) = initialization(tx)? {
        if previous.opId != operation.opId {
            seal(tx.raw(), &previous)?;
        }
    }
    record(tx.raw(), operation, None)?;
    tx.execute("INSERT INTO _operit_transaction_initialization(id,operation_id,origin,sequence,original) VALUES(1,?1,?2,?3,?4)
        ON CONFLICT(id) DO UPDATE SET operation_id=excluded.operation_id,origin=excluded.origin,sequence=excluded.sequence,original=excluded.original",
        vec![text(&operation.opId),text(&operation.originDeviceId),SqliteValue::Integer(operation.sequence),text(&serde_json::to_string(operation).map_err(|e|fail(e.to_string()))?)])?;
    tx.execute("DELETE FROM _operit_initialization_coverage", vec![])?;
    for id in &data.covered {
        tx.execute(
            "INSERT INTO _operit_initialization_coverage VALUES(?1)",
            vec![text(id)],
        )?;
    }
    Ok(())
}

#[cfg(feature = "full")]
pub(crate) fn initialization_identity(
    store: &crate::SqliteStore::SqliteStore,
) -> Result<Option<(String, String, i64)>, crate::SqliteStore::SqliteStoreError> {
    use crate::SqliteStore::SqliteRowGet;
    store.queryOne("SELECT operation_id,origin,sequence FROM _operit_transaction_initialization WHERE id=1",vec![])?.map(|row|
        Ok((row.get(0)?,row.get(1)?,row.get(2)?))).transpose()
}

#[cfg(all(test, feature = "full"))]
#[path = "CompactionTests.rs"]
mod tests;

/// Fault recovery can retire damaged payloads using their committed identity metadata.
pub(crate) fn seal_identity(
    tx: &mut dyn RuntimeSqliteTransaction,
    id: &str,
    origin: &str,
    sequence: i64,
    time: i64,
    fingerprint: &str,
) -> HostResult<()> {
    tx.execute("INSERT INTO _operit_compacted(id,origin,sequence,business_time,fingerprint,retired) VALUES(?1,?2,?3,?4,?5,1)
        ON CONFLICT(id) DO UPDATE SET retired=1",vec![text(id),text(origin),SqliteValue::Integer(sequence),SqliteValue::Integer(time),text(fingerprint)])?;
    tx.execute(
        "DELETE FROM _operit_compaction_scopes WHERE id=?1",
        vec![text(id)],
    )?;
    Ok(())
}

#[cfg(feature = "full")]
pub(crate) fn observe_coverage(
    store: &crate::SqliteStore::SqliteStore,
    receipt: &mut crate::SyncOperationStore::SyncReceipt,
) -> Result<(), crate::SqliteStore::SqliteStoreError> {
    use crate::SqliteStore::SqliteRowGet;
    for row in store.queryRows("SELECT id FROM _operit_initialization_coverage", vec![])? {
        observe_id(receipt, &row.get::<_, String>(0)?);
    }
    Ok(())
}

pub(crate) fn observe_id(receipt: &mut crate::SyncOperationStore::SyncReceipt, id: &str) {
    if let Some((origin, sequence)) = id.rsplit_once(':') {
        if let Ok(sequence) = sequence.parse::<i64>() {
            if sequence > 0 {
                receipt.observe(origin, sequence);
            }
        }
    }
}

#[cfg(feature = "full")]
pub(crate) fn prepare(
    store: &crate::SqliteStore::SqliteStore,
) -> Result<(), crate::SqliteStore::SqliteStoreError> {
    if store
        .queryOne(
            "SELECT 1 FROM sqlite_master WHERE name='_operit_transaction_initialization'",
            vec![],
        )?
        .is_none()
    {
        store.executeBatch(SCHEMA)?;
    }
    Ok(())
}
