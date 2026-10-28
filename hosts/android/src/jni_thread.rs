use jni::{objects::JValue, AttachGuard, JavaVM};

/// Carries a native worker's name into Java when JNI creates its Thread peer.
/// Already attached Java threads retain the name assigned by their Java owner.
pub fn attachCurrentThread(vm: &JavaVM) -> jni::errors::Result<AttachGuard<'_>> {
    let wasAttached = vm.get_env().is_ok();
    let mut env = vm.attach_current_thread()?;
    if !wasAttached {
        let thread = std::thread::current();
        if let Some(name) = thread.name().filter(|name| name.starts_with("operit-")) {
            env.with_local_frame(4, |env| -> jni::errors::Result<()> {
                let current = env
                    .call_static_method(
                        "java/lang/Thread",
                        "currentThread",
                        "()Ljava/lang/Thread;",
                        &[],
                    )?
                    .l()?;
                let name = env.new_string(name)?;
                env.call_method(
                    current,
                    "setName",
                    "(Ljava/lang/String;)V",
                    &[JValue::Object(name.as_ref())],
                )?;
                Ok(())
            })?;
        }
    }
    Ok(env)
}
