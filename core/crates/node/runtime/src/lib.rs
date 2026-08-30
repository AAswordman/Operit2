#![allow(non_snake_case)]

/// Identifies the CoreNode selected by one protocol request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneratedCoreRoute {
    Local,
    Binding { scope: usize, key: String },
}

/// Identifies lifecycle hooks registered by annotated Space routes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedRouteLifecycle {
    Normal,
    BeforeChangeRoute,
    AfterChangeRoute,
}

/// Describes one annotation-generated Space route independent of local Proxy object IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedSpaceRoute {
    pub routeId: u32,
    pub methodName: &'static str,
    pub bindingArgument: &'static str,
    pub targetType: &'static str,
    pub lifecycle: GeneratedRouteLifecycle,
}

impl GeneratedSpaceRoute {
    /// Returns the binding key carried by one routed request.
    pub fn bindingKey(
        &self,
        args: &operit_link::CoreValue,
    ) -> Result<String, operit_link::CoreLinkError> {
        let operit_link::CoreValue::Map(arguments) = args else {
            return Err(operit_link::CoreLinkError::new(
                "INVALID_ARGS",
                "Space route arguments must be a map",
            ));
        };
        let Some(value) = arguments.get(self.bindingArgument) else {
            return Err(operit_link::CoreLinkError::new(
                "CORE_BINDING_KEY_REQUIRED",
                "Space route request does not include its binding key",
            ));
        };
        let operit_link::CoreValue::String(key) = value else {
            return Err(operit_link::CoreLinkError::new(
                "CORE_BINDING_KEY_INVALID",
                "Space route binding key must be a string",
            ));
        };
        if key.trim().is_empty() {
            return Err(operit_link::CoreLinkError::new(
                "CORE_BINDING_KEY_REQUIRED",
                "Space route binding key must not be empty",
            ));
        }
        Ok(key.clone())
    }
}

include!(concat!(env!("OUT_DIR"), "/generated_route_catalog.rs"));

pub mod CoreNodeRouter;
#[cfg(not(target_arch = "wasm32"))]
pub mod RuntimeRemoteLinkDiscovery;
pub mod RuntimeRemoteLinkService;
pub mod SpacePersistenceSyncService;
pub mod SpaceRuntime;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_route_catalog_contains_route_lifecycle_hooks() {
        let before = generated_space_lifecycle_route(GeneratedRouteLifecycle::BeforeChangeRoute)
            .expect("before-change route hook must be registered");
        let after = generated_space_lifecycle_route(GeneratedRouteLifecycle::AfterChangeRoute)
            .expect("after-change route hook must be registered");

        assert_eq!(before.methodName, "beforeChangeRoute");
        assert_eq!(before.bindingArgument, "chatId");
        assert_eq!(before.lifecycle, GeneratedRouteLifecycle::BeforeChangeRoute);
        assert_eq!(after.methodName, "afterChangeRoute");
        assert_eq!(after.bindingArgument, "chatId");
        assert_eq!(after.lifecycle, GeneratedRouteLifecycle::AfterChangeRoute);
    }
}
