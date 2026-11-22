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
    RuntimePackageManager::reloadRegistry(app.packageManager()).await.unwrap();

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

/// Invokes the shared command router and requires an explicit machine-readable result.
async fn execute(app: &mut OperitApplication, args: &[&str]) -> Result<Value, String> {
    let args = std::iter::once("--json")
        .chain(args.iter().copied())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let output = crate::run_core_command(app, &args).await?;
    serde_json::from_str(&output.stdout).map_err(|error| error.to_string())
}
