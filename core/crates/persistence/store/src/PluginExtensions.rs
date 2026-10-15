use crate::SqliteStore::SqliteStoreError;
use serde_json::Value;
use std::collections::BTreeMap;

/// Validates the namespaced objects persisted on a real chat or message record.
pub(crate) fn validatePluginExtensions(
    extensions: &BTreeMap<String, Value>,
) -> Result<(), SqliteStoreError> {
    for (owner, value) in extensions {
        if owner.trim().is_empty() || !value.is_object() {
            return Err(SqliteStoreError::Message(format!(
                "Plugin extension namespace {owner:?} requires a nonempty owner and a JSON object"
            )));
        }
    }
    Ok(())
}

/// Decodes persisted extensions strictly without replacing malformed record data.
pub(crate) fn decodePluginExtensions(
    json: &str,
) -> Result<BTreeMap<String, Value>, SqliteStoreError> {
    let extensions = serde_json::from_str(json).map_err(|error| {
        SqliteStoreError::Message(format!("Invalid pluginExtensions JSON: {error}"))
    })?;
    validatePluginExtensions(&extensions)?;
    Ok(extensions)
}

/// Encodes validated extension objects for the owning SQLite record column.
pub(crate) fn encodePluginExtensions(
    extensions: &BTreeMap<String, Value>,
) -> Result<String, SqliteStoreError> {
    validatePluginExtensions(extensions)?;
    serde_json::to_string(extensions).map_err(|error| {
        SqliteStoreError::Message(format!("Cannot serialize pluginExtensions: {error}"))
    })
}
