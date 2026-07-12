#![allow(non_snake_case)]

use std::collections::{hash_map::Entry, BTreeMap, HashMap};
use std::ffi::{c_char, CStr, CString};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use operit_core_proxy::LocalCoreProxy;
#[cfg(not(target_arch = "wasm32"))]
mod access;

#[cfg(not(target_arch = "wasm32"))]
mod mdnss;

#[cfg(not(target_arch = "wasm32"))]
use access::{
    link_token_hash, AcceptedRemoteSessionLoader, AcceptedRemoteSessionRecord,
    AcceptedRemoteSessionStore, PairStartState, RemoteDeviceInfo, RemoteLinkClient,
    RemoteLinkServer, RemoteLinkServerConfig, RemotePairingCodeRecord, RemotePairingCodeSink,
    RemoteWebAccessConfig,
};
use operit_host_api::HostManager::HostManager;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkSharedClient, CorePushItem, CorePushRequest, CoreRequestId,
    CoreWatchRequest,
};
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerAudioPlay, requestOwnerBluetooth, requestOwnerBrowserAutomation,
    requestOwnerBrowserSession, requestOwnerComposeWebViewController, requestOwnerMusicPlayback,
    requestOwnerSystemCaptureScreenshot, requestOwnerSystemLanguageCode,
    requestOwnerSystemRecognizeText, requestOwnerToolPermission, requestOwnerTtsPlayback,
    requestOwnerTtsSynthesis, requestOwnerWebVisit, RuntimeHostInteractionAudioPlayPayload,
    RuntimeHostInteractionBluetoothPayload, RuntimeHostInteractionBrowserAutomationPayload,
    RuntimeHostInteractionBrowserSessionPayload,
    RuntimeHostInteractionComposeWebViewControllerPayload,
    RuntimeHostInteractionMusicPlaybackPayload, RuntimeHostInteractionSystemRecognizeTextPayload,
    RuntimeHostInteractionToolPermissionPayload, RuntimeHostInteractionToolPermissionTool,
    RuntimeHostInteractionToolPermissionToolParameter, RuntimeHostInteractionTtsPlaybackPayload,
    RuntimeHostInteractionTtsSynthesisPayload, RuntimeHostInteractionWebVisitHeader,
    RuntimeHostInteractionWebVisitPayload,
};
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_tools::ToolExecutionManager::AITool;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(target_os = "android")]
use operit_host_android_native::{
    AndroidAudioPlaybackHost as NativeAudioPlaybackHost,
    AndroidBluetoothHost as NativeBluetoothHost, AndroidFileSystemHost as NativeFileSystemHost,
    AndroidHttpHost as NativeHttpHost, AndroidManagedRuntimeHost as NativeManagedRuntimeHost,
    AndroidMusicCommand as NativeMusicCommand,
    AndroidRuntimeStorageHost as NativeRuntimeStorageHost,
    AndroidSystemOperationHost as NativeSystemOperationHost,
    AndroidTerminalHost as NativeTerminalHost, AndroidTtsPlaybackHost as NativeTtsPlaybackHost,
    AndroidTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(target_os = "android")]
use operit_host_api::SystemOperationHost;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use operit_host_apple_native::{
    AppleAudioPlaybackHost as NativeAudioPlaybackHost, AppleBluetoothHost as NativeBluetoothHost,
    AppleFileSystemHost as NativeFileSystemHost, AppleHttpHost as NativeHttpHost,
    AppleManagedRuntimeHost as NativeManagedRuntimeHost, AppleMusicCommand as NativeMusicCommand,
    AppleRuntimeStorageHost as NativeRuntimeStorageHost,
    AppleSystemOperationHost as NativeSystemOperationHost,
    AppleTtsPlaybackHost as NativeTtsPlaybackHost, AppleTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(target_os = "macos")]
use operit_host_apple_native::{
    AppleHostRuntimeEventHost as NativeHostRuntimeEventHost,
    AppleTerminalHost as NativeTerminalHost,
};
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxAudioPlaybackHost as NativeAudioPlaybackHost, LinuxBluetoothHost as NativeBluetoothHost,
    LinuxFileSystemHost as NativeFileSystemHost,
    LinuxHostRuntimeEventHost as NativeHostRuntimeEventHost, LinuxHttpHost as NativeHttpHost,
    LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxRuntimeStorageHost as NativeRuntimeStorageHost,
    LinuxSystemOperationHost as NativeSystemOperationHost, LinuxTerminalHost as NativeTerminalHost,
};
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxTtsPlaybackHost as NativeTtsPlaybackHost, LinuxTtsSynthesisHost as NativeTtsSynthesisHost,
};
#[cfg(target_os = "ohos")]
use operit_host_ohos_native::{
    newOhosFileSystemHost, OhosHttpHost as NativeHttpHost,
    OhosRuntimeStorageHost as NativeRuntimeStorageHost, OhosTerminalHost as NativeTerminalHost,
};
#[cfg(target_arch = "wasm32")]
use operit_host_web::{
    WebAudioPlaybackHost as NativeAudioPlaybackHost, WebBluetoothHost as NativeBluetoothHost,
    WebFileSystemHost as NativeFileSystemHost, WebHttpHost as NativeHttpHost,
    WebManagedRuntimeHost as NativeManagedRuntimeHost,
    WebRuntimeStorageHost as NativeRuntimeStorageHost,
    WebSystemOperationHost as NativeSystemOperationHost,
    WebTtsPlaybackHost as NativeTtsPlaybackHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsAudioPlaybackHost as NativeAudioPlaybackHost,
    WindowsBluetoothHost as NativeBluetoothHost, WindowsFileSystemHost as NativeFileSystemHost,
    WindowsHostRuntimeEventHost as NativeHostRuntimeEventHost, WindowsHttpHost as NativeHttpHost,
    WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
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

pub struct OperitFlutterBridge {
    #[cfg(not(target_arch = "wasm32"))]
    runtime: tokio::runtime::Runtime,
    proxyCore: Arc<LocalCoreProxy>,
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
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ohos"
    ))]
    terminalHost: Arc<NativeTerminalHost>,
}

struct NativePushState {
    request: CorePushRequest,
    nextSequence: u64,
}

#[derive(Debug, Deserialize)]
struct FlutterWatchChannelOpenEnvelope {
    #[allow(dead_code)]
    channelId: String,
    subscriptionId: String,
    request: CoreWatchRequest,
}

#[derive(Debug, Serialize)]
struct FlutterPushOpenResponse {
    pushId: String,
}

#[derive(Debug, Serialize)]
struct FlutterPushItemResponse {
    pushId: String,
    sequence: u64,
}

#[derive(Debug, Serialize)]
struct FlutterWatchChannelOpenResponse {
    subscriptionId: String,
}

#[derive(Debug, Serialize)]
struct FlutterWatchChannelEvent {
    subscriptionId: String,
    event: CoreEvent,
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
        let response = requestOwnerSystemLanguageCode(Duration::from_secs(60))
            .map_err(operit_host_api::HostError::new)?;
        Ok(response.languageCode)
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
    fn new() -> Result<Self, String> {
        let (runtime_root, workspace_root) = default_native_storage_roots()?;
        Self::new_with_storage_roots(runtime_root, workspace_root)
    }

    /// Creates a bridge using caller-supplied runtime and workspace roots.
    fn new_with_storage_roots(
        runtime_root: PathBuf,
        workspace_root: PathBuf,
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
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ohos"
        ))]
        let terminalHost = Arc::new(NativeTerminalHost::new());
        let mut core = create_local_core(
            runtime_root,
            workspace_root,
            Arc::new(webVisitBridge),
            Some(Arc::new(browserAutomationBridge)),
            Some(Arc::new(browserSessionBridge)),
            Some(Arc::new(composeDslWebViewBridge)),
            #[cfg(any(
                windows,
                target_os = "linux",
                target_os = "android",
                target_os = "macos",
                target_os = "ohos"
            ))]
            terminalHost.clone(),
        )?;
        core.localApplicationMut().onCreate()?;
        install_permission_requester(&mut core);
        Ok(Self {
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
            proxyCore: Arc::new(core),
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
                target_os = "linux",
                target_os = "android",
                target_os = "macos",
                target_os = "ohos"
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
    fn pushItem(&self, item: CorePushItem) -> Result<FlutterPushItemResponse, CoreLinkError> {
        let request = self.takePushItemRequest(&item)?;
        let response = self.call(request.itemCall(item.sequence, item.args));
        response.result?;
        Ok(FlutterPushItemResponse {
            pushId: item.pushId,
            sequence: item.sequence,
        })
    }

    /// Dispatches one wasm push item in stream order.
    #[cfg(target_arch = "wasm32")]
    async fn pushItem(&self, item: CorePushItem) -> Result<FlutterPushItemResponse, CoreLinkError> {
        let request = self.takePushItemRequest(&item)?;
        let response = self.call(request.itemCall(item.sequence, item.args)).await;
        response.result?;
        Ok(FlutterPushItemResponse {
            pushId: item.pushId,
            sequence: item.sequence,
        })
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
                channel.send(
                    operit_link::encodeLink(&FlutterWatchChannelEvent {
                        subscriptionId: taskSubscriptionId.clone(),
                        event,
                    })
                    .expect("Flutter watch channel event must encode"),
                );
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
                        let frame = operit_link::encodeLink(&FlutterWatchChannelEvent {
                            subscriptionId: taskSubscriptionId.clone(),
                            event,
                        })
                        .expect("Flutter watch channel event must encode");
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
            "services.runtimeEventIngressService",
            "ingestEvent",
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
        serde_json::to_string(&session.exportRecord()).map_err(|error| error.to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn startWebAccessServer(
        &self,
        bindAddress: String,
        token: String,
        shutdownToken: String,
        webRoot: PathBuf,
        deviceId: String,
        acceptedSessionsJson: String,
        acceptedSessionStorePath: PathBuf,
        pairingCodePath: PathBuf,
        deviceInfo: RemoteDeviceInfo,
        enableWebAccess: bool,
        enableDiscovery: bool,
    ) -> Result<(), String> {
        self.stopWebAccessServer();
        let acceptedSessions =
            serde_json::from_str::<BTreeMap<String, AcceptedRemoteSessionRecord>>(
                &acceptedSessionsJson,
            )
            .map_err(|error| format!("invalid accepted sessions JSON: {error}"))?;
        let acceptedSessionLoaderPath = acceptedSessionStorePath.clone();
        let acceptedSessionLoader: AcceptedRemoteSessionLoader =
            Arc::new(move || load_accepted_remote_sessions(&acceptedSessionLoaderPath));
        let acceptedSessionStore: AcceptedRemoteSessionStore =
            Arc::new(move |sessionId, record| {
                save_accepted_remote_session(&acceptedSessionStorePath, sessionId, record)
            });
        let pairingCodeSink: RemotePairingCodeSink =
            Arc::new(move |record| save_remote_pairing_code(&pairingCodePath, record));
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
            runtimeHandle: self.runtime.handle().clone(),
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
                        })
                    } else {
                        None
                    },
                    printStartupInfo: false,
                    acceptedSessions,
                    acceptedSessionLoader: Some(acceptedSessionLoader),
                    acceptedSessionStore: Some(acceptedSessionStore),
                    pairingCodeSink: Some(pairingCodeSink),
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
        Ok(())
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

#[cfg(not(target_arch = "wasm32"))]
fn load_accepted_remote_sessions(
    path: &PathBuf,
) -> Result<BTreeMap<String, AcceptedRemoteSessionRecord>, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn save_accepted_remote_session(
    path: &PathBuf,
    sessionId: String,
    record: AcceptedRemoteSessionRecord,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid accepted session path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut sessions = load_accepted_remote_sessions(path)?;
    sessions.insert(sessionId, record);
    let content = serde_json::to_string_pretty(&sessions).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn save_remote_pairing_code(path: &PathBuf, record: RemotePairingCodeRecord) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid remote pairing code path: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let content = serde_json::to_string_pretty(&record).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct SharedFlutterCoreClient {
    proxyCore: Arc<LocalCoreProxy>,
    runtimeHandle: tokio::runtime::Handle,
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl CoreLinkClient for SharedFlutterCoreClient {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let proxyCore = self.proxyCore.clone();
        let runtimeHandle = self.runtimeHandle.clone();
        match tokio::task::spawn_blocking(move || {
            Ok::<CoreCallResponse, CoreLinkError>(
                runtimeHandle.block_on(CoreLinkSharedClient::call(proxyCore.as_ref(), request)),
            )
        })
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => CoreCallResponse::err(requestId, error),
            Err(error) => CoreCallResponse::err(
                requestId,
                CoreLinkError::internal(format!("core call task join failed: {error}")),
            ),
        }
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let proxyCore = self.proxyCore.clone();
        let runtimeHandle = self.runtimeHandle.clone();
        tokio::task::spawn_blocking(move || {
            runtimeHandle.block_on(CoreLinkSharedClient::watchSnapshot(
                proxyCore.as_ref(),
                request,
            ))
        })
        .await
        .map_err(|error| {
            CoreLinkError::internal(format!("core watch snapshot task join failed: {error}"))
        })?
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let proxyCore = self.proxyCore.clone();
        let runtimeHandle = self.runtimeHandle.clone();
        tokio::task::spawn_blocking(move || {
            runtimeHandle.block_on(CoreLinkSharedClient::watch(proxyCore.as_ref(), request))
        })
        .await
        .map_err(|error| CoreLinkError::internal(format!("core watch task join failed: {error}")))?
    }
}

fn install_permission_requester(core: &mut LocalCoreProxy) {
    let handler = core.localApplicationMut().toolHandler.clone();
    handler
        .getToolPermissionSystem()
        .setPermissionRequester(move |tool, description| {
            let response = requestOwnerToolPermission(
                RuntimeHostInteractionToolPermissionPayload {
                    tool: tool_to_permission_payload(tool),
                    description: description.to_string(),
                },
                Duration::from_millis(PERMISSION_REQUEST_TIMEOUT_MS),
            );
            let response = response.expect("permission request failed");
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

#[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
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

#[cfg(any(
    windows,
    target_os = "linux",
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
                        paramsJson: params.to_string(),
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
        context = context.withTtsSynthesisHost(Arc::new(NativeTtsSynthesisHost::fromSynthesizer(
            Arc::new(|request| {
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
            }),
        )));
        context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::fromController(
            Arc::new(|command| {
                let payload = match command.command.as_str() {
                    "speak" => {
                        let request = match command.request {
                            Some(request) => request,
                            None => {
                                return Err(operit_host_api::HostError::new(
                                    "tts speak request is required",
                                ))
                            }
                        };
                        RuntimeHostInteractionTtsPlaybackPayload {
                            command: command.command,
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
                        )))
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
    #[cfg(target_os = "linux")]
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

#[cfg(target_os = "ohos")]
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
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(runtime_root, workspace_root));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let mut context = HostManager::withFileSystemAndWebVisitHosts(
        Arc::new(newOhosFileSystemHost()),
        webVisitHost,
    );
    context.httpHost = Some(Arc::new(NativeHttpHost::new()));
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
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(any(windows, target_os = "linux", target_os = "ios", target_os = "macos"))]
fn default_native_storage_roots() -> Result<(PathBuf, PathBuf), String> {
    Ok((
        NativeRuntimeStorageHost::defaultRuntimeRoot(),
        NativeRuntimeStorageHost::defaultWorkspaceRoot(),
    ))
}

#[cfg(target_os = "ohos")]
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
                    paramsJson: params.to_string(),
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
    context = context.withTtsSynthesisHost(Arc::new(NativeTtsSynthesisHost::fromSynthesizer(
        Arc::new(|request| {
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
        }),
    )));
    context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::fromController(
        Arc::new(|command| {
            let payload = match command.command.as_str() {
                "speak" => {
                    let request = match command.request {
                        Some(request) => request,
                        None => {
                            return Err(operit_host_api::HostError::new(
                                "tts speak request is required",
                            ))
                        }
                    };
                    RuntimeHostInteractionTtsPlaybackPayload {
                        command: command.command,
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
                    )))
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
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(target_arch = "wasm32")]
fn create_local_core(
    _runtime_root: PathBuf,
    _workspace_root: PathBuf,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
) -> Result<LocalCoreProxy, String> {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new());
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
    context = context.withAudioPlaybackHost(Arc::new(NativeAudioPlaybackHost::new()));
    context = context.withBluetoothHost(Arc::new(NativeBluetoothHost::new()));
    context = context.withTtsPlaybackHost(Arc::new(NativeTtsPlaybackHost::new()));
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(not(any(
    windows,
    target_os = "linux",
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_os = "ohos",
    target_arch = "wasm32"
)))]
fn create_local_core(
    _runtime_root: PathBuf,
    _workspace_root: PathBuf,
    _webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    _browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    _browserSessionHost: Option<Arc<dyn operit_host_api::BrowserSessionHost>>,
    _composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    #[cfg(any(windows, target_os = "linux", target_os = "android"))] _terminalHost: Arc<
        NativeTerminalHost,
    >,
) -> Result<LocalCoreProxy, String> {
    Err("operit flutter native runtime bridge is not available for this target".to_string())
}

#[no_mangle]
pub extern "C" fn operit_flutter_bridge_create() -> *mut OperitFlutterBridge {
    match OperitFlutterBridge::new() {
        Ok(bridge) => Box::into_raw(Box::new(bridge)),
        Err(error) => {
            set_last_create_error(error);
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
    match OperitFlutterBridge::new_with_storage_roots(runtime_root, workspace_root) {
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

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_call(
    handle: *mut OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> OperitByteBuffer {
    if handle.is_null() {
        return call_error_bytes("flutter-bridge-null", "runtime bridge is not initialized");
    }
    if request_ptr.is_null() {
        return call_error_bytes(
            "flutter-bridge-null-request",
            "runtime request pointer is null",
        );
    }
    let request_bytes = std::slice::from_raw_parts(request_ptr, request_len);
    bytes_to_buffer(bridge_call(&mut *handle, request_bytes))
}

#[cfg(not(target_arch = "wasm32"))]
fn bridge_call(handle: &mut OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request: CoreCallRequest = match operit_link::decodeLink(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return call_error_vec(
                "flutter-bridge-invalid-request",
                format!("invalid core request: {error}"),
            );
        }
    };
    let response = handle.call(request);
    operit_link::encodeLink(&response).expect("CoreCallResponse must encode")
}

#[cfg(target_arch = "wasm32")]
async fn bridge_call_async(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request: CoreCallRequest = match operit_link::decodeLink(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return call_error_vec(
                "flutter-bridge-invalid-request",
                format!("invalid core request: {error}"),
            );
        }
    };
    let response = handle.call(request).await;
    operit_link::encodeLink(&response).expect("CoreCallResponse must encode")
}

/// Decodes and opens one local Link push stream.
fn bridge_push_open(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request = match operit_link::decodeLink::<CorePushRequest>(request_bytes) {
        Ok(value) => value,
        Err(error) => {
            return operit_link::encodeLink(CoreLinkError::internal(format!(
                "invalid core push request: {error}"
            )))
            .expect("CoreLinkError must encode");
        }
    };
    match handle.pushOpen(request) {
        Ok(pushId) => operit_link::encodeLink(FlutterPushOpenResponse { pushId })
            .expect("FlutterPushOpenResponse must encode"),
        Err(error) => operit_link::encodeLink(error).expect("CoreLinkError must encode"),
    }
}

/// Decodes and dispatches one native Link push item.
#[cfg(not(target_arch = "wasm32"))]
fn bridge_push_item(handle: &OperitFlutterBridge, item_bytes: &[u8]) -> Vec<u8> {
    let item = match operit_link::decodeLink::<CorePushItem>(item_bytes) {
        Ok(value) => value,
        Err(error) => {
            return operit_link::encodeLink(CoreLinkError::internal(format!(
                "invalid core push item: {error}"
            )))
            .expect("CoreLinkError must encode");
        }
    };
    match handle.pushItem(item) {
        Ok(response) => {
            operit_link::encodeLink(response).expect("FlutterPushItemResponse must encode")
        }
        Err(error) => operit_link::encodeLink(error).expect("CoreLinkError must encode"),
    }
}

/// Decodes and dispatches one wasm Link push item.
#[cfg(target_arch = "wasm32")]
async fn bridge_push_item_async(handle: &OperitFlutterBridge, item_bytes: &[u8]) -> Vec<u8> {
    let item = match operit_link::decodeLink::<CorePushItem>(item_bytes) {
        Ok(value) => value,
        Err(error) => {
            return operit_link::encodeLink(CoreLinkError::internal(format!(
                "invalid core push item: {error}"
            )))
            .expect("CoreLinkError must encode");
        }
    };
    match handle.pushItem(item).await {
        Ok(response) => {
            operit_link::encodeLink(response).expect("FlutterPushItemResponse must encode")
        }
        Err(error) => operit_link::encodeLink(error).expect("CoreLinkError must encode"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_push_open(
    handle: *mut OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> OperitByteBuffer {
    if handle.is_null() || request_ptr.is_null() {
        return link_error_buffer("runtime push open arguments are invalid");
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
        return link_error_buffer("runtime push item arguments are invalid");
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
        return link_error_buffer("runtime push close arguments are invalid");
    }
    let pushId = match CStr::from_ptr(push_id_ptr).to_str() {
        Ok(value) => value,
        Err(error) => return link_error_buffer(error.to_string()),
    };
    match (*handle).pushClose(pushId) {
        Ok(()) => bytes_to_buffer(
            operit_link::encodeLink(BTreeMap::<String, operit_link::CoreValue>::new())
                .expect("push close response must encode"),
        ),
        Err(error) => {
            bytes_to_buffer(operit_link::encodeLink(error).expect("CoreLinkError must encode"))
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_watch_snapshot(
    handle: *mut OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> OperitByteBuffer {
    if handle.is_null() {
        return link_error_buffer("runtime bridge is not initialized");
    }
    if request_ptr.is_null() {
        return link_error_buffer("runtime request pointer is null");
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
        return link_error_buffer("runtime bridge is not initialized");
    }
    if request_ptr.is_null() {
        return link_error_buffer("runtime request pointer is null");
    }
    let request_bytes = std::slice::from_raw_parts(request_ptr, request_len);
    bytes_to_buffer(bridge_watch_stream(&mut *handle, request_bytes))
}

#[cfg(not(target_arch = "wasm32"))]
fn bridge_watch_stream(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let envelope: FlutterWatchChannelOpenEnvelope = match operit_link::decodeLink(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return operit_link::encodeLink(CoreLinkError::internal(format!(
                "invalid core watch open envelope: {error}"
            )))
            .expect("CoreLinkError must encode");
        }
    };
    match handle.watchStream(envelope.subscriptionId, envelope.request) {
        Ok(subscriptionId) => {
            operit_link::encodeLink(&FlutterWatchChannelOpenResponse { subscriptionId })
                .expect("FlutterWatchChannelOpenResponse must encode")
        }
        Err(error) => operit_link::encodeLink(&error).expect("CoreLinkError must encode"),
    }
}

#[cfg(target_arch = "wasm32")]
fn bridge_watch_stream_wasm(
    handle: &OperitFlutterBridge,
    request_bytes: &[u8],
    onEvent: Function,
) -> Vec<u8> {
    let envelope: FlutterWatchChannelOpenEnvelope = match operit_link::decodeLink(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return operit_link::encodeLink(CoreLinkError::internal(format!(
                "invalid core watch open envelope: {error}"
            )))
            .expect("CoreLinkError must encode");
        }
    };
    match handle.watchStream(envelope.subscriptionId, envelope.request, onEvent) {
        Ok(subscriptionId) => {
            operit_link::encodeLink(&FlutterWatchChannelOpenResponse { subscriptionId })
                .expect("FlutterWatchChannelOpenResponse must encode")
        }
        Err(error) => operit_link::encodeLink(&error).expect("CoreLinkError must encode"),
    }
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
        return link_error_buffer("runtime bridge is not initialized");
    }
    if subscription_ptr.is_null() {
        return link_error_buffer("watch subscription pointer is null");
    }
    if let Ok(subscriptionId) = CStr::from_ptr(subscription_ptr).to_str() {
        (*handle).closeWatchStream(subscriptionId);
    }
    bytes_to_buffer(
        operit_link::encodeLink(
            std::collections::BTreeMap::<String, operit_link::CoreValue>::new(),
        )
        .expect("close response must encode"),
    )
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_start_web_access_server(
    handle: *mut OperitFlutterBridge,
    bind_address: *const c_char,
    token: *const c_char,
    shutdown_token: *const c_char,
    web_root: *const c_char,
    device_id: *const c_char,
    accepted_sessions_json: *const c_char,
    accepted_session_store_path: *const c_char,
    pairing_code_path: *const c_char,
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
        ("device id", device_id),
        ("accepted sessions", accepted_sessions_json),
        ("accepted session store path", accepted_session_store_path),
        ("pairing code path", pairing_code_path),
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
        values[4].clone(),
        values[5].clone(),
        PathBuf::from(&values[6]),
        PathBuf::from(&values[7]),
        match serde_json::from_str::<RemoteDeviceInfo>(&values[8]) {
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
        values[9] == "true",
        values[10] == "true",
    ) {
        Ok(()) => string_to_ptr("{\"ok\":true}"),
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

fn bridge_watch_snapshot(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request: CoreWatchRequest = match operit_link::decodeLink(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return operit_link::encodeLink(CoreLinkError::internal(format!(
                "invalid core watch request: {error}"
            )))
            .expect("CoreLinkError must encode");
        }
    };
    match handle.watchSnapshot(request) {
        Ok(event) => operit_link::encodeLink(&event).expect("CoreEvent must encode"),
        Err(error) => operit_link::encodeLink(&error).expect("CoreLinkError must encode"),
    }
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
        bridge_call_async(&self.inner, request).await
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
        match self.inner.pushClose(pushId) {
            Ok(()) => operit_link::encodeLink(BTreeMap::<String, operit_link::CoreValue>::new())
                .expect("push close response must encode"),
            Err(error) => operit_link::encodeLink(error).expect("CoreLinkError must encode"),
        }
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
        operit_link::encodeLink(std::collections::BTreeMap::<String, operit_link::CoreValue>::new())
            .expect("close response must encode")
    }
}

/// Encodes a failed call response into an owned native byte buffer.
fn call_error_bytes(requestId: impl Into<String>, message: impl Into<String>) -> OperitByteBuffer {
    bytes_to_buffer(call_error_vec(requestId, message))
}

/// Encodes a failed call response into Link bytes.
fn call_error_vec(requestId: impl Into<String>, message: impl Into<String>) -> Vec<u8> {
    let response = CoreCallResponse::err(
        CoreRequestId::new(requestId),
        CoreLinkError::internal(message.into()),
    );
    operit_link::encodeLink(response).expect("CoreCallResponse must encode")
}

/// Encodes a Link error into an owned native byte buffer.
fn link_error_buffer(message: impl Into<String>) -> OperitByteBuffer {
    bytes_to_buffer(link_error_vec(message))
}

/// Encodes an internal Link error into protocol bytes.
fn link_error_vec(message: impl Into<String>) -> Vec<u8> {
    operit_link::encodeLink(CoreLinkError::internal(message.into()))
        .expect("CoreLinkError must encode")
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
        match OperitFlutterBridge::new_with_storage_roots(runtime_root, workspace_root) {
            Ok(bridge) => Box::into_raw(Box::new(bridge)) as jlong,
            Err(error) => {
                operit_host_android_native::clearAndroidHostSecretStoreBridge();
                set_last_create_error(error);
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
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_mut() else {
            return new_java_bytes(
                &mut env,
                &call_error_vec("flutter-bridge-null", "runtime bridge is not initialized"),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &call_error_vec(
                        "flutter-bridge-invalid-request",
                        format!("invalid JNI request bytes: {error}"),
                    ),
                );
            }
        };
        new_java_bytes(&mut env, &bridge_call(bridge, &bytes))
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
                &link_error_vec("runtime bridge is not initialized"),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => return new_java_bytes(&mut env, &link_error_vec(error.to_string())),
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
                &link_error_vec("runtime bridge is not initialized"),
            );
        };
        let bytes = match env.convert_byte_array(item) {
            Ok(value) => value,
            Err(error) => return new_java_bytes(&mut env, &link_error_vec(error.to_string())),
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
                &link_error_vec("runtime bridge is not initialized"),
            );
        };
        let pushId = match env.get_string(&push_id) {
            Ok(value) => String::from(value),
            Err(error) => return new_java_bytes(&mut env, &link_error_vec(error.to_string())),
        };
        let response = match bridge.pushClose(&pushId) {
            Ok(()) => operit_link::encodeLink(BTreeMap::<String, operit_link::CoreValue>::new())
                .expect("push close response must encode"),
            Err(error) => operit_link::encodeLink(error).expect("CoreLinkError must encode"),
        };
        new_java_bytes(&mut env, &response)
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_watchSnapshot(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        request: JByteArray,
    ) -> jbyteArray {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_mut() else {
            return new_java_bytes(
                &mut env,
                &operit_link::encodeLink(CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must encode"),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &operit_link::encodeLink(CoreLinkError::internal(format!(
                        "invalid JNI watch request bytes: {error}"
                    )))
                    .expect("CoreLinkError must encode"),
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
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_mut() else {
            return new_java_bytes(
                &mut env,
                &operit_link::encodeLink(CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must encode"),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_bytes(
                    &mut env,
                    &operit_link::encodeLink(CoreLinkError::internal(format!(
                        "invalid JNI watch request bytes: {error}"
                    )))
                    .expect("CoreLinkError must encode"),
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
        if let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() {
            if let Ok(subscriptionId) = env.get_string(&subscriptionId) {
                bridge.closeWatchStream(&String::from(subscriptionId));
            }
        }
        new_java_bytes(
            &mut env,
            &operit_link::encodeLink(
                std::collections::BTreeMap::<String, operit_link::CoreValue>::new(),
            )
            .expect("close response must encode"),
        )
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
        deviceId: JString,
        acceptedSessionsJson: JString,
        acceptedSessionStorePath: JString,
        pairingCodePath: JString,
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
        let deviceId = match env.get_string(&deviceId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI deviceId: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let acceptedSessionsJson = match env.get_string(&acceptedSessionsJson) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI acceptedSessionsJson: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let acceptedSessionStorePath = match env.get_string(&acceptedSessionStorePath) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI acceptedSessionStorePath: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let pairingCodePath = match env.get_string(&pairingCodePath) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("invalid JNI pairingCodePath: {error}"),
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
            deviceId,
            acceptedSessionsJson,
            PathBuf::from(acceptedSessionStorePath),
            PathBuf::from(pairingCodePath),
            deviceInfo,
            enableWebAccess,
            enableDiscovery,
        ) {
            Ok(()) => new_java_string(env, "{\"ok\":true}"),
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
