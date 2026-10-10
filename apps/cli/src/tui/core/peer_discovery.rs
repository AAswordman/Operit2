//! One-shot LAN discovery for the network hub. A scan blocks for its whole
//! timeout, so it runs on the local executor and reports back as an event
//! instead of freezing the terminal loop.

use std::sync::mpsc;

use operit_node_runtime::RuntimeRemoteLinkService::RuntimeRemoteLinkService;

use crate::tui::NetworkUiEvent;

/// Scan timeout: long enough for a slow LAN answer, short enough that a
/// repeated `d` press stays useful.
pub(super) const PEER_DISCOVERY_TIMEOUT_MS: u64 = 3000;

/// Scans for LAN peer candidates in the background; the result arrives as
/// `NetworkUiEvent::DiscoveryFinished`. The task dies with the local executor,
/// so an in-flight scan can never outlive the Core it queries.
pub(super) fn spawn_peer_discovery(
    network: RuntimeRemoteLinkService,
    events: mpsc::Sender<NetworkUiEvent>,
) {
    tokio::task::spawn_local(async move {
        let result = network.discoverPeers(PEER_DISCOVERY_TIMEOUT_MS).await;
        let _ = events.send(NetworkUiEvent::DiscoveryFinished(result));
    });
}
