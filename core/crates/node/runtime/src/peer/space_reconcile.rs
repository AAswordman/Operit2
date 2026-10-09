//! Converges two paired devices that still advertise one shared Space identity
//! but hold divergent member tables. The control log is the only authority:
//! both sides offer their operations for the shared Space, accept only facts
//! the peer originated, and replay the merged log into one identical policy.
//!
//! Protocol v2 reports vector clocks and exchanges only the difference, one
//! byte-budgeted page per direction and round: a constrained 8KB link can
//! catch up after long offline periods instead of being killed by its own
//! transport ceiling. v1 peers see exactly the old single full-list exchange;
//! capability discovery is cached per peer, so mixed fleets converge on v2
//! after first contact without ever breaking v1.

use super::*;
use crate::PeerStateStore::PeerStateStore;
use operit_store::NetworkControlStore::NetworkControlCommandRecord;
use operit_store::SyncOperationStore::{SyncClock, SyncOperation, SyncOperationStore};
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const RECONCILE_PROTOCOL_VERSION: u32 = 2;
/// Rounds cap: every merged page is an idempotent prefix advance, so hitting
/// the cap leaves consistent partial progress for the next run to continue.
const MAX_RECONCILE_ROUNDS: usize = 32;
/// Hard receive-side ceilings, refusing hostile or runaway offers outright.
const MAX_RECONCILE_PAGE_OPS: usize = 4096;
const MAX_RECONCILE_PAGE_BYTES: usize = 4 * 1024 * 1024;
/// Page budget both sides declare by default: fits a constrained 8KB link
/// with room for the envelope, so legitimate catch-up cannot kill the link.
const DEFAULT_RECONCILE_PAGE_BYTES: usize = 6 * 1024;
const RECONCILE_CAPS_PATH: &str = "runtime/link_access/space_reconcile_caps.preferences.json";

#[derive(Serialize, Deserialize)]
struct ReconciliationOffer {
    spaceId: String,
    operations: Vec<SyncOperation>,
    deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    #[serde(default)]
    protocolVersion: Option<u32>,
    #[serde(default)]
    have: Option<SyncClock>,
    #[serde(default)]
    maxPageBytes: Option<u32>,
    /// Every operation id the sender already holds. An empty list on a v2
    /// offer means the set did not fit the page budget and the receiver must
    /// answer with everything, paged. Id sets, not vector clocks, drive the
    /// diff: a replica can hold arbitrary subsets of another origin's journal
    /// after a split, and a max-sequence clock would lie about the holes.
    #[serde(default)]
    haveOpIds: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
struct ReconciliationOutcome {
    spaceId: String,
    operations: Vec<SyncOperation>,
    deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    members: Vec<String>,
    #[serde(default)]
    protocolVersion: Option<u32>,
    #[serde(default)]
    have: Option<SyncClock>,
    #[serde(default)]
    floors: Option<SyncClock>,
    #[serde(default)]
    maxPageBytes: Option<u32>,
    #[serde(default)]
    more: Option<bool>,
    #[serde(default)]
    historyGap: Option<Vec<String>>,
    /// The responder's full operation-id set after merging the offer; empty
    /// means it did not fit and the initiator must send everything, paged.
    #[serde(default)]
    haveOpIds: Option<Vec<String>>,
}

/// What one initiator knows after an exchange: the converged membership and
/// whether this device is still part of the shared Space.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedSpaceReconciliation {
    pub spaceId: String,
    pub members: Vec<String>,
    pub localIsMember: bool,
    /// False when the round budget ran out: merges so far are consistent and
    /// idempotent, and the next exchange resumes from the reported clocks.
    #[serde(default)]
    pub converged: bool,
}

fn paired(service: &dyn NodeSpaceContext, peer: &str) -> Result<(), String> {
    if !service.peers()?.pairedPeers().map_err(|e| e.to_string())?
        .iter().any(|p| p.nodeId == peer) {
        return Err("Reconciliation requires a current pairing with the peer".into());
    }
    Ok(())
}

/// The highest sequence per origin one command set has reached. Sequences are
/// assigned by the origin device and never rewritten, so this vector clock is
/// a stable cursor for incremental exchanges.
fn controlClock(operations: &[SyncOperation]) -> SyncClock {
    let mut clock = SyncClock::empty();
    for operation in operations {
        if operation.sequence > clock.sequenceFor(&operation.originDeviceId) {
            clock.setSequence(operation.originDeviceId.clone(), operation.sequence);
        }
    }
    clock
}

/// One page of at most `budget` encoded bytes; the flag reports truncation.
fn pageByBudget(operations: Vec<SyncOperation>, budget: usize) -> (Vec<SyncOperation>, bool) {
    let mut page = Vec::new();
    let mut used = 0usize;
    let mut more = false;
    for operation in operations {
        let size = serde_json::to_vec(&operation).map(|encoded| encoded.len()).unwrap_or(usize::MAX);
        if !page.is_empty() && used + size > budget {
            more = true;
            break;
        }
        used += size;
        page.push(operation);
    }
    (page, more)
}

fn declaredBudget(maxPageBytes: Option<u32>) -> usize {
    maxPageBytes
        .map(|bytes| bytes as usize)
        .unwrap_or(DEFAULT_RECONCILE_PAGE_BYTES)
        .min(MAX_RECONCILE_PAGE_BYTES)
        .max(1)
}

/// Packs the id set for one message; None means "too large, fall back to a
/// full paged exchange" so the diff never breaks the page budget.
fn packOpIds(operations: &[SyncOperation], budget: usize) -> Option<Vec<String>> {
    let mut packed = Vec::new();
    let mut used = 0usize;
    for operation in operations {
        used += operation.opId.len() + 1;
        if used > budget {
            return None;
        }
        packed.push(operation.opId.clone());
    }
    Some(packed)
}

/// The exact difference a receiver must send back: what it holds that the
/// sender's id set does not cover. A None (or v1) set means "send everything".
fn missingFor(operations: &[SyncOperation], haveOpIds: &Option<Vec<String>>) -> Vec<SyncOperation> {
    match haveOpIds {
        Some(ids) => {
            let known: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
            operations
                .iter()
                .filter(|operation| !known.contains(operation.opId.as_str()))
                .cloned()
                .collect()
        }
        None => operations.to_vec(),
    }
}

fn peerProtocol(service: &dyn NodeSpaceContext, peer: &str) -> Option<u32> {
    PeerStateStore::new(service.storage())
        .records::<u32>(RECONCILE_CAPS_PATH)
        .ok()
        .and_then(|records| records.get(peer).copied())
}

fn rememberPeerProtocol(service: &dyn NodeSpaceContext, peer: &str, version: u32) {
    let store = PeerStateStore::new(service.storage());
    if store.putRecord(RECONCILE_CAPS_PATH, peer, &version).is_err() {
        operit_util::AppLogger::AppLogger::w("SpaceReconcile", &format!(
            "failed to remember protocol version {version} for {peer}"
        ));
    }
}

/// Accepts only facts the peer originated. Relayed third-party commands would
/// let one endpoint launder another issuer's authority through an otherwise
/// read-only channel, and every replica always holds its own journal, so the
/// union of both self-originated logs is already the complete history.
/// Operations outside the shared Space identity are refused outright, and a
/// page that exceeds the receive ceilings is refused whole instead of being
/// applied halfway: the merge must stay an idempotent prefix advance.
fn mergePeerOperations(service: &dyn NodeSpaceContext, peer: &str, spaceId: &str,
    operations: &[SyncOperation]) -> Result<usize, String> {
    if operations.len() > MAX_RECONCILE_PAGE_OPS {
        space_doctor::updateCounters(service, |counters| counters.oversizeOffersRefused += 1).ok();
        return Err(format!(
            "Reconciliation page of {} operations exceeds the {} operation ceiling",
            operations.len(),
            MAX_RECONCILE_PAGE_OPS
        ));
    }
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
/// caller's member record to the converged policy, and answers with what the
/// caller is missing - one budgeted page for v2 callers, the full merged log
/// for v1 callers. The host never rewires its own membership because a network
/// message said so: the converged policy is reported, and the local operator
/// leaves explicitly when excluded.
pub(crate) fn receive(service: &dyn NodeSpaceContext, peer: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
    let input: ReconciliationOffer = fromCoreValue(request.args).map_err(|e| e.to_string())?;
    let current = service.spaceStore().initialize()?;
    if input.spaceId != current.spaceId {
        return Err("Reconciliation requires the peer to advertise the same shared Space".into());
    }
    let applied = mergePeerOperations(service, peer, &input.spaceId, &input.operations)?;
    service.spaceStore().importDeviceProfiles(input.deviceProfiles)?;
    let merged = service.networkControlStore().currentSpaceOperations()?;
    let state = service.networkControlStore().currentState()?;
    service.spaceStore().alignRemoteMemberRecord(peer.to_string(), state.memberNodeIds.contains(peer))?;
    operit_util::AppLogger::AppLogger::d("SpaceReconcile", &format!(
        "shared Space reconciliation with {peer}: merged {applied} operations, members {}",
        state.memberNodeIds.len()
    ));

    let v2 = input.protocolVersion == Some(RECONCILE_PROTOCOL_VERSION);
    let requested = input.have.clone();
    let (operations, more) = if v2 {
        pageByBudget(missingFor(&merged, &input.haveOpIds), declaredBudget(input.maxPageBytes))
    } else {
        (merged.clone(), false)
    };
    let mut outcome = ReconciliationOutcome {
        spaceId: input.spaceId,
        operations,
        deviceProfiles: service.spaceStore().deviceProfilesForCurrentSpace()?,
        members: state.memberNodeIds.into_iter().collect(),
        protocolVersion: Some(RECONCILE_PROTOCOL_VERSION),
        have: Some(controlClock(&merged)),
        floors: None,
        maxPageBytes: Some(DEFAULT_RECONCILE_PAGE_BYTES as u32),
        more: None,
        historyGap: None,
        haveOpIds: None,
    };
    if v2 {
        outcome.more = Some(more);
        // None keeps meaning "set too large": the initiator then sends its
        // full paged list next round instead of trusting a truncated set.
        outcome.haveOpIds = packOpIds(&merged, DEFAULT_RECONCILE_PAGE_BYTES);
        // Report the export floors behind the requested clock: the caller can
        // then distinguish "already converged" from "history I can no longer
        // backfill" without another round trip.
        let sync = SyncOperationStore::new(service.storage(), RUNTIME_SYNC_DIR_PATH);
        let want = requested.unwrap_or_default();
        let mut floors = SyncClock::empty();
        let mut gap = Vec::new();
        for (origin, sequence) in want.sequences {
            let floor = sync.exportFloorFor(&origin).unwrap_or(0);
            floors.setSequence(origin.clone(), floor);
            if floor > sequence {
                gap.push(origin);
            }
        }
        outcome.floors = Some(floors);
        outcome.historyGap = Some(gap);
    }
    toCoreValue(outcome).map_err(|e| e.to_string())
}

/// Initiator side of the exchange. Merges the peer's pages, aligns the peer's
/// member record, and retires the local membership when the converged policy
/// excludes this device - the same transition as an explicit leave, keeping
/// pairing, policy audit and business data intact. Every round is an
/// idempotent prefix advance, so a round-budget abort is safe to resume.
pub(crate) async fn reconcile(service: &dyn NodeSpaceContext, peer: &str) -> Result<SharedSpaceReconciliation, String> {
    paired(service, peer)?;
    let current = service.spaceStore().initialize()?;
    let knownV2 = peerProtocol(service, peer) == Some(RECONCILE_PROTOCOL_VERSION);
    let mut peerBudget = DEFAULT_RECONCILE_PAGE_BYTES;
    let mut havePeerIds: Option<Vec<String>> = None;
    let mut rounds = 0usize;
    let mut converged = false;

    while rounds < MAX_RECONCILE_ROUNDS {
        rounds += 1;
        let all = service.networkControlStore().currentSpaceOperations()?;
        let localClock = controlClock(&all);
        // First contact and v1 peers get today's full list: a v1 gateway could
        // not make sense of a truncated offer. Known-v2 peers get one page of
        // exactly the operations whose ids the peer's last reply did not list.
        let (page, _pageTruncated) = if knownV2 {
            pageByBudget(missingFor(&all, &havePeerIds), peerBudget)
        } else {
            (all.clone(), false)
        };
        let offer = ReconciliationOffer {
            spaceId: current.spaceId.clone(),
            deviceProfiles: service.spaceStore().deviceProfilesForCurrentSpace()?,
            operations: page,
            protocolVersion: Some(RECONCILE_PROTOCOL_VERSION),
            have: Some(localClock),
            maxPageBytes: Some(DEFAULT_RECONCILE_PAGE_BYTES as u32),
            haveOpIds: packOpIds(&all, DEFAULT_RECONCILE_PAGE_BYTES),
        };
        let outcome: ReconciliationOutcome = callPeerSpace(service, peer, "reconcileSharedSpace",
            toCoreValue(offer).map_err(|e| e.to_string())?).await?;
        if outcome.spaceId != current.spaceId {
            return Err("Reconciliation response belongs to another Space".into());
        }
        let peerV2 = outcome.protocolVersion == Some(RECONCILE_PROTOCOL_VERSION);
        rememberPeerProtocol(service, peer, if peerV2 { RECONCILE_PROTOCOL_VERSION } else { 1 });
        mergePeerOperations(service, peer, &current.spaceId, &outcome.operations)?;
        service.spaceStore().importDeviceProfiles(outcome.deviceProfiles.clone())?;
        if let Some(gap) = outcome.historyGap.filter(|gap| !gap.is_empty()) {
            operit_util::AppLogger::AppLogger::w("SpaceReconcile", &format!(
                "peer {peer} cannot backfill origins below its export floors: {gap:?}"
            ));
        }
        if !peerV2 {
            // Legacy gateway: the full-list exchange above is already total.
            converged = true;
            break;
        }
        if let Some(budget) = outcome.maxPageBytes {
            peerBudget = declaredBudget(Some(budget));
        }
        let merged = service.networkControlStore().currentSpaceOperations()?;
        havePeerIds = outcome.haveOpIds.clone();
        // Terminated only by exact sets: the peer listed everything it holds
        // and claims nothing further, and we hold nothing outside that set.
        let stillPending = !missingFor(&merged, &outcome.haveOpIds).is_empty();
        if outcome.more != Some(true) && !stillPending {
            converged = true;
            break;
        }
    }

    let state = service.networkControlStore().currentState()?;
    let localIsMember = state.memberNodeIds.contains(&service.localNodeId());
    if localIsMember {
        service.spaceStore().alignRemoteMemberRecord(peer.to_string(), state.memberNodeIds.contains(peer))?;
    } else {
        space_join::leave(service)?;
    }
    operit_util::AppLogger::AppLogger::d("SpaceReconcile", &format!(
        "shared Space reconciliation with {peer}: members {}, localIsMember={localIsMember}, converged={converged} in {rounds} round(s)",
        state.memberNodeIds.len()
    ));
    space_doctor::updateCounters(service, |counters| {
        counters.reconcileExchanges += 1;
        counters.reconcileRounds += rounds as u64;
        if !converged {
            counters.reconcileIncomplete += 1;
        }
    }).ok();
    Ok(SharedSpaceReconciliation {
        spaceId: current.spaceId,
        members: state.memberNodeIds.into_iter().collect(),
        localIsMember,
        converged,
    })
}

async fn callPeerSpace<T: serde::de::DeserializeOwned>(service: &dyn NodeSpaceContext, node: &str, method: &str, args: CoreValue) -> Result<T, String> {
    let response = service.callNode(node.to_owned(), CoreCallRequest::new(
        format!("node-space-{method}-{}", currentTimeMillis()), NODE_SPACE_TARGET, method, args,
    )).await;
    fromCoreValue(response.result.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
