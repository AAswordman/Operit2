use std::sync::{Arc, Mutex, OnceLock};

use jni::objects::{GlobalRef, JByteArray, JObject, JString, JValue};
use jni::JavaVM;
use operit_host_api::{HostError, HostResult, HostRuntimeEventSchedule};

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
    crate::runtime_event_scheduler::clearAndroidHostRuntimeEventScheduleSink();
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

/// Reads the system language code from the registered Android runtime host.
pub(crate) fn androidHostSystemLanguageCode() -> HostResult<String> {
    androidHostSecretStoreBridge()?.systemLanguageCode()
}

impl AndroidHostSecretStoreBridge {
    /// Reads the system language code from the Java Android runtime host.
    pub fn systemLanguageCode(&self) -> HostResult<String> {
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|error| jniHostError("attaching current thread", error))?;
        let value = env
            .call_method(
                self.host.as_obj(),
                "systemLanguageCode",
                "()Ljava/lang/String;",
                &[],
            )
            .map_err(|error| jniHostError("reading system language code", error))?;
        let object = value
            .l()
            .map_err(|error| jniHostError("reading system language code result", error))?;
        if object.is_null() {
            return Err(HostError::new(
                "Android runtime host returned a null system language code",
            ));
        }
        let language_object = JString::from(object);
        let language = env
            .get_string(&language_object)
            .map_err(|error| jniHostError("decoding system language code", error))?;
        Ok(String::from(language))
    }

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

    /// Reconciles Android AlarmManager schedules through the application host.
    pub fn replaceHostRuntimeEventSchedules(
        &self,
        schedules: &[HostRuntimeEventSchedule],
    ) -> HostResult<()> {
        let mut env = self
            .vm
            .attach_current_thread()
            .map_err(|error| jniHostError("attaching current thread", error))?;
        let schedulesJson = serde_json::to_string(schedules)
            .map_err(|error| jniHostError("encoding runtime event schedules", error))?;
        let schedulesJson = env
            .new_string(schedulesJson)
            .map_err(|error| jniHostError("allocating runtime event schedules", error))?;
        let schedulesObject = JObject::from(schedulesJson);
        env.call_method(
            self.host.as_obj(),
            "replaceHostRuntimeEventSchedules",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&schedulesObject)],
        )
        .map_err(|error| jniHostError("reconciling runtime event schedules", error))?;
        Ok(())
    }
}
