//! Converges two paired devices that still advertise one shared Space identity
//! but hold divergent member tables. The control log is the only authority:
//! both sides offer their operations for the shared Space, accept only facts
//! the peer originated, and replay the merged log into one identical policy.

use super::*;
use operit_store::NetworkControlStore::NetworkControlCommandRecord;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize)]
struct ReconciliationOffer {
    spaceId: String,
    operations: Vec<SyncOperation>,
    deviceProfiles: Vec<CoreSpaceDeviceProfile>,
}

#[derive(Serialize, Deserialize)]
struct ReconciliationOutcome {
    spaceId: String,
    operations: Vec<SyncOperation>,
    deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    members: Vec<String>,
}

/// What one initiator knows after an exchange: the converged membership and
/// whether this device is still part of the shared Space.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedSpaceReconciliation {
    pub spaceId: String,
    pub members: Vec<String>,
    pub localIsMember: bool,
}

fn paired(service: &dyn NodeSpaceContext, peer: &str) -> Result<(), String> {
    if !service.peers()?.pairedPeers().map_err(|e| e.to_string())?
        .iter().any(|p| p.nodeId == peer) {
        return Err("Reconciliation requires a current pairing with the peer".into());
    }
    Ok(())
}

/// Accepts only facts the peer originated. Relayed third-party commands would
/// let one endpoint launder another issuer's authority through an otherwise
/// read-only channel, and every replica always holds its own journal, so the
/// union of both self-originated logs is already the complete history.
/// Operations outside the shared Space identity are refused outright.
fn mergePeerOperations(service: &dyn NodeSpaceContext, peer: &str, spaceId: &str,
    operations: &[SyncOperation]) -> Result<usize, String> {
    let known: BTreeSet<String> = service.networkControlStore().currentSpaceOperations()?
        .into_iter().map(|operation| operation.opId).collect();
    let mut applied = 0;
    for operation in operations {
        if operation.originDeviceId != peer || known.contains(&operation.opId) { continue; }
        let record = NetworkControlCommandRecord::deserialize(&operation.payload)
            .map_err(|e| format!("Reconciliation offered a malformed control command: {e}"))?;
        if record.spaceId != spaceId {
            return Err("Reconciliation offered a command from another Space".into());
        }
        service.networkControlStore().applySyncedOperation(operation)?;
        applied += 1;
    }
    Ok(applied)
}

/// Gateway side of the exchange. Merges the caller's facts, aligns the
/// caller's member record to the converged policy, and answers with the full
/// merged log so the initiator can converge too. The host never rewires its
/// own membership because a network message said so: the converged policy is
/// reported, and the local operator leaves explicitly when excluded.
pub(crate) fn receive(service: &dyn NodeSpaceContext, peer: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
    let input: ReconciliationOffer = fromCoreValue(request.args).map_err(|e| e.to_string())?;
    let current = service.spaceStore().initialize()?;
    if input.spaceId != current.spaceId {
        return Err("Reconciliation requires the peer to advertise the same shared Space".into());
    }
    let applied = mergePeerOperations(service, peer, &input.spaceId, &input.operations)?;
    service.spaceStore().importDeviceProfiles(input.deviceProfiles)?;
    let state = service.networkControlStore().currentState()?;
    service.spaceStore().alignRemoteMemberRecord(peer.to_string(), state.memberNodeIds.contains(peer))?;
    operit_util::AppLogger::AppLogger::d("SpaceReconcile", &format!(
        "shared Space reconciliation with {peer}: merged {applied} operations, members {}",
        state.memberNodeIds.len()
    ));
    let outcome = ReconciliationOutcome {
        spaceId: input.spaceId,
        operations: service.networkControlStore().currentSpaceOperations()?,
        deviceProfiles: service.spaceStore().deviceProfilesForCurrentSpace()?,
        members: state.memberNodeIds.into_iter().collect(),
    };
    toCoreValue(outcome).map_err(|e| e.to_string())
}

/// Initiator side of the exchange. Merges the peer's answer, aligns the peer's
/// member record, and retires the local membership when the converged policy
/// excludes this device - the same transition as an explicit leave, keeping
/// pairing, policy audit and business data intact.
pub(crate) async fn reconcile(service: &dyn NodeSpaceContext, peer: &str) -> Result<SharedSpaceReconciliation, String> {
    paired(service, peer)?;
    let current = service.spaceStore().initialize()?;
    let offer = ReconciliationOffer {
        spaceId: current.spaceId.clone(),
        operations: service.networkControlStore().currentSpaceOperations()?,
        deviceProfiles: service.spaceStore().deviceProfilesForCurrentSpace()?,
    };
    let outcome: ReconciliationOutcome = callPeerSpace(service, peer, "reconcileSharedSpace",
        toCoreValue(offer).map_err(|e| e.to_string())?).await?;
    if outcome.spaceId != current.spaceId {
        return Err("Reconciliation response belongs to another Space".into());
    }
    mergePeerOperations(service, peer, &current.spaceId, &outcome.operations)?;
    service.spaceStore().importDeviceProfiles(outcome.deviceProfiles)?;
    let state = service.networkControlStore().currentState()?;
    let localIsMember = state.memberNodeIds.contains(&service.localNodeId());
    if localIsMember {
        service.spaceStore().alignRemoteMemberRecord(peer.to_string(), state.memberNodeIds.contains(peer))?;
    } else {
        space_join::leave(service)?;
    }
    operit_util::AppLogger::AppLogger::d("SpaceReconcile", &format!(
        "shared Space reconciliation with {peer}: members {}, localIsMember={localIsMember}",
        state.memberNodeIds.len()
    ));
    Ok(SharedSpaceReconciliation {
        spaceId: current.spaceId,
        members: state.memberNodeIds.into_iter().collect(),
        localIsMember,
    })
}

async fn callPeerSpace<T: serde::de::DeserializeOwned>(service: &dyn NodeSpaceContext, node: &str, method: &str, args: CoreValue) -> Result<T, String> {
    let response = service.callNode(node.to_owned(), CoreCallRequest::new(
        format!("node-space-{method}-{}", currentTimeMillis()), NODE_SPACE_TARGET, method, args,
    )).await;
    fromCoreValue(response.result.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
