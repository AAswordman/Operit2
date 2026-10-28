//! Configuration editor endpoint owned by this firmware application.
#![allow(non_snake_case)]
use operit_host_api::RuntimeStorageHost;
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreCallResponse, CoreLinkError};
use operit_edge_sdk::storage::{
    CoreSpaceStore::CoreSpaceStore, DevicePreferencesStore::DevicePreferencesStore,
};
use serde_json::Value;
use std::sync::Arc;

pub fn call(
    storage: Arc<dyn RuntimeStorageHost>,
    nodeId: &str,
    request: &CoreCallRequest,
) -> Option<CoreCallResponse> {
    if request.target != "edge.preferences" {
        return None;
    }
    let result = (|| -> Result<_, String> {
        let space = CoreSpaceStore::newNodeLocal(storage.clone()).space()?;
        let store = DevicePreferencesStore::new(storage);
        let args: Value = if request.args == operit_link::CoreValue::Null {
            Value::Null
        } else {
            fromCoreValue(request.args.clone()).map_err(|e| e.to_string())?
        };
        let value = match request.methodName.as_str() {
            "read" => {
                let values = store.read(&space.spaceId, nodeId)?.values();
                if let Some(key) = args.get("key").and_then(Value::as_str) {
                    values.get(key).cloned().unwrap_or(Value::Null)
                } else {
                    serde_json::to_value(values).map_err(|e| e.to_string())?
                }
            }
            "keys" => serde_json::to_value(
                store
                    .read(&space.spaceId, nodeId)?
                    .values()
                    .keys()
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| e.to_string())?,
            "set" | "remove" => {
                let key = args["key"].as_str().ok_or("Missing preference key")?;
                let value = if request.methodName == "remove" {
                    None
                } else {
                    Some(
                        args.get("value")
                            .cloned()
                            .ok_or("Missing preference value")?,
                    )
                };
                store.set(&space.spaceId, nodeId, nodeId, key, value)?;
                serde_json::json!({"updated":true})
            }
            _ => return Err("Unknown device preference method".into()),
        };
        toCoreValue(value).map_err(|e| e.to_string())
    })()
    .map_err(CoreLinkError::internal);
    Some(CoreCallResponse {
        requestId: request.requestId.clone(),
        result,
    })
}
