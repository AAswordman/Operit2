//! Versions preference creation, ownership transfer and retirement through DataStore migrations.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;

use crate::PreferencesDataStore::{
    stringPreferencesKey, CoreNodeStateStore, Preferences, PreferencesDataStore,
    PreferencesDataStoreError, PreferencesSyncedEntry,
};
use crate::SyncOperationStore::SyncOperation;

const ROUTE_FILE: &str = "route_file:";
const ROUTES_PATH: &str = "runtime/sync/preference_routes.preferences.json";

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
        let state = self.state.data().map_err(|error| error.to_string())?;
        let routes = CoreNodeStateStore::newWithStorage(self.storage.clone(), ROUTES_PATH);
        routes.edit(|current| {
            for (key, value) in state.iterEntries().filter(|(key, _)| [ROUTE_FILE, RETIRED_FILE, RETIRED_DIRECTORY].iter().any(|prefix| key.starts_with(prefix))) {
                current.set(&stringPreferencesKey(key), value.clone());
            }
        }).map_err(|error| error.to_string())
    }

    /// Layout preparation preserves immutable originals. Missing owners remain pending.
    pub fn prepareOperations(
        &self,
        _history: &mut Vec<SyncOperation>,
        operations: &mut Vec<SyncOperation>,
    ) -> Result<usize, String> {
        self.prepare()?;
        for operation in operations {
            if PreferencesSyncedEntry::operationPath(operation)
                .is_ok_and(|path| std::path::Path::new(path) == self.state.path()) {
                return Err("Preference layout state is node-local and cannot be synchronized".into());
            }
        }
        Ok(0)
    }

}

impl PreferenceLayoutMigration<'_> {
    /// Moves the complete state and records the same routing rule for late original transactions.
    pub fn moveFile(&mut self, from: &str, to: &PreferencesDataStore) -> Result<(), PreferencesDataStoreError> {
        let destination = to.path().to_string_lossy().into_owned();
        if from == destination { return Ok(()); }
        let source = PreferencesDataStore::newWithStorage(self.storage.clone(), from);
        if self.storage.exists(from)? {
            crate::Transaction::preference::move_to(&source,to)?;
        }
        self.state.set(&stringPreferencesKey(&format!("{ROUTE_FILE}{from}")), destination);
        Ok(())
    }

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

/// Resolves layout ownership for both stored snapshots and immutable transaction views.
pub fn routes(storage: Arc<dyn RuntimeStorageHost>) -> Result<BTreeMap<String, String>, String> {
    let values = CoreNodeStateStore::newWithStorage(storage, ROUTES_PATH).data().map_err(|error| error.to_string())?;
    let routes = values.iterEntries().filter_map(|(key, value)| {
        Some((key.strip_prefix(ROUTE_FILE)?.to_owned(), value.as_str()?.to_owned()))
    }).collect::<BTreeMap<_, _>>();
    let mut resolved = BTreeMap::new();
    for source in routes.keys() {
        resolved.insert(source.clone(), crate::Transaction::Migrations::resolve_path(&routes, source)?);
    }
    Ok(resolved)
}

/// Retirement is an explicit layout decision; unknown paths have no such disposition.
pub fn retirements(storage: Arc<dyn RuntimeStorageHost>) -> Result<BTreeSet<String>, String> {
    let values = CoreNodeStateStore::newWithStorage(storage, ROUTES_PATH).data().map_err(|e| e.to_string())?;
    Ok(values.iterEntries().filter_map(|(key, value)| {
        if value != &serde_json::Value::Bool(true) { return None; }
        key.strip_prefix(RETIRED_FILE).or_else(|| key.strip_prefix(RETIRED_DIRECTORY)).map(str::to_owned)
    }).collect())
}
