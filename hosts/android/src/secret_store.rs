use std::sync::{Arc, Mutex, OnceLock};

use jni::objects::{GlobalRef, JByteArray, JObject, JValue};
use jni::JavaVM;
use operit_host_api::{HostError, HostResult};

pub(crate) struct AndroidHostSecretStoreBridge {
    pub(crate) vm: JavaVM,
    pub(crate) host: GlobalRef,
}

/// Returns the global Android host secret bridge slot.
fn androidHostSecretStoreBridgeSlot() -> &'static Mutex<Option<Arc<AndroidHostSecretStoreBridge>>> {
    static BRIDGE: OnceLock<Mutex<Option<Arc<AndroidHostSecretStoreBridge>>>> = OnceLock::new();
    BRIDGE.get_or_init(|| Mutex::new(None))
}

/// Converts a JNI error into a host secret store error.
fn jniHostError(action: &str, error: impl std::fmt::Display) -> HostError {
    HostError::new(format!(
        "Android host secret store failed while {action}: {error}"
    ))
}

/// Registers the Java host used by Android host secret store calls.
pub fn setAndroidHostSecretStoreBridge(vm: JavaVM, host: GlobalRef) -> HostResult<()> {
    let mut guard = androidHostSecretStoreBridgeSlot()
        .lock()
        .map_err(|_| HostError::new("Android host secret store bridge lock is poisoned"))?;
    *guard = Some(Arc::new(AndroidHostSecretStoreBridge { vm, host }));
    Ok(())
}

/// Clears the Java host used by Android host secret store calls.
pub fn clearAndroidHostSecretStoreBridge() {
    let mut guard = androidHostSecretStoreBridgeSlot()
        .lock()
        .expect("Android host secret store bridge lock must not be poisoned");
    *guard = None;
}

/// Returns the registered Android host secret store bridge.
pub(crate) fn androidHostSecretStoreBridge() -> HostResult<Arc<AndroidHostSecretStoreBridge>> {
    let guard = androidHostSecretStoreBridgeSlot()
        .lock()
        .map_err(|_| HostError::new("Android host secret store bridge lock is poisoned"))?;
    guard
        .clone()
        .ok_or_else(|| HostError::new("Android host secret store bridge is not registered"))
}

impl AndroidHostSecretStoreBridge {
    /// Reads secret bytes from the Java Android host.
    pub fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|error| jniHostError("attaching current thread", error))?;
        let key = env
            .new_string(key)
            .map_err(|error| jniHostError("allocating read key", error))?;
        let keyObject = JObject::from(key);
        let value = env
            .call_method(
                self.host.as_obj(),
                "readHostSecret",
                "(Ljava/lang/String;)[B",
                &[JValue::Object(&keyObject)],
            )
            .map_err(|error| jniHostError("reading secret", error))?;
        let object = value
            .l()
            .map_err(|error| jniHostError("reading secret result", error))?;
        if object.is_null() {
            return Ok(None);
        }
        let bytes = env
            .convert_byte_array(JByteArray::from(object))
            .map_err(|error| jniHostError("copying secret bytes", error))?;
        Ok(Some(bytes))
    }

    /// Writes secret bytes through the Java Android host.
    pub fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|error| jniHostError("attaching current thread", error))?;
        let key = env
            .new_string(key)
            .map_err(|error| jniHostError("allocating write key", error))?;
        let keyObject = JObject::from(key);
        let contentArray = env
            .byte_array_from_slice(content)
            .map_err(|error| jniHostError("allocating secret bytes", error))?;
        let contentObject = JObject::from(contentArray);
        env.call_method(
            self.host.as_obj(),
            "writeHostSecret",
            "(Ljava/lang/String;[B)V",
            &[JValue::Object(&keyObject), JValue::Object(&contentObject)],
        )
        .map_err(|error| jniHostError("writing secret", error))?;
        Ok(())
    }

    /// Deletes secret bytes through the Java Android host.
    pub fn deleteSecret(&self, key: &str) -> HostResult<()> {
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|error| jniHostError("attaching current thread", error))?;
        let key = env
            .new_string(key)
            .map_err(|error| jniHostError("allocating delete key", error))?;
        let keyObject = JObject::from(key);
        env.call_method(
            self.host.as_obj(),
            "deleteHostSecret",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&keyObject)],
        )
        .map_err(|error| jniHostError("deleting secret", error))?;
        Ok(())
    }
}
