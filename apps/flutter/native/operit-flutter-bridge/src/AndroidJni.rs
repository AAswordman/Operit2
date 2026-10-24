use crate::BridgeExports::*;
use crate::BridgeExports::{last_create_error, panic_payload_message, set_last_create_error};
use crate::OperitFlutterBridge;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{jlong, jstring};
use jni::JNIEnv;
use std::ffi::CStr;
use std::path::PathBuf;
use std::sync::Arc;

#[path = "AndroidJniTaskScheduler.rs"]
mod task_scheduler;

/// Accepts the borrowed pipe descriptors delivered by the Android Binder endpoint.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_acceptPluginSdkPipes(
    mut env: JNIEnv,
    _class: JClass,
    reader: jni::sys::jint,
    writer: jni::sys::jint,
) {
    if let Err(error) = operit_host_android_native::acceptAndroidPluginSdkPipes(reader, writer) {
        let _ = env.throw_new("java/lang/IllegalStateException", error.to_string());
    }
}

/// Decodes an exact JNI boolean literal and rejects invalid owner input.
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

/// Creates the reference-counted runtime and installs Android's Java host services.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_create(
    mut env: JNIEnv,
    _class: JClass,
    runtime_root: JString,
    workspace_root: JString,
    device_model: JString,
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
    let device_model: String = match env.get_string(&device_model) {
        Ok(value) => value.into(),
        Err(error) => {
            set_last_create_error(format!("Android startup device model is invalid: {error}"));
            return 0;
        }
    };
    if device_model.trim().is_empty() {
        set_last_create_error("Android startup device model must not be empty".to_string());
        return 0;
    }
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
        operit_host_android_native::setAndroidHostSecretStoreBridge(java_vm, host.clone())
    {
        set_last_create_error(error.to_string());
        return 0;
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        OperitFlutterBridge::new_with_storage_roots(runtime_root, workspace_root, device_model)
    })) {
        Ok(Ok(bridge)) => {
            let installed = env
                .get_java_vm()
                .map_err(|error| error.to_string())
                .and_then(|vm| install_work_presentation(vm, host));
            if let Err(error) = installed {
                set_last_create_error(error);
                return 0;
            }
            Arc::into_raw(Arc::new(bridge)) as jlong
        }
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

/// Reads the client bootstrap record before the Android Runtime is created.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_runtimeBootstrapRead(
    mut env: JNIEnv,
    _class: JClass,
    default_runtime_root: JString,
) -> jstring {
    let default_runtime_root = match env.get_string(&default_runtime_root) {
        Ok(value) => PathBuf::from(String::from(value)),
        Err(error) => {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "value": null,
                    "error": format!("default Runtime root is invalid: {error}"),
                })
                .to_string(),
            );
        }
    };
    new_java_string(
        env,
        &crate::PlatformRuntimeAbi::RuntimeBootstrapStore::readNativeRuntimeBootstrapConfig(
            default_runtime_root,
        ),
    )
}

/// Writes the client bootstrap record before the Android Runtime is created.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_runtimeBootstrapWrite(
    mut env: JNIEnv,
    _class: JClass,
    default_runtime_root: JString,
    content: JString,
) -> jstring {
    let default_runtime_root = match env.get_string(&default_runtime_root) {
        Ok(value) => PathBuf::from(String::from(value)),
        Err(error) => {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": format!("default Runtime root is invalid: {error}"),
                })
                .to_string(),
            );
        }
    };
    let content = match env.get_string(&content) {
        Ok(value) => String::from(value),
        Err(error) => {
            return new_java_string(
                env,
                &serde_json::json!({
                    "ok": false,
                    "error": format!("runtime bootstrap config is invalid: {error}"),
                })
                .to_string(),
            );
        }
    };
    new_java_string(
        env,
        &crate::PlatformRuntimeAbi::RuntimeBootstrapStore::writeNativeRuntimeBootstrapConfig(
            default_runtime_root,
            &content,
        ),
    )
}

/// Releases the Android owner's reference while active Dart connections retain the runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_destroy(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    operit_flutter_bridge_destroy(handle as *mut OperitFlutterBridge);
}

/// Creates one direct Dart connection while preserving Android's runtime ownership.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_connectCoreFfi(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
) -> jstring {
    let descriptor = crate::PlatformRuntimeAbi::FfiTransport::operit_flutter_bridge_ffi_connect(
        handle as *const OperitFlutterBridge,
    );
    let result = new_java_string(env, CStr::from_ptr(descriptor).to_str().expect("FFI JSON"));
    operit_flutter_bridge_free_string(descriptor);
    result
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

/// Runs offline maintenance with only the Android secret-store bridge installed.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_dataRescue(
    mut env: JNIEnv,
    _class: JClass,
    default_runtime_root: JString,
    request: JString,
    host: JObject,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (|| -> Result<String, String> {
            let root = PathBuf::from(String::from(
                env.get_string(&default_runtime_root)
                    .map_err(|error| error.to_string())?,
            ));
            let request = String::from(
                env.get_string(&request)
                    .map_err(|error| error.to_string())?,
            );
            let vm = env.get_java_vm().map_err(|error| error.to_string())?;
            let host = env
                .new_global_ref(host)
                .map_err(|error| error.to_string())?;
            operit_host_android_native::setAndroidHostSecretStoreBridge(vm, host)
                .map_err(|error| error.to_string())?;
            Ok(crate::PlatformRuntimeAbi::NativeDataRescue::execute(
                root, &request,
            ))
        })()
    }));
    let response = match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => serde_json::json!({"ok": false, "error": error}).to_string(),
        Err(payload) => {
            serde_json::json!({"ok": false, "error": panic_payload_message(payload.as_ref())})
                .to_string()
        }
    };
    new_java_string(env, &response)
}

/// Stops a service notification's exact execution without routing through a Flutter engine.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_stopBackgroundWork(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    work_id: JString,
) -> jstring {
    let outcome = (|| -> Result<(), String> {
        let bridge = (handle as *const OperitFlutterBridge)
            .as_ref()
            .ok_or_else(|| "Background work has no Core owner".to_string())?;
        let work_id = String::from(
            env.get_string(&work_id)
                .map_err(|error| error.to_string())?,
        );
        let context = bridge.localCore.hostManager().clone();
        bridge.runHostRuntimeAsyncTask("stop-background-work", move || async move {
            operit_runtime::services::WorkActivityService::WorkActivityService::getInstance(
                &context,
            )
            .stopWork(work_id)
            .await
        })?
    })();
    let encoded = match outcome {
        Ok(()) => serde_json::json!({"ok": true}),
        Err(error) => serde_json::json!({"ok": false, "error": error}),
    };
    new_java_string(env, &encoded.to_string())
}

/// Owns the process notification observer independently of any Flutter engine.
static WORK_PRESENTATION_SUBSCRIPTION: std::sync::Mutex<Option<usize>> =
    std::sync::Mutex::new(None);

/// Delivers complete Core revisions to the Java service owner on every work transition.
pub(crate) fn install_work_presentation(
    vm: jni::JavaVM,
    owner: jni::objects::GlobalRef,
) -> Result<(), String> {
    let mut subscription = WORK_PRESENTATION_SUBSCRIPTION
        .lock()
        .map_err(|error| error.to_string())?;
    if subscription.is_some() {
        return Err("Android work presentation is already installed".to_string());
    }
    let id = operit_runtime::services::WorkActivityService::subscribeWorkUpdates(move |encoded| {
        let delivery = (|| -> Result<(), String> {
            let mut env = vm
                .attach_current_thread()
                .map_err(|error| error.to_string())?;
            let encoded = env.new_string(encoded).map_err(|error| error.to_string())?;
            let result = env.call_method(
                owner.as_obj(),
                "publishBackgroundWork",
                "(Ljava/lang/String;)V",
                &[jni::objects::JValue::Object(&jni::objects::JObject::from(
                    encoded,
                ))],
            );
            if let Err(error) = result {
                if env.exception_check().map_err(|error| error.to_string())? {
                    env.exception_describe()
                        .map_err(|error| error.to_string())?;
                    env.exception_clear().map_err(|error| error.to_string())?;
                }
                return Err(error.to_string());
            }
            Ok(())
        })();
        if let Err(error) = delivery {
            operit_util::AppLogger::AppLogger::e(
                "WorkPresentation",
                &format!("Android work delivery failed: {error}"),
            );
        }
    });
    *subscription = Some(id);
    Ok(())
}

/// Releases only the observer installed by this Android Core bridge.
pub(crate) fn release_work_presentation() {
    let id = WORK_PRESENTATION_SUBSCRIPTION
        .lock()
        .expect("Android work subscription mutex poisoned")
        .take();
    if let Some(id) = id {
        operit_runtime::services::WorkActivityService::unsubscribeWorkUpdates(id);
    }
}
