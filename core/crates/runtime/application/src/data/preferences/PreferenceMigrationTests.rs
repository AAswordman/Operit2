//! Real owner migrations must commute through the production file/SQL receive path.
use super::*;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_store::{
    PreferenceMerge::applyOperations,
    PreferencesDataStore::{stringPreferencesKey, Preferences, PreferencesDataStore},
    SyncOperationStore::{SyncOperation, SyncOperationSemantics},
};
use operit_util::RuntimeStorageLayout as Layout;
use serde_json::{json, Value};

struct Fixture {
    root: std::path::PathBuf,
    host: Arc<dyn RuntimeStorageHost>,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("operit-owner-migration-{}", uuid::Uuid::new_v4()));
        let host = Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        ));
        Self { root, host }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn legacy<const VERSION: u32>(
    host: Arc<dyn RuntimeStorageHost>,
    path: &str,
) -> PreferencesDataStore {
    let store = match path {
        Layout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH => {
            PreferencesDataStore::newWithStorage(host, path)
        }
        _ => PreferencesDataStore::newEncryptedWithStorage(host, path),
    };
    store.withSchema(VERSION, |_, _| Ok(()))
}
fn operation(
    path: &str,
    version: u32,
    n: i64,
    key: &str,
    action: &str,
    value: Value,
) -> SyncOperation {
    SyncOperation {
        opId: format!("remote:{n}"),
        originDeviceId: "remote".into(),
        sequence: n,
        domain: "preferences".into(),
        entityType: "entry".into(),
        entityId: serde_json::to_string(&(path, key)).unwrap(),
        operation: action.into(),
        semantics: match action {
            "patch_object" => SyncOperationSemantics::Transaction,
            _ => SyncOperationSemantics::EntityState,
        },
        payload: json!({"storagePath":path,"key":key,"value":value,"schemaVersion":version,
            "formatVersion":2,"encrypted":path != Layout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH}),
        createdAt: 2_000_000_000_000 + n,
        schemaVersion: 1,
    }
}
fn commutes<const VERSION: u32>(
    path: &str,
    seed: impl Fn(&mut Preferences),
    operations: &[SyncOperation],
) {
    let actual = Fixture::new();
    let expected = Fixture::new();
    let old = legacy::<VERSION>(actual.host.clone(), path);
    old.edit(seed).unwrap();
    let baseline = old.syncSnapshot().unwrap();
    assert!(old
        .repairSyncedSnapshot(&baseline.token().unwrap(), &baseline)
        .unwrap());
    let reference = legacy::<VERSION>(expected.host.clone(), path);
    let empty = reference.syncSnapshot().unwrap();
    assert!(reference
        .repairSyncedSnapshot(&empty.token().unwrap(), &baseline)
        .unwrap());
    let old_schema = PreferenceMergeSchema {
        stores: vec![PreferenceMergeStore::values(
            path,
            path != Layout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH,
            legacy::<VERSION>,
        )],
    };
    applyOperations(
        expected.host.clone(),
        &old_schema,
        &[],
        operations,
        |_, _| true,
    )
    .unwrap();
    let expected_data = open(expected.host.clone(), path)
        .unwrap()
        .syncSnapshot()
        .unwrap()
        .data;
    let normalized = open(actual.host.clone(), path)
        .unwrap()
        .syncSnapshot()
        .unwrap();
    let receiver = Fixture::new();
    let store = open(receiver.host.clone(), path).unwrap();
    let empty = store.syncSnapshot().unwrap();
    assert!(store
        .repairSyncedSnapshot(&empty.token().unwrap(), &normalized)
        .unwrap());
    // The receiver has only the canonical snapshot, without the authority's old source file.
    for host in [actual.host.clone(), receiver.host.clone()] {
        for _ in 0..2 {
            applyOperations(host.clone(), &schema(), &[], operations, |_, _| true).unwrap();
            let actual_data = open(host.clone(), path)
                .unwrap()
                .syncSnapshot()
                .unwrap()
                .data;
            assert_eq!(
                actual_data, expected_data,
                "{path}: m(Apply(A,T)) = Apply(m(A),m_A(T))"
            );
        }
    }
}

#[test]
fn tts_schema_zero_defaults_and_existing_selection_commute() {
    let path = Layout::TTS_CONFIGS_PREFERENCES_PATH;
    let select = operation(path, 0, 1, "current_tts_config_id", "set", json!("custom2"));
    commutes::<0>(path, |_| {}, &[select.clone()]);
    commutes::<0>(
        path,
        |values| {
            values.set(&stringPreferencesKey("current_tts_config_id"), "custom");
            values.set(
                &stringPreferencesKey("tts_config_list"),
                json!(["custom", "custom2"]).to_string(),
            );
            for id in ["custom", "custom2"] {
                values.set(&stringPreferencesKey(&format!("tts_config_{id}_json")), json!({
                "id":id,"name":id,"providerType":"HTTP_TTS","endpoint":"https://example.test/tts",
                "apiKey":"","model":"voice","voice":id,"responseFormat":"mp3","speed":1.0,
                "createdAt":1,"updatedAt":2,
            }).to_string());
            }
        },
        &[select],
    );
    commutes::<0>(
        path,
        |_| {},
        &[operation(
            path,
            0,
            1,
            "current_tts_config_id",
            "delete",
            Value::Null,
        )],
    );
}

#[test]
fn functional_old_strings_native_objects_and_default_deletion_commute() {
    let path = Layout::FUNCTIONAL_CONFIGS_PREFERENCES_PATH;
    let binding = json!({"CHAT":{"providerId":"p","modelId":"m","followsChat":false},
        "TITLE_GENERATION":{"providerId":"p","modelId":"title","followsChat":false}});
    commutes::<0>(
        path,
        |_| {},
        &[
            operation(
                path,
                0,
                1,
                "function_model_binding",
                "set",
                json!(binding.to_string()),
            ),
            operation(path, 0, 2, "function_model_binding", "delete", Value::Null),
        ],
    );
    commutes::<1>(
        path,
        |values| {
            values.set(
                &stringPreferencesKey("function_model_binding"),
                binding.to_string(),
            )
        },
        &[operation(
            path,
            1,
            1,
            "function_model_binding",
            "set",
            json!(binding.to_string()),
        )],
    );
    commutes::<2>(
        path,
        |values| {
            values.set(
                &stringPreferencesKey("function_model_binding"),
                binding.clone(),
            )
        },
        &[operation(
            path,
            2,
            1,
            "function_model_binding",
            "set",
            binding.clone(),
        )],
    );
}

#[test]
fn model_type_normalization_preserves_name_patch_scope() {
    use operit_model::ModelConfigData::{ApiProviderType, ProviderProfile};
    let path = Layout::MODEL_CONFIGS_PREFERENCES_PATH;
    let mut record = serde_json::to_value(ProviderProfile::new(
        "p".into(),
        "P".into(),
        ApiProviderType::OTHER,
        String::new(),
    ))
    .unwrap();
    record["providerType"] = json!("IFLOW");
    record["providerTypeId"] = json!("IFLOW");
    commutes::<4>(
        path,
        |values| values.set(&stringPreferencesKey("provider_p"), record.clone()),
        &[operation(
            path,
            4,
            1,
            "provider_p",
            "patch_object",
            json!({"set":{"name":"Renamed"},"delete":[]}),
        )],
    );
}
