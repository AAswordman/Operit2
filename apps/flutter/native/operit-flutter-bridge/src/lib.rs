#![allow(non_snake_case)]

use std::collections::{BTreeMap, HashMap};
use std::ffi::{c_char, c_void, CStr, CString};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use operit_core_proxy::LocalCoreProxy;
use operit_link::{
    AcceptedRemoteSessionLoader, AcceptedRemoteSessionRecord, AcceptedRemoteSessionStore,
    PairStartState, RemoteDeviceInfo, RemoteLinkClient, RemoteLinkServer, RemoteLinkServerConfig,
    RemotePairingCodeRecord, RemotePairingCodeSink, RemoteWebAccessConfig,
};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkClient, CoreLinkError,
    CoreRequestId, CoreWatchRequest,
};
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::enhance::ToolExecutionManager::AITool;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;
use operit_runtime::core::tools::AIToolHandler::AIToolHandler;
use operit_runtime::core::tools::ToolPermissionSystem::PermissionRequestResult;

#[cfg(target_os = "android")]
use operit_host_android_native::{
    AndroidExternalRuntimeEventHost as NativeExternalRuntimeEventHost,
    AndroidFileSystemHost as NativeFileSystemHost, AndroidHttpHost as NativeHttpHost,
    AndroidManagedRuntimeHost as NativeManagedRuntimeHost,
    AndroidRuntimeStorageHost as NativeRuntimeStorageHost,
    AndroidSystemOperationHost as NativeSystemOperationHost,
    AndroidTerminalHost as NativeTerminalHost,
};
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
    approvalBridge: FlutterApprovalBridge,
    runtimeHostBridge: Option<FlutterRuntimeHostBridge>,
    #[cfg(not(target_arch = "wasm32"))]
    webAccessTask: Mutex<Option<tokio::task::JoinHandle<Result<(), String>>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pendingRemotePairings: Mutex<HashMap<String, PendingRemotePairing>>,
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

trait RuntimeHostRequestBridge: Send + Sync {
    fn handleRequest(&self, methodName: &str, payloadJson: &str) -> Result<String, String>;
}

#[derive(Clone)]
struct FlutterRuntimeHostBridge {
    inner: Arc<dyn RuntimeHostRequestBridge>,
}

impl FlutterRuntimeHostBridge {
    fn new(inner: Arc<dyn RuntimeHostRequestBridge>) -> Self {
        Self { inner }
    }

    fn handleRequest(&self, methodName: &str, payloadJson: &str) -> Result<String, String> {
        self.inner.handleRequest(methodName, payloadJson)
    }
}

type RuntimeHostRequestCallback =
    unsafe extern "C" fn(*const c_char, *const u8, usize, *mut c_void) -> *mut c_char;
type RuntimeHostRequestFree = unsafe extern "C" fn(*mut c_char, *mut c_void);

struct CallbackRuntimeHostBridge {
    callback: RuntimeHostRequestCallback,
    free: RuntimeHostRequestFree,
    userData: usize,
}

unsafe impl Send for CallbackRuntimeHostBridge {}
unsafe impl Sync for CallbackRuntimeHostBridge {}

impl RuntimeHostRequestBridge for CallbackRuntimeHostBridge {
    fn handleRequest(&self, methodName: &str, payloadJson: &str) -> Result<String, String> {
        let methodName = CString::new(methodName)
            .map_err(|error| format!("runtime host method is invalid: {error}"))?;
        let raw = unsafe {
            (self.callback)(
                methodName.as_ptr(),
                payloadJson.as_ptr(),
                payloadJson.len(),
                self.userData as *mut c_void,
            )
        };
        if raw.is_null() {
            return Err("runtime host bridge returned null".to_string());
        }
        let bytes = unsafe { CStr::from_ptr(raw).to_bytes().to_vec() };
        unsafe {
            (self.free)(raw, self.userData as *mut c_void);
        }
        let envelope = String::from_utf8(bytes)
            .map_err(|error| format!("runtime host bridge returned non UTF-8: {error}"))?;
        let value = serde_json::from_str::<serde_json::Value>(&envelope)
            .map_err(|error| format!("runtime host bridge response is invalid JSON: {error}"))?;
        let object = value
            .as_object()
            .ok_or_else(|| "runtime host bridge response must be a JSON object".to_string())?;
        let ok = object
            .get("ok")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| "runtime host bridge response is missing ok".to_string())?;
        if ok {
            return object
                .get("value")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| "runtime host bridge response is missing value".to_string());
        }
        let error = object
            .get("error")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| "runtime host bridge response is missing error".to_string())?;
        Err(error)
    }
}

#[derive(Clone)]
struct FlutterApprovalBridge {
    inner: Arc<ApprovalInner>,
}

struct ApprovalInner {
    state: Mutex<ApprovalState>,
    changed: Condvar,
}

#[derive(Clone, Debug, serde::Serialize)]
struct PendingApproval {
    tool: AITool,
    description: String,
    requestedAtMillis: u64,
    #[serde(skip)]
    requestedAt: Instant,
}

#[derive(Debug)]
struct ApprovalState {
    pending: Option<PendingApproval>,
    response: Option<PermissionRequestResult>,
}

impl FlutterApprovalBridge {
    fn new() -> Self {
        Self {
            inner: Arc::new(ApprovalInner {
                state: Mutex::new(ApprovalState {
                    pending: None,
                    response: None,
                }),
                changed: Condvar::new(),
            }),
        }
    }

    fn request(&self, tool: &AITool, description: &str) -> PermissionRequestResult {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("approval state mutex poisoned");
        state.pending = Some(PendingApproval {
            tool: tool.clone(),
            description: description.to_string(),
            requestedAtMillis: current_time_millis_u64(),
            requestedAt: Instant::now(),
        });
        state.response = None;
        self.inner.changed.notify_all();

        let timeout = Duration::from_millis(PERMISSION_REQUEST_TIMEOUT_MS);
        loop {
            if let Some(response) = state.response.take() {
                state.pending = None;
                self.inner.changed.notify_all();
                return response;
            }
            let pendingStartedAt = state.pending.as_ref().map(|pending| pending.requestedAt);
            let Some(startedAt) = pendingStartedAt else {
                return PermissionRequestResult::DENY;
            };
            let elapsed = startedAt.elapsed();
            if elapsed >= timeout {
                state.pending = None;
                self.inner.changed.notify_all();
                return PermissionRequestResult::DENY;
            }
            let wait = timeout.saturating_sub(elapsed);
            let (nextState, result) = self
                .inner
                .changed
                .wait_timeout(state, wait)
                .expect("approval state mutex poisoned");
            state = nextState;
            if result.timed_out() {
                state.pending = None;
                self.inner.changed.notify_all();
                return PermissionRequestResult::DENY;
            }
        }
    }

    fn current(&self) -> Option<PendingApproval> {
        self.inner
            .state
            .lock()
            .expect("approval state mutex poisoned")
            .pending
            .clone()
    }

    fn respond(&self, response: PermissionRequestResult) -> bool {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("approval state mutex poisoned");
        if state.pending.is_some() {
            state.response = Some(response);
            self.inner.changed.notify_all();
            return true;
        }
        false
    }
}

#[derive(Clone)]
struct FlutterBrowserAutomationBridge {
    runtimeHostBridge: Option<FlutterRuntimeHostBridge>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct PendingBrowserAutomationRequest {
    requestId: String,
    toolName: String,
    parametersJson: String,
    requestedAtMillis: u64,
}

#[derive(Clone, Debug)]
struct BrowserAutomationToolResponse {
    success: bool,
    result: String,
    error: Option<String>,
}

impl FlutterBrowserAutomationBridge {
    fn new(runtimeHostBridge: Option<FlutterRuntimeHostBridge>) -> Self {
        Self { runtimeHostBridge }
    }
}

impl operit_host_api::BrowserAutomationHost for FlutterBrowserAutomationBridge {
    fn executeBrowserTool(
        &self,
        request: operit_host_api::BrowserAutomationRequest,
    ) -> operit_host_api::HostResult<operit_host_api::BrowserAutomationResponse> {
        let requestId = request.requestId.clone();
        let pending = PendingBrowserAutomationRequest {
            requestId: request.requestId,
            toolName: request.toolName,
            parametersJson: request.parametersJson,
            requestedAtMillis: current_time_millis_u64(),
        };
        let runtimeHostBridge = self.runtimeHostBridge.as_ref().ok_or_else(|| {
            operit_host_api::HostError::new("runtime host bridge is not installed")
        })?;
        let payloadJson = serde_json::to_string(&pending)
            .map_err(|error| operit_host_api::HostError::new(error.to_string()))?;
        let resultJson = runtimeHostBridge
            .handleRequest("browserAutomationRequest", &payloadJson)
            .map_err(operit_host_api::HostError::new)?;
        let (responseRequestId, response) = parse_browser_automation_result(&resultJson)
            .map_err(operit_host_api::HostError::new)?;
        if responseRequestId != requestId {
            return Err(operit_host_api::HostError::new(format!(
                "browser automation response requestId mismatch: {responseRequestId} != {requestId}"
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
    runtimeHostBridge: Option<FlutterRuntimeHostBridge>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct PendingWebVisitHeader {
    name: String,
    value: String,
}

#[derive(Clone, Debug, serde::Serialize)]
struct PendingWebVisitRequest {
    requestId: String,
    url: String,
    headers: Vec<PendingWebVisitHeader>,
    userAgent: String,
    includeImageLinks: bool,
    requestedAtMillis: u64,
}

#[derive(Clone, Debug)]
struct WebVisitToolResponse {
    success: bool,
    result: Option<operit_host_api::WebVisitResult>,
    error: Option<String>,
}

impl FlutterWebVisitBridge {
    fn new(runtimeHostBridge: Option<FlutterRuntimeHostBridge>) -> Self {
        Self { runtimeHostBridge }
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
        let pending = PendingWebVisitRequest {
            requestId: requestId.clone(),
            url: request.url,
            headers: request
                .headers
                .into_iter()
                .map(|(name, value)| PendingWebVisitHeader { name, value })
                .collect(),
            userAgent: request.userAgent,
            includeImageLinks: request.includeImageLinks,
            requestedAtMillis: current_time_millis_u64(),
        };
        let runtimeHostBridge = self.runtimeHostBridge.as_ref().ok_or_else(|| {
            operit_host_api::HostError::new("runtime host bridge is not installed")
        })?;
        let payloadJson = serde_json::to_string(&pending)
            .map_err(|error| operit_host_api::HostError::new(error.to_string()))?;
        let resultJson = runtimeHostBridge
            .handleRequest("webVisitRequest", &payloadJson)
            .map_err(operit_host_api::HostError::new)?;
        let (responseRequestId, response) =
            parse_web_visit_result(&resultJson).map_err(operit_host_api::HostError::new)?;
        if responseRequestId != requestId {
            return Err(operit_host_api::HostError::new(format!(
                "web visit response requestId mismatch: {responseRequestId} != {requestId}"
            )));
        }
        if response.success {
            let Some(result) = response.result else {
                return Err(operit_host_api::HostError::new(
                    "web visit result is missing",
                ));
            };
            return Ok(result);
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
    runtimeHostBridge: Option<FlutterRuntimeHostBridge>,
}

impl FlutterComposeDslWebViewBridge {
    fn new(runtimeHostBridge: Option<FlutterRuntimeHostBridge>) -> Self {
        Self { runtimeHostBridge }
    }
}

impl operit_host_api::ComposeDslWebViewHost for FlutterComposeDslWebViewBridge {
    fn handleControllerCommand(&self, payloadJson: &str) -> operit_host_api::HostResult<String> {
        let runtimeHostBridge = self.runtimeHostBridge.as_ref().ok_or_else(|| {
            operit_host_api::HostError::new("runtime host bridge is not installed")
        })?;
        runtimeHostBridge
            .handleRequest("composeWebViewControllerCommand", payloadJson)
            .map_err(operit_host_api::HostError::new)
    }
}

impl OperitFlutterBridge {
    fn new() -> Result<Self, String> {
        Self::new_with_storage_root(None)
    }

    fn new_with_storage_root(storage_root: Option<PathBuf>) -> Result<Self, String> {
        Self::new_with_storage_root_and_runtime_host_bridge(storage_root, None)
    }

    fn new_with_storage_root_and_runtime_host_bridge(
        storage_root: Option<PathBuf>,
        runtimeHostBridge: Option<FlutterRuntimeHostBridge>,
    ) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let runtime = {
            let mut runtimeBuilder = tokio::runtime::Builder::new_multi_thread();
            runtimeBuilder
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
        };
        let approvalBridge = FlutterApprovalBridge::new();
        let runtimeHostBridgeForServer = runtimeHostBridge.clone();
        let browserAutomationBridge =
            FlutterBrowserAutomationBridge::new(runtimeHostBridge.clone());
        let webVisitBridge = FlutterWebVisitBridge::new(runtimeHostBridge.clone());
        let composeDslWebViewBridge =
            FlutterComposeDslWebViewBridge::new(runtimeHostBridge.clone());
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
        install_permission_requester(&mut core, approvalBridge.clone());
        let mainCore = core
            .localApplicationMut()
            .chatRuntimeHolder
            .getCore(ChatRuntimeSlot::MAIN);
        mainCore.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
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
            approvalBridge,
            runtimeHostBridge: runtimeHostBridgeForServer,
            #[cfg(not(target_arch = "wasm32"))]
            webAccessTask: Mutex::new(None),
            #[cfg(not(target_arch = "wasm32"))]
            pendingRemotePairings: Mutex::new(HashMap::new()),
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

    fn hostDescriptor(&self) -> serde_json::Value {
        let mut proxyCore = self.proxyCore.lock().expect("core proxy lock poisoned");
        let application = proxyCore.localApplicationMut();
        let context = &application.applicationContext;
        let host = &context.hostEnvironment;
        serde_json::json!({
            "id": host.id,
            "displayName": host.displayName,
            "pathStyleDescriptionEn": host.pathStyleDescriptionEn,
            "pathStyleDescriptionCn": host.pathStyleDescriptionCn,
            "examplePaths": host.examplePaths,
            "usesEnvironmentParameter": host.usesEnvironmentParameter,
            "environmentParameterDescriptionEn": host.environmentParameterDescriptionEn,
            "environmentParameterDescriptionCn": host.environmentParameterDescriptionCn,
            "capabilities": host.capabilities,
            "fileSystemHost": context.fileSystemHost.is_some(),
            "webVisitHost": context.webVisitHost.is_some(),
            "systemOperationHost": context.systemOperationHost.is_some(),
            "managedRuntimeHost": context.managedRuntimeHost.is_some(),
            "terminalHost": context.terminalHost.is_some(),
            "runtimeStorageHost": context.runtimeStorageHost.is_some(),
            "runtimeSqliteHost": context.runtimeSqliteHost.is_some(),
            "browserAutomationHost": context.browserAutomationHost.is_some(),
        })
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

    fn currentPermissionRequest(&self) -> String {
        json_string(&self.approvalBridge.current())
    }

    fn handlePermissionResult(&self, result: &str) -> String {
        let response = match result {
            "ALLOW" | "allow" => PermissionRequestResult::ALLOW,
            "DENY" | "deny" => PermissionRequestResult::DENY,
            "ALWAYS_ALLOW" | "always_allow" => PermissionRequestResult::ALWAYS_ALLOW,
            other => {
                return serde_json::json!({
                    "ok": false,
                    "error": format!("unknown permission result: {other}")
                })
                .to_string();
            }
        };
        serde_json::json!({ "ok": self.approvalBridge.respond(response) }).to_string()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn remotePairStart(
        &self,
        baseUrl: String,
        token: String,
        clientDeviceInfo: RemoteDeviceInfo,
    ) -> Result<String, String> {
        let client = RemoteLinkClient::new(baseUrl);
        let hello = self.runtime.block_on(client.hello(&token))?;
        let state = self
            .runtime
            .block_on(client.pairStart(&token, clientDeviceInfo))?;
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
        acceptedSessionsJson: String,
        acceptedSessionStorePath: PathBuf,
        pairingCodePath: PathBuf,
        deviceInfo: RemoteDeviceInfo,
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
                    deviceInfo,
                    hostInteractionBroker: None,
                    webAccess: Some(RemoteWebAccessConfig {
                        token,
                        shutdownToken,
                        webRoot,
                    }),
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

fn install_permission_requester(core: &mut LocalCoreProxy, approvalBridge: FlutterApprovalBridge) {
    let context = core.localApplicationMut().applicationContext.clone();
    let handler = AIToolHandler::getInstance(context);
    handler
        .getToolPermissionSystem()
        .setPermissionRequester(move |tool, description| approvalBridge.request(tool, description));
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
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(root_dir));
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
        );
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
        );
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
pub unsafe extern "C" fn operit_flutter_bridge_create_with_runtime_host_bridge(
    callback: Option<RuntimeHostRequestCallback>,
    free: Option<RuntimeHostRequestFree>,
    user_data: *mut c_void,
) -> *mut OperitFlutterBridge {
    let (Some(callback), Some(free)) = (callback, free) else {
        set_last_create_error("runtime host bridge callback is required".to_string());
        return std::ptr::null_mut();
    };
    let runtimeHostBridge = Some(FlutterRuntimeHostBridge::new(Arc::new(
        CallbackRuntimeHostBridge {
            callback,
            free,
            userData: user_data as usize,
        },
    )));
    match OperitFlutterBridge::new_with_storage_root_and_runtime_host_bridge(
        None,
        runtimeHostBridge,
    ) {
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
    accepted_sessions_json: *const c_char,
    accepted_session_store_path: *const c_char,
    pairing_code_path: *const c_char,
    device_info_json: *const c_char,
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
        ("accepted sessions", accepted_sessions_json),
        ("accepted session store path", accepted_session_store_path),
        ("pairing code path", pairing_code_path),
        ("device info", device_info_json),
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
        PathBuf::from(&values[5]),
        PathBuf::from(&values[6]),
        match serde_json::from_str::<RemoteDeviceInfo>(&values[7]) {
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
pub unsafe extern "C" fn operit_flutter_bridge_host_descriptor(
    handle: *mut OperitFlutterBridge,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(
            serde_json::json!({
                "error": "runtime bridge is not initialized"
            })
            .to_string(),
        );
    }
    string_to_ptr((*handle).hostDescriptor().to_string())
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_current_permission_request(
    handle: *mut OperitFlutterBridge,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr("null");
    }
    string_to_ptr((*handle).currentPermissionRequest())
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_handle_permission_result(
    handle: *mut OperitFlutterBridge,
    result_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(serde_json::json!({"ok": false}).to_string());
    }
    if result_ptr.is_null() {
        return string_to_ptr(serde_json::json!({"ok": false}).to_string());
    }
    let result = match CStr::from_ptr(result_ptr).to_str() {
        Ok(value) => value,
        Err(_) => return string_to_ptr(serde_json::json!({"ok": false}).to_string()),
    };
    string_to_ptr((*handle).handlePermissionResult(result))
}

#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_remote_pair_start(
    handle: *mut OperitFlutterBridge,
    base_url: *const c_char,
    token: *const c_char,
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
    if base_url.is_null() || token.is_null() || client_device_info_json.is_null() {
        return string_to_ptr(
            serde_json::to_string(&CoreLinkError::new(
                "INVALID_ARGS",
                "remote pair start expects baseUrl, token and clientDeviceInfo",
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
    let token = match CStr::from_ptr(token).to_str() {
        Ok(value) => value.to_string(),
        Err(error) => {
            return string_to_ptr(
                serde_json::to_string(&CoreLinkError::new(
                    "INVALID_ARGS",
                    format!("token is not valid UTF-8: {error}"),
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
    match (*handle).remotePairStart(baseUrl, token, clientDeviceInfo) {
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
    pub fn hostDescriptor(&self) -> String {
        self.inner.hostDescriptor().to_string()
    }

    #[allow(non_snake_case)]
    pub fn currentPermissionRequest(&self) -> String {
        self.inner.currentPermissionRequest()
    }

    #[allow(non_snake_case)]
    pub fn handlePermissionResult(&self, result: &str) -> String {
        self.inner.handlePermissionResult(result)
    }
}

fn error_response(requestId: impl Into<String>, message: impl Into<String>) -> *mut c_char {
    string_to_ptr(error_response_string(requestId, message))
}

fn parse_browser_automation_result(
    resultJson: &str,
) -> Result<(String, BrowserAutomationToolResponse), String> {
    let value =
        serde_json::from_str::<serde_json::Value>(resultJson).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "browser automation result must be a JSON object".to_string())?;
    let requestId = object
        .get("requestId")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "browser automation result is missing requestId".to_string())?;
    let success = object
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| "browser automation result is missing success".to_string())?;
    let result = object
        .get("result")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "browser automation result is missing result".to_string())?;
    let error = object
        .get("error")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    Ok((
        requestId,
        BrowserAutomationToolResponse {
            success,
            result,
            error,
        },
    ))
}

fn parse_web_visit_result(resultJson: &str) -> Result<(String, WebVisitToolResponse), String> {
    let value =
        serde_json::from_str::<serde_json::Value>(resultJson).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "web visit result must be a JSON object".to_string())?;
    let requestId = object
        .get("requestId")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "web visit result is missing requestId".to_string())?;
    let success = object
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| "web visit result is missing success".to_string())?;
    let error = object
        .get("error")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    if !success {
        return Ok((
            requestId,
            WebVisitToolResponse {
                success,
                result: None,
                error,
            },
        ));
    }
    let resultValue = object
        .get("result")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "web visit result is missing result".to_string())?;
    let metadata = resultValue
        .get("metadata")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flat_map(|object| {
            object.iter().filter_map(|(key, value)| {
                value.as_str().map(|value| (key.clone(), value.to_string()))
            })
        })
        .collect::<Vec<_>>();
    let links = resultValue
        .get("links")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(operit_host_api::WebVisitLinkData {
                url: item.get("url")?.as_str()?.to_string(),
                text: item.get("text")?.as_str()?.to_string(),
            })
        })
        .collect::<Vec<_>>();
    let imageLinks = resultValue
        .get("imageLinks")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    Ok((
        requestId,
        WebVisitToolResponse {
            success,
            result: Some(operit_host_api::WebVisitResult {
                url: required_string(resultValue, "url")?,
                title: required_string(resultValue, "title")?,
                content: required_string(resultValue, "content")?,
                metadata,
                links,
                imageLinks,
            }),
            error,
        },
    ))
}

fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<String, String> {
    object
        .get(name)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("web visit result is missing {name}"))
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
    use jni::objects::{GlobalRef, JByteArray, JClass, JObject, JString, JValue};
    use jni::sys::{jlong, jstring};
    use jni::{JNIEnv, JavaVM};

    struct AndroidRuntimeHostBridge {
        javaVm: JavaVM,
        host: GlobalRef,
    }

    impl RuntimeHostRequestBridge for AndroidRuntimeHostBridge {
        fn handleRequest(&self, methodName: &str, payloadJson: &str) -> Result<String, String> {
            let mut env = self
                .javaVm
                .attach_current_thread()
                .map_err(|error| error.to_string())?;
            let methodName = env
                .new_string(methodName)
                .map_err(|error| error.to_string())?;
            let payloadJson = env
                .new_string(payloadJson)
                .map_err(|error| error.to_string())?;
            let methodNameObject = JObject::from(methodName);
            let payloadJsonObject = JObject::from(payloadJson);
            let result = env
                .call_method(
                    self.host.as_obj(),
                    "handleRuntimeHostRequest",
                    "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                    &[
                        JValue::Object(&methodNameObject),
                        JValue::Object(&payloadJsonObject),
                    ],
                )
                .map_err(|error| error.to_string())?
                .l()
                .map_err(|error| error.to_string())?;
            let result = JString::from(result);
            env.get_string(&result)
                .map(String::from)
                .map_err(|error| error.to_string())
        }
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_create(
        mut env: JNIEnv,
        _class: JClass,
        storage_root: JString,
        host: JObject,
    ) -> jlong {
        let storage_root = match env.get_string(&storage_root) {
            Ok(value) => PathBuf::from(String::from(value)),
            Err(error) => {
                set_last_create_error(format!("runtime storage root is invalid: {error}"));
                return 0;
            }
        };
        let javaVm = match env.get_java_vm() {
            Ok(value) => value,
            Err(error) => {
                set_last_create_error(format!("runtime host JavaVM is unavailable: {error}"));
                return 0;
            }
        };
        let host = match env.new_global_ref(host) {
            Ok(value) => value,
            Err(error) => {
                set_last_create_error(format!("runtime host reference is unavailable: {error}"));
                return 0;
            }
        };
        let runtimeHostBridge =
            FlutterRuntimeHostBridge::new(Arc::new(AndroidRuntimeHostBridge { javaVm, host }));
        match OperitFlutterBridge::new_with_storage_root_and_runtime_host_bridge(
            Some(storage_root),
            Some(runtimeHostBridge),
        ) {
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
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_hostDescriptor(
        env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(env, "{\"error\":\"runtime bridge is not initialized\"}");
        };
        new_java_string(env, &bridge.hostDescriptor().to_string())
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
        acceptedSessionsJson: JString,
        acceptedSessionStorePath: JString,
        pairingCodePath: JString,
        deviceInfoJson: JString,
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
        match bridge.startWebAccessServer(
            bindAddress,
            token,
            shutdownToken,
            PathBuf::from(webRoot),
            acceptedSessionsJson,
            PathBuf::from(acceptedSessionStorePath),
            PathBuf::from(pairingCodePath),
            deviceInfo,
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
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_remotePairStart(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        baseUrl: JString,
        token: JString,
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
        match bridge.remotePairStart(baseUrl, token, clientDeviceInfo) {
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
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_currentPermissionRequest(
        env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(env, "null");
        };
        new_java_string(env, &bridge.currentPermissionRequest())
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_handlePermissionResult(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        permissionResult: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(env, &serde_json::json!({"ok": false}).to_string());
        };
        let permissionResult = match env.get_string(&permissionResult) {
            Ok(value) => String::from(value),
            Err(_) => {
                return new_java_string(env, &serde_json::json!({"ok": false}).to_string());
            }
        };
        new_java_string(env, &bridge.handlePermissionResult(&permissionResult))
    }

    fn new_java_string(mut env: JNIEnv, value: &str) -> jstring {
        env.new_string(value)
            .expect("JNI string allocation must succeed")
            .into_raw()
    }
}
