//! Prepares the plugin metadata contract shared by snapshot imports and synchronization.
use std::sync::Arc;
use operit_host_api::RuntimeStorageHost;

/// The native importer and unopened-owner synchronization use the exact contract shipped to TS.
pub(crate) fn prepareMetadata(storage: Arc<dyn RuntimeStorageHost>, path: &str) -> Result<(),String> {
    use operit_host_api::PluginStorage::{StorageRequest,StorageResponse};
    let session=operit_store::PluginStorage::PluginStorageSession::default();
    let StorageResponse::Open(opened)=session.requestPreferences(storage.clone(),StorageRequest::OpenPreferences {
        path:path.into(),merge:None,
    }).map_err(|e|e.to_string())? else { return Err("Unexpected metadata open result".into()); };
    let schema=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../../plugins/packages/buildin/character_cards/src/storage/meta-schema.json"))).map_err(|e|e.to_string())?;
    let result=session.requestPreferences(storage.clone(),StorageRequest::PreferencesSchema { handle:opened.handle.clone(),schema });
    session.requestPreferences(storage,StorageRequest::Close {handle:opened.handle}).map_err(|e|e.to_string())?;
    result.map(|_|()).map_err(|e|e.to_string())
}

/// Prepares metadata for an existing plugin without reading retired internal character data.
pub(crate) fn prepare(storage: Arc<dyn RuntimeStorageHost>) -> Result<(), String> {
    let directory = operit_store::ExtensionStore::ExtensionStore::spaceDataPath(
        super::CharacterPluginMigration::PLUGIN_ID,
    )?;
    let path = format!("{directory}/meta.preferences.json");
    if !storage.exists(&path).map_err(|error| error.to_string())? {
        return Ok(());
    }
    prepareMetadata(storage, &path)
}
