use std::sync::Arc;

use operit_host_api::RuntimeStorageHost;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_util::RuntimeStorageLayout::RUNTIME_LINK_ACCESS_IDENTITY_PATH;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::PreferencesDataStore::{emptyPreferences, stringPreferencesKey, CoreNodeStateStore};
use crate::RuntimeStorageHost::defaultRuntimeStorageHost;

const CORE_NODE_IDENTITY_RECORD_KEY: &str = "record";

/// Identifies one Operit CoreNode independently from its platform presentation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreNodeIdentity {
    pub nodeId: String,
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

    /// Reads the persisted CoreNode identity.
    pub fn identity(&self) -> Result<CoreNodeIdentity, String> {
        let preferences = self.dataStore().data().map_err(|error| error.to_string())?;
        let encoded = preferences
            .get(&stringPreferencesKey(CORE_NODE_IDENTITY_RECORD_KEY))
            .ok_or_else(|| "CoreNode identity is not initialized".to_string())?;
        decodeIdentity(encoded)
    }

    /// Creates the stable CoreNode identity and returns the persisted value.
    pub fn initialize(&self) -> Result<CoreNodeIdentity, String> {
        let store = self.dataStore();
        let preferences = store.data().map_err(|error| error.to_string())?;
        if let Some(encoded) = preferences.get(&stringPreferencesKey(CORE_NODE_IDENTITY_RECORD_KEY))
        {
            return decodeIdentity(encoded);
        }
        let identity = CoreNodeIdentity {
            nodeId: format!("core-{}-{}", currentTimeMillis(), Uuid::new_v4().simple()),
        };
        let mut preferences = emptyPreferences();
        preferences.set(
            &stringPreferencesKey(CORE_NODE_IDENTITY_RECORD_KEY),
            serde_json::to_string(&serde_json::json!({
                "deviceId": identity.nodeId.clone(),
            }))
            .map_err(|error| error.to_string())?,
        );
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
            Some(encoded) => serde_json::from_str::<serde_json::Value>(encoded)
                .map_err(|error| error.to_string())?,
            None => serde_json::json!({}),
        };
        let object = record
            .as_object_mut()
            .ok_or_else(|| "CoreNode identity record must be a JSON object".to_string())?;
        object.insert("deviceId".to_string(), serde_json::Value::String(nodeId));
        preferences.set(
            &key,
            serde_json::to_string(&record).map_err(|error| error.to_string())?,
        );
        store
            .replace(preferences)
            .map_err(|error| error.to_string())
    }

    /// Creates the node-local datastore that contains identity and Link Access presentation data.
    fn dataStore(&self) -> CoreNodeStateStore {
        CoreNodeStateStore::newWithStorage(self.storage.clone(), RUNTIME_LINK_ACCESS_IDENTITY_PATH)
    }
}

/// Decodes the shared Link Access identity record into the CoreNode identity projection.
fn decodeIdentity(encoded: &str) -> Result<CoreNodeIdentity, String> {
    let record: serde_json::Value =
        serde_json::from_str(encoded).map_err(|error| error.to_string())?;
    let nodeId = record
        .get("deviceId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "CoreNode identity record is missing deviceId".to_string())?
        .to_string();
    validateNodeId(&nodeId)?;
    Ok(CoreNodeIdentity { nodeId })
}

/// Validates one stable CoreNode identifier.
fn validateNodeId(nodeId: &str) -> Result<(), String> {
    if nodeId.trim().is_empty() {
        return Err("CoreNode id must not be empty".to_string());
    }
    Ok(())
}
