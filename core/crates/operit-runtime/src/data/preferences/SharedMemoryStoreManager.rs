use operit_store::PreferencesDataStore::{
    stringPreferencesKey, Flow, Preferences, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use uuid::Uuid;

use operit_model::Memory::SharedMemoryStore;

#[derive(Clone)]
pub struct SharedMemoryStoreManager {
    dataStore: PreferencesDataStore,
}

impl SharedMemoryStoreManager {
    /// Creates a shared memory store manager backed by the supplied runtime store paths.
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(paths.shared_memory_stores_preferences_path()),
        }
    }

    #[allow(non_snake_case)]
    /// Creates a shared memory store manager using the default runtime store paths.
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    #[allow(non_snake_case)]
    fn SHARED_MEMORY_STORE_LIST() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("shared_memory_store_list")
    }

    #[allow(non_snake_case)]
    /// Observes the ordered list of shared memory store identifiers.
    pub fn sharedMemoryStoreListFlow(&self) -> Flow<Vec<String>> {
        self.dataStore
            .dataFlow()
            .map(|preferences| readStoreList(&preferences))
    }

    #[allow(non_snake_case)]
    /// Reads every shared memory store in persisted list order.
    pub fn getAllSharedMemoryStores(&self) -> Result<Vec<SharedMemoryStore>, String> {
        let ids = self
            .sharedMemoryStoreListFlow()
            .first()
            .map_err(|error| error.to_string())?;
        let mut stores = Vec::new();
        for id in ids {
            stores.push(
                self.getSharedMemoryStore(&id)
                    .map_err(|error| error.to_string())?,
            );
        }
        Ok(stores)
    }

    #[allow(non_snake_case)]
    /// Reads one shared memory store by identifier.
    pub fn getSharedMemoryStore(
        &self,
        id: &str,
    ) -> Result<SharedMemoryStore, PreferencesDataStoreError> {
        self.getSharedMemoryStoreFlow(id).first()
    }

    #[allow(non_snake_case)]
    /// Observes one shared memory store by identifier.
    pub fn getSharedMemoryStoreFlow(&self, id: &str) -> Flow<SharedMemoryStore> {
        let id = id.to_string();
        self.dataStore
            .dataFlow()
            .map(move |preferences| readSharedMemoryStore(&preferences, &id))
    }

    #[allow(non_snake_case)]
    /// Creates a shared memory store with a generated identifier.
    pub fn createSharedMemoryStore(&self, name: String) -> Result<SharedMemoryStore, String> {
        let trimmedName = name.trim();
        if trimmedName.is_empty() {
            return Err("shared memory store name is empty".to_string());
        }
        let id = Uuid::new_v4().to_string();
        self.createSharedMemoryStoreWithId(id, trimmedName.to_string())
    }

    #[allow(non_snake_case)]
    /// Creates a shared memory store with an explicit identifier.
    pub fn createSharedMemoryStoreWithId(
        &self,
        id: String,
        name: String,
    ) -> Result<SharedMemoryStore, String> {
        let id = id.trim();
        if id.is_empty() {
            return Err("shared memory store id is empty".to_string());
        }
        let trimmedName = name.trim();
        if trimmedName.is_empty() {
            return Err("shared memory store name is empty".to_string());
        }
        let now = currentTimeMillis();
        let store = SharedMemoryStore {
            id: id.to_string(),
            name: trimmedName.to_string(),
            createdAt: now,
            updatedAt: now,
        };
        self.dataStore
            .try_edit_result(|preferences| {
                assertStoreNameUnique(preferences, trimmedName, Some(id))?;
                let mut list = readStoreList(preferences);
                list.push(id.to_string());
                list.sort();
                list.dedup();
                writeStoreList(preferences, &list);
                writeSharedMemoryStore(preferences, &store);
                Ok::<(), PreferencesDataStoreError>(())
            })
            .map_err(|error| error.to_string())?;
        Ok(store)
    }

    #[allow(non_snake_case)]
    /// Renames a shared memory store and refreshes its update timestamp.
    pub fn renameSharedMemoryStore(
        &self,
        id: &str,
        name: String,
    ) -> Result<SharedMemoryStore, String> {
        let trimmedName = name.trim();
        if trimmedName.is_empty() {
            return Err("shared memory store name is empty".to_string());
        }
        let id = id.trim();
        let now = currentTimeMillis();
        let mut exists = false;
        self.dataStore
            .try_edit_result(|preferences| {
                let list = readStoreList(preferences);
                exists = list.iter().any(|entry| entry == id);
                if !exists {
                    return Ok::<(), PreferencesDataStoreError>(());
                }
                assertStoreNameUnique(preferences, trimmedName, Some(id))?;
                let mut store = readSharedMemoryStore(preferences, id);
                store.name = trimmedName.to_string();
                store.updatedAt = now;
                writeSharedMemoryStore(preferences, &store);
                Ok::<(), PreferencesDataStoreError>(())
            })
            .map_err(|error| error.to_string())?;
        if !exists {
            return Err(format!("shared memory store not found: {id}"));
        }
        self.getSharedMemoryStore(id)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Deletes a shared memory store and returns whether an entry was removed.
    pub fn deleteSharedMemoryStore(&self, id: &str) -> Result<bool, String> {
        let id = id.trim().to_string();
        if id.is_empty() {
            return Err("shared memory store id is empty".to_string());
        }
        let mut deleted = false;
        self.dataStore
            .edit(|preferences| {
                let mut list = readStoreList(preferences);
                let originalLen = list.len();
                list.retain(|entry| entry != &id);
                deleted = list.len() != originalLen;
                writeStoreList(preferences, &list);
                preferences.remove(&stringPreferencesKey(&format!(
                    "shared_memory_store_{id}_name"
                )));
                preferences.remove(&stringPreferencesKey(&format!(
                    "shared_memory_store_{id}_created_at"
                )));
                preferences.remove(&stringPreferencesKey(&format!(
                    "shared_memory_store_{id}_updated_at"
                )));
            })
            .map_err(|error| error.to_string())?;
        Ok(deleted)
    }
}

fn assertStoreNameUnique(
    preferences: &Preferences,
    name: &str,
    currentStoreId: Option<&str>,
) -> Result<(), PreferencesDataStoreError> {
    let normalizedName = name.trim();
    let storeIds = readStoreList(preferences);
    for storeId in storeIds {
        if currentStoreId == Some(storeId.as_str()) {
            continue;
        }
        let existing = readSharedMemoryStore(preferences, &storeId);
        if existing.name.trim() == normalizedName {
            return Err(PreferencesDataStoreError::Message(format!(
                "shared memory store name already exists: {normalizedName}"
            )));
        }
    }
    Ok(())
}
fn readStoreList(preferences: &Preferences) -> Vec<String> {
    preferences
        .get(&SharedMemoryStoreManager::SHARED_MEMORY_STORE_LIST())
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default()
}

fn writeStoreList(preferences: &mut Preferences, list: &[String]) {
    preferences.set(
        &SharedMemoryStoreManager::SHARED_MEMORY_STORE_LIST(),
        serde_json::to_string(list).expect("shared memory store list must serialize"),
    );
}

fn readSharedMemoryStore(preferences: &Preferences, id: &str) -> SharedMemoryStore {
    SharedMemoryStore {
        id: id.to_string(),
        name: preferences
            .get(&stringPreferencesKey(&format!(
                "shared_memory_store_{id}_name"
            )))
            .cloned()
            .unwrap_or_else(|| id.to_string()),
        createdAt: preferences
            .get(&stringPreferencesKey(&format!(
                "shared_memory_store_{id}_created_at"
            )))
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_else(currentTimeMillis),
        updatedAt: preferences
            .get(&stringPreferencesKey(&format!(
                "shared_memory_store_{id}_updated_at"
            )))
            .and_then(|value| value.parse::<i64>().ok())
            .unwrap_or_else(currentTimeMillis),
    }
}

fn writeSharedMemoryStore(preferences: &mut Preferences, store: &SharedMemoryStore) {
    preferences.set(
        &stringPreferencesKey(&format!("shared_memory_store_{}_name", store.id)),
        store.name.clone(),
    );
    preferences.set(
        &stringPreferencesKey(&format!("shared_memory_store_{}_created_at", store.id)),
        store.createdAt.to_string(),
    );
    preferences.set(
        &stringPreferencesKey(&format!("shared_memory_store_{}_updated_at", store.id)),
        store.updatedAt.to_string(),
    );
}

#[allow(non_snake_case)]
fn currentTimeMillis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
