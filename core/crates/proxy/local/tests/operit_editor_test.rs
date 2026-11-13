use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, HostResult,
    LocationData, NotificationData, OCRLanguage, OCRQuality, SystemNotificationRequest,
    SystemOperationHost, SystemSettingData,
};
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeHttpHost,
    NativeRuntimeStorageHost, PosixFileSystemHost,
};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use serde_json::json;

/// Supplies a deterministic locale and rejects unrelated system side effects.
struct TestSystemLocale;

macro_rules! unsupported_system_operations {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $result:ty;)*) => {
        $(#[allow(non_snake_case, unused_variables)]
        fn $name(&self, $($arg: $ty),*) -> HostResult<$result> {
            panic!("unexpected system operation: {}", stringify!($name));
        })*
    };
}

impl SystemOperationHost for TestSystemLocale {
    #[allow(non_snake_case)]
    fn getSystemLanguageCode(&self) -> HostResult<String> {
        Ok("zh-CN".to_string())
    }

    unsupported_system_operations! {
        sendNotification(request: &SystemNotificationRequest) -> ();
        modifySystemSetting(namespace: &str, setting: &str, value: &str) -> SystemSettingData;
        getSystemSetting(namespace: &str, setting: &str) -> SystemSettingData;
        installApp(source: &dyn operit_host_api::FileSource, timing: operit_host_api::AppInstallTiming) -> AppOperationData;
        uninstallApp(packageName: &str) -> AppOperationData;
        listInstalledApps(includeSystemApps: bool) -> AppListData;
        startApp(packageName: &str) -> AppOperationData;
        stopApp(packageName: &str) -> AppOperationData;
        getNotifications(limit: i32, includeOngoing: bool) -> NotificationData;
        getAppUsageTime(packageName: &str, sinceHours: i32, limit: i32, includeSystemApps: bool) -> AppUsageTimeResultData;
        getDeviceLocation(timeout: i32, highAccuracy: bool, includeAddress: bool) -> LocationData;
        getDeviceInfo() -> DeviceInfoData;
        captureScreenshot() -> String;
        recognizeText(imagePath: &dyn operit_host_api::FileSource, language: OCRLanguage, quality: OCRQuality) -> String;
    }
}

/// Exercises the actual built-in script, SDK bridge, tool executor and Flutter's proxy route.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editor_executes_against_the_live_runtime_without_reentering_the_application_lock() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let root = std::env::temp_dir().join(format!(
                "operit-editor-{}-{}",
                std::process::id(),
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
            ));
            let runtime_root = root.join("runtime");
            let workspace_root = root.join("workspaces");
            std::fs::create_dir_all(&runtime_root).unwrap();
            std::fs::create_dir_all(&workspace_root).unwrap();
            let storage = Arc::new(NativeRuntimeStorageHost::new(runtime_root, workspace_root));
            let mut application = OperitApplication::newWithContext(HostManager {
                fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
                httpHost: Some(Arc::new(NativeHttpHost::new())),
                systemOperationHost: Some(Arc::new(TestSystemLocale)),
                runtimeStorageHost: Some(storage.clone()),
                runtimeSqliteHost: Some(storage),
                hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
                hostRuntimeTaskSchedulerHost: Some(Arc::new(NativeHostRuntimeTaskSchedulerHost::new())),
                ..HostManager::default()
            });
            application.toolHandler.registerDefaultTools();
            let handler = application.toolHandler.clone();
            let manager = handler.getOrCreatePackageManager();
            manager.lock().unwrap().loadAvailablePackages();
            assert!(manager.lock().unwrap().getPackageTools("operit_editor").is_some());
            assert!(handler.getContext().coreCommandExecutor.is_none());
            let proxy = LocalCoreProxy::new(application);
            let executor = proxy.hostManager().coreCommandExecutor.clone().unwrap();
            assert!(handler.getContext().coreCommandExecutor.is_some());
            let help = executor(vec![]).await.unwrap();
            assert!(help.contains("operit2 package"), "help must come from the live command executor");

            // Import a real target, then let the editor enable it via the real JS-to-host bridge.
            let target = root.join("editor_target.js");
            std::fs::write(&target, r#"/* METADATA
{"name":"editor_target","description":"Editor integration target","enabledByDefault":false,"tools":[{"name":"parallel","description":"Runs concurrent editor requests","parameters":[]}]}
*/
/** Exercises repeated CLI and editor invocations from one script's Promise.all. */
exports.parallel = async function () {
    const results = await Promise.all(Array.from({length: 12}, () => Tools.SoftwareSettings.exec([
        "package", "exec", "operit_editor:operit_editor",
        JSON.stringify({args: ["package", "show", "editor_target"]})
    ])));
    if (results.length !== 12 || results.some(result => !result.includes("editor_target"))) {
        throw new Error("Concurrent editor calls lost a result");
    }
    return `parallel calls completed: ${results.length}`;
};
"#).unwrap();
            executor(vec!["package".into(), "import".into(), target.to_string_lossy().into_owned()])
                .await.unwrap();
            assert!(!manager.lock().unwrap().isPackageEnabled("editor_target"));
            executor(vec!["package".into(), "enable".into(), "operit_editor".into()])
                .await.unwrap();

            let response = tokio::time::timeout(Duration::from_secs(20), CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new("editor-enable", "core/application", "runCoreCommand", toCoreValue(json!({
                    "args": ["package", "exec", "operit_editor:operit_editor",
                        json!({"args": ["package", "enable", "editor_target"]}).to_string(), "--json"]
                })).unwrap()),
            )).await.expect("nested editor execution must not deadlock on the application dispatch lock");
            let output: operit_command_core::CoreCommandOutput = fromCoreValue(response.result.unwrap()).unwrap();
            let result: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
            assert_eq!(result["success"], true, "{output:?}");
            assert!(manager.lock().unwrap().isPackageEnabled("editor_target"),
                "the original application must observe the editor's mutation, not a second package manager");

            // Re-enter both the editor and execute_cli_command while their parent calls await.
            let mut nested_args = vec!["package".to_string(), "show".to_string(), "editor_target".to_string()];
            for _ in 0..6 {
                nested_args = vec!["package".into(), "exec".into(), "operit_editor:operit_editor".into(),
                    json!({"args": nested_args}).to_string()];
            }
            let nested_output = tokio::time::timeout(Duration::from_secs(20), executor(nested_args))
                .await.expect("repeated nested editor and CLI calls must not deadlock")
                .expect("nested editor execution must succeed");
            assert!(nested_output.contains("editor_target"), "{nested_output}");

            let parallel_output = tokio::time::timeout(Duration::from_secs(20), executor(vec![
                "package".into(), "exec".into(), "editor_target:parallel".into(), "{}".into(),
            ])).await.expect("Promise.all must complete every repeated editor call")
                .expect("parallel editor execution must succeed");
            assert!(parallel_output.contains("parallel calls completed: 12"), "{parallel_output}");

            // Runtime failures must propagate instead of returning the old fixed guide.
            let response = CoreLinkSharedClient::call(&proxy, CoreCallRequest::new(
                "editor-invalid-query", "core/application", "runCoreCommand", toCoreValue(json!({
                    "args": ["package", "exec", "operit_editor:operit_editor", "{\"query\":\"enable target\"}"]
                })).unwrap(),
            )).await;
            assert!(response.result.unwrap_err().message.contains("args"));
            let response = CoreLinkSharedClient::call(&proxy, CoreCallRequest::new(
                "editor-command-error", "core/application", "runCoreCommand", toCoreValue(json!({
                    "args": ["package", "exec", "operit_editor:operit_editor",
                        json!({"args": ["package", "show", "__missing_editor_target__"]}).to_string()]
                })).unwrap(),
            )).await;
            assert!(response.result.is_err(), "host command failure must remain a failed editor call");
            assert!(executor(vec!["package".into(), "show".into(), "__missing_editor_target__".into()])
                .await.is_err());

            // External handler references must not keep the command runtime alive after shutdown.
            drop(proxy);
            assert!(executor(vec![]).await.unwrap_err().contains("released"));
            drop(manager);
            drop(handler);
            std::fs::remove_dir_all(root).unwrap();
        })
        .await;
}
