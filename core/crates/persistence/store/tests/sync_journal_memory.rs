//! Guards the journal's working set, rather than only the size of its returned page.
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::SyncOperationStore::{SyncClock, SyncOperationKnowledge, SyncOperationStore};
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
