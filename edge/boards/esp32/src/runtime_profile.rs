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
