//! Browser-only Host distribution for the shared offline rescue engine.
use std::sync::{Arc, Mutex, OnceLock};
use operit_data_rescue::{DataRescue, RescueFile, RescueHost, RescueManifest};
use serde_json::{json, Value};
use wasm_bindgen::JsValue;
use js_sys::Uint8Array;
use crate::common::{bytes_to_js, call_data_rescue, js_string};

struct WebRescueHost;
static SESSION: OnceLock<Mutex<Option<DataRescue>>> = OnceLock::new();

/// Invokes a structured browser storage operation without requiring Core or SQLite.
fn json_call(method: &str, args: &[JsValue]) -> Result<Value, String> {
    let value = call_data_rescue(method, args).map_err(|error| error.to_string())?;
    let text = js_string(value, method).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

/// Dispatches only offline operations on the maintenance worker's Host-owned stores.
pub fn executeDataRescue(encoded: &str) -> String {
    let result = (|| -> Result<Value, String> {
        let request: Value = serde_json::from_str(encoded).map_err(|error| error.to_string())?;
        if request["operation"] == "preferencesExportPath" {
            let path = request["path"].as_str().ok_or("Missing preference storage path")?;
            return Ok(Value::Bool(operit_store::PreferencesExport::isPreferencesPath(path)));
        }
        if request["operation"] == "exportPreferences" {
            let path = request["path"].as_str().ok_or("Missing preference storage path")?;
            let bytes: Vec<u8> = serde_json::from_value(request["bytes"].clone()).map_err(|error| error.to_string())?;
            let plaintext = operit_store::PreferencesExport::exportPreferences(path, bytes, || {
                serde_json::from_value::<Vec<u8>>(request["key"].clone())
                    .map_err(|error| format!("Cannot read the existing preferences key: {error}"))
            })?;
            return serde_json::to_value(plaintext).map_err(|error| error.to_string());
        }
        let mut session = SESSION.get_or_init(|| Mutex::new(None)).lock().map_err(|error| error.to_string())?;
        match request["operation"].as_str() {
            Some("open") => {
                *session = Some(DataRescue { host: Arc::new(WebRescueHost), archives: Arc::new(crate::WebArchiveStagingHost::new()) });
                json_call("startup", &[])
            }
            Some("configure") | Some("configureAutomatic") => json_call("describe", &[]),
            Some("close") => { *session = None; Ok(Value::Null) }
            _ => session.as_ref().ok_or("Open browser data rescue first")?.execute(request),
        }
    })();
    match result {
        Ok(value) => json!({"ok": true, "value": value}).to_string(),
        Err(error) => json!({"ok": false, "error": error}).to_string(),
    }
}

impl RescueHost for WebRescueHost {
    /// Sends measured synchronous WASM progress through the worker Host.
    fn report_progress(&self, progress: operit_data_rescue::RescueProgress) {
        json_call("reportProgress", &[JsValue::from_str(&serde_json::to_string(&progress).expect("Serializable rescue progress"))]).expect("Rescue progress Host is available");
    }

    /// Keeps browser physical storage distinct from native directory publication.
    fn layout(&self) -> &str { "operit.browser.v1" }
    /// Describes all origin-owned identities without reading their application configuration.
    fn describe(&self) -> Result<Value, String> { json_call("describe", &[]) }
    /// Enumerates raw OPFS files and serialized browser metadata independently of SQLite.
    fn list_files(&self) -> Result<Vec<RescueFile>, String> {
        serde_json::from_value(json_call("listFiles", &[])?).map_err(|error| error.to_string())
    }
    /// Reads a bounded raw browser storage range through the maintenance Host.
    fn read_file(&self, path: &str, offset: u64, length: usize) -> Result<Vec<u8>, String> {
        let value = call_data_rescue("readFile", &[JsValue::from_str(path), JsValue::from_f64(offset as f64), JsValue::from_f64(length as f64)]).map_err(|error| error.to_string())?;
        Ok(Uint8Array::new(&value).to_vec())
    }
    /// Uses preferences already converted while the browser Host captures its snapshot.
    fn export_preferences(&self, _path: &str) -> Result<Option<Vec<u8>>, String> { Ok(None) }
    /// Creates a private import staging tree after the shared engine verifies the entire archive.
    fn begin_restore(&self, id: &str, manifest: &RescueManifest) -> Result<(), String> {
        json_call("beginRestore", &[JsValue::from_str(id), JsValue::from_str(&serde_json::to_string(manifest).map_err(|error| error.to_string())?)])?;
        Ok(())
    }
    /// Appends a bounded verified payload chunk to worker-owned staging.
    fn write_restore(&self, id: &str, path: &str, chunk: &[u8]) -> Result<(), String> {
        call_data_rescue("writeRestore", &[JsValue::from_str(id), JsValue::from_str(path), bytes_to_js(chunk)]).map_err(|error| error.to_string())?;
        Ok(())
    }
    /// Flushes one staged file before publication can start.
    fn finish_restore_file(&self, id: &str, path: &str) -> Result<(), String> {
        json_call("finishRestoreFile", &[JsValue::from_str(id), JsValue::from_str(path)])?;
        Ok(())
    }
    /// Records a prepared publication for the asynchronous browser Host to complete durably.
    fn commit_restore(&self, id: &str, manifest: &RescueManifest) -> Result<Value, String> {
        json_call("commitRestore", &[JsValue::from_str(id), JsValue::from_str(&serde_json::to_string(manifest).map_err(|error| error.to_string())?)])
    }
    /// Requests completion of the persisted browser publication journal.
    fn resume_restore(&self) -> Result<Value, String> { json_call("resumeRestore", &[]) }
    /// Removes uncommitted staging without touching live origin data.
    fn discard_restore(&self, id: &str) -> Result<(), String> {
        json_call("discardRestore", &[JsValue::from_str(id)])?;
        Ok(())
    }
}
