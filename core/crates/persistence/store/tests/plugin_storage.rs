//! Exercises actual Host-backed SQLite and record synchronization with isolated runtime roots.
use operit_host_api::{PluginStorage::StorageRequest, RuntimeStorageHost};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PluginStorage::{self, PluginStorageSession},
    RuntimeFileSyncStore::RuntimeFileSyncStore,
    SqliteStore::SqliteStore,
    SyncOperationStore::{SyncClock, SyncOperation, SyncOperationStore},
};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};

struct Fixture {
    root: PathBuf,
    host: Arc<NativeRuntimeStorageHost>,
    session: PluginStorageSession,
    roots: Vec<String>,
    path: String,
    device: &'static str,
}
impl Fixture {
    /// Opens a real native Host with independent SQLite files and a stable plugin-owned path.
    fn new(device: &'static str) -> Self {
        Self::withScope(device, "space")
    }
    /// Opens an isolated database under the explicitly selected registered ownership scope.
    fn withScope(device: &'static str, scope: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("operit-plugin-storage-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        let config = operit_store::ExtensionStore::ExtensionStore::dataPathForScope(
            "storage_test",
            scope,
        )
        .unwrap();
        let path = format!("{config}/records.sqlite");
        Self {
            root,
            host,
            session: PluginStorageSession::default(),
            roots: vec![config, operit_store::ExtensionStore::ExtensionStore::localDataPath("storage_test").unwrap()],
            path,
            device,
        }
    }
    /// Dispatches the exact wire request through authenticated retained handles.
    fn call(&self, request: Value) -> Result<Value, String> {
        self.session
            .request(
                self.host.clone(),
                self.host.clone(),
                serde_json::from_value::<StorageRequest>(request).unwrap(),
                self.device,
            )
            .map_err(|e| e.to_string())
            .and_then(|response| serde_json::to_value(response).map_err(|error| error.to_string()))
    }
    /// Opens one storage kind and returns its private session-local handle.
    fn open(&self, kind: &str) -> String {
        self.call(json!({"op":"open","path":self.path,"kind":kind}))
            .unwrap()["handle"]
            .as_str()
            .unwrap()
            .to_string()
    }
    /// Reads one key and its exact durable version.
    fn get(&self, handle: &str, key: &str) -> Value {
        self.call(json!({"op":"get","handle":handle,"collection":"keys","key":key}))
            .unwrap()
    }
    /// Exports original record operations independently from file synchronization.
    fn export(&self) -> Vec<SyncOperation> {
        PluginStorage::operationsSince(self.host.clone(), &SyncClock::empty(), &[], 1000).unwrap()
    }
    /// Replays all exported transactions onto another actual SQLite host.
    fn replay(&self, operations: &[SyncOperation]) {
        for operation in operations {
            PluginStorage::applyOperation(
                self.host.clone(),
                self.host.clone(),
                operation,
                &Default::default(),
            )
            .unwrap();
        }
    }
    /// Releases all database handles before removing only this test's unique temporary directory.
    fn dispose(mut self) {
        self.session = PluginStorageSession::default();
        assert_eq!(self.root.parent().unwrap(), std::env::temp_dir());
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

/// Constructs an unconditional key mutation without confusing null with deletion.
fn put(key: &str, value: Value) -> Value {
    json!({"collection":"keys","key":key,"value":value,"deleted":false,"checkVersion":false,"expectedVersion":null})
}

/// Derives record replication from directory ownership for every storage kind while preserving local changes.
#[test]
fn storage_ownership_selects_replication_without_open_options() {
    for scope in ["space", "device"] {
        for kind in ["sqlite", "objects"] {
            let f = Fixture::withScope("first", scope);
            let handle = f.open(kind);
            if kind == "sqlite" {
                f.call(json!({"op":"define_table","handle":handle,"table":table()})).unwrap();
                f.call(json!({"op":"execute","handle":handle,"statement":{
                    "sql":"INSERT INTO records VALUES(?,?,?)",
                    "params":[{"kind":"text","value":"one"},{"kind":"integer","value":"1"},{"kind":"null"}]
                }})).unwrap();
            } else {
                f.call(json!({"op":"commit","handle":handle,"mutations":[put("one",json!(1))]})).unwrap();
            }
            let operations = f.export();
            if scope == "space" {
                assert!(!operations.is_empty());
                assert!(operations.iter().all(|operation| operation.domain == PluginStorage::PLUGIN_STORAGE_SYNC_DOMAIN));
            } else {
                assert!(operations.is_empty());
            }
            f.call(json!({"op":"close","handle":handle})).unwrap();
            let reopened = f.open(kind);
            if kind == "sqlite" {
                let rows=f.call(json!({"op":"query","handle":reopened,"statement":{"sql":"SELECT id FROM records","params":[]}})).unwrap();
                assert_eq!(rows.as_array().unwrap().len(), 1);
            } else {
                assert_eq!(f.get(&reopened,"one")["value"],1);
            }
            f.dispose();
        }
    }
}

/// Keeps a Space plugin's independent local database and ordinary local files outside every synchronization domain.
#[test]
fn plugins_share_local_databases_by_path_without_replication() {
    let mut f = Fixture::new("first");
    let local = operit_store::ExtensionStore::ExtensionStore::localDataPath("storage_test").unwrap();
    f.path = format!("{local}/cache.sqlite");
    let handle = f.open("objects");
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("cached",json!(1))]})).unwrap();
    assert_eq!(f.get(&handle, "cached")["value"], 1);
    assert!(f.export().is_empty());
    RuntimeFileSyncStore::new(f.host.clone(), "runtime/sync")
        .trackChanges(&f.roots, || f.host.writeBytes(&format!("{local}/cache.txt"), b"local").map_err(|error| error.to_string()))
        .unwrap();
    assert!(SyncOperationStore::new(f.host.clone(), "runtime/sync").operationsSince(&SyncClock::empty(), &[], 100).unwrap().is_empty());
    let other = operit_store::ExtensionStore::ExtensionStore::localDataPath("another_plugin").unwrap();
    let otherOpened = f.call(json!({"op":"open","path":format!("{other}/cache.sqlite"),"kind":"objects"})).unwrap();
    assert!(otherOpened["handle"].is_string());
    let otherSession = PluginStorageSession::default();
    let reopened = otherSession.request(f.host.clone(), f.host.clone(), serde_json::from_value(json!({"op":"open","path":f.path,"kind":"objects"})).unwrap(), "second-script").unwrap();
    let operit_host_api::PluginStorage::StorageResponse::Open(reopened) = reopened else { panic!("Expected storage open result") };
    let otherHandle = &reopened.handle;
    let stored = otherSession.request(f.host.clone(), f.host.clone(), serde_json::from_value(json!({"op":"get","handle":otherHandle,"collection":"keys","key":"cached"})).unwrap(), "second-script").unwrap();
    let operit_host_api::PluginStorage::StorageResponse::Get(Some(stored)) = stored else { panic!("Expected stored value") };
    assert_eq!(stored.value, 1);
    otherSession.request(f.host.clone(), f.host.clone(), serde_json::from_value(json!({"op":"commit","handle":otherHandle,"mutations":[put("cached",json!(2))]})).unwrap(), "second-script").unwrap();
    assert_eq!(f.get(&handle, "cached")["value"], 2);
    drop(otherSession);
    f.dispose();
}

/// Rejects a removed synchronization option instead of accepting a caller-selected storage policy.
#[test]
fn open_wire_contract_rejects_sync_parameter() {
    for sync in [true, false] {
        assert!(serde_json::from_value::<StorageRequest>(json!({
            "op":"open", "path":"runtime/plugin_data/space/storage_test/records.sqlite",
            "kind":"objects", "sync":sync
        })).is_err());
    }
}

/// Keeps null, missing keys, durable reopen and immutable storage policies distinct.
#[test]
fn objects_preserve_null_and_reopen_without_changing_identity() {
    let f = Fixture::withScope("first", "device");
    let handle = f.open("objects");
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("nullable",Value::Null)]}))
        .unwrap();
    assert_eq!(f.get(&handle, "nullable")["value"], Value::Null);
    assert!(f.get(&handle, "nullable")["version"].is_string());
    assert_eq!(f.get(&handle, "absent"), Value::Null);
    f.call(json!({"op":"close","handle":handle})).unwrap();
    assert!(
        f.call(json!({"op":"get","handle":handle,"collection":"keys","key":"nullable"}))
            .is_err()
    );
    let reopened = f.open("objects");
    assert!(f.get(&reopened, "nullable")["version"].is_string());
    assert!(
        f.call(json!({"op":"open","path":f.path,"kind":"sqlite"}))
            .is_err()
    );
    assert!(
        f.call(
            json!({"op":"open","path":"runtime/outside.sqlite","kind":"objects"})
        )
        .is_err()
    );
    assert!(f.call(json!({"op":"open","path":format!("{}/../escape.sqlite",f.roots[0]),"kind":"objects"})).is_err());
    f.dispose();
}

/// Rolls back earlier writes and the change journal when a later key version conflicts.
#[test]
fn compare_and_set_batches_are_atomic() {
    let f = Fixture::withScope("first", "device");
    let handle = f.open("objects");
    f.call(
        json!({"op":"commit","handle":handle,"mutations":[put("a",json!(1)),put("b",json!(2))]}),
    )
    .unwrap();
    let mut stale = put("b", json!(3));
    stale["checkVersion"] = json!(true);
    stale["expectedVersion"] = json!("stale");
    assert!(
        f.call(json!({"op":"commit","handle":handle,"mutations":[put("a",json!(9)),stale]}))
            .unwrap_err()
            .starts_with("Storage version conflict:")
    );
    assert_eq!(f.get(&handle, "a")["value"], 1);
    assert_eq!(f.get(&handle, "b")["value"], 2);
    assert!(f.export().is_empty());
    let mut create = put("a", json!(3));
    create["checkVersion"] = json!(true);
    assert!(
        f.call(json!({"op":"commit","handle":handle,"mutations":[create]}))
            .is_err()
    );
    let mut remove = put("a", Value::Null);
    remove["deleted"] = json!(true);
    remove["checkVersion"] = json!(true);
    remove["expectedVersion"] = f.get(&handle, "a")["version"].clone();
    f.call(json!({"op":"commit","handle":handle,"mutations":[remove]}))
        .unwrap();
    assert_eq!(f.get(&handle, "a"), Value::Null);
    f.dispose();
}

/// Declares a portable record schema for integer and binary fidelity checks.
fn table() -> Value {
    json!({"name":"records","primaryKey":"id","columns":[{"name":"id","affinity":"text","nullable":false},{"name":"count","affinity":"integer","nullable":false},{"name":"bytes","affinity":"blob","nullable":true}]})
}

/// Verifies actual SQL rollback and a single dirty-row delta across a larger unchanged table.
#[test]
fn sqlite_tracks_only_changed_rows_and_preserves_int64_and_blobs() {
    let f = Fixture::new("first");
    let handle = f.open("sqlite");
    f.call(json!({"op":"define_table","handle":handle,"table":table()}))
        .unwrap();
    let batch=(0..256).map(|i|json!({"sql":"INSERT INTO records VALUES(?1,?2,?3)","params":[{"kind":"text","value":format!("row{i}")},{"kind":"integer","value":"9223372036854775807"},{"kind":"blob","value":[0,255,128]}]})).collect::<Vec<_>>();
    f.call(json!({"op":"transaction","handle":handle,"statements":batch}))
        .unwrap();
    let before = f.export();
    assert_eq!(before.len(), 2);
    assert_eq!(before[1].payload["changes"].as_array().unwrap().len(), 256);
    f.call(json!({"op":"execute","handle":handle,"statement":{"sql":"UPDATE records SET count=7 WHERE id='row42'","params":[]}})).unwrap();
    let operations = f.export();
    assert_eq!(operations.len(), 3);
    assert_eq!(
        operations[2].payload["changes"].as_array().unwrap().len(),
        1
    );
    let rows=f.call(json!({"op":"query","handle":handle,"statement":{"sql":"SELECT count,bytes FROM records WHERE id='row0'","params":[]}})).unwrap();
    assert_eq!(
        rows[0]["values"],
        json!([{"kind":"integer","value":"9223372036854775807"},{"kind":"blob","value":[0,255,128]}])
    );
    assert!(f.call(json!({"op":"transaction","handle":handle,"statements":[{"sql":"UPDATE records SET count=8 WHERE id='row42'","params":[]},{"sql":"INSERT INTO records VALUES('row42',1,NULL)","params":[]}]})).is_err());
    assert!(f.call(json!({"op":"execute","handle":handle,"statement":{"sql":"UPDATE records SET count='wrong' WHERE id='row42'","params":[]}})).is_err());
    let rows=f.call(json!({"op":"query","handle":handle,"statement":{"sql":"SELECT count FROM records WHERE id='row42'","params":[]}})).unwrap();
    assert_eq!(rows[0]["values"][0]["value"], "7");
    assert_eq!(f.export().len(), 3);
    f.dispose();
}

/// Repeated outer UPSERTs must keep dirty tracking atomic, including persisted legacy triggers.
#[test]
fn sqlite_repeated_upserts_sync_after_legacy_trigger_upgrade() {
    for legacy in [false, true] {
        let f = Fixture::new("first");
        let peer = Fixture::new("second");
        let handle = f.open("sqlite");
        f.call(json!({"op":"define_table","handle":handle,"table":table()})).unwrap();
        f.call(json!({"op":"close","handle":handle})).unwrap();
        if legacy {
            let store = SqliteStore::openWithHost(f.host.clone(), &f.path).unwrap();
            store.executeBatch("DROP TRIGGER _operit_track_records_update;
                CREATE TRIGGER _operit_track_records_update AFTER UPDATE ON records BEGIN
                INSERT OR REPLACE INTO _operit_dirty VALUES('records',OLD.id,1);
                INSERT OR REPLACE INTO _operit_dirty VALUES('records',NEW.id,0); END;").unwrap();
        }
        let handle = f.open("sqlite");
        let statements = (1..=3).map(|value| json!({
            "sql":format!("INSERT INTO records VALUES('one',{value},NULL) ON CONFLICT(id) DO UPDATE SET count=excluded.count"),
            "params":[]
        })).collect::<Vec<_>>();
        f.call(json!({"op":"transaction","handle":handle,"statements":statements})).unwrap();
        let operations = f.export();
        assert_eq!(operations.last().unwrap().payload["changes"].as_array().unwrap().len(), 1);
        peer.replay(&operations);
        peer.replay(&operations);
        let remote = peer.open("sqlite");
        let rows = peer.call(json!({"op":"query","handle":remote,"statement":{
            "sql":"SELECT id,count FROM records","params":[]
        }})).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["values"][1]["value"], "3");
        f.dispose();
        peer.dispose();
    }
}

/// Rejects stale SQL mutations without publishing earlier writes or synchronization changes.
#[test]
fn sql_affected_row_guards_rollback_the_batch_and_journal() {
    let f = Fixture::new("first");
    let handle = f.open("sqlite");
    f.call(json!({"op":"define_table","handle":handle,"table":table()})).unwrap();
    f.call(json!({"op":"execute","handle":handle,"statement":{"sql":"INSERT INTO records VALUES('one',1,NULL)","params":[],"expectedAffectedRows":1}})).unwrap();
    let before = f.export();
    let error = f.call(json!({"op":"transaction","handle":handle,"statements":[
        {"sql":"UPDATE records SET count=2 WHERE id='one'","params":[],"expectedAffectedRows":1},
        {"sql":"DELETE FROM records WHERE id='absent'","params":[],"expectedAffectedRows":1}
    ]})).unwrap_err();
    assert!(error.contains("affected-row precondition"), "{error}");
    let rows = f.call(json!({"op":"query","handle":handle,"statement":{"sql":"SELECT count FROM records WHERE id='one'","params":[]}})).unwrap();
    assert_eq!(rows[0]["values"][0]["value"], "1");
    assert_eq!(f.export().len(), before.len());
    let error = f.call(json!({"op":"query","handle":handle,"statement":{"sql":"SELECT count FROM records","params":[],"expectedAffectedRows":1}})).unwrap_err();
    assert!(error.contains("require a mutation"), "{error}");
    f.dispose();
}

/// Rejects real quoted identifiers and introspection functions through parsed AST names.
#[test]
fn sql_cannot_reach_private_tables_or_external_functions() {
    let f = Fixture::withScope("first", "device");
    let handle = f.open("sqlite");
    for sql in [
        "SELECT * FROM _operit_log",
        "SELECT * FROM '_operit_log'",
        "SELECT * FROM main.\"_operit_log\"",
        "SELECT * FROM pragma_table_info('records')",
        "SELECT readfile('secret')",
        "SELECT * FROM sqlite_master",
        "PRAGMA journal_mode",
        "ATTACH DATABASE 'other' AS other",
    ] {
        assert!(
            f.call(json!({"op":"query","handle":handle,"statement":{"sql":sql,"params":[]}}))
                .is_err(),
            "{sql}"
        );
    }
    let rows=f.call(json!({"op":"query","handle":handle,"statement":{"sql":"SELECT '_operit_log' AS literal","params":[]}})).unwrap();
    assert_eq!(rows[0]["values"][0]["value"], "_operit_log");
    f.dispose();
}

/// Rejects adopting an unrelated database and leaves its original schema intact.
#[test]
fn existing_unowned_databases_are_not_modified() {
    let f = Fixture::withScope("first", "device");
    let store = SqliteStore::openWithHost(f.host.clone(), &f.path).unwrap();
    store
        .execute("CREATE TABLE unrelated(value TEXT)", vec![])
        .unwrap();
    assert!(
        f.call(json!({"op":"open","path":f.path,"kind":"sqlite"}))
            .is_err()
    );
    assert!(!store.tableExists("_operit_manifest").unwrap());
    assert!(store.tableExists("unrelated").unwrap());
    drop(store);
    f.dispose();
}

/// Replays nulls and tombstones idempotently while forwarding the original transaction identities.
#[test]
fn key_sync_resolves_conflicts_and_forwards_original_transactions() {
    let a = Fixture::new("a");
    let b = Fixture::new("b");
    let ah = a.open("objects");
    a.call(json!({"op":"commit","handle":ah,"mutations":[put("key",Value::Null)]}))
        .unwrap();
    let first = a.export();
    b.replay(&first);
    b.replay(&first);
    let bh = b.open("objects");
    assert!(b.get(&bh, "key")["version"].is_string());
    assert_eq!(b.export().len(), 1);
    assert_eq!(b.export()[0].opId, first[0].opId);
    let mut remove = put("key", Value::Null);
    remove["deleted"] = json!(true);
    a.call(json!({"op":"commit","handle":ah,"mutations":[remove]}))
        .unwrap();
    b.replay(&a.export());
    assert_eq!(b.get(&bh, "key"), Value::Null);
    b.call(json!({"op":"commit","handle":bh,"mutations":[put("key",json!({"latest":true}))]}))
        .unwrap();
    a.replay(&b.export());
    assert_eq!(a.get(&ah, "key")["value"], json!({"latest":true}));
    assert_eq!(a.export().len(), 3);
    let mut conflict = first[0].clone();
    conflict.payload["changes"][0]["value"] = json!("forged");
    assert!(
        PluginStorage::applyOperation(
            b.host.clone(),
            b.host.clone(),
            &conflict,
            &Default::default(),
        )
        .is_err()
    );
    assert_eq!(b.get(&bh, "key")["value"], json!({"latest":true}));
    a.dispose();
    b.dispose();
}

/// Replays declared SQL rows and rolls back a malformed multi-row incoming transaction.
#[test]
fn sqlite_sync_is_atomic_and_validates_declared_rows() {
    let a = Fixture::new("a");
    let b = Fixture::new("b");
    let ah = a.open("sqlite");
    a.call(json!({"op":"define_table","handle":ah,"table":table()}))
        .unwrap();
    a.call(json!({"op":"execute","handle":ah,"statement":{"sql":"INSERT INTO records VALUES('one',9,X'00ff')","params":[]}})).unwrap();
    let operations = a.export();
    b.replay(&operations);
    let bh = b.open("sqlite");
    let mut invalid = operations[1].clone();
    invalid.sequence += 100;
    invalid.opId = format!("{}:{}", invalid.originDeviceId, invalid.sequence);
    invalid.entityId = invalid.opId.clone();
    invalid.createdAt += 100;
    invalid.payload["changes"][0]["value"][1] = json!({"kind":"integer","value":"10"});
    let mut second = invalid.payload["changes"][0].clone();
    second["key"] = json!("two");
    second["value"][0] = json!({"kind":"text","value":"two"});
    second["value"][1] = json!({"kind":"text","value":"bad"});
    invalid.payload["changes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert!(
        PluginStorage::applyOperation(
            b.host.clone(),
            b.host.clone(),
            &invalid,
            &Default::default(),
        )
        .is_err()
    );
    let rows=b.call(json!({"op":"query","handle":bh,"statement":{"sql":"SELECT count,bytes FROM records","params":[]}})).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["values"][0]["value"], "9");
    assert_eq!(b.export().len(), 2);
    a.dispose();
    b.dispose();
}

/// Ensures database bytes and sidecars never enter automatic whole-file change capture.
#[test]
fn file_sync_excludes_database_and_sidecars_but_keeps_ordinary_files() {
    let f = Fixture::new("first");
    let handle = f.open("objects");
    let files = RuntimeFileSyncStore::new(f.host.clone(), "runtime/sync");
    assert!(files.writeBytes(&f.path, b"corrupt").is_err());
    for suffix in ["", ".operit-storage.json", "-wal", "-shm", "-journal"] {
        assert!(PluginStorage::ownsFile(f.host.clone(), &format!("{}{suffix}", f.path)).unwrap());
    }
    files
        .trackChanges(&f.roots, || {
            f.call(json!({"op":"commit","handle":handle,"mutations":[put("k",json!("v"))]}))?;
            f.host
                .writeBytes(&format!("{}/notes.txt", f.roots[0]), b"note")
                .map_err(|e| e.to_string())?;
            Ok(())
        })
        .unwrap();
    let operations = SyncOperationStore::new(f.host.clone(), "runtime/sync")
        .operationsSince(&SyncClock::empty(), &[], 100)
        .unwrap();
    assert_eq!(operations.len(), 1);
    assert!(operations[0].entityId.ends_with("/notes.txt"));
    f.call(json!({"op":"close","handle":handle})).unwrap();
    PluginStorage::unregisterDatabase(f.host.clone(), &f.path).unwrap();
    assert!(f.export().is_empty());
    f.dispose();
}

/// Uses a new durable origin epoch after deliberate database removal and recreation.
#[test]
fn recreated_database_does_not_reuse_old_sync_operation_ids() {
    let f = Fixture::new("first");
    let handle = f.open("objects");
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("key",json!(1))]}))
        .unwrap();
    let previous = f.export()[0].opId.clone();
    f.call(json!({"op":"close","handle":handle})).unwrap();
    PluginStorage::unregisterDatabase(f.host.clone(), &f.path).unwrap();
    f.host.delete(&f.path, false).unwrap();
    f.host
        .delete(&format!("{}.operit-storage.json", f.path), false)
        .unwrap();
    let handle = f.open("objects");
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("key",json!(2))]}))
        .unwrap();
    assert_ne!(f.export()[0].opId, previous);
    assert_eq!(f.export()[0].sequence, 1);
    f.dispose();
}

/// Makes changed-row operations self-contained when origins interleave before initial schema messages.
#[test]
fn row_transactions_carry_only_their_required_schema() {
    let a = Fixture::new("a");
    let b = Fixture::new("b");
    let ah = a.open("sqlite");
    a.call(json!({"op":"define_table","handle":ah,"table":table()}))
        .unwrap();
    a.call(json!({"op":"execute","handle":ah,"statement":{"sql":"INSERT INTO records VALUES('one',1,NULL)","params":[]}})).unwrap();
    let operations = a.export();
    assert_eq!(operations[1].payload["tables"].as_array().unwrap().len(), 1);
    b.replay(&operations[1..]);
    let bh = b.open("sqlite");
    let rows=b.call(json!({"op":"query","handle":bh,"statement":{"sql":"SELECT id FROM records","params":[]}})).unwrap();
    assert_eq!(rows[0]["values"][0]["value"], "one");
    b.replay(&operations[..1]);
    assert_eq!(b.export().len(), 2);
    a.dispose();
    b.dispose();
}

/// Keeps local and shared databases fixed while moving only installation files in both directions.
#[test]
fn plugin_scope_moves_preserve_data_scopes_and_database_handles() {
    let f = Fixture::new("first");
    let extensions = operit_store::ExtensionStore::ExtensionStore::new(f.host.clone());
    let settings = json!({"members":["storage_test"],"enabledNames":["storage_test"],"disabledNames":[],"subpackageStates":{},"order":0,"builtin":false,"installationId":"0123456789abcdef0123456789abcdef"});
    extensions.registerDevice("package", "storage_test", "", settings).unwrap();
    let config = operit_store::ExtensionStore::ExtensionStore::configPathForScope("storage_test", "device").unwrap();
    f.host.writeBytes(&format!("{config}/settings.txt"), b"configuration").unwrap();
    let local = operit_store::ExtensionStore::ExtensionStore::localDataPath("storage_test").unwrap();
    let localPath = format!("{local}/cache.sqlite");
    let localHandle = f.call(json!({"op":"open","path":localPath,"kind":"objects"})).unwrap()["handle"].as_str().unwrap().to_string();
    f.call(json!({"op":"commit","handle":localHandle,"mutations":[put("cache",json!(7))]})).unwrap();
    let sharedHandle = f.open("objects");
    f.call(json!({"op":"commit","handle":sharedHandle,"mutations":[put("memory",json!(9))]})).unwrap();
    let before = f.export();
    assert_eq!(before.len(), 1);
    for scope in ["space", "device", "space"] {
        extensions.moveScope("package", "storage_test", scope).unwrap();
        assert_eq!(extensions.record("package", "storage_test").unwrap().scope, scope);
        assert!(f.host.exists(&localPath).unwrap());
        assert!(f.host.exists(&f.path).unwrap());
        assert_eq!(f.get(&localHandle, "cache")["value"], 7);
        assert_eq!(f.get(&sharedHandle, "memory")["value"], 9);
        assert_eq!(f.export().iter().map(|operation| &operation.opId).collect::<Vec<_>>(), before.iter().map(|operation| &operation.opId).collect::<Vec<_>>());
        let config = operit_store::ExtensionStore::ExtensionStore::configPathForScope("storage_test", scope).unwrap();
        assert_eq!(f.host.readBytes(&format!("{config}/settings.txt")).unwrap(), b"configuration");
        assert!(!f.host.exists(&format!("{config}/cache.sqlite")).unwrap());
        assert!(!f.host.exists(&format!("{config}/records.sqlite")).unwrap());
    }
    f.dispose();
}

/// Rejects malformed initial remote state before any plugin database or marker is created.
#[test]
fn malformed_remote_transactions_leave_no_database_or_catalog_entry() {
    let a = Fixture::new("a");
    let b = Fixture::new("b");
    let ah = a.open("objects");
    a.call(json!({"op":"commit","handle":ah,"mutations":[put("key",json!(1))]}))
        .unwrap();
    let mut invalid = a.export()[0].clone();
    invalid.payload["changes"][0]["collection"] = json!("");
    assert!(
        PluginStorage::applyOperation(
            b.host.clone(),
            b.host.clone(),
            &invalid,
            &Default::default(),
        )
        .is_err()
    );
    assert!(!b.host.exists(&b.path).unwrap());
    assert!(
        !b.host
            .exists(&format!("{}.operit-storage.json", b.path))
            .unwrap()
    );
    assert!(b.export().is_empty());
    a.dispose();
    b.dispose();
}

/// Keeps incoming identities exportable when old local state is excluded during Space bootstrap.
#[test]
fn bootstrap_excludes_only_preexisting_local_operations() {
    let a = Fixture::new("a");
    let b = Fixture::new("b");
    let ah = a.open("objects");
    let bh = b.open("objects");
    a.call(json!({"op":"commit","handle":ah,"mutations":[put("key",json!(1))]}))
        .unwrap();
    b.call(json!({"op":"commit","handle":bh,"mutations":[put("local",json!(2))]}))
        .unwrap();
    PluginStorage::prepareSpaceJoin(b.host.clone()).unwrap();
    b.replay(&a.export());
    assert_eq!(b.export().len(), 1);
    assert_eq!(b.export()[0].opId, a.export()[0].opId);
    PluginStorage::prepareSpaceJoin(b.host.clone()).unwrap();
    assert_eq!(b.export().len(), 1);
    a.dispose();
    b.dispose();
}

/// Removes installation records while persistent local and shared data retain their independent lifetimes.
#[test]
fn plugin_uninstall_preserves_independent_local_and_shared_databases() {
    let f = Fixture::new("first");
    let extensions = operit_store::ExtensionStore::ExtensionStore::new(f.host.clone());
    extensions.registerDevice("package","storage_test","",json!({"members":["storage_test"],"enabledNames":["storage_test"],"disabledNames":[],"subpackageStates":{},"order":0,"builtin":false,"installationId":"0123456789abcdef0123456789abcdef"})).unwrap();
    extensions.moveScope("package", "storage_test", "space").unwrap();
    let sharedHandle = f.open("objects");
    f.call(json!({"op":"commit","handle":sharedHandle,"mutations":[put("memory",json!(1))]})).unwrap();
    let local = operit_store::ExtensionStore::ExtensionStore::localDataPath("storage_test").unwrap();
    let localPath = format!("{local}/cache.sqlite");
    let localHandle = f.call(json!({"op":"open","path":localPath,"kind":"objects"})).unwrap()["handle"].as_str().unwrap().to_string();
    f.call(json!({"op":"commit","handle":localHandle,"mutations":[put("cached",json!(9))]})).unwrap();
    extensions.delete("package", "storage_test").unwrap();
    assert!(extensions.records("package").unwrap().is_empty());
    assert!(f.host.exists(&localPath).unwrap());
    assert!(f.host.exists(&f.path).unwrap());
    assert_eq!(f.get(&localHandle, "cached")["value"], 9);
    assert_eq!(f.get(&sharedHandle, "memory")["value"], 1);
    assert_eq!(f.export().len(), 1);
    f.dispose();
}

/// Replays released row layouts into an additive schema without erasing newer columns.
#[test]
fn additive_schema_projects_historical_rows_and_preserves_new_fields() {
    let source = Fixture::new("historical");
    let source_handle = source.open("sqlite");
    source.call(json!({"op":"define_table","handle":source_handle,"table":table()})).unwrap();
    source.call(json!({"op":"execute","handle":source_handle,"statement":{
        "sql":"INSERT INTO records(id,count,bytes) VALUES('same',1,NULL)","params":[]
    }})).unwrap();
    let target = Fixture::new("current");
    let target_handle = target.open("sqlite");
    let mut extended = table();
    extended["columns"].as_array_mut().unwrap().insert(1,
        json!({"name":"owner_uuid","affinity":"text","nullable":true}));
    target.call(json!({"op":"define_table","handle":target_handle,"table":extended})).unwrap();
    target.replay(&source.export());
    let rows = target.call(json!({"op":"query","handle":target_handle,"statement":{
        "sql":"SELECT id,count,owner_uuid FROM records", "params":[]
    }})).unwrap();
    assert_eq!(rows[0]["values"][1], json!({"kind":"integer","value":"1"}));
    assert_eq!(rows[0]["values"][2], json!({"kind":"null"}));
    target.call(json!({"op":"execute","handle":target_handle,"statement":{
        "sql":"UPDATE records SET owner_uuid='exact-owner' WHERE id='same'","params":[]
    }})).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    source.call(json!({"op":"execute","handle":source_handle,"statement":{
        "sql":"UPDATE records SET count=2 WHERE id='same'","params":[]
    }})).unwrap();
    target.replay(&source.export());
    let rows = target.call(json!({"op":"query","handle":target_handle,"statement":{
        "sql":"SELECT count,owner_uuid FROM records WHERE id='same'", "params":[]
    }})).unwrap();
    assert_eq!(rows[0]["values"][0], json!({"kind":"integer","value":"2"}));
    assert_eq!(rows[0]["values"][1], json!({"kind":"text","value":"exact-owner"}));
    target.call(json!({"op":"define_table","handle":target_handle,"table":table()})).unwrap();
    let store = SqliteStore::openWithHost(target.host.clone(), &target.path).unwrap();
    let definition = store.queryOne("SELECT definition FROM _operit_tables WHERE name='records'", Vec::new()).unwrap().unwrap();
    assert_eq!(serde_json::from_str::<Value>(&definition.values[0].asString().unwrap()).unwrap(), extended);
    drop(store);
    source.dispose();
    target.dispose();
}

/// Rejects incompatible and nonnullable additions before changing the installed table.
#[test]
fn additive_schema_rejects_unsafe_column_changes() {
    let fixture = Fixture::new("schema");
    let handle = fixture.open("sqlite");
    fixture.call(json!({"op":"define_table","handle":handle,"table":table()})).unwrap();
    let mut incompatible = table();
    incompatible["columns"][1]["affinity"] = json!("text");
    assert!(fixture.call(json!({"op":"define_table","handle":handle,"table":incompatible})).is_err());
    let mut required = table();
    required["columns"].as_array_mut().unwrap().push(json!({"name":"required_new","affinity":"text","nullable":false}));
    assert!(fixture.call(json!({"op":"define_table","handle":handle,"table":required})).is_err());
    fixture.dispose();
}

/// Guards an entire initialization transaction without suppressing independent row constraints.
#[test]
fn initialization_guard_skips_the_whole_transaction_and_journal() {
    let fixture = Fixture::new("initialization");
    let handle = fixture.open("sqlite");
    fixture.call(json!({"op":"define_table","handle":handle,"table":table()})).unwrap();
    let request = json!({"op":"transaction","handle":handle,
        "whenAbsent":{"collection":"records","key":"state"},"statements":[
            {"sql":"INSERT INTO records VALUES('default',1,NULL)","params":[],"expectedAffectedRows":1},
            {"sql":"INSERT INTO records VALUES('state',1,NULL)","params":[],"expectedAffectedRows":1}
        ]});
    assert_eq!(fixture.call(request.clone()).unwrap().as_array().unwrap().len(),2);
    let operations = fixture.export();
    assert_eq!(fixture.call(request).unwrap(),json!([]));
    assert_eq!(fixture.export().len(),operations.len());
    assert!(fixture.call(json!({"op":"transaction","handle":handle,
        "whenAbsent":{"collection":"records","key":"different"},"statements":[
            {"sql":"INSERT INTO records VALUES('new',1,NULL)","params":[]},
            {"sql":"INSERT INTO records VALUES('default',1,NULL)","params":[]}
        ]})).is_err());
    let rows = fixture.call(json!({"op":"query","handle":handle,"statement":{
        "sql":"SELECT id FROM records ORDER BY id","params":[]}})).unwrap();
    assert_eq!(rows.as_array().unwrap().len(),2);
    fixture.dispose();
}

/// Rewrites multiple existing references cumulatively even when no plugin operation arrives.
#[test]
fn preference_aliases_reconcile_existing_rows_without_rewriting_history() {
    let fixture = Fixture::new("references");
    let descriptor = json!({"version":3,"maxEntities":1,"maxReferences":2,"entities":[{
        "collection":"records","identityColumn":"id","nameColumn":null,
        "ignoredColumns":[],"keyEncoding":"length_prefixed_root","matchMode":"identity",
        "keyReferences":[],"childCollections":[],"references":[
            {"collection":"records","column":"provider","target":{"kind":"preference","path":"models"}},
            {"collection":"records","column":"voice","target":{"kind":"preference","path":"voices"}}
        ]}]});
    let handle = fixture.call(json!({"op":"open","path":fixture.path,"kind":"sqlite","merge":descriptor})).unwrap()["handle"].as_str().unwrap().to_string();
    let schema = json!({"name":"records","primaryKey":"id","columns":[
        {"name":"id","affinity":"text","nullable":false},
        {"name":"provider","affinity":"text","nullable":false},
        {"name":"voice","affinity":"text","nullable":false}]});
    fixture.call(json!({"op":"define_table","handle":handle,"table":schema})).unwrap();
    fixture.call(json!({"op":"execute","handle":handle,"statement":{
        "sql":"INSERT INTO records VALUES('same','old-provider','old-voice')","params":[]}})).unwrap();
    let original = fixture.export().iter().map(|op|serde_json::to_value(op).unwrap()).collect::<Vec<_>>();
    let references = std::collections::BTreeMap::from([
        ("models\u{001f}old-provider".to_string(),"retained-provider".to_string()),
        ("voices\u{001f}old-voice".to_string(),"retained-voice".to_string())]);
    PluginStorage::reconcilePreferenceReferences(fixture.host.clone(),&references).unwrap();
    PluginStorage::reconcilePreferenceReferences(fixture.host.clone(),&references).unwrap();
    let rows = fixture.call(json!({"op":"query","handle":handle,"statement":{
        "sql":"SELECT provider,voice FROM records WHERE id='same'","params":[]}})).unwrap();
    assert_eq!(rows[0]["values"][0],json!({"kind":"text","value":"retained-provider"}));
    assert_eq!(rows[0]["values"][1],json!({"kind":"text","value":"retained-voice"}));
    assert_eq!(fixture.export().iter().map(|op|serde_json::to_value(op).unwrap()).collect::<Vec<_>>(),original);
    fixture.dispose();
}

/// Opens matching native tables with an explicit dependent owner identity contract.
fn open_merge_fixture(fixture: &Fixture) -> String {
    let descriptor = json!({"version":3,"maxEntities":2,"maxReferences":2,"entities":[
        {"collection":"items","identityColumn":"id","nameColumn":"name","ignoredColumns":[],
         "keyEncoding":"length_prefixed_root","references":[]},
        {"collection":"owners","identityColumn":"ownerKey","nameColumn":null,"ignoredColumns":[],
         "keyEncoding":"slash_root","matchMode":"identity","keyReferences":[{"target":{"kind":"collection","collection":"items"},"prefix":"owner:"}],
         "references":[{"collection":"owners","column":"document","target":{"kind":"collection","collection":"owners"},
                        "keyPrefix":"owners/","keySuffix":"/USER.md","valueEncoding":"uri_component"}]}
    ]});
    let handle = fixture.call(json!({"op":"open","path":fixture.path,"kind":"sqlite","merge":descriptor})).unwrap()["handle"].as_str().unwrap().to_string();
    for (name, columns) in [("items",vec!["_key","id","name","content"]),("owners",vec!["_key","ownerKey","document"])] {
        fixture.call(json!({"op":"define_table","handle":handle,"table":{"name":name,"primaryKey":"_key",
            "columns":columns.into_iter().map(|name|json!({"name":name,"affinity":"text","nullable":false})).collect::<Vec<_>>()}})).unwrap();
    }
    handle
}

/// Commits one independent entity and its owner through the actual journaled transaction API.
fn seed_merge_item(fixture: &Fixture, handle: &str, id: &str, name: &str, content: &str) {
    let owner = format!("owner:{id}");
    fixture.call(json!({"op":"transaction","handle":handle,"statements":[
        {"sql":"INSERT INTO items VALUES(?,?,?,?)","params":([id,id,name,content].map(|value|json!({"kind":"text","value":value})))},
        {"sql":"INSERT INTO owners VALUES(?,?,?)","params":([owner.clone(),owner,format!("owners/owner%3A{id}/USER.md")].map(|value|json!({"kind":"text","value":value})))}
    ]})).unwrap();
}

/// Requires independent peers to retain the same UUID, owners and collision names in either direction.
#[test]
fn equivalent_entities_and_dependent_owners_converge_in_uuid_order() {
    let left = Fixture::new("left"); let right = Fixture::new("right");
    let a = open_merge_fixture(&left); let b = open_merge_fixture(&right);
    seed_merge_item(&left,&a,"z-equal","Same","equal");
    seed_merge_item(&right,&b,"a-equal","Same","equal");
    seed_merge_item(&left,&a,"b-conflict","Collision","left");
    seed_merge_item(&right,&b,"y-conflict","Collision","right");
    let left_original = left.export(); let right_original = right.export();
    left.replay(&right_original); right.replay(&left_original);
    for sql in ["SELECT * FROM items ORDER BY _key","SELECT * FROM owners ORDER BY _key"] {
        let query = |fixture: &Fixture, handle: &str| fixture.call(json!({"op":"query","handle":handle,"statement":{"sql":sql,"params":[]}})).unwrap();
        assert_eq!(query(&left,&a),query(&right,&b));
        assert_eq!(query(&left,&a).as_array().unwrap().len(),3);
    }
    left.replay(&right.export()); right.replay(&left.export());
    let rows = left.call(json!({"op":"query","handle":a,"statement":{"sql":"SELECT _key,name FROM items ORDER BY _key","params":[]}})).unwrap();
    assert_eq!(rows[0]["values"][0],json!({"kind":"text","value":"a-equal"}));
    assert_eq!(rows[1]["values"][1],json!({"kind":"text","value":"Collision"}));
    assert_eq!(rows[2]["values"][1],json!({"kind":"text","value":"Collision（1）"}));
    left.dispose();right.dispose();
}

/// Counts storage notifications without timers or an application runtime.
#[derive(Default)]
struct ChangeWake(std::sync::atomic::AtomicUsize);
impl std::task::Wake for ChangeWake {
    /// Records each publication that wakes the pending storage future.
    fn wake(self: Arc<Self>) { self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }
    /// Records borrowed wake notifications without consuming the counter.
    fn wake_by_ref(self: &Arc<Self>) { self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }
}

/// Observes initial state, exact key isolation, deletion and recreation without reading history.
#[test]
fn record_watch_emits_only_the_selected_record_and_tracks_absence() {
    use std::{future::Future, task::{Context, Poll, Waker}};
    use operit_host_api::PluginStorage::StorageRecordWatchState;
    let f = Fixture::new("record-local");
    let handle = f.open("objects");
    let waker = Waker::from(Arc::new(ChangeWake::default()));
    let mut context = Context::from_waker(&waker);
    let mut initial = Box::pin(f.session.waitForRecord(handle.clone(), "keys".into(), "live".into(), None));
    assert!(matches!(initial.as_mut().poll(&mut context), Poll::Ready(Ok(None))));
    drop(initial);
    let mut wait = Box::pin(f.session.waitForRecord(handle.clone(), "keys".into(), "live".into(), Some(StorageRecordWatchState { version: None })));
    assert!(wait.as_mut().poll(&mut context).is_pending());
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("unrelated",json!(1)), {"collection":"other","key":"live","value":99,"deleted":false,"checkVersion":false}]})).unwrap();
    assert!(wait.as_mut().poll(&mut context).is_pending());
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("live",Value::Null)]})).unwrap();
    let Poll::Ready(Ok(Some(entry))) = wait.as_mut().poll(&mut context) else { panic!("Selected record was not emitted") };
    assert_eq!(entry.value, Value::Null);
    drop(wait);
    let mut wait = Box::pin(f.session.waitForRecord(handle.clone(), "keys".into(), "live".into(), Some(StorageRecordWatchState { version: Some(entry.version) })));
    assert!(wait.as_mut().poll(&mut context).is_pending());
    let mut remove=put("live",Value::Null); remove["deleted"]=json!(true);
    f.call(json!({"op":"commit","handle":handle,"mutations":[remove]})).unwrap();
    assert!(matches!(wait.as_mut().poll(&mut context), Poll::Ready(Ok(None))));
    drop(wait);
    f.dispose();
}

/// Observes remote application while rejecting handles from other sessions and databases.
#[test]
fn record_watch_receives_remote_changes_and_isolates_database_handles() {
    use std::{future::Future, task::{Context, Poll, Waker}};
    use operit_host_api::PluginStorage::StorageRecordWatchState;
    let source=Fixture::new("record-source"); let peer=Fixture::new("record-peer");
    let local=source.open("objects"); let handle=peer.open("objects");
    let waker=Waker::from(Arc::new(ChangeWake::default()));let mut context=Context::from_waker(&waker);
    let mut foreign=Box::pin(source.session.waitForRecord(handle.clone(),"keys".into(),"remote".into(),None));
    assert!(matches!(foreign.as_mut().poll(&mut context),Poll::Ready(Err(_))));drop(foreign);
    let mut wait=Box::pin(peer.session.waitForRecord(handle.clone(),"keys".into(),"remote".into(),Some(StorageRecordWatchState{version:None})));
    assert!(wait.as_mut().poll(&mut context).is_pending());
    source.call(json!({"op":"commit","handle":local,"mutations":[put("remote",json!(true))]})).unwrap();
    assert!(wait.as_mut().poll(&mut context).is_pending());
    peer.replay(&source.export());
    let Poll::Ready(Ok(Some(entry)))=wait.as_mut().poll(&mut context) else {panic!("Remote record was not emitted")};
    assert_eq!(entry.value,json!(true));drop(wait);
    source.dispose();peer.dispose();
}

/// Supports device-local state and releases pending subscriptions on close or cancellation.
#[test]
fn record_watch_supports_device_storage_and_releases_pending_waits() {
    use std::{future::Future, task::{Context, Poll, Waker}};
    use operit_host_api::PluginStorage::StorageRecordWatchState;
    let f=Fixture::withScope("record-device","device");let handle=f.open("objects");
    let counter=Arc::new(ChangeWake::default());let waker=Waker::from(counter.clone());let mut context=Context::from_waker(&waker);
    let mut wait=Box::pin(f.session.waitForRecord(handle.clone(),"keys".into(),"local".into(),Some(StorageRecordWatchState{version:None})));
    assert!(wait.as_mut().poll(&mut context).is_pending());
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("local",json!(1))]})).unwrap();
    let Poll::Ready(Ok(Some(entry)))=wait.as_mut().poll(&mut context) else {panic!("Device record was not emitted")};drop(wait);
    let mut wait=Box::pin(f.session.waitForRecord(handle.clone(),"keys".into(),"local".into(),Some(StorageRecordWatchState{version:Some(entry.version.clone())})));
    assert!(wait.as_mut().poll(&mut context).is_pending());drop(wait);
    let before=counter.0.load(std::sync::atomic::Ordering::SeqCst);
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("other",json!(2))]})).unwrap();
    assert_eq!(counter.0.load(std::sync::atomic::Ordering::SeqCst),before);
    let mut wait=Box::pin(f.session.waitForRecord(handle.clone(),"keys".into(),"local".into(),Some(StorageRecordWatchState{version:Some(entry.version)})));
    assert!(wait.as_mut().poll(&mut context).is_pending());f.call(json!({"op":"close","handle":handle})).unwrap();
    assert!(matches!(wait.as_mut().poll(&mut context),Poll::Ready(Err(_))));drop(wait);f.dispose();
}


/// Measures actual retained storage handles, file validation and SQLite reads independently.
#[test]
#[ignore = "Manual native storage benchmark with real files and SQLite"]
fn storage_query_roundtrip_benchmark() {
    use operit_store::SqliteStore::SqliteRowGet;
    /// Returns the median time of a fixed number of successful real host operations.
    fn median_micros(mut operation: impl FnMut(), iterations: usize) -> f64 {
        for _ in 0..10 { operation(); }
        let mut samples = Vec::new();
        for _ in 0..5 {
            let started = std::time::Instant::now();
            for _ in 0..iterations { operation(); }
            samples.push(started.elapsed().as_secs_f64() * 1e6 / iterations as f64);
        }
        samples.sort_by(f64::total_cmp);
        samples[2]
    }
    let fixture = Fixture::new("storage-benchmark");
    let handle = fixture.open("sqlite");
    let store = SqliteStore::openWithHost(fixture.host.clone(), &fixture.path).unwrap();
    let request = json!({"op":"query","handle":handle,"statement":{"sql":"SELECT 1 AS value","params":[]}});
    let session = median_micros(|| {
        let result = fixture.call(request.clone()).unwrap();
        assert_eq!(result[0]["values"][0], json!({"kind":"integer","value":"1"}));
    }, 200);
    let validation = median_micros(|| {
        assert!(fixture.host.exists(&fixture.path).unwrap());
        assert!(PluginStorage::ownsFile(fixture.host.clone(), &fixture.path).unwrap());
    }, 200);
    let query = median_micros(|| {
        assert_eq!(store.queryRows("SELECT 1 AS value", vec![]).unwrap()[0].get::<_,i64>(0).unwrap(), 1);
    }, 200);
    let transaction = median_micros(|| {
        assert_eq!(store.transaction(|tx| tx.queryRows("SELECT 1 AS value", vec![])).unwrap()[0].get::<_,i64>(0).unwrap(), 1);
    }, 200);
    eprintln!("STORAGE_BENCH session_us={session:.3} validation_us={validation:.3} query_us={query:.3} transaction_query_us={transaction:.3}");
    drop(store);
    fixture.dispose();
}


/// Allows a read-only SDK query while another connection holds SQLite's writer reservation.
#[test]
fn sql_query_does_not_reserve_the_writer() {
    let fixture = Fixture::new("concurrent-reader");
    let handle = fixture.open("sqlite");
    fixture.call(json!({"op":"define_table","handle":handle,"table":table()})).unwrap();
    let writer = SqliteStore::openWithHost(fixture.host.clone(), &fixture.path).unwrap();
    writer.transaction(|_| {
        let rows = fixture.call(json!({"op":"query","handle":handle,"statement":{
            "sql":"SELECT id FROM records", "params":[],
        }})).map_err(operit_store::SqliteStore::SqliteStoreError::Message)?;
        assert_eq!(rows, json!([]));
        Ok(())
    }).unwrap();
    drop(writer);
    fixture.dispose();
}

/// Opens complete JSON records with the same content matching and dependent ownership rules as SQL records.
fn open_object_merge_fixture(fixture: &Fixture) -> String {
    fixture.call(json!({"op":"open","path":fixture.path,"kind":"objects","merge":{
        "version":3,"maxEntities":2,"maxReferences":10,"entities":[{
            "collection":"cards","identityColumn":"id","nameColumn":"name",
            "ignoredColumns":["createdAt","updatedAt"],"keyEncoding":"slash_root","matchMode":"content",
            "keyReferences":[],"childCollections":[],"references":[
                {"collection":"cards","column":"providerId","target":{"kind":"preference","path":"models"}},
                {"collection":"cards","column":"mounts/*/providerId","target":{"kind":"preference","path":"models"}},
                {"collection":"meta","column":"active/CharacterCard/id","target":{"kind":"collection","collection":"cards"}},
                {"collection":"chat_bindings","column":"selection","target":{"kind":"collection","collection":"cards"},"keyPrefix":"card:"}
            ]
        },{
            "collection":"owners","identityColumn":"ownerKey","nameColumn":null,"ignoredColumns":[],
            "keyEncoding":"slash_root","matchMode":"identity","keyReferences":[{"target":{"kind":"collection","collection":"cards"},"prefix":"character:"}],
            "childCollections":["memories"],"references":[
                {"collection":"owners","column":"ownerKey","target":{"kind":"collection","collection":"cards"},"keyPrefix":"character:"},
                {"collection":"owners","column":"userDocumentPath","target":{"kind":"collection","collection":"owners"},"keyPrefix":"owners/","keySuffix":"/USER.md","valueEncoding":"uri_component"},
                {"collection":"memories","column":"ownerKey","target":{"kind":"collection","collection":"cards"},"keyPrefix":"character:"}
            ]
        }]
    }})).unwrap()["handle"].as_str().unwrap().into()
}

/// Writes one complete nested JSON document through the actual object transaction engine.
fn seed_object_card(fixture: &Fixture, handle: &str, id: &str, name: &str, content: &str) {
    fixture.call(json!({"op":"commit","handle":handle,"mutations":[{
        "collection":"cards","key":id,"value":{"id":id,"name":name,"providerId":"remote-provider","mounts":[{"providerId":"remote-provider","writable":true}],"prompt":{"text":content}},
        "deleted":false,"checkVersion":false,"expectedVersion":null
    }]})).unwrap();
}

/// Keeps duplicate input positions and distinguishes absent records from stored JSON null.
#[test]
fn object_batch_reads_are_ordered_bounded_and_preserve_null_records() {
    let fixture = Fixture::new("object-batch");
    let handle = fixture.open("objects");
    fixture.call(json!({"op":"commit","handle":handle,"mutations":[put("one",json!({"nested":[1,2]})),put("nullable",Value::Null)]})).unwrap();
    let result = fixture.call(json!({"op":"get_many","handle":handle,"collection":"keys","keys":["one","missing","nullable","one"]})).unwrap();
    assert_eq!(result.as_array().unwrap().len(), 4);
    assert_eq!(result[0], result[3]);
    assert_eq!(result[0]["value"]["nested"], json!([1,2]));
    assert!(result[1].is_null());
    assert!(!result[2].is_null());
    assert!(result[2]["value"].is_null());
    assert!(fixture.call(json!({"op":"get_many","handle":handle,"collection":"keys","keys":[]})).is_err());
    assert!(fixture.call(json!({"op":"get_many","handle":handle,"collection":"keys","keys":vec!["one";1001]})).is_err());
    assert!(fixture.call(json!({"op":"get_many","handle":handle,"collection":"keys","keys":[""]})).is_err());
    fixture.dispose();
}

/// Uses the existing content deduplication, UUID ordering and name collision policy for complete JSON.
#[test]
fn object_entities_use_the_existing_content_and_name_merge_rules() {
    let left = Fixture::new("object-left");
    let right = Fixture::new("object-right");
    let left_handle = open_object_merge_fixture(&left);
    let right_handle = open_object_merge_fixture(&right);
    let first = "11111111-1111-4111-8111-111111111111";
    let second = "22222222-2222-4222-8222-222222222222";
    seed_object_card(&left, &left_handle, first, "Same", "equal");
    seed_object_card(&right, &right_handle, second, "Same", "equal");
    seed_object_card(&left, &left_handle, "33333333-3333-4333-8333-333333333333", "Role", "left");
    seed_object_card(&right, &right_handle, "44444444-4444-4444-8444-444444444444", "Role", "right");
    let left_operations = left.export();
    let right_operations = right.export();
    left.replay(&right_operations); right.replay(&left_operations);
    let request = |handle: &str| json!({"op":"list","handle":handle,"collection":"cards","after":null,"limit":100});
    let left_rows = left.call(request(&left_handle)).unwrap();
    let right_rows = right.call(request(&right_handle)).unwrap();
    let values = |rows: &Value| rows.as_array().unwrap().iter().map(|row| row["value"].clone()).collect::<Vec<_>>();
    assert_eq!(values(&left_rows), values(&right_rows));
    assert_eq!(left_rows.as_array().unwrap().len(), 3);
    assert_eq!(left_rows[0]["key"], first);
    assert_eq!(left_rows[0]["value"]["id"], first);
    assert_eq!(left_rows[1]["value"]["name"], "Role");
    assert_eq!(left_rows[2]["value"]["name"], "Role（1）");
    assert_eq!(left_rows[2]["value"]["prompt"]["text"], "right");
    left.replay(&right_operations);
    assert_eq!(left.call(request(&left_handle)).unwrap(), left_rows);
    let sql = SqliteStore::openWithHost(left.host.clone(), &left.path).unwrap();
    assert!(sql.queryRows("SELECT name FROM sqlite_master WHERE type='table' AND name='cards'",vec![]).unwrap().is_empty());
    drop(sql); left.dispose(); right.dispose();
}

/// Preserves all owner-scoped memory and rewrites existing selections when equivalent card UUIDs converge.
#[test]
fn object_content_merge_rekeys_owners_memories_and_existing_selection_references() {
    let left = Fixture::new("object-owner-left"); let right = Fixture::new("object-owner-right");
    let a = open_object_merge_fixture(&left); let b = open_object_merge_fixture(&right);
    let low = "11111111-1111-4111-8111-111111111111";
    let high = "22222222-2222-4222-8222-222222222222";
    for (fixture, handle, id, timestamp, memory) in [(&left, &a, high, 10, "memory-left"), (&right, &b, low, 20, "memory-right")] {
        let owner = format!("character:{id}");
        fixture.call(json!({"op":"commit","handle":handle,"mutations":[
            {"collection":"cards","key":id,"value":{"id":id,"name":"Same","nested":{"array":[1,2]},"createdAt":timestamp,"updatedAt":timestamp},"deleted":false,"checkVersion":false,"expectedVersion":null},
            {"collection":"owners","key":owner,"value":{"ownerKey":owner,"userDocumentPath":format!("owners/character%3A{id}/USER.md")},"deleted":false,"checkVersion":false,"expectedVersion":null},
            {"collection":"memories","key":format!("{owner}/{memory}"),"value":{"ownerKey":owner,"record":{"id":memory,"content":memory}},"deleted":false,"checkVersion":false,"expectedVersion":null},
            {"collection":"meta","key":"state","value":{"active":{"CharacterCard":{"id":id}}},"deleted":false,"checkVersion":false,"expectedVersion":null},
            {"collection":"chat_bindings","key":memory,"value":{"selection":format!("card:{id}")},"deleted":false,"checkVersion":false,"expectedVersion":null}
        ]})).unwrap();
    }
    let left_ops = left.export(); let right_ops = right.export();
    left.replay(&right_ops); right.replay(&left_ops);
    for (fixture, handle) in [(&left, &a), (&right, &b)] {
        let owner = fixture.call(json!({"op":"list","handle":handle,"collection":"owners","limit":100})).unwrap();
        assert_eq!(owner.as_array().unwrap().len(), 1);
        assert_eq!(owner[0]["key"],format!("character:{low}"));
        assert_eq!(owner[0]["value"]["ownerKey"],format!("character:{low}"));
        assert_eq!(owner[0]["value"]["userDocumentPath"],format!("owners/character%3A{low}/USER.md"));
        let memories = fixture.call(json!({"op":"list","handle":handle,"collection":"memories","limit":100})).unwrap();
        assert_eq!(memories.as_array().unwrap().len(), 2);
        for row in memories.as_array().unwrap() {
            assert!(row["key"].as_str().unwrap().starts_with(&format!("character:{low}/")));
            assert_eq!(row["value"]["ownerKey"],format!("character:{low}"));
        }
        let metadata = fixture.call(json!({"op":"get","handle":handle,"collection":"meta","key":"state"})).unwrap();
        assert_eq!(metadata["value"]["active"]["CharacterCard"]["id"],low);
        let bindings = fixture.call(json!({"op":"list","handle":handle,"collection":"chat_bindings","limit":100})).unwrap();
        assert!(bindings.as_array().unwrap().iter().all(|row|row["value"]["selection"]==format!("card:{low}")));
    }
    left.dispose(); right.dispose();
}

/// Rewrites explicit nested JSON references without changing synchronization history or document identity.
#[test]
fn object_preference_aliases_rewrite_nested_json_without_new_operations() {
    let fixture = Fixture::new("object-reference");
    let handle = open_object_merge_fixture(&fixture);
    let id = "11111111-1111-4111-8111-111111111111";
    seed_object_card(&fixture, &handle, id, "Role", "Complete original JSON");
    let history = serde_json::to_value(fixture.export()).unwrap();
    let references = std::collections::BTreeMap::from([("models\u{001f}remote-provider".to_string(), "canonical-provider".to_string())]);
    PluginStorage::reconcilePreferenceReferences(fixture.host.clone(), &references).unwrap();
    let result = fixture.call(json!({"op":"get","handle":handle,"collection":"cards","key":id})).unwrap();
    assert_eq!(result["value"]["id"], id);
    assert_eq!(result["value"]["providerId"], "canonical-provider");
    assert_eq!(result["value"]["mounts"][0]["providerId"], "canonical-provider");
    assert_eq!(result["value"]["mounts"][0]["writable"], true);
    assert_eq!(serde_json::to_value(fixture.export()).unwrap(), history);
    fixture.dispose();
}

/// Keeps memory ownership aligned with native preference aliases without recreating configuration catalogs in SQLite.
#[test]
fn character_memory_contract_uses_native_preference_owner_aliases() {
    let descriptor: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../../plugins/packages/buildin/character_cards/src/storage/memory-merge.json"))).unwrap();
    assert_eq!(descriptor["entities"].as_array().unwrap().len(), 1);
    let fixture = Fixture::new("character-memory");
    let handle = fixture.call(json!({"op":"open","path":fixture.path,"kind":"objects","merge":descriptor})).unwrap()["handle"].as_str().unwrap().to_owned();
    let old = "ffffffff-ffff-4fff-8fff-ffffffffffff";
    let canonical = "00000000-0000-4000-8000-000000000001";
    let owner = format!("character:{old}");
    fixture.call(json!({"op":"commit","handle":handle,"mutations":[
        {"collection":"owners","key":owner,"value":{"ownerKey":owner,"userDocumentPath":format!("owners/character%3A{old}/USER.md")},"deleted":false,"checkVersion":false,"expectedVersion":null},
        {"collection":"memories","key":format!("{owner}/memory"),"value":{"ownerKey":owner,"record":{"id":"memory","content":"Retained content"}},"deleted":false,"checkVersion":false,"expectedVersion":null}
    ]})).unwrap();
    let history = serde_json::to_value(fixture.export()).unwrap();
    let references = std::collections::BTreeMap::from([(
        format!("runtime/plugin_data/space/com.operit.character_cards/cards.preferences.json\u{001f}{old}"), canonical.to_owned()
    )]);
    PluginStorage::reconcilePreferenceReferences(fixture.host.clone(), &references).unwrap();
    let owners = fixture.call(json!({"op":"list","handle":handle,"collection":"owners","limit":100})).unwrap();
    assert_eq!(owners.as_array().unwrap().len(), 1);
    assert_eq!(owners[0]["key"], format!("character:{canonical}"));
    assert_eq!(owners[0]["value"]["userDocumentPath"], format!("owners/character%3A{canonical}/USER.md"));
    let memories = fixture.call(json!({"op":"list","handle":handle,"collection":"memories","limit":100})).unwrap();
    assert_eq!(memories[0]["key"], format!("character:{canonical}/memory"));
    assert_eq!(memories[0]["value"]["ownerKey"], format!("character:{canonical}"));
    assert_eq!(memories[0]["value"]["record"]["content"], "Retained content");
    assert_eq!(serde_json::to_value(fixture.export()).unwrap(), history);
    fixture.dispose();
}

/// Byte budgets bound one exported page without splitting a transaction or reordering it.
#[test]
fn byte_budgets_bound_exported_pages_without_reordering() {
    let f = Fixture::new("first");
    let handle = f.open("objects");
    // Each object commit is one synchronization transaction. A 48 KiB value
    // makes a single transaction larger than a small page budget.
    for index in 0..4 {
        f.call(json!({
            "op": "commit",
            "handle": handle,
            "mutations": [put(&format!("key-{index}"), json!("x".repeat(48 * 1024)))],
        }))
        .unwrap();
    }
    let all = f.export();
    assert!(all.len() >= 4, "each commit must export one transaction");
    let budget = 64 * 1024;
    let page = PluginStorage::operationsSinceWithBudget(
        f.host.clone(),
        &SyncClock::empty(),
        &[],
        1000,
        budget,
    )
    .unwrap();
    assert!(!page.is_empty());
    assert!(page.len() < all.len(), "the byte budget must cut the page");
    // The page is an ordered prefix of the unbounded export: no transaction is
    // split and nothing is reordered.
    let pageIds = page
        .iter()
        .map(|operation| operation.opId.clone())
        .collect::<Vec<_>>();
    let prefixIds = all
        .iter()
        .take(page.len())
        .map(|operation| operation.opId.clone())
        .collect::<Vec<_>>();
    assert_eq!(pageIds, prefixIds);
    if page.len() > 1 {
        let bytes = page
            .iter()
            .map(|operation| serde_json::to_vec(&operation.payload).unwrap().len())
            .sum::<usize>();
        assert!(bytes <= budget, "a multi-operation page must stay in budget");
    }
    f.call(json!({"op":"close","handle":handle})).unwrap();
    f.dispose();
}

/// Opens the owners/memory fixture whose identities follow a preference catalog.
fn open_owner_merge_fixture(fixture: &Fixture, withMerge: bool) -> String {
    let mut request = json!({"op":"open","path":fixture.path,"kind":"objects"});
    if withMerge {
        request["merge"] = json!({"version":3,"maxEntities":1,"maxReferences":2,"entities":[{
            "collection":"owners","identityColumn":"ownerKey","nameColumn":null,"ignoredColumns":[],
            "keyEncoding":"slash_root","matchMode":"identity",
            "keyReferences":[{"prefix":"character:","target":{"kind":"preference","path":"runtime/config/preferences/cards.preferences.json"}}],
            "childCollections":["memories"],
            "references":[
                {"collection":"owners","column":"document","target":{"kind":"collection","collection":"owners"},
                 "keyPrefix":"owners/","keySuffix":"/USER.md","valueEncoding":"uri_component"},
                {"collection":"memories","column":"owner","target":{"kind":"preference","path":"runtime/config/preferences/cards.preferences.json"},
                 "keyPrefix":"character:","keySuffix":"","valueEncoding":"direct"}
            ]}]});
    }
    fixture.call(request).unwrap()["handle"].as_str().unwrap().to_string()
}

/// Builds one object mutation for an explicitly selected collection.
fn put_in(collection: &str, key: &str, value: Value) -> Value {
    json!({"collection":collection,"key":key,"value":value,"deleted":false,"checkVersion":false,"expectedVersion":null})
}

/// Canonicalizes one owners space plus its memory children for the alias tests.
fn seed_aliased_owner(fixture: &Fixture, handle: &str, oldId: &str, newId: &str) {
    let oldKey = format!("character:{oldId}");
    fixture.call(json!({"op":"commit","handle":handle,"mutations":[
        put_in("owners", &oldKey, json!({"ownerKey": oldKey, "document": format!("owners/character%3A{oldId}/USER.md")})),
        put_in("memories", &format!("{oldKey}/m1"), json!({"owner": oldKey, "title": "kept"}))
    ]})).unwrap();
    PluginStorage::reconcilePreferenceReferences(fixture.host.clone(), &std::collections::BTreeMap::from([
        (format!("runtime/config/preferences/cards.preferences.json\u{001f}{oldId}"), newId.to_string()),
    ])).unwrap();
}

/// Redirects a live writer's pre-merge identities onto the canonical rows so a
/// background rename can never be undone by the next local commit.
#[test]
fn local_commits_redirect_aliased_keys_onto_canonical_rows() {
    let fixture = Fixture::new("redirect");
    let oldId = "c3c356e7-17fb-4cce-b38b-4e230352564f";
    let newId = "13f64b4b-a19c-4daf-a27c-9f62ba873fe6";
    let oldKey = format!("character:{oldId}");
    let canonicalKey = format!("character:{newId}");
    let handle = open_owner_merge_fixture(&fixture, true);
    seed_aliased_owner(&fixture, &handle, oldId, newId);

    // A stale writer still holding the pre-merge identity commits again.
    fixture.call(json!({"op":"commit","handle":handle,"mutations":[
        put_in("owners", &oldKey, json!({"ownerKey": oldKey, "document": format!("owners/character%3A{oldId}/USER.md")})),
        put_in("memories", &format!("{oldKey}/m2"), json!({"owner": oldKey, "title": "resurrected"}))
    ]})).unwrap();

    let oldOwners = fixture.call(json!({"op":"get","handle":handle,"collection":"owners","key":oldKey})).unwrap();
    assert_eq!(oldOwners, json!(null), "the retired owners key must stay empty");
    let canonicalOwners = fixture.call(json!({"op":"get","handle":handle,"collection":"owners","key":canonicalKey})).unwrap();
    assert_eq!(canonicalOwners["value"]["ownerKey"], json!(canonicalKey));
    assert_eq!(canonicalOwners["value"]["document"], json!(format!("owners/character%3A{newId}/USER.md")));
    let resurrected = fixture.call(json!({"op":"get","handle":handle,"collection":"memories","key":format!("{canonicalKey}/m2")})).unwrap();
    assert_eq!(resurrected["value"]["owner"], json!(canonicalKey));
    assert_eq!(resurrected["value"]["title"], json!("resurrected"));
    assert_eq!(
        fixture.call(json!({"op":"get","handle":handle,"collection":"memories","key":format!("{oldKey}/m2")})).unwrap(),
        json!(null)
    );
    fixture.dispose();
}

/// Reopening a merged database moves rows that a pre-fix writer left under an
/// aliased identity back onto the canonical rows before the first read.
#[test]
fn opening_a_database_heals_rows_written_under_aliased_identities() {
    let fixture = Fixture::new("heal");
    let oldId = "c3c356e7-17fb-4cce-b38b-4e230352564f";
    let newId = "13f64b4b-a19c-4daf-a27c-9f62ba873fe6";
    let oldKey = format!("character:{oldId}");
    let canonicalKey = format!("character:{newId}");
    let handle = open_owner_merge_fixture(&fixture, true);
    seed_aliased_owner(&fixture, &handle, oldId, newId);

    // A writer without the merge contract recreates rows under the retired key.
    let bare = open_owner_merge_fixture(&fixture, false);
    fixture.call(json!({"op":"commit","handle":bare,"mutations":[
        put_in("owners", &oldKey, json!({"ownerKey": oldKey, "document": format!("owners/character%3A{oldId}/USER.md")})),
        put_in("memories", &format!("{oldKey}/m2"), json!({"owner": oldKey, "title": "orphan"}))
    ]})).unwrap();
    fixture.call(json!({"op":"close","handle":bare})).unwrap();
    fixture.call(json!({"op":"close","handle":handle})).unwrap();

    let reopened = open_owner_merge_fixture(&fixture, true);
    assert_eq!(
        fixture.call(json!({"op":"get","handle":reopened,"collection":"owners","key":oldKey})).unwrap(),
        json!(null)
    );
    let canonicalOwners = fixture.call(json!({"op":"get","handle":reopened,"collection":"owners","key":canonicalKey})).unwrap();
    assert_eq!(canonicalOwners["value"]["ownerKey"], json!(canonicalKey));
    assert_eq!(canonicalOwners["value"]["document"], json!(format!("owners/character%3A{newId}/USER.md")));
    let orphan = fixture.call(json!({"op":"get","handle":reopened,"collection":"memories","key":format!("{canonicalKey}/m2")})).unwrap();
    assert_eq!(orphan["value"]["owner"], json!(canonicalKey));
    assert_eq!(orphan["value"]["title"], json!("orphan"));
    assert_eq!(
        fixture.call(json!({"op":"get","handle":reopened,"collection":"memories","key":format!("{oldKey}/m2")})).unwrap(),
        json!(null)
    );
    fixture.dispose();
}

/// Exports immutable history without parsing an obsolete catalog merge declaration.
#[test]
fn obsolete_catalog_contract_does_not_block_history_export() {
    let source = Fixture::new("broken-contract-source");
    let target = Fixture::new("broken-contract-target");
    let handle = source.open("objects");
    source
        .call(json!({"op":"commit","handle":handle,"mutations":[put("pending",json!(7))]}))
        .unwrap();
    let pending = source.export();
    assert!(!pending.is_empty());
    let committed = PluginStorage::localClock(source.host.clone()).unwrap();
    let catalog = SqliteStore::openWithHost(
        source.host.clone(),
        "runtime/extensions/device/storage/catalog.sqlite",
    )
    .unwrap();
    let invalid = json!({
        "version": 1, "maxEntities": 1, "maxReferences": 1,
        "entities": [{
            "collection":"keys", "identityColumn":"key", "nameColumn":null,
            "ignoredColumns":[], "keyEncoding":"slash_root", "references":[],
            "keyReferences":[{"collection":"cards","prefix":"character:"}]
        }]
    });
    catalog
        .execute(
            "UPDATE plugin_databases SET merge=?1 WHERE path=?2",
            vec![
                operit_host_api::SqliteValue::Text(invalid.to_string()),
                operit_host_api::SqliteValue::Text(source.path.clone()),
            ],
        )
        .unwrap();
    assert_eq!(
        PluginStorage::localClock(source.host.clone()).unwrap(),
        committed
    );
    assert_eq!(
        PluginStorage::operationsSince(
            source.host.clone(),
            &SyncClock::empty(),
            &["plugin_storage".into()],
            100,
        )
        .unwrap(),
        pending
    );
    assert!(
        PluginStorage::operationsSince(
            source.host.clone(),
            &SyncClock::empty(),
            &["preferences".into()],
            100,
        )
        .unwrap()
        .is_empty()
    );
    assert!(
        PluginStorage::localClock(target.host.clone())
            .unwrap()
            .sequences
            .is_empty()
    );

    catalog
        .execute("UPDATE plugin_databases SET merge=NULL", Vec::new())
        .unwrap();
    let retried = source.export();
    assert_eq!(retried, pending);
    target.replay(&retried);
    assert_eq!(
        PluginStorage::localClock(target.host.clone()).unwrap(),
        committed
    );
    let targetHandle = target.open("objects");
    assert_eq!(target.get(&targetHandle, "pending")["value"], json!(7));
    drop(catalog);
    source.dispose();
    target.dispose();
}

/// Migrates the historical declaration only for execution and retains byte-equivalent JSON operation content.
#[test]
fn historical_key_reference_payload_replays_without_mutating_operation_identity() {
    let source = Fixture::new("historical-source");
    let target = Fixture::new("historical-target");
    let handle = open_object_merge_fixture(&source);
    seed_object_card(&source, &handle, "a", "Card", "content");
    let store = SqliteStore::openWithHost(source.host.clone(), &source.path).unwrap();
    for mut operation in source.export() {
        for entity in operation.payload["manifest"]["merge"]["entities"]
            .as_array_mut()
            .unwrap()
        {
            for reference in entity["keyReferences"].as_array_mut().unwrap() {
                let collection = reference["target"]["collection"].clone();
                reference.as_object_mut().unwrap().remove("target");
                reference["collection"] = collection;
            }
        }
        store
            .execute(
                "UPDATE _operit_log SET payload=?1 WHERE origin=?2 AND sequence=?3",
                vec![
                    operit_host_api::SqliteValue::Text(operation.payload.to_string()),
                    operit_host_api::SqliteValue::Text(operation.originDeviceId),
                    operit_host_api::SqliteValue::Integer(operation.sequence),
                ],
            )
            .unwrap();
    }
    let originals = source.export();
    assert!(originals.iter().any(|operation| {
        operation.payload["manifest"]["merge"]["entities"][1]["keyReferences"][0]
            .get("collection")
            .is_some()
    }));
    target.replay(&originals);
    target.replay(&originals);
    assert_eq!(target.export(), originals);
    drop(store);
    source.dispose();
    target.dispose();
}

/// Reports a broken database while exporting healthy databases in the same page.
#[test]
fn unavailable_database_does_not_block_healthy_database_export() {
    use operit_store::SyncOperationStore::SyncReceipt;
    let source = Fixture::new("independent-databases");
    let handle = source.open("objects");
    source
        .call(json!({"op":"commit","handle":handle,"mutations":[put("ready",json!(7))]}))
        .unwrap();
    let originals = source.export();
    let catalog = SqliteStore::openWithHost(
        source.host.clone(),
        "runtime/extensions/device/storage/catalog.sqlite",
    )
    .unwrap();
    let brokenPath = format!("{}/broken.sqlite", source.roots[0]);
    catalog.execute("INSERT INTO plugin_databases(path,owner,kind,sync,merge) VALUES(?1,'storage_test','objects',1,NULL)", vec![operit_host_api::SqliteValue::Text(brokenPath.clone())]).unwrap();
    let (operations, errors) = PluginStorage::exportWithBudget(
        source.host.clone(),
        &SyncReceipt::default(),
        &[],
        100,
        100_000,
    )
    .unwrap();
    assert_eq!(operations, originals);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains(&brokenPath));
    let (receipt, errors) = PluginStorage::receipt(source.host.clone()).unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        receipt.ranges[&originals[0].originDeviceId],
        vec![[1, originals.last().unwrap().sequence]]
    );
    drop(catalog);
    source.dispose();
}

/// Ordinary references survive delivery before their preference target exists.
#[test]
fn preference_references_do_not_block_independent_plugin_transactions() {
    let source = Fixture::new("reference-source");
    let target = Fixture::new("reference-target");
    let handle = open_owner_merge_fixture(&source, true);
    source.call(json!({"op":"commit","handle":handle,"mutations":[put_in("owners", "character:a", json!({"ownerKey":"character:a", "document":"owners/character%3Aa/USER.md"}))]})).unwrap();
    let originals = source.export();
    for _ in 0..2 {
        for operation in &originals {
            PluginStorage::applyOperation(
                target.host.clone(), target.host.clone(), operation, &Default::default(),
            ).unwrap();
        }
    }
    assert_eq!(target.export(), originals);
    let handle = open_owner_merge_fixture(&target, true);
    let record = target.call(json!({"op":"get","handle":handle,"collection":"owners","key":"character:a"})).unwrap();
    assert_eq!(record["value"]["ownerKey"], "character:a");
    source.dispose();
    target.dispose();
}

/// Queries synchronized source envelopes through the same parameterized CTE shape as memory retrieval.
#[test]
fn object_queries_rank_vectors_after_sync_replay() {
    let first = Fixture::new("first");
    let handle = first.open("objects");
    first.call(json!({"op":"commit","handle":handle,"mutations":[
        put("source", json!({"record":{"content":"source"},"embedding":{"vector":[1.0,0.0]}})),
        put("other", json!({"record":{"content":"other"},"embedding":{"vector":[0.0,1.0]}}))
    ]})).unwrap();
    // SQLiteDialect rejects AS MATERIALIZED before SQLite sees the statement.
    let sql = "WITH query_vector AS (SELECT CAST(key AS INTEGER) AS dimension,value FROM json_each(?1)) SELECT key,(SELECT sum(q.value*v.value) FROM json_each(o.value,'$.embedding.vector') v JOIN query_vector q ON q.dimension=v.key) AS score FROM plugin_objects o WHERE collection='keys' ORDER BY score DESC LIMIT 1";
    let second = Fixture::new("second");
    second.replay(&first.export());
    let other = second.open("objects");
    for (vector, expected) in [("[1.0,0.0]", "source"), ("[0.0,1.0]", "other")] {
        let params = json!([{"kind":"text","value":vector}]);
        let rows = first.call(json!({"op":"query","handle":handle,"statement":{"sql":sql,"params":params}})).unwrap();
        assert_eq!(rows[0]["values"][0]["value"], expected);
        assert_eq!(rows[0]["values"][1]["value"], 1.0);
        assert_eq!(rows, second.call(json!({"op":"query","handle":other,"statement":{"sql":sql,"params":params}})).unwrap());
    }
    for statement in [
        "SELECT * FROM _operit_log",
        "DELETE FROM plugin_objects",
        "SELECT * FROM sqlite_master",
        "WITH hidden AS (SELECT * FROM _operit_log) SELECT * FROM hidden",
        "WITH hidden AS (SELECT * FROM sqlite_master) SELECT * FROM hidden",
        "WITH keys AS (SELECT key FROM plugin_objects) DELETE FROM plugin_objects WHERE key IN (SELECT key FROM keys)",
    ] {
        assert!(first.call(json!({"op":"query","handle":handle,"statement":{"sql":statement,"params":[]}})).is_err());
    }
    first.dispose(); second.dispose();
}

/// Rejects a source write when related owner metadata changed while the caller awaited a provider.
#[test]
fn object_record_guards_are_checked_before_source_publication() {
    let f = Fixture::new("first");
    let handle = f.open("objects");
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("owner",json!({"model":"old"}))]})).unwrap();
    let version = f.get(&handle,"owner")["version"].clone();
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("owner",json!({"model":"new"}))]})).unwrap();
    assert!(f.call(json!({"op":"commit","handle":handle,"mutations":[put("source",json!({"vector":[1.0]}))],"guards":[{"collection":"keys","key":"owner","expectedVersion":version}]})).is_err());
    assert!(f.get(&handle,"source").is_null());
    let current = f.get(&handle,"owner");
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("source",json!({"vector":[1.0]}))],"guards":[{"collection":"keys","key":"owner","expectedVersion":current["version"]}]})).unwrap();
    assert_eq!(f.get(&handle,"owner"),current);
    f.dispose();
}

/// Changes are snapshot pages shared across handles; rejected transactions publish no identities.
#[test]
fn object_change_pages_include_other_handles_deletions_and_rollback() {
    let f = Fixture::new("first");
    let reader = f.open("objects");
    let writer = f.open("objects");
    let initial = f.call(json!({"op":"changes","handle":reader,"after":null,"limit":2})).unwrap();
    assert_eq!(initial["changes"], json!([]));
    f.call(json!({"op":"commit","handle":writer,"mutations":[put("a",json!({"vector":[1,0]})),put("b",json!(null))]})).unwrap();
    let mut cursor = initial["cursor"].clone();
    let mut found = std::collections::BTreeMap::new();
    loop {
        let page = f.call(json!({"op":"changes","handle":reader,"after":cursor,"limit":2})).unwrap();
        assert_eq!(page["reset"], false);
        for change in page["changes"].as_array().unwrap() { found.insert(change["key"].as_str().unwrap().to_string(), change["entry"].clone()); }
        cursor = page["cursor"].clone();
        if page["hasMore"] == false { break; }
    }
    assert_eq!(found["a"]["value"], json!({"vector":[1,0]}));
    assert!(found["b"].is_object());
    assert!(found["b"]["value"].is_null());
    assert!(f.call(json!({"op":"commit","handle":writer,"mutations":[put("rolled-back",json!(1)),{"collection":"keys","key":"a","value":2,"deleted":false,"checkVersion":true,"expectedVersion":"obsolete"}]})).is_err());
    assert_eq!(f.call(json!({"op":"changes","handle":reader,"after":cursor,"limit":10})).unwrap()["changes"],json!([]));
    f.call(json!({"op":"commit","handle":writer,"mutations":[{"collection":"keys","key":"a","value":null,"deleted":true,"checkVersion":false}]})).unwrap();
    let deleted = f.call(json!({"op":"changes","handle":reader,"after":cursor,"limit":10})).unwrap();
    assert_eq!(deleted["changes"],json!([{"collection":"keys","key":"a","entry":null}]));
    assert!(f.call(json!({"op":"changes","handle":reader,"after":null,"limit":0})).is_err());
    assert!(f.call(json!({"op":"changes","handle":reader,"after":"broken","limit":10})).is_err());
    f.dispose();
}

/// An already-open reader sees synchronized transactions and synthetic merge writes.
#[test]
fn object_change_pages_include_sync_and_merged_identities() {
    let first = Fixture::new("first");
    let source = open_object_merge_fixture(&first);
    seed_object_card(&first, &source, "card-a", "same", "identical");
    let second = Fixture::new("second");
    let reader = open_object_merge_fixture(&second);
    seed_object_card(&second, &reader, "card-z", "same", "identical");
    let before = second.call(json!({"op":"changes","handle":reader,"after":null,"limit":1000})).unwrap();
    second.replay(&first.export());
    let page = second.call(json!({"op":"changes","handle":reader,"after":before["cursor"],"limit":1000})).unwrap();
    assert_eq!(page["reset"],false);
    let changes = page["changes"].as_array().unwrap();
    assert!(changes.iter().any(|change| change["key"] == "card-a" && change["entry"].is_object()));
    assert!(changes.iter().any(|change| change["key"] == "card-z" && change["entry"].is_null()));
    first.dispose(); second.dispose();
}

/// A recreated database and a consumer falling behind bounded retention require a fresh snapshot.
#[test]
fn object_change_pages_reset_for_foreign_and_trimmed_cursors() {
    use operit_host_api::RuntimeSqliteHost;
    let f = Fixture::new("first");
    let handle = f.open("objects");
    let initial = f.call(json!({"op":"changes","handle":handle,"after":null,"limit":10})).unwrap();
    let foreign = f.call(json!({"op":"changes","handle":handle,"after":"foreign#0","limit":10})).unwrap();
    assert_eq!(foreign["reset"],true);
    // Feed a real transaction through the production triggers without 66k bridge round trips.
    let mut connection = f.host.openSqliteDatabase(&f.path).unwrap();
    connection.executeBatch("BEGIN; WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<66560) INSERT INTO _operit_changes(collection,key) SELECT 'keys',CAST(i AS TEXT) FROM n; COMMIT;").unwrap();
    drop(connection);
    let page = f.call(json!({"op":"changes","handle":handle,"after":initial["cursor"],"limit":10})).unwrap();
    assert_eq!(page["reset"],true);
    assert_eq!(page["changes"],json!([]));
    f.dispose();
}
