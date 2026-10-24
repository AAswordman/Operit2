//! Validates persistent plugin storage paths independently of the storage engine.
use crate::RuntimeStorageLayout::{PLUGIN_DATA_SPACE_DIR_PATH, PLUGIN_DATA_DEVICE_DIR_PATH};

/// Validates supported persistent data paths without restricting access by the calling plugin identity.
pub fn resolveStoragePath(path: &str) -> Result<String, String> {
    let path = path.replace('\\', "/");
    if path.is_empty()
        || path.split('/').any(|segment| {
            segment.is_empty()
                || segment == "."
                || segment == ".."
                || segment.chars().any(char::is_control)
        })
    {
        return Err("Storage path contains invalid segments".into());
    }
    storageNamespace(&path)?;
    if [".operit-storage.json", "-wal", "-shm", "-journal"]
        .iter()
        .any(|suffix| path.ends_with(suffix))
    {
        return Err("Storage path uses a reserved sidecar suffix".into());
    }
    Ok(path)
}

/// Reads the durable data namespace from the canonical path independently of the script opening it.
pub fn storageNamespace(path: &str) -> Result<&str, String> {
    for root in [PLUGIN_DATA_SPACE_DIR_PATH, PLUGIN_DATA_DEVICE_DIR_PATH] {
        if let Some(relative) = path.strip_prefix(&format!("{root}/")) {
            let (namespace, file) = relative.split_once('/')
                .ok_or_else(|| "Storage path requires a namespace and database filename".to_string())?;
            if !namespace.is_empty() && !file.is_empty() {
                return Ok(namespace);
            }
        }
    }
    Err("Databases must use persistent plugin_data directories independent of installation scope".into())
}
