use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_host_native_filesystem::PosixFileSystemHost;
use operit_host_native_scheduler::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
};
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::data::preferences::ApiPreferences::ApiPreferences;
use operit_runtime::data::preferences::PreferenceStorageManager::PreferenceStorageManager;
use operit_runtime::data::preferences::ThemeConfigManager::ThemeConfigManager;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use serde_json::{json, Value};

/// Gives the complete application and command futures a dedicated test stack.
#[test]
fn settings_commands_share_gui_stores_and_reject_invalid_edits() {
    std::thread::Builder::new()
        .name("cli-settings-regression".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(Box::pin(exercise_settings()));
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Exercises settings through the public router, JSON finalizer, VFS and real preference owners.
async fn exercise_settings() {
    let root = std::env::temp_dir().join(format!(
        "operit-cli-settings-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("runtime")).unwrap();
    std::fs::create_dir_all(root.join("workspaces/demo")).unwrap();
    let storage = Arc::new(NativeRuntimeStorageHost::new(
        root.join("runtime"),
        root.join("workspaces"),
    ));
    let mut hosts = HostManager::withFileSystemHost(Arc::new(PosixFileSystemHost::new()));
    hosts.runtimeStorageHost = Some(storage.clone());
    hosts.runtimeSqliteHost = Some(storage.clone());
    hosts.hostJavaScriptRuntimeHost = Some(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    hosts.hostRuntimeTaskSchedulerHost = Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let mut app = OperitApplication::newWithContext(hosts);
    RuntimePackageManager::reloadRegistry(app.packageManager())
        .await
        .unwrap();

    for args in [
        vec!["terminal"],
        vec!["theme"],
        vec!["tts"],
        vec!["model"],
        vec!["prefs"],
        vec!["stt"],
        vec!["local-models"],
    ] {
        assert!(execute(&mut app, &args).await.unwrap()["usage"].is_array());
    }
    execute(&mut app, &["prefs", "tools", "off"]).await.unwrap();
    assert!(!ApiPreferences::getInstance()
        .enableToolsFlow()
        .first()
        .unwrap());
    execute(&mut app, &["prefs", "input", "long-paste", "off", "5000"])
        .await
        .unwrap();
    let store = PreferenceStorageManager::getInstance();
    let file = "user_preferences.preferences.json";
    assert_eq!(
        store
            .getPreference(file, "long_pasted_text_input_threshold")
            .unwrap(),
        Some(json!(5000))
    );
    assert!(
        execute(&mut app, &["prefs", "input", "long-paste", "on", "10"])
            .await
            .is_err()
    );
    assert_eq!(
        store
            .getPreference(file, "long_pasted_text_input_threshold")
            .unwrap(),
        Some(json!(5000))
    );
    execute(&mut app, &["prefs", "input", "enter-send", "off"])
        .await
        .unwrap();
    execute(&mut app, &["prefs", "input", "enter-send", "auto"])
        .await
        .unwrap();
    assert_eq!(
        store.getPreference(file, "enter_sends_message").unwrap(),
        None
    );
    assert!(execute(&mut app, &["prefs", "language", "invalid"])
        .await
        .is_err());
    execute(&mut app, &["prefs", "language", "zh-CN"])
        .await
        .unwrap();
    assert_eq!(
        execute(&mut app, &["prefs", "show"]).await.unwrap()["language"],
        "zh-CN"
    );
    exercise_model_options(&mut app, &root).await;

    let mut snapshot: Value = serde_json::from_str(include_str!(
        "../../../../runtime/application/src/data/preferences/theme_snapshot_fixture.json"
    ))
    .unwrap();
    let snapshot_json = snapshot.to_string();
    std::fs::write(root.join("workspaces/demo/theme.json"), &snapshot_json).unwrap();
    let created = execute(
        &mut app,
        &[
            "theme",
            "create",
            "CLI theme",
            "@/operitapp/workspaces/demo/theme.json",
        ],
    )
    .await
    .unwrap();
    let id = created["id"].as_str().unwrap();
    execute(&mut app, &["theme", "apply", id]).await.unwrap();
    assert!(execute(&mut app, &["theme", "delete", id]).await.is_err());
    snapshot["themeMode"] = json!("dark");
    execute(
        &mut app,
        &["theme", "appearance-save", &snapshot.to_string()],
    )
    .await
    .unwrap();
    assert_eq!(
        ThemeConfigManager::new(storage)
            .getActive()
            .unwrap()
            .unwrap()
            .snapshot["themeMode"],
        "dark"
    );
    let before = execute(&mut app, &["theme", "active"]).await.unwrap();
    assert!(execute(
        &mut app,
        &["theme", "appearance-save", r#"{"themeMode":"invalid"}"#]
    )
    .await
    .is_err());
    assert_eq!(
        execute(&mut app, &["theme", "active"]).await.unwrap(),
        before
    );
    assert!(execute(
        &mut app,
        &[
            "theme",
            "appearance-patch",
            &snapshot.to_string(),
            r#"{"app_language":"en"}"#
        ]
    )
    .await
    .is_err());

    let tts = execute(
        &mut app,
        &[
            "tts",
            "config",
            "create",
            "CLI voice",
            "https://example.test",
            "test-secret",
            "tts-1",
            "alloy",
            "mp3",
            "1",
        ],
    )
    .await
    .unwrap();
    let voice_id = tts["id"].as_str().unwrap();
    let mut tts = tts.clone();
    tts["headers"] = json!([{"name": "X-Voice", "value": "example"}]);
    execute(&mut app, &["tts", "config", "save", &tts.to_string()])
        .await
        .unwrap();
    assert_eq!(
        execute(&mut app, &["tts", "config", "show", voice_id])
            .await
            .unwrap()["headers"],
        tts["headers"]
    );
    let mut human = crate::CoreCommandOutput::new();
    super::tts::run_tts_command(
        &app,
        &["config".into(), "show".into(), voice_id.into()],
        &mut human,
    )
    .unwrap();
    assert!(!human.stdout.contains("test-secret"));

    let stt = json!({"id":"", "name":"CLI recognition", "providerType":"OPENAI_COMPATIBLE", "endpoint":"https://example.test", "apiKey":"stt-key", "model":"whisper-1", "fileFieldName":"file", "modelFieldName":"model", "languageFieldName":"language", "responseTextJsonPath":"text", "headers":[{"name":"X-Source", "value":"cli"}], "createdAt":0, "updatedAt":0});
    let created = execute(
        &mut app,
        &["stt", "config", "create-json", &stt.to_string()],
    )
    .await
    .unwrap();
    let stt_id = created["id"].as_str().unwrap();
    let mut stt = stt;
    stt["id"] = json!(stt_id);
    stt["fileFieldName"] = json!("audio");
    execute(&mut app, &["stt", "config", "save", &stt.to_string()])
        .await
        .unwrap();
    assert_eq!(
        execute(&mut app, &["stt", "config", "show", stt_id])
            .await
            .unwrap()["fileFieldName"],
        "audio"
    );
    assert!(execute(&mut app, &["terminal", "profiles"]).await.is_err());
}

/// Checks single-option edits, inheritance, VFS input and rejected writes through the public router.
async fn exercise_model_options(app: &mut OperitApplication, root: &std::path::Path) {
    let created = execute(
        app,
        &[
            "model",
            "provider-create",
            "CLI switches",
            "OPENAI",
            "https://example.test/v1",
        ],
    )
    .await
    .unwrap();
    let provider = created["providerId"].as_str().unwrap();
    execute(
        app,
        &[
            "model",
            "provider-model-create",
            provider,
            "cli-settings-model",
        ],
    )
    .await
    .unwrap();
    let model = "cli-settings-model";
    execute(
        app,
        &["model", "provider-set-key", provider, "preserved-secret"],
    )
    .await
    .unwrap();

    for (field, key) in [
        ("tool-call", "toolCall"),
        ("direct-image", "directImage"),
        ("direct-audio", "directAudio"),
        ("direct-video", "directVideo"),
    ] {
        for enabled in ["true", "false"] {
            let result = execute(
                app,
                &["model", "capabilities-set", provider, model, field, enabled],
            )
            .await
            .unwrap();
            assert_eq!(result["capabilities"][key], enabled == "true");
            assert_eq!(result["overrides"][key], enabled == "true");
            assert_eq!(
                execute(app, &["model", "show", provider, model])
                    .await
                    .unwrap()["capabilities"][key],
                enabled == "true"
            );
        }
    }
    let capabilities = execute(app, &["model", "capabilities-show", provider, model])
        .await
        .unwrap();
    for (field, key) in [
        ("tool-call", "toolCall"),
        ("direct-image", "directImage"),
        ("direct-audio", "directAudio"),
        ("direct-video", "directVideo"),
    ] {
        let result = execute(
            app,
            &["model", "capabilities-set", provider, model, field, "auto"],
        )
        .await
        .unwrap();
        assert!(result["overrides"][key].is_null());
    }
    for (field, key) in [
        ("structured-tools", "supportsStructuredTools"),
        ("claude-1h-prompt-cache", "enableClaude1hPromptCache"),
    ] {
        for enabled in ["true", "false", "auto"] {
            let result = execute(
                app,
                &["model", "request-set", provider, model, field, enabled],
            )
            .await
            .unwrap();
            if enabled == "auto" {
                assert!(result["overrides"][key].is_null());
            } else {
                assert_eq!(result["request"][key], enabled == "true");
            }
        }
    }
    assert!(execute(app, &["model", "request-show", provider, model])
        .await
        .unwrap()["request"]
        .is_object());
    let original = execute(app, &["model", "profile-show", provider, model])
        .await
        .unwrap();
    for args in [
        vec!["model", "capabilities-set", provider, model, "typo", "true"],
        vec![
            "model",
            "capabilities-set",
            provider,
            model,
            "tool-call",
            "enabled",
        ],
        vec!["model", "request-set", provider, model, "tool-call", "true"],
        vec!["model", "profile-set", provider, model, "id", "\"other\""],
        vec![
            "model",
            "profile-set",
            provider,
            model,
            "local-runtime",
            "{}",
        ],
        vec![
            "model",
            "capabilities-set",
            provider,
            model,
            "tool-call",
            "true",
            "extra",
        ],
    ] {
        assert!(execute(app, &args).await.is_err());
    }
    assert_eq!(
        execute(app, &["model", "profile-show", provider, model])
            .await
            .unwrap(),
        original
    );
    assert!(execute(
        app,
        &[
            "model",
            "capabilities-set",
            provider,
            "missing",
            "tool-call",
            "true"
        ]
    )
    .await
    .is_err());

    let tools = json!([{"toolType":"WebSearch", "displayName":"Search", "enabled":false,
        "requestFormat":"OpenAiWebSearch", "exclusivity":"CanMixWithExternalTools", "config":{"searchContextSize":"low"}}]);
    execute(
        app,
        &[
            "model",
            "profile-set",
            provider,
            model,
            "builtin-tools",
            &tools.to_string(),
        ],
    )
    .await
    .unwrap();
    let enabled = execute(
        app,
        &[
            "model",
            "builtin-tool-set",
            provider,
            model,
            "WebSearch",
            "true",
        ],
    )
    .await
    .unwrap();
    assert_eq!(enabled["builtinTools"][0]["enabled"], true);
    assert_eq!(enabled["builtinTools"][0]["config"], tools[0]["config"]);
    assert_eq!(
        execute(app, &["model", "builtin-tools-show", provider, model])
            .await
            .unwrap()["builtinTools"],
        enabled["builtinTools"]
    );
    assert!(execute(
        app,
        &[
            "model",
            "builtin-tool-set",
            provider,
            model,
            "CodeExecution",
            "true"
        ]
    )
    .await
    .is_err());

    let parameters = json!([{"id":"temperature", "name":"Temperature", "apiName":"temperature", "description":"", "defaultValue":0.7,
        "currentValue":0.7, "isEnabled":true, "valueType":"FLOAT", "minValue":0, "maxValue":2, "category":"GENERATION", "isCustom":false}]);
    execute(
        app,
        &[
            "model",
            "profile-set",
            provider,
            model,
            "parameters",
            &parameters.to_string(),
        ],
    )
    .await
    .unwrap();
    execute(
        app,
        &[
            "model",
            "param-set",
            provider,
            model,
            "temperature",
            "false",
        ],
    )
    .await
    .unwrap();
    let parameters = execute(app, &["model", "params", provider, model])
        .await
        .unwrap();
    assert_eq!(parameters[0]["isEnabled"], false);
    assert_eq!(parameters[0]["currentValue"], 0.7);
    std::fs::write(root.join("workspaces/demo/parameter.json"), "0.3").unwrap();
    execute(
        app,
        &[
            "model",
            "param-set",
            provider,
            model,
            "temperature",
            "true",
            "@/operitapp/workspaces/demo/parameter.json",
        ],
    )
    .await
    .unwrap();
    let parameters = execute(app, &["model", "params", provider, model])
        .await
        .unwrap();
    assert_eq!(parameters[0]["currentValue"], 0.3);
    assert_eq!(parameters[0]["isEnabled"], true);
    assert!(execute(
        app,
        &[
            "model",
            "param-set",
            provider,
            model,
            "temperature",
            "false",
            "invalid-json"
        ]
    )
    .await
    .is_err());
    assert!(execute(
        app,
        &["model", "param-set", provider, model, "missing", "false"]
    )
    .await
    .is_err());
    assert_eq!(
        execute(app, &["model", "params", provider, model])
            .await
            .unwrap(),
        parameters
    );

    let mut runtime = original["localRuntime"].clone();
    runtime["llamaThreadCount"] = json!(7);
    std::fs::write(
        root.join("workspaces/demo/runtime.json"),
        runtime.to_string(),
    )
    .unwrap();
    execute(
        app,
        &[
            "model",
            "profile-set",
            provider,
            model,
            "local-runtime",
            "@/operitapp/workspaces/demo/runtime.json",
        ],
    )
    .await
    .unwrap();
    let profile = execute(app, &["model", "profile-show", provider, model])
        .await
        .unwrap();
    assert_eq!(profile["localRuntime"], runtime);
    assert_eq!(profile["summary"], original["summary"]);
    assert_eq!(
        profile["capabilitiesOverride"],
        original["capabilitiesOverride"]
    );
    let pricing = json!({"billingMode":"TOKEN", "inputPricePerMillion":1, "cachedInputPricePerMillion":null,
        "cacheWritePricePerMillion":null, "outputPricePerMillion":2, "pricePerRequest":0, "currency":"USD"});
    execute(
        app,
        &[
            "model",
            "profile-set",
            provider,
            model,
            "pricing",
            &pricing.to_string(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        execute(app, &["model", "show", provider, model])
            .await
            .unwrap()["pricing"]["inputPricePerMillion"],
        1.0
    );
    execute(
        app,
        &["model", "profile-set", provider, model, "pricing", "null"],
    )
    .await
    .unwrap();
    assert!(execute(app, &["model", "profile-show", provider, model])
        .await
        .unwrap()["pricingOverride"]
        .is_null());

    for (field, value) in [
        ("name", "CLI renamed"),
        ("request-limit", "12"),
        ("max-concurrent", "3"),
        ("multiple-api-keys", "true"),
        ("custom-headers", r#"{"X-Source":"cli"}"#),
    ] {
        execute(app, &["model", "provider-set", provider, field, value])
            .await
            .unwrap();
    }
    let saved = execute(app, &["model", "provider-show", provider])
        .await
        .unwrap();
    assert_eq!(saved["requestLimitPerMinute"], 12);
    assert_eq!(saved["maxConcurrentRequests"], 3);
    assert_eq!(saved["useMultipleApiKeys"], true);
    assert_eq!(saved["name"], "CLI renamed");
    assert_eq!(saved["apiKeyLength"], "preserved-secret".len());
    assert!(!saved.to_string().contains("preserved-secret"));
    assert_eq!(
        saved["models"][0],
        execute(app, &["model", "profile-show", provider, model])
            .await
            .unwrap()
    );
    for (field, value) in [
        ("request-limit", "-1"),
        ("max-concurrent", "no"),
        ("multiple-api-keys", "no"),
        ("custom-headers", r#"{"bad":123}"#),
        ("models", "[]"),
    ] {
        assert!(
            execute(app, &["model", "provider-set", provider, field, value])
                .await
                .is_err()
        );
    }
    assert_eq!(
        execute(app, &["model", "provider-show", provider])
            .await
            .unwrap(),
        saved
    );
    assert!(capabilities["capabilities"].is_object());
}

/// Invokes the shared command router and requires an explicit machine-readable result.
async fn execute(app: &mut OperitApplication, args: &[&str]) -> Result<Value, String> {
    let args = std::iter::once("--json")
        .chain(args.iter().copied())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let output = crate::run_core_command(app, &args).await?;
    serde_json::from_str(&output.stdout).map_err(|error| error.to_string())
}
