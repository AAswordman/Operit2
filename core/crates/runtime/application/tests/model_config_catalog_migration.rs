#![allow(non_snake_case)]

use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_model::ModelConfigData::{
    ApiProviderType, BuiltinToolExclusivity, BuiltinToolRequestFormat, BuiltinToolType,
    ModelBuiltinTool, ModelCapabilities, ModelCapabilityOverrides, ModelContextSpec,
    ModelListedSpec, ModelPricing, ModelProfile, PricingCurrency, ProviderProfile,
};
use operit_model::ModelParameter::{ModelParameter, ParameterValueType};
use operit_model::ModelThinking::ModelThinkingSpec;
use operit_runtime::data::preferences::ModelConfigManager::ModelConfigManager;
use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStore};
use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{
    MODEL_CONFIGS_PREFERENCES_PATH, RUNTIME_MODELS_DEV_CACHE_DIR_PATH,
};

/// Makes an existing profile whose overrides and provider-list facts block fresh recognition.
fn staleModel(id: &str) -> ModelProfile {
    let mut model = ModelProfile::new(id.into());
    model.pricingOverride = Some(ModelPricing::token(99.0, None, 99.0, PricingCurrency::USD));
    model.contextOverride = Some(ModelContextSpec {
        maxContextLength: 8.0,
    });
    model.capabilitiesOverride = ModelCapabilityOverrides {
        directImage: Some(false),
        directAudio: Some(true),
        directVideo: Some(true),
        toolCall: Some(false),
    };
    model.requestOverride.supportsStructuredTools = Some(false);
    model.requestOverride.enableClaude1hPromptCache = Some(true);
    model.listed = Some(ModelListedSpec {
        context: Some(ModelContextSpec {
            maxContextLength: 4.0,
        }),
        capabilities: Some(ModelCapabilities::default()),
        supportsStructuredTools: Some(false),
        ..ModelListedSpec::default()
    });
    model.thinkingOverride = Some(ModelThinkingSpec::switchable());
    model.builtinToolsOverride = Some(vec![ModelBuiltinTool::disabled(
        BuiltinToolType::WebSearch,
        "Stale search".into(),
        BuiltinToolRequestFormat::OpenAiWebSearch,
        BuiltinToolExclusivity::ExclusiveWithExternalTools,
    )]);
    model.summary.enableSummary = false;
    model.summary.summaryTokenThreshold = 0.12;
    model.parameters.push(ModelParameter::new(
        "temperature".into(),
        "Temperature".into(),
        "temperature".into(),
        serde_json::json!(1.0),
        serde_json::json!(0.7),
        true,
        ParameterValueType::FLOAT,
    ));
    model
}

/// Reidentifies every remote model offline, preserves provider settings, and runs only once.
#[test]
fn schema_eight_reidentifies_models_and_initializes_context_and_tools_once() {
    let root = std::env::temp_dir().join(format!(
        "operit-model-config-migration-{}",
        uuid::Uuid::new_v4()
    ));
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    setDefaultRuntimeStorageHost(storage.clone());
    let cache = RUNTIME_MODELS_DEV_CACHE_DIR_PATH;
    storage
        .writeBytes(
            &format!("{cache}/api.json"),
            br#"{
        "openai": {"api": "https://api.openai.com/v1", "models": {
            "migration-large": {
                "id": "migration-large", "tool_call": true,
                "modalities": {"input": ["text", "image"], "output": ["text"]},
                "limit": {"context": 1048576}
            },
            "migration-small": {
                "id": "migration-small", "tool_call": true,
                "limit": {"context": 128000}
            },
            "migration-no-tools": {
                "id": "migration-no-tools", "tool_call": false,
                "limit": {"context": 400000}
            }
        }}
    }"#,
        )
        .unwrap();
    storage
        .writeBytes(&format!("{cache}/models.json"), b"{}")
        .unwrap();
    storage
        .writeBytes(&format!("{cache}/downloaded.json"), br#"{"updatedAt":1}"#)
        .unwrap();

    let mut provider = ProviderProfile::new(
        "remote".into(),
        "Remote".into(),
        ApiProviderType::OPENAI,
        "https://api.openai.com/v1/chat/completions".into(),
    );
    provider.apiKey = "preserved-test-key".into();
    provider.customHeaders = r#"{"X-Custom":"preserved"}"#.into();
    provider.requestLimitPerMinute = 15;
    provider.maxConcurrentRequests = 3;
    provider.models = [
        "migration-large",
        "migration-small",
        "migration-no-tools",
        "migration-unknown",
    ]
    .into_iter()
    .map(staleModel)
    .collect();
    let mut unindexed = provider.clone();
    unindexed.id = "unindexed".into();
    unindexed.name = "Unindexed".into();
    let mut local = ProviderProfile::new(
        "local".into(),
        "Local".into(),
        ApiProviderType::LOCAL_MODEL,
        String::new(),
    );
    local.models.push(staleModel("local-model"));
    let legacy = serde_json::json!({
        "__operit_preferences_schema_version": "8",
        "provider_remote": provider,
        "provider_unindexed": unindexed,
        "provider_local": local,
        "unrelated_preference": "preserved"
    });
    storage
        .writeBytes(
            MODEL_CONFIGS_PREFERENCES_PATH,
            &serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();

    let manager = ModelConfigManager::default();
    for providerId in ["remote", "unindexed"] {
        let migrated = manager.getProviderProfile(providerId).unwrap();
        let mut expected = if providerId == "remote" {
            provider.clone()
        } else {
            unindexed.clone()
        };
        expected.models = migrated.models.clone();
        assert_eq!(
            migrated, expected,
            "provider credentials and settings must survive"
        );
        for (modelId, context, threshold, tools) in [
            ("migration-large", 1048.576, 0.28, true),
            ("migration-small", 128.0, 0.85, true),
            ("migration-no-tools", 400.0, 0.75, false),
            ("migration-unknown", 200.0, 0.85, true),
        ] {
            let profile = manager.getModelProfile(providerId, modelId).unwrap();
            assert!(profile.pricingOverride.is_none());
            assert!(profile.contextOverride.is_none());
            assert!(profile.listed.is_none());
            assert!(profile.thinkingOverride.is_none());
            assert_eq!(
                profile.builtinToolsOverride,
                (modelId != "migration-unknown").then(Vec::new),
            );
            assert_eq!(
                profile.requestOverride.enableClaude1hPromptCache,
                Some(true)
            );
            assert_eq!(profile.parameters, staleModel(modelId).parameters);
            assert!(profile.summary.enableSummary);
            assert!((profile.summary.summaryTokenThreshold - threshold).abs() < 1e-6);
            assert_eq!(
                profile.capabilitiesOverride.toolCall,
                (modelId == "migration-unknown").then_some(true)
            );
            let resolved = manager.getResolvedModelConfig(providerId, modelId).unwrap();
            assert!((resolved.context.maxContextLength - context).abs() < 1e-3);
            assert_eq!(resolved.capabilities.toolCall, tools);
            assert_eq!(resolved.request.supportsStructuredTools, tools);
            assert_eq!(
                resolved.capabilities.directImage,
                modelId == "migration-large"
            );
            assert!(!resolved.capabilities.directAudio);
            assert!(!resolved.capabilities.directVideo);
        }
    }

    // Choices made after migration survive an independent storage reopen.
    let mut edited = manager
        .getModelProfile("remote", "migration-unknown")
        .unwrap();
    edited.capabilitiesOverride.toolCall = Some(false);
    edited.requestOverride.supportsStructuredTools = Some(false);
    edited.summary.summaryTokenThreshold = 0.61;
    manager
        .updateModelProfile("remote", edited.clone())
        .unwrap();
    let reopened = PreferencesDataStore::newWithStorage(
        Arc::new(NativeRuntimeStorageHost::new(
            root.join("runtime"),
            root.join("workspaces"),
        )),
        MODEL_CONFIGS_PREFERENCES_PATH,
    )
    .withMigration(9, |_, _| panic!("completed migration must not run again"));
    assert_eq!(reopened.schemaVersion().unwrap(), 9);
    let persisted = reopened.data().unwrap();
    let savedLocal: ProviderProfile = serde_json::from_value(
        persisted
            .get(&stringPreferencesKey("provider_local"))
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert_eq!(savedLocal, local);
    assert_eq!(
        persisted.get(&stringPreferencesKey("unrelated_preference")),
        Some(&serde_json::json!("preserved"))
    );
    let saved: ProviderProfile = serde_json::from_value(
        persisted
            .get(&stringPreferencesKey("provider_remote"))
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert_eq!(saved.models.last().unwrap(), &edited);
}
