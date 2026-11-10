//! Guards the journal's working set, rather than only the size of its returned page.
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::SyncOperationStore::{SyncClock, SyncOperationKnowledge, SyncOperationStore, SyncReceipt, SyncOperation, SyncOperationSemantics};
use operit_store::RuntimeFileSyncStore::RuntimeFileSyncStore;
use operit_store::Transaction::{Backend, Data, Owner, Snapshot, compaction::Initialization, kernel::{Registry, Participant}};
use std::collections::{BTreeMap, BTreeSet};
use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
struct MeasuredAllocator;
fn allocated(size: usize) {
    let live = LIVE.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(live, Ordering::Relaxed);
}
unsafe impl GlobalAlloc for MeasuredAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc_zeroed(layout);
        if !pointer.is_null() {
            allocated(layout.size());
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(pointer, layout);
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = System.realloc(pointer, layout, size);
        if !next.is_null() {
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            allocated(size);
        }
        next
    }
}
#[global_allocator]
static ALLOCATOR: MeasuredAllocator = MeasuredAllocator;

// This binary intentionally has one test, so measurements do not include other tests.
#[test]
fn small_pages_and_metadata_queries_do_not_materialize_or_pin_business_history() {
    production_sync_memory_budget();
    let root = std::env::temp_dir().join(format!("operit-journal-memory-{}", uuid::Uuid::new_v4()));
    let runtime = root.join("runtime");
    std::fs::create_dir_all(runtime.join("sync/operations")).unwrap();
    std::fs::write(runtime.join("sync/devices.json"), b"[\"probe\"]").unwrap();
    let host = Arc::new(NativeRuntimeStorageHost::new(
        runtime.clone(),
        root.join("workspaces"),
    ));
    let weak_host = Arc::downgrade(&host);
    let path = runtime.join("sync/operations/probe.jsonl");
    {
        let mut output = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
        let fields = (0..128)
            .map(|key| format!("\"field_{key}\":{key}"))
            .collect::<Vec<_>>()
            .join(",");
        for sequence in 1..=4096 {
            writeln!(output, "{{\"opId\":\"probe:{sequence}\",\"originDeviceId\":\"probe\",\"sequence\":{sequence},\"domain\":\"preferences\",\"entityType\":\"entry\",\"entityId\":\"entry-{sequence}\",\"operation\":\"upsert\",\"semantics\":\"entity_state\",\"payload\":{{\"storagePath\":\"runtime/data/probe.json\",\"data\":{{{fields}}}}},\"createdAt\":{sequence},\"schemaVersion\":1}}").unwrap();
        }
    }
    let bytes = std::fs::metadata(&path).unwrap().len();
    assert!(bytes > 8 * 1024 * 1024);
    let baseline = LIVE.load(Ordering::Relaxed);
    let domains = ["preferences".to_owned()];
    for mode in 0..3 {
        PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
        let store = SyncOperationStore::new(host.clone(), "runtime/sync");
        if mode == 0 {
            assert!(store.preferenceMergeHistory().unwrap().is_empty());
        } else {
            let filter = if mode == 1 { &domains[..] } else { &[] };
            let page = store
                .operationsSinceWithBudget(&SyncClock::empty(), filter, 1, 32 * 1024, |_| true)
                .unwrap();
            assert_eq!(page.len(), 1);
            assert_eq!(page[0].sequence, 1);
        }
        let peak = PEAK.load(Ordering::Relaxed).saturating_sub(baseline);
        // The old paths use 69–123 MB on exactly this 8.6 MB fixture.
        // Leave ample room for metadata/index overhead across architectures.
        assert!(
            peak < 12 * 1024 * 1024,
            "mode={mode}: hidden history working set {peak} bytes"
        );
        drop(store);
        let retained = LIVE.load(Ordering::Relaxed).saturating_sub(baseline);
        assert!(
            retained < 256 * 1024,
            "mode={mode}: released journal pinned {retained} bytes"
        );
    }
    // Legacy replay intent must migrate/count without constructing its full Value map.
    std::fs::create_dir_all(runtime.join("received")).unwrap();
    let ledger = runtime.join("received/receive_state.json");
    {
        let mut output = std::io::BufWriter::new(std::fs::File::create(&ledger).unwrap());
        let fields = (0..128)
            .map(|key| format!("\"field_{key}\":{key}"))
            .collect::<Vec<_>>()
            .join(",");
        write!(
            output,
            "{{\"receipt\":{{\"ranges\":{{\"probe\":[[1,512]]}}}},\"pending\":{{"
        )
        .unwrap();
        for sequence in 1..=512 {
            if sequence != 1 {
                write!(output, ",").unwrap();
            }
            write!(output, "\"probe:{sequence}\":{{\"opId\":\"probe:{sequence}\",\"originDeviceId\":\"probe\",\"sequence\":{sequence},\"domain\":\"preferences\",\"entityType\":\"entry\",\"entityId\":\"entry-{sequence}\",\"operation\":\"upsert\",\"semantics\":\"entity_state\",\"payload\":{{\"data\":{{{fields}}}}},\"createdAt\":{sequence},\"schemaVersion\":1}}").unwrap();
        }
        write!(output, "}},\"fingerprints\":{{}}}}").unwrap();
    }
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    let store = SyncOperationStore::new(host.clone(), "runtime/received");
    assert_eq!(store.pendingOperationCount().unwrap(), 512);
    assert!(store.receipt().unwrap().containsSequence("probe", 512));
    assert!(PEAK.load(Ordering::Relaxed).saturating_sub(before) < 4 * 1024 * 1024);
    assert_eq!(
        store
            .operationsSinceWithBudget(&SyncClock::empty(), &[], 1, 32 * 1024, |_| true)
            .unwrap()
            .len(),
        1
    );
    assert!(PEAK.load(Ordering::Relaxed).saturating_sub(before) < 4 * 1024 * 1024);
    drop(store);
    assert!(LIVE.load(Ordering::Relaxed).saturating_sub(before) < 256 * 1024);
    drop(host);
    assert!(
        weak_host.upgrade().is_none(),
        "the registry must not own a Host"
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn measured<T>(label: &str, action: impl FnOnce() -> T) -> (T, usize) {
    let baseline = LIVE.load(Ordering::SeqCst);
    PEAK.store(baseline, Ordering::SeqCst);
    let value = action();
    let peak = PEAK.load(Ordering::SeqCst).saturating_sub(baseline);
    println!("SYNC_MEMORY {label} peak_bytes={peak}");
    (value, peak)
}
fn new_registry(journal: SyncOperationStore) -> Registry<'static> {
    Registry { journal, domains: BTreeSet::new(), owners: BTreeMap::new(), sources: Vec::new(), routes: BTreeMap::new(), retired: BTreeSet::new(), locate: Box::new(|op| Ok(op.entityId.clone())) }
}
fn snapshot(path: &str, bytes: usize) -> Snapshot {
    let mut data = Data::empty(Owner { backend: Backend::DataStore, path: path.into(), format: 1, layout: 1 });
    // Many bounded records reflect a large database rather than one unusually large string.
    for row in 0..bytes / 4096 { data.values.insert(format!("row-{row:08}"), serde_json::Value::String("x".repeat(4096))); }
    Snapshot { damage: None, node: "memory-probe".into(), data, covered: BTreeSet::new(), generation: 0 }
}

// One test keeps allocator measurements isolated from other test threads.
fn production_sync_memory_budget() {
    let observe = std::env::var_os("OPERIT_SYNC_MEMORY_BASELINE").is_some();
    let mib = std::env::var("OPERIT_SYNC_PROBE_MIB").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(32);
    let sql_mib=std::env::var("OPERIT_SYNC_SQL_PROBE_MIB").ok().and_then(|v|v.parse::<usize>().ok()).unwrap_or(mib);
    let root = std::env::temp_dir().join(format!("operit-transaction-memory-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let host = Arc::new(NativeRuntimeStorageHost::new(root.join("runtime"), root.join("workspaces")));
    let journal = SyncOperationStore::new(host.clone(), "runtime/sync");
    for sequence in 1..=64 {
        let op = SyncOperation { opId: format!("probe:{sequence}"), originDeviceId: "probe".into(), sequence, domain: "runtime_file".into(), entityType: "file".into(), entityId: format!("file-{sequence}"), operation: "upsert".into(), semantics: SyncOperationSemantics::Transaction, payload: serde_json::json!({"content":"x".repeat(512 * 1024)}), createdAt: sequence, schemaVersion: 1 };
        journal.appendOperations(&[op]).unwrap();
    }
    let registry = new_registry(journal.clone());
    let (page, export_peak) = measured("registry_export_32MiB", || registry.export(&SyncReceipt::default(), &[], 512).unwrap().0);
    println!("SYNC_MEMORY registry_export returned_records={}", page.len());
    drop(page);
    drop(registry);

    let files = RuntimeFileSyncStore::new(host.clone(), "runtime/sync");
    let ((wire, expected), encode_peak) = measured("initialization_export_from_snapshot", || {
        let original = Initialization::from_snapshot(&snapshot("runtime/config/preferences/memory.preferences.json", mib * 1024 * 1024)).operation("preferences", false).unwrap();
        let expected = operit_store::Transaction::digest(&original).unwrap();
        (files.prepareTransactionForTransport(original).unwrap(), expected)
    });
    let reference = RuntimeFileSyncStore::requiredBlob(&wire).unwrap().unwrap();
    println!("SYNC_MEMORY initialization blob_bytes={}", reference.size);
    let (restored, decode_peak) = measured("initialization_receive", || files.hydrateTransaction(wire).unwrap());
    assert_eq!(operit_store::Transaction::digest(&restored).unwrap(), expected);
    drop(restored);

    // Exercise actual SQL installation, rollback, redelivery and CAS, not just transport decoding.
    let source = operit_store::SqliteStore::SqliteStore::openWithHost(host.clone(), "runtime/source.db").unwrap();
    let target = operit_store::SqliteStore::SqliteStore::openWithHost(host.clone(), "runtime/target.db").unwrap();
    for store in [&source, &target] { store.executeBatch("CREATE TABLE probe(id INTEGER PRIMARY KEY, content TEXT)").unwrap(); }
    source.transaction(|tx| {
        for row in 0..sql_mib * 256 { tx.execute("INSERT INTO probe VALUES(?1,?2)", vec![operit_host_api::SqliteValue::Integer(row as i64), operit_host_api::SqliteValue::Text("x".repeat(4096))])?; }
        Ok(())
    }).unwrap();
    let dataset = |store| operit_store::Transaction::SqlDataset { store, owner: Owner { backend: Backend::Sql,
        path: operit_util::RuntimeStorageLayout::SQLITE_DATABASE_PATH.into(), format: 1, layout: 1 }, node: "probe".into(), tables: &["probe"],
        revisions: "SELECT CAST(id AS TEXT),0,'probe',id FROM probe", cleanup: "", restore_schema: None };
    println!("SYNC_MEMORY sql_fixture_mib={sql_mib}");
    let ((wire, source_digest), snapshot_peak) = measured("sql_initialization_export_from_database", || {
        let source_data = dataset(&source).snapshot().unwrap();
        let digest = operit_store::Transaction::digest(&source_data.data).unwrap();
        let operation = Initialization::from_snapshot(&source_data).operation("chat", false).unwrap();
        drop(source_data);
        (files.prepareTransactionForTransport(operation).unwrap(), digest)
    });
    println!("SYNC_MEMORY sql_blob_bytes={}",RuntimeFileSyncStore::requiredBlob(&wire).unwrap().unwrap().size);
    let sql_owner = dataset(&target);
    let empty = sql_owner.snapshot().unwrap();
    let (_, apply_peak) = measured("sql_initialization_receive_from_blob", || {
        let sql_op = files.hydrateTransaction(wire).unwrap();
        assert!(sql_owner.receive_initialization(&sql_op, |_,_| Err(operit_store::SqliteStore::SqliteStoreError::Message("injected rollback".into()))).is_err());
        assert_eq!(sql_owner.snapshot().unwrap(), empty);
        assert!(sql_owner.receive_initialization(&sql_op, |_,_| Ok(())).unwrap());
        assert!(!sql_owner.receive_initialization(&sql_op, |_,_| Ok(())).unwrap());
    });
    let (_, verify_peak) = measured("sql_initialization_verify_and_stale_cas", || {
        let installed = sql_owner.snapshot().unwrap();
        assert_eq!(operit_store::Transaction::digest(&installed.data).unwrap(), source_digest);
        assert!(!sql_owner.repair(&empty.token().unwrap(), &installed).unwrap());
    });
    drop(empty); drop(source); drop(target);

    let mut registry = new_registry(journal);
    for owner in 0..16 {
        let path = format!("runtime/config/preferences/probe-{owner}.json");
        let source = path.clone();
        registry.register(Participant { path, prepare: Box::new(||Ok(())), validate:None, apply:Box::new(|_,_|Ok(BTreeMap::new())), project:None,
            snapshot:Some(Box::new(move || Ok(snapshot(&source, 2 * 1024 * 1024)))), definition:None, coverage:Box::new(||Ok(BTreeSet::new())), content_digest:Snapshot::content_digest, repair:None }).unwrap();
    }
    let ((manifests, _, errors), manifest_peak) = measured("sixteen_2MiB_manifests", || registry.manifests().unwrap());
    assert_eq!(manifests.len(), 16); assert!(errors.is_empty());
    if !observe {
        assert!(export_peak < 16 * 1024 * 1024, "export materialized hidden pages: {export_peak}");
        assert!(manifest_peak < 8 * 1024 * 1024, "manifests retained other owners: {manifest_peak}");
    }
    drop(registry); drop(files); drop(host);
    std::fs::remove_dir_all(root).unwrap();
    // A constant ceiling: increasing the fixture must never increase the permitted working set.
    // These Rust-only peaks can prove failure; passing also requires native/RSS measurements.
    if !observe {
        for (stage, peak) in [("source initialization", encode_peak), ("hydrate", decode_peak),
            ("SQL export", snapshot_peak), ("SQL receive", apply_peak), ("SQL verify", verify_peak)] {
            assert!(peak <= 128 * 1024 * 1024, "{stage}: preferences={mib} MiB SQL={sql_mib} MiB exceeded the fixed 128 MiB sync budget: {peak} bytes");
        }
    }
}
