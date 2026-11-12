//! Divergent re-sealed initializations of one shared SQL database must converge
//! instead of deadlocking synchronization with conflicting immutable operations.
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::SqliteRowGet;
use operit_store::SyncOperationStore::{SyncOperation, SyncOperationStore};
use operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore;
use operit_store::Transaction::{Backend, Data, Owner, Snapshot, compaction::Initialization};
use std::collections::BTreeSet;
use std::sync::Arc;

fn stored_initialization(store: &operit_store::SqliteStore::SqliteStore) -> SyncOperation {
    let row = store
        .queryOne(
            "SELECT original FROM _operit_transaction_initialization WHERE id=1",
            vec![],
        )
        .unwrap()
        .expect("replica has no initialization");
    serde_json::from_str(&row.get::<_, String>(0).unwrap()).unwrap()
}

fn initialization_id(store: &operit_store::SqliteStore::SqliteStore) -> Option<String> {
    store
        .queryOne(
            "SELECT operation_id FROM _operit_transaction_initialization WHERE id=1",
            vec![],
        )
        .unwrap()
        .map(|row| row.get::<_, String>(0).unwrap())
}

fn dataset<'a>(
    store: &'a operit_store::SqliteStore::SqliteStore,
    node: &str,
    owner: Owner,
) -> operit_store::Transaction::SqlDataset<'a> {
    operit_store::Transaction::SqlDataset {
        store,
        owner,
        node: node.into(),
        tables: &["probe"],
        revisions: "SELECT CAST(id AS TEXT),id,'probe',id FROM probe",
        cleanup: "",
        restore_schema: None,
    }
}

fn open_replica(
    host: &Arc<NativeRuntimeStorageHost>,
    path: &str,
    rows: i64,
) -> operit_store::SqliteStore::SqliteStore {
    let store = operit_store::SqliteStore::SqliteStore::openWithHost(host.clone(), path).unwrap();
    store
        .executeBatch("CREATE TABLE IF NOT EXISTS probe(id INTEGER PRIMARY KEY, content TEXT)")
        .unwrap();
    store
        .transaction(|tx| {
            for row in 0..rows {
                tx.execute(
                    "INSERT INTO probe VALUES(?1,?2)",
                    vec![
                        operit_host_api::SqliteValue::Integer(row),
                        operit_host_api::SqliteValue::Text("x".repeat(2048)),
                    ],
                )?;
            }
            Ok(())
        })
        .unwrap();
    store
}

/// A snapshot whose serialized initialization exceeds the inline transport budget
/// so the operation travels as a verified content-addressed blob.
fn large_snapshot(path: &str, bytes: usize) -> Snapshot {
    let mut data = Data::empty(Owner { backend: Backend::DataStore, path: path.into(), format: 1, layout: 1 });
    for row in 0..bytes / 4096 { data.values.insert(format!("row-{row:08}"), serde_json::Value::String("x".repeat(4096))); }
    Snapshot { damage: None, node: "memory-probe".into(), data, covered: BTreeSet::new(), generation: 0 }
}

/// A blob-transported initialization must complete its journal reception under the
/// same content-hash fingerprint it was received with, whatever form is handed back.
#[test]
fn blob_transport_initializations_complete_under_their_reception_fingerprint() {
    let root = std::env::temp_dir().join(format!("operit-init-blob-complete-{}", uuid::Uuid::new_v4()));
    let runtime = root.join("runtime");
    std::fs::create_dir_all(runtime.join("sync/operations")).unwrap();
    let host = Arc::new(NativeRuntimeStorageHost::new(runtime, root.join("workspaces")));
    let files = RuntimeFileSyncStore::new(host.clone(), "runtime/sync");
    let journal = SyncOperationStore::new(host.clone(), "runtime/sync");

    let original = Initialization::from_snapshot(&large_snapshot("runtime/config/preferences/demo.json", 96 * 1024))
        .operation("plugin_storage", false).unwrap();
    let wire = files.prepareTransactionForTransport(original).unwrap();
    assert!(
        RuntimeFileSyncStore::requiredBlob(&wire).unwrap().is_some(),
        "fixture must exercise the blob transport form"
    );

    // Production receives the hydrated operation and later completes the stored wire form.
    journal.receiveOperations(&[files.hydrateTransaction(wire.clone()).unwrap()]).unwrap();
    journal.completeOperations(std::slice::from_ref(&wire)).unwrap();
    assert!(
        journal.pendingOperations().unwrap().is_empty(),
        "completed initialization must leave the pending set"
    );

    // Repeated delivery of the same immutable operation stays conflict-free.
    journal.receiveOperations(&[files.hydrateTransaction(wire.clone()).unwrap()]).unwrap();
    journal.completeOperations(std::slice::from_ref(&wire)).unwrap();
    assert!(journal.pendingOperations().unwrap().is_empty());
}

/// One shared logical database, two replicas that each re-seal their own snapshot
/// (the upgrade-repair path seals with a freshly minted initialization identity).
#[test]
fn independently_resealed_replicas_converge_without_conflicting_operations() {
    let root = std::env::temp_dir().join(format!("operit-init-convergence-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let host = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    let files = RuntimeFileSyncStore::new(host.clone(), "runtime/sync");
    let owner = Owner {
        backend: Backend::Sql,
        path: "runtime/plugin_data/space/demo/memory.sqlite".into(),
        format: 1,
        layout: 1,
    };

    let a_store = open_replica(&host, "runtime/a.db", 40);
    let b_store = open_replica(&host, "runtime/b.db", 40);

    // Shared origin: B adopts A's first sealed initialization like a joining peer.
    let a_first = dataset(&a_store, "A", owner.clone());
    let base = a_first.snapshot().unwrap();
    assert!(a_first.repair(&base.token().unwrap(), &base.clone()).unwrap());
    drop(base);
    let shared = files.prepareTransactionForTransport(stored_initialization(&a_store)).unwrap();
    let b_first = dataset(&b_store, "B", owner.clone());
    let joined = files.hydrateTransaction(shared).unwrap();
    assert!(b_first.receive_initialization(&joined, |_, _| Ok(())).unwrap());
    drop(joined);
    assert_eq!(
        initialization_id(&a_store),
        initialization_id(&b_store),
        "joined replica must share the origin initialization"
    );

    // Upgrade night: both replicas re-seal their own divergent snapshots.
    a_store
        .execute("INSERT INTO probe VALUES(1000,?1)", vec![
            operit_host_api::SqliteValue::Text("a-local".repeat(512)),
        ])
        .unwrap();
    b_store
        .execute("INSERT INTO probe VALUES(2000,?1)", vec![
            operit_host_api::SqliteValue::Text("b-local".repeat(512)),
        ])
        .unwrap();
    for (store, node) in [(&a_store, "A"), (&b_store, "B")] {
        let replica = dataset(store, node, owner.clone());
        let snapshot = replica.snapshot().unwrap();
        assert!(replica.repair(&snapshot.token().unwrap(), &snapshot.clone()).unwrap());
        drop(snapshot);
    }
    assert_ne!(
        initialization_id(&a_store),
        initialization_id(&b_store),
        "each replica minted its own re-seal identity"
    );

    // Sync exchanges both re-sealed initializations in both directions.
    let a_op = files
        .hydrateTransaction(files.prepareTransactionForTransport(stored_initialization(&a_store)).unwrap())
        .unwrap();
    let b_op = files
        .hydrateTransaction(files.prepareTransactionForTransport(stored_initialization(&b_store)).unwrap())
        .unwrap();
    let b_accepted = dataset(&b_store, "B", owner.clone())
        .receive_initialization(&a_op, |_, _| Ok(()))
        .expect("receiving a peer re-seal must not conflict");
    let a_accepted = dataset(&a_store, "A", owner.clone())
        .receive_initialization(&b_op, |_, _| Ok(()))
        .expect("receiving a peer re-seal must not conflict");
    drop(a_op);
    drop(b_op);
    assert!(
        b_accepted || a_accepted,
        "one replica must adopt the other's re-sealed initialization"
    );
    assert_eq!(
        initialization_id(&a_store),
        initialization_id(&b_store),
        "both replicas must converge on one initialization"
    );

    // Repeated delivery of the settled initialization stays conflict-free.
    let settled = stored_initialization(&b_store);
    let receiver = dataset(&a_store, "A", owner.clone());
    let redelivered = files.hydrateTransaction(files.prepareTransactionForTransport(settled).unwrap()).unwrap();
    assert!(
        !receiver.receive_initialization(&redelivered, |_, _| Ok(())).unwrap(),
        "an already converged replica must skip the settled initialization"
    );
}
