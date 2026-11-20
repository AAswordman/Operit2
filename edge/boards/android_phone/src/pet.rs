use operit_edge_sdk::{
    contract::EDGE_WATCH_PET_OBJECT_ID,
    host::RuntimeStorageHost,
    link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreCallResponse, CoreLinkError, CoreValue},
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

const SETTINGS_PATH: &str = "watch-pet/settings.json";
const VERSION: u8 = 1;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum Accent {
    Mint,
    Peach,
    Lilac,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PetSettings {
    version: u8,
    accent: Accent,
    haptics: bool,
    keep_awake: bool,
}

pub(crate) struct PetPreferences {
    storage: Arc<dyn RuntimeStorageHost>,
    current: Mutex<PetSettings>,
}

impl PetPreferences {
    /// Initializes the device preferences once, retaining the old screen-on choice.
    pub(crate) fn open(storage: Arc<dyn RuntimeStorageHost>, keep_awake: bool) -> Result<Self, String> {
        let current = if storage.exists(SETTINGS_PATH).map_err(|error| error.to_string())? {
            let bytes = storage.readBytes(SETTINGS_PATH).map_err(|error| error.to_string())?;
            serde_json::from_slice::<PetSettings>(&bytes).map_err(|error| error.to_string())?
        } else {
            let settings = PetSettings { version: VERSION, accent: Accent::Mint, haptics: true, keep_awake };
            persist(storage.as_ref(), settings)?;
            settings
        };
        validate(current)?;
        Ok(Self { storage, current: Mutex::new(current) })
    }

    /// Returns the last committed configuration for both renderers.
    pub(crate) fn read(&self) -> Result<PetSettings, String> {
        Ok(*self.current.lock().map_err(|error| error.to_string())?)
    }

    /// Writes the device snapshot before publishing it to the local renderer.
    fn write(&self, settings: PetSettings) -> Result<PetSettings, String> {
        validate(settings)?;
        let mut current = self.current.lock().map_err(|error| error.to_string())?;
        persist(self.storage.as_ref(), settings)?;
        *current = settings;
        Ok(settings)
    }

    /// Handles only the watch settings capability on the authenticated Edge router.
    pub(crate) fn call(&self, request: CoreCallRequest) -> Option<CoreCallResponse> {
        if request.target != EDGE_WATCH_PET_OBJECT_ID {
            return None;
        }
        let result = match request.methodName.as_str() {
            "getConfig" if matches!(&request.args, CoreValue::Map(fields) if fields.is_empty()) =>
                self.read().map_err(CoreLinkError::internal),
            "getConfig" => Err(CoreLinkError::new("INVALID_ARGUMENT", "Expected an empty settings request")),
            "setConfig" => fromCoreValue(request.args)
                .map_err(|error| CoreLinkError::new("INVALID_ARGUMENT", error.to_string()))
                .and_then(|settings| self.write(settings).map_err(CoreLinkError::internal)),
            _ => Err(CoreLinkError::methodNotFound(&request.registryKey())),
        }
        .and_then(|settings| toCoreValue(settings).map_err(|error| CoreLinkError::internal(error.to_string())));
        Some(CoreCallResponse { requestId: request.requestId, result })
    }
}

/// Requires both ends to understand the same persisted configuration fields.
fn validate(settings: PetSettings) -> Result<(), String> {
    if settings.version != VERSION {
        return Err("Unsupported watch pet settings version".into());
    }
    Ok(())
}

/// Commits settings through the same small device-local namespace as the node.
fn persist(storage: &dyn RuntimeStorageHost, settings: PetSettings) -> Result<(), String> {
    let data = serde_json::to_vec(&settings).map_err(|error| error.to_string())?;
    storage.writeBytesAtomically(SETTINGS_PATH, &data).map_err(|error| error.to_string())
}
