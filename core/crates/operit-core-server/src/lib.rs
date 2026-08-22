#![allow(non_snake_case)]


/// Identifies the CoreNode selected by one protocol request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneratedCoreRoute {
    Local,
    Binding { scope: usize, key: String },
}

/// Describes one annotation-generated Space route independent of local Proxy object IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedSpaceRoute {
    pub routeId: u32,
    pub methodName: &'static str,
    pub bindingArgument: &'static str,
    pub targetType: &'static str,
}

impl GeneratedSpaceRoute {
    /// Returns the binding key carried by one routed request.
    pub fn bindingKey(&self, args: &operit_link::CoreValue) -> Result<String, operit_link::CoreLinkError> {
        let operit_link::CoreValue::Map(arguments) = args else {
            return Err(operit_link::CoreLinkError::new("INVALID_ARGS", "Space route arguments must be a map"));
        };
        let Some(value) = arguments.get(self.bindingArgument) else {
            return Err(operit_link::CoreLinkError::new("CORE_BINDING_KEY_REQUIRED", "Space route request does not include its binding key"));
        };
        let operit_link::CoreValue::String(key) = value else {
            return Err(operit_link::CoreLinkError::new("CORE_BINDING_KEY_INVALID", "Space route binding key must be a string"));
        };
        if key.trim().is_empty() {
            return Err(operit_link::CoreLinkError::new("CORE_BINDING_KEY_REQUIRED", "Space route binding key must not be empty"));
        }
        Ok(key.clone())
    }
}

pub const CORE_ROUTE_CURSOR_ARGUMENT: &str = "__operit_route_cursor";
pub const CORE_ROUTE_CURSOR_PROPERTY: &str = "__operit_route_cursor";

/// Returns the stable object id owned by the Space route namespace.
pub fn spaceObjectIdForSchema(schema: &str) -> Option<u32> {
    match schema {
        "application" => Some(0),
        "services.syncBlobTransferManager" => Some(1),
        "runtimeRemoteLinkService" => Some(2),
        _ => None,
    }
}

include!(concat!(env!("OUT_DIR"), "/generated_route_catalog.rs"));

pub mod CoreNodeRouter;
pub mod SpaceRuntime;
#[cfg(not(target_arch = "wasm32"))]
pub mod RuntimeRemoteLinkDiscovery;
pub mod RuntimeRemoteLinkService;
pub mod SpacePersistenceSyncService;
