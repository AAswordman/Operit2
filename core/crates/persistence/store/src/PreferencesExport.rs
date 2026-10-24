//! Converts stored preferences into portable plaintext without changing live storage.
use crate::PreferencesEncryption::PreferencesEncryption;
use operit_host_api::{HostSecretStore, RuntimeStorageHost};
use operit_util::RuntimeStorageLayout::{
    CONFIG_PREFERENCES_DIR_PATH, PREFERENCES_ENCRYPTION_KEY_PATH,
};

/// Bounds the JSON document required to decode one preferences envelope.
pub const MAX_PREFERENCES_EXPORT_BYTES: usize = 16 * 1024 * 1024;

/// Reads one bounded preference document through Host ranges before JSON conversion.
#[allow(non_snake_case)]
pub fn readPreferencesExportBytes(
    storage: &dyn RuntimeStorageHost,
    path: &str,
    byteLength: u64,
) -> Result<Vec<u8>, String> {
    if byteLength > MAX_PREFERENCES_EXPORT_BYTES as u64 {
        return Err(format!("Preferences exceed 16 MiB: {path}"));
    }
    let mut bytes = Vec::with_capacity(byteLength as usize);
    while bytes.len() < byteLength as usize {
        let count = (byteLength as usize - bytes.len()).min(64 * 1024);
        let chunk = storage.readBytesRange(path, bytes.len() as u64, count).map_err(|error| error.to_string())?;
        if chunk.is_empty() || chunk.len() > count { return Err(format!("Invalid preferences read range: {path}")); }
        bytes.extend_from_slice(&chunk);
    }
    if !storage.readBytesRange(path, byteLength, 1).map_err(|error| error.to_string())?.is_empty() {
        return Err(format!("Preferences grew while exporting: {path}"));
    }
    Ok(bytes)
}

/// Identifies preference files by their declared storage namespaces.
#[allow(non_snake_case)]
pub fn isPreferencesPath(path: &str) -> bool {
    let inDirectory = |directory: &str| {
        path.strip_prefix(directory)
            .and_then(|relative| relative.strip_prefix('/'))
            .is_some_and(|relative| !relative.contains('/') && relative.ends_with(".json"))
    };
    inDirectory(CONFIG_PREFERENCES_DIR_PATH) || inDirectory("runtime/node_preferences")
}

/// Reads an existing key from the selected Host secret contract without creating one.
#[allow(non_snake_case)]
pub fn readPreferencesExportKey(
    storage: &dyn RuntimeStorageHost,
    secrets: Option<&dyn HostSecretStore>,
) -> Result<Vec<u8>, String> {
    match secrets {
        Some(secrets) => secrets
            .readSecret("operit.preferences.encryption_key.v1")
            .map_err(|error| error.to_string())?
            .ok_or_else(|| {
                "Preferences encryption key is missing from the Host secret store".to_string()
            }),
        None => storage
            .readBytes(PREFERENCES_ENCRYPTION_KEY_PATH)
            .map_err(|error| error.to_string()),
    }
}

/// Decodes an explicit encrypted envelope and preserves plaintext preference maps exactly.
#[allow(non_snake_case)]
pub fn exportPreferences(
    path: &str,
    content: Vec<u8>,
    readKey: impl FnOnce() -> Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    let convert = || -> Result<Vec<u8>, String> {
        if content.len() > MAX_PREFERENCES_EXPORT_BYTES { return Err("Preferences exceed 16 MiB".into()); }
        let value: serde_json::Value =
            serde_json::from_slice(&content).map_err(|error| error.to_string())?;
        if value.get("format").and_then(serde_json::Value::as_str)
            == Some("operit.preferences.encrypted")
        {
            let key = readKey()?;
            let encryption =
                PreferencesEncryption::decodeKeyBytes(&key).map_err(|error| error.to_string())?;
            let plaintext = encryption
                .decrypt(path, &content)
                .map_err(|error| error.to_string())?;
            serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(&plaintext)
                .map_err(|error| error.to_string())?;
            return Ok(plaintext);
        }
        if !value.is_object() {
            return Err("Preferences must contain a JSON object".into());
        }
        Ok(content)
    };
    convert().map_err(|error| format!("Cannot export preferences {path}: {error}"))
}
