use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use operit_host_api::HostManager::HostManager;
use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, HostResult,
    LocationData, NotificationData, OCRLanguage, OCRQuality, SystemNotificationRequest,
    SystemOperationHost, SystemSettingData,
};
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost, NativeRuntimeStorageHost,
    PosixFileSystemHost,
};
use operit_link::{toCoreValue, CoreCallRequest, CoreLinkSharedClient};
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_INPUT_MENU_TOGGLE;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::decodeToolPkgHookResult;
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::plugins::toolpkg::ToolPkgChatViewHookBridge::ToolPkgChatViewHookBridge;
use operit_runtime::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use serde_json::{json, Value};

const MODES: [&str; 2] = ["com.operit.plan_mode_bundle", "com.operit.goal_mode_bundle"];

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
        installApp(path: &str) -> AppOperationData;
        uninstallApp(packageName: &str) -> AppOperationData;
        listInstalledApps(includeSystemApps: bool) -> AppListData;
        startApp(packageName: &str) -> AppOperationData;
        stopApp(packageName: &str) -> AppOperationData;
        getNotifications(limit: i32, includeOngoing: bool) -> NotificationData;
        getAppUsageTime(packageName: &str, sinceHours: i32, limit: i32, includeSystemApps: bool) -> AppUsageTimeResultData;
        getDeviceLocation(timeout: i32, highAccuracy: bool, includeAddress: bool) -> LocationData;
        getDeviceInfo() -> DeviceInfoData;
        captureScreenshot() -> String;
        recognizeText(imagePath: &str, language: OCRLanguage, quality: OCRQuality) -> String;
    }
}

/// Runs the actual embedded mode hook in its cached main JavaScript context.
async fn input_hook(
    manager: &RuntimePackageManager,
    package: &str,
    payload: Value,
) -> Option<Value> {
    decodeToolPkgHookResult(
        manager
            .runToolPkgMainHook(
                package,
                "onInputMenuToggle",
                TOOLPKG_EVENT_INPUT_MENU_TOGGLE,
                None,
                None,
                None,
                payload,
                None,
                None,
                None,
            )
            .await
            .expect("mode input hook must execute"),
    )
}

async fn definition(manager: &RuntimePackageManager, package: &str, chat_id: &str) -> Value {
    input_hook(
        manager,
        package,
        json!({"action": "create", "chatId": chat_id, "runtime": "main"}),
    )
    .await
    .expect("mode must return its menu definition")["toggles"][0]
        .clone()
}

async fn assert_workspace_state(manager: &RuntimePackageManager, chat_id: &str, bound: bool) {
    for package in MODES {
        let toggle = definition(manager, package, chat_id).await;
        let description = toggle["description"].as_str().unwrap();
        let missing = description.contains("未绑定")
            || description.contains("No workspace is bound")
            || description.contains("A workspace is required");
        assert_eq!(!missing, bound, "{package}: {description}");
        assert_eq!(toggle["isChecked"], false, "{package} must start disarmed");
    }
}

async fn enable_modes(manager: &RuntimePackageManager, chat_id: &str) {
    for package in MODES {
        let toggle = definition(manager, package, chat_id).await;
        input_hook(
            manager,
            package,
            json!({"action": "toggle", "chatId": chat_id, "runtime": "main", "toggleId": toggle["id"]}),
        ).await;
        assert_eq!(
            definition(manager, package, chat_id).await["isChecked"],
            true
        );
    }
}

/// Covers generated Flutter creation/unbinding routes and real plugin engine rebuilds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_modes_follow_creation_reload_unbind_and_rebind() {
    tokio::task::LocalSet::new()
        .run_until(async {
            let root = std::env::temp_dir().join(format!(
                "operit-workspace-mode-sync-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            ));
            let runtime_root = root.join("runtime");
            let workspace_root = root.join("workspaces");
            std::fs::create_dir_all(&runtime_root).unwrap();
            std::fs::create_dir_all(&workspace_root).unwrap();
            setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
                runtime_root.clone(),
                workspace_root.clone(),
            ));
            let storage = Arc::new(NativeRuntimeStorageHost::new(runtime_root, workspace_root));
            let host = HostManager {
                fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
                runtimeStorageHost: Some(storage.clone()),
                runtimeSqliteHost: Some(storage),
                systemOperationHost: Some(Arc::new(TestSystemLocale)),
                hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
                hostRuntimeTaskSchedulerHost: Some(Arc::new(
                    NativeHostRuntimeTaskSchedulerHost::new(),
                )),
                ..HostManager::default()
            };
            let app = OperitApplication::newWithContext(host.clone());
            let packages = app.packageManager();
            packages.lock().unwrap().loadAvailablePackages();
            let bridge = ToolPkgBridgeRuntime::new(app.aiToolHandler(), host);
            ToolPkgChatViewHookBridge::register(bridge.clone());
            let proxy = LocalCoreProxy::new(app);
            let target = LocalCoreProxy::generatedTargetForSchema("chatRuntimeHolderMain").unwrap();
            let chat_id = {
                let holder = proxy.chatRuntimeHolder();
                let mut holder = holder.lock().await;
                holder
                    .coreForTarget(target)
                    .unwrap()
                    .chatHistoryDelegate
                    .currentChatIdFlow()
                    .value()
                    .unwrap()
            };
            let snapshot = || packages.lock().unwrap().clone();
            assert!(
                snapshot()
                    .getEnabledToolPkgContainerRuntimes()
                    .iter()
                    .any(|runtime| runtime.packageName == MODES[0]),
                "built-in modes must load: {:?}",
                snapshot().getToolPkgLoadIssues()
            );
            assert_workspace_state(&snapshot(), &chat_id, false).await;

            CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    "create-bound-workspace",
                    target,
                    "createAndBindWorkspace",
                    toCoreValue(json!({"chatId": chat_id, "name": "mode-sync-project"})).unwrap(),
                ),
            )
            .await
            .result
            .expect("workspace creation through Flutter route must succeed");
            assert_workspace_state(&snapshot(), &chat_id, true).await;
            enable_modes(&snapshot(), &chat_id).await;

            // A scan destroys both JS instances while leaving identical hook registrations.
            packages.lock().unwrap().loadAvailablePackages();
            let manager = snapshot();
            ToolPkgChatViewHookBridge::syncAndReplayToolPkgRegistrations(
                &bridge,
                &manager.getEnabledToolPkgContainerRuntimes(),
            );
            assert_workspace_state(&manager, &chat_id, true).await;
            enable_modes(&manager, &chat_id).await;

            CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    "unbind-workspace",
                    target,
                    "unbindChatFromWorkspace",
                    toCoreValue(json!({"chatId": chat_id})).unwrap(),
                ),
            )
            .await
            .result
            .expect("workspace unbind must succeed");
            assert_workspace_state(&snapshot(), &chat_id, false).await;

            CoreLinkSharedClient::call(
                &proxy,
                CoreCallRequest::new(
                    "rebind-workspace",
                    target,
                    "bindChatToWorkspace",
                    toCoreValue(json!({"chatId": chat_id, "workspace": "/app/workspaces/rebound"}))
                        .unwrap(),
                ),
            )
            .await
            .result
            .expect("workspace rebind must succeed");
            assert_workspace_state(&snapshot(), &chat_id, true).await;
            enable_modes(&snapshot(), &chat_id).await;
        })
        .await;
}
