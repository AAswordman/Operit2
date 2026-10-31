//! The production carrier for immutable originals, pending work and durable receipts.
use operit_host_api::{RuntimeSqliteConnection, RuntimeSqliteTransaction, RuntimeStorageHost, SqliteValue};
use serde::Deserialize;
use std::collections::BTreeMap;

use crate::LegacyTransactionImport::JournalRecord;
use crate::SyncOperationStore::{SyncOperation, SyncOperationStoreError as Error, SyncReceipt};

pub(crate) struct Journal(Box<dyn RuntimeSqliteConnection>);

// Direct appends and recoverable preference edits share a durable identity floor.
const AUTHOR_SEQUENCE_HIGH_WATER_SQL: &str = "SELECT MAX(
    COALESCE((SELECT sequence FROM transaction_authors WHERE origin=?1),0),
    COALESCE((SELECT MAX(sequence) FROM originals WHERE origin=?1),0),
    COALESCE((SELECT MAX(sequence) FROM _operit_compacted WHERE origin=?1),0),
    COALESCE((SELECT MAX(last) FROM legacy_receipts WHERE origin=?1),0))";

impl Journal {
    pub(crate) fn open(host: &dyn RuntimeStorageHost, root: &str) -> Result<Self, Error> {
        let mut connection = host.openTransactionDatabase(&format!("{root}/transactions.sqlite"))?;
        connection.executeBatch("PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS originals(
                id TEXT PRIMARY KEY, origin TEXT NOT NULL, sequence INTEGER NOT NULL,
                domain TEXT NOT NULL, header TEXT NOT NULL, original TEXT NOT NULL,
                fingerprint TEXT NOT NULL, pending INTEGER NOT NULL,
                business_time INTEGER NOT NULL, target TEXT NOT NULL, format_version INTEGER NOT NULL,
                UNIQUE(origin, sequence));
            CREATE INDEX IF NOT EXISTS originals_pending ON originals(pending, domain);
            CREATE INDEX IF NOT EXISTS originals_time ON originals(business_time);
            CREATE TABLE IF NOT EXISTS legacy_receipts(origin TEXT, first INTEGER, last INTEGER,
                PRIMARY KEY(origin, first));
            CREATE TABLE IF NOT EXISTS legacy_fingerprints(id TEXT PRIMARY KEY, fingerprint TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS transaction_owners(path TEXT PRIMARY KEY,state TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS transaction_files(path TEXT PRIMARY KEY,encrypted INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS transaction_authors(origin TEXT PRIMARY KEY,sequence INTEGER NOT NULL,logical_time INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS transaction_commits(id TEXT PRIMARY KEY);
            CREATE INDEX IF NOT EXISTS originals_target ON originals(target,pending,business_time);
            CREATE TABLE IF NOT EXISTS journal_meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
        connection.executeBatch(super::compaction::SCHEMA)?;
        connection.executeBatch("CREATE TABLE IF NOT EXISTS preference_compaction_policy(path TEXT PRIMARY KEY,format INTEGER NOT NULL,safe INTEGER NOT NULL)")?;
        Ok(Self(connection))
    }

    pub(crate) fn transaction<T>(&mut self, action: impl FnOnce(&mut dyn RuntimeSqliteTransaction) -> Result<T, Error>) -> Result<T, Error> {
        let mut tx = self.0.beginTransaction()?;
        match action(tx.as_mut()) {
            Ok(value) => { tx.commit()?; Ok(value) }
            Err(error) => {
                if let Err(rollback) = tx.rollback() {
                    return Err(Error::Message(format!("{error}; journal rollback failed: {rollback}")));
                }
                Err(error)
            }
        }
    }

    /// Validation and publication share a SQL transaction, including duplicate IDs in a batch.
    pub(crate) fn put(tx: &mut dyn RuntimeSqliteTransaction, original: &SyncOperation, stored: &SyncOperation, pending: bool) -> Result<(), Error> {
        if original.originDeviceId.is_empty() || original.originDeviceId.len() > 256 || original.sequence <= 0
            || original.opId != format!("{}:{}", original.originDeviceId, original.sequence) {
            return Err(Error::Message("Invalid transaction identity".into()));
        }
        if super::compaction::retired(tx,original)? { return Ok(()); }
        if !tx.query("SELECT 1 FROM _operit_initialization_coverage WHERE id=?1",vec![text(&original.opId)])?.is_empty() { return Ok(()); }
        let fingerprint = crate::Transaction::digest(original).map_err(Error::Message)?;
        let previous = tx.query("SELECT fingerprint FROM originals WHERE id=?1 UNION ALL SELECT fingerprint FROM legacy_fingerprints WHERE id=?1", vec![text(&original.opId)])?;
        for row in &previous {
            if row.valueAt(0)?.asString()? != fingerprint {
                return Err(Error::Message(format!("Conflicting immutable operation {}", original.opId)));
            }
        }
        // A completed legacy receipt may have no retained payload. A matching redelivery restores it.
        let completed = !pending || !tx.query("SELECT 1 FROM legacy_receipts WHERE origin=?1 AND first<=?2 AND last>=?2", vec![text(&original.originDeviceId), SqliteValue::Integer(original.sequence)])?.is_empty();
        let header = JournalRecord::parse(&serde_json::to_vec(original)?, 0)?;
        tx.execute("INSERT INTO originals(id,origin,sequence,domain,header,original,fingerprint,pending,business_time,target,format_version)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(id) DO NOTHING", vec![
            text(&original.opId), text(&original.originDeviceId), SqliteValue::Integer(original.sequence), text(&original.domain),
            text(&serde_json::to_string(&header)?), text(&serde_json::to_string(stored)?), text(&fingerprint), SqliteValue::Integer(i64::from(!completed)),
            SqliteValue::Integer(crate::Transaction::Revision::fromOperation(original).createdAt),
            text(original.payload.get("storagePath").and_then(serde_json::Value::as_str).unwrap_or(&original.entityId)),
            SqliteValue::Integer(original.payload.get("schemaVersion").and_then(serde_json::Value::as_i64).unwrap_or(0)),
        ])?;
        Ok(())
    }

    pub(crate) fn get(&mut self, id: &str) -> Result<Option<SyncOperation>, Error> {
        self.0.query("SELECT original FROM originals WHERE id=?1", vec![text(id)])?.first()
            .map(|row| Ok(serde_json::from_str(&row.valueAt(0)?.asString()?)?)).transpose()
    }

    /// Replay reads only completed originals for this owner through originals_target.
    pub(crate) fn preference_originals(&mut self, path: &str) -> Result<Vec<SyncOperation>, Error> {
        self.0.query(
            "SELECT original FROM originals WHERE target=?1 AND pending=0 AND domain='preferences' ORDER BY origin,sequence",
            vec![text(path)],
        )?.into_iter().map(|row| Ok(serde_json::from_str(&row.valueAt(0)?.asString()?)?)).collect()
    }

    /// Local edits compare current-format originals only; initialization blobs stay unopened.
    pub(crate) fn preference_compaction_history(&mut self, path: &str, format: u32) -> Result<Vec<SyncOperation>, Error> {
        self.0.query(
            "SELECT o.original FROM originals o JOIN preference_compaction_policy p
             ON p.path=o.target AND p.format=o.format_version AND p.safe=1
             WHERE o.target=?1 AND p.format=?2 AND o.pending=0 AND o.domain='preferences'
             AND json_extract(o.header,'$.operation') NOT IN ('initialize','alias') ORDER BY o.origin,o.sequence",
            vec![text(path), SqliteValue::Integer(format.into())],
        )?.into_iter().map(|row| Ok(serde_json::from_str(&row.valueAt(0)?.asString()?)?)).collect()
    }

    /// Small headers are enough for export planning; business payloads are read only after selection.
    pub(crate) fn headers(&mut self, pending: Option<bool>) -> Result<Vec<JournalRecord>, Error> {
        let (sql, params) = match pending {
            Some(pending) => ("SELECT header FROM originals WHERE pending=?1 ORDER BY origin,sequence", vec![SqliteValue::Integer(i64::from(pending))]),
            None => ("SELECT header FROM originals ORDER BY origin,sequence", vec![]),
        };
        self.0.query(sql, params)?.into_iter().map(|row| Ok(serde_json::from_str(&row.valueAt(0)?.asString()?)?)).collect()
    }

    /// Uses origin/sequence indexes and exact intervals without decoding already received headers.
    pub(crate) fn headers_since<K: crate::SyncOperationStore::SyncOperationKnowledge>(
        &mut self, clock: &K, domains: &[String], projection: &str, export: bool, floors: &BTreeMap<String,i64>,
    ) -> Result<Vec<JournalRecord>, Error> {
        let mut headers = Vec::new();
        let domains = serde_json::to_string(domains)?;
        let sql = format!("SELECT header FROM originals WHERE origin=?1 AND sequence>=?2
            AND (?3 OR pending=0) AND (?4='[]' OR domain IN (SELECT value FROM json_each(?4)))
            AND ({projection}) AND NOT EXISTS (SELECT 1 FROM json_each(?5) r
                WHERE sequence BETWEEN json_extract(r.value,'$[0]') AND json_extract(r.value,'$[1]'))
            ORDER BY sequence");
        for row in self.0.query("SELECT origin,MAX(sequence) FROM originals GROUP BY origin", vec![])? {
            let origin = row.valueAt(0)?.asString()?;
            let Some(next) = clock.nextMissingSequence(&origin, floors.get(&origin).copied().unwrap_or(0)) else { continue; };
            if next > row.valueAt(1)?.asI64()? { continue; }
            for row in self.0.query(&sql, vec![text(&origin),SqliteValue::Integer(next),
                SqliteValue::Integer(i64::from(export)),text(&domains),text(&serde_json::to_string(&clock.knownRanges(&origin))?)])? {
                headers.push(serde_json::from_str(&row.valueAt(0)?.asString()?)?);
            }
        }
        Ok(headers)
    }

    pub(crate) fn pending_ids(&mut self) -> Result<Vec<String>, Error> {
        self.0.query("SELECT id FROM originals WHERE pending=1 ORDER BY id", vec![])?.into_iter()
            .map(|row| Ok(row.valueAt(0)?.asString()?)).collect()
    }

    // 0 = applied, 1 = retryable, 2 = skipped until file correction. All retain their receipts.
    pub(crate) fn skip(&mut self, operations: &[SyncOperation]) -> Result<(), Error> {
        self.transaction(|tx| {
            for operation in operations {
                tx.execute("UPDATE originals SET pending=2 WHERE id=?1 AND pending=1", vec![text(&operation.opId)])?;
            }
            Ok(())
        })
    }

    pub(crate) fn skipped_ids(&mut self) -> Result<Vec<String>, Error> {
        self.0.query("SELECT id FROM originals WHERE pending=2 ORDER BY id", vec![])?.into_iter()
            .map(|row| Ok(row.valueAt(0)?.asString()?)).collect()
    }

    pub(crate) fn complete(&mut self, operations: &[SyncOperation]) -> Result<(), Error> {
        self.transaction(|tx| {
            for operation in operations {
                let fingerprint = crate::Transaction::digest(operation).map_err(Error::Message)?;
                let rows = tx.query("SELECT fingerprint FROM originals WHERE id=?1", vec![text(&operation.opId)])?;
                if rows.is_empty() && (super::compaction::retired(tx,operation)? || !tx.query("SELECT 1 FROM _operit_initialization_coverage WHERE id=?1",vec![text(&operation.opId)])?.is_empty()) { continue; }
                let row = rows.first().ok_or_else(|| Error::Message(format!("Unreceived operation {}", operation.opId)))?;
                if row.valueAt(0)?.asString()? != fingerprint {
                    return Err(Error::Message(format!("Conflicting immutable operation {}", operation.opId)));
                }
                tx.execute("UPDATE originals SET pending=0 WHERE id=?1", vec![text(&operation.opId)])?;
                // SQL owners export through their own source; this journal only carries pending reception.
                if matches!(operation.domain.as_str(), "chat" | "plugin_storage") {
                    Self::retire_ids(tx,std::iter::once(operation.opId.clone()))?;
                }
            }
            Ok(())
        })
    }

    pub(crate) fn receipt(&mut self) -> Result<SyncReceipt, Error> {
        let mut receipt = SyncReceipt::default();
        for row in self.0.query("SELECT origin,first,last FROM legacy_receipts", vec![])? {
            receipt.observeRange(&row.valueAt(0)?.asString()?, row.valueAt(1)?.asI64()?, row.valueAt(2)?.asI64()?);
        }
        for row in self.0.query("SELECT origin,sequence FROM originals UNION SELECT origin,sequence FROM _operit_compacted ORDER BY origin,sequence", vec![])? {
            receipt.observe(&row.valueAt(0)?.asString()?, row.valueAt(1)?.asI64()?);
        }
        for row in self.0.query("SELECT id FROM _operit_initialization_coverage",vec![])? {
            super::compaction::observe_id(&mut receipt,&row.valueAt(0)?.asString()?);
        }
        Ok(receipt)
    }

    pub(crate) fn highest(&mut self, origin: &str) -> Result<i64, Error> {
        let rows = self.0.query(AUTHOR_SEQUENCE_HIGH_WATER_SQL, vec![text(origin)])?;
        Ok(rows[0].valueAt(0)?.asI64()?)
    }

    pub(crate) fn imported(&mut self) -> Result<bool, Error> {
        Ok(!self.0.query("SELECT 1 FROM journal_meta WHERE key='legacy_imported'", vec![])?.is_empty())
    }

    pub(crate) fn import_receipts(tx: &mut dyn RuntimeSqliteTransaction, state: &LegacyReceiveState) -> Result<(), Error> {
        state.receipt.validate().map_err(Error::Message)?;
        for (origin, ranges) in &state.receipt.ranges {
            for [first, last] in ranges {
                tx.execute("INSERT OR IGNORE INTO legacy_receipts VALUES(?1,?2,?3)", vec![text(origin), SqliteValue::Integer(*first), SqliteValue::Integer(*last)])?;
            }
        }
        for (id, fingerprint) in &state.fingerprints {
            tx.execute("INSERT INTO legacy_fingerprints VALUES(?1,?2)", vec![text(id), text(fingerprint)])?;
        }
        Ok(())
    }

    pub(crate) fn finish_import(tx: &mut dyn RuntimeSqliteTransaction) -> Result<(), Error> {
        tx.execute("INSERT INTO journal_meta VALUES('legacy_imported','1')", vec![])?;
        Ok(())
    }
}

pub(crate) fn text(value: &str) -> SqliteValue { SqliteValue::Text(value.to_owned()) }

/// Read-only compatibility input. Running synchronization never writes the old receive ledger.
#[derive(Default, Deserialize)]
pub(crate) struct LegacyReceiveState {
    #[serde(default, rename = "formatVersion")]
    pub(crate) format_version: Option<u32>,
    pub(crate) receipt: SyncReceipt,
    pub(crate) pending: BTreeMap<String, Box<serde_json::value::RawValue>>,
    #[serde(default)]
    pub(crate) fingerprints: BTreeMap<String, String>,
}

impl Journal {
    pub(crate) fn owner_state(&mut self, path: &str) -> Result<Option<super::kernel::OwnerState>, Error> {
        self.0.query("SELECT state FROM transaction_owners WHERE path=?1", vec![text(path)])?.first()
            .map(|row| Ok(serde_json::from_str(&row.valueAt(0)?.asString()?)?)).transpose()
    }

    pub(crate) fn save_owner(tx: &mut dyn RuntimeSqliteTransaction, state: &super::kernel::OwnerState) -> Result<(), Error> {
        tx.execute("INSERT INTO transaction_owners(path,state) VALUES(?1,?2) ON CONFLICT(path) DO UPDATE SET state=excluded.state",
            vec![text(&state.current.data.owner.path), text(&serde_json::to_string(state)?)])?;
        Ok(())
    }

    /// Reserving an identity does not publish an operation. Gaps are represented by exact receipts.
    pub(crate) fn reserve(&mut self, origin: &str, mut mutation: crate::SyncOperationStore::NewSyncOperation) -> Result<SyncOperation, Error> {
        self.transaction(|tx| {
            let rows = tx.query("SELECT sequence,logical_time FROM transaction_authors WHERE origin=?1", vec![text(origin)])?;
            let (sequence, logical) = match rows.first() {
                Some(row) => (row.valueAt(0)?.asI64()?, row.valueAt(1)?.asI64()?),
                None => (0,0),
            };
            let seen_sequence = tx.query(AUTHOR_SEQUENCE_HIGH_WATER_SQL, vec![text(origin)])?[0].valueAt(0)?.asI64()?;
            let sequence = sequence.max(seen_sequence).checked_add(1).ok_or_else(|| Error::Message("Transaction sequence overflow".into()))?;
            let created_at = operit_host_api::TimeUtils::tryCurrentTimeMillis().map_err(Error::Message)?;
            let seen = tx.query("SELECT MAX(COALESCE((SELECT MAX(business_time) FROM originals),0),COALESCE((SELECT MAX(business_time) FROM _operit_compacted),0))", vec![])?[0].valueAt(0)?.asI64()?;
            let logical = logical.max(seen).max(created_at).checked_add(1).ok_or_else(|| Error::Message("Transaction logical clock overflow".into()))?;
            mutation.payload["logicalTime"] = serde_json::Value::from(logical);
            tx.execute("INSERT INTO transaction_authors(origin,sequence,logical_time) VALUES(?1,?2,?3) ON CONFLICT(origin) DO UPDATE SET sequence=excluded.sequence,logical_time=excluded.logical_time",
                vec![text(origin), SqliteValue::Integer(sequence), SqliteValue::Integer(logical)])?;
            Ok(SyncOperation { opId: format!("{origin}:{sequence}"), originDeviceId: origin.into(), sequence,
                domain: mutation.domain, entityType: mutation.entityType, entityId: mutation.entityId,
                operation: mutation.operation, semantics: mutation.semantics, payload: mutation.payload,
                createdAt: created_at, schemaVersion: 1 })
        })
    }

    pub(crate) fn owner_paths(&mut self) -> Result<Vec<String>, Error> {
        self.0.query("SELECT path FROM transaction_owners UNION SELECT target FROM originals WHERE pending<>0 AND domain='preferences' ORDER BY path", vec![])?.iter()
            .map(|row| Ok(row.valueAt(0)?.asString()?)).collect()
    }

    pub(crate) fn mark_complete(tx: &mut dyn RuntimeSqliteTransaction, ids: &[String]) -> Result<(), Error> {
        for id in ids { tx.execute("UPDATE originals SET pending=0 WHERE id=?1", vec![text(id)])?; }
        Ok(())
    }
}

impl Journal {
    pub(crate) fn files(&mut self) -> Result<BTreeMap<String,bool>, Error> {
        self.0.query("SELECT path,encrypted FROM transaction_files ORDER BY path", vec![])?.iter()
            .map(|row| Ok((row.valueAt(0)?.asString()?, row.valueAt(1)?.asI64()? != 0))).collect()
    }
    pub(crate) fn register_file(&mut self, path: &str, encrypted: bool) -> Result<(), Error> {
        self.transaction(|tx| {
            let rows = tx.query("SELECT encrypted FROM transaction_files WHERE path=?1",vec![text(path)])?;
            if let Some(row) = rows.first() {
                if (row.valueAt(0)?.asI64()? != 0) != encrypted { return Err(Error::Message("Conflicting preference encryption policy".into())); }
                return Ok(());
            }
            tx.execute("INSERT INTO transaction_files VALUES(?1,?2)",vec![text(path), SqliteValue::Integer(i64::from(encrypted))])?;
            Ok(())
        })
    }
}

impl Journal {
    pub(crate) fn set_compaction(&mut self,path: &str,format: u32,safe: bool) -> Result<bool,Error> {
        self.transaction(|tx| {
            if !tx.query("SELECT 1 FROM preference_compaction_policy WHERE path=?1 AND format=?2 AND safe=?3",
                vec![text(path),SqliteValue::Integer(format.into()),SqliteValue::Integer(i64::from(safe))])?.is_empty() { return Ok(false); }
            tx.execute("INSERT INTO preference_compaction_policy VALUES(?1,?2,?3) ON CONFLICT(path) DO UPDATE SET format=excluded.format,safe=excluded.safe",
                vec![text(path),SqliteValue::Integer(format.into()),SqliteValue::Integer(i64::from(safe))])?;
            Ok(true)
        })
    }

    /// A coverage decision retires pending and completed copies through the same atomic path.
    pub(crate) fn retire_ids(tx: &mut dyn RuntimeSqliteTransaction, ids: impl IntoIterator<Item=String>) -> Result<(),Error> {
        for id in ids {
            for row in tx.query("SELECT origin,sequence,business_time,fingerprint FROM originals WHERE id=?1",vec![text(&id)])? {
                super::compaction::seal_identity(tx,&id,&row.valueAt(0)?.asString()?,row.valueAt(1)?.asI64()?,row.valueAt(2)?.asI64()?,&row.valueAt(3)?.asString()?)?;
                tx.execute("DELETE FROM originals WHERE id=?1",vec![text(&id)])?;
            }
        }
        Ok(())
    }

    pub(crate) fn retire_covered(tx: &mut dyn RuntimeSqliteTransaction, paths: &[String], covered: &std::collections::BTreeSet<String>) -> Result<(),Error> {
        for id in covered {
            tx.execute("INSERT OR IGNORE INTO _operit_initialization_coverage VALUES(?1)",vec![text(id)])?;
        }
        for path in paths {
            let ids = tx.query("SELECT id FROM originals WHERE target=?1 AND domain='preferences'",vec![text(path)])?.into_iter()
                .map(|row|row.valueAt(0)?.asString()).collect::<Result<Vec<_>,_>>()?;
            Self::retire_ids(tx,ids.into_iter().filter(|id|covered.contains(id)))?;
        }
        Ok(())
    }

    /// Upgrade the old duplicate SQL archive once. Completed SQL commits belong to their source.
    pub(crate) fn reclaim_sql_copies(&mut self) -> Result<(),Error> {
        self.transaction(|tx| {
            if !tx.query("SELECT 1 FROM journal_meta WHERE key='sql_sources_own_completed'",vec![])?.is_empty() { return Ok(()); }
            let ids = tx.query("SELECT id FROM originals WHERE pending=0 AND domain IN ('chat','plugin_storage')",vec![])?.into_iter()
                .map(|row|row.valueAt(0)?.asString()).collect::<Result<Vec<_>,_>>()?;
            Self::retire_ids(tx,ids)?;
            tx.execute("INSERT INTO journal_meta VALUES('sql_sources_own_completed','1')",vec![])?;
            Ok(())
        })
    }
}
