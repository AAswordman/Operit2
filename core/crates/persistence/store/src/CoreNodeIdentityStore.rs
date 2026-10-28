use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_IDENTITY_PATH;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::PreferencesDataStore::{
    stringPreferencesKey, CoreNodeStateStore, PreferencesDataStoreError,
};
use crate::RuntimeStorageHost::{defaultRuntimeStorageHost, defaultRuntimeStorageHostOption};

const CORE_NODE_IDENTITY_RECORD_KEY: &str = "record";

/// Identifies one Operit CoreNode independently from its platform presentation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreNodeIdentity {
    pub nodeId: String,
}

/// Decodes only the identity field from the shared presentation record.
#[derive(Serialize, Deserialize)]
struct IdentityRecord {
    #[serde(rename = "deviceId")]
    nodeId: String,
}

impl IdentityRecord {
    /// Validates the stored identifier without materializing unrelated presentation fields.
    fn identity(self) -> Result<CoreNodeIdentity, String> {
        validateNodeId(&self.nodeId)?;
        Ok(CoreNodeIdentity {
            nodeId: self.nodeId,
        })
    }
}

/// Owns the stable CoreNode identity shared by runtime, sync, CLI, and Link Access.
#[derive(Clone)]
pub struct CoreNodeIdentityStore {
    storage: Arc<dyn RuntimeStorageHost>,
}

impl CoreNodeIdentityStore {
    /// Creates an identity store over an explicit runtime storage host.
    pub fn new(storage: Arc<dyn RuntimeStorageHost>) -> Self {
        Self { storage }
    }

    /// Creates an identity store over the process-wide runtime storage host.
    pub fn native() -> Self {
        Self::new(defaultRuntimeStorageHost())
    }

    /// Reads the local source identity without initializing storage or inventing a node.
    #[allow(non_snake_case)]
    pub fn localNodeId() -> Option<String> {
        let storage = defaultRuntimeStorageHostOption()?;
        Self::new(storage)
            .identity()
            .ok()
            .map(|identity| identity.nodeId)
    }

    /// Reads the persisted CoreNode identity.
    pub fn identity(&self) -> Result<CoreNodeIdentity, String> {
        self.dataStore()
            .typedEntry::<IdentityRecord>(CORE_NODE_IDENTITY_RECORD_KEY)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "CoreNode identity is not initialized".to_owned())?
            .identity()
    }

    /// Creates the stable identity only when the shared record has not been initialized.
    pub fn initialize(&self) -> Result<CoreNodeIdentity, String> {
        let store = self.dataStore();
        let mut preferences = store.data().map_err(|error| error.to_string())?;
        if let Some(encoded) = preferences.get(&stringPreferencesKey(CORE_NODE_IDENTITY_RECORD_KEY))
        {
            if let Ok(identity) = serde_json::from_value::<IdentityRecord>(encoded.clone()).map_err(|error| error.to_string()).and_then(IdentityRecord::identity) { return Ok(identity); }
        }
        let identity = CoreNodeIdentity {
            nodeId: format!("core-{}-{}", currentTimeMillis(), Uuid::new_v4().simple()),
        };
        let key = stringPreferencesKey(CORE_NODE_IDENTITY_RECORD_KEY);
        let mut record = preferences.get(&key).and_then(serde_json::Value::as_object).cloned().unwrap_or_default();
        record.insert("deviceId".into(), serde_json::Value::String(identity.nodeId.clone()));
        preferences.set(&key, serde_json::Value::Object(record));
        store
            .replace(preferences)
            .map_err(|error| error.to_string())?;
        Ok(identity)
    }

    /// Rewrites the shared identity record while preserving Link Access presentation metadata.
    pub fn writeNodeId(&self, nodeId: String) -> Result<(), String> {
        validateNodeId(&nodeId)?;
        let store = self.dataStore();
        let mut preferences = store.data().map_err(|error| error.to_string())?;
        let key = stringPreferencesKey(CORE_NODE_IDENTITY_RECORD_KEY);
        let mut record = match preferences.get(&key) {
            Some(encoded) => serde_json::from_value::<serde_json::Value>(encoded.clone())
                .map_err(|error| error.to_string())?,
            None => serde_json::json!({}),
        };
        let object = record
            .as_object_mut()
            .ok_or_else(|| "CoreNode identity record must be a JSON object".to_string())?;
        object.insert("deviceId".to_string(), serde_json::Value::String(nodeId));
        preferences.set(
            &key,
            serde_json::to_value(&record).map_err(|error| error.to_string())?,
        );
        store
            .replace(preferences)
            .map_err(|error| error.to_string())
    }

    /// Creates the node-local datastore that contains identity and Link Access presentation data.
    pub fn dataStore(&self) -> CoreNodeStateStore {
        CoreNodeStateStore::newWithStorage(self.storage.clone(), RUNTIME_LINK_ACCESS_IDENTITY_PATH)
            .withSchema(2, |version, preferences| match version {
                0 => Ok(()),
                1 => {
                    let key = stringPreferencesKey(CORE_NODE_IDENTITY_RECORD_KEY);
                    if let Some(encoded) = preferences.get(&key).cloned() {
                        // Version 1 shipped both a JSON string and, transiently,
                        // the already-decoded object. Accept either shape so a
                        // profile cannot fail Core startup during migration.
                        let record = match encoded {
                            serde_json::Value::String(text) => {
                                serde_json::from_str::<serde_json::Value>(&text)?
                            }
                            value => value,
                        };
                        serde_json::from_value::<IdentityRecord>(record.clone())?
                            .identity()
                            .map_err(PreferencesDataStoreError::Message)?;
                        preferences.set(&key, record);
                    }
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            })
    }
}

/// Validates one stable CoreNode identifier.
fn validateNodeId(nodeId: &str) -> Result<(), String> {
    if nodeId.trim().is_empty() {
        return Err("CoreNode id must not be empty".to_string());
    }
    Ok(())
}
