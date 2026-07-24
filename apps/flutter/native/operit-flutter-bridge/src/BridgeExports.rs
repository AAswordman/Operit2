use super::*;

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
pub(crate) fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
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
pub(crate) fn bridge_native_call(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request = match decode_native_call_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<operit_link::CoreValue, _>(error)),
    };
    native_result_vec(handle.call(request).result)
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
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn bridge_push_open(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let request = match decode_native_push_open_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<String, _>(error)),
    };
    native_result_vec(handle.pushOpen(request))
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

/// Decodes and dispatches one compact native CoreProxy push item.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn bridge_push_item(handle: &OperitFlutterBridge, item_bytes: &[u8]) -> Vec<u8> {
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
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
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
pub(crate) fn bridge_watch_stream(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
    let (subscription_id, request) = match decode_native_watch_stream_request(request_bytes) {
        Ok(request) => request,
        Err(error) => return native_result_vec(Err::<String, _>(error)),
    };
    native_result_vec(handle.watchStream(subscription_id, request))
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

/// Wakes the native watch-channel reader during host shutdown.
#[no_mangle]
#[cfg(not(target_arch = "wasm32"))]
pub unsafe extern "C" fn operit_flutter_bridge_close_watch_channel(
    handle: *mut OperitFlutterBridge,
) {
    if !handle.is_null() {
        (*handle).watchChannel.close();
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
        Ok(deviceId) => {
            string_to_ptr(&serde_json::json!({"ok": true, "deviceId": deviceId}).to_string())
        }
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
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn bridge_watch_snapshot(handle: &OperitFlutterBridge, request_bytes: &[u8]) -> Vec<u8> {
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
