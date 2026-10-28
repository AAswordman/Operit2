//! Android JNI dispatch into the native Host scheduler.
#![allow(non_snake_case)]
use jni::objects::{JClass, JObject, JObjectArray};
use jni::JNIEnv;

/// Retains one Java task and invokes it only while a shared Host worker is attached to the VM.
fn retainedJavaTask(
    env: &mut JNIEnv,
    task: JObject,
) -> Result<operit_host_api::HostRuntimeTask, String> {
    if task.is_null() {
        return Err("Host Runnable must not be null".into());
    }
    let task = env
        .new_global_ref(task)
        .map_err(|error| error.to_string())?;
    let vm = env.get_java_vm().map_err(|error| error.to_string())?;
    Ok(Box::new(move || {
        let result = (|| -> jni::errors::Result<()> {
            let mut env = operit_host_android_native::attachCurrentThread(&vm)?;
            let result = env.with_local_frame(16, |env| -> jni::errors::Result<()> {
                env.call_static_method("java/lang/Thread", "interrupted", "()Z", &[])?;
                let result = env.call_method(task.as_obj(), "run", "()V", &[]);
                if env.exception_check()? {
                    env.exception_describe()?;
                    env.exception_clear()?;
                }
                env.call_static_method("java/lang/Thread", "interrupted", "()Z", &[])?;
                result.map(|_| ())
            });
            result
        })();
        if let Err(error) = result {
            eprintln!("Host Java task failed: {error}");
        }
    }))
}

/// Allocates Java callbacks from Host-owned capacity independent of waiting Rust callers.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_scheduleRuntimeTask(
    mut env: JNIEnv,
    _class: JClass,
    task: JObject,
) {
    let result = retainedJavaTask(&mut env, task).and_then(|task| {
        operit_host_native_common::NativeHostRuntimeTaskSchedulerHost::new()
            .scheduleOwnerTask("android-java-task", task)
            .map_err(|error| error.to_string())
    });
    if let Err(error) = result {
        let _ = env.throw_new("java/util/concurrent/RejectedExecutionException", error);
    }
}

/// Atomically reserves blocking service capacity for all components of a Java pipeline.
#[no_mangle]
pub unsafe extern "system" fn Java_app_operit_OperitRuntimeNative_scheduleRuntimeServices(
    mut env: JNIEnv,
    _class: JClass,
    tasks: JObjectArray,
) {
    let result = (|| -> Result<(), String> {
        let count = env
            .get_array_length(&tasks)
            .map_err(|error| error.to_string())?;
        let mut retained = Vec::with_capacity(count as usize);
        for index in 0..count {
            let task = env
                .get_object_array_element(&tasks, index)
                .map_err(|error| error.to_string())?;
            retained.push(retainedJavaTask(&mut env, task)?);
        }
        operit_host_native_common::NativeHostRuntimeTaskSchedulerHost::new()
            .scheduleServiceTasks("android-java-services", retained)
            .map_err(|error| error.to_string())
    })();
    if let Err(error) = result {
        let _ = env.throw_new("java/util/concurrent/RejectedExecutionException", error);
    }
}
