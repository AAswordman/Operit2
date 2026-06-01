#![allow(non_snake_case)]

use std::collections::{HashMap, VecDeque};
use std::ffi::{c_char, CStr, CString};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use operit_core_proxy::LocalCoreProxy;
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
    AndroidFileSystemHost as NativeFileSystemHost, AndroidHttpHost as NativeHttpHost,
    AndroidManagedRuntimeHost as NativeManagedRuntimeHost,
    AndroidRuntimeStorageHost as NativeRuntimeStorageHost,
    AndroidSystemOperationHost as NativeSystemOperationHost,
    AndroidTerminalHost as NativeTerminalHost,
    AndroidWebVisitHost as NativeWebVisitHost,
};
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxFileSystemHost as NativeFileSystemHost, LinuxHttpHost as NativeHttpHost,
    LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxRuntimeStorageHost as NativeRuntimeStorageHost,
    LinuxSystemOperationHost as NativeSystemOperationHost,
    LinuxTerminalHost as NativeTerminalHost, LinuxWebVisitHost as NativeWebVisitHost,
};
#[cfg(target_arch = "wasm32")]
use operit_host_web::{
    WebFileSystemHost as NativeFileSystemHost, WebHttpHost as NativeHttpHost,
    WebManagedRuntimeHost as NativeManagedRuntimeHost,
    WebRuntimeStorageHost as NativeRuntimeStorageHost,
    WebSystemOperationHost as NativeSystemOperationHost, WebWebVisitHost as NativeWebVisitHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsFileSystemHost as NativeFileSystemHost, WindowsHttpHost as NativeHttpHost,
    WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
    WindowsRuntimeStorageHost as NativeRuntimeStorageHost,
    WindowsSystemOperationHost as NativeSystemOperationHost,
    WindowsTerminalHost as NativeTerminalHost,
    WindowsWebVisitHost as NativeWebVisitHost,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct OperitFlutterBridge {
    #[cfg(not(target_arch = "wasm32"))]
    runtime: tokio::runtime::Runtime,
    proxyCore: Mutex<LocalCoreProxy>,
    watchStreams: Mutex<HashMap<String, CoreEventStream>>,
    nextWatchStreamId: Mutex<u64>,
    approvalBridge: FlutterApprovalBridge,
    browserAutomationBridge: FlutterBrowserAutomationBridge,
    #[cfg(any(windows, target_os = "linux", target_os = "android"))]
    terminalHost: Arc<NativeTerminalHost>,
}

const PERMISSION_REQUEST_TIMEOUT_MS: u64 = 60_000;
const BROWSER_AUTOMATION_REQUEST_TIMEOUT_MS: u64 = 180_000;

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
    inner: Arc<BrowserAutomationInner>,
}

struct BrowserAutomationInner {
    state: Mutex<BrowserAutomationState>,
    changed: Condvar,
}

#[derive(Debug)]
struct BrowserAutomationState {
    queue: VecDeque<PendingBrowserAutomationRequest>,
    responses: HashMap<String, BrowserAutomationToolResponse>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct PendingBrowserAutomationRequest {
    requestId: String,
    toolName: String,
    chatId: String,
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
    fn new() -> Self {
        Self {
            inner: Arc::new(BrowserAutomationInner {
                state: Mutex::new(BrowserAutomationState {
                    queue: VecDeque::new(),
                    responses: HashMap::new(),
                }),
                changed: Condvar::new(),
            }),
        }
    }

    fn nextRequest(&self) -> Option<PendingBrowserAutomationRequest> {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("browser automation state mutex poisoned");
        state.queue.pop_front()
    }

    fn respond(&self, requestId: String, response: BrowserAutomationToolResponse) -> bool {
        let mut state = self
            .inner
            .state
            .lock()
            .expect("browser automation state mutex poisoned");
        state.responses.insert(requestId, response);
        self.inner.changed.notify_all();
        true
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
            chatId: request.chatId,
            parametersJson: request.parametersJson,
            requestedAtMillis: current_time_millis_u64(),
        };
        let mut state = self
            .inner
            .state
            .lock()
            .expect("browser automation state mutex poisoned");
        state.queue.push_back(pending);
        self.inner.changed.notify_all();

        let timeout = Duration::from_millis(BROWSER_AUTOMATION_REQUEST_TIMEOUT_MS);
        let startedAt = Instant::now();
        loop {
            if let Some(response) = state.responses.remove(&requestId) {
                if response.success {
                    return Ok(operit_host_api::BrowserAutomationResponse {
                        output: response.result,
                    });
                }
                return Err(operit_host_api::HostError::new(
                    response
                        .error
                        .unwrap_or_else(|| "Browser automation failed".to_string()),
                ));
            }
            let elapsed = startedAt.elapsed();
            if elapsed >= timeout {
                state.queue.retain(|item| item.requestId != requestId);
                return Err(operit_host_api::HostError::new(format!(
                    "Browser automation request timed out: {requestId}"
                )));
            }
            let wait = timeout.saturating_sub(elapsed);
            let (nextState, result) = self
                .inner
                .changed
                .wait_timeout(state, wait)
                .expect("browser automation state mutex poisoned");
            state = nextState;
            if result.timed_out() {
                state.queue.retain(|item| item.requestId != requestId);
                return Err(operit_host_api::HostError::new(format!(
                    "Browser automation request timed out: {requestId}"
                )));
            }
        }
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
        let approvalBridge = FlutterApprovalBridge::new();
        let browserAutomationBridge = FlutterBrowserAutomationBridge::new();
        #[cfg(any(windows, target_os = "linux", target_os = "android"))]
        let terminalHost = Arc::new(NativeTerminalHost::new());
        let mut core = create_local_core(
            storage_root,
            Some(Arc::new(browserAutomationBridge.clone())),
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
        Ok(Self {
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
            proxyCore: Mutex::new(core),
            watchStreams: Mutex::new(HashMap::new()),
            nextWatchStreamId: Mutex::new(1),
            approvalBridge,
            browserAutomationBridge,
            #[cfg(any(windows, target_os = "linux", target_os = "android"))]
            terminalHost,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let mut proxyCore = match self.proxyCore.lock() {
            Ok(core) => core,
            Err(error) => {
                return CoreCallResponse::err(
                    request.requestId,
                    CoreLinkError::internal(format!("runtime bridge lock poisoned: {error}")),
                );
            }
        };
        self.runtime.block_on(proxyCore.call(request))
    }

    #[cfg(target_arch = "wasm32")]
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let mut proxyCore = match self.proxyCore.lock() {
            Ok(core) => core,
            Err(error) => {
                return CoreCallResponse::err(
                    request.requestId,
                    CoreLinkError::internal(format!("runtime bridge lock poisoned: {error}")),
                );
            }
        };
        proxyCore.call(request).await
    }

    #[allow(non_snake_case)]
    fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        let mut proxyCore = self.proxyCore.lock().map_err(|error| {
            CoreLinkError::internal(format!("runtime bridge lock poisoned: {error}"))
        })?;
        #[cfg(target_arch = "wasm32")]
        {
            return proxyCore.watchSnapshotSync(request);
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.runtime.block_on(proxyCore.watchSnapshot(request))
    }

    fn hostDescriptor(&self) -> serde_json::Value {
        let mut proxyCore = self
            .proxyCore
            .lock()
            .expect("runtime bridge lock must not be poisoned");
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
        let mut proxyCore = self.proxyCore.lock().map_err(|error| {
            CoreLinkError::internal(format!("runtime bridge lock poisoned: {error}"))
        })?;
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

    fn nextBrowserAutomationRequest(&self) -> String {
        json_string(&self.browserAutomationBridge.nextRequest())
    }

    fn handleBrowserAutomationResult(&self, resultJson: &str) -> String {
        match parse_browser_automation_result(resultJson) {
            Ok((requestId, response)) => serde_json::json!({
                "ok": self.browserAutomationBridge.respond(requestId, response)
            })
            .to_string(),
            Err(error) => serde_json::json!({
                "ok": false,
                "error": error
            })
            .to_string(),
        }
    }

    #[cfg(any(windows, target_os = "linux", target_os = "android"))]
    fn startTerminalPty(&self, workingDir: &str, rows: u16, cols: u16) -> String {
        #[cfg(target_os = "android")]
        {
            return match self.terminalHost.startPtySession(workingDir, rows, cols) {
                Ok(sessionId) => serde_json::json!({
                    "ok": true,
                    "sessionId": sessionId
                })
                .to_string(),
                Err(error) => serde_json::json!({
                    "ok": false,
                    "error": error.message
                })
                .to_string(),
            };
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (workingDir, rows, cols);
            serde_json::json!({
                "ok": false,
                "error": "PTY bridge is implemented by the platform terminal host on Android"
            })
            .to_string()
        }
    }

    #[cfg(target_os = "android")]
    fn readTerminalPty(&self, sessionId: &str) -> String {
        match self.terminalHost.readPtySession(sessionId) {
            Ok(data) => serde_json::json!({
                "ok": true,
                "data": data
            })
            .to_string(),
            Err(error) => serde_json::json!({
                "ok": false,
                "error": error.message
            })
            .to_string(),
        }
    }

    #[cfg(target_os = "android")]
    fn writeTerminalPty(&self, sessionId: &str, data: &[u8]) -> String {
        match self.terminalHost.writePtySession(sessionId, data) {
            Ok(count) => serde_json::json!({
                "ok": true,
                "count": count
            })
            .to_string(),
            Err(error) => serde_json::json!({
                "ok": false,
                "error": error.message
            })
            .to_string(),
        }
    }

    #[cfg(target_os = "android")]
    fn resizeTerminalPty(&self, sessionId: &str, rows: u16, cols: u16) -> String {
        match self.terminalHost.resizePtySession(sessionId, rows, cols) {
            Ok(()) => serde_json::json!({ "ok": true }).to_string(),
            Err(error) => serde_json::json!({
                "ok": false,
                "error": error.message
            })
            .to_string(),
        }
    }

    #[cfg(target_os = "android")]
    fn pollTerminalPtyExit(&self, sessionId: &str) -> String {
        match self.terminalHost.pollPtyExitCode(sessionId) {
            Ok(exitCode) => serde_json::json!({
                "ok": true,
                "exitCode": exitCode
            })
            .to_string(),
            Err(error) => serde_json::json!({
                "ok": false,
                "error": error.message
            })
            .to_string(),
        }
    }

    #[cfg(target_os = "android")]
    fn closeTerminalPty(&self, sessionId: &str) -> String {
        match self.terminalHost.closePtySession(sessionId) {
            Ok(()) => serde_json::json!({ "ok": true }).to_string(),
            Err(error) => serde_json::json!({
                "ok": false,
                "error": error.message
            })
            .to_string(),
        }
    }

    #[cfg(target_os = "android")]
    fn terminalDebugInfo(&self, workingDir: &str) -> String {
        match self.terminalHost.terminalDebugInfo(workingDir) {
            Ok(info) => serde_json::json!({
                "ok": true,
                "info": info
            })
            .to_string(),
            Err(error) => serde_json::json!({
                "ok": false,
                "error": error.message
            })
            .to_string(),
        }
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
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
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
            Arc::new(NativeWebVisitHost::new()),
            Arc::new(NativeHttpHost::new()),
            Arc::new(NativeSystemOperationHost::new()),
            Arc::new(NativeManagedRuntimeHost::new()),
            runtimeStorageHost,
            runtimeSqliteHost,
        );
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
    }
    context = context.withTerminalHost(terminalHost);
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
    browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
) -> Result<LocalCoreProxy, String> {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new());
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let mut context =
        OperitApplicationContext::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
            Arc::new(NativeFileSystemHost::new()),
            Arc::new(NativeWebVisitHost::new()),
            Arc::new(NativeHttpHost::new()),
            Arc::new(NativeSystemOperationHost::new()),
            Arc::new(NativeManagedRuntimeHost::new()),
            runtimeStorageHost,
            runtimeSqliteHost,
        );
    if let Some(host) = browserAutomationHost {
        context = context.withBrowserAutomationHost(host);
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
    _browserAutomationHost: Option<Arc<dyn operit_host_api::BrowserAutomationHost>>,
    #[cfg(any(windows, target_os = "linux", target_os = "android"))]
    _terminalHost: Arc<NativeTerminalHost>,
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
pub unsafe extern "C" fn operit_flutter_bridge_next_browser_automation_request(
    handle: *mut OperitFlutterBridge,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr("null");
    }
    string_to_ptr((*handle).nextBrowserAutomationRequest())
}

#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_handle_browser_automation_result(
    handle: *mut OperitFlutterBridge,
    result_ptr: *const c_char,
) -> *mut c_char {
    if handle.is_null() {
        return string_to_ptr(serde_json::json!({"ok": false}).to_string());
    }
    if result_ptr.is_null() {
        return string_to_ptr(serde_json::json!({"ok": false}).to_string());
    }
    let resultJson = match CStr::from_ptr(result_ptr).to_str() {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::json!({
                    "ok": false,
                    "error": format!("browser automation result is not valid UTF-8: {error}")
                })
                .to_string(),
            );
        }
    };
    string_to_ptr((*handle).handleBrowserAutomationResult(resultJson))
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

    #[allow(non_snake_case)]
    pub fn nextBrowserAutomationRequest(&self) -> String {
        self.inner.nextBrowserAutomationRequest()
    }

    #[allow(non_snake_case)]
    pub fn handleBrowserAutomationResult(&self, resultJson: &str) -> String {
        self.inner.handleBrowserAutomationResult(resultJson)
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
    use jni::objects::{JByteArray, JClass, JString};
    use jni::sys::{jlong, jstring};
    use jni::JNIEnv;

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_create(
        mut env: JNIEnv,
        _class: JClass,
        storage_root: JString,
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_createError(
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_destroy(
        _env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) {
        operit_flutter_bridge_destroy(handle as *mut OperitFlutterBridge);
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_call(
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_watchSnapshot(
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_watchStream(
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_pollWatchStream(
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_closeWatchStream(
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_hostDescriptor(
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_currentPermissionRequest(
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
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_handlePermissionResult(
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

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_nextBrowserAutomationRequest(
        env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(env, "null");
        };
        new_java_string(env, &bridge.nextBrowserAutomationRequest())
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_handleBrowserAutomationResult(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        resultJson: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(env, &serde_json::json!({"ok": false}).to_string());
        };
        let resultJson = match env.get_string(&resultJson) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid JNI browser automation result: {error}")
                    })
                    .to_string(),
                );
            }
        };
        new_java_string(env, &bridge.handleBrowserAutomationResult(&resultJson))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_startTerminalPty(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        workingDir: JString,
        rows: jni::sys::jint,
        cols: jni::sys::jint,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": "runtime bridge is not initialized"
                })
                .to_string(),
            );
        };
        let workingDir = match env.get_string(&workingDir) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid terminal working directory: {error}")
                    })
                    .to_string(),
                );
            }
        };
        new_java_string(
            env,
            &bridge.startTerminalPty(&workingDir, rows as u16, cols as u16),
        )
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_readTerminalPty(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        sessionId: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": "runtime bridge is not initialized"
                })
                .to_string(),
            );
        };
        let sessionId = match env.get_string(&sessionId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid terminal session id: {error}")
                    })
                    .to_string(),
                );
            }
        };
        new_java_string(env, &bridge.readTerminalPty(&sessionId))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_writeTerminalPty(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        sessionId: JString,
        data: JByteArray,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": "runtime bridge is not initialized"
                })
                .to_string(),
            );
        };
        let sessionId = match env.get_string(&sessionId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid terminal session id: {error}")
                    })
                    .to_string(),
                );
            }
        };
        let data = match env.convert_byte_array(data) {
            Ok(value) => value,
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid terminal input bytes: {error}")
                    })
                    .to_string(),
                );
            }
        };
        new_java_string(env, &bridge.writeTerminalPty(&sessionId, &data))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_resizeTerminalPty(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        sessionId: JString,
        rows: jni::sys::jint,
        cols: jni::sys::jint,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": "runtime bridge is not initialized"
                })
                .to_string(),
            );
        };
        let sessionId = match env.get_string(&sessionId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid terminal session id: {error}")
                    })
                    .to_string(),
                );
            }
        };
        new_java_string(
            env,
            &bridge.resizeTerminalPty(&sessionId, rows as u16, cols as u16),
        )
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_pollTerminalPtyExit(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        sessionId: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": "runtime bridge is not initialized"
                })
                .to_string(),
            );
        };
        let sessionId = match env.get_string(&sessionId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid terminal session id: {error}")
                    })
                    .to_string(),
                );
            }
        };
        new_java_string(env, &bridge.pollTerminalPtyExit(&sessionId))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_closeTerminalPty(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        sessionId: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": "runtime bridge is not initialized"
                })
                .to_string(),
            );
        };
        let sessionId = match env.get_string(&sessionId) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid terminal session id: {error}")
                    })
                    .to_string(),
                );
            }
        };
        new_java_string(env, &bridge.closeTerminalPty(&sessionId))
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_ai_assistance_operit2_OperitRuntimeNative_terminalDebugInfo(
        mut env: JNIEnv,
        _class: JClass,
        handle: jlong,
        workingDir: JString,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": "runtime bridge is not initialized"
                })
                .to_string(),
            );
        };
        let workingDir = match env.get_string(&workingDir) {
            Ok(value) => String::from(value),
            Err(error) => {
                return new_java_string(
                    env,
                    &serde_json::json!({
                        "ok": false,
                        "error": format!("invalid terminal working directory: {error}")
                    })
                    .to_string(),
                );
            }
        };
        new_java_string(env, &bridge.terminalDebugInfo(&workingDir))
    }

    fn new_java_string(mut env: JNIEnv, value: &str) -> jstring {
        env.new_string(value)
            .expect("JNI string allocation must succeed")
            .into_raw()
    }
}
