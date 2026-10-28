//! Layout migrations establish destination ownership before retiring source operations.
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PreferenceLayout::PreferenceLayout,
    PreferencesDataStore::{
        stringPreferencesKey, CoreNodeStateStore, PreferencesDataStore, PreferencesDataStoreError,
    },
    SyncOperationStore::{SyncOperation, SyncOperationStore},
};
use serde_json::json;

const STATE: &str = "runtime/sync/test_preference_layout.preferences.json";
const OLD: &str = "runtime/config/preferences/retired.preferences.json";
const CURRENT: &str = "runtime/config/preferences/current.preferences.json";
const DIRECTORY: &str = "runtime/extensions/space/settings";

struct Fixture {
    root: PathBuf,
    host: Arc<NativeRuntimeStorageHost>,
}

impl Fixture {
    /// Creates an isolated storage profile for layout state, source values and journals.
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("operit-preference-layout-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        Self { root, host }
    }

    /// Opens one explicitly versioned business preference store.
    fn store(&self, path: &str) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(self.host.clone(), path).withSchema(2, |_, _| Ok(()))
    }

    /// Opens the durable receipt and immutable operation archive.
    fn journal(&self) -> SyncOperationStore {
        SyncOperationStore::new(self.host.clone(), "runtime/sync")
    }

    /// Seeds the local source without authoring synchronized business operations.
    fn seed(&self) {
        CoreNodeStateStore::newWithStorage(self.host.clone(), OLD)
            .edit(|preferences| {
                preferences.set(&stringPreferencesKey("settings"), json!({"enabled":true}))
            })
            .unwrap();
    }
}

impl Drop for Fixture {
    /// Deletes only this fixture's explicitly created temporary profile.
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

/// Defines two ordered layout steps using the DataStore's existing schema dispatcher.
fn layout(
    storage: Arc<dyn RuntimeStorageHost>,
    attempts: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
) -> PreferenceLayout {
    PreferenceLayout::new(storage, STATE).withSchema(2, move |from, migration| match from {
        0 => {
            attempts.fetch_add(1, Ordering::SeqCst);
            let source =
                CoreNodeStateStore::newWithStorage(migration.storage.clone(), OLD).data()?;
            let settings = source
                .get(&stringPreferencesKey("settings"))
                .cloned()
                .ok_or_else(|| {
                    PreferencesDataStoreError::Message(
                        "Migration source is missing settings".into(),
                    )
                })?;
            let destination =
                PreferencesDataStore::newWithStorage(migration.storage.clone(), CURRENT)
                    .withSchema(2, |_, _| Ok(()));
            migration.create(&destination)?;
            destination
                .edit(|preferences| preferences.set(&stringPreferencesKey("settings"), settings))?;
            if fail.load(Ordering::SeqCst) {
                return Err(PreferencesDataStoreError::Message(
                    "Interrupted before ownership transfer".into(),
                ));
            }
            migration.retireFile(OLD);
            Ok(())
        }
        1 => {
            migration.retireDirectory(DIRECTORY);
            Ok(())
        }
        from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
    })
}

/// Generates exactly fifteen original operations across one file and two retired namespace members.
fn originals(source: &Fixture) -> Vec<SyncOperation> {
    for path in [
        OLD.to_owned(),
        format!("{DIRECTORY}/package-a.preferences.json"),
        format!("{DIRECTORY}/skill-b.preferences.json"),
    ] {
        let store = source.store(&path);
        for revision in 1..=5 {
            store
                .edit(|preferences| {
                    preferences.set(
                        &stringPreferencesKey("settings"),
                        json!({"revision":revision}),
                    )
                })
                .unwrap();
        }
    }
    let operations = source.journal().preferenceHistory().unwrap();
    assert_eq!(operations.len(), 15);
    operations
}

/// Retired originals finish durably without recreating files or replacing current-owner edits.
#[test]
fn fifteen_retired_operations_complete_after_local_layout_migration() {
    let source = Fixture::new();
    let incoming = originals(&source);
    let target = Fixture::new();
    target.seed();
    let attempts = Arc::new(AtomicUsize::new(0));
    let manager = layout(
        target.host.clone(),
        attempts.clone(),
        Arc::new(AtomicBool::new(false)),
    );
    manager.prepare().unwrap();
    let destination = target.store(CURRENT);
    assert_eq!(
        destination
            .data()
            .unwrap()
            .get(&stringPreferencesKey("settings")),
        Some(&json!({"enabled":true}))
    );
    destination
        .edit(|preferences| {
            preferences.set(&stringPreferencesKey("settings"), json!({"enabled":false}))
        })
        .unwrap();
    let current = target.host.readBytes(CURRENT).unwrap();
    let journal = target.journal();
    journal.receiveOperations(&incoming).unwrap();
    let mut pending = journal.pendingOperations().unwrap();
    let mut context = incoming.clone();
    assert_eq!(
        manager
            .prepareOperations(&mut context, &mut pending)
            .unwrap(),
        15
    );
    assert!(pending.is_empty());
    assert!(context.is_empty());
    assert_eq!(journal.pendingOperationCount().unwrap(), 0);
    assert_eq!(target.host.readBytes(CURRENT).unwrap(), current);
    assert!(!target
        .host
        .exists(&format!("{DIRECTORY}/package-a.preferences.json"))
        .unwrap());
    assert!(!target
        .host
        .exists(&format!("{DIRECTORY}/skill-b.preferences.json"))
        .unwrap());
    let history = journal.preferenceHistory().unwrap();
    for operation in &incoming {
        assert!(history.iter().any(|archived| archived == operation));
    }
    assert!(!history
        .iter()
        .any(|operation| operation.payload["storagePath"] == STATE));
    let reopened = Arc::new(NativeRuntimeStorageHost::new(
        target.root.join("runtime"),
        target.root.join("workspaces"),
    ));
    let manager = layout(
        reopened.clone(),
        attempts.clone(),
        Arc::new(AtomicBool::new(false)),
    );
    let journal = SyncOperationStore::new(reopened, "runtime/sync");
    journal.receiveOperations(&incoming).unwrap();
    assert_eq!(
        manager
            .prepareOperations(&mut Vec::new(), &mut journal.pendingOperations().unwrap())
            .unwrap(),
        0
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert_eq!(journal.preferenceHistory().unwrap(), history);
    assert_eq!(journal.pendingOperationCount().unwrap(), 0);
}

/// A failed ownership transfer preserves the layout version, input batches and pending originals.
#[test]
fn interrupted_layout_step_retries_before_retiring_any_operations() {
    let source = Fixture::new();
    let incoming = originals(&source);
    let target = Fixture::new();
    target.seed();
    let attempts = Arc::new(AtomicUsize::new(0));
    let fail = Arc::new(AtomicBool::new(true));
    let manager = layout(target.host.clone(), attempts.clone(), fail.clone());
    let journal = target.journal();
    journal.receiveOperations(&incoming).unwrap();
    let mut pending = journal.pendingOperations().unwrap();
    let before = pending.clone();
    let mut context = incoming.clone();
    assert!(manager
        .prepareOperations(&mut context, &mut pending)
        .is_err());
    assert_eq!(context, incoming);
    assert_eq!(pending, before);
    assert_eq!(journal.pendingOperationCount().unwrap(), 15);
    assert!(!target.host.exists(STATE).unwrap());
    assert!(target.host.exists(CURRENT).unwrap());
    fail.store(false, Ordering::SeqCst);
    assert_eq!(
        manager
            .prepareOperations(&mut context, &mut pending)
            .unwrap(),
        15
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(journal.pendingOperationCount().unwrap(), 0);
    assert_eq!(
        target
            .store(CURRENT)
            .data()
            .unwrap()
            .get(&stringPreferencesKey("settings")),
        Some(&json!({"enabled":true}))
    );
}

/// Retirement honors directory boundaries and refuses to run under an older layout declaration.
#[test]
fn retirement_is_scoped_and_a_future_layout_blocks_application() {
    let source = Fixture::new();
    let path = format!("{DIRECTORY}-current/skill.preferences.json");
    source
        .store(&path)
        .edit(|preferences| preferences.set(&stringPreferencesKey("value"), true))
        .unwrap();
    let incoming = source.journal().preferenceHistory().unwrap();
    let target = Fixture::new();
    target.seed();
    let manager = layout(
        target.host.clone(),
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
    );
    let mut pending = incoming.clone();
    assert_eq!(
        manager
            .prepareOperations(&mut Vec::new(), &mut pending)
            .unwrap(),
        0
    );
    assert_eq!(pending, incoming);
    let older = PreferenceLayout::new(target.host.clone(), STATE).withSchema(1, |_, _| Ok(()));
    assert!(older
        .prepareOperations(&mut Vec::new(), &mut pending)
        .is_err());
    assert_eq!(pending, incoming);
}

/// Remote preference operations cannot change the node's layout progress or retirement declarations.
#[test]
fn layout_progress_is_never_a_remote_business_dataset() {
    let source = Fixture::new();
    source
        .store(STATE)
        .edit(|preferences| {
            preferences.set(
                &stringPreferencesKey(
                    "retired_file:runtime/config/preferences/current.preferences.json",
                ),
                true,
            )
        })
        .unwrap();
    let incoming = source.journal().preferenceHistory().unwrap();
    let target = Fixture::new();
    target.seed();
    let manager = layout(
        target.host.clone(),
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicBool::new(false)),
    );
    manager.prepare().unwrap();
    let committed = target.host.readBytes(STATE).unwrap();
    target.journal().receiveOperations(&incoming).unwrap();
    let mut pending = target.journal().pendingOperations().unwrap();
    assert!(manager
        .prepareOperations(&mut Vec::new(), &mut pending)
        .is_err());
    assert_eq!(target.host.readBytes(STATE).unwrap(), committed);
    assert_eq!(pending, incoming);
    assert_eq!(target.journal().pendingOperationCount().unwrap(), 1);
}
