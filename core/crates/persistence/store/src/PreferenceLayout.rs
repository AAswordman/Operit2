//! Versions preference creation, ownership transfer and retirement through DataStore migrations.
use std::collections::BTreeSet;
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;

use crate::PreferencesDataStore::{
    stringPreferencesKey, CoreNodeStateStore, Preferences, PreferencesDataStore,
    PreferencesDataStoreError, PreferencesSyncedEntry,
};
use crate::SyncOperationStore::{SyncOperation, SyncOperationStore};

const RETIRED_FILE: &str = "retired_file:";
const RETIRED_DIRECTORY: &str = "retired_directory:";

/// Commits a node's layout version and retirement declarations after its business migrations succeed.
pub struct PreferenceLayout {
    storage: Arc<dyn RuntimeStorageHost>,
    state: CoreNodeStateStore,
}

/// Provides the storage host and retirement declarations for one layout migration step.
pub struct PreferenceLayoutMigration<'a> {
    pub storage: Arc<dyn RuntimeStorageHost>,
    state: &'a mut Preferences,
}

impl PreferenceLayout {
    /// Opens node-local layout progress independently of synchronized business preferences.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>, path: &str) -> Self {
        Self {
            state: CoreNodeStateStore::newWithStorage(storage.clone(), path),
            storage,
        }
    }

    /// Uses the DataStore's version dispatcher and atomic commit for layout migration progress.
    pub fn withSchema<F>(mut self, version: u32, migrate: F) -> Self
    where
        F: Fn(u32, &mut PreferenceLayoutMigration<'_>) -> Result<(), PreferencesDataStoreError>
            + Send
            + Sync
            + 'static,
    {
        let storage = self.storage.clone();
        self.state = self.state.withSchema(version, move |from, state| {
            migrate(
                from,
                &mut PreferenceLayoutMigration {
                    storage: storage.clone(),
                    state,
                },
            )
        });
        self
    }

    /// Completes all required layout steps before preference preparation or synchronization.
    pub fn prepare(&self) -> Result<(), String> {
        self.state
            .data()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Archives retired originals before removing their replay intent and the caller's active batch.
    pub fn prepareOperations(
        &self,
        history: &mut Vec<SyncOperation>,
        operations: &mut Vec<SyncOperation>,
    ) -> Result<usize, String> {
        let state = self.state.data().map_err(|error| error.to_string())?;
        let mut retired = Vec::new();
        for operation in operations.iter() {
            if let Ok(path) = PreferencesSyncedEntry::operationPath(operation) {
                if std::path::Path::new(path) == self.state.path() {
                    return Err(
                        "Preference layout state is node-local and cannot be synchronized".into(),
                    );
                }
                if isRetired(&state, path) {
                    PreferencesSyncedEntry::fromOperation(operation)
                        .map_err(|error| error.to_string())?;
                    retired.push(operation.clone());
                }
            }
        }
        if !retired.is_empty() {
            SyncOperationStore::new(self.storage.clone(), RUNTIME_SYNC_DIR_PATH)
                .skipOperations(&retired)
                .map_err(|error| error.to_string())?;
        }
        let identities = retired
            .iter()
            .map(|operation| operation.opId.as_str())
            .collect::<BTreeSet<_>>();
        operations.retain(|operation| !identities.contains(operation.opId.as_str()));
        history.retain(|operation| {
            !PreferencesSyncedEntry::operationPath(operation)
                .is_ok_and(|path| isRetired(&state, path))
        });
        Ok(retired.len())
    }
}

impl PreferenceLayoutMigration<'_> {
    /// Creates or upgrades a destination using its owner's existing DataStore schema.
    pub fn create(&self, store: &PreferencesDataStore) -> Result<(), PreferencesDataStoreError> {
        store.data().map(|_| ())
    }

    /// Ends a source file's synchronization authority after its destination has been committed.
    pub fn retireFile(&mut self, path: &str) {
        self.state.set(
            &stringPreferencesKey(&format!("{RETIRED_FILE}{path}")),
            true,
        );
    }

    /// Ends a retired namespace's authority, including paths unknown to the local installation.
    pub fn retireDirectory(&mut self, directory: &str) {
        self.state.set(
            &stringPreferencesKey(&format!(
                "{RETIRED_DIRECTORY}{}/",
                directory.trim_end_matches('/')
            )),
            true,
        );
    }
}

/// Matches explicit retirement declarations with exact file identities and directory boundaries.
fn isRetired(state: &Preferences, path: &str) -> bool {
    state.iterEntries().any(|(key, value)| {
        value.as_bool() == Some(true)
            && (key
                .strip_prefix(RETIRED_FILE)
                .is_some_and(|file| file == path)
                || key
                    .strip_prefix(RETIRED_DIRECTORY)
                    .is_some_and(|directory| path.starts_with(directory)))
    })
}
