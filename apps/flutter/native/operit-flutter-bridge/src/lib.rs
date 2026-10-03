#![allow(non_snake_case)]

mod BridgeCodec;
mod BridgeExports;
mod BridgeTransport;
mod FlutterHostAdapters;
mod PlatformRuntimeFactory;
#[cfg(not(target_arch = "wasm32"))]
mod RuntimeBootstrapStore;

pub use BridgeExports::*;
#[cfg(target_os = "android")]
pub(crate) use BridgeExports::{
    bridge_native_call, bridge_push_item, bridge_push_open, bridge_watch_snapshot,
    bridge_watch_stream, panic_payload_message,
};
#[cfg(not(target_arch = "wasm32"))]
use PlatformRuntimeFactory::create_local_core;
use PlatformRuntimeFactory::default_native_storage_roots;

use std::any::Any;
use std::collections::{hash_map::Entry, HashMap};
use std::ffi::{c_char, CStr, CString};
#[cfg(not(target_arch = "wasm32"))]
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use operit_core_application::CoreApplication;
use operit_proxy_local::LocalCoreProxy;

#[cfg(not(target_arch = "wasm32"))]
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::HostManager::HostManager;
#[cfg(not(target_arch = "wasm32"))]
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_host_api::RuntimeStorageHost;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CorePushItem, CorePushRequest,
    CoreWatchRequest, LinkDeviceInfo,
};
use operit_runtime::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestChatToolPermissionAsync, requestOwnerAudioPlay, requestOwnerBluetooth,
    requestOwnerBrowserAutomation, requestOwnerBrowserSession,
    requestOwnerComposeWebViewController, requestOwnerFileOpen, requestOwnerFileShare,
    requestOwnerLocalInference, requestOwnerMusicPlayback, requestOwnerSystemCaptureScreenshot,
    requestOwnerSystemOperation, requestOwnerSystemRecognizeText, requestOwnerTtsPlayback,
    requestOwnerTtsSynthesis, requestOwnerWebVisit, RuntimeHostInteractionAudioPlayPayload,
    RuntimeHostInteractionBluetoothPayload, RuntimeHostInteractionBrowserAutomationPayload,
    RuntimeHostInteractionBrowserSessionPayload,
    RuntimeHostInteractionComposeWebViewControllerPayload, RuntimeHostInteractionFileOpenPayload,
    RuntimeHostInteractionFileSharePayload, RuntimeHostInteractionLocalInferencePayload,
    RuntimeHostInteractionMusicPlaybackPayload, RuntimeHostInteractionSystemOperationPayload,
    RuntimeHostInteractionSystemRecognizeTextPayload, RuntimeHostInteractionToolPermissionTool,
    RuntimeHostInteractionToolPermissionToolParameter, RuntimeHostInteractionTtsPlaybackPayload,
    RuntimeHostInteractionTtsSynthesisPayload, RuntimeHostInteractionWebVisitHeader,
    RuntimeHostInteractionWebVisitPayload,
};
#[cfg(not(target_arch = "wasm32"))]
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_tools::ToolExecutionManager::AITool;

use BridgeCodec::{
    decode_native_call_request, decode_native_push_item, decode_native_push_open_request,
    decode_native_watch_snapshot_request, decode_native_watch_stream_request,
    native_result_error_vec, native_result_vec, native_watch_event_payload, native_watch_event_vec,
};
use BridgeTransport::NativePushState;
#[cfg(not(target_arch = "wasm32"))]
use BridgeTransport::NativeWatchChannel;
use FlutterHostAdapters::{
    FlutterBrowserAutomationBridge, FlutterBrowserSessionBridge, FlutterComposeDslWebViewBridge,
    FlutterWebVisitBridge,
};

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Reflect};
#[cfg(target_os = "android")]
use operit_host_android_native::{
    createRuntimeHostManager as create_platform_runtime_host_manager,
    AndroidAudioPlaybackHost as NativeAudioPlaybackHost,
    AndroidBluetoothHost as NativeBluetoothHost, AndroidFileSystemHost as NativeFileSystemHost,
    AndroidHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    AndroidHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    AndroidHttpHost as NativeHttpHost, AndroidManagedRuntimeHost as NativeManagedRuntimeHost,
    AndroidMusicCommand as NativeMusicCommand,
    AndroidRuntimeStorageHost as NativeRuntimeStorageHost,
    AndroidSystemOperationHost as NativeSystemOperationHost,
    AndroidTerminalHost as NativeTerminalHost, AndroidTtsPlaybackHost as NativeTtsPlaybackHost,
    AndroidTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
use operit_host_api::SystemOperationHost;
#[cfg(target_os = "ios")]
use operit_host_ios_native::{
    createRuntimeHostManager as create_platform_runtime_host_manager,
    IosAudioPlaybackHost as NativeAudioPlaybackHost, IosBluetoothHost as NativeBluetoothHost,
    IosFileSystemHost as NativeFileSystemHost,
    IosHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    IosHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    IosHttpHost as NativeHttpHost, IosLocalInferenceHost as NativeLocalInferenceHost,
    IosManagedRuntimeHost as NativeManagedRuntimeHost, IosMusicCommand as NativeMusicCommand,
    IosRuntimeStorageHost as NativeRuntimeStorageHost,
    IosSystemOperationHost as NativeSystemOperationHost, IosTerminalHost as NativeTerminalHost,
    IosTtsPlaybackHost as NativeTtsPlaybackHost, IosTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
use operit_host_linux_native::{
    createRuntimeHostManager as create_platform_runtime_host_manager,
    LinuxAudioPlaybackHost as NativeAudioPlaybackHost, LinuxBluetoothHost as NativeBluetoothHost,
    LinuxFileSystemHost as NativeFileSystemHost,
    LinuxHostRuntimeEventHost as NativeHostRuntimeEventHost,
    LinuxHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    LinuxHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    LinuxHttpHost as NativeHttpHost, LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxRuntimeStorageHost as NativeRuntimeStorageHost,
    LinuxSystemOperationHost as NativeSystemOperationHost, LinuxTerminalHost as NativeTerminalHost,
};
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
use operit_host_linux_native::{
    LinuxTtsPlaybackHost as NativeTtsPlaybackHost, LinuxTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(target_os = "macos")]
use operit_host_macos_native::{
    createRuntimeHostManager as create_platform_runtime_host_manager,
    MacosAudioPlaybackHost as NativeAudioPlaybackHost, MacosBluetoothHost as NativeBluetoothHost,
    MacosFileSystemHost as NativeFileSystemHost,
    MacosHostRuntimeEventHost as NativeHostRuntimeEventHost,
    MacosHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    MacosHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    MacosHttpHost as NativeHttpHost, MacosManagedRuntimeHost as NativeManagedRuntimeHost,
    MacosMusicCommand as NativeMusicCommand, MacosRuntimeStorageHost as NativeRuntimeStorageHost,
    MacosSystemOperationHost as NativeSystemOperationHost, MacosTerminalHost as NativeTerminalHost,
    MacosTtsPlaybackHost as NativeTtsPlaybackHost, MacosTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(target_env = "ohos")]
use operit_host_ohos_native::{
    createRuntimeHostManager as create_platform_runtime_host_manager,
    OhosAudioPlaybackHost as NativeAudioPlaybackHost, OhosBluetoothHost as NativeBluetoothHost,
    OhosFileSystemHost as NativeFileSystemHost,
    OhosHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    OhosHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    OhosHttpHost as NativeHttpHost, OhosLocalInferenceHost as NativeLocalInferenceHost,
    OhosManagedRuntimeHost as NativeManagedRuntimeHost, OhosMusicCommand as NativeMusicCommand,
    OhosRuntimeStorageHost as NativeRuntimeStorageHost,
    OhosSystemOperationHost as NativeSystemOperationHost, OhosTerminalHost as NativeTerminalHost,
    OhosTtsPlaybackHost as NativeTtsPlaybackHost,
};
#[cfg(target_arch = "wasm32")]
use operit_host_web::{createLocalCore as create_local_core, WebRuntimeStorageHost};
#[cfg(windows)]
use operit_host_windows_native::{
    createRuntimeHostManager as create_platform_runtime_host_manager,
    WindowsAudioPlaybackHost as NativeAudioPlaybackHost,
    WindowsBluetoothHost as NativeBluetoothHost, WindowsFileSystemHost as NativeFileSystemHost,
    WindowsHostRuntimeEventHost as NativeHostRuntimeEventHost,
    WindowsHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    WindowsHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    WindowsHttpHost as NativeHttpHost, WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
    WindowsRuntimeStorageHost as NativeRuntimeStorageHost,
    WindowsSystemOperationHost as NativeSystemOperationHost,
    WindowsTerminalHost as NativeTerminalHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsTtsPlaybackHost as NativeTtsPlaybackHost,
    WindowsTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

pub struct OperitFlutterBridge {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) runtime: tokio::runtime::Runtime,
    localCore: Arc<LocalCoreProxy>,
    #[cfg(not(target_arch = "wasm32"))]
    chatRuntimeHolder:
        Arc<tokio::sync::Mutex<operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder>>,
    runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) watchChannel: NativeWatchChannel,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) watchSubscriptions: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) watchSubscriptions: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
    pub(crate) pushStreams: Mutex<HashMap<String, NativePushState>>,
    coreApplication: Mutex<Option<CoreApplication>>,
    #[cfg(any(
        windows,
        all(target_os = "linux", not(target_env = "ohos")),
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_env = "ohos"
    ))]
    terminalHost: Arc<NativeTerminalHost>,
}

const PERMISSION_REQUEST_TIMEOUT_MS: u64 = 60_000;

impl OperitFlutterBridge {
    /// Runs one async runtime operation on the host scheduler and waits for its result.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn runHostRuntimeAsyncTask<T, F>(
        &self,
        taskName: &'static str,
        task: impl FnOnce() -> F + Send + 'static,
    ) -> Result<T, String>
    where
        T: Send + 'static,
        F: Future<Output = T> + 'static,
    {
        let (resultSender, resultReceiver) = mpsc::channel();
        HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(
            defaultHostRuntimeTaskSchedulerHost().as_ref(),
            taskName,
            Box::new(move || {
                Box::pin(async move {
                    let result = task().await;
                    let _ = resultSender.send(result);
                })
            }),
        )
        .map_err(|error| error.to_string())?;
        resultReceiver
            .recv()
            .map_err(|error| format!("runtime task result channel closed: {error}"))
    }

    /// Creates a bridge using the platform's explicit runtime and workspace roots.
    #[cfg(not(target_env = "ohos"))]
    fn new() -> Result<Self, String> {
        let (runtime_root, workspace_root) = default_native_storage_roots()?;
        Self::new_with_storage_roots(runtime_root, workspace_root)
    }

    /// Creates a bridge using caller-supplied runtime and workspace roots.
    fn new_with_storage_roots(
        runtime_root: PathBuf,
        workspace_root: PathBuf,
        #[cfg(target_env = "ohos")] systemLanguageCode: String,
    ) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let runtime = {
            let mut runtimeBuilder = tokio::runtime::Builder::new_multi_thread();
            runtimeBuilder
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
        };
        let browserAutomationBridge = FlutterBrowserAutomationBridge::new();
        let browserSessionBridge = FlutterBrowserSessionBridge::new();
        let webVisitBridge = FlutterWebVisitBridge::new();
        let composeDslWebViewBridge = FlutterComposeDslWebViewBridge::new();
        #[cfg(not(target_env = "ohos"))]
        #[cfg(any(
            windows,
            all(target_os = "linux", not(target_env = "ohos")),
            target_os = "android",
            target_os = "ios",
            target_os = "macos"
        ))]
        let terminalHost = Arc::new(NativeTerminalHost::new());
        #[cfg(target_env = "ohos")]
        let terminalHost = Arc::new(
            NativeTerminalHost::new(runtime_root.clone(), workspace_root.clone())
                .map_err(|error| error.message)?,
        );
        let mut core = create_local_core(
            runtime_root,
            workspace_root,
            #[cfg(target_env = "ohos")]
            systemLanguageCode,
            Arc::new(webVisitBridge),
            Some(Arc::new(browserAutomationBridge)),
            Some(Arc::new(browserSessionBridge)),
            Some(Arc::new(composeDslWebViewBridge)),
            #[cfg(any(
                windows,
                all(target_os = "linux", not(target_env = "ohos")),
                target_os = "android",
                target_os = "ios",
                target_os = "macos",
                target_env = "ohos"
            ))]
            terminalHost.clone(),
        )?;
        let coreInitializationStartedAt = operit_host_api::TimeUtils::currentTimeMillis();
        core.localApplicationMut().onCreate()?;
        operit_util::AppLogger::AppLogger::i(
            "OperitFlutterBridge",
            &format!(
                "core onCreate done elapsedMs={}",
                operit_host_api::TimeUtils::currentTimeMillis() - coreInitializationStartedAt
            ),
        );
        let coreApplicationStartedAt = operit_host_api::TimeUtils::currentTimeMillis();
        install_permission_requester(&mut core);
        #[cfg(not(target_arch = "wasm32"))]
        let chatRuntimeHolder = core.localApplicationMut().chatRuntimeHolder.clone();
        let runtimeStorageHost = core.runtimeStorageHost();
        let localCore = Arc::new(core);
        let coreApplication = CoreApplication::startWithSharedLocalClient(
            localCore.clone(),
            LinkDeviceInfo::native(),
        )?;
        operit_util::AppLogger::AppLogger::i(
            "OperitFlutterBridge",
            &format!(
                "CoreApplication start done elapsedMs={}",
                operit_host_api::TimeUtils::currentTimeMillis() - coreApplicationStartedAt
            ),
        );
        Ok(Self {
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
            localCore,
            #[cfg(not(target_arch = "wasm32"))]
            chatRuntimeHolder,
            runtimeStorageHost,
            #[cfg(not(target_arch = "wasm32"))]
            watchChannel: NativeWatchChannel::new(),
            #[cfg(not(target_arch = "wasm32"))]
            watchSubscriptions: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(target_arch = "wasm32")]
            watchSubscriptions: Arc::new(Mutex::new(HashMap::new())),
            pushStreams: Mutex::new(HashMap::new()),
            coreApplication: Mutex::new(Some(coreApplication)),
            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(any(
                windows,
                all(target_os = "linux", not(target_env = "ohos")),
                target_os = "android",
                target_os = "ios",
                target_os = "macos",
                target_env = "ohos"
            ))]
            terminalHost,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Calls the local Core runtime without entering the server-side node router.
    fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let requestIdForPanic = request.requestId.clone();
        let localCore = self.localCore.clone();
        match self.runHostRuntimeAsyncTask("operit-flutter-call", move || async move {
            // A panic inside a Core call kills the dedicated scheduler thread and
            // drops the result sender unevaluated, which surfaces to Dart as a
            // bare "runtime task result channel closed" — on Android the panic
            // message itself never reaches logcat (stderr is discarded), making
            // the failure undiagnosable. Catch it at this layer (the only layer
            // that knows the concrete response type) and deliver it as a
            // structured error response instead.
            let callFuture =
                std::panic::AssertUnwindSafe(CoreLinkSharedClient::call(localCore.as_ref(), request));
            match futures_util::FutureExt::catch_unwind(callFuture).await {
                Ok(response) => response,
                Err(payload) => {
                    let detail = payload
                        .downcast_ref::<&str>()
                        .map(|message| message.to_string())
                        .or_else(|| payload.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "non-string panic payload".to_string());
                    CoreCallResponse::err(
                        requestIdForPanic,
                        CoreLinkError::internal(format!(
                            "runtime task panicked: {detail}"
                        )),
                    )
                }
            }
        }) {
            Ok(response) => response,
            Err(error) => CoreCallResponse::err(requestId, CoreLinkError::internal(error)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self.localCore.as_ref(), request).await
    }



    #[cfg(not(target_arch = "wasm32"))]
    fn emitRuntimeEvent(&self, eventJson: &str) -> String {
        let eventValue: serde_json::Value = match serde_json::from_str(eventJson) {
            Ok(value) => value,
            Err(error) => {
                return serde_json::json!({
                    "ok": false,
                    "error": format!("runtime event is invalid JSON: {error}"),
                })
                .to_string();
            }
        };
        let response = self.call(CoreCallRequest::new(
            format!("runtime-event-{}", current_time_millis_u64()),
            LocalCoreProxy::generatedTargetForSchema("application")
                .expect("generated application object id must exist"),
            "ingestRuntimeEvent",
            operit_link::toCoreValue(serde_json::json!({
                "event": eventValue,
            }))
            .expect("runtime event arguments must convert to CoreValue"),
        ));
        match response.result {
            Ok(value) => serde_json::json!({"ok": true, "result": value}).to_string(),
            Err(error) => serde_json::json!({"ok": false, "error": error.message}).to_string(),
        }
    }
}

/// Installs the asynchronous controller permission requester for every runtime.
fn install_permission_requester(core: &mut LocalCoreProxy) {
    let handler = core.localApplicationMut().toolHandler.clone();
    handler
        .getToolPermissionSystem()
        .setAsyncPermissionRequester(move |tool, description, chatId| async move {
            let Some(chatId) = chatId else {
                return PermissionRequestResult::DENY;
            };
            let response = requestChatToolPermissionAsync(
                chatId,
                tool_to_permission_payload(&tool),
                description,
                Duration::from_millis(PERMISSION_REQUEST_TIMEOUT_MS),
            )
            .await;
            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    eprintln!("tool permission request failed: {error}");
                    return PermissionRequestResult::DENY;
                }
            };
            match response.as_str() {
                "allow" => PermissionRequestResult::ALLOW,
                "allow_session" => PermissionRequestResult::ALLOW_SESSION,
                "deny" => PermissionRequestResult::DENY,
                other => {
                    eprintln!("unknown tool permission response result: {other}");
                    PermissionRequestResult::DENY
                }
            }
        });
}

fn tool_to_permission_payload(tool: &AITool) -> RuntimeHostInteractionToolPermissionTool {
    RuntimeHostInteractionToolPermissionTool {
        name: tool.name.clone(),
        parameters: tool
            .parameters
            .iter()
            .map(
                |parameter| RuntimeHostInteractionToolPermissionToolParameter {
                    name: parameter.name.clone(),
                    value: parameter.value.clone(),
                },
            )
            .collect(),
    }
}

fn current_time_millis_u64() -> u64 {
    operit_host_api::TimeUtils::currentTimeMillisU128().min(u64::MAX as u128) as u64
}

fn last_create_error() -> &'static Mutex<String> {
    static LAST_CREATE_ERROR: OnceLock<Mutex<String>> = OnceLock::new();
    LAST_CREATE_ERROR.get_or_init(|| Mutex::new(String::new()))
}

fn set_last_create_error(value: String) {
    *last_create_error()
        .lock()
        .expect("create error lock must not be poisoned") = value;
}

#[cfg(target_os = "android")]
mod AndroidJni;
