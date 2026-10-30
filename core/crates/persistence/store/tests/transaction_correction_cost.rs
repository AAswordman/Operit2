//! Aligning an authoritative SQL baseline must not rewrite unchanged business rows.
use std::sync::Arc;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{SqliteRowGet, SqliteStore::{SqliteStore, SqliteValue}, Transaction::{Backend, Owner, SqlDataset}};

#[test]
fn unchanged_sql_correction_updates_only_the_checkpoint_and_still_enforces_cas() {
    let root = std::env::temp_dir().join(format!("operit-correction-cost-{}", uuid::Uuid::new_v4()));
    {
        let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
        let store = SqliteStore::openWithHost(host, "runtime/correction.sqlite").unwrap();
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
            tables: &["business"], revisions: "SELECT 'unused',0,'peer',1 WHERE 0", cleanup: "" };
        let original = dataset.snapshot().unwrap();
        let mut source = original.clone();
        source.covered.insert("other-history:1".into());
        assert!(dataset.repair(&original.token().unwrap(), &source).unwrap());
        assert_eq!(dataset.baseline_digest().unwrap(), source.baseline_digest().unwrap());
        assert_eq!(store.queryOne("SELECT count(*) FROM audit", vec![]).unwrap().unwrap().get::<_, i64>(0).unwrap(), 0);
        assert!(!dataset.repair(&original.token().unwrap(), &source).unwrap());
        assert_eq!(store.queryOne("SELECT count(*) FROM audit", vec![]).unwrap().unwrap().get::<_, i64>(0).unwrap(), 0);
        let current = dataset.snapshot().unwrap();
        source.data.values.get_mut("business").unwrap()["rows"][0][1] = serde_json::json!({"Text":"corrected"});
        assert!(dataset.repair(&current.token().unwrap(), &source).unwrap());
        assert_eq!(dataset.snapshot().unwrap().data, source.data);
        assert_eq!(store.queryOne("SELECT count(*) FROM audit", vec![]).unwrap().unwrap().get::<_, i64>(0).unwrap(), 4000);
    }
    std::fs::remove_dir_all(root).unwrap();
}
