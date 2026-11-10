//! Equivalent history replacement. Payloads retire only after their whole atomic effect is covered.
use super::{digest, newer, Data, Edit, Path, Revision, Snapshot, Target, Transaction};
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
CREATE TABLE IF NOT EXISTS _operit_initialization_metadata(id INTEGER PRIMARY KEY CHECK(id=1), data TEXT NOT NULL);
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
            damage: None,
            node: node.into(),
            generation,
            data: self.data.clone(),
            covered: self.covered.clone(),
        }
    }
    pub fn operation(&self, domain: &str, encrypted: bool) -> super::Result<SyncOperation> {
        #[cfg(feature="full")]
        if self.data.sql.is_some() { return super::sql_snapshot::operation(self,domain,encrypted); }
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
    /// Checks the immutable envelope without materializing a second copy of its business payload.
    /// Owner decoding still validates the complete schema when applying the transaction.
    pub fn validate_identity(operation: &SyncOperation) -> super::Result<bool> {
        if operation.operation != "initialize" || operation.entityType != "initialization" { return Ok(false); }
        let Some(payload) = operation.payload.as_object() else { return Ok(false); };
        let Some(data) = payload.get("initialization").and_then(|v|v.get("data")) else { return Ok(false); };
        let Some(path) = data.get("owner").and_then(|v|v.get("path")).and_then(serde_json::Value::as_str) else { return Ok(false); };
        let Some(format) = data.get("owner").and_then(|v|v.get("format")).and_then(serde_json::Value::as_u64) else { return Ok(false); };
        let created = data.get("revision").and_then(|v|v.get("createdAt")).and_then(serde_json::Value::as_i64).unwrap_or(0);
        let origin = format!("~init:{}", digest(&(&operation.domain, &operation.payload))?);
        Ok(payload.len() == 4 && payload.get("encrypted").is_some_and(serde_json::Value::is_boolean)
            && payload.get("storagePath").and_then(serde_json::Value::as_str) == Some(path)
            && payload.get("schemaVersion").and_then(serde_json::Value::as_u64) == Some(format)
            && operation.originDeviceId == origin && operation.opId == format!("{origin}:1")
            && operation.sequence == 1 && operation.createdAt == created && operation.schemaVersion == 1
            && operation.semantics == SyncOperationSemantics::Transaction && operation.entityId == path)
    }

    pub fn decode(operation: &SyncOperation) -> super::Result<Option<Self>> {
        if operation.operation != "initialize" || operation.entityType != "initialization" {
            return Ok(None);
        }
        let mut value = Self::deserialize(&operation.payload["initialization"]).map_err(|e| e.to_string())?;
        // Canonical metadata stays small; compare business values in place instead of re-encoding them.
        let values = std::mem::take(&mut value.data.values);
        let canonical = serde_json::to_value(&value).map_err(|e|e.to_string());
        value.data.values = values;
        let canonical = canonical?;
        let raw = operation.payload["initialization"].as_object().ok_or("Invalid initialization")?;
        let data = raw["data"].as_object().ok_or("Invalid initialization data")?;
        let expected = canonical["data"].as_object().ok_or("Invalid canonical data")?;
        let canonical = canonical.as_object().ok_or("Invalid canonical initialization")?;
        let matches = raw.len() == canonical.len()
            && canonical.iter().all(|(key,value)| key == "data" || raw.get(key) == Some(value))
            && data.len() == expected.len()
            && expected.iter().all(|(key,value)| key == "values" || data.get(key) == Some(value))
            && data["values"].as_object().is_some_and(|values| values.len() == value.data.values.len()
                && value.data.values.iter().all(|(key,value)| values.get(key) == Some(value)));
        if !matches || !Self::validate_identity(operation)? {
            return Err("Invalid initialization transaction identity".into());
        }
        Ok(Some(value))
    }
}

/// A footprint includes every effect of an atomic transaction. Only absolute
/// assignments can cover earlier effects; increments never cover each other.
#[derive(Clone, Debug)]
struct Effect {
    owner: String,
    key: String,
    path: Vec<Path>,
    absolute: bool,
    creation: bool,
}

fn effects(transaction: &Transaction) -> super::Result<Vec<Effect>> {
    let Target::Data(owner) = &transaction.target else {
        return Ok(Vec::new());
    };
    let owner = serde_json::to_string(owner).map_err(|e| e.to_string())?;
    let mut effects = Vec::new();
    let mut push = |key: &str, path: &[Path], absolute: bool, creates: bool| {
        // Array writes can create/reorder identities, and positional deletions
        // shift siblings. Their containing array must be replaced before the
        // original is disposable; equality of item paths is not a proof.
        let array = path.iter().position(|part| {
            matches!(part, Path::Item(_))
                || matches!(part, Path::Field(field) if field.parse::<usize>().is_ok())
        });
        effects.push(Effect {
            owner: owner.clone(),
            key: key.into(),
            path: array.map_or_else(|| path.to_vec(), |index| path[..index].to_vec()),
            absolute: absolute && array.is_none(),
            creation: false,
        });
        // A child write can leave an empty ancestor object after a later child
        // deletion. Keep that creation unless a later write recreates the same
        // container, or an absolute ancestor replacement removes it entirely.
        if creates {
            for end in 0..array.unwrap_or(path.len()) {
                effects.push(Effect {
                    owner: owner.clone(),
                    key: key.into(),
                    path: path[..end].to_vec(),
                    absolute: false,
                    creation: true,
                });
            }
        }
    };
    for edit in &transaction.edits {
        match edit {
            Edit::Set { key, path, .. } => push(key, path, true, true),
            Edit::Delete { key, path } => push(key, path, true, false),
            Edit::Increment { key, path, .. } => push(key, path, false, true),
            Edit::Patch { key, set, delete } => {
                for field in set.keys() {
                    push(key, &[Path::Field(field.clone())], true, true);
                }
                for field in delete {
                    push(key, &[Path::Field(field.clone())], true, false);
                }
            }
            Edit::ReplaceRecords { prefix, .. } => push(prefix, &[Path::Records], true, false),
        }
    }
    Ok(effects)
}

/// Exact absolute footprints for owners using the indexed replacement ledger.
pub fn scopes(transaction: &Transaction) -> super::Result<Option<BTreeSet<String>>> {
    let effects = effects(transaction)?;
    if effects.is_empty() || effects.iter().any(|effect| !effect.absolute) {
        return Ok(None);
    }
    effects
        .into_iter()
        .map(|effect| {
            let owner: super::Owner =
                serde_json::from_str(&effect.owner).map_err(|e| e.to_string())?;
            serde_json::to_string(&(owner, effect.key, effect.path)).map_err(|e| e.to_string())
        })
        .collect::<super::Result<BTreeSet<_>>>()
        .map(Some)
}

/// Select whole originals using ancestor and namespace coverage. A partial
/// winner, an unabsorbed increment, or a referenced original remains intact.
pub fn superseded<'a>(
    transactions: impl IntoIterator<Item = &'a Transaction>,
) -> super::Result<BTreeSet<String>> {
    let transactions = super::ordered(transactions, |tx| &tx.id, |tx| tx.revision.clone())?;
    // Legacy clock regressions can make an otherwise overwritten original a
    // causal ordering barrier against another author. Preserve that owner until
    // its revisions can be normalized rather than changing replay order.
    let mut clocks = BTreeMap::<&str, i64>::new();
    let mut regressed = BTreeSet::new();
    for transaction in &transactions {
        let revision = &transaction.revision;
        let previous = clocks.insert(&revision.originDeviceId, revision.createdAt);
        if previous.is_some_and(|time| time > revision.createdAt) {
            regressed
                .insert(serde_json::to_string(&transaction.target).map_err(|e| e.to_string())?);
        }
    }
    let protected = transactions
        .iter()
        .filter_map(|tx| match &tx.target {
            Target::Transaction(id) => Some(id.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut footprints = Vec::new();
    let mut assignments = BTreeMap::<(String, String, Vec<Path>), Revision>::new();
    let mut creations = BTreeMap::<(String, String, Vec<Path>), Revision>::new();
    for transaction in transactions {
        if protected.contains(&transaction.id)
            || regressed
                .contains(&serde_json::to_string(&transaction.target).map_err(|e| e.to_string())?)
        {
            continue;
        }
        let effects = effects(transaction)?;
        for effect in effects
            .iter()
            .filter(|effect| effect.absolute || effect.creation)
        {
            let key = (
                effect.owner.clone(),
                effect.key.clone(),
                effect.path.clone(),
            );
            let index = if effect.creation {
                &mut creations
            } else {
                &mut assignments
            };
            let winner = index
                .entry(key)
                .or_insert_with(|| transaction.revision.clone());
            if newer(&transaction.revision, winner) {
                *winner = transaction.revision.clone();
            }
        }
        footprints.push((transaction, effects));
    }
    let covered = |effect: &Effect, revision: &Revision| {
        let wins = |key: &str, path: &[Path]| {
            assignments
                .get(&(effect.owner.clone(), key.into(), path.to_vec()))
                .is_some_and(|winner| newer(winner, revision))
        };
        // A whole namespace replaces every matching key, including identities
        // absent locally and nested namespace replacements.
        if effect
            .key
            .char_indices()
            .map(|(index, _)| index)
            .chain([effect.key.len()])
            .any(|end| wins(&effect.key[..end], &[Path::Records]))
        {
            return true;
        }
        if effect.path == [Path::Records] {
            return false;
        }
        if effect.creation
            && creations
                .get(&(
                    effect.owner.clone(),
                    effect.key.clone(),
                    effect.path.clone(),
                ))
                .is_some_and(|winner| newer(winner, revision))
        {
            return true;
        }
        (0..=effect.path.len()).any(|end| wins(&effect.key, &effect.path[..end]))
    };
    Ok(footprints
        .into_iter()
        .filter(|(transaction, effects)| {
            !effects.is_empty()
                && effects
                    .iter()
                    .all(|effect| covered(effect, &transaction.revision))
        })
        .map(|(transaction, _)| transaction.id.clone())
        .collect())
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
    if !fingerprint.is_empty() && fingerprint != super::operation_digest(operation).map_err(error)? {
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
    if !register(tx, operation)? { return Ok(vec![]); }
    let revision = Revision::fromOperation(operation);
    let id = &operation.opId;
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
    retire_unscoped(tx, candidates)
}

fn register(tx: &mut dyn RuntimeSqliteTransaction, operation: &SyncOperation) -> HostResult<bool> {
    if retired(tx, operation)? { return Ok(false); }
    let revision = Revision::fromOperation(operation);
    Ok(tx.execute(
        "INSERT OR IGNORE INTO _operit_compacted(id,origin,sequence,business_time,fingerprint) VALUES(?1,?2,?3,?4,?5)",
        vec![text(&operation.opId), text(&revision.originDeviceId), SqliteValue::Integer(revision.sequence),
            SqliteValue::Integer(revision.createdAt), text(&super::operation_digest(operation).map_err(error)?)],
    )? != 0)
}

fn retire_unscoped(tx: &mut dyn RuntimeSqliteTransaction, candidates: BTreeSet<String>) -> HostResult<Vec<String>> {
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

/// Local preference commits arrive in author sequence order. Track remaining
/// effects, not payload history; removing a covered effect makes later edits cheap
/// even when an unrelated effect still pins its atomic original.
pub(crate) fn record_preference(
    tx: &mut dyn RuntimeSqliteTransaction,
    operation: &SyncOperation,
    transaction: &Transaction,
) -> HostResult<Vec<String>> {
    fn hex(value: &[u8]) -> String {
        const DIGITS: &[u8] = b"0123456789abcdef";
        value.iter().flat_map(|byte| [DIGITS[(byte >> 4) as usize] as char, DIGITS[(byte & 15) as usize] as char]).collect()
    }
    let effects = effects(transaction).map_err(error)?;
    if effects.is_empty() || !register(tx, operation)? { return Ok(vec![]); }
    let mut scopes = BTreeSet::new();
    let mut coverage = BTreeSet::new();
    for effect in effects {
        // Hex components keep arbitrary Unicode keys and escaped JSON paths
        // unambiguous. Every descendant sorts below the prefix followed by 'g'.
        let root = format!("preference/{}/{}/", effect.owner, operation.originDeviceId);
        let key = hex(effect.key.as_bytes());
        let path = effect.path.iter().filter(|part| **part != Path::Records)
            .map(|part| serde_json::to_vec(part).map(|value| format!("{}/", hex(&value))))
            .collect::<Result<String, _>>().map_err(error)?;
        let suffix = if effect.path == [Path::Records] { key } else { format!("{key}/{path}") };
        let kind = if effect.creation { "c" } else { "e" };
        scopes.insert(format!("{root}{kind}/{suffix}"));
        if effect.absolute {
            coverage.insert((format!("{root}e/{suffix}"), true));
            coverage.insert((format!("{root}c/{suffix}"), true));
        } else if effect.creation {
            coverage.insert((format!("{root}c/{suffix}"), false));
        }
    }
    let mut candidates = BTreeSet::new();
    for (scope, descendants) in coverage {
        let mut params = vec![text(&scope), text(&operation.originDeviceId), SqliteValue::Integer(operation.sequence)];
        let predicate = if descendants {
            params.push(text(&format!("{scope}g")));
            "s.scope>=?1 AND s.scope<?4"
        } else { "s.scope=?1" };
        let rows = tx.query(&format!(
            "SELECT s.scope,s.id FROM _operit_compaction_scopes s
             CROSS JOIN _operit_compacted c ON c.id=s.id WHERE {predicate}
             AND c.origin=?2 AND c.sequence<?3"),
            params)?;
        for row in rows {
            let id = row.valueAt(1)?.asString()?;
            tx.execute("DELETE FROM _operit_compaction_scopes WHERE scope=?1 AND id=?2",
                vec![row.valueAt(0)?.clone(), text(&id)])?;
            candidates.insert(id);
        }
    }
    for scope in scopes {
        tx.execute("INSERT INTO _operit_compaction_scopes(scope,id) VALUES(?1,?2)",
            vec![text(&scope), text(&operation.opId)])?;
    }
    retire_unscoped(tx, candidates)
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
/// Callers supply coverage from the already validated initializer or the committed snapshot.
pub(crate) fn save_initialization(
    tx: &mut crate::SqliteStore::SqliteTransaction<'_>,
    operation: &SyncOperation,
    covered: &BTreeSet<String>,
    data: &Data,
) -> Result<(), crate::SqliteStore::SqliteStoreError> {
    use crate::SqliteStore::{SqliteStoreError, SqliteRowGet};
    let fail = |e: String| SqliteStoreError::Message(e);
    if let Some(row) = tx.queryOne("SELECT operation_id,origin,sequence FROM _operit_transaction_initialization WHERE id=1",vec![])? {
        let id: String = row.get(0)?;
        if id != operation.opId {
            seal_identity(tx.raw(), &id, &row.get::<_,String>(1)?, row.get(2)?, 0, "")?;
        }
    }
    record(tx.raw(), operation, None)?;
    let original = serde_json::to_string(operation).map_err(|e|fail(e.to_string()))?;
    tx.execute("INSERT INTO _operit_transaction_initialization(id,operation_id,origin,sequence,original) VALUES(1,?1,?2,?3,?4)
        ON CONFLICT(id) DO UPDATE SET operation_id=excluded.operation_id,origin=excluded.origin,sequence=excluded.sequence,original=excluded.original",
        vec![text(&operation.opId),text(&operation.originDeviceId),SqliteValue::Integer(operation.sequence),SqliteValue::Text(original)])?;
    tx.execute("INSERT INTO _operit_initialization_metadata VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET data=excluded.data",vec![text(&serde_json::to_string(&serde_json::json!({"owner":data.owner,"revision":data.revision,"originalFingerprint":super::digest(operation).map_err(fail)?})).map_err(|e|fail(e.to_string()))?)])?;
    tx.execute("DELETE FROM _operit_initialization_coverage", vec![])?;
    for id in covered {
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
/// Installs every ledger table idempotently, including partially upgraded databases.
pub(crate) fn prepare(
    store: &crate::SqliteStore::SqliteStore,
) -> Result<(), crate::SqliteStore::SqliteStoreError> {
    store.executeBatch(SCHEMA)?;
    super::sql_snapshot::migrate_initialization(store)
}
