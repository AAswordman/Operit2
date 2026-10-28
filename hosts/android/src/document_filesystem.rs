use operit_host_api::{HostError, HostResult};
use serde_json::Value;

/// Executes resource operations on the Java host already owned by this runtime.
/// JVM exceptions are cleared on error so they cannot poison the worker thread.
#[cfg(target_os = "android")]
pub(crate) fn call(request: Value) -> HostResult<Value> {
    use jni::objects::{JObject, JString, JValue};
    let bridge = crate::secret_store::androidHostSecretStoreBridge()?;
    let mut env = crate::attachCurrentThread(&bridge.vm).map_err(|e| HostError::new(e.to_string()))?;
    let encoded = env.new_string(request.to_string()).map_err(|e| HostError::new(e.to_string()))?;
    let argument = JObject::from(encoded);
    let result = env.call_method(bridge.host.as_obj(), "fileSystemResourceOperation",
        "(Ljava/lang/String;)Ljava/lang/String;", &[JValue::Object(&argument)]);
    let value = match result {
        Ok(value) => value,
        Err(error) => {
            let _ = env.exception_clear();
            return Err(HostError::new(format!("Android document filesystem JNI call failed: {error}")));
        }
    };
    let object = value.l().map_err(|e| HostError::new(e.to_string()))?;
    if object.is_null() { return Err(HostError::new("Android document filesystem returned null")) }
    let text: String = env.get_string(&JString::from(object)).map_err(|e| HostError::new(e.to_string()))?.into();
    decode_response(&text)
}

#[cfg(not(target_os = "android"))]
pub(crate) fn call(_request: Value) -> HostResult<Value> {
    Err(HostError::new("Android document filesystem is unavailable on this platform"))
}

#[cfg(any(target_os = "android", test))]
fn decode_response(text: &str) -> HostResult<Value> {
    let response: Value = serde_json::from_str(text).map_err(|e| HostError::new(e.to_string()))?;
    if response.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(HostError::new(response.get("error").and_then(Value::as_str).unwrap_or("Invalid Android document filesystem response")));
    }
    response.get("value").cloned().ok_or_else(|| HostError::new("Android document filesystem response has no value"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn errorsNeverBecomeSuccessfulEmptyResults() {
        assert!(decode_response(r#"{"ok":false,"error":"SecurityException: grant revoked"}"#).unwrap_err().to_string().contains("grant revoked"));
        assert!(decode_response(r#"{"ok":true}"#).is_err());
        assert!(decode_response("broken").is_err());
        assert_eq!(decode_response(r#"{"ok":true,"value":null}"#).unwrap(), Value::Null);
    }
}
