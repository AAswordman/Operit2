//! Verifies the Space journal export keeps one page inside a byte budget.
use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::SyncOperationStore::{
    SyncClock, SyncOperation, SyncOperationSemantics, SyncOperationStore,
};
use serde_json::json;
use std::sync::Arc;

const SYNC: &str = "runtime/sync";

fn operation(sequence: i64, payloadBytes: usize) -> SyncOperation {
    SyncOperation {
        opId: format!("device:{sequence}"),
        originDeviceId: "device".into(),
        sequence,
        domain: "runtime_file".into(),
        entityType: "file".into(),
        entityId: format!("file-{sequence}"),
        operation: "upsert".into(),
        semantics: SyncOperationSemantics::EntityState,
        payload: json!({"content": "x".repeat(payloadBytes)}),
        createdAt: sequence,
        schemaVersion: 1,
    }
}

/// Byte budgets cut one journal page without reordering or splitting records.
#[test]
fn byte_budgets_bound_journal_pages_without_reordering() {
    let root = std::env::temp_dir().join(format!(
        "operit-sync-journal-{}",
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let host = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    let store = SyncOperationStore::new(host.clone(), SYNC);
    store
        .appendOperations(
            &(1..=6)
                .map(|sequence| operation(sequence, 48 * 1024))
                .collect::<Vec<_>>(),
        )
        .unwrap();
    let domains = vec!["runtime_file".to_string()];
    let all = store
        .operationsSince(&SyncClock::empty(), &domains, 100)
        .unwrap();
    assert_eq!(all.len(), 6);
    let budget = 64 * 1024;
    let page = store
        .operationsSinceWithBudget(&SyncClock::empty(), &domains, 100, budget, |_| true)
        .unwrap();
    assert!(!page.is_empty());
    assert!(page.len() < all.len(), "the byte budget must cut the page");
    // The page is an ordered prefix: no record is split or reordered.
    assert_eq!(
        page.iter()
            .map(|operation| operation.opId.clone())
            .collect::<Vec<_>>(),
        all.iter()
            .take(page.len())
            .map(|operation| operation.opId.clone())
            .collect::<Vec<_>>()
    );
    if page.len() > 1 {
        let bytes = page
            .iter()
            .map(|operation| serde_json::to_vec(operation).unwrap().len())
            .sum::<usize>();
        assert!(bytes <= budget, "a multi-record page must stay in budget");
    }
    drop(store);
    std::fs::remove_dir_all(&root).unwrap();
}
