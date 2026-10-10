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
            PluginStorage::applyOperation(self.host.clone(), self.host.clone(), operation, false)
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
        for kind in ["sqlite", "objects", "data_store"] {
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
            let changes = f.call(json!({"op":"changes","handle":handle,"after":"0","limit":100})).unwrap();
            assert!(!changes.as_array().unwrap().is_empty());
            assert_eq!(changes[0]["transaction"]["manifest"]["sync"], scope == "space");
            let operations = f.export();
            if scope == "space" {
                assert_eq!(operations.len(), changes.as_array().unwrap().len());
                assert!(operations.iter().all(|operation| operation.domain == PluginStorage::PLUGIN_STORAGE_SYNC_DOMAIN));
            } else {
                assert!(operations.is_empty());
            }
            f.call(json!({"op":"close","handle":handle})).unwrap();
            let reopened = f.open(kind);
            let reopened_changes = f.call(json!({"op":"changes","handle":reopened,"after":"0","limit":100})).unwrap();
            assert_eq!(reopened_changes, changes);
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
    let handle = f.open("data_store");
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("cached",json!(1))]})).unwrap();
    assert_eq!(f.get(&handle, "cached")["value"], 1);
    assert!(f.export().is_empty());
    RuntimeFileSyncStore::new(f.host.clone(), "runtime/sync")
        .trackChanges(&f.roots, || f.host.writeBytes(&format!("{local}/cache.txt"), b"local").map_err(|error| error.to_string()))
        .unwrap();
    assert!(SyncOperationStore::new(f.host.clone(), "runtime/sync").operationsSince(&SyncClock::empty(), &[], 100).unwrap().is_empty());
    let other = operit_store::ExtensionStore::ExtensionStore::localDataPath("another_plugin").unwrap();
    let otherOpened = f.call(json!({"op":"open","path":format!("{other}/cache.sqlite"),"kind":"data_store"})).unwrap();
    assert!(otherOpened["handle"].is_string());
    let otherSession = PluginStorageSession::default();
    let reopened = otherSession.request(f.host.clone(), f.host.clone(), serde_json::from_value(json!({"op":"open","path":f.path,"kind":"data_store"})).unwrap(), "second-script").unwrap();
    let otherHandle = reopened["handle"].as_str().unwrap();
    let stored = otherSession.request(f.host.clone(), f.host.clone(), serde_json::from_value(json!({"op":"get","handle":otherHandle,"collection":"keys","key":"cached"})).unwrap(), "second-script").unwrap();
    assert_eq!(stored["value"], 1);
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
            "kind":"data_store", "sync":sync
        })).is_err());
    }
}

/// Keeps null, missing keys, durable reopen and immutable storage policies distinct.
#[test]
fn datastore_preserves_null_and_reopens_without_changing_identity() {
    let f = Fixture::withScope("first", "device");
    let handle = f.open("data_store");
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
    let reopened = f.open("data_store");
    assert!(f.get(&reopened, "nullable")["version"].is_string());
    assert!(
        f.call(json!({"op":"open","path":f.path,"kind":"objects"}))
            .is_err()
    );
    assert!(
        f.call(
            json!({"op":"open","path":"runtime/outside.sqlite","kind":"data_store"})
        )
        .is_err()
    );
    assert!(f.call(json!({"op":"open","path":format!("{}/../escape.sqlite",f.roots[0]),"kind":"data_store"})).is_err());
    f.dispose();
}

/// Rolls back earlier writes and the change journal when a later key version conflicts.
#[test]
fn compare_and_set_batches_are_atomic() {
    let f = Fixture::withScope("first", "device");
    let handle = f.open("data_store");
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
    let pages = f
        .call(json!({"op":"changes","handle":handle,"after":"0","limit":100}))
        .unwrap();
    assert_eq!(pages.as_array().unwrap().len(), 1);
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
    let ah = a.open("data_store");
    a.call(json!({"op":"commit","handle":ah,"mutations":[put("key",Value::Null)]}))
        .unwrap();
    let first = a.export();
    b.replay(&first);
    b.replay(&first);
    let bh = b.open("data_store");
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
        PluginStorage::applyOperation(b.host.clone(), b.host.clone(), &conflict, false).is_err()
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
        PluginStorage::applyOperation(b.host.clone(), b.host.clone(), &invalid, false).is_err()
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
    let handle = f.open("data_store");
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
    let handle = f.open("data_store");
    f.call(json!({"op":"commit","handle":handle,"mutations":[put("key",json!(1))]}))
        .unwrap();
    let previous = f.export()[0].opId.clone();
    f.call(json!({"op":"close","handle":handle})).unwrap();
    PluginStorage::unregisterDatabase(f.host.clone(), &f.path).unwrap();
    f.host.delete(&f.path, false).unwrap();
    f.host
        .delete(&format!("{}.operit-storage.json", f.path), false)
        .unwrap();
    let handle = f.open("data_store");
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
    let localHandle = f.call(json!({"op":"open","path":localPath,"kind":"data_store"})).unwrap()["handle"].as_str().unwrap().to_string();
    f.call(json!({"op":"commit","handle":localHandle,"mutations":[put("cache",json!(7))]})).unwrap();
    let sharedHandle = f.open("data_store");
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
    let ah = a.open("data_store");
    a.call(json!({"op":"commit","handle":ah,"mutations":[put("key",json!(1))]}))
        .unwrap();
    let mut invalid = a.export()[0].clone();
    invalid.payload["changes"][0]["collection"] = json!("other");
    assert!(
        PluginStorage::applyOperation(b.host.clone(), b.host.clone(), &invalid, false).is_err()
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
    let ah = a.open("data_store");
    let bh = b.open("data_store");
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
    let sharedHandle = f.open("data_store");
    f.call(json!({"op":"commit","handle":sharedHandle,"mutations":[put("memory",json!(1))]})).unwrap();
    let local = operit_store::ExtensionStore::ExtensionStore::localDataPath("storage_test").unwrap();
    let localPath = format!("{local}/cache.sqlite");
    let localHandle = f.call(json!({"op":"open","path":localPath,"kind":"data_store"})).unwrap()["handle"].as_str().unwrap().to_string();
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
