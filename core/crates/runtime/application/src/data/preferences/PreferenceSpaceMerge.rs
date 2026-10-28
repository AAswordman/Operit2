//! Composes preference merge declarations and prepares source-owned migrations.
use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_store::PreferenceMerge::PreferenceMergeSchema;

use super::FunctionalConfigManager::FunctionalConfigManager;
use super::ModelConfigManager::ModelConfigManager;
use super::SttConfigManager::SttConfigManager;
use super::ThemeConfigManager::ThemeConfigManager;
use super::TtsConfigManager::TtsConfigManager;

/// Collects merge declarations without opening preference stores.
pub(crate) fn schema(
    history: &[operit_store::SyncOperationStore::SyncOperation],
) -> PreferenceMergeSchema {
    let mut schema = PreferenceMergeSchema {
        stores: vec![
            ModelConfigManager::spaceMergeStore(),
            TtsConfigManager::spaceMergeStore(),
            SttConfigManager::spaceMergeStore(),
            ThemeConfigManager::spaceMergeStore(),
            FunctionalConfigManager::spaceMergeStore(),
            super::ApiPreferences::ApiPreferences::spaceMergeStore(),
            super::OnboardingPreferences::OnboardingPreferences::spaceMergeStore(),
            super::RemoteAnnouncementPreferences::RemoteAnnouncementPreferences::spaceMergeStore(),
            super::SidebarDockPreferences::SidebarDockPreferences::spaceMergeStore(),
            operit_store::repository::WorkspacePreferenceStore::WorkspacePreferenceStore::preferenceSyncStore(),
        ],
    };
    let paths = history
        .iter()
        .filter(|operation| operation.domain == "preferences")
        .filter_map(|operation| {
            operation
                .payload
                .get("storagePath")
                .and_then(serde_json::Value::as_str)
        })
        .collect::<std::collections::BTreeSet<_>>();
    schema.stores.extend(
        paths
            .into_iter()
            .filter_map(operit_store::CoreSpaceStore::CoreSpaceStore::preferenceSyncStore),
    );
    schema
}
/// Versions preference ownership changes separately from each file's representation.
pub(crate) fn layout(
    storage: Arc<dyn RuntimeStorageHost>,
) -> operit_store::PreferenceLayout::PreferenceLayout {
    use operit_store::PreferencesDataStore::PreferencesDataStoreError;
    use operit_util::RuntimeStorageLayout::{
        PACKAGE_MANAGER_PREFERENCES_PATH, SKILL_VISIBILITY_PREFERENCES_PATH,
    };
    operit_store::PreferenceLayout::PreferenceLayout::new(
        storage,
        &format!(
            "{}/preference_layout.preferences.json",
            operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH
        ),
    )
    .withSchema(1, |from, migration| match from {
        0 => {
            let extensions =
                operit_store::ExtensionStore::ExtensionStore::new(migration.storage.clone());
            extensions
                .migrateLegacyLayout()
                .map_err(PreferencesDataStoreError::Message)?;
            for kind in ["package", "skill", "mcp"] {
                extensions
                    .records(kind)
                    .map_err(PreferencesDataStoreError::Message)?;
            }
            migration.retireFile(PACKAGE_MANAGER_PREFERENCES_PATH);
            migration.retireFile(SKILL_VISIBILITY_PREFERENCES_PATH);
            migration.retireDirectory("runtime/extensions/space/settings");
            Ok(())
        }
        from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
    })
}

/// Prepares existing datasets through the same migration, import and replay path as incoming data.
pub(crate) fn prepare(storage: Arc<dyn RuntimeStorageHost>) -> Vec<String> {
    use operit_store::PreferenceMerge::applyIndependentOperations;
    if let Err(error) = layout(storage.clone()).prepare() {
        return vec![format!("preference layout: {error}")];
    }
    let mut schema = schema(&[]);
    let mut errors = Vec::new();
    schema.stores.retain(|owner| {
        let required = (|| -> Result<bool, String> {
            if !storage.exists(&owner.path).map_err(|error| error.to_string())? {
                return Ok(false);
            }
            let store = (owner.open)(storage.clone(), &owner.path);
            store
                .isSyncPrepared()
                .map(|ready| !ready)
                .map_err(|error| error.to_string())
        })();
        match required {
            Ok(required) => required,
            Err(error) => {
                errors.push(format!("preference migration path={}: {error}", owner.path));
                false
            }
        }
    });
    for outcome in applyIndependentOperations(storage, &schema, &[], &[], |_, _| true) {
        if let Err(error) = outcome.result {
            errors.push(format!("preference migration paths={:?}: {error}", outcome.paths));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_native_storage::NativeRuntimeStorageHost;
    use operit_util::RuntimeStorageLayout;
    use operit_store::PreferenceMerge::{applyIndependentOperations, PreferenceMergeSchema};
    use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
    use operit_store::SyncOperationStore::{SyncOperation, SyncOperationSemantics};
    use serde_json::{json, Value};
    use std::path::PathBuf;

    struct Fixture { root: PathBuf, host: Arc<NativeRuntimeStorageHost> }

    impl Fixture {
        /// Opens a real storage host in an isolated directory for owner-contract replay checks.
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "operit-preference-contract-{}",
                uuid::Uuid::new_v4()
            ));
            std::fs::create_dir_all(&root).unwrap();
            let host = Arc::new(NativeRuntimeStorageHost::new(
                root.join("runtime"),
                root.join("workspaces"),
            ));
            Self { root, host }
        }
    }

    impl Drop for Fixture {
        /// Removes only the verified temporary fixture directory after host operations have completed.
        fn drop(&mut self) {
            assert_eq!(self.root.parent().unwrap(), std::env::temp_dir());
            std::fs::remove_dir_all(&self.root).unwrap();
        }
    }

    /// Builds a historical whole-root operation with an explicit source storage representation version.
    fn historical(
        path: &str,
        key: &str,
        value: Value,
        version: u32,
        encrypted: bool,
        sequence: i64,
    ) -> SyncOperation {
        SyncOperation {
            opId: format!("historical:{sequence}"),
            originDeviceId: "historical".into(),
            sequence,
            domain: "preferences".into(),
            entityType: "entry".into(),
            entityId: serde_json::to_string(&(path, key)).unwrap(),
            operation: "set".into(),
            semantics: SyncOperationSemantics::EntityState,
            payload: json!({"storagePath":path,"key":key,"value":value,"schemaVersion":version,"formatVersion":1,"encrypted":encrypted}),
            createdAt: sequence,
            schemaVersion: 1,
        }
    }


    /// Migrates released catalog files on the sender before replaying their current-version operations.
    #[test]
    fn owners_prepare_migrated_catalogs_before_peer_replay() {
        use operit_store::PreferencesDataStore::CoreNodeSecretStore;
        use operit_store::SyncOperationStore::SyncOperationStore;
        let source = Fixture::new();
        let target = Fixture::new();
        let provider = operit_model::ModelConfigData::default_deepseek_provider();
        let providerKey = format!("provider_{}", provider.id);
        let tts = json!({"id":"remote-tts","name":"Remote TTS","providerType":"HTTP_TTS","endpoint":"https://example.test/tts",
            "apiKey":"","model":"voice","voice":"a","responseFormat":"mp3","speed":1.0,"createdAt":1,"updatedAt":2});
        let stt = json!({"id":"remote-stt","name":"Remote STT","providerType":"HTTP_STT","endpoint":"https://example.test/stt",
            "apiKey":"","model":"speech","fileFieldName":"file","modelFieldName":"model","languageFieldName":"language",
            "responseTextJsonPath":"text","headers":[],"createdAt":1,"updatedAt":2});
        for (path, key, value, version) in [
            (
                RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH,
                providerKey.as_str(),
                serde_json::to_string(&provider).unwrap(),
                3,
            ),
            (
                RuntimeStorageLayout::TTS_CONFIGS_PREFERENCES_PATH,
                "tts_config_remote-tts_json",
                tts.to_string(),
                1,
            ),
            (
                RuntimeStorageLayout::STT_CONFIGS_PREFERENCES_PATH,
                "stt_config_remote-stt_json",
                stt.to_string(),
                1,
            ),
        ] {
            CoreNodeSecretStore::newWithStorage(source.host.clone(), path)
                .withSchema(version, |_, _| Ok(()))
                .edit(|preferences| preferences.set(&stringPreferencesKey(key), value))
                .unwrap();
        }
        assert!(prepare(source.host.clone()).is_empty());
        let models = ModelConfigManager::preferenceStore(
            source.host.clone(),
            RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH,
        );
        let mut migrated = models
            .data()
            .unwrap()
            .get(&stringPreferencesKey(&providerKey))
            .unwrap()
            .clone();
        migrated["name"] = json!("Migrated peer provider");
        models
            .edit(|preferences| {
                preferences.set(&stringPreferencesKey(&providerKey), migrated.clone())
            })
            .unwrap();
        let journal = SyncOperationStore::new(
            source.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        );
        let operations = journal.preferenceHistory().unwrap();
        let outcomes = applyIndependentOperations(
            target.host.clone(),
            &schema(&operations),
            &[],
            &operations,
            |_, _| true,
        );
        assert!(
            outcomes
                .iter()
                .all(|outcome| !outcome.skipped && outcome.result.is_ok()),
            "{:?}",
            outcomes
                .iter()
                .map(|outcome| &outcome.result)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            ModelConfigManager::preferenceStore(
                target.host.clone(),
                RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH
            )
            .data()
            .unwrap()
            .get(&stringPreferencesKey(&providerKey)),
            Some(&migrated)
        );
        assert_eq!(
            TtsConfigManager::preferenceStore(
                target.host.clone(),
                RuntimeStorageLayout::TTS_CONFIGS_PREFERENCES_PATH
            )
            .data()
            .unwrap()
            .get(&stringPreferencesKey("tts_config_remote-tts_json"))
            .unwrap()["id"],
            "remote-tts"
        );
        assert_eq!(
            SttConfigManager::preferenceStore(
                target.host.clone(),
                RuntimeStorageLayout::STT_CONFIGS_PREFERENCES_PATH
            )
            .data()
            .unwrap()
            .get(&stringPreferencesKey("stt_config_remote-stt_json")),
            Some(&stt)
        );
        let archived = SyncOperationStore::new(
            target.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        )
        .preferenceHistory()
        .unwrap();
        for operation in &operations {
            assert!(
                archived.contains(operation),
                "original operation must remain immutable"
            );
        }
        assert_eq!(journal.preferenceHistory().unwrap(), operations);
    }

    /// Imports historical counters locally and preserves current-version transactions on repeated delivery.
    #[test]
    fn upgraded_api_owner_publishes_current_counter_transactions() {
        use super::super::ApiPreferences::ApiPreferences;
        use operit_store::PreferencesDataStore::CoreNodeStateStore;
        use operit_store::SyncOperationStore::SyncOperationStore;
        let source = Fixture::new();
        let target = Fixture::new();
        let path = operit_util::OperitPaths::API_PREFERENCES_PATH;
        let counter = stringPreferencesKey("token_input_test");
        CoreNodeStateStore::newWithStorage(source.host.clone(), path)
            .withSchema(1, |_, _| Ok(()))
            .edit(|preferences| preferences.set(&counter, "10"))
            .unwrap();
        assert!(prepare(source.host.clone()).is_empty());
        ApiPreferences::store(source.host.clone())
            .try_edit_result(|preferences| {
                preferences.incrementInteger(&counter, 2)?;
                preferences.set(&stringPreferencesKey("enable_tools"), true);
                Ok::<(), operit_store::PreferencesDataStore::PreferencesDataStoreError>(())
            })
            .unwrap();
        let operations = SyncOperationStore::new(
            source.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        )
        .preferenceHistory()
        .unwrap();
        assert!(operations
            .iter()
            .all(|operation| operation.payload["schemaVersion"] == 2));
        for _ in 0..2 {
            let outcomes = applyIndependentOperations(
                target.host.clone(),
                &schema(&operations),
                &[],
                &operations,
                |_, _| true,
            );
            assert!(
                outcomes
                    .iter()
                    .all(|outcome| !outcome.skipped && outcome.result.is_ok()),
                "{:?}",
                outcomes
                    .iter()
                    .map(|outcome| &outcome.result)
                    .collect::<Vec<_>>()
            );
            let data = ApiPreferences::store(target.host.clone()).data().unwrap();
            assert_eq!(data.get(&counter), Some(&json!(12)));
            assert_eq!(
                data.get(&stringPreferencesKey("enable_tools")),
                Some(&json!(true))
            );
        }
    }

    /// Released Space journals omitted schemaVersion and stored record as encoded JSON.
    #[test]
    fn historical_space_records_stay_typed_after_replay_and_reopen() {
        use operit_store::CoreSpaceStore::CoreSpaceStore;
        let fixture = Fixture::new();
        let record = json!({"spaceId":"old-space","spaceName":"ios-aarch64","spaceRevision":12,
            "nodeId":"old-peer","joinedAt":1,"updatedAt":2});
        let mut operations = Vec::new();
        for (index, directory) in [RuntimeStorageLayout::RUNTIME_SPACE_MEMBERS_DIR_PATH,
            RuntimeStorageLayout::RUNTIME_SPACE_DEVICE_PROFILES_DIR_PATH,
            RuntimeStorageLayout::RUNTIME_SPACE_DEVICE_PRESENCE_DIR_PATH,
            RuntimeStorageLayout::RUNTIME_SPACE_TOPOLOGY_DIR_PATH].iter().enumerate() {
            let path = format!("{directory}/old-peer.preferences.json");
            fixture.host.writeBytesAtomically(&path, &serde_json::to_vec(&json!({
                "record":record.to_string(),"__operit_preferences_schema_version":2,
            })).unwrap()).unwrap();
            let mut operation = historical(&path, "record", json!(record.to_string()), 0, false, index as i64 + 1);
            operation.payload.as_object_mut().unwrap().remove("schemaVersion");
            operations.push(operation);
        }
        let journal = operit_store::SyncOperationStore::SyncOperationStore::new(
            fixture.host.clone(), RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH);
        journal.receiveOperations(&operations).unwrap();
        for _ in 0..2 {
            let outcomes = applyIndependentOperations(fixture.host.clone(), &schema(&operations),
                &[], &operations, |_, _| true);
            assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()), "{:?}",
                outcomes.iter().map(|outcome| &outcome.result).collect::<Vec<_>>());
            let skipped = outcomes.into_iter().filter(|outcome| outcome.skipped)
                .flat_map(|outcome| outcome.operations).collect::<Vec<_>>();
            assert_eq!(skipped.len(), operations.len());
            journal.skipOperations(&skipped).unwrap();
            for operation in &operations {
                let path = operation.payload["storagePath"].as_str().unwrap();
                let data = PreferencesDataStore::newWithStorage(fixture.host.clone(), path).data().unwrap();
                assert_eq!(data.get(&stringPreferencesKey("record")), Some(&record));
            }
            CoreSpaceStore::new(fixture.host.clone()).initialize().unwrap();
        }
        let journal = operit_store::SyncOperationStore::SyncOperationStore::new(
            fixture.host.clone(), RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH);
        for operation in operations {
            assert!(journal.preferenceHistory().unwrap().contains(&operation), "original operation must remain immutable");
        }
    }

    /// Generic string entries remain strings even when their contents happen to be valid JSON.
    #[test]
    fn undeclared_string_values_are_never_guessed_as_encoded_json() {
        let fixture = Fixture::new();
        let path = "runtime/config/preferences/literal.preferences.json";
        PreferencesDataStore::newWithStorage(fixture.host.clone(), path)
            .withSchema(1, |_, _| Ok(())).data().unwrap();
        let original = json!("{\"id\":\"literal\"}");
        let operation = historical(path, "literal", original.clone(), 1, false, 1);
        let outcomes = applyIndependentOperations(
            fixture.host.clone(),
            &PreferenceMergeSchema::default(),
            &[],
            &[operation],
            |_, _| true,
        );
        assert!(
            outcomes.iter().all(|outcome| outcome.result.is_ok()),
            "{:?}",
            outcomes
                .iter()
                .map(|outcome| &outcome.result)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            PreferencesDataStore::newWithStorage(fixture.host.clone(), path)
                .data()
                .unwrap()
                .get(&stringPreferencesKey("literal")),
            Some(&original)
        );
    }

    /// Converges two real storage hosts after disconnected edits, reversed pages and repeated delivery.
    #[test]
    fn two_hosts_converge_after_offline_counter_transactions() {
        use super::super::ApiPreferences::ApiPreferences;
        use operit_store::SyncOperationStore::SyncOperationStore;
        let first = Fixture::new();
        let second = Fixture::new();
        let firstStore = ApiPreferences::store(first.host.clone());
        let secondStore = ApiPreferences::store(second.host.clone());
        let firstJournal = SyncOperationStore::new(
            first.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        );
        let secondJournal = SyncOperationStore::new(
            second.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        );
        let key = stringPreferencesKey("token_input_test");
        firstStore
            .edit(|preferences| preferences.set(&key, 10))
            .unwrap();
        let baseline = firstJournal.preferenceHistory().unwrap();
        let outcomes = applyIndependentOperations(
            second.host.clone(),
            &schema(&baseline),
            &[],
            &baseline,
            |_, _| true,
        );
        assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
        firstStore
            .edit(|preferences| {
                preferences.incrementInteger(&key, 2).unwrap();
            })
            .unwrap();
        secondStore
            .edit(|preferences| {
                preferences.incrementInteger(&key, 3).unwrap();
            })
            .unwrap();
        let firstHistory = firstJournal.preferenceHistory().unwrap();
        let secondHistory = secondJournal.preferenceHistory().unwrap();
        for (host, incoming) in [
            (first.host.clone(), secondHistory),
            (second.host.clone(), firstHistory),
        ] {
            let journal =
                SyncOperationStore::new(host.clone(), RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH);
            for _ in 0..2 {
                for operation in incoming.iter().rev() {
                    journal.receiveOperations(&[operation.clone()]).unwrap();
                    let history = journal.preferenceHistory().unwrap();
                    let outcomes = applyIndependentOperations(
                        host.clone(),
                        &schema(&history),
                        &history,
                        &[operation.clone()],
                        |_, _| true,
                    );
                    assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
                    journal.completeOperations(&[operation.clone()]).unwrap();
                }
            }
        }
        assert_eq!(firstStore.data().unwrap().get(&key), Some(&json!(15)));
        assert_eq!(secondStore.data().unwrap().get(&key), Some(&json!(15)));
        assert_eq!(
            firstJournal.preferenceHistory().unwrap(),
            secondJournal.preferenceHistory().unwrap()
        );
        assert_eq!(firstJournal.preferenceHistory().unwrap().len(), 3);
        assert!(firstJournal.pendingOperations().unwrap().is_empty());
        assert!(secondJournal.pendingOperations().unwrap().is_empty());
    }

    /// Background sync imports unopened preference owners without reviving retired extension settings.
    #[test]
    fn preparation_imports_unopened_owners_once_through_the_shared_schema() {
        let fixture = Fixture::new();
        let sidebar = super::super::SidebarDockPreferences::SidebarDockPreferences::spaceMergeStore().path;
        let extension = format!("runtime/extensions/space/settings/package-{}.preferences.json", "a".repeat(64));
        fixture.host.writeBytesAtomically(&sidebar, br#"{"layout":"{\"wide\":true}"}"#).unwrap();
        fixture.host.writeBytesAtomically(&extension, br#"{"settings":"{\"enabled\":true}"}"#).unwrap();
        assert_eq!(prepare(fixture.host.clone()), Vec::<String>::new());
        let journal = operit_store::SyncOperationStore::SyncOperationStore::new(
            fixture.host.clone(), RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        );
        let history = journal.preferenceHistory().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(PreferencesDataStore::newWithStorage(fixture.host.clone(), &sidebar)
            .data().unwrap().get(&stringPreferencesKey("layout")), Some(&json!({"wide":true})));
        assert!(!history.iter().any(|operation| operation.payload["storagePath"] == extension));
        assert!(!fixture.host.exists(RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH).unwrap());
        assert!(prepare(fixture.host.clone()).is_empty());
        assert_eq!(journal.preferenceHistory().unwrap(), history);
    }


    /// Creates the three retired source paths that previously left fifteen operations pending.
    fn retired_extension_operations(source: &Fixture) -> Vec<SyncOperation> {
        use sha2::{Digest, Sha256};
        let digest = format!("{:x}", Sha256::digest(b"demo"));
        for path in [
            RuntimeStorageLayout::PACKAGE_MANAGER_PREFERENCES_PATH.to_owned(),
            format!("runtime/extensions/space/settings/package-{digest}.preferences.json"),
            format!("runtime/extensions/space/settings/skill-{digest}.preferences.json"),
        ] {
            let store = PreferencesDataStore::newWithStorage(source.host.clone(), path)
                .withSchema(2, |_, _| Ok(()));
            for order in 1..=5 {
                store
                    .edit(|preferences| {
                        preferences.set(&stringPreferencesKey("settings"), json!({"order":order}))
                    })
                    .unwrap();
            }
        }
        let journal = operit_store::SyncOperationStore::SyncOperationStore::new(
            source.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        );
        let operations = journal.preferenceHistory().unwrap();
        assert_eq!(operations.len(), 15);
        operations
    }

    /// Commits the existing extension migration before retiring sources and keeps new-owner edits authoritative.
    #[test]
    fn layout_transfers_extension_settings_before_completing_fifteen_retired_operations() {
        use operit_store::ExtensionStore::{ExtensionRecord, ExtensionStore};
        use operit_store::SyncOperationStore::SyncOperationStore;
        use sha2::{Digest, Sha256};
        let source = Fixture::new();
        let incoming = retired_extension_operations(&source);
        let target = Fixture::new();
        let digest = format!("{:x}", Sha256::digest(b"demo"));
        let recordPath = format!("runtime/extensions/space/records/package-{digest}.json");
        let settingsPath =
            format!("runtime/extensions/space/settings/package-{digest}.preferences.json");
        let record = ExtensionRecord {
            schemaVersion: 0,
            kind: "package".into(),
            id: "demo".into(),
            scope: "space".into(),
            sourceName: "demo.toolpkg".into(),
            files: std::collections::BTreeMap::new(),
            settings: json!({"members":["demo"],"enabledNames":[],"disabledNames":[],"subpackageStates":{},"order":0,"builtin":false,"installationId":"00000000000000000000000000000000"}),
        };
        target
            .host
            .writeBytesAtomically(&recordPath, &serde_json::to_vec(&record).unwrap())
            .unwrap();
        target
            .host
            .writeBytesAtomically(
                &settingsPath,
                &serde_json::to_vec(&json!({
                    "__operit_preferences_schema_version":1,
                    "settings":json!({"order":7,"builtin":true}).to_string()
                }))
                .unwrap(),
            )
            .unwrap();
        let journal = SyncOperationStore::new(
            target.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        );
        journal.receiveOperations(&incoming).unwrap();
        let mut pending = journal.pendingOperations().unwrap();
        let mut history = journal.preferenceMergeHistory().unwrap();
        assert_eq!(
            layout(target.host.clone())
                .prepareOperations(&mut history, &mut pending)
                .unwrap(),
            15
        );
        assert!(pending.is_empty());
        assert_eq!(journal.pendingOperationCount().unwrap(), 0);
        assert_eq!(journal.preferenceHistory().unwrap(), incoming);
        let extensions = ExtensionStore::new(target.host.clone());
        let mut migrated = extensions.record("package", "demo").unwrap();
        assert_eq!(migrated.schemaVersion, 1);
        assert_eq!(migrated.settings["order"], 7);
        assert_eq!(migrated.settings["builtin"], false);
        assert!(!target
            .host
            .exists(&format!(
                "runtime/extensions/space/settings/skill-{digest}.preferences.json"
            ))
            .unwrap());
        migrated.settings["order"] = json!(9);
        extensions
            .setSettings("package", "demo", migrated.settings)
            .unwrap();
        PreferencesDataStore::newWithStorage(source.host.clone(), settingsPath)
            .withSchema(2, |_, _| Ok(()))
            .edit(|preferences| {
                preferences.set(&stringPreferencesKey("settings"), json!({"order":99}))
            })
            .unwrap();
        let delayed = SyncOperationStore::new(
            source.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        )
        .preferenceHistory()
        .unwrap();
        journal.receiveOperations(&delayed).unwrap();
        let mut pending = journal.pendingOperations().unwrap();
        assert_eq!(
            layout(target.host.clone())
                .prepareOperations(&mut history, &mut pending)
                .unwrap(),
            1
        );
        assert_eq!(
            extensions.record("package", "demo").unwrap().settings["order"],
            9
        );
        assert_eq!(journal.pendingOperationCount().unwrap(), 0);
    }

    /// Failed business migration leaves the layout uncommitted and preserves all received originals.
    #[test]
    fn failed_extension_layout_preserves_pending_until_business_migration_succeeds() {
        use operit_store::SyncOperationStore::SyncOperationStore;
        let source = Fixture::new();
        let incoming = retired_extension_operations(&source);
        let target = Fixture::new();
        let broken = "runtime/extensions/space/records/package-broken.json";
        target
            .host
            .writeBytesAtomically(broken, b"invalid record")
            .unwrap();
        let journal = SyncOperationStore::new(
            target.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        );
        journal.receiveOperations(&incoming).unwrap();
        let mut pending = journal.pendingOperations().unwrap();
        let before = pending.clone();
        let mut history = Vec::new();
        assert!(layout(target.host.clone())
            .prepareOperations(&mut history, &mut pending)
            .is_err());
        assert_eq!(pending, before);
        assert_eq!(journal.pendingOperationCount().unwrap(), 15);
        assert!(!target
            .host
            .exists("runtime/sync/preference_layout.preferences.json")
            .unwrap());
        target.host.delete(broken, false).unwrap();
        assert_eq!(
            layout(target.host.clone())
                .prepareOperations(&mut history, &mut pending)
                .unwrap(),
            15
        );
        assert_eq!(journal.pendingOperationCount().unwrap(), 0);
        assert_eq!(journal.preferenceHistory().unwrap(), incoming);
    }
}
