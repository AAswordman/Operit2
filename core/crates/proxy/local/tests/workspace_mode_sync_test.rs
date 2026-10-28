use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_INPUT_MENU_TOGGLE;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::decodeToolPkgHookResult;
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
        /// Rejects unrelated system operations in this test.
        fn $name(&self, $($arg: $ty),*) -> HostResult<$result> {
            panic!("unexpected system operation: {}", stringify!($name));
        })*
    };
}

impl SystemOperationHost for TestSystemLocale {
    /// Returns the locale expected by the bundled mode fixtures.
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

/// Reads one mode menu definition for the selected chat.
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

/// Verifies both modes expose their persisted enabled state.
async fn assert_mode_state(manager: &RuntimePackageManager, chat_id: &str, checked: bool) {
    for package in MODES {
        let toggle = definition(manager, package, chat_id).await;
        assert_eq!(toggle["isChecked"], checked, "{package} toggle state");
    }
}

/// Enables both modes through their actual input hooks.
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

/// Executes an actual decorated Goal implementation through the annotation-generated Space catalog.
async fn goal_route(
    router: &operit_node_runtime::CoreNodeRouter::CoreNodeRouter,
    entry: &str,
    method: &str,
    chat: &str,
    arguments: Value,
) -> operit_link::CoreValue {
    let request = operit_link::CoreCallRequest::new(operit_link::nextCoreRouteRequestId(entry), operit_link::CORE_INTERNAL_TARGET, entry,
        operit_link::toCoreValue(json!({"bindingKey":chat,"packageName":MODES[1],"methodName":method,"arguments":arguments})).unwrap());
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        router.callSpace(request),
    )
    .await
    .expect("Plugin Call must release the Core lock")
    .result
    .unwrap()
}

/// Opens a captured stream through another service view of the same Core tree.
async fn verify_decorated_goal_routes(
    router: &operit_node_runtime::CoreNodeRouter::CoreNodeRouter,
    watch_router: &operit_node_runtime::CoreNodeRouter::CoreNodeRouter,
    chat: &str,
) {
    operit_store::CoreNodeBindingStore::CoreNodeBindingStore::default()
        .unwrap()
        .create(chat, &router.localNodeId())
        .unwrap();
    let value = goal_route(
        router,
        "callToolPkgRoute",
        "GoalService.set",
        chat,
        json!([chat, "Routed durable goal"]),
    )
    .await;
    let value: Value = operit_link::fromCoreValue(value).unwrap();
    assert_eq!(value["objective"], "Routed durable goal");
    let value = goal_route(
        router,
        "watchToolPkgRoute",
        "GoalService.watch",
        chat,
        json!([chat]),
    )
    .await;
    let descriptor = operit_link::fromCoreValue::<operit_link::CoreStream<Value>>(value)
        .unwrap()
        .descriptor;
    let args=operit_rslink_runtime::core_route_embedded_stream_open_args(descriptor.args,"call","watchToolPkgRoute".into(),
        operit_link::toCoreValue(json!({"bindingKey":chat,"packageName":MODES[1],"methodName":"GoalService.watch","arguments":[chat]})).unwrap()).unwrap();
    let mut stream = watch_router
        .watchSpace(operit_link::CoreWatchRequest::new(
            "plugin-goal-watch",
            descriptor.target,
            descriptor.propertyName,
            args,
        ))
        .await
        .unwrap();
    let event = tokio::time::timeout(std::time::Duration::from_secs(10), stream.recv())
        .await
        .unwrap()
        .unwrap();
    let mut current = event.value;
    let first: Value = operit_link::fromCoreValue(current.clone()).unwrap();
    assert_eq!(
        first["value"]["objective"], "Routed durable goal",
        "First Stream item: {first}"
    );
    goal_route(
        router,
        "callToolPkgRoute",
        "GoalService.status",
        chat,
        json!([chat, "paused"]),
    )
    .await;
    loop {
        let event = tokio::time::timeout(std::time::Duration::from_secs(10), stream.recv())
            .await
            .unwrap()
            .unwrap();
        current = if event.kind == operit_link::CoreEventKind::Delta {
            current.intoIncrementalDelta(&event.value).unwrap()
        } else {
            event.value
        };
        let value: Value = operit_link::fromCoreValue(current.clone()).unwrap();
        if value["value"]["status"] == "paused" {
            break;
        }
    }
    drop(stream);
}

/// Installs the actual TypeScript decorator fixture as an isolated Space package.
fn write_route_fixture(storage: Arc<NativeRuntimeStorageHost>) {
    use std::io::Write;
    let directory = operit_store::RuntimeStorePaths::RuntimeStorePaths::default()
        .runtime_storage_path(
            &operit_store::ExtensionStore::ExtensionStore::root("package", "space").unwrap(),
        );
    std::fs::create_dir_all(&directory).unwrap();
    let mut archive = zip::ZipWriter::new(
        std::fs::File::create(directory.join("route_fixture.toolpkg")).unwrap(),
    );
    let fixture = include_str!("../../../../../tools/tests/fixtures/compiled/toolpkg_route.js");
    let main = format!("{}\nexports.registerToolPkg=function(){{}};", fixture);
    let manifest=json!({"toolpkg_id":"route.fixture","version":"1.0.0","api_version":"2.0.0","main":"main.js","installation_scope":"space","enabled_by_default":true}).to_string();
    for (name, contents) in [("manifest.json", manifest), ("main.js", main)] {
        archive
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(contents.as_bytes()).unwrap();
    }
    archive.finish().unwrap();
    operit_store::ExtensionStore::ExtensionStore::new(storage)
        .registerInScope("package", "route.fixture", "route_fixture.toolpkg", json!({
            "members": ["route.fixture"], "enabledNames": ["route.fixture"], "disabledNames": [],
            "subpackageStates": {}, "order": 0, "builtin": false,
            "installationId": "00000000000000000000000000000000", "installationScope": "space"
        }), "space")
        .unwrap();
}

/// Uses the standard ReverseStream session on the real Binding-selected Space runtime.
async fn verify_decorated_reverse_stream(
    router: &operit_node_runtime::CoreNodeRouter::CoreNodeRouter,
    chat: &str,
) {
    let arguments=operit_link::toCoreValue(json!({"bindingKey":chat,"packageName":"route.fixture","methodName":"ExampleService.consume","arguments":[chat,null],"inputIndex":1})).unwrap();
    let request = operit_link::CorePushRequest::new(
        "plugin-input",
        operit_link::CORE_INTERNAL_TARGET,
        "pushToolPkgRoute",
    )
    .withArgs(arguments.clone());
    let mut stream = router.openSpacePush(request).await.unwrap();
    stream
        .send(operit_link::toCoreValue(2).unwrap())
        .await
        .unwrap();
    stream
        .send(operit_link::toCoreValue(3).unwrap())
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), stream.close())
        .await
        .unwrap()
        .unwrap();

    let mut rejected = router
        .openSpacePush(
            operit_link::CorePushRequest::new(
                "plugin-input-error",
                operit_link::CORE_INTERNAL_TARGET,
                "pushToolPkgRoute",
            )
            .withArgs(arguments),
        )
        .await
        .unwrap();
    rejected
        .send(operit_link::toCoreValue(-1).unwrap())
        .await
        .unwrap();
    let error = tokio::time::timeout(std::time::Duration::from_secs(10), rejected.close())
        .await
        .unwrap()
        .expect_err("The decorated consumer error must reach the ReverseStream producer");
    assert!(
        error
            .message
            .starts_with("Consumer rejected a negative value"),
        "{error}"
    );
}

/// Covers plugin-owned mode state and real plugin engine rebuilds without workspace state.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn space_modes_survive_plugin_reload_without_workspace_state() {
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
                httpHost: Some(Arc::new(NativeHttpHost::new())),
                runtimeStorageHost: Some(storage.clone()),
                runtimeSqliteHost: Some(storage.clone()),
                systemOperationHost: Some(Arc::new(TestSystemLocale)),
                hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
                hostRuntimeTaskSchedulerHost: Some(Arc::new(
                    NativeHostRuntimeTaskSchedulerHost::new(),
                )),
                ..HostManager::default()
            };
            write_route_fixture(storage.clone());
            let app = OperitApplication::newWithContext(host.clone());
            let packages = app.packageManager();
            packages.lock().unwrap().loadAvailablePackages();
            let bridge = ToolPkgBridgeRuntime::new(app.aiToolHandler(), host.clone());
            ToolPkgChatViewHookBridge::register(bridge.clone());
            let proxy = operit_proxy_local::LocalCoreProxy::new(app);
            let router = Arc::new(operit_node_runtime::CoreNodeRouter::CoreNodeRouter::new(
                proxy.coreNodeLocalRuntime(),
            ).unwrap());
            let peers = operit_node_runtime::HostRuntimePeerService::HostRuntimePeerService::new(
                Arc::new(host),
                &router,
                operit_link::protocol::LinkDeviceInfo {
                    platform: "test".into(),
                    model: "plugin-route-integration".into(),
                    pairingIncarnation: None,
                },
            )
            .unwrap();
            router
                .installNodeServices(operit_node_runtime::NodeServices::NodeServices::new(peers))
                .unwrap();
            operit_store::NetworkControlStore::NetworkControlStore::newNodeLocal(storage.clone())
                .unwrap()
                .bootstrapCurrentSpace()
                .unwrap();
            let chat_id = "space-mode-sync-chat".to_string();
            let snapshot = || packages.lock().unwrap().clone();
            assert!(
                snapshot()
                    .getEnabledToolPkgContainerRuntimes()
                    .iter()
                    .any(|runtime| runtime.packageName == MODES[0]),
                "built-in modes must load: {:?}",
                snapshot().getToolPkgLoadIssues()
            );
            let watch_router = operit_node_runtime::CoreNodeRouter::CoreNodeRouter::new(
                proxy.coreNodeLocalRuntime(),
            ).unwrap();
            verify_decorated_goal_routes(&router, &watch_router, &chat_id).await;
            verify_decorated_reverse_stream(&router, &chat_id).await;
            assert_mode_state(&snapshot(), &chat_id, false).await;

            assert_mode_state(&snapshot(), &chat_id, false).await;
            enable_modes(&snapshot(), &chat_id).await;

            // A scan destroys both JS instances while leaving identical hook registrations.
            packages.lock().unwrap().loadAvailablePackages();
            let manager = snapshot();
            ToolPkgChatViewHookBridge::syncAndReplayToolPkgRegistrations(
                &bridge,
                &manager.getEnabledToolPkgContainerRuntimes(),
            );
            assert_mode_state(&manager, &chat_id, true).await;

            assert_mode_state(&snapshot(), &chat_id, true).await;
        })
        .await;
}
