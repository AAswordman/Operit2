//! Worker-owned reads of immutable application assets.

use operit_host_api::{HostResult, BundledResourceHost::{BundledResourceHost, validateResourceKey}};
use wasm_bindgen::JsValue;

pub struct WebBundledResourceHost;

impl BundledResourceHost for WebBundledResourceHost {
    /// Reads the static resource through the browser worker's synchronous resource binding.
    fn read(&self, key: &str) -> HostResult<Vec<u8>> {
        validateResourceKey(key)?;
        let value = crate::common::call_bundled_resources("read", &[JsValue::from_str(key)])?;
        Ok(js_sys::Uint8Array::new(&value).to_vec())
    }
}
