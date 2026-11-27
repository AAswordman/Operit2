//! Reads Flutter application assets through the retained Android owner.

use jni::objects::{JByteArray, JObject, JValue};
use operit_host_api::{HostError, HostResult};

pub struct AndroidBundledResourceHost;

impl operit_host_api::BundledResourceHost::BundledResourceHost for AndroidBundledResourceHost {
    /// Opens the exact Flutter asset without copying it into mutable runtime storage.
    fn read(&self, key: &str) -> HostResult<Vec<u8>> {
        operit_host_api::BundledResourceHost::validateResourceKey(key)?;
        let bridge = crate::secret_store::androidHostSecretStoreBridge()?;
        let mut env = crate::attachCurrentThread(&bridge.vm).map_err(|error| HostError::new(error.to_string()))?;
        let key = JObject::from(env.new_string(key).map_err(|error| HostError::new(error.to_string()))?);
        let result = env.call_method(bridge.host.as_obj(), "readBundledResource", "(Ljava/lang/String;)[B", &[JValue::Object(&key)]);
        let value = match result {
            Ok(value) => value.l().map_err(|error| HostError::new(error.to_string()))?,
            Err(error) => {
                let _ = env.exception_clear();
                return Err(HostError::new(format!("Android bundled resource read failed: {error}")));
            }
        };
        if value.is_null() {
            return Err(HostError::new("Android bundled resource returned null"));
        }
        env.convert_byte_array(JByteArray::from(value)).map_err(|error| HostError::new(error.to_string()))
    }
}
