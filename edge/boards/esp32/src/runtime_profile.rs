//! ESP32 product budgets, shared by firmware and the constrained simulator.
//! Shared Peer/Scene implementations do not choose this board's allocations.
use operit_edge_sdk::{
    runtime::HostRuntimePeerService::PeerRuntimeLimits,
    scene::SceneLimits,
};

pub const SCENE_LIMITS: SceneLimits = SceneLimits {
    pack_bytes: 8192,
    cache_bytes: 16384,
};

pub fn peer_limits() -> PeerRuntimeLimits {
    PeerRuntimeLimits {
        maxFrameBytes: 2 * 1024,
        ..PeerRuntimeLimits::constrained()
    }
}

/// Compose the board's existing authenticated runtime with its TCP/UART carriers.
pub fn create_peer_service<R: operit_edge_sdk::runtime::PeerRouter::PeerRouter + 'static>(
    host: std::sync::Arc<operit_host_api::HostManager::HostManager>,
    router: &std::sync::Arc<R>,
    info: operit_link::protocol::LinkDeviceInfo,
    limits: PeerRuntimeLimits,
) -> Result<std::sync::Arc<operit_edge_sdk::runtime::HostRuntimePeerService::HostRuntimePeerService>, String> {
    let link = std::sync::Arc::new(operit_peer_link::StreamHostPeerLink::withMaxMessageBytes(limits.maxFrameBytes)?);
    operit_edge_sdk::runtime::HostRuntimePeerService::HostRuntimePeerService::newEndpointWithLink(
        host, router, info, limits, link,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_budgets_do_not_change_other_products_defaults() {
        let shared = PeerRuntimeLimits::constrained();
        let board = peer_limits();
        assert_eq!(shared.maxFrameBytes, 8 * 1024);
        assert_eq!(shared.maxMessageBytes, 8 * 1024);
        assert_eq!(board.maxFrameBytes, 2 * 1024);
        assert_eq!(board.maxMessageBytes, shared.maxMessageBytes);
        assert_eq!(board.incomingSessions, shared.incomingSessions);
        assert_eq!(board.pendingPairings, shared.pendingPairings);
        assert_eq!(SceneLimits::default().pack_bytes, 4096);
        assert_eq!(SceneLimits::default().cache_bytes, 8192);
        assert_eq!(SCENE_LIMITS.pack_bytes, 8192);
        assert_eq!(SCENE_LIMITS.cache_bytes, 16384);
    }
}
