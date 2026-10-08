use super::{installTestHosts, DATABASE_MUTEX, NEXT_ID};

use std::sync::atomic::Ordering;

use crate::repository::MemoryRepository::MemoryRepository;
use crate::SyncOperationStore::{SyncClock, SyncOperationStore};

/// Verifies matching, missing, filtered, wildcard, and blank searches preserve entities and sync clocks.
#[test]
fn memory_search_preserves_entities_and_does_not_record_sync_operations() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    installTestHosts();
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let repository = MemoryRepository::new(format!("character:read-only-search-{id}"));
    let mut alpha = repository
        .createMemory(
            "alpha".to_string(),
            "first memory".to_string(),
            "text/plain".to_string(),
            "test".to_string(),
            "work".to_string(),
            Some(Vec::new()),
        )
        .unwrap();
    alpha.lastAccessedAt = 17;
    let alpha = repository.saveMemory(alpha).unwrap();
    let mut beta = repository
        .createMemory(
            "beta".to_string(),
            "second memory".to_string(),
            "text/plain".to_string(),
            "test".to_string(),
            "work".to_string(),
            Some(Vec::new()),
        )
        .unwrap();
    beta.lastAccessedAt = 23;
    let beta = repository.saveMemory(beta).unwrap();
    let before = repository.getMemoriesByFolderPath("work").unwrap();
    let syncStore =
        SyncOperationStore::native(crate::RuntimeStorePaths::RuntimeStorePaths::default());
    let clockBefore = syncStore.localClock().unwrap();
    let cases = [
        ("alpha", 0.1, Some("work"), None, None, vec![alpha]),
        ("missing", 0.1, None, None, None, Vec::new()),
        ("beta", 0.1, Some("other"), None, None, Vec::new()),
        ("alpha", 2.0, None, None, None, Vec::new()),
        ("*", 0.0, None, Some(i64::MAX), None, Vec::new()),
        ("", 0.0, None, None, Some(-1), Vec::new()),
        ("*", 2.0, Some("work"), None, None, before.clone()),
        ("", 2.0, Some("work"), None, None, before.clone()),
        ("   ", 2.0, Some("work"), None, None, before.clone()),
        ("beta", 0.1, None, None, None, vec![beta]),
    ];
    for (query, threshold, folder, start, end, expected) in cases {
        let found = repository
            .searchMemories(query, folder, threshold, start, end)
            .unwrap();
        assert_eq!(found, expected, "unexpected results for {query:?}");
        assert_eq!(
            repository.getMemoriesByFolderPath("work").unwrap(),
            before,
            "search changed stored entities for {query:?}"
        );
        assert_eq!(
            syncStore.localClock().unwrap(),
            clockBefore,
            "search advanced the sync clock for {query:?}"
        );
        assert!(
            syncStore
                .operationsSince(
                    &clockBefore,
                    &[crate::ObjectBoxStore::OBJECTBOX_SYNC_DOMAIN.to_string()],
                    usize::MAX,
                )
                .unwrap()
                .is_empty(),
            "search recorded sync operations for {query:?}"
        );
    }
}

/// Verifies searches on an empty memory store leave its synchronization state unchanged.
#[test]
fn memory_search_on_empty_store_does_not_record_sync_operations() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    installTestHosts();
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let repository = MemoryRepository::new(format!("character:empty-search-{id}"));
    let syncStore =
        SyncOperationStore::native(crate::RuntimeStorePaths::RuntimeStorePaths::default());
    let clockBefore: SyncClock = syncStore.localClock().unwrap();
    for query in ["alpha", "*", "", "   "] {
        assert!(repository
            .searchMemories(query, None, 0.0, None, None)
            .unwrap()
            .is_empty());
        assert_eq!(syncStore.localClock().unwrap(), clockBefore);
        assert!(syncStore
            .operationsSince(
                &clockBefore,
                &[crate::ObjectBoxStore::OBJECTBOX_SYNC_DOMAIN.to_string()],
                usize::MAX,
            )
            .unwrap()
            .is_empty());
    }
}

#[test]
fn owner_settings_keep_schedule_isolation_normalization_and_queue_retry() {
    use crate::repository::MemoryAutoSaveCandidateRepository::MemoryAutoSaveCandidateRepository;
    use crate::repository::MemorySettingsRepository::MemorySettingsRepository;
    let _guard = DATABASE_MUTEX.lock().unwrap();
    installTestHosts();
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let owner = format!("character:settings-{id}");
    let other = format!("shared:settings-{id}");
    let repo = MemorySettingsRepository::new(&owner);
    let isolated = MemorySettingsRepository::new(&other);
    assert_eq!(repo.load().unwrap().autoSaveIntervalMinutes, 5);
    assert_eq!(repo.nextRunAt(1_000).unwrap(), 301_000);
    repo.scheduleNextRun(400_000).unwrap();
    let mut settings = repo.load().unwrap();
    settings.nextAutoSaveRunAtMs = 1;
    settings.memoryExtractionCustomRules = "retain facts".into();
    repo.save(settings).unwrap();
    assert_eq!(repo.load().unwrap().nextAutoSaveRunAtMs, 400_000);
    assert_eq!(isolated.load().unwrap().nextAutoSaveRunAtMs, 0);
    let mut settings = repo.load().unwrap();
    settings.autoSaveIntervalMinutes = 99;
    repo.save(settings).unwrap();
    assert_eq!(repo.load().unwrap().autoSaveIntervalMinutes, 30);
    let candidates = MemoryAutoSaveCandidateRepository::new(&owner);
    candidates
        .enqueueSelectedUserMessages("chat".into(), vec![3, 2, 3])
        .unwrap();
    let queued = candidates.getPendingAndFailedCandidates().unwrap();
    assert_eq!(queued.len(), 2);
    let ids = queued.iter().map(|c| c.id).collect::<Vec<_>>();
    candidates.markProcessing(&ids).unwrap();
    assert!(candidates
        .getPendingAndFailedCandidates()
        .unwrap()
        .is_empty());
    candidates.markFailed(&ids, "provider error").unwrap();
    assert_eq!(candidates.getPendingAndFailedCandidates().unwrap().len(), 2);
    assert!(MemoryAutoSaveCandidateRepository::new(&other)
        .allCandidates()
        .unwrap()
        .is_empty());
    candidates.deleteCandidates(&ids).unwrap();
    assert!(candidates.allCandidates().unwrap().is_empty());
}

#[test]
fn persisted_weights_control_tag_and_graph_search() {
    use crate::repository::MemorySettingsRepository::MemorySettingsRepository;
    use operit_model::MemorySearchConfig::MemorySearchConfig;
    let _guard = DATABASE_MUTEX.lock().unwrap();
    installTestHosts();
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let owner = format!("shared:hybrid-{id}");
    let repo = MemoryRepository::new(&owner);
    let a = repo
        .createMemory(
            "Coffee preference".into(),
            "User likes coffee".into(),
            "text/plain".into(),
            "test".into(),
            "".into(),
            Some(vec!["espresso".into()]),
        )
        .unwrap();
    let b = repo
        .createMemory(
            "Morning routine".into(),
            "Drinks before work".into(),
            "text/plain".into(),
            "test".into(),
            "".into(),
            None,
        )
        .unwrap();
    repo.linkMemories(a.id, b.id, "RELATED".into(), 0.8, "Routine".into())
        .unwrap();
    let settings = MemorySettingsRepository::new(&owner);
    let mut config = MemorySearchConfig::default();
    config.keywordWeight = 0.0;
    config.tagWeight = 10.0;
    config.edgeWeight = 0.4;
    settings.saveSearchConfig(config).unwrap();
    let result = repo
        .searchMemories("espresso", None, 0.0, None, None)
        .unwrap();
    assert!(result.iter().any(|m| m.id == a.id));
    assert!(result.iter().any(|m| m.id == b.id));
    let mut config = settings.loadSearchConfig().unwrap();
    config.tagWeight = 0.0;
    settings.saveSearchConfig(config).unwrap();
    assert!(repo
        .searchMemories("espresso", None, 0.0, None, None)
        .unwrap()
        .is_empty());
}

#[test]
fn document_paragraphs_are_owner_scoped_searchable_and_deleted_with_parent() {
    let _guard = DATABASE_MUTEX.lock().unwrap();
    installTestHosts();
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let repo = MemoryRepository::new(format!("character:document-{id}"));
    let memory = repo
        .createMemoryFromDocument(
            "Tool notes".into(),
            "/notes.txt".into(),
            "---\nSSH connection recovered\n\nTMUX close works\n\n===\n".into(),
            "Tools".into(),
        )
        .unwrap();
    assert!(memory.isDocumentNode);
    assert_eq!(repo.getTotalChunkCount(memory.id).unwrap(), 2);
    let chunks = repo
        .searchChunksInDocument(memory.id, "TMUX".into(), 20)
        .unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].chunkIndex, 1);
    assert!(repo
        .getChunkByIndex(memory.id, 0)
        .unwrap()
        .unwrap()
        .content
        .contains("SSH"));
    let isolated = MemoryRepository::new(format!("shared:document-{id}"));
    assert!(isolated.getChunksForMemory(memory.id).unwrap().is_empty());
    repo.deleteMemory(memory.id).unwrap();
    assert!(repo.getChunksForMemory(memory.id).unwrap().is_empty());
}
