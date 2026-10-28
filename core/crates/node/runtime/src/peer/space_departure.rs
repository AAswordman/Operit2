//! A local exit survives the Space switch and is retried over ordinary paired
//! snapshot exchanges. Once a neighbor has the receipt in its control log, the
//! receipt travels through the existing same-Space synchronization paths.
use operit_util::RuntimeStorageLayout::{RUNTIME_SPACE_DEPARTURES_PATH};
use super::*;
use crate::PeerStateStore::PeerStateStore;
use operit_store::NetworkControlStore::{NetworkControlCommand, NetworkControlCommandRecord};
use std::collections::BTreeSet;

const PENDING: &str = RUNTIME_SPACE_DEPARTURES_PATH;

#[derive(Serialize, Deserialize)]
struct Departure {
    operation: SyncOperation,
    peers: BTreeSet<String>,
}

/// Persist the receipt before changing the local Space, including offline peers.
/// This state is separate from the bounded endpoint's prunable policy cache.
pub(super) fn prepare(service: &dyn NodeSpaceContext) -> Result<(), String> {
    let space = service.spaceStore().space()?;
    if space.members.len() <= 1 { return Ok(()); }
    let peers = match service.peers() {
        Ok(peers) => peers.pairedPeers().map_err(|error| error.to_string())?
            .into_iter().map(|peer| peer.nodeId).filter(|peer| *peer != service.localNodeId() && space.members.contains(peer)).collect(),
        Err(_) => space.members.iter().filter(|node| **node != service.localNodeId()).cloned().collect(),
    };
    let Some(operation) = service.networkControlStore().leaveCurrentSpace()? else { return Ok(()); };
    let store = PeerStateStore::new(service.storage());
    let departure = store.records::<Departure>(PENDING)?.remove(&operation.opId)
        .unwrap_or(Departure { operation, peers });
    if departure.peers.is_empty() { return Ok(()); }
    let id = departure.operation.opId.clone();
    store.putRecord(PENDING, &id, &departure)
}

pub(crate) fn pending(service: &dyn NodeSpaceContext, peer: &str) -> Result<Vec<SyncOperation>, String> {
    if !service.storage().exists(PENDING).map_err(|error| error.to_string())? { return Ok(Vec::new()); }
    Ok(PeerStateStore::new(service.storage()).records::<Departure>(PENDING)?.into_values()
        .filter(|record| record.peers.contains(peer)).map(|record| record.operation).collect())
}

/// The direct authenticated peer may announce only its own departure. Relaying
/// happens in the established control log, with its normal authorization replay.
pub(crate) fn observe(service: &dyn NodeSpaceContext, peer: &str,
    departures: &[SyncOperation], control: &[SyncOperation]) -> Result<(), String> {
    let scope = service.spaceStore().space()?.spaceId;
    let mut changed = false;
    for operation in departures {
        let record = NetworkControlCommandRecord::deserialize(&operation.payload).map_err(|error| error.to_string())?;
        if operation.originDeviceId != peer || record.issuerNodeId != peer
            || !matches!(record.command, NetworkControlCommand::LeaveSpace) {
            return Err("A paired device can announce only its own Space departure".into());
        }
        if record.spaceId == scope {
            service.networkControlStore().applyBootstrapOperation(operation)?;
            changed = true;
        }
    }
    if changed {
        space_join::applyRemovedMemberLocalEffects(service, &service.networkControlStore().currentSpaceOperations()?)?;
    }
    // A transport response alone is not an acknowledgement: older peers may
    // ignore optional fields. Require the exact receipt in the peer's log.
    if !service.storage().exists(PENDING).map_err(|error| error.to_string())? { return Ok(()); }
    let store = PeerStateStore::new(service.storage());
    for (id, mut record) in store.records::<Departure>(PENDING)? {
        if record.peers.contains(peer) && control.contains(&record.operation) {
            record.peers.remove(peer);
            if record.peers.is_empty() { store.deleteRecord(PENDING, &id)?; }
            else { store.putRecord(PENDING, &id, &record)?; }
        }
    }
    Ok(())
}

/// Retry only pending receipts, including on endpoints without business sync.
/// Paired snapshots remain available after the two devices' Spaces diverge.
pub(crate) async fn synchronize(service: &dyn NodeSpaceContext) -> Result<(), String> {
    space_join::recoverLocalDeparture(service)?;
    let peers = service.peers()?;
    let active = peers.activePeerNodeIds().map_err(|error| error.to_string())?;
    let mut errors = Vec::new();
    for peer in peers.outboundPeerNodeIds().map_err(|error| error.to_string())? {
        if !active.contains(&peer) || pending(service, &peer)?.is_empty() { continue; }
        let result = exchange(service, &peer).await;
        if let Err(error) = result { errors.push(error); }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors.join(" | ")) }
}

/// Paired-only exit/ack exchange; it never migrates either device's Space.
pub(super) async fn exchange(service: &dyn NodeSpaceContext, peer: &str) -> Result<(), String> {
    let request = CoreCallRequest::new(operit_link::nextCoreRouteRequestId("space-exit"),
        NODE_SPACE_TARGET, "snapshot", CoreValue::Null);
    let remote: PeerSpaceSnapshot = fromCoreValue(service.callNode(peer.to_owned(), request).await.result
        .map_err(|error| error.to_string())?).map_err(|error| error.to_string())?;
    // The former neighbor may still list us in the old Space. This
    // exchange delivers/acks exits; it must never adopt that stale view.
    observe(service, peer, &remote.departures, &remote.controlOperations)?;
    let remoteHasDepartures = !remote.departures.is_empty();
    // The peer's full snapshot is no longer needed. Release it before building
    // our response and revalidating the authenticated transport's authority.
    drop(remote);
    if pending(service, peer)?.is_empty() && !remoteHasDepartures { return Ok(()); }
    let snapshot = peerSpaceSnapshotForPeer(service, peer)?;
    let request = CoreCallRequest::new(operit_link::nextCoreRouteRequestId("space-exit"),
        NODE_SPACE_TARGET, "observeSpaceSnapshot", toCoreValue(snapshot).map_err(|error| error.to_string())?);
    service.callNode(peer.to_owned(), request).await.result.map_err(|error| error.to_string())?;
    Ok(())
}
