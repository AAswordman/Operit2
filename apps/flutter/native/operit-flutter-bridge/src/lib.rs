#![allow(non_snake_case)]

mod BridgeCodec;
mod BridgeExports;
mod BridgeTransport;
mod FlutterHostAdapters;
mod PlatformRuntimeFactory;

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
use std::collections::{hash_map::Entry, BTreeMap, HashMap};
use std::ffi::{c_char, CStr, CString};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use operit_core_proxy::{
    LocalCoreProxy, RuntimeCoreRouter::RuntimeCorePushTarget, RuntimeCoreRouter::RuntimeCoreRouter,
};
#[cfg(not(target_arch = "wasm32"))]
mod mdnss;

use operit_host_api::HostManager::HostManager;
use operit_host_api::RuntimeStorageHost;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkSharedClient, CorePushItem, CorePushRequest, CoreWatchRequest,
};
#[cfg(not(target_arch = "wasm32"))]
use operit_link_access::{
    link_token_hash, LinkAccessHostConfig, LinkAccessHostPortMode, LinkAccessStore,
    RemoteDeviceInfo, RemoteLinkServer, RemoteLinkServerConfig, RemoteWebAccessConfig,
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
#[cfg(target_os = "android")]
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
    pub(crate) proxyCore: Arc<RuntimeCoreRouter>,
    runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) watchChannel: NativeWatchChannel,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) watchSubscriptions: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    #[cfg(target_arch = "wasm32")]
    pub(crate) watchSubscriptions: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
    pub(crate) pushStreams: Mutex<HashMap<String, NativePushState>>,
    #[cfg(not(target_arch = "wasm32"))]
    webAccessTask: Mutex<Option<tokio::task::JoinHandle<Result<(), String>>>>,
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

const PERMISSION_REQUEST_TIMEOUT_MS: u64 = 60_000;

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
        let localCore = Arc::new(core);
        let proxyCore = Arc::new(RuntimeCoreRouter::new(localCore.clone()));
        Ok(Self {
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
            localCore,
            proxyCore,
            runtimeStorageHost,
            #[cfg(not(target_arch = "wasm32"))]
            watchChannel: NativeWatchChannel::new(),
            #[cfg(not(target_arch = "wasm32"))]
            watchSubscriptions: Mutex::new(HashMap::new()),
            #[cfg(target_arch = "wasm32")]
            watchSubscriptions: Arc::new(Mutex::new(HashMap::new())),
            pushStreams: Mutex::new(HashMap::new()),
            #[cfg(not(target_arch = "wasm32"))]
            webAccessTask: Mutex::new(None),
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
    /// Reads the Link Access store owned by the local Core Runtime.
    fn linkAccessStore(&self) -> LinkAccessStore {
        LinkAccessStore::new(self.runtimeStorageHost.clone())
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Calls the local Core runtime.
    fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.runtime
            .block_on(CoreLinkSharedClient::call(self.proxyCore.as_ref(), request))
    }

    #[cfg(target_arch = "wasm32")]
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self.proxyCore.as_ref(), request).await
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
        accessStore.saveHostConfig(LinkAccessHostConfig {
            bindAddress: bindAddress.clone(),
            token: token.clone(),
            webAccessEnabled: enableWebAccess,
            discoveryEnabled: enableDiscovery,
            portMode: LinkAccessHostPortMode::Fixed,
            updatedAt: current_time_millis_u64() as i64,
        })?;
        let deviceId = identity.deviceId;
        let deviceInfo = identity.deviceInfo;
        let responseDeviceId = deviceId.clone();
        let runtimeStorageHost = self.runtimeStorageHost.clone();
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
            proxyCore: self.localCore.clone(),
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
                                runtimeStorageHost
                                    .readBytes(&path.to_string_lossy())
                                    .map_err(|error| error.message)
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
