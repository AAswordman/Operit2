use operit_host_api::RuntimeStorageHost;
use operit_store::PreferencesDataStore::{
    Preferences, PreferencesDataStore, PreferencesDataStoreError, stringPreferencesKey,
};
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use operit_util::OperitPaths;
use std::sync::Arc;

/// Preference key storing the thinking-persistence switch as a typed boolean.
const PERSIST_THINKING_PARTS_KEY: &str = "persist_thinking_parts";

/// Dedicated preferences file for this switch (post-2026-10 upstream migration
/// each preference module owns its own versioned file instead of a shared map).
const PREFERENCES_FILE: &str = "thinking_persistence_preferences";

/// The store is created at the current preference schema; this key is new, so it
/// has no legacy string values to convert at versions 0/1.
const SCHEMA_VERSION: u32 = 2;

/// Assistant thinking parts persist unless the user switches them off.
pub const DEFAULT_PERSIST_THINKING_PARTS: bool = true;

/// Owns the user switch controlling whether assistant thinking parts persist.
pub struct ThinkingPersistencePreferences {
    dataStore: PreferencesDataStore,
}

fn migrate(
    version: u32,
    _preferences: &mut Preferences,
) -> Result<(), PreferencesDataStoreError> {
    match version {
        // New key: nothing to convert from the old string-encoded layout.
        0 | 1 => Ok(()),
        from => Err(PreferencesDataStoreError::MissingMigration {
            from,
            to: from + 1,
        }),
    }
}

fn open(host: Arc<dyn RuntimeStorageHost>, path: &str) -> PreferencesDataStore {
    PreferencesDataStore::newWithStorage(host, path).withSchema(SCHEMA_VERSION, migrate)
}

impl ThinkingPersistencePreferences {
    /// Opens the thinking-persistence facade from its own versioned preference file.
    pub fn getInstance() -> Self {
        let path = OperitPaths::customPreferenceStoragePath(PREFERENCES_FILE)
            .expect("declared thinking-persistence preference path");
        Self {
            dataStore: open(defaultRuntimeStorageHost(), &path),
        }
    }

    /// Reads whether assistant thinking parts persist.
    ///
    /// A missing key resolves to [`DEFAULT_PERSIST_THINKING_PARTS`]; a storage
    /// read failure or a value that is not a boolean is propagated as `Err`
    /// (#187): "the preferences cannot be read" must never become "the user
    /// left it on". The chat write path then fails closed for that one write.
    pub fn persistThinkingParts(&self) -> Result<bool, PreferencesDataStoreError> {
        let stored = self
            .dataStore
            .data()?
            .getTyped::<bool>(&stringPreferencesKey(PERSIST_THINKING_PARTS_KEY))?;
        Ok(stored.unwrap_or(DEFAULT_PERSIST_THINKING_PARTS))
    }

    /// Saves whether assistant thinking parts persist.
    pub fn savePersistThinkingParts(
        &self,
        persist: bool,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore
            .edit(|preferences| preferences.set(&stringPreferencesKey(PERSIST_THINKING_PARTS_KEY), persist))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    /// In-memory storage host: no registration, no real files, and it can be
    /// told to fail reads so the storage-failure path is reproducible.
    struct MemoryStorageHost {
        files: Mutex<HashMap<String, Vec<u8>>>,
        failReads: bool,
    }

    impl MemoryStorageHost {
        /// Creates an empty host; `failReads` turns every read into an error.
        fn new(failReads: bool) -> Self {
            Self {
                files: Mutex::new(HashMap::new()),
                failReads,
            }
        }

        /// Creates a host that already holds one file for `path`.
        fn withFile(path: &str, content: &[u8], failReads: bool) -> Self {
            let host = Self::new(failReads);
            host.files
                .lock()
                .expect("test storage mutex must not be poisoned")
                .insert(path.to_string(), content.to_vec());
            host
        }
    }

    impl RuntimeStorageHost for MemoryStorageHost {
        fn runtimeRootDir(&self) -> Option<PathBuf> {
            None
        }

        fn workspaceRootDir(&self) -> Option<PathBuf> {
            None
        }

        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            if self.failReads {
                return Err(HostError::new("simulated preferences read failure"));
            }
            self.files
                .lock()
                .expect("test storage mutex must not be poisoned")
                .get(path)
                .cloned()
                .ok_or_else(|| HostError::new("storage entry not found"))
        }

        fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            self.files
                .lock()
                .expect("test storage mutex must not be poisoned")
                .insert(path.to_string(), content.to_vec());
            Ok(())
        }

        fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            self.files
                .lock()
                .expect("test storage mutex must not be poisoned")
                .entry(path.to_string())
                .or_default()
                .extend_from_slice(content);
            Ok(())
        }

        fn delete(&self, path: &str, _recursive: bool) -> HostResult<()> {
            self.files
                .lock()
                .expect("test storage mutex must not be poisoned")
                .remove(path);
            Ok(())
        }

        fn exists(&self, path: &str) -> HostResult<bool> {
            Ok(self
                .files
                .lock()
                .expect("test storage mutex must not be poisoned")
                .contains_key(path))
        }

        fn list(&self, _prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
            Ok(Vec::new())
        }
    }

    const TEST_PREFERENCES_PATH: &str = "runtime/preferences/thinking-persistence-test.json";

    /// Builds a facade over one throwaway in-memory preferences store.
    fn storeWith(host: Arc<dyn RuntimeStorageHost>) -> ThinkingPersistencePreferences {
        ThinkingPersistencePreferences {
            dataStore: open(host, TEST_PREFERENCES_PATH),
        }
    }

    /// #187: a preferences read failure must surface as `Err`, never as `Ok(true)`.
    ///
    /// Before the fix the same failure produced `Ok(DEFAULT = true)`, which the
    /// chat write path read as "the user allows persisting thinking parts".
    #[test]
    fn read_failure_is_not_user_consent() {
        let host = MemoryStorageHost::withFile(TEST_PREFERENCES_PATH, b"{}", true);
        let result = storeWith(Arc::new(host)).persistThinkingParts();
        assert!(
            result.is_err(),
            "storage read failure must propagate, got {result:?}"
        );
    }

    /// A missing preferences file keeps the designed default: parts persist.
    #[test]
    fn missing_preferences_default_to_persisting() {
        let host = MemoryStorageHost::new(false);
        let result = storeWith(Arc::new(host)).persistThinkingParts();
        assert_eq!(result.ok(), Some(true));
    }

    /// The stored off switch reads back as `false` (the read path still works).
    #[test]
    fn stored_off_switch_is_read_back() {
        let store = storeWith(Arc::new(MemoryStorageHost::new(false)));
        store
            .savePersistThinkingParts(false)
            .expect("preference write must succeed");
        assert_eq!(store.persistThinkingParts().ok(), Some(false));
    }

    /// The production path builds a fresh facade per call, so the switch must
    /// also come back from a cold read of an existing file - not only from the
    /// same instance that wrote it (#143 review T3).
    #[test]
    fn cold_instance_reads_stored_off_switch() {
        // Post-migration layout: the value is a typed JSON boolean.
        let host = MemoryStorageHost::withFile(
            TEST_PREFERENCES_PATH,
            br#"{"persist_thinking_parts":false}"#,
            false,
        );
        let result = storeWith(Arc::new(host)).persistThinkingParts();
        match result {
            Ok(persist) => assert!(
                !persist,
                "a cold read of a stored off switch must stay off, got true"
            ),
            Err(error) => panic!("cold read of an existing file must succeed, got {error:?}"),
        }
    }
}
