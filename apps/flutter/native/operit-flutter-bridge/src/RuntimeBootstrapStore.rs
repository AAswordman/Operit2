use std::path::{Path, PathBuf};

use operit_host_api::RuntimeStorageHost;
use operit_host_native_storage::NativeRuntimeStorageHost;
use operit_util::RuntimeStorageLayout::CLIENT_RUNTIME_BOOTSTRAP_PATH;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeBootstrapReadResponse {
    ok: bool,
    value: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeBootstrapWriteResponse {
    ok: bool,
    error: Option<String>,
}

/// Reads the opaque client bootstrap record through one Host storage implementation.
fn readRuntimeBootstrapConfig(storage: &dyn RuntimeStorageHost) -> Result<Option<String>, String> {
    if !storage
        .exists(CLIENT_RUNTIME_BOOTSTRAP_PATH)
        .map_err(|error| error.to_string())?
    {
        return Ok(None);
    }
    let bytes = storage
        .readBytes(CLIENT_RUNTIME_BOOTSTRAP_PATH)
        .map_err(|error| error.to_string())?;
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| format!("runtime bootstrap config is not valid UTF-8: {error}"))
}

/// Writes the opaque client bootstrap record through one Host storage implementation.
fn writeRuntimeBootstrapConfig(
    storage: &dyn RuntimeStorageHost,
    content: &str,
) -> Result<(), String> {
    if content.is_empty() {
        return Err("runtime bootstrap config must not be empty".to_string());
    }
    storage
        .writeBytes(CLIENT_RUNTIME_BOOTSTRAP_PATH, content.as_bytes())
        .map_err(|error| error.to_string())
}

/// Resolves the fixed client bootstrap root beside the platform default Runtime root.
fn nativeRuntimeBootstrapStorage(
    defaultRuntimeRoot: &Path,
) -> Result<NativeRuntimeStorageHost, String> {
    let applicationRoot = defaultRuntimeRoot.parent().ok_or_else(|| {
        format!(
            "default Runtime root has no application parent: {}",
            defaultRuntimeRoot.display()
        )
    })?;
    let clientRoot = applicationRoot.join("client");
    Ok(NativeRuntimeStorageHost::new(
        clientRoot.clone(),
        clientRoot.join("workspaces"),
    ))
}

/// Reads the native bootstrap record without creating the Runtime.
#[allow(non_snake_case)]
pub fn readNativeRuntimeBootstrapConfig(defaultRuntimeRoot: PathBuf) -> String {
    let result = nativeRuntimeBootstrapStorage(&defaultRuntimeRoot)
        .and_then(|storage| readRuntimeBootstrapConfig(&storage));
    let response = match result {
        Ok(value) => RuntimeBootstrapReadResponse {
            ok: true,
            value,
            error: None,
        },
        Err(error) => RuntimeBootstrapReadResponse {
            ok: false,
            value: None,
            error: Some(error),
        },
    };
    serde_json::to_string(&response).expect("runtime bootstrap read response must serialize")
}

/// Writes the native bootstrap record without creating the Runtime.
#[allow(non_snake_case)]
pub fn writeNativeRuntimeBootstrapConfig(defaultRuntimeRoot: PathBuf, content: &str) -> String {
    let result = nativeRuntimeBootstrapStorage(&defaultRuntimeRoot)
        .and_then(|storage| writeRuntimeBootstrapConfig(&storage, content));
    let response = match result {
        Ok(()) => RuntimeBootstrapWriteResponse {
            ok: true,
            error: None,
        },
        Err(error) => RuntimeBootstrapWriteResponse {
            ok: false,
            error: Some(error),
        },
    };
    serde_json::to_string(&response).expect("runtime bootstrap write response must serialize")
}
