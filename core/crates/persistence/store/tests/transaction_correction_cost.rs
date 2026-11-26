//! Aligning an authoritative SQL baseline must not rewrite unchanged business rows.
use std::sync::Arc;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{SqliteRowGet, SqliteStore::{SqliteStore, SqliteValue}, Transaction::{Backend, Owner, SqlDataset}};

#[test]
fn staged_rows_keep_wire_order_and_multiplicity_but_project_distinct_contents() {
    use operit_store::{sync::SqlChatSyncStore::SqlChatSyncStore, Transaction::Snapshot};
    use serde_json::json;

    let root = std::env::temp_dir().join(format!("operit-snapshot-order-{}", uuid::Uuid::new_v4()));
    {
        let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        let store = SqliteStore::openWithHost(host, "runtime/order.sqlite").unwrap();
        store.executeBatch("CREATE TABLE messages(messageId INTEGER,body TEXT);
            INSERT INTO messages VALUES(2,'same'),(10,'same'),(2,'same'),(2,'other');").unwrap();
        let dataset = SqlDataset { store: &store, owner: Owner { backend: Backend::Sql,
            path: operit_util::RuntimeStorageLayout::SQLITE_DATABASE_PATH.into(), format: 1, layout: 1 },
            node: "authority".into(), tables: &["messages"],
            revisions: "SELECT 'unused',0,'peer',1 WHERE 0", cleanup: "", restore_schema: None };
        let staged = dataset.snapshot().unwrap();
        let wire = serde_json::to_value(&staged).unwrap();
        assert_eq!(wire["data"]["values"]["messages"]["rows"], json!([
            [{"Integer":10},{"Text":"same"}],
            [{"Integer":2},{"Text":"other"}],
            [{"Integer":2},{"Text":"same"}],
            [{"Integer":2},{"Text":"same"}]
        ]));
        let legacy: Snapshot = serde_json::from_value(wire).unwrap();
        assert_eq!(staged.token().unwrap(), legacy.token().unwrap());
        assert_eq!(SqlChatSyncStore::snapshotContentSummary(&staged).unwrap(),
            SqlChatSyncStore::snapshotContentSummary(&legacy).unwrap());
        let (digest, bytes) = SqlChatSyncStore::snapshotContentSummary(&staged).unwrap();
        assert_eq!(bytes, serde_json::to_vec(&json!({"messages":{"columns":["body"],"rows":[
            [{"Text":"other"}],[{"Text":"same"}]
        ]}})).unwrap().len() as u64);
        assert_eq!(staged.manifest_with_content((digest, bytes)).unwrap().contentBytes, bytes);
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unchanged_sql_correction_updates_only_the_checkpoint_and_still_enforces_cas() {
    let root = std::env::temp_dir().join(format!("operit-correction-cost-{}", uuid::Uuid::new_v4()));
    {
        let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        let store = SqliteStore::openWithHost(host.clone(), "runtime/correction.sqlite").unwrap();
        store.executeBatch("CREATE TABLE business(id INTEGER PRIMARY KEY,payload TEXT); CREATE TABLE audit(kind TEXT);
            CREATE TRIGGER audit_delete AFTER DELETE ON business BEGIN INSERT INTO audit VALUES('delete'); END;
            CREATE TRIGGER audit_insert AFTER INSERT ON business BEGIN INSERT INTO audit VALUES('insert'); END;").unwrap();
        store.transaction(|tx| {
            for index in 0..2000 {
                tx.execute("INSERT INTO business VALUES(?1,?2)", vec![SqliteValue::Integer(index), SqliteValue::Text("x".repeat(512))])?;
            }
            tx.execute("DELETE FROM audit", vec![])?;
            Ok(())
        }).unwrap();
        let dataset = SqlDataset { store: &store, owner: Owner { backend: Backend::Sql,
            path: "runtime/correction.sqlite".into(), format: 1, layout: 1 }, node: "authority".into(),
            tables: &["business"], revisions: "SELECT 'unused',0,'peer',1 WHERE 0", cleanup: "", restore_schema: None };
        let original = dataset.snapshot().unwrap();
        let mut source = original.clone();
        source.covered.insert("other-history:1".into());
        assert!(dataset.repair(&original.token().unwrap(), &source).unwrap());
        let operation = dataset.initialization().unwrap().unwrap();
        let files = operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore::new(host, "runtime/sync");
        let initialization = if operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore::requiredBlob(&operation).unwrap().is_some() { files.readSqlInitialization(&operation).unwrap() } else { operit_store::Transaction::compaction::Initialization::decode(&operation).unwrap().unwrap() };
        assert_eq!(initialization.data, source.data);
        assert_eq!(initialization.covered, source.covered);
        assert_eq!(store.queryOne("SELECT count(*) FROM audit", vec![]).unwrap().unwrap().get::<_, i64>(0).unwrap(), 0);
        assert!(!dataset.repair(&original.token().unwrap(), &source).unwrap());
        assert_eq!(store.queryOne("SELECT count(*) FROM audit", vec![]).unwrap().unwrap().get::<_, i64>(0).unwrap(), 0);
        let current = dataset.snapshot().unwrap();
        source.data = serde_json::from_slice(&serde_json::to_vec(&source.data).unwrap()).unwrap();
        source.data.values.get_mut("business").unwrap()["rows"][0][1] = serde_json::json!({"Text":"corrected"});
        assert!(dataset.repair(&current.token().unwrap(), &source).unwrap());
        assert_eq!(dataset.snapshot().unwrap().data, source.data);
        assert_eq!(store.queryOne("SELECT count(*) FROM audit", vec![]).unwrap().unwrap().get::<_, i64>(0).unwrap(), 4000);
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// Initializer installation, coverage and uncovered local replay commit or roll back together.
#[test]
fn initialization_replays_current_local_edits_atomically() {
    use operit_store::{SqliteStore::SqliteStoreError, Transaction::compaction::Initialization};

    let root = std::env::temp_dir().join(format!("operit-initialization-atomic-{}", uuid::Uuid::new_v4()));
    {
        let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        let store = SqliteStore::openWithHost(host, "runtime/atomic.sqlite").unwrap();
        store.executeBatch("CREATE TABLE business(id INTEGER PRIMARY KEY,payload TEXT);
            CREATE TABLE local_edits(id INTEGER PRIMARY KEY,payload TEXT);
            INSERT INTO business VALUES(1,'baseline');").unwrap();
        let dataset = SqlDataset { store: &store, owner: Owner { backend: Backend::Sql,
            path: "runtime/atomic.sqlite".into(), format: 1, layout: 1 }, node: "receiver".into(),
            tables: &["business"], revisions: "SELECT 'local:'||id,id,'local',id FROM local_edits",
            cleanup: "", restore_schema: None };
        let source = dataset.snapshot().unwrap();
        let initializer = Initialization::from_snapshot(&source).operation("plugin_storage", false).unwrap();
        // Local edits arrive after the baseline was exported, before it is received.
        store.executeBatch("UPDATE business SET payload='local'; INSERT INTO local_edits VALUES(1,'local');").unwrap();
        let before = dataset.snapshot().unwrap().token().unwrap();
        let failure = dataset.receive_initialization(&initializer, |tx, snapshot| {
            assert!(!snapshot.covered.contains("local:1"));
            assert_eq!(tx.queryOne("SELECT payload FROM business", vec![])?.unwrap().get::<_, String>(0)?, "baseline");
            tx.execute("INSERT OR REPLACE INTO business SELECT * FROM local_edits", vec![])?;
            Err(SqliteStoreError::Message("injected replay failure".into()))
        }).unwrap_err();
        assert_eq!(failure, "injected replay failure");
        assert_eq!(dataset.snapshot().unwrap().token().unwrap(), before);
        assert!(dataset.initialization().unwrap().is_none());
        assert!(dataset.receive_initialization(&initializer, |tx, snapshot| {
            assert!(!snapshot.covered.contains("local:1"));
            tx.execute("INSERT OR REPLACE INTO business SELECT * FROM local_edits", vec![])?;
            Ok(())
        }).unwrap());
        assert_eq!(store.queryOne("SELECT payload FROM business", vec![]).unwrap().unwrap().get::<_, String>(0).unwrap(), "local");
        assert!(dataset.coverage().unwrap().contains("local:1"));
        assert_eq!(dataset.initialization().unwrap().unwrap().opId, initializer.opId);
        assert!(!dataset.receive_initialization(&initializer, |_, _| panic!("covered initializer replayed")).unwrap());
    }
    std::fs::remove_dir_all(root).unwrap();
}
