//! Publishes one CoreNode's live direct-link adjacency into the synchronized Space
//! topology so every member can resolve multi-hop routes through it.

use std::collections::BTreeSet;

use operit_store::CoreSpaceStore::CoreSpaceStore;
use operit_store::NetworkControlStore::NetworkControlStore;

/// Keeps one published direct link valid for this long while the peer stays connected.
pub const SPACE_TOPOLOGY_LINK_TTL_MS: i64 = 600_000;
/// Refreshes a published link once its remaining lifetime drops to this margin.
pub const SPACE_TOPOLOGY_REFRESH_MARGIN_MS: i64 = 300_000;

/// Publishes this device's direct-link adjacency for the current Space membership.
///
/// Only live, admitted, non-disconnected peers are announced. The record is what lets
/// every other member resolve a route through this device, so it is refreshed while the
/// links stay up and pruned as soon as a peer leaves the set. Announcements carry no
/// measured link quality yet, which keeps route selection at hop count. Returns whether
/// the local record changed.
#[allow(non_snake_case)]
pub fn publishLocalTopology(
    spaceStore: &CoreSpaceStore,
    networkControlStore: &NetworkControlStore,
    localNodeId: &str,
    activePeerNodeIds: &BTreeSet<String>,
) -> Result<bool, String> {
    let members = spaceStore
        .space()?
        .members
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut directPeers = Vec::new();
    for peerNodeId in activePeerNodeIds {
        if peerNodeId == localNodeId || !members.contains(peerNodeId) {
            continue;
        }
        if networkControlStore.nodeIsDisconnected(peerNodeId)? {
            continue;
        }
        directPeers.push(peerNodeId.clone());
    }
    spaceStore.publishLocalAdjacency(
        directPeers,
        SPACE_TOPOLOGY_LINK_TTL_MS,
        SPACE_TOPOLOGY_REFRESH_MARGIN_MS,
    )
}
