//! Edge startup uses the shared code-confirmed pairing engine.
//! Session encryption and Space authorization remain in the shared runtime.
use std::sync::Arc;

use operit_host_api::HostManager::HostManager;
use operit_link::protocol::LinkDeviceInfo;
use operit_node_runtime::HostRuntimePeerService::{
    HostRuntimePeerService, PeerRuntimeLimits,
};
use operit_node_runtime::PeerRouter::PeerRouter;

/// Creates the Edge receiver using the existing pairing and session engine.
/// Core and Edge share confirmation codes, encryption and authorization.
#[allow(non_snake_case)]
pub fn createPeerService<R: PeerRouter + 'static>(
    host: Arc<HostManager>,
    router: &Arc<R>,
    info: LinkDeviceInfo,
    limits: PeerRuntimeLimits,
) -> Result<Arc<HostRuntimePeerService>, String> {
    HostRuntimePeerService::newWithLimits(
        host,
        router,
        info,
        limits,
    )
}
