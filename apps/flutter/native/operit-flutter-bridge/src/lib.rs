#![allow(non_snake_case)]

use std::collections::{BTreeMap, HashMap};
use std::ffi::{c_char, CStr, CString};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
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
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkClient, CoreLinkError,
    CoreRequestId, CoreWatchRequest,
};
use operit_runtime::api::chat::enhance::ToolExecutionManager::AITool;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::core::tools::ToolPermissionSystem::PermissionRequestResult;
use operit_runtime::services::RuntimeHostInteractionService::{
    requestOwnerBrowserAutomation, requestOwnerComposeWebViewController,
    requestOwnerSystemCaptureScreenshot, requestOwnerSystemRecognizeText,
    requestOwnerToolPermission, requestOwnerWebVisit,
    RuntimeHostInteractionBrowserAutomationPayload,
    RuntimeHostInteractionComposeWebViewControllerPayload,
    RuntimeHostInteractionSystemRecognizeTextPayload,
    RuntimeHostInteractionToolPermissionPayload,
    RuntimeHostInteractionToolPermissionTool,
    RuntimeHostInteractionToolPermissionToolParameter,
    RuntimeHostInteractionWebVisitHeader, RuntimeHostInteractionWebVisitPayload,
};
use operit_runtime::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgHostEventRegistration;
use operit_runtime::plugins::toolpkg::ToolPkgHostEventHookBridge::ToolPkgHostEventHookBridge;

#[cfg(target_os = "android")]
use operit_host_android_native::{
    AndroidExternalRuntimeEventHost as NativeExternalRuntimeEventHost,
    AndroidFileSystemHost as NativeFileSystemHost, AndroidHttpHost as NativeHttpHost,
    AndroidManagedRuntimeHost as NativeManagedRuntimeHost,
    AndroidRuntimeStorageHost as NativeRuntimeStorageHost,
    AndroidSystemOperationHost as NativeSystemOperationHost,
    AndroidTerminalHost as NativeTerminalHost,
};
#[cfg(target_os = "android")]
use operit_host_api::SystemOperationHost;
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxExternalRuntimeEventHost as NativeExternalRuntimeEventHost,
    LinuxFileSystemHost as NativeFileSystemHost, LinuxHttpHost as NativeHttpHost,
    LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxRuntimeStorageHost as NativeRuntimeStorageHost,
    LinuxSystemOperationHost as NativeSystemOperationHost, LinuxTerminalHost as NativeTerminalHost,
};
#[cfg(target_arch = "wasm32")]
use operit_host_web::{
    WebFileSystemHost as NativeFileSystemHost, WebHttpHost as NativeHttpHost,
    WebManagedRuntimeHost as NativeManagedRuntimeHost,
    WebRuntimeStorageHost as NativeRuntimeStorageHost,
    WebSystemOperationHost as NativeSystemOperationHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsExternalRuntimeEventHost as NativeExternalRuntimeEventHost,
    WindowsFileSystemHost as NativeFileSystemHost, WindowsHttpHost as NativeHttpHost,
    WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
    WindowsRuntimeStorageHost as NativeRuntimeStorageHost,
    WindowsSystemOperationHost as NativeSystemOperationHost,
    WindowsTerminalHost as NativeTerminalHost,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct OperitFlutterBridge {
    #[cfg(not(target_arch = "wasm32"))]
    runtime: tokio::runtime::Runtime,
    #[cfg(not(target_arch = "wasm32"))]
    externalRuntimeEventRegistration: Box<dyn operit_host_api::ExternalRuntimeEventRegistration>,
    proxyCore: Arc<ConcurrentLocalCoreProxy>,
    watchStreams: Mutex<HashMap<String, CoreEventStream>>,
    nextWatchStreamId: Mutex<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    webAccessTask: Mutex<Option<tokio::task::JoinHandle<Result<(), String>>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pendingRemotePairings: Mutex<HashMap<String, PendingRemotePairing>>,
    #[cfg(not(target_arch = "wasm32"))]
    mdns: Mutex<Option<mdnss::MdnsHandle>>,
    #[cfg(any(windows, target_os = "linux", target_os = "android"))]
    terminalHost: Arc<NativeTerminalHost>,
}

#[cfg(not(target_arch = "wasm32"))]
struct PendingRemotePairing {
    client: RemoteLinkClient,
    state: PairStartState,
}

struct ConcurrentLocalCoreProxy {
    inner: Mutex<LocalCoreProxy>,
}

impl ConcurrentLocalCoreProxy {
    fn new(core: LocalCoreProxy) -> Self {
        Self {
            inner: Mutex::new(core),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, LocalCoreProxy>, CoreLinkError> {
        self.inner
            .lock()
            .map_err(|error| CoreLinkError::internal(format!("core proxy lock poisoned: {error}")))
    }
}

const PERMISSION_REQUEST_TIMEOUT_MS: u64 = 60_000;

#[derive(Clone)]
struct FlutterBrowserAutomationBridge {
}

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
struct FlutterWebVisitBridge {
}

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
        let response =
            requestOwnerWebVisit(pending, Duration::from_secs(60))
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
struct FlutterComposeDslWebViewBridge {
}

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
    fn new() -> Result<Self, String> {
        Self::new_with_storage_root(None)
    }

    fn new_with_storage_root(storage_root: Option<PathBuf>) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let runtime = {
            let mut runtimeBuilder = tokio::runtime::Builder::new_multi_thread();
            runtimeBuilder
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
        };
        let browserAutomationBridge = FlutterBrowserAutomationBridge::new();
        let webVisitBridge = FlutterWebVisitBridge::new();
        let composeDslWebViewBridge = FlutterComposeDslWebViewBridge::new();
        #[cfg(any(windows, target_os = "linux", target_os = "android"))]
        let terminalHost = Arc::new(NativeTerminalHost::new());
        let mut core = create_local_core(
            storage_root,
            Arc::new(webVisitBridge),
            Some(Arc::new(browserAutomationBridge)),
            Some(Arc::new(composeDslWebViewBridge)),
            #[cfg(any(windows, target_os = "linux", target_os = "android"))]
            terminalHost.clone(),
        )?;
        core.localApplicationMut().onCreate()?;
        install_permission_requester(&mut core);
        #[cfg(not(target_arch = "wasm32"))]
        let externalRuntimeEventRegistration =
            operit_runtime::core::application::ExternalRuntimeEventSupport::startExternalRuntimeEventSupport(
                core.localApplicationMut().applicationContext.clone(),
                "flutter",
            )?;
        Ok(Self {
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
            #[cfg(not(target_arch = "wasm32"))]
            externalRuntimeEventRegistration,
            proxyCore: Arc::new(ConcurrentLocalCoreProxy::new(core)),
            watchStreams: Mutex::new(HashMap::new()),
            nextWatchStreamId: Mutex::new(1),
            #[cfg(not(target_arch = "wasm32"))]
            webAccessTask: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            pendingRemotePairings: Mutex::new(HashMap::new()),
            #[cfg(not(target_arch = "wasm32"))]
            mdns: Mutex::new(None),
            #[cfg(any(windows, target_os = "linux", target_os = "android"))]
            terminalHost,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let Ok(mut proxyCore) = self.proxyCore.lock() else {
            return CoreCallResponse::err(
                request.requestId,
                CoreLinkError::internal("core proxy lock poisoned"),
            );
        };
        self.runtime.block_on(proxyCore.call(request))
    }

    #[cfg(target_arch = "wasm32")]
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let Ok(mut proxyCore) = self.proxyCore.lock() else {
            return CoreCallResponse::err(
                request.requestId,
                CoreLinkError::internal("core proxy lock poisoned"),
            );
        };
        proxyCore.call(request).await
    }

    #[allow(non_snake_case)]
    fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        let mut proxyCore = self.proxyCore.lock()?;
        #[cfg(target_arch = "wasm32")]
        {
            return proxyCore.watchSnapshotSync(request);
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.runtime.block_on(proxyCore.watchSnapshot(request))
    }

    fn watchStream(&self, request: CoreWatchRequest) -> Result<String, CoreLinkError> {
        let mut proxyCore = self.proxyCore.lock()?;
        #[cfg(target_arch = "wasm32")]
        let receiver = proxyCore.watchSync(request)?;
        #[cfg(not(target_arch = "wasm32"))]
        let receiver = self.runtime.block_on(proxyCore.watch(request))?;
        let mut nextWatchStreamId = self.nextWatchStreamId.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch stream id lock poisoned: {error}"))
        })?;
        let subscriptionId = format!(
            "flutter-watch-{}-{}",
            operit_host_api::TimeUtils::currentTimeMillisU128(),
            *nextWatchStreamId
        );
        *nextWatchStreamId += 1;
        self.watchStreams
            .lock()
            .map_err(|error| {
                CoreLinkError::internal(format!("watch stream lock poisoned: {error}"))
            })?
            .insert(subscriptionId.clone(), receiver);
        Ok(subscriptionId)
    }

    fn pollWatchStream(&self, subscriptionId: &str) -> Result<Vec<CoreEvent>, CoreLinkError> {
        let mut streams = self.watchStreams.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch stream lock poisoned: {error}"))
        })?;
        let Some(receiver) = streams.get_mut(subscriptionId) else {
            return Err(CoreLinkError::new(
                "WATCH_NOT_FOUND",
                "watch subscription not found",
            ));
        };
        let mut events = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            events.push(event);
        }
        Ok(events)
    }

    fn pollWatchStreams(
        &self,
        subscriptionIds: &[String],
    ) -> Result<HashMap<String, Vec<CoreEvent>>, CoreLinkError> {
        let mut streams = self.watchStreams.lock().map_err(|error| {
            CoreLinkError::internal(format!("watch stream lock poisoned: {error}"))
        })?;
        let mut eventsBySubscription = HashMap::new();
        for subscriptionId in subscriptionIds {
            let Some(receiver) = streams.get_mut(subscriptionId) else {
                return Err(CoreLinkError::new(
                    "WATCH_NOT_FOUND",
                    "watch subscription not found",
                ));
            };
            let mut events = Vec::new();
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
            }
            eventsBySubscription.insert(subscriptionId.clone(), events);
        }
        Ok(eventsBySubscription)
    }

    fn closeWatchStream(&self, subscriptionId: &str) {
        if let Ok(mut streams) = self.watchStreams.lock() {
            streams.remove(subscriptionId);
        }
    }

    fn dispatchHostEvent(&self, source: &str, payloadJson: &str) -> String {
        let payloadValue: serde_json::Value = match serde_json::from_str(payloadJson) {
            Ok(value) => value,
            Err(error) => {
                return serde_json::json!({
                    "ok": false,
                    "error": format!("host event payload is invalid JSON: {error}"),
                })
                .to_string();
            }
        };
        ToolPkgHostEventHookBridge::dispatchHostEvent(source, payloadValue);
        serde_json::json!({"ok": true}).to_string()
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
    proxyCore: Arc<ConcurrentLocalCoreProxy>,
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
            let mut proxyCore = proxyCore.lock()?;
            Ok::<CoreCallResponse, CoreLinkError>(runtimeHandle.block_on(proxyCore.call(request)))
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
            let mut proxyCore = proxyCore.lock()?;
            runtimeHandle.block_on(proxyCore.watchSnapshot(request))
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
            let mut proxyCore = proxyCore.lock()?;
            runtimeHandle.block_on(proxyCore.watch(request))
        })
        .await
        .map_err(|error| CoreLinkError::internal(format!("core watch task join failed: {error}")))?
    }
}

fn install_permission_requester(core: &mut LocalCoreProxy) {
    let context = core.localApplicationMut().applicationContext.clone();
    let handler = AIToolHandler::getInstance(context);
    handler.getToolPermissionSystem().setPermissionRequester(
        move |tool, description| {
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
                "always_allow" => PermissionRequestResult::ALWAYS_ALLOW,
                "deny" => PermissionRequestResult::DENY,
                other => panic!("unknown permission response result: {other}"),
            }
        },
    );
}

fn tool_to_permission_payload(tool: &AITool) -> RuntimeHostInteractionToolPermissionTool {
    RuntimeHostInteractionToolPermissionTool {
        name: tool.name.clone(),
        parameters: tool
            .parameters
            .iter()
            .map(|parameter| RuntimeHostInteractionToolPermissionToolParameter {
                name: parameter.name.clone(),
                value: parameter.value.clone(),
            })
            .collect(),
    }
}

#[cfg(any(windows, target_os = "linux", target_os = "android"))]
fn create_local_core(
    storage_root: Option<PathBuf>,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
    terminalHost: Arc<NativeTerminalHost>,
) -> Result<LocalCoreProxy, String> {
    let root_dir = match storage_root {
        Some(root_dir) => root_dir,
        None => default_native_storage_root()?,
    };
    let appFilesRoot = root_dir.clone();
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(root_dir));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    #[cfg(target_os = "android")]
    let systemOperationHost: Arc<dyn operit_host_api::SystemOperationHost> =
        Arc::new(FlutterSystemOperationBridge::new());
    #[cfg(not(target_os = "android"))]
    let systemOperationHost: Arc<dyn operit_host_api::SystemOperationHost> =
        Arc::new(NativeSystemOperationHost::new());
    let mut context =
        OperitApplicationContext::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
            Arc::new(NativeFileSystemHost::new()),
            webVisitHost,
            Arc::new(NativeHttpHost::new()),
            systemOperationHost,
            Arc::new(NativeManagedRuntimeHost::new()),
            runtimeStorageHost,
            runtimeSqliteHost,
        )
        .withAppFilesRoot(appFilesRoot);
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    if let Some(host) = composeDslWebViewHost {
        context = context.withComposeDslWebViewHost(host);
    }
    context = context.withTerminalHost(terminalHost);
    context = context.withExternalRuntimeEventHost(Arc::new(NativeExternalRuntimeEventHost::new()));
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(any(windows, target_os = "linux"))]
fn default_native_storage_root() -> Result<PathBuf, String> {
    Ok(NativeRuntimeStorageHost::defaultRoot())
}

#[cfg(target_os = "android")]
fn default_native_storage_root() -> Result<PathBuf, String> {
    Err("Android runtime storage root must be provided by the Android host".to_string())
}

#[cfg(target_arch = "wasm32")]
fn create_local_core(
    _storage_root: Option<PathBuf>,
    webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    composeDslWebViewHost: Option<Arc<dyn operit_host_api::ComposeDslWebViewHost>>,
) -> Result<LocalCoreProxy, String> {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new());
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let mut context =
        OperitApplicationContext::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
            Arc::new(NativeFileSystemHost::new()),
            webVisitHost,
            Arc::new(NativeHttpHost::new()),
            Arc::new(NativeSystemOperationHost::new()),
            Arc::new(NativeManagedRuntimeHost::new()),
            runtimeStorageHost,
            runtimeSqliteHost,
        )
        .withAppFilesRoot(NativeRuntimeStorageHost::defaultRoot());
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    if let Some(host) = composeDslWebViewHost {
        context = context.withComposeDslWebViewHost(host);
    }
    let application = OperitApplication::newWithContext(context);
    Ok(LocalCoreProxy::new(application))
}

#[cfg(not(any(
    windows,
    target_os = "linux",
    target_os = "android",
    target_arch = "wasm32"
)))]
fn create_local_core(
    _storage_root: Option<PathBuf>,
    _webVisitHost: Arc<dyn operit_host_api::WebVisitHost>,
    _browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
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

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_create_with_storage_root(
    storage_root: *const c_char,
) -> *mut OperitFlutterBridge {
    if storage_root.is_null() {
        set_last_create_error("runtime storage root pointer is null".to_string());
        return std::ptr::null_mut();
    }
    let storage_root = match CStr::from_ptr(storage_root).to_str() {
        Ok(value) => PathBuf::from(value),
        Err(error) => {
            set_last_create_error(format!("runtime storage root is not valid UTF-8: {error}"));
            return std::ptr::null_mut();
        }
    };
    match OperitFlutterBridge::new_with_storage_root(Some(storage_root)) {
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
) -> *mut c_char {
    if handle.is_null() {
        return error_response("flutter-bridge-null", "runtime bridge is not initialized");
    }
    if request_ptr.is_null() {
        return error_response(
            "flutter-bridge-null-request",
            "runtime request pointer is null",
        );
    }
    let request_bytes = std::slice::from_raw_parts(request_ptr, request_len);
    string_to_ptr(bridge_call_json(&mut *handle, request_bytes))
}

#[cfg(not(target_arch = "wasm32"))]
fn bridge_call_json(handle: &mut OperitFlutterBridge, request_bytes: &[u8]) -> String {
    let request: CoreCallRequest = match serde_json::from_slice(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return error_response_string(
                "flutter-bridge-invalid-request",
                format!("invalid core request: {error}"),
            );
        }
    };
    let response = handle.call(request);
    json_string(&response)
}

#[cfg(target_arch = "wasm32")]
async fn bridge_call_json_async(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> String {
    let request: CoreCallRequest = match serde_json::from_slice(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return error_response_string(
                "flutter-bridge-invalid-request",
                format!("invalid core request: {error}"),
            );
        }
    };
    let response = handle.call(request).await;
    json_string(&response)
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_watch_snapshot(
    handle: *mut OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if request_ptr.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal("runtime request pointer is null"))
                .expect("CoreLinkError must serialize"),
        );
    }
    let request_bytes = std::slice::from_raw_parts(request_ptr, request_len);
    string_to_ptr(bridge_watch_snapshot_json(&mut *handle, request_bytes))
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_watch_stream(
    handle: *mut OperitFlutterBridge,
    request_ptr: *const u8,
    request_len: usize,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if request_ptr.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal("runtime request pointer is null"))
                .expect("CoreLinkError must serialize"),
        );
    }
    let request_bytes = std::slice::from_raw_parts(request_ptr, request_len);
    string_to_ptr(bridge_watch_stream_json(&mut *handle, request_bytes))
}

fn bridge_watch_stream_json(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> String {
    let request: CoreWatchRequest = match serde_json::from_slice(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return serde_json::to_string(&CoreLinkError::internal(format!(
                "invalid core watch request: {error}"
            )))
            .expect("CoreLinkError must serialize");
        }
    };
    match handle.watchStream(request) {
        Ok(subscriptionId) => serde_json::json!({ "subscriptionId": subscriptionId }).to_string(),
        Err(error) => serde_json::to_string(&error).expect("CoreLinkError must serialize"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_poll_watch_stream(
    handle: *mut OperitFlutterBridge,
    subscription_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if subscription_ptr.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "watch subscription pointer is null",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    let subscriptionId = match CStr::from_ptr(subscription_ptr).to_str() {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::internal(format!(
                    "watch subscription id is not valid UTF-8: {error}"
                )))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    match (*handle).pollWatchStream(subscriptionId) {
        Ok(events) => json_to_ptr(&events),
        Err(error) => serde_json::to_string(&error)
            .map(string_to_ptr)
            .expect("CoreLinkError must serialize"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_poll_watch_streams(
    handle: *mut OperitFlutterBridge,
    subscription_ids_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if subscription_ids_ptr.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "watch subscription id array pointer is null",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    let subscriptionIdsJson = match CStr::from_ptr(subscription_ids_ptr).to_str() {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::internal(format!(
                    "watch subscription id array is not valid UTF-8: {error}"
                )))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    let subscriptionIds: Vec<String> = match serde_json::from_str(subscriptionIdsJson) {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("pollWatchStreams expects a JSON string array: {error}"),
                ))
                .expect("CoreLinkError must serialize"),
            );
        }
    };
    match (*handle).pollWatchStreams(&subscriptionIds) {
        Ok(events) => json_to_ptr(&events),
        Err(error) => serde_json::to_string(&error)
            .map(string_to_ptr)
            .expect("CoreLinkError must serialize"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_close_watch_stream(
    handle: *mut OperitFlutterBridge,
    subscription_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "runtime bridge is not initialized",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if subscription_ptr.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::internal(
                "watch subscription pointer is null",
            ))
            .expect("CoreLinkError must serialize"),
        );
    }
    if let Ok(subscriptionId) = CStr::from_ptr(subscription_ptr).to_str() {
        (*handle).closeWatchStream(subscriptionId);
    }
    string_to_ptr("{\"ok\":true}")
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

fn bridge_watch_snapshot_json(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> String {
    let request: CoreWatchRequest = match serde_json::from_slice(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            return serde_json::to_string(&CoreLinkError::internal(format!(
                "invalid core watch request: {error}"
            )))
            .expect("CoreLinkError must serialize");
        }
    };
    match handle.watchSnapshot(request) {
        Ok(event) => json_string(&event),
        Err(error) => serde_json::to_string(&error).expect("CoreLinkError must serialize"),
    }
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_dispatch_host_event(
    handle: *mut OperitFlutterBridge,
    source_ptr: *const c_char,
    payload_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::json!({"ok": false, "error": "bridge handle is null"}).to_string(),
        );
    }
    if source_ptr.is_null() {
        return string_to_ptr(
            serde_json::json!({"ok": false, "error": "source is null"}).to_string(),
        );
    }
    if payload_ptr.is_null() {
        return string_to_ptr(
            serde_json::json!({"ok": false, "error": "payload is null"}).to_string(),
        );
    }
    let source = match CStr::from_ptr(source_ptr).to_str() {
        Ok(value) => value,
        Err(_) => {
            return string_to_ptr(
                serde_json::json!({"ok": false, "error": "source is not UTF-8"}).to_string(),
            )
        }
    };
    let payload = match CStr::from_ptr(payload_ptr).to_str() {
        Ok(value) => value,
        Err(_) => {
            return string_to_ptr(
                serde_json::json!({"ok": false, "error": "payload is not UTF-8"}).to_string(),
            )
        }
    };
    string_to_ptr((*handle).dispatchHostEvent(source, payload))
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

    pub async fn call(&self, request: &str) -> String {
        bridge_call_json_async(&self.inner, request.as_bytes()).await
    }

    #[allow(non_snake_case)]
    pub fn watchSnapshot(&self, request: &str) -> String {
        bridge_watch_snapshot_json(&self.inner, request.as_bytes())
    }

    #[allow(non_snake_case)]
    pub fn watchStream(&self, request: &str) -> String {
        bridge_watch_stream_json(&self.inner, request.as_bytes())
    }

    #[allow(non_snake_case)]
    pub fn pollWatchStream(&self, subscriptionId: &str) -> String {
        match self.inner.pollWatchStream(subscriptionId) {
            Ok(events) => json_string(&events),
            Err(error) => serde_json::to_string(&error).expect("CoreLinkError must serialize"),
        }
    }

    #[allow(non_snake_case)]
    pub fn pollWatchStreams(&self, subscriptionIdsJson: &str) -> String {
        let subscriptionIds: Vec<String> = match serde_json::from_str(subscriptionIdsJson) {
            Ok(value) => value,
            Err(error) => {
                return serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("pollWatchStreams expects a JSON string array: {error}"),
                ))
                .expect("CoreLinkError must serialize");
            }
        };
        match self.inner.pollWatchStreams(&subscriptionIds) {
            Ok(events) => json_string(&events),
            Err(error) => serde_json::to_string(&error).expect("CoreLinkError must serialize"),
        }
    }

    #[allow(non_snake_case)]
    pub fn closeWatchStream(&self, subscriptionId: &str) -> String {
        self.inner.closeWatchStream(subscriptionId);
        "{\"ok\":true}".to_string()
    }

    #[allow(non_snake_case)]
    pub fn dispatchHostEvent(&self, source: &str, payloadJson: &str) -> String {
        self.inner.dispatchHostEvent(source, payloadJson)
    }
}

fn error_response(requestId: impl Into<String>, message: impl Into<String>) -> *mut c_char {
    string_to_ptr(error_response_string(requestId, message))
}

fn error_response_string(requestId: impl Into<String>, message: impl Into<String>) -> String {
    let response = CoreCallResponse::err(
        CoreRequestId::new(requestId),
        CoreLinkError::internal(message.into()),
    );
    json_string(&response)
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
    use jni::sys::{jlong, jstring};
    use jni::JNIEnv;

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_create(
        mut env: JNIEnv,
        _class: JClass,
        storage_root: JString,
        _host: JObject,
    ) -> jlong {
        let storage_root = match env.get_string(&storage_root) {
            Ok(value) => PathBuf::from(String::from(value)),
            Err(error) => {
                set_last_create_error(format!("runtime storage root is invalid: {error}"));
                return 0;
            }
        };
        match OperitFlutterBridge::new_with_storage_root(Some(storage_root)) {
            Ok(bridge) => Box::into_raw(Box::new(bridge)) as jlong,
            Err(error) => {
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
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_call(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        request: JByteArray,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_mut() else {
            return new_java_string(
                env,
                &error_response_string("flutter-bridge-null", "runtime bridge is not initialized"),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &error_response_string(
                        "flutter-bridge-invalid-request",
                        format!("invalid JNI request bytes: {error}"),
                    ),
                );
            }
        };
        new_java_string(env, &bridge_call_json(bridge, &bytes))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_watchSnapshot(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        request: JByteArray,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_mut() else {
            return new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must serialize"),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::internal(format!(
                        "invalid JNI watch request bytes: {error}"
                    )))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        new_java_string(env, &bridge_watch_snapshot_json(bridge, &bytes))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_watchStream(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        request: JByteArray,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_mut() else {
            return new_java_string(
                env,
                &serde_json::to_string(&CoreLinkError::internal(
                    "runtime bridge is not initialized",
                ))
                .expect("CoreLinkError must serialize"),
            );
        };
        let bytes = match env.convert_byte_array(request) {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::internal(format!(
                        "invalid JNI watch request bytes: {error}"
                    )))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        new_java_string(env, &bridge_watch_stream_json(bridge, &bytes))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_pollWatchStream(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        subscriptionId: JString,
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
        let subscriptionId = match env.get_string(&subscriptionId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::internal(format!(
                        "invalid JNI subscription id: {error}"
                    )))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let response = match bridge.pollWatchStream(&subscriptionId) {
            Ok(events) => json_string(&events),
            Err(error) => serde_json::to_string(&error).expect("CoreLinkError must serialize"),
        };
        new_java_string(env, &response)
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_pollWatchStreams(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        subscriptionIdsJson: JString,
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
        let subscriptionIdsJson = match env.get_string(&subscriptionIdsJson) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::internal(format!(
                        "invalid JNI subscription id array: {error}"
                    )))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let subscriptionIds: Vec<String> = match serde_json::from_str(&subscriptionIdsJson) {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::to_string(&CoreLinkError::new(
                        "INVALID_ARGS",
                        format!("pollWatchStreams expects a JSON string array: {error}"),
                    ))
                    .expect("CoreLinkError must serialize"),
                );
            }
        };
        let response = match bridge.pollWatchStreams(&subscriptionIds) {
            Ok(events) => json_string(&events),
            Err(error) => serde_json::to_string(&error).expect("CoreLinkError must serialize"),
        };
        new_java_string(env, &response)
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_closeWatchStream(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        subscriptionId: JString,
    ) -> jstring {
        if let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() {
            if let Ok(subscriptionId) = env.get_string(&subscriptionId) {
                bridge.closeWatchStream(&String::from(subscriptionId));
            }
        }
        new_java_string(env, "{\"ok\":true}")
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
        let enableWebAccess = match env.get_string(&enableWebAccess) {
            Ok(value) => value.to_str() == "true",
            Err(_) => false,
        };
        let enableDiscovery = match env.get_string(&enableDiscovery) {
            Ok(value) => value.to_str() == "true",
            Err(_) => false,
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
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_dispatchHostEvent(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        source: JString,
        payload: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(env, &serde_json::json!({"ok": false}).to_string());
        };
        let source = match env.get_string(&source) {
            Ok(value) => String::from(value),
            Err(_) => return new_java_string(env, &serde_json::json!({"ok": false}).to_string()),
        };
        let payload = match env.get_string(&payload) {
            Ok(value) => String::from(value),
            Err(_) => return new_java_string(env, &serde_json::json!({"ok": false}).to_string()),
        };
        new_java_string(env, &bridge.dispatchHostEvent(&source, &payload))
    }

    fn new_java_string(mut env: JNIEnv, value: &str) -> jstring {
        env.new_string(value)
            .expect("JNI string allocation must succeed")
            .into_raw()
    }
}
