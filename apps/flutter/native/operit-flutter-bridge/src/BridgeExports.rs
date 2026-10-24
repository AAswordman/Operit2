//! Runtime lifecycle and host exports, plus the browser MessagePack ABI.
//! Native Core calls and streams are owned exclusively by FfiTransport ABI v2.

use crate::BridgeCodec::*;
use crate::OperitFlutterBridge;
#[cfg(target_arch = "wasm32")]
use operit_link::{CoreEvent, CoreLinkError};
use std::any::Any;
use std::ffi::{c_char, CStr, CString};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use crate::PlatformRuntimeAbi::RuntimeBootstrapStore::{
    readNativeRuntimeBootstrapConfig, writeNativeRuntimeBootstrapConfig,
};

/// Creates a reference-counted runtime using the platform storage roots.
#[no_mangle]
#[cfg(not(any(target_env = "ohos", target_os = "android")))]
pub extern "C" fn operit_flutter_bridge_create() -> *mut OperitFlutterBridge {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(OperitFlutterBridge::new)) {
        Ok(Ok(bridge)) => Arc::into_raw(Arc::new(bridge)) as *mut OperitFlutterBridge,
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

/// Creates a C-hosted runtime; Android creation is owned by its JNI host entry point.
#[no_mangle]
#[cfg(not(any(target_env = "ohos", target_os = "android")))]
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
        Ok(Ok(bridge)) => Arc::into_raw(Arc::new(bridge)) as *mut OperitFlutterBridge,
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
        Ok(bridge) => Arc::into_raw(Arc::new(bridge)) as *mut OperitFlutterBridge,
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

/// Reads the client bootstrap record before a native Runtime is created.
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_runtime_bootstrap_read(
    default_runtime_root: *const c_char,
) -> *mut c_char {
    if default_runtime_root.is_null() {
        return string_to_ptr(
            serde_json::json!({
                "ok": false,
                "value": null,
                "error": "default Runtime root pointer is null",
            })
            .to_string(),
        );
    }
    let default_runtime_root = match CStr::from_ptr(default_runtime_root).to_str() {
        Ok(value) => PathBuf::from(value),
        Err(error) => {
            return string_to_ptr(
                serde_json::json!({
                    "ok": false,
                    "value": null,
                    "error": format!("default Runtime root is not valid UTF-8: {error}"),
                })
                .to_string(),
            );
        }
    };
    string_to_ptr(readNativeRuntimeBootstrapConfig(default_runtime_root))
}

/// Writes the client bootstrap record before a native Runtime is created.
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_runtime_bootstrap_write(
    default_runtime_root: *const c_char,
    content: *const c_char,
) -> *mut c_char {
    if default_runtime_root.is_null() || content.is_null() {
        return string_to_ptr(
            serde_json::json!({
                "ok": false,
                "error": "runtime bootstrap write pointers must not be null",
            })
            .to_string(),
        );
    }
    let default_runtime_root = match CStr::from_ptr(default_runtime_root).to_str() {
        Ok(value) => PathBuf::from(value),
        Err(error) => {
            return string_to_ptr(
                serde_json::json!({
                    "ok": false,
                    "error": format!("default Runtime root is not valid UTF-8: {error}"),
                })
                .to_string(),
            );
        }
    };
    let content = match CStr::from_ptr(content).to_str() {
        Ok(value) => value,
        Err(error) => {
            return string_to_ptr(
                serde_json::json!({
                    "ok": false,
                    "error": format!("runtime bootstrap config is not valid UTF-8: {error}"),
                })
                .to_string(),
            );
        }
    };
    string_to_ptr(writeNativeRuntimeBootstrapConfig(
        default_runtime_root,
        content,
    ))
}

/// Releases one host reference without invalidating retained FFI connections.
#[no_mangle]
pub unsafe extern "C" fn operit_flutter_bridge_destroy(handle: *mut OperitFlutterBridge) {
    if !handle.is_null() {
        drop(Arc::from_raw(handle));
    }
}

/// Converts a Rust panic payload into the stable text carried by crash reports.
pub(crate) fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "non-string panic payload".to_string()
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

/// Decodes and opens one compact wasm CoreProxy push stream.
#[cfg(target_arch = "wasm32")]
async fn bridge_push_open_async(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request = match decode_native_push_open_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<String, _>(error)),
    };
    native_result_vec(handle.pushOpen(request).await)
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

#[cfg(target_arch = "wasm32")]
/// Decodes and opens one compact wasm CoreProxy watch stream.
async fn bridge_watch_stream_wasm(
    handle: &OperitFlutterBridge,
    request_bytes: &[u8],
    onEvent: Function,
) -> Vec<u8> {
    let (subscription_id, request) = match decode_native_watch_stream_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<String, _>(error)),
    };
    native_result_vec(handle.watchStream(subscription_id, request, onEvent).await)
}

/// Decodes and reads one compact wasm CoreProxy watch snapshot.
#[cfg(target_arch = "wasm32")]
async fn bridge_watch_snapshot_async(
    handle: &OperitFlutterBridge,
    request_bytes: &[u8],
) -> Vec<u8> {
    let request = match decode_native_watch_snapshot_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<CoreEvent, _>(error)),
    };
    native_result_vec(
        handle
            .watchSnapshot(request)
            .await
            .map(native_watch_event_payload),
    )
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

    /// Delivers owner events through the same application ingress as native hosts.
    #[allow(non_snake_case)]
    pub async fn emitRuntimeEvent(&self, eventJson: &str) -> String {
        self.inner.emitRuntimeEvent(eventJson).await
    }

    pub async fn call(&self, request: &[u8]) -> Vec<u8> {
        bridge_native_call_async(&self.inner, request).await
    }

    /// Opens one wasm Link push stream.
    #[allow(non_snake_case)]
    pub async fn pushOpen(&self, request: &[u8]) -> Vec<u8> {
        bridge_push_open_async(&self.inner, request).await
    }

    /// Dispatches one wasm Link push item.
    #[allow(non_snake_case)]
    pub async fn pushItem(&self, item: &[u8]) -> Vec<u8> {
        bridge_push_item_async(&self.inner, item).await
    }

    /// Closes one wasm Link push stream.
    #[allow(non_snake_case)]
    pub async fn pushClose(&self, pushId: &str) -> Vec<u8> {
        native_result_vec(self.inner.pushClose(pushId).await)
    }

    /// Reads one wasm Link watch snapshot.
    #[allow(non_snake_case)]
    pub async fn watchSnapshot(&self, request: &[u8]) -> Vec<u8> {
        bridge_watch_snapshot_async(&self.inner, request).await
    }

    /// Opens one wasm Link watch stream.
    #[allow(non_snake_case)]
    pub async fn watchStream(&self, request: &[u8], onEvent: Function) -> Vec<u8> {
        bridge_watch_stream_wasm(&self.inner, request, onEvent).await
    }

    #[allow(non_snake_case)]
    pub fn closeWatchStream(&self, subscriptionId: &str) -> Vec<u8> {
        native_result_vec(self.inner.closeWatchStream(subscriptionId))
    }
}

fn string_to_ptr(value: impl Into<String>) -> *mut c_char {
    let sanitized = value.into().replace('\0', "");
    CString::new(sanitized)
        .expect("sanitized bridge string must not contain nul")
        .into_raw()
}

/// Returns the most recent explicit creation failure reported by the ABI boundary.
pub(crate) fn last_create_error() -> &'static Mutex<String> {
    static LAST_CREATE_ERROR: OnceLock<Mutex<String>> = OnceLock::new();
    LAST_CREATE_ERROR.get_or_init(|| Mutex::new(String::new()))
}

/// Records a creation failure without attempting another startup configuration.
pub(crate) fn set_last_create_error(value: String) {
    *last_create_error()
        .lock()
        .expect("Create error lock must not be poisoned") = value;
}

/// Runs offline data rescue without allocating or starting an OperitFlutterBridge.
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_data_rescue(
    default_runtime_root: *const c_char,
    request: *const c_char,
) -> *mut c_char {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if default_runtime_root.is_null() || request.is_null() {
            return serde_json::json!({"ok": false, "error": "Data rescue arguments must not be null"}).to_string();
        }
        let root = match CStr::from_ptr(default_runtime_root).to_str() {
            Ok(value) => PathBuf::from(value),
            Err(error) => return serde_json::json!({"ok": false, "error": error.to_string()}).to_string(),
        };
        let request = match CStr::from_ptr(request).to_str() {
            Ok(value) => value,
            Err(error) => return serde_json::json!({"ok": false, "error": error.to_string()}).to_string(),
        };
        crate::PlatformRuntimeAbi::NativeDataRescue::execute(root, request)
    }));
    let encoded = match result {
        Ok(value) => value,
        Err(payload) => serde_json::json!({"ok": false, "error": format!("Data rescue panic: {}", panic_payload_message(payload.as_ref()))}).to_string(),
    };
    CString::new(encoded).expect("JSON must not contain raw NUL").into_raw()
}

/// Exposes the independent browser storage engine without creating a Core tree.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = operitDataRescue)]
pub fn operit_data_rescue(request: &str) -> String {
    operit_host_web::executeDataRescue(request)
}
