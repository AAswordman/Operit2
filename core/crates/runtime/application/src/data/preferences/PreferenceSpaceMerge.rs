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
            .filter_map(|path| {
                operit_store::CoreSpaceStore::CoreSpaceStore::preferenceSyncStore(path)
            }),
    );
    schema
}
/// Prepares existing datasets through the same migration, import and replay path as incoming data.
pub(crate) fn prepare(storage: Arc<dyn RuntimeStorageHost>) -> Vec<String> {
    use operit_store::PreferenceMerge::applyIndependentOperations;
    use operit_store::PreferencesDataStore::PreferencesDataStore;
    let mut schema = schema(&[]);
    let mut errors = Vec::new();
    schema.stores.retain(|owner| {
        let required = (|| -> Result<bool, String> {
            if !storage.exists(&owner.path).map_err(|error| error.to_string())? {
                return Ok(false);
            }
            let store = if owner.encrypted {
                PreferencesDataStore::newEncryptedWithStorage(storage.clone(), owner.path.clone())
            } else {
                PreferencesDataStore::newWithStorage(storage.clone(), owner.path.clone())
            };
            store.isSyncPrepared(owner.storageVersion.expect("native owner schema version"))
                .map(|ready| !ready).map_err(|error| error.to_string())
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

    /// Replays released provider and voice encodings without creating defaults or losing original identities.
    #[test]
    fn historical_catalog_contracts_decode_before_identity_validation() {
        let fixture = Fixture::new();
        let provider = operit_model::ModelConfigData::default_deepseek_provider();
        let providerKey = format!("provider_{}", provider.id);
        let tts = json!({"id":"remote-tts","name":"Remote TTS","providerType":"HTTP_TTS","endpoint":"https://example.test/tts",
            "apiKey":"","model":"voice","voice":"a","responseFormat":"mp3","speed":1.0,"createdAt":1,"updatedAt":2});
        let stt = json!({"id":"remote-stt","name":"Remote STT","providerType":"HTTP_STT","endpoint":"https://example.test/stt",
            "apiKey":"","model":"speech","fileFieldName":"file","modelFieldName":"model","languageFieldName":"language",
            "responseTextJsonPath":"text","headers":[],"createdAt":1,"updatedAt":2});
        let operations = vec![
            historical(
                RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH,
                &providerKey,
                json!(serde_json::to_string(&provider).unwrap()),
                3,
                true,
                1,
            ),
            historical(
                RuntimeStorageLayout::TTS_CONFIGS_PREFERENCES_PATH,
                "tts_config_remote-tts_json",
                json!(tts.to_string()),
                1,
                true,
                2,
            ),
            historical(
                RuntimeStorageLayout::STT_CONFIGS_PREFERENCES_PATH,
                "stt_config_remote-stt_json",
                json!(stt.to_string()),
                1,
                true,
                3,
            ),
        ];
        let outcomes = applyIndependentOperations(
            fixture.host.clone(),
            &schema(&operations),
            &[],
            &operations,
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
        let models = ModelConfigManager::preferenceStore(
            fixture.host.clone(),
            RuntimeStorageLayout::MODEL_CONFIGS_PREFERENCES_PATH,
        )
        .data()
        .unwrap();
        assert_eq!(
            models.get(&stringPreferencesKey(&providerKey)),
            Some(&serde_json::to_value(provider).unwrap())
        );
        assert_eq!(
            models
                .iterEntries()
                .filter(|(key, _)| key.starts_with("provider_"))
                .count(),
            1
        );
        assert_eq!(
            TtsConfigManager::preferenceStore(
                fixture.host.clone(),
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
                fixture.host.clone(),
                RuntimeStorageLayout::STT_CONFIGS_PREFERENCES_PATH
            )
            .data()
            .unwrap()
            .get(&stringPreferencesKey("stt_config_remote-stt_json")),
            Some(&stt)
        );
        let archived = operit_store::SyncOperationStore::SyncOperationStore::new(
            fixture.host.clone(),
            RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH,
        )
        .preferenceHistory()
        .unwrap();
        assert_eq!(archived, operations);
    }

    /// Converts older scalar values and preserves an offline delta after newer native values arrive.
    #[test]
    fn mixed_storage_versions_preserve_api_counter_transactions() {
        let fixture = Fixture::new();
        let path = operit_util::OperitPaths::API_PREFERENCES_PATH;
        let baseline = historical(path, "token_input_test", json!("10"), 1, false, 1);
        let mut native = historical(path, "enable_tools", json!(true), 2, false, 2);
        native.payload["formatVersion"] = json!(2);
        let mut delta = historical(path, "token_input_test", json!(2), 1, false, 3);
        delta.originDeviceId = "offline".into();
        delta.sequence = 1;
        delta.opId = "offline:1".into();
        delta.operation = "increment".into();
        delta.semantics = SyncOperationSemantics::Transaction;
        delta.payload["formatVersion"] = json!(2);
        let operations = vec![baseline, native, delta];
        for _ in 0..2 {
            let outcomes = applyIndependentOperations(
                fixture.host.clone(),
                &schema(&operations),
                &[],
                &operations,
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
            let data = super::super::ApiPreferences::ApiPreferences::store(fixture.host.clone())
                .data()
                .unwrap();
            assert_eq!(
                data.get(&stringPreferencesKey("token_input_test")),
                Some(&json!(12))
            );
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
        for _ in 0..2 {
            let outcomes = applyIndependentOperations(fixture.host.clone(), &schema(&operations),
                &[], &operations, |_, _| true);
            assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()), "{:?}",
                outcomes.iter().map(|outcome| &outcome.result).collect::<Vec<_>>());
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

}
