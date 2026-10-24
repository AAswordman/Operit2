use super::*;
use crate::CoreApplication;
use operit_host_api::HostManager::HostManager;
use operit_host_native_common::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
    NativeRuntimeStorageHost, PosixFileSystemHost,
};
use operit_plugin_sdk::javascript::{JsDeviceToolCall, JsToolCallResultData};
use operit_proxy_local::LocalCoreProxy;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::AIToolHandler::{AIToolHandler, AsyncFnToolExecutor, FnToolExecutor};
use operit_tools::tools::AIToolHook::{AIToolHook, AIToolHookDecision};
use operit_tools::tools::ToolResultDataClasses::{
    stringResultData, BinaryResultData, ToolResultData,
};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{ToolEffect, ToolValidationResult};
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use operit_host_api::{
    AppListData, AppOperationData, AppUsageTimeResultData, DeviceInfoData, HostResult,
    LocationData, NotificationData, OCRLanguage, OCRQuality, SystemNotificationRequest,
    SystemOperationHost, SystemSettingData,
};

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
        Ok("en".to_string())
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

#[derive(Default)]
struct BlockProbe { calls: AtomicUsize }
impl AIToolHook for BlockProbe {
    fn id(&self) -> &str { "device-probe-hook" }
    fn onToolCallRequested(&self, _: &operit_tools::ToolExecutionManager::AITool) { self.calls.fetch_add(1, Ordering::SeqCst); }
    fn onToolCallIntercept(&self, tool: &operit_tools::ToolExecutionManager::AITool) -> AIToolHookDecision {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if tool.name == "blocked_probe" { AIToolHookDecision::Block("Blocked by AI hook".into()) }
        else { AIToolHookDecision::Allow }
    }
    fn onToolPermissionChecked(&self, _: &operit_tools::ToolExecutionManager::AITool, _: bool, _: Option<&str>) { self.calls.fetch_add(1, Ordering::SeqCst); }
    fn onToolExecutionStarted(&self, _: &operit_tools::ToolExecutionManager::AITool) { self.calls.fetch_add(1, Ordering::SeqCst); }
    fn onToolExecutionResult(&self, _: &operit_tools::ToolExecutionManager::AITool, _: &ToolResult) { self.calls.fetch_add(1, Ordering::SeqCst); }
    fn onToolExecutionError(&self, _: &operit_tools::ToolExecutionManager::AITool, _: &str) { self.calls.fetch_add(1, Ordering::SeqCst); }
    fn onToolExecutionFinished(&self, _: &operit_tools::ToolExecutionManager::AITool) { self.calls.fetch_add(1, Ordering::SeqCst); }
}
struct Cancelled(Arc<AtomicBool>);
impl Drop for Cancelled {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn request(node: &str, id: &str, name: &str, timeout: u64) -> JsToolCallRequest {
    JsToolCallRequest {
        tool_type: "default".into(),
        tool_name: name.into(),
        parameters: Default::default(),
        device: Some(JsDeviceToolCall {
            node_id: node.into(),
            invocation_id: id.into(),
            action: JsDeviceToolAction::Execute,
            timeout_ms: timeout,
        }),
    }
}
async fn wait(flag: &AtomicBool) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !flag.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn device_tools_direct_calls_skip_ai_policy_and_preserve_transport_semantics() {
    let root = std::env::temp_dir().join(format!("operit-device-runtime-{}", uuid_suffix()));
    let data = root.join("runtime");
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(data.clone(), workspace.clone()));
    let storage = Arc::new(NativeRuntimeStorageHost::new(data, workspace));
    let scheduler = Arc::new(NativeHostRuntimeTaskSchedulerHost::new());
    let speech_http=Arc::new(crate::speech_transport_tests::SpeechHttp::default());
    let host = HostManager {
        runtimeStorageHost: Some(storage.clone()),
        runtimeStorageWriteHost: Some(storage.clone()),
        runtimeSqliteHost: Some(storage),
        fileSystemHost: Some(Arc::new(PosixFileSystemHost::new())),
        httpHost: Some(speech_http.clone()),
        tcpHost: Some(Arc::new(operit_host_native_common::NativeTcpHost)),
        systemOperationHost: Some(Arc::new(TestSystemLocale)),
        hostRuntimeTaskSchedulerHost: Some(scheduler.clone()),
        hostJavaScriptRuntimeHost: Some(Arc::new(NativeHostJavaScriptRuntimeHost::new())),
        ..HostManager::default()
    };
    let application = OperitApplication::newWithContext(host);
    let mut handler: AIToolHandler = application.toolHandler.clone();
    let executor = || {
        Box::new(FnToolExecutor {
            invoke: Arc::new(|tool| ToolResult {
                toolName: tool.name.clone(),
                success: true,
                result: ToolResultData::BinaryResultData(BinaryResultData {
                    value: vec![0, 1, 255],
                }),
                error: None,
            }),
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            effect: ToolEffect::WRITE,
        })
    };
    handler.registerTool("binary_probe".into(), executor());
    handler.registerTool("blocked_probe".into(), executor());
    handler.registerInternalTool("internal_probe".into(), executor());
    handler.registerTool("enabled_device_probe:echo".into(), executor());
    handler.registerTool("disabled_device_probe:echo".into(), executor());
    {
        let manager = handler.getOrCreatePackageManager();
        let mut manager = manager.lock().unwrap();
        for (name, enabled) in [
            ("enabled_device_probe", true),
            ("disabled_device_probe", false),
        ] {
            manager.setAvailablePackage(
                name.into(),
                operit_plugin_sdk::package::ToolPackage {
                    name: name.into(),
                    enabled_by_default: enabled,
                    is_built_in: true,
                    tools: vec![operit_plugin_sdk::package::PackageTool {
                        name: "echo".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            );
        }
    }
    let hooks = Arc::new(BlockProbe::default());
    handler.addToolHook(hooks.clone());
    let approvals = Arc::new(AtomicUsize::new(0));
    let requested = approvals.clone();
    handler.getToolPermissionSystem().saveAiPermissionMode(
        operit_tools::tools::ToolPermissionSystem::AiPermissionMode::ReadOnly,
    ).unwrap();
    handler.getToolPermissionSystem().setAsyncPermissionRequester(move |_, _, _| {
        requested.fetch_add(1, Ordering::SeqCst);
        async { operit_tools::tools::ToolPermissionSystem::PermissionRequestResult::DENY }
    });
    // The identical executors must still obey AI interception and permission policy.
    use operit_tools::ToolExecutionManager::{ToolCall, ToolExecutionPolicy, ToolExposureMode};
    let ai = |name: &str| ToolCall { name: name.into(), parameters: Default::default() };
    let blocked = handler.executeToolInvocation(&ai("blocked_probe"),
        ToolExecutionPolicy::Ai { exposure: ToolExposureMode::FULL }).await.unwrap();
    assert!(blocked[0].error.as_deref().unwrap().contains("Blocked by AI hook"));
    let denied = handler.executeToolInvocation(&ai("binary_probe"),
        ToolExecutionPolicy::Ai { exposure: ToolExposureMode::FULL }).await.unwrap();
    assert!(!denied[0].success);
    assert_eq!(approvals.load(Ordering::SeqCst), 1);
    assert!(hooks.calls.swap(0, Ordering::SeqCst) > 0);
    approvals.store(0, Ordering::SeqCst);

    let entered = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let started = entered.clone();
    let dropped = cancelled.clone();
    handler.registerAsyncTool(
        "delayed_probe".into(),
        Box::new(AsyncFnToolExecutor {
            invoke: Arc::new(move |tool, _context| {
                let started = started.clone();
                let dropped = dropped.clone();
                Box::pin(async move {
                    if !tool
                        .parameters
                        .iter()
                        .any(|parameter| parameter.name == "fast")
                    {
                        let _cancelled = Cancelled(dropped);
                        started.store(true, Ordering::SeqCst);
                        std::future::pending::<()>().await;
                    }
                    ToolResult {
                        toolName: tool.name,
                        success: true,
                        result: stringResultData("ready"),
                        error: None,
                    }
                })
            }),
            validate: Arc::new(|_| ToolValidationResult {
                valid: true,
                errorMessage: String::new(),
            }),
            effect: ToolEffect::READ,
        }),
    );
    let proxy = Arc::new(LocalCoreProxy::new(application));
    let mut core = CoreApplication::startWithSharedLocalClientConfigured(
        proxy.clone(),
        operit_link::protocol::LinkDeviceInfo {
            platform: "test".into(),
            model: "device tools".into(),
            pairingIncarnation: None,
        },
        false,
    )
    .unwrap();
    let node = core.localNodeId();
    let runtime = CoreDeviceToolRuntime::new(Arc::downgrade(&core.nodeRouter), scheduler);
    // Exercise the service injected by Core assembly through the actual JS host.
    let bound = operit_plugin_sdk::javascript::JsExecutionHost::execute_tool_call(
        &handler,
        request(&node, "bound-runtime", "binary_probe", 2000),
    )
    .await;
    assert!(bound.success, "{:?}", bound.error);
    let direct = operit_plugin_sdk::javascript::JsExecutionHost::execute_tool_call(
        &handler, request(&node, "bound-no-intercept", "blocked_probe", 2000),
    ).await;
    assert!(direct.success, "Direct JS must bypass both source and target AI hooks: {:?}", direct.error);
    assert_eq!(hooks.calls.load(Ordering::SeqCst), 0);
    assert_eq!(approvals.load(Ordering::SeqCst), 0);
    // Exercise real SDK namespaces through native QuickJS, not only the Rust adapter.
    let engine = handler.runtimeDependencies().js_execution_provider()
        .create_execution_engine(Arc::new(handler.clone())).unwrap();
    let mut params = std::collections::BTreeMap::new();
    params.insert("node".into(), node.clone().into());
    params.insert("__operit_toolpkg_api_version".into(), "2.0.0".into());
    let result = engine.execute_script_function_async(
        r#"exports.run=async function(params) {
            await Tools.Files.mkdir('/app/data/direct-sdk',true);
            await Tools.Files.write('/app/data/direct-sdk/probe.txt','direct');
            var local=await Tools.Files.exists('/app/data/direct-sdk/probe.txt');
            var remote=await Tools.device(params.node).Files.exists('/app/data/direct-sdk/probe.txt');
            await Tools.device(params.node).call('blocked_probe',{});
            await Tools.device(params.node).call('internal_probe',{});
            return {local:local.exists,remote:remote.exists};
        };"#.into(),
        "run".into(), params, Default::default(), None, false,
    ).await.unwrap().unwrap();
    engine.destroy();
    let result: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(result, serde_json::json!({"local":true,"remote":true}));
    assert_eq!(hooks.calls.load(Ordering::SeqCst), 0, "Real Tools namespaces must skip AI lifecycle");
    assert_eq!(approvals.load(Ordering::SeqCst), 0, "Real Tools writes must skip AI approval");
    let immediate = operit_plugin_sdk::javascript::JsExecutionHost::execute_tool_call(
        &handler,
        request(&node, "immediate-cancel", "delayed_probe", 2000),
    );
    let mut immediate_cancel = request(&node, "immediate-cancel", "delayed_probe", 2000);
    immediate_cancel.device.as_mut().unwrap().action = JsDeviceToolAction::Cancel;
    assert!(
        operit_plugin_sdk::javascript::JsExecutionHost::execute_tool_call(
            &handler,
            immediate_cancel
        )
        .await
        .success
    );
    assert!(immediate.await.error.unwrap().contains("cancelled"));
    let mut catalog_request = request(&node, "catalog", "unused", 2000);
    catalog_request.device.as_mut().unwrap().action = JsDeviceToolAction::Catalog;
    let catalog = runtime.dispatch(catalog_request).await;
    assert!(catalog.success, "{:?}", catalog.error);
    let JsToolCallResultData::Value(catalog) = catalog.data else {
        panic!("Catalog must be structured");
    };
    let tools = catalog["tools"].as_array().unwrap();
    assert!(!tools.is_empty());
    assert!(tools
        .iter()
        .any(|tool| tool["name"] == "enabled_device_probe:echo"));
    let local_catalog = operit_plugin_sdk::javascript::JsExecutionHost::get_tool_catalog(&handler).unwrap();
    assert_eq!(catalog, local_catalog, "Device selection must not add an AI visibility filter");
    let enabled = runtime
        .dispatch(request(
            &node,
            "enabled-plugin",
            "enabled_device_probe:echo",
            2000,
        ))
        .await;
    assert!(enabled.success, "{:?}", enabled.error);
    let disabled = runtime
        .dispatch(request(
            &node,
            "disabled-plugin",
            "disabled_device_probe:echo",
            2000,
        ))
        .await;
    assert!(disabled.success, "Direct device calls use the same registered executors as local JS: {:?}", disabled.error);
    assert!(
        !runtime
            .dispatch(request(&node, "missing", "unknown_device_probe", 2000))
            .await
            .success
    );
    let binary = runtime
        .dispatch(request(&node, "binary", "binary_probe", 2000))
        .await;
    assert!(binary.success, "{:?}", binary.error);
    assert!(matches!(binary.data, JsToolCallResultData::Binary(bytes) if bytes == [0,1,255]));
    let hidden = runtime
        .dispatch(request(&node, "hidden", "internal_probe", 2000))
        .await;
    assert!(hidden.success, "SDK-internal tools must remain callable on selected devices: {:?}", hidden.error);
    let blocked = runtime
        .dispatch(request(&node, "blocked", "blocked_probe", 2000))
        .await;
    assert!(blocked.success, "Direct device calls must not enter AI interception: {:?}", blocked.error);

    let pending = runtime.dispatch(request(&node, "cancelled", "delayed_probe", 2000));
    wait(&entered).await;
    let duplicate = runtime
        .dispatch(request(&node, "cancelled", "delayed_probe", 2000))
        .await;
    assert!(!duplicate.success);
    assert!(duplicate.error.unwrap().contains("Duplicate"));
    let mut cancel = request(&node, "cancelled", "delayed_probe", 2000);
    cancel.device.as_mut().unwrap().action = JsDeviceToolAction::Cancel;
    assert!(runtime.dispatch(cancel).await.success);
    assert!(pending.await.error.unwrap().contains("cancelled"));
    wait(&cancelled).await;
    let mut again = request(&node, "after-cancel", "delayed_probe", 2000);
    again.parameters.insert("fast".into(), true.into());
    assert!(
        runtime.dispatch(again).await.success,
        "Cancelled executor must return to the registry"
    );
    entered.store(false, Ordering::SeqCst);
    cancelled.store(false, Ordering::SeqCst);
    let timed = runtime
        .dispatch(request(&node, "timeout", "delayed_probe", 25))
        .await;
    assert!(!timed.success);
    assert!(timed.error.unwrap().contains("timed out"));
    wait(&cancelled).await;
    let mut again = request(&node, "after-timeout", "delayed_probe", 2000);
    again.parameters.insert("fast".into(), true.into());
    assert!(
        runtime.dispatch(again).await.success,
        "Timed-out executor must return to the registry"
    );
    assert!(runtime.pending.lock().unwrap().is_empty());
    assert_eq!(hooks.calls.load(Ordering::SeqCst), 0, "All direct device operations must be hook-free");
    assert_eq!(approvals.load(Ordering::SeqCst), 0, "All direct device operations must skip AI approval");
    crate::speech_transport_tests::verify(&mut core,&handler,&root,speech_http).await;
    assert_eq!(hooks.calls.load(Ordering::SeqCst), 0, "Direct tools must never dispatch AI lifecycle events, including failures and cancellation");
    assert_eq!(approvals.load(Ordering::SeqCst), 0, "Direct tools must never request AI approval");
    // Close the application before deleting its runtime files.
    operit_link::clearCoreRouteRuntime();
    drop(runtime);
    drop(core);
    drop(proxy);
    drop(handler);
    let _ = std::fs::remove_dir_all(root);
}
fn uuid_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
