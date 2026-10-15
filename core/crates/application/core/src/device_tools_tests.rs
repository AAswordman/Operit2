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
use std::sync::atomic::{AtomicBool, Ordering};

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

struct BlockProbe;
impl AIToolHook for BlockProbe {
    fn id(&self) -> &str {
        "device-probe-hook"
    }
    fn onToolCallIntercept(
        &self,
        tool: &operit_tools::ToolExecutionManager::AITool,
    ) -> AIToolHookDecision {
        if tool.name == "blocked_probe" {
            AIToolHookDecision::Block("Blocked by target device hook".into())
        } else {
            AIToolHookDecision::Allow
        }
    }
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
async fn device_tools_runtime_executes_permissions_hooks_binary_cancellation_and_timeout() {
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
            effect: ToolEffect::READ,
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
    handler.addToolHook(Arc::new(BlockProbe));
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
    let source_blocked = operit_plugin_sdk::javascript::JsExecutionHost::execute_tool_call(
        &handler,
        request(
            "unreachable-source-probe",
            "bound-blocked",
            "blocked_probe",
            2000,
        ),
    )
    .await;
    assert!(source_blocked
        .error
        .unwrap()
        .contains("Blocked by target device hook"));
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
    let denied = runtime
        .dispatch_with_preflight(
            request(&node, "source-denied", "binary_probe", 2000),
            Some(Box::pin(async { Err("Blocked by source hook".into()) })),
        )
        .await;
    assert!(denied.error.unwrap().contains("Blocked by source hook"));
    let source_waiting = Arc::new(AtomicBool::new(false));
    let source_dropped = Arc::new(AtomicBool::new(false));
    let started = source_waiting.clone();
    let dropped = source_dropped.clone();
    let preflight = Box::pin(async move {
        let _cancelled = Cancelled(dropped);
        started.store(true, Ordering::SeqCst);
        std::future::pending::<Result<(), String>>().await
    });
    let pending_source = runtime.dispatch_with_preflight(
        request(&node, "source-cancel", "binary_probe", 2000),
        Some(preflight),
    );
    wait(&source_waiting).await;
    let mut cancel_source = request(&node, "source-cancel", "binary_probe", 2000);
    cancel_source.device.as_mut().unwrap().action = JsDeviceToolAction::Cancel;
    assert!(runtime.dispatch(cancel_source).await.success);
    assert!(pending_source.await.error.unwrap().contains("cancelled"));
    wait(&source_dropped).await;
    let timed_source = runtime
        .dispatch_with_preflight(
            request(&node, "source-timeout", "binary_probe", 25),
            Some(Box::pin(std::future::pending())),
        )
        .await;
    assert!(timed_source.error.unwrap().contains("timed out"));
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
    assert!(!tools
        .iter()
        .any(|tool| tool["name"] == "disabled_device_probe:echo"));
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
    assert!(disabled.error.unwrap().contains("disabled"));
    assert!(tools.iter().all(|tool| tool["source"] == "package"
        || handler.getToolVisibility(tool["name"].as_str().unwrap())
            == Some(operit_tools::tools::AIToolHandler::ToolRegistrationVisibility::PUBLIC)));
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
    assert!(!hidden.success);
    assert!(hidden.error.unwrap().contains("not public"));
    let blocked = runtime
        .dispatch(request(&node, "blocked", "blocked_probe", 2000))
        .await;
    assert!(!blocked.success);
    assert!(blocked
        .error
        .unwrap()
        .contains("Blocked by target device hook"));

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
    crate::speech_transport_tests::verify(&mut core,&handler,&root,speech_http).await;
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
