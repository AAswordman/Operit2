use crate::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
    PreferencesKey,
};
use crate::RuntimeStorePaths::RuntimeStorePaths;
use operit_model::Workspace::{Workspace, WorkspaceFolder};
use uuid::Uuid;

/// Preference-backed store for named workspaces and their mounted folders.
#[derive(Clone)]
pub struct WorkspacePreferenceStore {
    dataStore: PreferencesDataStore,
}

impl WorkspacePreferenceStore {
    const PREFERENCES_VERSION: u32 = 2;

    /// Opens the same versioned workspace owner for background synchronization.
    pub fn openStore(storage: std::sync::Arc<dyn operit_host_api::RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
        PreferencesDataStore::newWithStorage(storage, path)
            .withSchema(Self::PREFERENCES_VERSION, Self::migratePreferences)
    }

    /// Imports historical complete workspace records without running initialization migrations.
    pub fn decodeSyncedEntry(entry: &mut crate::PreferencesDataStore::PreferencesSyncedEntry) -> Result<(), PreferencesDataStoreError> {
        entry.decodeHistoricalValues(|version, values| {
            if version > Self::PREFERENCES_VERSION {
                return Err(PreferencesDataStoreError::SchemaVersionTooNew { actual: version, expected: Self::PREFERENCES_VERSION });
            }
            if version < 2 { Self::migratePreferences(1, values)?; }
            Ok(())
        })
    }

    /// Creates a workspace preference store from explicit runtime paths.
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(paths.workspaces_preferences_path())
                .withSchema(Self::PREFERENCES_VERSION, Self::migratePreferences),
        }
    }

    /// Creates a workspace preference store from the default runtime paths.
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    /// Returns the independently synchronized record key for one workspace.
    fn workspaceDataKey(workspaceId: &str) -> PreferencesKey {
        stringPreferencesKey(&format!("workspace_{workspaceId}_data"))
    }

    /// Observes workspace records directly so a separately merged index cannot hide them.
    pub fn allWorkspacesFlow(&self) -> Flow<Vec<Workspace>> {
        self.dataStore.dataFlow().mapResult(|preferences| {
            let mut workspaces = preferences
                .iterEntries()
                .filter_map(|(key, json)| {
                    let id = key.strip_prefix("workspace_")?.strip_suffix("_data")?;
                    Some(Self::decodeWorkspace(id, json))
                })
                .collect::<Result<Vec<_>, _>>()?;
            workspaces.sort_by(|left, right| {
                left.name
                    .to_lowercase()
                    .cmp(&right.name.to_lowercase())
                    .then_with(|| left.id.cmp(&right.id))
            });
            Ok(workspaces)
        })
    }

    /// Loads every stored workspace.
    pub fn getAll(&self) -> Result<Vec<Workspace>, PreferencesDataStoreError> {
        self.allWorkspacesFlow().first()
    }

    /// Loads and validates the exact record addressed by the workspace id.
    pub fn getById(
        &self,
        workspaceId: &str,
    ) -> Result<Option<Workspace>, PreferencesDataStoreError> {
        if workspaceId.trim().is_empty() {
            return Ok(None);
        }
        let preferences = self.dataStore.dataFlow().first()?;
        preferences
            .get(&Self::workspaceDataKey(workspaceId))
            .map(|json| Self::decodeWorkspace(workspaceId, json))
            .transpose()
    }

    /// Creates a named workspace with the supplied folders.
    pub fn create(
        &self,
        name: String,
        folders: Vec<WorkspaceFolder>,
    ) -> Result<Workspace, PreferencesDataStoreError> {
        let timestamp = currentTimeMillis();
        let workspace = Workspace {
            id: Uuid::new_v4().to_string(),
            name,
            folders,
            createdAt: timestamp,
            updatedAt: timestamp,
        };
        self.upsert(workspace)
    }

    /// Writes one workspace record without a shared list that concurrent edits can overwrite.
    pub fn upsert(&self, workspace: Workspace) -> Result<Workspace, PreferencesDataStoreError> {
        workspace
            .validate()
            .map_err(PreferencesDataStoreError::Message)?;
        self.dataStore.try_edit_result(|preferences| {
            preferences.set(
                &Self::workspaceDataKey(&workspace.id),
                serde_json::to_value(&workspace)
                    .map_err(|error| PreferencesDataStoreError::Message(error.to_string()))?,
            );
            Ok::<(), PreferencesDataStoreError>(())
        })?;
        Ok(workspace)
    }

    /// Deletes the independently synchronized record for one workspace.
    pub fn delete(&self, workspaceId: &str) -> Result<(), PreferencesDataStoreError> {
        if workspaceId.trim().is_empty() {
            return Ok(());
        }
        self.dataStore.edit(|preferences| {
            preferences.remove(&Self::workspaceDataKey(workspaceId));
        })
    }

    /// Rejects invalid records and mismatches between their keys and embedded identities.
    fn decodeWorkspace(
        workspaceId: &str,
        json: &serde_json::Value,
    ) -> Result<Workspace, PreferencesDataStoreError> {
        let workspace: Workspace = serde_json::from_value(json.clone())?;
        workspace
            .validate()
            .map_err(PreferencesDataStoreError::Message)?;
        if workspace.id != workspaceId {
            return Err(PreferencesDataStoreError::Message(format!(
                "Workspace record key {workspaceId} does not match its id {}",
                workspace.id
            )));
        }
        Ok(workspace)
    }

    /// Keeps existing record keys readable without creating a separately synchronized index.
    fn migratePreferences(
        version: u32,
        preferences: &mut Preferences,
    ) -> Result<(), PreferencesDataStoreError> {
        match version {
            0 => Ok(()),
            1 => {
                let records = preferences
                    .iterEntries()
                    .filter_map(|(key, value)| {
                        let id = key.strip_prefix("workspace_")?.strip_suffix("_data")?;
                        Some((key.to_owned(), id.to_owned(), value.clone()))
                    })
                    .collect::<Vec<_>>();
                for (key, id, value) in records {
                    let encoded = serde_json::from_value::<String>(value)?;
                    let value: serde_json::Value = serde_json::from_str(&encoded)?;
                    Self::decodeWorkspace(&id, &value)?;
                    preferences.set(&stringPreferencesKey(&key), value);
                }
                preferences.remove(&stringPreferencesKey("workspace_list"));
                Ok(())
            }
            from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
        }
    }
}

/// Returns the host clock timestamp for persisted workspace metadata.
fn currentTimeMillis() -> i64 {
    operit_host_api::TimeUtils::currentTimeMillis()
}

#[cfg(test)]
#[path = "WorkspacePreferenceStoreTests.rs"]
mod tests;
