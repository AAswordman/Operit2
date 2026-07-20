#![allow(non_snake_case)]

use std::any::Any;
use std::collections::{hash_map::Entry, BTreeMap, HashMap};
use std::ffi::{c_char, CStr, CString};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use operit_core_proxy::LocalCoreProxy;
#[cfg(not(target_arch = "wasm32"))]
mod mdnss;
#[cfg(all(not(target_arch = "wasm32"), feature = "process-stdio"))]
pub mod process_stdio;

#[cfg(not(target_arch = "wasm32"))]
use operit_link_access::{
    link_token_hash, LinkAccessHostConfig, LinkAccessStore, PairStartState, RemoteDeviceInfo,
    RemoteLinkClient, RemoteLinkServer, RemoteLinkServerConfig, RemoteWebAccessConfig,
};
use operit_host_api::HostManager::HostManager;
use operit_host_api::RuntimeStorageHost;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkSharedClient, CorePushItem, CorePushRequest, CoreWatchRequest,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerAudioPlay, requestOwnerBluetooth, requestOwnerBrowserAutomation,
    requestOwnerBrowserSession, requestOwnerComposeWebViewController, requestOwnerFileOpen,
    requestOwnerFileShare, requestOwnerLocalInference, requestOwnerMusicPlayback,
    requestOwnerSystemCaptureScreenshot, requestOwnerSystemOperation,
    requestOwnerSystemRecognizeText, requestOwnerToolPermissionAsync, requestOwnerTtsPlayback,
    requestOwnerTtsSynthesis, requestOwnerWebVisit, RuntimeHostInteractionAudioPlayPayload,
    RuntimeHostInteractionBluetoothPayload, RuntimeHostInteractionBrowserAutomationPayload,
    RuntimeHostInteractionBrowserSessionPayload,
    RuntimeHostInteractionComposeWebViewControllerPayload, RuntimeHostInteractionFileOpenPayload,
    RuntimeHostInteractionFileSharePayload, RuntimeHostInteractionLocalInferencePayload,
    RuntimeHostInteractionMusicPlaybackPayload, RuntimeHostInteractionSystemOperationPayload,
    RuntimeHostInteractionSystemRecognizeTextPayload, RuntimeHostInteractionToolPermissionPayload,
    RuntimeHostInteractionToolPermissionTool, RuntimeHostInteractionToolPermissionToolParameter,
    RuntimeHostInteractionTtsPlaybackPayload, RuntimeHostInteractionTtsSynthesisPayload,
    RuntimeHostInteractionWebVisitHeader, RuntimeHostInteractionWebVisitPayload,
};
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_tools::ToolExecutionManager::AITool;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Reflect};
#[cfg(target_os = "android")]
use operit_host_android_native::{
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
#[cfg(target_os = "android")]
use operit_host_api::SystemOperationHost;
#[cfg(target_os = "ios")]
use operit_host_ios_native::{
    IosAudioPlaybackHost as NativeAudioPlaybackHost,
    IosBluetoothHost as NativeBluetoothHost, IosFileSystemHost as NativeFileSystemHost,
    IosHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    IosHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    IosHttpHost as NativeHttpHost, IosLocalInferenceHost as NativeLocalInferenceHost,
    IosManagedRuntimeHost as NativeManagedRuntimeHost, IosMusicCommand as NativeMusicCommand,
    IosRuntimeStorageHost as NativeRuntimeStorageHost,
    IosSystemOperationHost as NativeSystemOperationHost,
    IosTerminalHost as NativeTerminalHost,
    IosTtsPlaybackHost as NativeTtsPlaybackHost,
    IosTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(target_os = "macos")]
use operit_host_macos_native::{
    MacosAudioPlaybackHost as NativeAudioPlaybackHost,
    MacosBluetoothHost as NativeBluetoothHost, MacosFileSystemHost as NativeFileSystemHost,
    MacosHostRuntimeEventHost as NativeHostRuntimeEventHost,
    MacosHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    MacosHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    MacosHttpHost as NativeHttpHost, MacosManagedRuntimeHost as NativeManagedRuntimeHost,
    MacosMusicCommand as NativeMusicCommand, MacosRuntimeStorageHost as NativeRuntimeStorageHost,
    MacosSystemOperationHost as NativeSystemOperationHost, MacosTerminalHost as NativeTerminalHost,
    MacosTtsPlaybackHost as NativeTtsPlaybackHost,
    MacosTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
use operit_host_linux_native::{
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
#[cfg(target_env = "ohos")]
use operit_host_ohos_native::{
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
use operit_host_web::{
    WebAudioPlaybackHost as NativeAudioPlaybackHost, WebBluetoothHost as NativeBluetoothHost,
    WebBrowserSessionHost as NativeBrowserSessionHost, WebFileSystemHost as NativeFileSystemHost,
    WebHostRuntimeEventHost as NativeHostRuntimeEventHost,
    WebHostRuntimeEventSchedulerHost as NativeHostRuntimeEventSchedulerHost,
    WebHostRuntimeTaskSchedulerHost as NativeHostRuntimeTaskSchedulerHost,
    WebHttpHost as NativeHttpHost, WebLocalInferenceHost as NativeLocalInferenceHost,
    WebManagedRuntimeHost as NativeManagedRuntimeHost,
    WebRuntimeStorageHost as NativeRuntimeStorageHost,
    WebSystemOperationHost as NativeSystemOperationHost, WebTerminalHost as NativeTerminalHost,
    WebTtsPlaybackHost as NativeTtsPlaybackHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
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
    runtime: tokio::runtime::Runtime,
    proxyCore: Arc<LocalCoreProxy>,
    runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    #[cfg(not(target_arch = "wasm32"))]
    watchChannel: NativeWatchChannel,
    #[cfg(not(target_arch = "wasm32"))]
    watchSubscriptions: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    #[cfg(target_arch = "wasm32")]
    watchSubscriptions: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pushStreams: Mutex<HashMap<String, NativePushState>>,
    #[cfg(not(target_arch = "wasm32"))]
    webAccessTask: Mutex<Option<tokio::task::JoinHandle<Result<(), String>>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pendingRemotePairings: Mutex<HashMap<String, PendingRemotePairing>>,
    #[cfg(not(target_arch = "wasm32"))]
    mdns: Mutex<Option<mdnss::MdnsHandle>>,
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

struct NativePushState {
    request: CorePushRequest,
    nextSequence: u64,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct NativeWatchChannel {
    sender: mpsc::Sender<NativeWatchChannelMessage>,
    receiver: Arc<Mutex<mpsc::Receiver<NativeWatchChannelMessage>>>,
    closed: Arc<AtomicBool>,
}

#[cfg(not(target_arch = "wasm32"))]
enum NativeWatchChannelMessage {
    Event(Vec<u8>),
    Closed,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeWatchChannel {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn send(&self, frame: Vec<u8>) {
        if !self.closed.load(Ordering::SeqCst) {
            let _ = self.sender.send(NativeWatchChannelMessage::Event(frame));
        }
    }

    fn close(&self) {
        if !self.closed.swap(true, Ordering::SeqCst) {
            let _ = self.sender.send(NativeWatchChannelMessage::Closed);
        }
    }

    fn nextEvent(&self) -> Result<Vec<u8>, CoreLinkError> {
        let receiver = self.receiver.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch channel lock poisoned: {error}"))
        })?;
        match receiver.recv() {
            Ok(NativeWatchChannelMessage::Event(frame)) => Ok(frame),
            Ok(NativeWatchChannelMessage::Closed) | Err(_) => Err(CoreLinkError::new(
                "WATCH_CHANNEL_CLOSED",
                "watch channel closed",
            )),
        }
    }
}

impl Drop for OperitFlutterBridge {
    fn drop(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.watchChannel.close();
            if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
                for (_, task) in subscriptions.drain() {
                    task.abort();
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct PendingRemotePairing {
    client: RemoteLinkClient,
    state: PairStartState,
}

const PERMISSION_REQUEST_TIMEOUT_MS: u64 = 60_000;

#[derive(Clone)]
struct FlutterBrowserAutomationBridge {}

impl FlutterBrowserAutomationBridge {
    fn new() -> Self {
        Self {}
    }
}

impl operit_host_api::BrowserAutomationHost for FlutterBrowserAutomationBridge {
    fn executeBrowserTool(
        &self,
        request: operit_host_api::BrowserAutomationRequest,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserAutomationResponse> {
        let requestId = request.requestId.clone();
        let pending = RuntimeHostInteractionBrowserAutomationPayload {
            requestId: request.requestId,
            toolName: request.toolName,
            parametersJson: request.parametersJson,
            requestedAtMillis: current_time_millis_u64(),
        };
        let response = requestOwnerBrowserAutomation(pending, Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        if response.requestId != requestId {
            return Err(operit_host_api::HostError::new(format!(
                "browser automation response requestId mismatch: {} != {requestId}",
                response.requestId
            )));
        }
        if response.success {
            return Ok(operit_host_api::BrowserAutomationResponse {
                output: response.result,
            });
        }
        let Some(error) = response.error else {
            return Err(operit_host_api::HostError::new(
                "browser automation error is missing",
            ));
        };
        Err(operit_host_api::HostError::new(error))
    }
}

#[derive(Clone)]
struct FlutterBrowserSessionBridge {}

impl FlutterBrowserSessionBridge {
    /// Creates a browser session bridge that delegates to the owner app.
    fn new() -> Self {
        Self {}
    }

    /// Sends one browser session command to the owner app.
    fn requestCommand(
        &self,
        command: operit_host_api::BrowserSessionCommand,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionCommandResult> {
        let commandJson = serde_json::to_string(&command).map_err(|error| {
            operit_host_api::HostError::new(format!(
                "browser session command encode failed: {error}"
            ))
        })?;
        let response = requestOwnerBrowserSession(
            RuntimeHostInteractionBrowserSessionPayload { commandJson },
            Duration::from_secs(60),
        )
        .map_err(operit_host_api::HostError::new)?;
        serde_json::from_str(&response.resultJson).map_err(|error| {
            operit_host_api::HostError::new(format!(
                "browser session response decode failed: {error}"
            ))
        })
    }

    /// Builds a browser session command envelope.
    fn command(action: &str) -> operit_host_api::BrowserSessionCommand {
        operit_host_api::BrowserSessionCommand {
            action: action.to_string(),
            sessionId: None,
            url: None,
            script: None,
            payloadJson: String::new(),
            userAgent: None,
            headers: BTreeMap::new(),
        }
    }

    /// Requires the command result to include a browser session.
    fn requireSession(
        result: operit_host_api::BrowserSessionCommandResult,
        operation: &str,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionInfo> {
        result.session.ok_or_else(|| {
            operit_host_api::HostError::new(format!(
                "browser session {operation} result is missing session"
            ))
        })
    }
}

impl operit_host_api::BrowserSessionHost for FlutterBrowserSessionBridge {
    /// Lists interactive browser sessions owned by the Flutter app.
    fn listBrowserSessions(
        &self,
    ) -> operit_host_api::HostResult<Vec<operit_host_api::BrowserSessionInfo>> {
        let result = self.requestCommand(Self::command("list"))?;
        Ok(result.sessions)
    }

    /// Creates an interactive browser session in the Flutter app.
    fn createBrowserSession(
        &self,
        initialUrl: &str,
        userAgent: Option<&str>,
        headers: BTreeMap<String, String>,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionInfo> {
        let mut command = Self::command("create");
        command.url = Some(initialUrl.to_string());
        command.userAgent = userAgent.map(str::to_string);
        command.headers = headers;
        Self::requireSession(self.requestCommand(command)?, "create")
    }

    /// Updates a browser session owned by the Flutter app.
    fn updateBrowserSession(
        &self,
        sessionId: &str,
        userAgent: Option<&str>,
        headers: BTreeMap<String, String>,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionInfo> {
        let mut command = Self::command("update");
        command.sessionId = Some(sessionId.to_string());
        command.userAgent = userAgent.map(str::to_string);
        command.headers = headers;
        Self::requireSession(self.requestCommand(command)?, "update")
    }

    /// Submits a semantic browser command to the Flutter app.
    fn submitBrowserCommand(
        &self,
        command: operit_host_api::BrowserSessionCommand,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionCommandResult> {
        self.requestCommand(command)
    }

    /// Reads a browser session snapshot from the Flutter app.
    fn getBrowserSessionSnapshot(
        &self,
        sessionId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionSnapshot> {
        let mut command = Self::command("snapshot");
        command.sessionId = Some(sessionId.to_string());
        let result = self.requestCommand(command)?;
        let session = Self::requireSession(result.clone(), "snapshot")?;
        Ok(operit_host_api::BrowserSessionSnapshot {
            session,
            resultJson: result.resultJson,
        })
    }

    /// Closes a browser session owned by the Flutter app.
    fn closeBrowserSession(
        &self,
        sessionId: &str,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserSessionCommandResult> {
        let mut command = Self::command("close");
        command.sessionId = Some(sessionId.to_string());
        self.requestCommand(command)
    }
}

#[derive(Clone)]
struct FlutterWebVisitBridge {}

impl FlutterWebVisitBridge {
    fn new() -> Self {
        Self {}
    }
}

impl operit_host_api::WebVisitHost for FlutterWebVisitBridge {
    fn visitWeb(
        &self,
        request: operit_host_api::WebVisitRequest,
    ) -> operit_host_api::HostResult<operit_host_api::WebVisitResult> {
        static NEXT_WEB_VISIT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);
        let requestId = format!(
            "web-visit-{}-{}",
            current_time_millis_u64(),
            NEXT_WEB_VISIT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
        );
        let pending = RuntimeHostInteractionWebVisitPayload {
            requestId: requestId.clone(),
            url: request.url,
            headers: request
                .headers
                .into_iter()
                .map(|(name, value)| RuntimeHostInteractionWebVisitHeader { name, value })
                .collect(),
            userAgent: request.userAgent,
            includeImageLinks: request.includeImageLinks,
            requestedAtMillis: current_time_millis_u64(),
        };
        let response = requestOwnerWebVisit(pending, Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        if response.requestId != requestId {
            return Err(operit_host_api::HostError::new(format!(
                "web visit response requestId mismatch: {} != {requestId}",
                response.requestId
            )));
        }
        if response.success {
            let Some(result) = response.result else {
                return Err(operit_host_api::HostError::new(
                    "web visit result is missing",
                ));
            };
            return Ok(operit_host_api::WebVisitResult {
                url: result.url,
                title: result.title,
                content: result.content,
                metadata: result
                    .metadata
                    .into_iter()
                    .map(|entry| (entry.name, entry.value))
                    .collect(),
                links: result
                    .links
                    .into_iter()
                    .map(|link| operit_host_api::WebVisitLinkData {
                        url: link.url,
                        text: link.text,
                    })
                    .collect(),
                imageLinks: result.imageLinks,
            });
        }
        let Some(error) = response.error else {
            return Err(operit_host_api::HostError::new(
                "web visit error is missing",
            ));
        };
        Err(operit_host_api::HostError::new(error))
    }
}

#[derive(Clone)]
struct FlutterComposeDslWebViewBridge {}

impl FlutterComposeDslWebViewBridge {
    fn new() -> Self {
        Self {}
    }
}

impl operit_host_api::ComposeDslWebViewHost for FlutterComposeDslWebViewBridge {
    fn handleControllerCommand(&self, payloadJson: &str) -> operit_host_api::HostResult<String> {
        let response = requestOwnerComposeWebViewController(
            RuntimeHostInteractionComposeWebViewControllerPayload {
                commandJson: payloadJson.to_string(),
            },
            Duration::from_secs(60),
        )
        .map_err(operit_host_api::HostError::new)?;
        Ok(response.result)
    }
}

#[cfg(target_os = "android")]
#[derive(Clone)]
struct FlutterSystemOperationBridge {
    native: NativeSystemOperationHost,
}

#[cfg(target_os = "android")]
impl FlutterSystemOperationBridge {
    fn new() -> Self {
        Self {
            native: NativeSystemOperationHost::new(),
        }
    }
}

#[cfg(target_os = "android")]
impl operit_host_api::SystemOperationHost for FlutterSystemOperationBridge {
    fn getSystemLanguageCode(&self) -> operit_host_api::HostResult<String> {
        self.native.getSystemLanguageCode()
    }

    fn toast(&self, message: &str) -> operit_host_api::HostResult<()> {
        self.native.toast(message)
    }

    fn sendNotification(&self, title: &str, message: &str) -> operit_host_api::HostResult<()> {
        self.native.sendNotification(title, message)
    }

    fn modifySystemSetting(
        &self,
        namespace: &str,
        setting: &str,
        value: &str,
    ) -> operit_host_api::HostResult<operit_host_api::SystemSettingData> {
        self.native.modifySystemSetting(namespace, setting, value)
    }

    fn getSystemSetting(
        &self,
        namespace: &str,
        setting: &str,
    ) -> operit_host_api::HostResult<operit_host_api::SystemSettingData> {
        self.native.getSystemSetting(namespace, setting)
    }

    fn installApp(
        &self,
        path: &str,
    ) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        self.native.installApp(path)
    }

    fn uninstallApp(
        &self,
        packageName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        self.native.uninstallApp(packageName)
    }

    fn listInstalledApps(
        &self,
        includeSystemApps: bool,
    ) -> operit_host_api::HostResult<operit_host_api::AppListData> {
        self.native.listInstalledApps(includeSystemApps)
    }

    fn startApp(
        &self,
        packageName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        self.native.startApp(packageName)
    }

    fn stopApp(
        &self,
        packageName: &str,
    ) -> operit_host_api::HostResult<operit_host_api::AppOperationData> {
        self.native.stopApp(packageName)
    }

    fn getNotifications(
        &self,
        limit: i32,
        includeOngoing: bool,
    ) -> operit_host_api::HostResult<operit_host_api::NotificationData> {
        self.native.getNotifications(limit, includeOngoing)
    }

    fn getAppUsageTime(
        &self,
        packageName: &str,
        sinceHours: i32,
        limit: i32,
        includeSystemApps: bool,
    ) -> operit_host_api::HostResult<operit_host_api::AppUsageTimeResultData> {
        self.native
            .getAppUsageTime(packageName, sinceHours, limit, includeSystemApps)
    }

    fn getDeviceLocation(
        &self,
        timeout: i32,
        highAccuracy: bool,
        includeAddress: bool,
    ) -> operit_host_api::HostResult<operit_host_api::LocationData> {
        self.native
            .getDeviceLocation(timeout, highAccuracy, includeAddress)
    }

    fn getDeviceInfo(&self) -> operit_host_api::HostResult<operit_host_api::DeviceInfoData> {
        self.native.getDeviceInfo()
    }

    fn captureScreenshot(&self) -> operit_host_api::HostResult<String> {
        let response = requestOwnerSystemCaptureScreenshot(Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        Ok(response.path)
    }

    fn recognizeText(
        &self,
        imagePath: &str,
        language: operit_host_api::OCRLanguage,
        quality: operit_host_api::OCRQuality,
    ) -> operit_host_api::HostResult<String> {
        let request = RuntimeHostInteractionSystemRecognizeTextPayload {
            imagePath: imagePath.to_string(),
            language: language.asHostValue().to_string(),
            quality: quality.asHostValue().to_string(),
        };
        let response = requestOwnerSystemRecognizeText(request, Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        Ok(response.text)
    }
}

impl OperitFlutterBridge {
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
        #[cfg(any(
            windows,
            all(target_os = "linux", not(target_env = "ohos")),
            target_os = "android",
            target_os = "ios",
            target_os = "macos",
            target_env = "ohos"
        ))]
        let terminalHost = Arc::new(NativeTerminalHost::new());
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
        core.localApplicationMut().onCreate()?;
        install_permission_requester(&mut core);
        let runtimeStorageHost = core.runtimeStorageHost();
        Ok(Self {
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
            proxyCore: Arc::new(core),
            runtimeStorageHost,
            #[cfg(not(target_arch = "wasm32"))]
            watchChannel: NativeWatchChannel::new(),
            #[cfg(not(target_arch = "wasm32"))]
            watchSubscriptions: Mutex::new(HashMap::new()),
            #[cfg(target_arch = "wasm32")]
            watchSubscriptions: Mutex::new(HashMap::new()),
            pushStreams: Mutex::new(HashMap::new()),
            #[cfg(not(target_arch = "wasm32"))]
            webAccessTask: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            pendingRemotePairings: Mutex::new(HashMap::new()),
            #[cfg(not(target_arch = "wasm32"))]
            mdns: Mutex::new(None),
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
    fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.runtime
            .block_on(CoreLinkSharedClient::call(self.proxyCore.as_ref(), request))
    }

    #[cfg(target_arch = "wasm32")]
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self.proxyCore.as_ref(), request).await
    }

    /// Opens one client-owned input stream on the local runtime carrier.
    fn pushOpen(&self, request: CorePushRequest) -> Result<String, CoreLinkError> {
        let pushId = request.requestId.0.clone();
        let mut pushes = self.pushStreams.lock().map_err(|error| {
            CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
        })?;
        if pushes
            .insert(
                pushId.clone(),
                NativePushState {
                    request,
                    nextSequence: 0,
                },
            )
            .is_some()
        {
            return Err(CoreLinkError::new(
                "PUSH_ALREADY_EXISTS",
                "Link push stream already exists",
            ));
        }
        Ok(pushId)
    }

    /// Dispatches one native push item in stream order.
    #[cfg(not(target_arch = "wasm32"))]
    fn pushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        let request = self.takePushItemRequest(&item)?;
        let response = self.call(request.itemCall(item.sequence, item.args));
        response.result?;
        Ok(())
    }

    /// Dispatches one wasm push item in stream order.
    #[cfg(target_arch = "wasm32")]
    async fn pushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        let request = self.takePushItemRequest(&item)?;
        let response = self.call(request.itemCall(item.sequence, item.args)).await;
        response.result?;
        Ok(())
    }

    /// Closes one client-owned input stream.
    fn pushClose(&self, pushId: &str) -> Result<(), CoreLinkError> {
        let removed = self
            .pushStreams
            .lock()
            .map_err(|error| {
                CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
            })?
            .remove(pushId);
        if removed.is_none() {
            return Err(CoreLinkError::new(
                "PUSH_NOT_FOUND",
                "Link push stream not found",
            ));
        }
        Ok(())
    }

    /// Validates one item sequence and returns its registered target.
    fn takePushItemRequest(&self, item: &CorePushItem) -> Result<CorePushRequest, CoreLinkError> {
        let mut pushes = self.pushStreams.lock().map_err(|error| {
            CoreLinkError::internal(format!("push stream lock poisoned: {error}"))
        })?;
        let state = pushes
            .get_mut(&item.pushId)
            .ok_or_else(|| CoreLinkError::new("PUSH_NOT_FOUND", "Link push stream not found"))?;
        if item.sequence != state.nextSequence {
            return Err(CoreLinkError::new(
                "PUSH_SEQUENCE_MISMATCH",
                format!(
                    "Link push sequence is {}, expected {}",
                    item.sequence, state.nextSequence
                ),
            ));
        }
        state.nextSequence += 1;
        Ok(state.request.clone())
    }

    #[allow(non_snake_case)]
    fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        #[cfg(target_arch = "wasm32")]
        {
            return self.proxyCore.watchSnapshotSync(request);
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.runtime.block_on(CoreLinkSharedClient::watchSnapshot(
            self.proxyCore.as_ref(),
            request,
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn watchStream(
        &self,
        subscriptionId: String,
        request: CoreWatchRequest,
    ) -> Result<String, CoreLinkError> {
        {
            let subscriptions = self.watchSubscriptions.lock().map_err(|error| {
                CoreLinkError::internal(format!("watch subscription lock poisoned: {error}"))
            })?;
            if subscriptions.contains_key(&subscriptionId) {
                return Err(CoreLinkError::new(
                    "WATCH_ALREADY_EXISTS",
                    "watch subscription already exists",
                ));
            }
        }
        let receiver = self.runtime.block_on(CoreLinkSharedClient::watch(
            self.proxyCore.as_ref(),
            request,
        ))?;
        let channel = self.watchChannel.clone();
        let taskSubscriptionId = subscriptionId.clone();
        let task = self.runtime.spawn(async move {
            let mut receiver = receiver;
            while let Some(event) = receiver.recv().await {
                let completed = event.kind == CoreEventKind::Completed;
                channel.send(native_watch_event_vec(&taskSubscriptionId, event));
                if completed {
                    break;
                }
            }
        });
        let mut subscriptions = self.watchSubscriptions.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch subscription lock poisoned: {error}"))
        })?;
        match subscriptions.entry(subscriptionId.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(task);
                Ok(subscriptionId)
            }
            Entry::Occupied(_) => {
                task.abort();
                Err(CoreLinkError::new(
                    "WATCH_ALREADY_EXISTS",
                    "watch subscription already exists",
                ))
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn watchStream(
        &self,
        subscriptionId: String,
        request: CoreWatchRequest,
        onEvent: Function,
    ) -> Result<String, CoreLinkError> {
        let mut subscriptions = self.watchSubscriptions.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch subscription lock poisoned: {error}"))
        })?;
        match subscriptions.entry(subscriptionId.clone()) {
            Entry::Vacant(entry) => {
                let cancelled = Arc::new(AtomicBool::new(false));
                entry.insert(cancelled.clone());
                drop(subscriptions);
                let receiver = match self.proxyCore.watchSync(request) {
                    Ok(receiver) => receiver,
                    Err(error) => {
                        if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
                            subscriptions.remove(&subscriptionId);
                        }
                        return Err(error);
                    }
                };
                let taskSubscriptionId = subscriptionId.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut receiver = receiver;
                    while !cancelled.load(Ordering::SeqCst) {
                        let Some(event) = receiver.recv().await else {
                            break;
                        };
                        let completed = event.kind == CoreEventKind::Completed;
                        let frame = native_watch_event_vec(&taskSubscriptionId, event);
                        let frame = js_sys::Uint8Array::from(frame.as_slice());
                        let _ = onEvent.call1(&JsValue::NULL, &frame.into());
                        if completed {
                            break;
                        }
                    }
                });
                Ok(subscriptionId)
            }
            Entry::Occupied(_) => Err(CoreLinkError::new(
                "WATCH_ALREADY_EXISTS",
                "watch subscription already exists",
            )),
        }
    }

    fn closeWatchStream(&self, subscriptionId: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
            if let Some(task) = subscriptions.remove(subscriptionId) {
                task.abort();
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Ok(mut subscriptions) = self.watchSubscriptions.lock() {
            if let Some(cancelled) = subscriptions.remove(subscriptionId) {
                cancelled.store(true, Ordering::SeqCst);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn nextWatchChannelEvent(&self) -> Result<Vec<u8>, CoreLinkError> {
        self.watchChannel.nextEvent()
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
            "application",
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

    #[cfg(not(target_arch = "wasm32"))]
    fn remotePairStart(
        &self,
        baseUrl: String,
        tokenHash: String,
        clientDeviceInfo: RemoteDeviceInfo,
    ) -> Result<String, String> {
        let client = RemoteLinkClient::new(baseUrl);
        let hello = self.runtime.block_on(client.hello(&tokenHash))?;
        let state = self
            .runtime
            .block_on(client.pairStart(&tokenHash, clientDeviceInfo))?;
        let pairingId = state.pairingId.clone();
        let pairingServiceVersion = state.pairingServiceVersion;
        self.pendingRemotePairings
            .lock()
            .map_err(|error| format!("pending remote pairing lock poisoned: {error}"))?
            .insert(pairingId.clone(), PendingRemotePairing { client, state });
        Ok(serde_json::json!({
            "pairingId": pairingId,
            "pairingServiceVersion": pairingServiceVersion,
            "coreDeviceId": hello.coreDeviceId,
            "coreDeviceInfo": hello.coreDeviceInfo,
        })
        .to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn remotePairFinish(&self, pairingId: String, pairingCode: String) -> Result<String, String> {
        let pending = self
            .pendingRemotePairings
            .lock()
            .map_err(|error| format!("pending remote pairing lock poisoned: {error}"))?
            .remove(&pairingId)
            .ok_or_else(|| "remote pairing not found".to_string())?;
        let session = self
            .runtime
            .block_on(pending.client.pairFinish(&pending.state, &pairingCode))?;
        let record = session.exportRecord();
        LinkAccessStore::new(self.runtimeStorageHost.clone())
            .saveOutboundSession(pairingId, record.clone())?;
        serde_json::to_string(&record).map_err(|error| error.to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn startWebAccessServer(
        &self,
        bindAddress: String,
        token: String,
        shutdownToken: String,
        webRoot: PathBuf,
        deviceInfo: RemoteDeviceInfo,
        enableWebAccess: bool,
        enableDiscovery: bool,
    ) -> Result<String, String> {
        self.stopWebAccessServer();
        let accessStore = LinkAccessStore::new(self.runtimeStorageHost.clone());
        let identity = accessStore.initializeIdentity(deviceInfo)?;
        accessStore.saveHostConfig(&LinkAccessHostConfig {
            bindAddress: bindAddress.clone(),
            token: token.clone(),
            webAccessEnabled: enableWebAccess,
            discoveryEnabled: enableDiscovery,
        })?;
        let deviceId = identity.deviceId;
        let deviceInfo = identity.deviceInfo;
        let responseDeviceId = deviceId.clone();
        let webAssetFileSystem = self.proxyCore.fileSystemHost();
        let address: SocketAddr = bindAddress
            .parse()
            .map_err(|error| format!("invalid bind address: {error}"))?;
        let listener = self
            .runtime
            .block_on(tokio::net::TcpListener::bind(address))
            .map_err(|error| error.to_string())?;

        if enableDiscovery {
            let mut mdns_guard = self
                .mdns
                .lock()
                .map_err(|error| format!("mDNS lock poisoned: {error}"))?;
            if mdns_guard.is_none() {
                let mut mdns = mdnss::MdnsHandle::new()?;
                let mut props = std::collections::HashMap::new();
                props.insert("deviceId".to_string(), deviceId.clone());
                props.insert("displayName".to_string(), deviceInfo.displayName());
                props.insert("platform".to_string(), deviceInfo.platform.clone());
                props.insert("model".to_string(), deviceInfo.model.clone());
                props.insert("tokenHash".to_string(), link_token_hash(&token));
                props.insert("version".to_string(), "1".to_string());
                mdns.register(address.port(), props)?;
                *mdns_guard = Some(mdns);
            }
        }
        let coreClient = SharedFlutterCoreClient {
            proxyCore: self.proxyCore.clone(),
        };
        let task = self.runtime.spawn(async move {
            RemoteLinkServer::serveWithListener(
                coreClient,
                RemoteLinkServerConfig {
                    bindAddress,
                    token: token.clone(),
                    localControlToken: Some(shutdownToken.clone()),
                    deviceId,
                    deviceInfo,
                    webAccess: if enableWebAccess {
                        Some(RemoteWebAccessConfig {
                            token,
                            shutdownToken,
                            webRoot,
                            readAsset: Arc::new(move |path| {
                                webAssetFileSystem
                                    .readFileBytes(&path.to_string_lossy())
                                    .map_err(|error| error.to_string())
                            }),
                        })
                    } else {
                        None
                    },
                    printStartupInfo: false,
                    accessStore,
                },
                listener,
                address,
            )
            .await
        });
        *self
            .webAccessTask
            .lock()
            .expect("web access task mutex poisoned") = Some(task);
        Ok(responseDeviceId)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn stopWebAccessServer(&self) {
        if let Some(task) = self
            .webAccessTask
            .lock()
            .expect("web access task mutex poisoned")
            .take()
        {
            task.abort();
        }
        if let Ok(mut mdns_guard) = self.mdns.lock() {
            if let Some(mdns) = mdns_guard.take() {
                let _ = mdns.unregister();
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn discoverDevices(&self, timeout_ms: u64) -> Result<String, String> {
        let devices = mdnss::discover_devices(timeout_ms)?;
        serde_json::to_string(&devices).map_err(|e| e.to_string())
    }
}

/// Loads one string-keyed JSON record map from persistent storage.
fn load_accepted_remote_sessions<T>(
    storage: &dyn RuntimeStorageHost,
    path: &str,
) -> Result<BTreeMap<String, T>, String>
where
    T: DeserializeOwned,
{
    if !storage.exists(path).map_err(|error| error.to_string())? {
        return Ok(BTreeMap::new());
    }
    let content = storage.readBytes(path).map_err(|error| error.to_string())?;
    let content = String::from_utf8(content).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

/// Inserts one record into a string-keyed JSON record map.
fn save_accepted_remote_session<T>(
    storage: &dyn RuntimeStorageHost,
    path: &str,
    sessionId: String,
    record: T,
) -> Result<(), String>
where
    T: Serialize + DeserializeOwned,
{
    let mut sessions = load_accepted_remote_sessions(storage, path)?;
    sessions.insert(sessionId, record);
    let content = serde_json::to_string_pretty(&sessions).map_err(|error| error.to_string())?;
    storage
        .writeBytes(path, content.as_bytes())
        .map_err(|error| error.to_string())
}

/// Saves one JSON record for a remote pairing code.
fn save_remote_pairing_code<T>(
    storage: &dyn RuntimeStorageHost,
    path: &str,
    record: T,
) -> Result<(), String>
where
    T: Serialize,
{
    let content = serde_json::to_string_pretty(&record).map_err(|error| error.to_string())?;
    storage
        .writeBytes(path, content.as_bytes())
        .map_err(|error| error.to_string())
}

#[derive(Clone)]
struct SharedFlutterCoreClient {
    proxyCore: Arc<LocalCoreProxy>,
}

#[async_trait(?Send)]
impl CoreLinkClient for SharedFlutterCoreClient {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self.proxyCore.as_ref(), request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self.proxyCore.as_ref(), request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self.proxyCore.as_ref(), request).await
    }
}

/// Installs the asynchronous controller permission requester for every runtime.
fn install_permission_requester(core: &mut LocalCoreProxy) {
    let handler = core.localApplicationMut().toolHandler.clone();
    handler
        .getToolPermissionSystem()
        .setAsyncPermissionRequester(move |tool, description| async move {
            let response = requestOwnerToolPermissionAsync(
                RuntimeHostInteractionToolPermissionPayload {
                    tool: tool_to_permission_payload(&tool),
                    description,
                },
                Duration::from_millis(PERMISSION_REQUEST_TIMEOUT_MS),
            )
            .await
            .expect("permission request failed");
            match response.result.as_str() {
                "allow" => PermissionRequestResult::ALLOW,
                "allow_session" => PermissionRequestResult::ALLOW_SESSION,
                "deny" => PermissionRequestResult::DENY,
                other => panic!("unknown permission response result: {other}"),
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

#[cfg(any(
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_env = "ohos"
))]
/// Creates a music playback command payload with default optional fields.
fn musicCommandPayload(command: &str) -> RuntimeHostInteractionMusicPlaybackPayload {
    RuntimeHostInteractionMusicPlaybackPayload {
        command: command.to_string(),
        source: None,
        sourceType: None,
        title: None,
        artist: None,
        loopPlayback: false,
        volume: 1.0,
        positionMs: 0,
    }
}

/// Serializes owner-command parameters into the JSON string expected by the platform channel.
fn serialize_owner_params_json(
    params: &serde_json::Value,
    label: &str,
) -> operit_host_api::HostResult<String> {
    serde_json::to_string(params).map_err(|error| {
        operit_host_api::HostError::new(format!("{label} params JSON encode failed: {error}"))
    })
}

#[cfg(any(
    windows,
    all(target_os = "linux", not(target_env = "ohos")),
    target_os = "android",
    target_os = "macos"
))]
fn create_local_core(
    runtime_root: PathBuf,
    workspace_root: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(runtime_root, workspace_root));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    #[cfg(target_os = "android")]
    let systemOperationHost: Arc<dyn operit_host_api::SystemOperationHost> =
        Arc::new(FlutterSystemOperationBridge::new());
    #[cfg(not(target_os = "android"))]
    let systemOperationHost: Arc<dyn operit_host_api::SystemOperationHost> =
        Arc::new(NativeSystemOperationHost::new());
    let mut context = HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(NativeFileSystemHost::new()),
        webVisitHost,
        Arc::new(NativeHttpHost::new()),
        systemOperationHost,
        Arc::new(NativeManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore);
    #[cfg(target_os = "android")]
    {
        context = context.withLocalInferenceHost(Arc::new(
            operit_host_android_native::AndroidLocalInferenceHost::new(),
        ));
        context = context.withHostRuntimeEventSchedulerHost(Arc::new(
            NativeHostRuntimeEventSchedulerHost::new(),
        ));
        context = context
            .withHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    }
    #[cfg(not(target_os = "android"))]
    {
        context = context.withHostRuntimeEventSchedulerHost(Arc::new(
            NativeHostRuntimeEventSchedulerHost::new(),
        ));
        context = context
            .withHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    }
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    if let Some(host) = browserSessionHost {
        context = context.withBrowserSessionHost(host);
    }
    if let Some(host) = composeDslWebViewHost {
        context = context.withComposeDslWebViewHost(host);
    }
    context = context.withTerminalHost(terminalHost);
    #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
    {
        context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::fromPlayers(
            Arc::new(|path| {
                let response = requestOwnerAudioPlay(
                    RuntimeHostInteractionAudioPlayPayload {
                        path: path.to_string(),
                    },
                    Duration::from_secs(60),
                )
                .map_err(operit_host_api::HostError::new)?;
                Ok(operit_host_api::AudioPlaybackStatus {
                    path: response.path,
                    started: response.started,
                    details: response.details,
                })
            }),
            Arc::new(|command| {
                let payload = match command {
                    NativeMusicCommand::Play(request) => {
                        RuntimeHostInteractionMusicPlaybackPayload {
                            command: "play".to_string(),
                            source: Some(request.source),
                            sourceType: Some(request.sourceType),
                            title: request.title,
                            artist: request.artist,
                            loopPlayback: request.loopPlayback,
                            volume: request.volume,
                            positionMs: request.startPositionMs,
                        }
                    }
                    NativeMusicCommand::Pause => musicCommandPayload("pause"),
                    NativeMusicCommand::Resume => musicCommandPayload("resume"),
                    NativeMusicCommand::Stop => musicCommandPayload("stop"),
                    NativeMusicCommand::Status => musicCommandPayload("status"),
                    NativeMusicCommand::Seek(positionMs) => {
                        RuntimeHostInteractionMusicPlaybackPayload {
                            positionMs,
                            ..musicCommandPayload("seek")
                        }
                    }
                    NativeMusicCommand::SetVolume(volume) => {
                        RuntimeHostInteractionMusicPlaybackPayload {
                            volume,
                            ..musicCommandPayload("set_volume")
                        }
                    }
                };
                let response = requestOwnerMusicPlayback(payload, Duration::from_secs(60))
                    .map_err(operit_host_api::HostError::new)?;
                Ok(operit_host_api::MusicPlaybackStatus {
                    state: response.state,
                    source: response.source,
                    sourceType: response.sourceType,
                    title: response.title,
                    artist: response.artist,
                    durationMs: response.durationMs,
                    positionMs: response.positionMs,
                    bufferedPositionMs: response.bufferedPositionMs,
                    volume: response.volume,
                    loopPlayback: response.loopPlayback,
                    message: response.message,
                })
            }),
        )));
        context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::fromController(
            Arc::new(|command, params| {
                let response = requestOwnerBluetooth(
                    RuntimeHostInteractionBluetoothPayload {
                        command: command.to_string(),
                        paramsJson: serialize_owner_params_json(&params, "platform Bluetooth")?,
                    },
                    Duration::from_secs(120),
                )
                .map_err(operit_host_api::HostError::new)?;
                serde_json::from_str(&response.resultJson).map_err(|error| {
                    operit_host_api::HostError::new(format!(
                        "platform Bluetooth response JSON decode failed: {error}"
                    ))
                })
            }),
        )));
        #[cfg(target_os = "android")]
        {
            context = context.withTtsSynthesisHost(Arc::new(
                NativeTtsSynthesisHost::fromSynthesizer(Arc::new(|request| {
                    let response = requestOwnerTtsSynthesis(
                        RuntimeHostInteractionTtsSynthesisPayload {
                            text: request.text,
                            voice: request.voice,
                            locale: request.locale,
                            speed: request.speed,
                            pitch: request.pitch,
                            outputFormat: request.outputFormat,
                        },
                        Duration::from_secs(120),
                    )
                    .map_err(operit_host_api::HostError::new)?;
                    Ok(operit_host_api::TtsSynthesisResponse {
                        audioPath: response.audioPath,
                        details: response.details,
                    })
                })),
            ));
        }
        context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::fromController(
            Arc::new(|command| {
                let payload = match command.command.as_str() {
                    "play" => {
                        let audioPath = command.audioPath.ok_or_else(|| {
                            operit_host_api::HostError::new("tts play audio path is required")
                        })?;
                        RuntimeHostInteractionTtsPlaybackPayload {
                            command: command.command,
                            audioPath: Some(audioPath),
                            text: String::new(),
                            voice: String::new(),
                            locale: String::new(),
                            speed: 1.0,
                            pitch: 1.0,
                            interrupt: true,
                        }
                    }
                    "speak" => {
                        let request = match command.request {
                            Some(request) => request,
                            None => {
                                return Err(operit_host_api::HostError::new(
                                    "tts speak request is required",
                                ));
                            }
                        };
                        RuntimeHostInteractionTtsPlaybackPayload {
                            command: command.command,
                            audioPath: None,
                            text: request.text,
                            voice: request.voice,
                            locale: request.locale,
                            speed: request.speed,
                            pitch: request.pitch,
                            interrupt: request.interrupt,
                        }
                    }
                    "pause" | "resume" | "stop" | "state" | "status" => {
                        RuntimeHostInteractionTtsPlaybackPayload {
                            command: command.command,
                            audioPath: None,
                            text: String::new(),
                            voice: String::new(),
                            locale: String::new(),
                            speed: 1.0,
                            pitch: 1.0,
                            interrupt: false,
                        }
                    }
                    other => {
                        return Err(operit_host_api::HostError::new(format!(
                            "unsupported tts playback command: {other}"
                        )));
                    }
                };
                let response = requestOwnerTtsPlayback(payload, Duration::from_secs(120))
                    .map_err(operit_host_api::HostError::new)?;
                Ok(operit_host_api::TtsPlaybackStatus {
                    path: response.path,
                    active: response.active,
                    paused: response.paused,
                    details: response.details,
                })
            }),
        )));
    }
    #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
    {
        context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::new()));
        context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::new()));
        context = context.withTtsSynthesisHost(Arc::new(NativeTtsSynthesisHost::new()));
        context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::new()));
        context = context.withHostRuntimeEventHost(Arc::new(NativeHostRuntimeEventHost::new()));
    }
    #[cfg(windows)]
    {
        context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::new()));
        context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::new()));
        context = context.withTtsSynthesisHost(Arc::new(NativeTtsSynthesisHost::new()));
        context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::new()));
        context = context.withHostRuntimeEventHost(Arc::new(NativeHostRuntimeEventHost::new()));
    }
    #[cfg(target_os = "macos")]
    {
        context = context.withHostRuntimeEventHost(Arc::new(NativeHostRuntimeEventHost::new()));
    }
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(target_env = "ohos")]
/// Creates the OpenHarmony runtime context from explicit app storage roots.
fn create_local_core(
    runtime_root: PathBuf,
    workspace_root: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    let managedRuntimeHost = Arc::new(NativeManagedRuntimeHost::new(workspace_root.clone()));
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(
        runtime_root.clone(),
        workspace_root,
    ));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let systemLanguageCode = Arc::new(systemLanguageCode);
    let systemOperationHost = Arc::new(NativeSystemOperationHost::fromOwnerCallbacks(
        Arc::new(move || Ok(systemLanguageCode.as_ref().clone())),
        Arc::new(|| {
            let response = requestOwnerSystemCaptureScreenshot(Duration::from_secs(60))
                .map_err(operit_host_api::HostError::new)?;
            Ok(response.path)
        }),
        Arc::new(|imagePath, language, quality| {
            Err(operit_host_api::HostError::new(format!(
                "OpenHarmony OCR is unavailable in the configured SDK; imagePath={imagePath}, language={}, quality={}",
                language.asHostValue(),
                quality.asHostValue()
            )))
        }),
        Arc::new(|operation, params| {
            let response = requestOwnerSystemOperation(
                RuntimeHostInteractionSystemOperationPayload {
                    operation: operation.to_string(),
                    paramsJson: serialize_owner_params_json(
                        &params,
                        "OpenHarmony system operation",
                    )?,
                },
                Duration::from_secs(120),
            )
            .map_err(operit_host_api::HostError::new)?;
            serde_json::from_str(&response.resultJson).map_err(|error| {
                operit_host_api::HostError::new(format!(
                    "OpenHarmony system operation response JSON decode failed: {error}"
                ))
            })
        }),
    ));
    let mut context = HostManager::withFileSystemWebVisitAndSystemOperationHosts(
        Arc::new(NativeFileSystemHost::fromPlatformActions(
            Arc::new(|path| {
                let response = requestOwnerFileOpen(
                    RuntimeHostInteractionFileOpenPayload {
                        path: path.to_string(),
                    },
                    Duration::from_secs(60),
                )
                .map_err(operit_host_api::HostError::new)?;
                if response.success {
                    return Ok(());
                }
                let Some(error) = response.error else {
                    return Err(operit_host_api::HostError::new(
                        "file open error is missing",
                    ));
                };
                Err(operit_host_api::HostError::new(error))
            }),
            Arc::new(|path, title| {
                let response = requestOwnerFileShare(
                    RuntimeHostInteractionFileSharePayload {
                        path: path.to_string(),
                        title: title.to_string(),
                    },
                    Duration::from_secs(60),
                )
                .map_err(operit_host_api::HostError::new)?;
                if response.success {
                    return Ok(());
                }
                let Some(error) = response.error else {
                    return Err(operit_host_api::HostError::new(
                        "file share error is missing",
                    ));
                };
                Err(operit_host_api::HostError::new(error))
            }),
        )),
        webVisitHost,
        systemOperationHost,
    );
    context.httpHost = Some(Arc::new(NativeHttpHost::new()));
    context.managedRuntimeHost = Some(managedRuntimeHost);
    context.runtimeStorageHost = Some(runtimeStorageHost);
    context.runtimeSqliteHost = Some(runtimeSqliteHost);
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    if let Some(host) = browserSessionHost {
        context = context.withBrowserSessionHost(host);
    }
    if let Some(host) = composeDslWebViewHost {
        context = context.withComposeDslWebViewHost(host);
    }
    context = context.withTerminalHost(terminalHost);
    context = context.withLocalInferenceHost(Arc::new(NativeLocalInferenceHost::new()));
    context =
        context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::new(Arc::new(|command| {
            let payload = RuntimeHostInteractionTtsPlaybackPayload {
                command: command.command,
                audioPath: command.audioPath,
                text: String::new(),
                voice: String::new(),
                locale: String::new(),
                speed: 1.0,
                pitch: 1.0,
                interrupt: false,
            };
            let response = requestOwnerTtsPlayback(payload, Duration::from_secs(120))
                .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::TtsPlaybackStatus {
                path: response.path,
                active: response.active,
                paused: response.paused,
                details: response.details,
            })
        }))));
    context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::fromPlayers(
        Arc::new(|path| {
            let response = requestOwnerAudioPlay(
                RuntimeHostInteractionAudioPlayPayload {
                    path: path.to_string(),
                },
                Duration::from_secs(60),
            )
            .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::AudioPlaybackStatus {
                path: response.path,
                started: response.started,
                details: response.details,
            })
        }),
        Arc::new(|command| {
            let payload = match command {
                NativeMusicCommand::Play(request) => RuntimeHostInteractionMusicPlaybackPayload {
                    command: "play".to_string(),
                    source: Some(request.source),
                    sourceType: Some(request.sourceType),
                    title: request.title,
                    artist: request.artist,
                    loopPlayback: request.loopPlayback,
                    volume: request.volume,
                    positionMs: request.startPositionMs,
                },
                NativeMusicCommand::Pause => musicCommandPayload("pause"),
                NativeMusicCommand::Resume => musicCommandPayload("resume"),
                NativeMusicCommand::Stop => musicCommandPayload("stop"),
                NativeMusicCommand::Status => musicCommandPayload("status"),
                NativeMusicCommand::Seek(positionMs) => {
                    RuntimeHostInteractionMusicPlaybackPayload {
                        positionMs,
                        ..musicCommandPayload("seek")
                    }
                }
                NativeMusicCommand::SetVolume(volume) => {
                    RuntimeHostInteractionMusicPlaybackPayload {
                        volume,
                        ..musicCommandPayload("set_volume")
                    }
                }
            };
            let response = requestOwnerMusicPlayback(payload, Duration::from_secs(60))
                .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::MusicPlaybackStatus {
                state: response.state,
                source: response.source,
                sourceType: response.sourceType,
                title: response.title,
                artist: response.artist,
                durationMs: response.durationMs,
                positionMs: response.positionMs,
                bufferedPositionMs: response.bufferedPositionMs,
                volume: response.volume,
                loopPlayback: response.loopPlayback,
                message: response.message,
            })
        }),
    )));
    context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::fromController(Arc::new(
        |command, params| {
            let response = requestOwnerBluetooth(
                RuntimeHostInteractionBluetoothPayload {
                    command: command.to_string(),
                    paramsJson: serialize_owner_params_json(&params, "OpenHarmony Bluetooth")?,
                },
                Duration::from_secs(120),
            )
            .map_err(operit_host_api::HostError::new)?;
            serde_json::from_str(&response.resultJson).map_err(|error| {
                operit_host_api::HostError::new(format!(
                    "OpenHarmony Bluetooth response JSON decode failed: {error}"
                ))
            })
        },
    ))));
    context = context
        .withHostRuntimeEventSchedulerHost(Arc::new(NativeHostRuntimeEventSchedulerHost::new()));
    context = context
        .withHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(any(
    windows,
    all(target_os = "linux", not(target_env = "ohos")),
    target_os = "ios",
    target_os = "macos"
))]
fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Ok((
        NativeRuntimeStorageHost::defaultRuntimeRoot(),
        NativeRuntimeStorageHost::defaultWorkspaceRoot(),
    ))
}

#[cfg(target_env = "ohos")]
/// Requires OpenHarmony owner code to provide application storage roots.
fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Err(
        "OpenHarmony runtime and workspace roots must be provided by the OpenHarmony host"
            .to_string(),
    )
}

#[cfg(target_os = "android")]
fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Err("Android runtime and workspace roots must be provided by the Android host".to_string())
}

#[cfg(target_arch = "wasm32")]
fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Ok((
        NativeRuntimeStorageHost::defaultRuntimeRoot(),
        NativeRuntimeStorageHost::defaultWorkspaceRoot(),
    ))
}

#[cfg(target_os = "ios")]
fn create_local_core(
    runtime_root: PathBuf,
    workspace_root: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(runtime_root, workspace_root));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    let mut context = HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(NativeFileSystemHost::new()),
        webVisitHost,
        Arc::new(NativeHttpHost::new()),
        Arc::new(NativeSystemOperationHost::new()),
        Arc::new(NativeManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore);
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    if let Some(host) = browserSessionHost {
        context = context.withBrowserSessionHost(host);
    }
    if let Some(host) = composeDslWebViewHost {
        context = context.withComposeDslWebViewHost(host);
    }
    context = context.withTerminalHost(terminalHost);
    context = context.withLocalInferenceHost(Arc::new(NativeLocalInferenceHost::fromExecutor(
        Arc::new(|command| {
            let response = requestOwnerLocalInference(
                RuntimeHostInteractionLocalInferencePayload {
                    method: command.method,
                    requestJson: command.requestJson,
                },
                Duration::from_secs(600),
            )
            .map_err(operit_host_api::HostError::new)?;
            Ok(response.resultJson)
        }),
    )));
    context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::fromPlayers(
        Arc::new(|path| {
            let response = requestOwnerAudioPlay(
                RuntimeHostInteractionAudioPlayPayload {
                    path: path.to_string(),
                },
                Duration::from_secs(60),
            )
            .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::AudioPlaybackStatus {
                path: response.path,
                started: response.started,
                details: response.details,
            })
        }),
        Arc::new(|command| {
            let payload = match command {
                NativeMusicCommand::Play(request) => RuntimeHostInteractionMusicPlaybackPayload {
                    command: "play".to_string(),
                    source: Some(request.source),
                    sourceType: Some(request.sourceType),
                    title: request.title,
                    artist: request.artist,
                    loopPlayback: request.loopPlayback,
                    volume: request.volume,
                    positionMs: request.startPositionMs,
                },
                NativeMusicCommand::Pause => musicCommandPayload("pause"),
                NativeMusicCommand::Resume => musicCommandPayload("resume"),
                NativeMusicCommand::Stop => musicCommandPayload("stop"),
                NativeMusicCommand::Status => musicCommandPayload("status"),
                NativeMusicCommand::Seek(positionMs) => {
                    RuntimeHostInteractionMusicPlaybackPayload {
                        positionMs,
                        ..musicCommandPayload("seek")
                    }
                }
                NativeMusicCommand::SetVolume(volume) => {
                    RuntimeHostInteractionMusicPlaybackPayload {
                        volume,
                        ..musicCommandPayload("set_volume")
                    }
                }
            };
            let response = requestOwnerMusicPlayback(payload, Duration::from_secs(60))
                .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::MusicPlaybackStatus {
                state: response.state,
                source: response.source,
                sourceType: response.sourceType,
                title: response.title,
                artist: response.artist,
                durationMs: response.durationMs,
                positionMs: response.positionMs,
                bufferedPositionMs: response.bufferedPositionMs,
                volume: response.volume,
                loopPlayback: response.loopPlayback,
                message: response.message,
            })
        }),
    )));
    context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::fromController(Arc::new(
        |command, params| {
            let response = requestOwnerBluetooth(
                RuntimeHostInteractionBluetoothPayload {
                    command: command.to_string(),
                    paramsJson: serialize_owner_params_json(&params, "platform Bluetooth")?,
                },
                Duration::from_secs(120),
            )
            .map_err(operit_host_api::HostError::new)?;
            serde_json::from_str(&response.resultJson).map_err(|error| {
                operit_host_api::HostError::new(format!(
                    "platform Bluetooth response JSON decode failed: {error}"
                ))
            })
        },
    ))));
    context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::fromController(
        Arc::new(|command| {
            let payload = match command.command.as_str() {
                "play" => {
                    let audioPath = command.audioPath.ok_or_else(|| {
                        operit_host_api::HostError::new("tts play audio path is required")
                    })?;
                    RuntimeHostInteractionTtsPlaybackPayload {
                        command: command.command,
                        audioPath: Some(audioPath),
                        text: String::new(),
                        voice: String::new(),
                        locale: String::new(),
                        speed: 1.0,
                        pitch: 1.0,
                        interrupt: true,
                    }
                }
                "speak" => {
                    let request = match command.request {
                        Some(request) => request,
                        None => {
                            return Err(operit_host_api::HostError::new(
                                "tts speak request is required",
                            ));
                        }
                    };
                    RuntimeHostInteractionTtsPlaybackPayload {
                        command: command.command,
                        audioPath: None,
                        text: request.text,
                        voice: request.voice,
                        locale: request.locale,
                        speed: request.speed,
                        pitch: request.pitch,
                        interrupt: request.interrupt,
                    }
                }
                "pause" | "resume" | "stop" | "status" => {
                    RuntimeHostInteractionTtsPlaybackPayload {
                        command: command.command,
                        audioPath: None,
                        text: String::new(),
                        voice: String::new(),
                        locale: String::new(),
                        speed: 1.0,
                        pitch: 1.0,
                        interrupt: false,
                    }
                }
                other => {
                    return Err(operit_host_api::HostError::new(format!(
                        "unsupported tts playback command: {other}"
                    )));
                }
            };
            let response = requestOwnerTtsPlayback(payload, Duration::from_secs(120))
                .map_err(operit_host_api::HostError::new)?;
            Ok(operit_host_api::TtsPlaybackStatus {
                path: response.path,
                active: response.active,
                paused: response.paused,
                details: response.details,
            })
        }),
    )));
    context = context
        .withHostRuntimeEventSchedulerHost(Arc::new(NativeHostRuntimeEventSchedulerHost::new()));
    context = context
        .withHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(target_arch = "wasm32")]
fn create_local_core(
    _runtime_root: PathBuf,
    _workspace_root: PathBuf,
    _webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    _browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    _browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    _composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
) -> Result<LocalCoreProxy, String> {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new());
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let hostSecretStore = runtimeStorageHost.clone();
    let mut context = HostManager::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
        Arc::new(NativeFileSystemHost::new()),
        Arc::new(operit_host_web::WebWebVisitHost::new()),
        Arc::new(NativeHttpHost::new()),
        Arc::new(NativeSystemOperationHost::new()),
        Arc::new(NativeManagedRuntimeHost::new()),
        runtimeStorageHost,
        runtimeSqliteHost,
    )
    .withHostSecretStore(hostSecretStore);
    context = context.withBrowserSessionHost(Arc::new(NativeBrowserSessionHost::new()));
    context = context.withTerminalHost(Arc::new(NativeTerminalHost::new()));
    context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::new()));
    context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::new()));
    context = context.withLocalInferenceHost(Arc::new(NativeLocalInferenceHost::new()));
    context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::new()));
    context = context
        .withHostRuntimeEventSchedulerHost(Arc::new(NativeHostRuntimeEventSchedulerHost::new()));
    context = context
        .withHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    context = context.withHostRuntimeEventHost(Arc::new(NativeHostRuntimeEventHost::new()));
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(not(any(
    windows,
    all(target_os = "linux", not(target_env = "ohos")),
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_env = "ohos",
    target_arch = "wasm32"
)))]
fn create_local_core(
    _runtime_root: PathBuf,
    _workspace_root: PathBuf,
    _webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    _browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    _browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    _composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    #[cfg(any(
        windows,
        all(target_os = "linux", not(target_env = "ohos")),
        target_os = "android"
    ))]
    _terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    Err("operit flutter native runtime bridge is not available for this target".to_string())
}

#[no_mangle]
#[cfg(not(target_env = "ohos"))]
pub extern "C" fn operit_flutter_bridge_create() -> *mut OperitFlutterBridge {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(OperitFlutterBridge::new)) {
        Ok(Ok(bridge)) => Box::into_raw(Box::new(bridge)),
        Ok(Err(error)) => {
            set_last_create_error(error);
            std::ptr::null_mut()
        }
        Err(payload) => {
            set_last_create_error(format!(
                "FATAL_CORE_PANIC: {}",
                panic_payload_message(payload.as_ref())
            ));
            std::ptr::null_mut()
        }
    }
}

#[repr(C)]
pub struct OperitByteBuffer {
    pub ptr: *mut u8,
    pub len: usize,
}

impl OperitByteBuffer {
    /// Creates an empty byte buffer for a failed or closed native operation.
    fn empty() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
        }
    }
}

#[no_mangle]
#[cfg(not(target_env = "ohos"))]
pub unsafe extern "C" fn operit_flutter_bridge_create_with_storage_roots(
    runtime_root: *const c_char,
    workspace_root: *const c_char,
) -> *mut OperitFlutterBridge {
    if runtime_root.is_null() {
        set_last_create_error("runtime storage root pointer is null".to_string());
        return std::ptr::null_mut();
    }
    if workspace_root.is_null() {
        set_last_create_error("workspace storage root pointer is null".to_string());
        return std::ptr::null_mut();
    }
    let runtime_root = match CStr::from_ptr(runtime_root).to_str() {
        Ok(value) => PathBuf::from(value),
        Err(error) => {
            set_last_create_error(format!("runtime storage root is not valid UTF-8: {error}"));
            return std::ptr::null_mut();
        }
    };
    let workspace_root = match CStr::from_ptr(workspace_root).to_str() {
        Ok(value) => PathBuf::from(value),
        Err(error) => {
            set_last_create_error(format!(
                "workspace storage root is not valid UTF-8: {error}"
            ));
            return std::ptr::null_mut();
        }
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        OperitFlutterBridge::new_with_storage_roots(runtime_root, workspace_root)
    })) {
        Ok(Ok(bridge)) => Box::into_raw(Box::new(bridge)),
        Ok(Err(error)) => {
            set_last_create_error(error);
            std::ptr::null_mut()
        }
        Err(payload) => {
            set_last_create_error(format!(
                "FATAL_CORE_PANIC: {}",
                panic_payload_message(payload.as_ref())
            ));
            std::ptr::null_mut()
        }
    }
}

/// Creates an OpenHarmony bridge with an owner-supplied system language code.
#[no_mangle]
#[cfg(target_env = "ohos")]
pub unsafe extern "C" fn operit_flutter_bridge_create_with_storage_roots_and_system_language(
    runtime_root: *const c_char,
    workspace_root: *const c_char,
    system_language_code: *const c_char,
) -> *mut OperitFlutterBridge {
    if runtime_root.is_null() {
        set_last_create_error("runtime storage root pointer is null".to_string());
        return std::ptr::null_mut();
    }
    if workspace_root.is_null() {
        set_last_create_error("workspace storage root pointer is null".to_string());
        return std::ptr::null_mut();
    }
    if system_language_code.is_null() {
        set_last_create_error("system language code pointer is null".to_string());
        return std::ptr::null_mut();
    }
    let runtime_root = match CStr::from_ptr(runtime_root).to_str() {
        Ok(value) => PathBuf::from(value),
        Err(error) => {
            set_last_create_error(format!("runtime storage root is not valid UTF-8: {error}"));
            return std::ptr::null_mut();
        }
    };
    let workspace_root = match CStr::from_ptr(workspace_root).to_str() {
        Ok(value) => PathBuf::from(value),
        Err(error) => {
            set_last_create_error(format!(
                "workspace storage root is not valid UTF-8: {error}"
            ));
            return std::ptr::null_mut();
        }
    };
    let system_language_code = match CStr::from_ptr(system_language_code).to_str() {
        Ok(value) if !value.trim().is_empty() => value.to_string(),
        Ok(_) => {
            set_last_create_error("system language code is empty".to_string());
            return std::ptr::null_mut();
        }
        Err(error) => {
            set_last_create_error(format!("system language code is not valid UTF-8: {error}"));
            return std::ptr::null_mut();
        }
    };
    match OperitFlutterBridge::new_with_storage_roots(
        runtime_root,
        workspace_root,
        system_language_code,
    ) {
        Ok(bridge) => Box::into_raw(Box::new(bridge)),
        Err(error) => {
            set_last_create_error(error);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn operit_flutter_bridge_create_error() -> *mut c_char {
    string_to_ptr(
        last_create_error()
            .lock()
            .expect("create error lock must not be poisoned")
            .clone(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_destroy(handle: *mut OperitFlutterBridge) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Dispatches one compact native CoreProxy call for every native platform channel.
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_native_call(
    handle: *const OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> OperitByteBuffer {
    if handle.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-null",
            "runtime bridge is not initialized",
        ));
    }
    if request_ptr.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-null-request",
            "runtime request pointer is null",
        ));
    }
    let request_bytes = std::slice::from_raw_parts(request_ptr, request_len);
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bridge_native_call(&*handle, request_bytes)
    })) {
        Ok(response) => bytes_to_buffer(response),
        Err(payload) => bytes_to_buffer(native_core_panic_result(payload.as_ref())),
    }
}

/// Serializes one captured Core panic into the standard native Link result envelope.
#[cfg(not(target_arch = "wasm32"))]
fn native_core_panic_result(payload: &(dyn Any + Send)) -> Vec<u8> {
    native_result_vec(Err::<operit_link::CoreValue, _>(CoreLinkError {
        code: "FATAL_CORE_PANIC".to_string(),
        message: format!("Core runtime panic: {}", panic_payload_message(payload)),
        details: None,
        location: None,
        backtrace: Some(std::backtrace::Backtrace::force_capture().to_string()),
    }))
}

/// Converts a Rust panic payload into the stable text carried by crash reports.
fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "non-string panic payload".to_string()
}

/// Decodes and dispatches one compact native CoreProxy call.
#[cfg(not(target_arch = "wasm32"))]
fn bridge_native_call(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request = match decode_native_call_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<operit_link::CoreValue, _>(error)),
    };
    native_result_vec(handle.call(request).result)
}

/// Decodes one compact native CoreProxy request into the shared Link model.
fn decode_native_call_request(request_bytes: &[u8]) -> Result<CoreCallRequest, CoreLinkError> {
    let (request_id, target_segments, method_name, args): (
        String,
        Vec<String>,
        String,
        operit_link::CoreValue,
    ) = operit_link::decodeLink(request_bytes).map_err(|error| {
        CoreLinkError::new(
            "flutter-bridge-invalid-request",
            format!("invalid compact core request: {error}"),
        )
    })?;
    Ok(CoreCallRequest::new(
        request_id,
        operit_link::CoreObjectPath {
            segments: target_segments,
        },
        method_name,
        args,
    ))
}

/// Decodes one compact native CoreProxy push-open request.
fn decode_native_push_open_request(request_bytes: &[u8]) -> Result<CorePushRequest, CoreLinkError> {
    let (request_id, target_segments, method_name): (String, Vec<String>, String) =
        operit_link::decodeLink(request_bytes).map_err(|error| {
            CoreLinkError::new(
                "flutter-bridge-invalid-request",
                format!("invalid compact push-open request: {error}"),
            )
        })?;
    Ok(CorePushRequest::new(
        request_id,
        operit_link::CoreObjectPath {
            segments: target_segments,
        },
        method_name,
    ))
}

/// Decodes one compact native CoreProxy push item.
fn decode_native_push_item(request_bytes: &[u8]) -> Result<CorePushItem, CoreLinkError> {
    let (push_id, sequence, args): (String, u64, operit_link::CoreValue) =
        operit_link::decodeLink(request_bytes).map_err(|error| {
            CoreLinkError::new(
                "flutter-bridge-invalid-request",
                format!("invalid compact push item: {error}"),
            )
        })?;
    Ok(CorePushItem {
        pushId: push_id,
        sequence,
        args,
    })
}

/// Decodes one compact native CoreProxy watch snapshot request.
fn decode_native_watch_snapshot_request(
    request_bytes: &[u8],
) -> Result<CoreWatchRequest, CoreLinkError> {
    let (request_id, target_segments, property_name, args): (
        String,
        Vec<String>,
        String,
        operit_link::CoreValue,
    ) = operit_link::decodeLink(request_bytes).map_err(|error| {
        CoreLinkError::new(
            "flutter-bridge-invalid-request",
            format!("invalid compact watch snapshot request: {error}"),
        )
    })?;
    Ok(CoreWatchRequest::new(
        request_id,
        operit_link::CoreObjectPath {
            segments: target_segments,
        },
        property_name,
        args,
    ))
}

/// Decodes one compact native CoreProxy watch stream open request.
fn decode_native_watch_stream_request(
    request_bytes: &[u8],
) -> Result<(String, CoreWatchRequest), CoreLinkError> {
    let (subscription_id, request_id, target_segments, property_name, args): (
        String,
        String,
        Vec<String>,
        String,
        operit_link::CoreValue,
    ) = operit_link::decodeLink(request_bytes).map_err(|error| {
        CoreLinkError::new(
            "flutter-bridge-invalid-request",
            format!("invalid compact watch stream request: {error}"),
        )
    })?;
    Ok((
        subscription_id,
        CoreWatchRequest::new(
            request_id,
            operit_link::CoreObjectPath {
                segments: target_segments,
            },
            property_name,
            args,
        ),
    ))
}

/// Encodes one compact native CoreProxy result without a field-name map.
fn native_result_vec<T>(result: Result<T, CoreLinkError>) -> Vec<u8>
where
    T: Serialize,
{
    match result {
        Ok(value) => operit_link::encodeLink((0u8, value))
            .expect("compact native success response must encode"),
        Err(error) => operit_link::encodeLink((
            1u8,
            error.code,
            error.message,
            error.details,
            error
                .location
                .map(|location| (location.file, location.line, location.column)),
            error.backtrace,
        ))
        .expect("compact native error response must encode"),
    }
}

/// Encodes one compact native CoreProxy error result.
fn native_result_error_vec(code: &str, message: impl Into<String>) -> Vec<u8> {
    native_result_vec(Err::<(), _>(CoreLinkError::new(code, message.into())))
}

/// Encodes one compact native CoreProxy watch channel event.
fn native_watch_event_vec(subscription_id: &str, event: CoreEvent) -> Vec<u8> {
    operit_link::encodeLink((subscription_id, native_watch_event_payload(event)))
        .expect("compact native watch event must encode")
}

/// Converts one CoreProxy event into its compact native payload tuple.
fn native_watch_event_payload(
    event: CoreEvent,
) -> (
    Option<String>,
    Vec<String>,
    String,
    &'static str,
    operit_link::CoreValue,
) {
    let CoreEvent {
        requestId,
        targetPath,
        propertyName,
        kind,
        value,
    } = event;
    (
        requestId.map(|request_id| request_id.0),
        targetPath.segments,
        propertyName,
        native_event_kind_name(kind),
        value,
    )
}

/// Converts one native CoreProxy event kind into its direct wire literal.
fn native_event_kind_name(kind: CoreEventKind) -> &'static str {
    match kind {
        CoreEventKind::Snapshot => "Snapshot",
        CoreEventKind::Changed => "Changed",
        CoreEventKind::Completed => "Completed",
    }
}

#[cfg(test)]
mod native_call_codec_tests {
    use super::*;

    /// Verifies the compact request tuple decodes without a Link envelope map.
    #[test]
    fn decodes_compact_request_tuple() {
        let bytes = operit_link::encodeLink((
            "request-1",
            vec!["preferences", "cardManager"],
            "getCards",
            operit_link::CoreValue::Bool(true),
        ))
        .expect("compact request must encode");

        let request = decode_native_call_request(&bytes).expect("compact request must decode");

        assert_eq!(request.requestId.0, "request-1");
        assert_eq!(
            request.targetPath.segments,
            vec!["preferences", "cardManager"]
        );
        assert_eq!(request.methodName, "getCards");
        assert_eq!(request.args, operit_link::CoreValue::Bool(true));
    }

    /// Verifies every local stream request decodes from a compact tuple.
    #[test]
    fn decodes_compact_push_and_watch_tuples() {
        let push_open = operit_link::encodeLink(("push-1", vec!["runtime", "browser"], "interact"))
            .expect("compact push open must encode");
        let push_request =
            decode_native_push_open_request(&push_open).expect("compact push open must decode");
        assert_eq!(push_request.requestId.0, "push-1");
        assert_eq!(push_request.targetPath.segments, vec!["runtime", "browser"]);
        assert_eq!(push_request.methodName, "interact");

        let push_item = operit_link::encodeLink((
            "push-1",
            2u64,
            operit_link::CoreValue::String("click".to_string()),
        ))
        .expect("compact push item must encode");
        let item = decode_native_push_item(&push_item).expect("compact push item must decode");
        assert_eq!(item.pushId, "push-1");
        assert_eq!(item.sequence, 2);
        assert_eq!(
            item.args,
            operit_link::CoreValue::String("click".to_string())
        );

        let snapshot = operit_link::encodeLink((
            "watch-1",
            vec!["preferences", "cardManager"],
            "cards",
            operit_link::CoreValue::Null,
        ))
        .expect("compact watch snapshot must encode");
        let snapshot_request = decode_native_watch_snapshot_request(&snapshot)
            .expect("compact watch snapshot must decode");
        assert_eq!(snapshot_request.requestId.0, "watch-1");
        assert_eq!(snapshot_request.propertyName, "cards");

        let stream = operit_link::encodeLink((
            "subscription-1",
            "watch-1",
            vec!["preferences", "cardManager"],
            "cards",
            operit_link::CoreValue::Null,
        ))
        .expect("compact watch stream must encode");
        let (subscription_id, stream_request) =
            decode_native_watch_stream_request(&stream).expect("compact watch stream must decode");
        assert_eq!(subscription_id, "subscription-1");
        assert_eq!(stream_request.requestId.0, "watch-1");
        assert_eq!(stream_request.propertyName, "cards");
    }

    /// Verifies the compact success response retains its status and value fields.
    #[test]
    fn encodes_compact_success_tuple() {
        let bytes = native_result_vec(Ok(operit_link::CoreValue::String("ready".to_string())));

        let (status, value): (u8, operit_link::CoreValue) =
            operit_link::decodeLink(&bytes).expect("compact success must decode");

        assert_eq!(status, 0);
        assert_eq!(value, operit_link::CoreValue::String("ready".to_string()));
    }

    /// Verifies the compact error response retains every error field without a map envelope.
    #[test]
    fn encodes_compact_error_tuple() {
        let bytes = native_result_vec(Err::<(), _>(CoreLinkError {
            code: "CARD_NOT_FOUND".to_string(),
            message: "Card does not exist".to_string(),
            details: Some(operit_link::CoreValue::String("card-1".to_string())),
            location: Some(operit_link::protocol::CoreLinkErrorLocation {
                file: "CharacterCardManager.rs".to_string(),
                line: 28,
                column: 7,
            }),
            backtrace: Some("native backtrace".to_string()),
        }));

        let (status, code, message, details, location, backtrace): (
            u8,
            String,
            String,
            Option<operit_link::CoreValue>,
            Option<(String, u32, u32)>,
            Option<String>,
        ) = operit_link::decodeLink(&bytes).expect("compact error must decode");

        assert_eq!(status, 1);
        assert_eq!(code, "CARD_NOT_FOUND");
        assert_eq!(message, "Card does not exist");
        assert_eq!(
            details,
            Some(operit_link::CoreValue::String("card-1".to_string()))
        );
        assert_eq!(
            location,
            Some(("CharacterCardManager.rs".to_string(), 28, 7))
        );
        assert_eq!(backtrace.as_deref(), Some("native backtrace"));
    }

    /// Verifies watch snapshots and channel events keep fixed tuple field order.
    #[test]
    fn encodes_compact_watch_tuples() {
        let event = CoreEvent {
            requestId: Some(operit_link::CoreRequestId::new("watch-1")),
            targetPath: operit_link::CoreObjectPath {
                segments: vec!["preferences".to_string(), "cardManager".to_string()],
            },
            propertyName: "cards".to_string(),
            kind: CoreEventKind::Snapshot,
            value: operit_link::CoreValue::String("card-1".to_string()),
        };
        let snapshot = native_result_vec(Ok(native_watch_event_payload(event.clone())));
        let (status, payload): (
            u8,
            (
                Option<String>,
                Vec<String>,
                String,
                String,
                operit_link::CoreValue,
            ),
        ) = operit_link::decodeLink(&snapshot).expect("compact watch snapshot must decode");
        assert_eq!(status, 0);
        assert_eq!(payload.0.as_deref(), Some("watch-1"));
        assert_eq!(payload.1, vec!["preferences", "cardManager"]);
        assert_eq!(payload.2, "cards");
        assert_eq!(payload.3, "Snapshot");

        let frame = native_watch_event_vec("subscription-1", event);
        let (subscription_id, frame_payload): (
            String,
            (
                Option<String>,
                Vec<String>,
                String,
                String,
                operit_link::CoreValue,
            ),
        ) = operit_link::decodeLink(&frame).expect("compact watch frame must decode");
        assert_eq!(subscription_id, "subscription-1");
        assert_eq!(frame_payload.0.as_deref(), Some("watch-1"));
        assert_eq!(frame_payload.3, "Snapshot");
    }
}

#[cfg(target_arch = "wasm32")]
/// Decodes and dispatches one compact wasm CoreProxy call.
async fn bridge_native_call_async(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request = match decode_native_call_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<operit_link::CoreValue, _>(error)),
    };
    native_result_vec(handle.call(request).await.result)
}

/// Decodes and opens one compact native CoreProxy push stream.
fn bridge_push_open(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request = match decode_native_push_open_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<String, _>(error)),
    };
    native_result_vec(handle.pushOpen(request))
}

/// Decodes and dispatches one compact native CoreProxy push item.
#[cfg(not(target_arch = "wasm32"))]
fn bridge_push_item(handle: &OperitFlutterBridge, item_bytes: &[u8]) -> Vec<u8> {
    let item = match decode_native_push_item(item_bytes) {
        Ok(item) => item,
        Err(error) => return native_result_vec(Err::<(), _>(error)),
    };
    native_result_vec(handle.pushItem(item))
}

/// Decodes and dispatches one compact wasm CoreProxy push item.
#[cfg(target_arch = "wasm32")]
async fn bridge_push_item_async(handle: &OperitFlutterBridge, item_bytes: &[u8]) -> Vec<u8> {
    let item = match decode_native_push_item(item_bytes) {
        Ok(item) => item,
        Err(error) => return native_result_vec(Err::<(), _>(error)),
    };
    native_result_vec(handle.pushItem(item).await)
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_push_open(
    handle: *mut OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> OperitByteBuffer {
    if handle.is_null() || request_ptr.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-invalid-request",
            "runtime push open arguments are invalid",
        ));
    }
    bytes_to_buffer(bridge_push_open(
        &*handle,
        std::slice::from_raw_parts(request_ptr, request_len),
    ))
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_push_item(
    handle: *mut OperitFlutterBridge,
    item_ptr: *const u8,
    item_len: usize,
) -> OperitByteBuffer {
    if handle.is_null() || item_ptr.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-invalid-request",
            "runtime push item arguments are invalid",
        ));
    }
    bytes_to_buffer(bridge_push_item(
        &*handle,
        std::slice::from_raw_parts(item_ptr, item_len),
    ))
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_push_close(
    handle: *mut OperitFlutterBridge,
    push_id_ptr: *const c_char,
) -> OperitByteBuffer {
    if handle.is_null() || push_id_ptr.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-invalid-request",
            "runtime push close arguments are invalid",
        ));
    }
    let pushId = match CStr::from_ptr(push_id_ptr).to_str() {
        Ok(value) => value,
        Err(error) => {
            return bytes_to_buffer(native_result_error_vec(
                "flutter-bridge-invalid-request",
                error.to_string(),
            ));
        }
    };
    bytes_to_buffer(native_result_vec((*handle).pushClose(pushId)))
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_watch_snapshot(
    handle: *mut OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> OperitByteBuffer {
    if handle.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-null",
            "runtime bridge is not initialized",
        ));
    }
    if request_ptr.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-null-request",
            "runtime request pointer is null",
        ));
    }
    let request_bytes = std::slice::from_raw_parts(request_ptr, request_len);
    bytes_to_buffer(bridge_watch_snapshot(&mut *handle, request_bytes))
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_watch_stream(
    handle: *mut OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> OperitByteBuffer {
    if handle.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-null",
            "runtime bridge is not initialized",
        ));
    }
    if request_ptr.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-null-request",
            "runtime request pointer is null",
        ));
    }
    let request_bytes = std::slice::from_raw_parts(request_ptr, request_len);
    bytes_to_buffer(bridge_watch_stream(&mut *handle, request_bytes))
}

#[cfg(not(target_arch = "wasm32"))]
/// Decodes and opens one compact native CoreProxy watch stream.
fn bridge_watch_stream(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let (subscription_id, request) = match decode_native_watch_stream_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<String, _>(error)),
    };
    native_result_vec(handle.watchStream(subscription_id, request))
}

#[cfg(target_arch = "wasm32")]
/// Decodes and opens one compact wasm CoreProxy watch stream.
fn bridge_watch_stream_wasm(
    handle: &OperitFlutterBridge,
    request_bytes: &[u8],
    onEvent: Function,
) -> Vec<u8> {
    let (subscription_id, request) = match decode_native_watch_stream_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<String, _>(error)),
    };
    native_result_vec(handle.watchStream(subscription_id, request, onEvent))
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_next_watch_channel_event(
    handle: *mut OperitFlutterBridge,
) -> OperitByteBuffer {
    if handle.is_null() {
        return OperitByteBuffer::empty();
    }
    match (*handle).nextWatchChannelEvent() {
        Ok(event) => bytes_to_buffer(event),
        Err(_) => OperitByteBuffer::empty(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_close_watch_stream(
    handle: *mut OperitFlutterBridge,
    subscription_ptr: *const c_char,
) -> OperitByteBuffer {
    if handle.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-null",
            "runtime bridge is not initialized",
        ));
    }
    if subscription_ptr.is_null() {
        return bytes_to_buffer(native_result_error_vec(
            "flutter-bridge-null-request",
            "watch subscription pointer is null",
        ));
    }
    let subscription_id = match CStr::from_ptr(subscription_ptr).to_str() {
        Ok(value) => value,
        Err(error) => {
            return bytes_to_buffer(native_result_error_vec(
                "flutter-bridge-invalid-request",
                error.to_string(),
            ));
        }
    };
    (*handle).closeWatchStream(subscription_id);
    bytes_to_buffer(native_result_vec(Ok::<(), CoreLinkError>(())))
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_start_web_access_server(
    handle: *mut OperitFlutterBridge,
    bind_address: *const c_char,
    token: *const c_char,
    shutdown_token: *const c_char,
    web_root: *const c_char,
    device_info_json: *const c_char,
    enable_web_access: *const c_char,
    enable_discovery: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    let args = [
        ("bind address", bind_address),
        ("token", token),
        ("shutdown token", shutdown_token),
        ("web root", web_root),
        ("device info", device_info_json),
        ("enable web access", enable_web_access),
        ("enable discovery", enable_discovery),
    ];
    let mut values = Vec::new();
    for (name, ptr) in args {
        if ptr.is_null() {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("{name} pointer is null"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
        let value = match CStr::from_ptr(ptr).to_str() {
            Ok(value) => value.to_string(),
            Err(error) => {
                return string_to_ptr(
                    serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("{name} is not valid UTF-8: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        values.push(value);
    }
    match (*handle).startWebAccessServer(
        values[0].clone(),
        values[1].clone(),
        values[2].clone(),
        PathBuf::from(&values[3]),
        match serde_json::from_str::<RemoteDeviceInfo>(&values[4]) {
            Ok(value) => value,
            Err(error) => {
                return string_to_ptr(
                    serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("device info is invalid: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        },
        values[5] == "true",
        values[6] == "true",
    ) {
        Ok(deviceId) => string_to_ptr(
            &serde_json::json!({"ok": true, "deviceId": deviceId}).to_string(),
        ),
        Err(error) => string_to_ptr(
            &serde_json::to_string(&CoreLinkError::internal(error))
                .expect("CoreLinkError must serialize"),
        ),
    }
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_discover_devices(
    handle: *mut OperitFlutterBridge,
    timeout_ms: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if timeout_ms.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::new(
                "INVALID_ARGS",
                "timeout_ms pointer is null",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    let timeout_value = match CStr::from_ptr(timeout_ms).to_str() {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("timeout_ms is not valid UTF-8: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    let timeout: u64 = match timeout_value.parse() {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("timeout_ms is not a valid number: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    match (*handle).discoverDevices(timeout) {
        Ok(json) => string_to_ptr(&json),
        Err(error) => string_to_ptr(
            &serde_json::to_string(&CoreLinkError::internal(error))
                .expect("CoreLinkError must serialize"),
        ),
    }
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_stop_web_access_server(
    handle: *mut OperitFlutterBridge,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    (*handle).stopWebAccessServer();
    string_to_ptr("{\"ok\":true}")
}

/// Decodes and reads one compact native CoreProxy watch snapshot.
fn bridge_watch_snapshot(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request = match decode_native_watch_snapshot_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<CoreEvent, _>(error)),
    };
    native_result_vec(
        handle
            .watchSnapshot(request)
            .map(native_watch_event_payload),
    )
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_remote_pair_start(
    handle: *mut OperitFlutterBridge,
    base_url: *const c_char,
    token_hash: *const c_char,
    client_device_info_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if base_url.is_null() || token_hash.is_null() || client_device_info_json.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::new(
                "INVALID_ARGS",
                "remote pair start expects baseUrl, tokenHash and clientDeviceInfo",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    let baseUrl = match CStr::from_ptr(base_url).to_str() {
        Ok(value) => value.to_string(),
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("baseUrl is not valid UTF-8: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    let tokenHash = match CStr::from_ptr(token_hash).to_str() {
        Ok(value) => value.to_string(),
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("tokenHash is not valid UTF-8: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    let clientDeviceInfoJson = match CStr::from_ptr(client_device_info_json).to_str() {
        Ok(value) => value.to_string(),
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("clientDeviceInfo is not valid UTF-8: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    let clientDeviceInfo = match serde_json::from_str::<RemoteDeviceInfo>(&clientDeviceInfoJson) {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("clientDeviceInfo is invalid: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    match (*handle).remotePairStart(baseUrl, tokenHash, clientDeviceInfo) {
        Ok(value) => string_to_ptr(value),
        Err(error) => string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(error))
                .expect("CoreLinkError must serialize"),
        ),
    }
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_remote_pair_finish(
    handle: *mut OperitFlutterBridge,
    pairing_id: *const c_char,
    pairing_code: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if pairing_id.is_null() || pairing_code.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::new(
                "INVALID_ARGS",
                "remote pair finish expects pairingId and pairingCode",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    let pairingId = match CStr::from_ptr(pairing_id).to_str() {
        Ok(value) => value.to_string(),
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("pairingId is not valid UTF-8: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    let pairingCode = match CStr::from_ptr(pairing_code).to_str() {
        Ok(value) => value.to_string(),
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("pairingCode is not valid UTF-8: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    match (*handle).remotePairFinish(pairingId, pairingCode) {
        Ok(value) => string_to_ptr(value),
        Err(error) => string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(error))
                .expect("CoreLinkError must serialize"),
        ),
    }
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_emit_runtime_event(
    handle: *mut OperitFlutterBridge,
    event_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::json!({
                "ok": false,
                "error": "runtime bridge is not initialized",
            })
            .to_string(),
        );
    }
    if event_json.is_null() {
        return string_to_ptr(
            serde_json::json!({
                "ok": false,
                "error": "runtime event pointer is null",
            })
            .to_string(),
        );
    }
    let eventJson = match CStr::from_ptr(event_json).to_str() {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::json!({
                    "ok": false,
                    "error": format!("runtime event is not valid UTF-8: {error}"),
                })
                .to_string(),
            );
        }
    };
    string_to_ptr((*handle).emitRuntimeEvent(eventJson))
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_free_string(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct OperitFlutterBridgeWasm {
    inner: OperitFlutterBridge,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl OperitFlutterBridgeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<OperitFlutterBridgeWasm, JsValue> {
        console_error_panic_hook::set_once();
        OperitFlutterBridge::new()
            .map(|inner| OperitFlutterBridgeWasm { inner })
            .map_err(|error| JsValue::from_str(&error))
    }

    pub async fn call(&self, request: &[u8]) -> Vec<u8> {
        bridge_native_call_async(&self.inner, request).await
    }

    /// Opens one wasm Link push stream.
    #[allow(non_snake_case)]
    pub fn pushOpen(&self, request: &[u8]) -> Vec<u8> {
        bridge_push_open(&self.inner, request)
    }

    /// Dispatches one wasm Link push item.
    #[allow(non_snake_case)]
    pub async fn pushItem(&self, item: &[u8]) -> Vec<u8> {
        bridge_push_item_async(&self.inner, item).await
    }

    /// Closes one wasm Link push stream.
    #[allow(non_snake_case)]
    pub fn pushClose(&self, pushId: &str) -> Vec<u8> {
        native_result_vec(self.inner.pushClose(pushId))
    }

    #[allow(non_snake_case)]
    pub fn watchSnapshot(&self, request: &[u8]) -> Vec<u8> {
        bridge_watch_snapshot(&self.inner, request)
    }

    #[allow(non_snake_case)]
    pub fn watchStream(&self, request: &[u8], onEvent: Function) -> Vec<u8> {
        bridge_watch_stream_wasm(&self.inner, request, onEvent)
    }

    #[allow(non_snake_case)]
    pub fn closeWatchStream(&self, subscriptionId: &str) -> Vec<u8> {
        self.inner.closeWatchStream(subscriptionId);
        native_result_vec(Ok::<(), CoreLinkError>(()))
    }
}

/// Transfers ownership of a Rust byte vector to the C ABI.
fn bytes_to_buffer(value: Vec<u8>) -> OperitByteBuffer {
    let mut value = value.into_boxed_slice();
    let buffer = OperitByteBuffer {
        ptr: value.as_mut_ptr(),
        len: value.len(),
    };
    std::mem::forget(value);
    buffer
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_free_bytes(value: OperitByteBuffer) {
    if !value.ptr.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            value.ptr, value.len,
        )));
    }
}

fn json_to_ptr(value: &impl serde::Serialize) -> *mut c_char {
    string_to_ptr(json_string(value))
}

fn json_string(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap_or_else(|error| {
        format!(
            "{{\"requestId\":\"flutter-bridge-serialize\",\"result\":{{\"Err\":{{\"code\":\"INTERNAL_ERROR\",\"message\":\"{error}\"}}}}}}"
        )
    })
}

fn string_to_ptr(value: impl Into<String>) -> *mut c_char {
    let sanitized = value.into().replace('\0', "");
    CString::new(sanitized)
        .expect("sanitized bridge string must not contain nul")
        .into_raw()
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
mod android_jni {
    use super::*;
    use jni::objects::{JByteArray, JClass, JObject, JString};
    use jni::sys::{jbyteArray, jlong, jstring};
    use jni::JNIEnv;

    fn jni_bool_arg(env: &mut JNIEnv, value: &JString, name: &str) -> Result<bool, String> {
        let value = env
            .get_string(value)
            .map_err(|error| format!("invalid JNI {name}: {error}"))?;
        let value = value
            .to_str()
            .map_err(|error| format!("invalid JNI {name}: {error}"))?;
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            other => Err(format!(
                "invalid JNI {name}: expected true or false, got {other}"
            )),
        }
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_create(
        mut env: JNIEnv,
        _class: JClass,
        runtime_root: JString,
        workspace_root: JString,
        host: JObject,
    ) -> jlong {
        let runtime_root = match env.get_string(&runtime_root) {
            Ok(value) => PathBuf::from(String::from(value)),
            Err(error) => {
                set_last_create_error(format!("runtime storage root is invalid: {error}"));
                return 0;
            }
        };
        let workspace_root = match env.get_string(&workspace_root) {
            Ok(value) => PathBuf::from(String::from(value)),
            Err(error) => {
                set_last_create_error(format!("workspace storage root is invalid: {error}"));
                return 0;
            }
        };
        let java_vm = match env.get_java_vm() {
            Ok(value) => value,
            Err(error) => {
                set_last_create_error(format!("Android Java VM is unavailable: {error}"));
                return 0;
            }
        };
        let host = match env.new_global_ref(host) {
            Ok(value) => value,
            Err(error) => {
                set_last_create_error(format!("Android host secret bridge is invalid: {error}"));
                return 0;
            }
        };
        if let Err(error) =
            operit_host_android_native::setAndroidHostSecretStoreBridge(java_vm, host)
        {
            set_last_create_error(error.to_string());
            return 0;
        }
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            OperitFlutterBridge::new_with_storage_roots(runtime_root, workspace_root)
        })) {
            Ok(Ok(bridge)) => Box::into_raw(Box::new(bridge)) as jlong,
            Ok(Err(error)) => {
                operit_host_android_native::clearAndroidHostSecretStoreBridge();
                set_last_create_error(error);
                0
            }
            Err(payload) => {
                operit_host_android_native::clearAndroidHostSecretStoreBridge();
                set_last_create_error(format!(
                    "FATAL_CORE_PANIC: {}",
                    panic_payload_message(payload.as_ref())
                ));
                0
            }
        }
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_createError(
        env: JNIEnv,
        _class: JClass,
    ) -> jstring {
        new_java_string(
            env,
            &last_create_error()
                .lock()
                .expect("create error lock")
                .clone(),
        )
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_destroy(
        _env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) {
        operit_flutter_bridge_destroy(handle as *mut OperitFlutterBridge);
        operit_host_android_native::clearAndroidHostSecretStoreBridge();
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_call(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        request: JByteArray,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *const OperitFlutterBridge).as_ref() else {
            return new_java_bytes(
                &mut env,
                &native_result_error_vec(
                    "flutter-bridge-null",
                    "runtime bridge is not initialized",
                ),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &native_result_error_vec(
                        "flutter-bridge-invalid-request",
                        format!("invalid JNI request bytes: {error}"),
                    ),
                );
            }
        };
        new_java_bytes(&mut env, &bridge_native_call(bridge, &bytes))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_pushOpen(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        request: JByteArray,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_bytes(
                &mut env,
                &native_result_error_vec(
                    "flutter-bridge-null",
                    "runtime bridge is not initialized",
                ),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &native_result_error_vec("flutter-bridge-invalid-request", error.to_string()),
                );
            }
        };
        new_java_bytes(&mut env, &bridge_push_open(bridge, &bytes))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_pushItem(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        item: JByteArray,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_bytes(
                &mut env,
                &native_result_error_vec(
                    "flutter-bridge-null",
                    "runtime bridge is not initialized",
                ),
            );
        };
        let bytes = match env.convert_byte_array(item) {
            Ok(value) => value,
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &native_result_error_vec("flutter-bridge-invalid-request", error.to_string()),
                );
            }
        };
        new_java_bytes(&mut env, &bridge_push_item(bridge, &bytes))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_pushClose(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        push_id: JString,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_bytes(
                &mut env,
                &native_result_error_vec(
                    "flutter-bridge-null",
                    "runtime bridge is not initialized",
                ),
            );
        };
        let pushId = match env.get_string(&push_id) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &native_result_error_vec("flutter-bridge-invalid-request", error.to_string()),
                );
            }
        };
        let response = native_result_vec(bridge.pushClose(&pushId));
        new_java_bytes(&mut env, &response)
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_watchSnapshot(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        request: JByteArray,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *const OperitFlutterBridge).as_ref() else {
            return new_java_bytes(
                &mut env,
                &native_result_error_vec(
                    "flutter-bridge-null",
                    "runtime bridge is not initialized",
                ),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &native_result_error_vec(
                        "flutter-bridge-invalid-request",
                        format!("invalid JNI watch request bytes: {error}"),
                    ),
                );
            }
        };
        new_java_bytes(&mut env, &bridge_watch_snapshot(bridge, &bytes))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_watchStream(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        request: JByteArray,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *const OperitFlutterBridge).as_ref() else {
            return new_java_bytes(
                &mut env,
                &native_result_error_vec(
                    "flutter-bridge-null",
                    "runtime bridge is not initialized",
                ),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &native_result_error_vec(
                        "flutter-bridge-invalid-request",
                        format!("invalid JNI watch request bytes: {error}"),
                    ),
                );
            }
        };
        new_java_bytes(&mut env, &bridge_watch_stream(bridge, &bytes))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_nextWatchChannelEvent(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return std::ptr::null_mut();
        };
        match bridge.nextWatchChannelEvent() {
            Ok(frame) => new_java_bytes(&mut env, &frame),
            Err(_) => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_closeWatchStream(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        subscriptionId: JString,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_bytes(
                &mut env,
                &native_result_error_vec(
                    "flutter-bridge-null",
                    "runtime bridge is not initialized",
                ),
            );
        };
        let subscription_id = match env.get_string(&subscriptionId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &native_result_error_vec("flutter-bridge-invalid-request", error.to_string()),
                );
            }
        };
        bridge.closeWatchStream(&subscription_id);
        new_java_bytes(&mut env, &native_result_vec(Ok::<(), CoreLinkError>(())))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_startWebAccessServer(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        bindAddress: JString,
        token: JString,
        shutdownToken: JString,
        webRoot: JString,
        deviceInfoJson: JString,
        enableWebAccess: JString,
        enableDiscovery: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must serialize"),
            );
        };
        let bindAddress = match env.get_string(&bindAddress) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI bindAddress: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let token = match env.get_string(&token) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI token: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let shutdownToken = match env.get_string(&shutdownToken) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI shutdownToken: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let webRoot = match env.get_string(&webRoot) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI webRoot: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let deviceInfoJson = match env.get_string(&deviceInfoJson) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI deviceInfoJson: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let deviceInfo = match serde_json::from_str::<RemoteDeviceInfo>(&deviceInfoJson) {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("deviceInfoJson is invalid: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let enableWebAccess = match jni_bool_arg(&mut env, &enableWebAccess, "enableWebAccess") {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new("INVALID_ARGS", error))
                        .expect("CoreLinkError must serialize"),
                );
            }
        };
        let enableDiscovery = match jni_bool_arg(&mut env, &enableDiscovery, "enableDiscovery") {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new("INVALID_ARGS", error))
                        .expect("CoreLinkError must serialize"),
                );
            }
        };
        match bridge.startWebAccessServer(
            bindAddress,
            token,
            shutdownToken,
            PathBuf::from(webRoot),
            deviceInfo,
            enableWebAccess,
            enableDiscovery,
        ) {
            Ok(deviceId) => new_java_string(
                env,
                &serde_json::json!({"ok": true, "deviceId": deviceId}).to_string(),
            ),
            Err(error) => new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(error))
                    .expect("CoreLinkError must serialize"),
            ),
        }
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_stopWebAccessServer(
        env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must serialize"),
            );
        };
        bridge.stopWebAccessServer();
        new_java_string(env, "{\"ok\":true}")
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_discoverDevices(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        timeoutMs: jlong,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must serialize"),
            );
        };
        let json = bridge
            .discoverDevices(timeoutMs as u64)
            .unwrap_or_else(|e| serde_json::json!({ "error": e }).to_string());
        new_java_string(env, &json)
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_remotePairStart(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        baseUrl: JString,
        tokenHash: JString,
        clientDeviceInfoJson: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must serialize"),
            );
        };
        let baseUrl = match env.get_string(&baseUrl) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI baseUrl: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let tokenHash = match env.get_string(&tokenHash) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI tokenHash: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let clientDeviceInfoJson = match env.get_string(&clientDeviceInfoJson) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI clientDeviceInfoJson: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let clientDeviceInfo = match serde_json::from_str::<RemoteDeviceInfo>(&clientDeviceInfoJson)
        {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("clientDeviceInfoJson is invalid: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        match bridge.remotePairStart(baseUrl, tokenHash, clientDeviceInfo) {
            Ok(value) => new_java_string(env, &value),
            Err(error) => new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(error))
                    .expect("CoreLinkError must serialize"),
            ),
        }
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_remotePairFinish(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        pairingId: JString,
        pairingCode: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must serialize"),
            );
        };
        let pairingId = match env.get_string(&pairingId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI pairingId: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let pairingCode = match env.get_string(&pairingCode) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI pairingCode: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        match bridge.remotePairFinish(pairingId, pairingCode) {
            Ok(value) => new_java_string(env, &value),
            Err(error) => new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(error))
                    .expect("CoreLinkError must serialize"),
            ),
        }
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_emitRuntimeEvent(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        eventJson: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(env, &serde_json::json!({"ok": false}).to_string());
        };
        let eventJson = match env.get_string(&eventJson) {
            Ok(value) => String::from(value),
            Err(_) => return new_java_string(env, &serde_json::json!({"ok": false}).to_string()),
        };
        new_java_string(env, &bridge.emitRuntimeEvent(&eventJson))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_emitHostRuntimeEventSchedule(
        env: JNIEnv,
        _class: JClass,
        _handle: jlong,
        scheduleId: JString,
        scheduledAtMillis: jlong,
        firedAtMillis: jlong,
    ) -> jstring {
        let mut env = env;
        let scheduleId = match env.get_string(&scheduleId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid JNI scheduleId: {error}"),
                    })
                    .to_string(),
                );
            }
        };
        let fire = operit_host_api::HostRuntimeEventScheduleFire {
            scheduleId,
            scheduledAtMillis: scheduledAtMillis as u64,
            firedAtMillis: firedAtMillis as u64,
        };
        match operit_host_android_native::emitAndroidHostRuntimeEventSchedule(fire) {
            Ok(()) => new_java_string(env, &serde_json::json!({"ok": true}).to_string()),
            Err(error) => new_java_string(
                env,
                &serde_json::json!({"ok": false, "error": error.to_string()}).to_string(),
            ),
        }
    }

    fn new_java_string(mut env: JNIEnv, value: &str) -> jstring {
        env.new_string(value)
            .expect("JNI string allocation must succeed")
            .into_raw()
    }

    /// Allocates a Java byte array containing one encoded Link payload.
    fn new_java_bytes(env: &mut JNIEnv, value: &[u8]) -> jbyteArray {
        env.byte_array_from_slice(value)
            .expect("JNI byte array allocation must succeed")
            .into_raw()
    }
}
