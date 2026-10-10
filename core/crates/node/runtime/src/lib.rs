#![allow(non_snake_case)]
pub mod PeerRouter;

/// Identifies the CoreNode selected by one protocol request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneratedCoreRoute {
    Local,
    Binding { scope: usize, key: String },
}

/// Identifies the device whose capability is required by a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeneratedRoutePermissionSubject {
    Caller,
    Target,
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
    pub routeId: &'static str,
    pub methodName: &'static str,
    pub bindingArgument: &'static str,
    pub targetType: &'static str,
    pub permissionScope: GeneratedRoutePermissionSubject,
    pub permissionCapability: &'static str,
    pub lifecycle: GeneratedRouteLifecycle,
    /// Only explicit creation commands may allocate a missing binding.
    pub createBindingCapability: &'static str,
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

#[cfg(feature = "route-catalog")]
include!(concat!(env!("OUT_DIR"), "/generated_route_catalog.rs"));

#[cfg(feature = "full")]
include!(concat!(env!("OUT_DIR"), "/generated_space_dispatch.rs"));

#[cfg(feature = "full")]
pub mod CoreNodeRouter;
#[cfg(feature = "full")]
pub mod NodeClient;
pub mod RuntimePeerService;
#[cfg(feature = "peer-runtime")]
pub mod HostRuntimePeerService;
#[cfg(feature = "peer-runtime")]
pub mod NodeSpaceService;
pub mod NodeServices;
#[cfg(feature = "peer-state")]
pub mod PeerStateStore;
#[cfg(feature = "full")]
pub mod RuntimeRemoteLinkService;
#[cfg(feature = "full")]
pub mod SpacePersistenceSyncService;
pub mod SpaceDoctor;
#[cfg(feature = "full")]
mod PeerSync;
#[cfg(feature = "full")]
pub mod SpaceRuntime;

#[cfg(all(test, feature = "route-catalog"))]
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

    /// Resolves the semantic Space target without a declaration-position identifier.
    #[test]
    fn space_routes_use_stable_named_targets() {
        let route = generated_space_route_for_method("beforeChangeRoute").unwrap();
        assert_eq!(route.routeId, format!("space/{}/beforeChangeRoute", route.targetType));
        assert_eq!(generated_space_route_for_id(route.routeId, route.methodName), Some(route.clone()));
        assert!(generated_space_route_for_id("0", route.methodName).is_none());
        assert!(generated_space_route_for_id(route.routeId, "notAnExportedMethod").is_none());
        let request = operit_link::CoreCallRequest::new("space-test", route.routeId, route.methodName, operit_link::CoreValue::emptyMap());
        assert_eq!(generated_space_call_route(&request), Some(route));
    }

    #[test]
    fn route_catalog_contains_no_chat_execution_functions() {
        let catalog = include_str!(concat!(env!("OUT_DIR"), "/generated_route_catalog.rs"));
        assert!(!catalog.contains("generated_space_call_on_chat_core"));
        assert!(!catalog.contains("generated_space_watch_on_chat_core"));
        assert!(!catalog.contains("generated_space_watch_snapshot_on_chat_core"));
    }

    #[test]
    fn shared_catalog_preserves_binding_validation() {
        use operit_link::{CoreCallRequest, CoreValue};
        let route = generated_space_route_for_method("beforeChangeRoute").unwrap();
        let request =
            |args| CoreCallRequest::new("binding-test", route.routeId, route.methodName, args);
        let empty = request(CoreValue::emptyMap());
        assert_eq!(
            generated_core_call_route(&empty).unwrap_err().code,
            "CORE_BINDING_KEY_REQUIRED"
        );
        let args = CoreValue::Map(std::collections::BTreeMap::from([(
            route.bindingArgument.into(),
            CoreValue::String("chat-1".into()),
        )]));
        assert_eq!(
            generated_core_call_route(&request(args)).unwrap(),
            GeneratedCoreRoute::Binding {
                scope: 0,
                key: "chat-1".into()
            }
        );
        assert!(generated_space_route_for_id("device/unknown", route.methodName).is_none());
    }
}
