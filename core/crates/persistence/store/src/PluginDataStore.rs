//! Bridges JavaScript preference callbacks to native JSON transactions and the Space journal.
use std::sync::{Arc, Mutex};

use operit_host_api::PluginStorage::{
    StoragePreferencesEdit, StoragePreferencesSnapshot, StorageRequest, StorageResponse,
    StorageValueEntry, StoragePreferencesMergeDescriptor, StoragePreferenceEdit,
};
use operit_host_api::RuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{runtimeStorageOwnership, RuntimeStorageOwnership};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::PreferencesDataStore::{
    stringPreferencesKey, CoreNodeStateStore, Preferences, PreferencesDataStore,
    PreferencesDataStoreError,
};
use crate::SqliteStore::SqliteStoreError;
use super::failure;

impl From<PreferencesDataStoreError> for SqliteStoreError {
    /// Preserves the original preferences failure at the SDK boundary.
    fn from(error: PreferencesDataStoreError) -> Self {
        failure(error.to_string())
    }
}

/// Retains the persistence policy selected by the common ownership abstraction.
enum Store {
    Space(PreferencesDataStore),
    Device(CoreNodeStateStore),
}

/// Retains one immutable baseline without writing concurrency metadata into user data.
struct PendingEdit {
    token: String,
    baseline: Preferences,
}

/// Owns a native preference handle and one explicitly bounded callback invocation.
pub(super) struct DataStore {
    store: Store,
    pending: Mutex<Option<PendingEdit>>,
}

impl DataStore {
    /// Opens native JSON storage using the same ownership policy as Rust preference managers.
    pub(super) fn open(host: Arc<dyn RuntimeStorageHost>, path: &str, merge: Option<&StoragePreferencesMergeDescriptor>) -> Result<Self, SqliteStoreError> {
        if !path.ends_with(".preferences.json") {
            return Err(failure("DataStore paths must end with .preferences.json"));
        }
        let store = match runtimeStorageOwnership(path).map_err(failure)? {
            RuntimeStorageOwnership::Space => Store::Space(PreferencesDataStore::newWithStorage(host, path)),
            RuntimeStorageOwnership::CoreNode => Store::Device(CoreNodeStateStore::newWithStorage(host, path)),
            RuntimeStorageOwnership::Ephemeral => return Err(failure("DataStore requires durable storage ownership")),
        };
        let result = Self { store, pending: Mutex::new(None) };
        snapshot(&result.preferences()?.data()?)?;
        if let Some(merge) = merge {
            crate::PreferenceMerge::validatePluginDeclaration(path, merge).map_err(failure)?;
            result.preferences()?.try_edit_result::<_, _, SqliteStoreError>(|preferences| {
                let key = stringPreferencesKey(crate::PreferenceMerge::PLUGIN_MERGE_DECLARATION_KEY);
                let combined = match preferences.get(&key) {
                    Some(existing) => crate::PreferenceMerge::combineDeclarations(&serde_json::from_value(existing.clone()).map_err(|error| failure(error.to_string()))?, merge).map_err(failure)?,
                    None => merge.clone(),
                };
                preferences.set(&key, serde_json::to_value(combined).map_err(|error| failure(error.to_string()))?);
                Ok(())
            })?;
        }
        Ok(result)
    }

    /// Borrows the existing transaction implementation for either ownership scope.
    fn raw(&self) -> &PreferencesDataStore {
        match &self.store { Store::Space(store) => store, Store::Device(store) => store }
    }

    fn preferences(&self) -> Result<PreferencesDataStore, SqliteStoreError> {
        let store = self.raw().clone();
        Ok(match crate::PluginPreferenceSchema::read(&store)? {
            Some(schema) => crate::PluginPreferenceSchema::configure(store,schema)?, None => store,
        })
    }

    /// Reads the complete JSON snapshot without exposing any object-database API.
    pub(super) fn request(&self, request: StorageRequest) -> Result<StorageResponse, SqliteStoreError> {
        match request {
            StorageRequest::PreferencesSchema { schema, .. } => {
                if self.pending.lock().map_err(|_|failure("Preferences edit state is poisoned"))?.is_some() {
                    return Err(failure("Cannot declare a schema during an edit"));
                }
                crate::PluginPreferenceSchema::declare(self.raw(),schema)?;
                Ok(StorageResponse::PreferencesData(snapshot(&self.preferences()?.data()?)?))
            }
            StorageRequest::PreferencesData { .. } => {
                Ok(StorageResponse::PreferencesData(snapshot(&self.preferences()?.data()?)?))
            }
            StorageRequest::PreferencesBeginEdit { .. } => {
                let mut pending = self.pending.lock().map_err(|_| failure("Preferences edit state is poisoned"))?;
                if pending.is_some() { return Err(failure("DataStore already has an active edit callback")); }
                let baseline = self.preferences()?.data()?;
                let token = uuid::Uuid::new_v4().to_string();
                let result = StoragePreferencesEdit { token: token.clone(), snapshot: snapshot(&baseline)? };
                *pending = Some(PendingEdit { token, baseline });
                Ok(StorageResponse::PreferencesBeginEdit(result))
            }
            StorageRequest::PreferencesFinishEdit { token, edits, schema_version, .. } => {
                self.finish(token, edits, schema_version).map(StorageResponse::PreferencesFinishEdit)
            }
            StorageRequest::PreferencesCancelEdit { token, .. } => {
                let mut pending = self.pending.lock().map_err(|_| failure("Preferences edit state is poisoned"))?;
                requireToken(&pending, &token)?;
                *pending = None;
                Ok(StorageResponse::PreferencesCancelEdit)
            }
            _ => Err(failure("DataStore requires preference operations")),
        }
    }

    /// Atomically validates the baseline and publishes the entire edit through the native journal.
    fn finish(&self, token: String, edits: Vec<StoragePreferenceEdit>, version: u32) -> Result<StoragePreferencesSnapshot, SqliteStoreError> {
        let baseline = {
            let mut pending = self.pending.lock().map_err(|_| failure("Preferences edit state is poisoned"))?;
            requireToken(&pending, &token)?;
            pending.take().expect("validated preference edit").baseline
        };
        for edit in &edits {
            let key = match edit {
                StoragePreferenceEdit::Set { key, .. } | StoragePreferenceEdit::Remove { key }
                | StoragePreferenceEdit::IncrementInteger { key, .. } | StoragePreferenceEdit::SetObjectField { key, .. } => key,
            };
            if crate::PluginPreferenceSchema::reserved(key) {
                return Err(failure("Engine-owned preference metadata cannot be edited by a callback"));
            }
            if key.is_empty() { return Err(failure("Preference keys must be nonempty")); }
        }
        if version != PreferencesDataStore::readSchemaVersion(&baseline)? {
            return Err(failure("Schema upgrades require a portable migration declaration"));
        }
        self.preferences()?.try_edit_result(|preferences| {
            if !preferences.sharesSnapshot(&baseline) {
                return Err(failure("DataStore edit conflict: preferences changed while the callback was running"));
            }
            for edit in edits {
                match edit {
                    StoragePreferenceEdit::Set { key, value } => preferences.set(&stringPreferencesKey(&key), value),
                    StoragePreferenceEdit::Remove { key } => preferences.remove(&stringPreferencesKey(&key)),
                    StoragePreferenceEdit::IncrementInteger { key, delta } => preferences.incrementInteger(&stringPreferencesKey(&key), delta)?,
                    StoragePreferenceEdit::SetObjectField { key, field, value } => preferences.setObjectField(&stringPreferencesKey(&key), &field, value)?,
                }
            }
            snapshot(preferences)
        })
    }

    /// Supplies complete snapshot observation through the existing cancellable host subscription.
    pub(super) fn watchSnapshot(&self) -> Result<StorageValueEntry, SqliteStoreError> {
        let value = serde_json::to_value(snapshot(&self.preferences()?.data()?)?)
            .map_err(|error| failure(error.to_string()))?;
        let content = serde_json::to_vec(&value).map_err(|error| failure(error.to_string()))?;
        Ok(StorageValueEntry { value, version: format!("{:x}", Sha256::digest(content)) })
    }
}

/// Requires the exact token for the outstanding callback without consuming another invocation.
fn requireToken(pending: &Option<PendingEdit>, token: &str) -> Result<(), SqliteStoreError> {
    match pending {
        Some(edit) if edit.token == token => Ok(()),
        _ => Err(failure("Unknown or completed DataStore edit")),
    }
}

/// Converts native typed JSON entries while keeping the engine-owned schema key private.
fn snapshot(preferences: &Preferences) -> Result<StoragePreferencesSnapshot, SqliteStoreError> {
    Ok(StoragePreferencesSnapshot {
        values: preferences.iterEntries().filter(|(key, _)| !crate::PluginPreferenceSchema::reserved(key))
            .map(|(key, value)| (key.to_string(), value.clone())).collect(),
        schema_version: PreferencesDataStore::readSchemaVersion(preferences)?,
    })
}
