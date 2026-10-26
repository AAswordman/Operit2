//! Authenticated device tool entrypoint, shared by direct and relayed watches.
use super::*;
use operit_tools::tools::DeviceTools::{DeviceToolService, DEVICE_TOOL_TARGET};

impl CoreNodeRouter {
    pub fn bindDeviceToolService(&self, service: Arc<dyn DeviceToolService>) -> Result<(), String> {
        self.deviceToolService
            .set(service)
            .map_err(|_| "Device tool service is already initialized".into())
    }

    fn requireDeviceToolMember(&self, origin: &str) -> Result<(), CoreLinkError> {
        let space = self.spaceStore.space().map_err(CoreLinkError::internal)?;
        let policy = RoutingPolicy::load(&self.networkControlStore)
            .map_err(CoreLinkError::internal)?;
        requireRouteMember(
            &space, &policy, origin,
            RouteParticipant::Origin,
        )?;
        requireRouteMember(
            &space, &policy, &self.localNodeId,
            RouteParticipant::Target,
        )
    }

    /// This surface accepts only explicit Target routing; it never falls back to another device.
    pub fn openDeviceToolWatch(
        &self,
        request: CoreWatchRequest,
        origin: String,
        kind: RoutedCoreRequestKind,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if request.target != DEVICE_TOOL_TARGET || kind != RoutedCoreRequestKind::Target {
            return Err(CoreLinkError::new(
                "DEVICE_TOOL_ROUTE_INVALID",
                "Device tools require explicit Target routing",
            ));
        }
        self.requireDeviceToolMember(&origin)?;
        self.deviceToolService
            .get()
            .ok_or_else(|| {
                CoreLinkError::new(
                    "DEVICE_TOOLS_UNAVAILABLE",
                    "This device has no Core tool service",
                )
            })?
            .open(request, origin)
    }
}
