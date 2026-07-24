use crate::*;
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
    if let Err(error) = operit_host_android_native::setAndroidHostSecretStoreBridge(java_vm, host) {
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
            &native_result_error_vec("flutter-bridge-null", "runtime bridge is not initialized"),
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
            &native_result_error_vec("flutter-bridge-null", "runtime bridge is not initialized"),
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
            &native_result_error_vec("flutter-bridge-null", "runtime bridge is not initialized"),
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
            &native_result_error_vec("flutter-bridge-null", "runtime bridge is not initialized"),
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
            &native_result_error_vec("flutter-bridge-null", "runtime bridge is not initialized"),
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
            &native_result_error_vec("flutter-bridge-null", "runtime bridge is not initialized"),
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
            &native_result_error_vec("flutter-bridge-null", "runtime bridge is not initialized"),
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
