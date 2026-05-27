#![allow(non_snake_case)]

use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use operit_core_proxy::LocalCoreProxy;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkClient, CoreLinkError,
    CoreRequestId, CoreWatchRequest,
};
use operit_runtime::api::chat::enhance::ConversationService::ConversationService;
use operit_runtime::api::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use operit_runtime::api::chat::EnhancedAIService::EnhancedAIService;
use operit_runtime::core::application::OperitApplication::OperitApplication;
use operit_runtime::core::application::OperitApplicationContext::OperitApplicationContext;

#[cfg(target_os = "android")]
use operit_host_android_native::{
    AndroidFileSystemHost as NativeFileSystemHost,
    AndroidManagedRuntimeHost as NativeManagedRuntimeHost,
    AndroidRuntimeStorageHost as NativeRuntimeStorageHost,
    AndroidSystemOperationHost as NativeSystemOperationHost,
    AndroidWebVisitHost as NativeWebVisitHost,
};
#[cfg(target_os = "linux")]
use operit_host_linux_native::{
    LinuxFileSystemHost as NativeFileSystemHost,
    LinuxManagedRuntimeHost as NativeManagedRuntimeHost,
    LinuxRuntimeStorageHost as NativeRuntimeStorageHost,
    LinuxSystemOperationHost as NativeSystemOperationHost, LinuxWebVisitHost as NativeWebVisitHost,
};
#[cfg(windows)]
use operit_host_windows_native::{
    WindowsFileSystemHost as NativeFileSystemHost,
    WindowsManagedRuntimeHost as NativeManagedRuntimeHost,
    WindowsRuntimeStorageHost as NativeRuntimeStorageHost,
    WindowsSystemOperationHost as NativeSystemOperationHost,
    WindowsWebVisitHost as NativeWebVisitHost,
};
#[cfg(target_arch = "wasm32")]
use operit_host_web::{
    WebFileSystemHost as NativeFileSystemHost,
    WebManagedRuntimeHost as NativeManagedRuntimeHost,
    WebRuntimeStorageHost as NativeRuntimeStorageHost,
    WebSystemOperationHost as NativeSystemOperationHost,
    WebWebVisitHost as NativeWebVisitHost,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct OperitFlutterBridge {
    runtime: tokio::runtime::Runtime,
    proxyCore: Mutex<LocalCoreProxy>,
    watchStreams: Mutex<HashMap<String, CoreEventStream>>,
}

impl OperitFlutterBridge {
    fn new() -> Result<Self, String> {
        Self::new_with_storage_root(None)
    }

    fn new_with_storage_root(storage_root: Option<PathBuf>) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let mut runtimeBuilder = tokio::runtime::Builder::new_multi_thread();
        #[cfg(target_arch = "wasm32")]
        let mut runtimeBuilder = tokio::runtime::Builder::new_current_thread();
        let runtime = runtimeBuilder
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        let mut core = create_local_core(storage_root)?;
        core.localApplicationMut().onCreate()?;
        let mainCore = core
            .localApplicationMut()
            .chatRuntimeHolder
            .getCore(ChatRuntimeSlot::MAIN);
        mainCore.enhancedAiService = Some(EnhancedAIService::new(ConversationService));
        Ok(Self {
            runtime,
            proxyCore: Mutex::new(core),
            watchStreams: Mutex::new(HashMap::new()),
        })
    }

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

    #[allow(non_snake_case)]
    fn watchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<operit_link::CoreEvent, CoreLinkError> {
        let mut proxyCore = self.proxyCore.lock().map_err(|error| {
            CoreLinkError::internal(format!("runtime bridge lock poisoned: {error}"))
        })?;
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
            "runtimeStorageHost": context.runtimeStorageHost.is_some(),
            "runtimeSqliteHost": context.runtimeSqliteHost.is_some(),
        })
    }

    fn watchStream(&self, request: CoreWatchRequest) -> Result<String, CoreLinkError> {
        let mut proxyCore = self.proxyCore.lock().map_err(|error| {
            CoreLinkError::internal(format!("runtime bridge lock poisoned: {error}"))
        })?;
        let receiver = self.runtime.block_on(proxyCore.watch(request))?;
        let subscriptionId = format!(
            "flutter-watch-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time must be after UNIX_EPOCH")
                .as_nanos()
        );
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
}

#[cfg(any(windows, target_os = "linux", target_os = "android"))]
fn create_local_core(storage_root: Option<PathBuf>) -> Result<LocalCoreProxy, String> {
    let root_dir = match storage_root {
        Some(root_dir) => root_dir,
        None => default_native_storage_root()?,
    };
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new(root_dir));
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let application = OperitApplication::newWithContext(
        OperitApplicationContext::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
            Arc::new(NativeFileSystemHost::new()),
            Arc::new(NativeWebVisitHost::new()),
            Arc::new(NativeSystemOperationHost::new()),
            Arc::new(NativeManagedRuntimeHost::new()),
            runtimeStorageHost,
            runtimeSqliteHost,
        ),
    );
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
fn create_local_core(_storage_root: Option<PathBuf>) -> Result<LocalCoreProxy, String> {
    let runtimeStorageHost = Arc::new(NativeRuntimeStorageHost::new());
    let runtimeSqliteHost = runtimeStorageHost.clone();
    let application = OperitApplication::newWithContext(
        OperitApplicationContext::withFileSystemWebVisitSystemOperationAndManagedRuntimeHosts(
            Arc::new(NativeFileSystemHost::new()),
            Arc::new(NativeWebVisitHost::new()),
            Arc::new(NativeSystemOperationHost::new()),
            Arc::new(NativeManagedRuntimeHost::new()),
            runtimeStorageHost,
            runtimeSqliteHost,
        ),
    );
    Ok(LocalCoreProxy::new(application))
}

#[cfg(not(any(
    windows,
    target_os = "linux",
    target_os = "android",
    target_arch = "wasm32"
)))]
fn create_local_core(_storage_root: Option<PathBuf>) -> Result<LocalCoreProxy, String> {
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

fn bridge_call_json(handle: &mut OperitFlutterBridge, request_bytes: &[u8]) -> String {
    let request: CoreCallRequest = match serde_json::from_slice(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("[OperitFlutterBridge] call invalid request: {error}");
            return error_response_string(
                "flutter-bridge-invalid-request",
                format!("invalid core request: {error}"),
            );
        }
    };
    let requestId = request.requestId.0.clone();
    let target = request.targetPath.key();
    let method = request.methodName.clone();
    eprintln!(
        "[OperitFlutterBridge] call -> {target}.{method} id={requestId} args={}",
        arg_keys(&request.args)
    );
    let response = handle.call(request);
    match &response.result {
        Ok(value) => {
            eprintln!(
                "[OperitFlutterBridge] call <- ok {target}.{method} id={requestId} value={}",
                value_kind(value)
            );
        }
        Err(error) => {
            eprintln!("[OperitFlutterBridge] call <- err {target}.{method} id={requestId} {error}");
        }
    }
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

fn bridge_watch_stream_json(handle: &mut OperitFlutterBridge, request_bytes: &[u8]) -> String {
    let request: CoreWatchRequest = match serde_json::from_slice(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("[OperitFlutterBridge] watchStream invalid request: {error}");
            return serde_json::to_string(&CoreLinkError::internal(format!(
                "invalid core watch request: {error}"
            )))
            .expect("CoreLinkError must serialize");
        }
    };
    let requestId = request.requestId.0.clone();
    let target = request.targetPath.key();
    let property = request.propertyName.clone();
    eprintln!(
        "[OperitFlutterBridge] watchStream -> {target}.{property} id={requestId} args={}",
        arg_keys(&request.args)
    );
    match handle.watchStream(request) {
        Ok(subscriptionId) => {
            eprintln!(
                "[OperitFlutterBridge] watchStream <- subscribed {target}.{property} id={requestId} subscription={subscriptionId}"
            );
            serde_json::json!({ "subscriptionId": subscriptionId }).to_string()
        }
        Err(error) => {
            eprintln!(
                "[OperitFlutterBridge] watchStream <- err {target}.{property} id={requestId} {error}"
            );
            serde_json::to_string(&error).expect("CoreLinkError must serialize")
        }
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

fn bridge_watch_snapshot_json(handle: &mut OperitFlutterBridge, request_bytes: &[u8]) -> String {
    let request: CoreWatchRequest = match serde_json::from_slice(request_bytes) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("[OperitFlutterBridge] watchSnapshot invalid request: {error}");
            return serde_json::to_string(&CoreLinkError::internal(format!(
                "invalid core watch request: {error}"
            )))
            .expect("CoreLinkError must serialize");
        }
    };
    let requestId = request.requestId.0.clone();
    let target = request.targetPath.key();
    let property = request.propertyName.clone();
    eprintln!(
        "[OperitFlutterBridge] watchSnapshot -> {target}.{property} id={requestId} args={}",
        arg_keys(&request.args)
    );
    match handle.watchSnapshot(request) {
        Ok(event) => {
            eprintln!(
                "[OperitFlutterBridge] watchSnapshot <- ok {target}.{property} id={requestId}"
            );
            json_string(&event)
        }
        Err(error) => {
            eprintln!(
                "[OperitFlutterBridge] watchSnapshot <- err {target}.{property} id={requestId} {error}"
            );
            serde_json::to_string(&error).expect("CoreLinkError must serialize")
        }
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
        OperitFlutterBridge::new()
            .map(|inner| OperitFlutterBridgeWasm { inner })
            .map_err(|error| JsValue::from_str(&error))
    }

    pub fn call(&mut self, request: &str) -> String {
        bridge_call_json(&mut self.inner, request.as_bytes())
    }

    #[allow(non_snake_case)]
    pub fn watchSnapshot(&mut self, request: &str) -> String {
        bridge_watch_snapshot_json(&mut self.inner, request.as_bytes())
    }

    #[allow(non_snake_case)]
    pub fn watchStream(&mut self, request: &str) -> String {
        bridge_watch_stream_json(&mut self.inner, request.as_bytes())
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

fn arg_keys(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(object) => object.keys().cloned().collect::<Vec<_>>().join(","),
        other => value_kind(other).to_string(),
    }
}

fn value_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn string_to_ptr(value: impl Into<String>) -> *mut c_char {
    let sanitized = value.into().replace('\0', "");
    CString::new(sanitized)
        .expect("sanitized bridge string must not contain nul")
        .into_raw()
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
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_create(
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
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_createError(
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
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_destroy(
        _env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) {
        operit_flutter_bridge_destroy(handle as *mut OperitFlutterBridge);
    }

    #[no_mangle]
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_call(
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
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_watchSnapshot(
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
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_watchStream(
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
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_pollWatchStream(
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
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_closeWatchStream(
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
    pub unsafe extern "system" fn Java_com_operit_operit2_OperitRuntimeNative_hostDescriptor(
        env: JNIEnv,
        _class: JClass,
        handle: jlong,
    ) -> jstring {
        let Some(bridge) = (handle as *mut OperitFlutterBridge).as_ref() else {
            return new_java_string(env, "{\"error\":\"runtime bridge is not initialized\"}");
        };
        new_java_string(env, &bridge.hostDescriptor().to_string())
    }

    fn new_java_string(mut env: JNIEnv, value: &str) -> jstring {
        env.new_string(value)
            .expect("JNI string allocation must succeed")
            .into_raw()
    }
}
