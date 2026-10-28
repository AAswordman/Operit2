//! Durable application records, not storage-engine changes. A paired gateway
//! coordinates one request; only its assigned, authorized reviewer may decide.
//! Applicant traffic stays direct. Reviewer traffic uses authenticated Space routes.
use operit_util::RuntimeStorageLayout::{RUNTIME_SPACE_MERGE_INBOUND_PATH, RUNTIME_SPACE_MERGE_OUTBOUND_PATH, RUNTIME_SPACE_MERGE_REVIEW_INBOX_PATH, RUNTIME_SPACE_MERGE_REVIEW_RESULTS_PATH, RUNTIME_SPACE_DEPARTURES_PATH, RUNTIME_SPACE_REMOVAL_EFFECTS_PATH};
use super::*;
use crate::PeerStateStore::PeerStateStore;
use operit_store::CoreSpaceStore::CoreSpaceDeviceConnection;
use operit_store::NetworkControlStore::{NetworkControlCommand, NetworkControlCommandRecord};
use operit_store::PreferencesDataStore::{CoreNodeStateStore, stringPreferencesKey};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Mutex;
#[path = "space_join_projection.rs"]
mod projection;
use projection::{ConsentProjection, JoinReply, MembershipReceipt};

// Group consent uses its own durable schema; single-device approval records are not group approvals.
pub(crate) const INBOUND: &str = RUNTIME_SPACE_MERGE_INBOUND_PATH;
pub(crate) const OUTBOUND: &str = RUNTIME_SPACE_MERGE_OUTBOUND_PATH;
pub(crate) const INBOX: &str = RUNTIME_SPACE_MERGE_REVIEW_INBOX_PATH;
pub(crate) const RESULTS: &str = RUNTIME_SPACE_MERGE_REVIEW_RESULTS_PATH;
const LIFETIME_MS: i64 = 15 * 60 * 1000;
const OFFLINE_GRACE_MS: i64 = 30_000;
static MUTATION: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Record<S = PeerSpaceSnapshot> {
    pub(crate) request: SpaceJoinRequest,
    pub(crate) sourceSpaceId: String,
    pub(crate) sourceRevision: i64,
    pub(crate) targetSpaceId: String,
    pub(crate) profile: CoreSpaceDeviceProfile,
    pub(crate) source: S,
    #[serde(default)]
    pub(crate) unavailableSince: Option<i64>,
    #[serde(default)]
    pub(crate) approvedDecision: Option<bool>,
    #[serde(default)]
    pub(crate) decisionRevision: Option<i64>,
    #[serde(default)]
    pub(crate) cancelRequested: bool,
    #[serde(default)]
    pub(crate) submitted: bool,
}
impl<S> Record<S> {
    /// Moves the consent envelope onto its independently decoded source evidence.
    fn withSource<T>(self, source: T) -> Record<T> {
        Record { request: self.request, sourceSpaceId: self.sourceSpaceId, sourceRevision: self.sourceRevision,
            targetSpaceId: self.targetSpaceId, profile: self.profile, source,
            unavailableSince: self.unavailableSince, approvedDecision: self.approvedDecision,
            decisionRevision: self.decisionRevision, cancelRequested: self.cancelRequested, submitted: self.submitted }
    }
}

#[derive(Serialize, Deserialize)]
struct Submission {
    requestId: String,
    sourceSpaceId: String,
    sourceRevision: i64,
    targetSpaceId: String,
    profile: CoreSpaceDeviceProfile,
    source: PeerSpaceSnapshot,
}
#[derive(Serialize, Deserialize)]
struct Decision {
    requestId: String,
    assignmentVersion: u64,
    approve: bool,
}
#[derive(Serialize, Deserialize)]
struct Claim {
    record: Box<Record>,
    current: PeerSpaceJoin,
}
#[derive(Serialize, Deserialize)]
struct Outcome {
    decision: Decision,
    admission: Option<SyncOperation>,
    revision: i64,
}
/// Returns the peer record adapter for the current storage host.
fn store(service: &dyn NodeSpaceContext) -> PeerStateStore {
    PeerStateStore::new(service.storage())
}
/// Identifies consent states that can still advance or be cancelled.
fn active(status: &SpaceJoinStatus) -> bool {
    matches!(status, SpaceJoinStatus::Pending | SpaceJoinStatus::Approving | SpaceJoinStatus::Approved)
}
/// Expires only unclaimed consent; an authoritative admission never expires with its UI lease.
fn expire(request: &mut SpaceJoinRequest, targetSpaceId: &str, now: i64, spaceId: &str) {
    if request.status == SpaceJoinStatus::Pending {
        if targetSpaceId != spaceId { request.status = SpaceJoinStatus::Cancelled; }
        else if request.expiresAt <= now { request.status = SpaceJoinStatus::Expired; }
    }
}
/// Loads one owned application record without carrying its snapshots in caller stack frames.
fn load(service: &dyn NodeSpaceContext, path: &str, id: &str) -> Result<Box<Record>, String> {
    let encoded: Box<serde_json::value::RawValue> = store(service).record(path, id)?.ok_or("Join request not found")?;
    let record: Record<Box<serde_json::value::RawValue>> = serde_json::from_str(encoded.get()).map_err(|error| error.to_string())?;
    drop(encoded);
    let source = serde_json::from_str(record.source.get()).map_err(|error| error.to_string())?;
    Ok(Box::new(record.withSource(source)))
}
/// Persists evidence only for live consent; accepted policy remains in the shared control log.
pub(crate) fn saveRecord(service: &dyn NodeSpaceContext, path: &str, id: &str, record: &Record) -> Result<(), String> {
    let source = &record.source;
    let constrained = service.storage().nodeLocalApplicationHistoryLimit().is_some();
    let live = active(&record.request.status);
    // Source commands have already passed departure observation on receipt.
    // Review/recovery needs the source membership and presentation facts, not
    // another copy of its authority log. Outgoing submissions retain that log.
    let compactSource = constrained && path == INBOUND;
    if live && !compactSource {
        return store(service).putRecord(path, id, record);
    }
    #[derive(Serialize)]
    struct Snapshot<'a> {
        space: &'a CoreSpace,
        deviceProfiles: &'a [CoreSpaceDeviceProfile],
        controlOperations: &'a [SyncOperation],
        topology: &'a [operit_store::CoreSpaceStore::CoreSpaceTopologyRecord],
        #[serde(skip_serializing_if = "<[SyncOperation]>::is_empty")]
        departures: &'a [SyncOperation],
    }
    #[derive(Serialize)]
    struct Durable<'a> {
        request: &'a SpaceJoinRequest,
        sourceSpaceId: &'a str,
        sourceRevision: i64,
        targetSpaceId: &'a str,
        profile: &'a CoreSpaceDeviceProfile,
        source: Snapshot<'a>,
        #[serde(skip_serializing_if = "Option::is_none")]
        unavailableSince: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        approvedDecision: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        decisionRevision: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cancelRequested: Option<bool>,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        submitted: bool,
    }
    let durable = Durable {
        request: &record.request, sourceSpaceId: &record.sourceSpaceId,
        sourceRevision: record.sourceRevision, targetSpaceId: &record.targetSpaceId,
        profile: &record.profile,
        source: Snapshot {
            space: &source.space,
            deviceProfiles: if live { &source.deviceProfiles } else { &[] },
            topology: if live { &source.topology } else { &[] },
            controlOperations: if live && !compactSource { &source.controlOperations } else { &[] },
            departures: if live && !compactSource { &source.departures } else { &[] },
        },
        unavailableSince: record.unavailableSince, approvedDecision: record.approvedDecision,
        decisionRevision: record.decisionRevision,
        // Full Core retains its explicit cancellation flag. Endpoint receipts
        // may omit false; Record's default restores the same durable intent.
        cancelRequested: if constrained && !record.cancelRequested { None } else { Some(record.cancelRequested) },
        submitted: record.submitted,
    };
    if compactSource { store(service).putCompactRecord(path, id, &durable) }
    else { store(service).putRecord(path, id, &durable) }
}
/// Checks current local authority and device review capability.
fn canReview(service: &dyn NodeSpaceContext, node: &str) -> Result<bool, String> {
    Ok(!service.networkControlStore().nodeIsDisconnected(node)?
        && service.networkControlStore().nodeHasCapability(node, "network.members.join", None)?
        && service.networkControlStore().nodeHasCapability(node, "network.approval", None)?)
}
/// Join rides any current pairing with the peer; the direction the pairing was
/// established in is transport history, not applicant authority. Approval on
/// the paired side stays the only membership consent.
fn paired(service: &dyn NodeSpaceContext, peer: &str, direction: crate::NodeServices::PairingDirection) -> Result<(), String> {
    if !service.peers()?.hasPairing(peer, direction).map_err(|e| e.to_string())? {
        return Err("Join request requires a current pairing".into());
    }
    Ok(())
}

/// Hop distance from the gateway, respecting directed links and relay permission.
/// Applicant-to-gateway adds one identical hop to every candidate. Never use RTT
/// as an unannounced replacement for hop count; ties use stable device identity.
fn hopDistances(source: &str, edges: &[CoreSpaceDeviceConnection], transit: &BTreeSet<String>) -> BTreeMap<String, u32> {
    let mut distances = BTreeMap::from([(source.to_owned(), 0)]);
    let mut queue = VecDeque::from([source.to_owned()]);
    while let Some(node) = queue.pop_front() {
        if node != source && !transit.contains(&node) { continue; }
        let nextDistance = distances[&node] + 1;
        for edge in edges.iter().filter(|e| e.firstDeviceId == node) {
            if !distances.contains_key(&edge.secondDeviceId) {
                distances.insert(edge.secondDeviceId.clone(), nextDistance);
                queue.push_back(edge.secondDeviceId.clone());
            }
        }
    }
    distances
}
/// A decision is committed once its admission operation reached the control
/// log. The log, not any local receipt, is the authority: every replica
/// replaying the shared history determines this identically, which is what
/// makes commit-based pinning and recovery safe across crashes.
pub(crate) fn admissionCommitted(service: &dyn NodeSpaceContext, record: &Record) -> Result<bool, String> {
    committedAdmission(service, &record.request, &record.sourceSpaceId, &record.targetSpaceId, &record.source.space.members)
}

/// Compares consent correlation facts against the authoritative immutable admission operation.
fn committedAdmission(service: &dyn NodeSpaceContext, request: &SpaceJoinRequest, sourceSpaceId: &str,
    targetSpaceId: &str, sourceMembers: &[String]) -> Result<bool, String> {
    let Some(reviewer) = request.reviewerDeviceId.as_deref() else { return Ok(false); };
    let id = format!("control-review-{}-{}", request.requestId, request.assignmentVersion);
    let members = sourceMembers.iter().cloned().collect::<BTreeSet<_>>();
    let currentSpaceId = service.spaceStore().initialize()?.spaceId;
    Ok(service.networkControlStore().spaceControlOperation(&currentSpaceId, &id, reviewer)?
        .is_some_and(|operation| {
            NetworkControlCommandRecord::deserialize(&operation.payload).is_ok_and(|command| {
                command.spaceId == targetSpaceId && command.issuerNodeId == operation.originDeviceId
                    && matches!(command.command, NetworkControlCommand::AdmitSpace { sourceSpaceId: admittedSource, identities }
                        if admittedSource == sourceSpaceId && identities.keys().cloned().collect::<BTreeSet<_>>() == members)
            })
    }))
}
/// Assigns review ownership using consent facts and the current authoritative admission.
pub(crate) fn assign(service: &dyn NodeSpaceContext, record: &mut Record, now: i64) -> Result<(), String> {
    let committed = record.request.status == SpaceJoinStatus::Approving && admissionCommitted(service, record)?;
    assignReviewer(service, &mut record.request, &mut record.unavailableSince, committed, now)
}

/// Updates a reviewer lease without materializing applicant profiles or policy evidence.
fn assignReviewer(service: &dyn NodeSpaceContext, request: &mut SpaceJoinRequest,
    unavailableSince: &mut Option<i64>, committed: bool, now: i64) -> Result<(), String> {
    if !matches!(request.status, SpaceJoinStatus::Pending | SpaceJoinStatus::Approving) || committed { return Ok(()); }
    let local = service.localNodeId();
    let space = service.spaceStore().initialize()?;
    let capable = |node: &str| -> Result<bool, String> {
        Ok(space.members.iter().any(|n| n == node) && canReview(service, node)?)
    };
    if let Some(node) = request.reviewerDeviceId.as_deref() {
        if capable(node)? {
            if service.nodeIsReachable(node)? {
                *unavailableSince = None;
                return Ok(()); // Stable assignment: do not bounce between equally close peers.
            }
            let since = *unavailableSince.get_or_insert(now);
            if now.saturating_sub(since) < OFFLINE_GRACE_MS { return Ok(()); }
        }
    }
    let chosen = if capable(&local)? {
        Some((0, local.clone()))
    } else {
        let peers = service.peers()?.activePeerNodeIds().map_err(|e| e.to_string())?;
        let mut edges = service.spaceStore().deviceConnections()?;
        edges.retain(|e| e.firstDeviceId != local);
        edges.extend(peers.into_iter().filter(|p| space.members.contains(p)).map(|p| CoreSpaceDeviceConnection {
            firstDeviceId: local.clone(), secondDeviceId: p,
        }));
        let distances = hopDistances(&local, &edges, &service.networkControlStore().relayNodeIds()?);
        let mut candidates = Vec::new();
        for (node, hops) in distances {
            if capable(&node)? && service.nodeIsReachable(&node)? { candidates.push((hops, node)); }
        }
        candidates.sort();
        candidates.into_iter().next()
    };
    let newId = chosen.as_ref().map(|(_, node)| node.clone());
    if newId != request.reviewerDeviceId {
        request.assignmentVersion = request.assignmentVersion.checked_add(1).ok_or("Assignment overflow")?;
    }
    request.reviewerName = match newId.as_ref() {
        Some(node) => Some(service.spaceStore().deviceProfiles()?.get(node)
            .ok_or_else(|| format!("Reviewer profile is missing: {node}"))?.displayName.clone()),
        None => None,
    };
    request.reviewerDeviceId = newId;
    request.reviewerHops = chosen.map(|(hops, _)| hops + 1);
    *unavailableSince = None;
    Ok(())
}
/// Completes a claimed approval locally when its admission is already durable
/// policy but the reviewer crashed between applying the operation and writing
/// the result record. Membership already exists in the replayed policy; this
/// only rebuilds the projection and the applicant-facing receipt.
fn recoverCommittedApproval(service: &dyn NodeSpaceContext, record: &mut Record, current: &CoreSpace) -> Result<(), String> {
    if record.request.status != SpaceJoinStatus::Approving || record.approvedDecision != Some(true) { return Ok(()); }
    if !admissionCommitted(service, record)? { return Ok(()); }
    // Publish approval only when replay still admits every source member. A
    // later policy change that excluded them leaves the record pinned for the
    // reviewer's own retry; doctor reports it instead of inventing approval.
    let state = service.networkControlStore().currentState()?;
    if !record.source.space.members.iter().all(|node| state.memberNodeIds.contains(node)) { return Ok(()); }
    drop(state);
    restoreClaimedMemberProfiles(service, record, current)?;
    // The projection rebuild needs the applicant's own presentation facts;
    // restoreClaimedMemberProfiles only covers members already projected.
    service.spaceStore().importDeviceProfiles(record.source.deviceProfiles.clone())?;
    service.spaceStore().importTopologyRecords(record.source.topology.clone())?;
    let mut joined = current.clone();
    joined.members.extend(record.source.space.members.iter().cloned());
    joined.members.sort();
    joined.members.dedup();
    if let Some(revision) = record.decisionRevision {
        joined.spaceRevision = joined.spaceRevision.max(revision);
    }
    service.spaceStore().adoptAt(joined, record.request.createdAt)?;
    record.request.status = SpaceJoinStatus::Approved;
    Ok(())
}
/// Reconciles expiry, committed admission and reviewer ownership.
pub(crate) fn reconcile(service: &dyn NodeSpaceContext, record: &mut Record) -> Result<(), String> {
    let now = currentTimeMillis();
    let current = service.spaceStore().initialize()?;
    expire(&mut record.request, &record.targetSpaceId, now, &current.spaceId);
    recoverCommittedApproval(service, record, &current)?;
    assign(service, record, now)
}

/// Reports whether the replayed control log already carries a removal command
/// for one device. The log is the authority: every replica that can read it
/// decides this identically, which is what makes self-retirement verifiable on
/// the receiving side instead of trusting a peer's claim.
pub(crate) fn memberRemovalCommitted(service: &dyn NodeSpaceContext, nodeId: &str) -> Result<bool, String> {
    let spaceId = service.spaceStore().space()?.spaceId;
    Ok(service.networkControlStore().currentSpaceOperations()?.into_iter().any(|operation| {
        NetworkControlCommandRecord::deserialize(&operation.payload).is_ok_and(|command| {
            command.spaceId == spaceId
                && matches!(&command.command, NetworkControlCommand::RemoveMember { nodeId: removed, .. } if removed == nodeId)
        })
    }))
}

/// Removals whose local effects were already applied, keyed by operation id.
/// A peer message always carries its whole log, and the same removal reaches
/// this device through several paths — control exchange, paired snapshot,
/// notice, business replication. Tracking the operation itself, instead of the
/// delivery, lets every path evaluate the removal and exactly one of them act.
pub(crate) const REMOVAL_EFFECTS_PATH: &str = RUNTIME_SPACE_REMOVAL_EFFECTS_PATH;

/// Applies the membership consequences of removals that reached this device
/// from any path: every member shrinks its own membership projection for the
/// removed device, exactly like the remover does, so the member table and the
/// device list converge from the shared log instead of a peer's rewrite. The
/// shrink is idempotent and reads the replayed current state, never the mere
/// presence of the historical removal: a device admitted again since is a
/// member, so replaying old history can never eject it.
///
/// Pairing credentials drop once per removal operation, recorded in a durable
/// ledger by operation id: whichever path first delivers the removal acts
/// here, and every later replay of the same command is inert. Binding the
/// effects to the operation rather than to one delivery path is what keeps a
/// fresh pairing for re-admission out of reach — the removal only revokes the
/// generation it was issued against.
pub(crate) fn applyRemovedMemberLocalEffects(service: &dyn NodeSpaceContext, operations: &[SyncOperation]) -> Result<(), String> {
    let local = service.localNodeId();
    let spaceId = service.spaceStore().space()?.spaceId;
    // Most exchanges contain only admissions or grants. They cannot revoke a
    // local projection or credential, so avoid a second authority replay while
    // their decoded log is still held by the caller.
    if !operations.iter().any(|operation| {
        NetworkControlCommandRecord::deserialize(&operation.payload).is_ok_and(|record| {
            record.spaceId == spaceId && match record.command {
                NetworkControlCommand::LeaveSpace => record.issuerNodeId != local,
                NetworkControlCommand::RemoveMember { nodeId, .. } => nodeId != local,
                _ => false,
            }
        })
    }) { return Ok(()); }
    let state = service.networkControlStore().currentState()?;
    let applied = removalEffectsApplied(service)?;
    let mut removed = Vec::new();
    // Every unprocessed removal of one device contributes its revoked
    // generation: a device that missed remove -> re-join -> remove receives
    // them in one batch, and the pairing is revoked when any of them revoked
    // the generation it was made under.
    let mut revocations: BTreeMap<String, Vec<Option<String>>> = BTreeMap::new();
    let mut completed = Vec::new();
    for operation in operations {
        let Ok(record) = NetworkControlCommandRecord::deserialize(&operation.payload) else { continue };
        if record.spaceId != spaceId { continue; }
        if matches!(record.command, NetworkControlCommand::LeaveSpace)
            && record.issuerNodeId != local && !state.memberNodeIds.contains(&record.issuerNodeId) {
            removed.push(record.issuerNodeId.clone());
        }
        if let NetworkControlCommand::RemoveMember { nodeId, revokedIncarnation } = &record.command {
            // The local device's own removal retires the whole Space separately,
            // and a removal the replayed policy already superseded is inert.
            if nodeId == &local || state.memberNodeIds.contains(nodeId) { continue; }
            if !removed.contains(nodeId) { removed.push(nodeId.clone()); }
            if !applied.contains(&operation.opId) && !completed.contains(&operation.opId) {
                completed.push(operation.opId.clone());
                revocations.entry(nodeId.clone()).or_default().push(revokedIncarnation.clone());
            }
        }
    }
    for nodeId in &removed {
        // No-op when the device is absent or was admitted again since.
        service.spaceStore().removeRemoteMember(nodeId.clone())?;
    }
    if !revocations.is_empty() {
        let paired = service.peers()?.pairedPeers().map_err(|error| error.to_string())?
            .into_iter().map(|peer| (peer.nodeId, peer.pairingIncarnation)).collect::<BTreeMap<_, _>>();
        let forgotten = revocations.into_iter().filter_map(|(nodeId, revoked)| {
            let held = paired.get(&nodeId)?;
            removalForgetsPairing(held.as_deref(), &revoked).then_some(nodeId)
        }).collect::<Vec<_>>();
        if !forgotten.is_empty() {
            // Dropping credentials is the step the ledger records: a failed
            // forget leaves the removals unprocessed for the next round.
            let peers = service.peers()?;
            peers.forgetPairingRecords(&forgotten).map_err(|error| error.to_string())?;
        }
    }
    markRemovalEffectsApplied(service, &completed)
}

/// Decides whether the evaluated removals forget one pairing. Each removal
/// revokes the pairing generation the device announced while it was admitted:
/// a pairing stamped with any revoked generation (or carrying no generation at
/// all, from an older core) is forgotten, while a pairing made under a
/// generation none of them revoked — the device left, reset and paired again
/// to re-apply — survives. This is the only check that tells "the credential
/// the removals revoked" from "the credential a re-admission made", because
/// pairing records carry no other version of themselves.
fn removalForgetsPairing(paired: Option<&str>, revoked: &[Option<String>]) -> bool {
    paired.is_none() || revoked.iter().any(|revoked| {
        revoked.as_deref().is_none() || revoked.as_deref() == paired
    })
}

/// Reads the removal operation ids whose local effects were already applied.
fn removalEffectsApplied(service: &dyn NodeSpaceContext) -> Result<BTreeSet<String>, String> {
    let store = CoreNodeStateStore::newWithStorage(service.storage(), REMOVAL_EFFECTS_PATH);
    let preferences = store.data().map_err(|error| error.to_string())?;
    Ok(preferences.iterEntries().map(|(opId, _)| opId.to_owned()).collect())
}

/// Records removals whose local effects were applied, so no later delivery
/// applies them a second time.
fn markRemovalEffectsApplied(service: &dyn NodeSpaceContext, operationIds: &[String]) -> Result<(), String> {
    if operationIds.is_empty() { return Ok(()); }
    let store = CoreNodeStateStore::newWithStorage(service.storage(), REMOVAL_EFFECTS_PATH);
    let mut preferences = store.data().map_err(|error| error.to_string())?;
    for operationId in operationIds {
        preferences.set(&stringPreferencesKey(operationId), "applied".to_string());
    }
    store.replaceRecoverably(preferences).map_err(|error| error.to_string())
}

/// Drops pairing credentials in the very step that changes the member state.
/// Deferring this cannot tell an old pairing from one the device created by
/// pairing again for re-admission, because pairing records carry no instance
/// identity, so a late sweep would delete the fresh edge as well.
pub(crate) fn forgetPairingRecords(service: &dyn NodeSpaceContext, nodeIds: &[String]) {
    if nodeIds.is_empty() { return; }
    let result = service
        .peers()
        .map_err(|error| error.to_string())
        .and_then(|peers| peers.forgetPairingRecords(nodeIds).map_err(|error| error.to_string()));
    if let Err(error) = result {
        operit_util::AppLogger::AppLogger::w(
            "SpaceJoin",
            &format!("Pairing credential forget failed devices={nodeIds:?} error={error}"),
        );
    }
}

/// Retires this device once the shared policy removed it. The check is
/// replay-based, never a peer's word: a removal may arrive as a direct notice,
/// a control exchange or a snapshot, and each one only takes effect after the
/// local log proves it. Returns the Space id it left.
pub(crate) fn retireRemovedLocalDevice(service: &dyn NodeSpaceContext, keepPeer: Option<&str>) -> Result<Option<String>, String> {
    let local = service.localNodeId();
    let state = service.networkControlStore().currentState()?;
    // The decision reads the replayed current state, never the mere presence of
    // a removal in history: a device that was admitted again after an earlier
    // removal is a member, so recovering the shared log can never retire it
    // again. The same rule makes re-read history harmless after a re-join.
    if state.memberNodeIds.contains(&local) { return Ok(None); }
    // A device with an application in flight is re-joining on purpose: the join
    // flow reconciles the freshest truth (and retires at that point), while a
    // partially recovered log can transiently look like a removal.
    if outgoing(service)?.iter().any(|request| active(&request.status)) { return Ok(None); }
    if !memberRemovalCommitted(service, &local)? { return Ok(None); }
    let previous = service.spaceStore().space()?.spaceId;
    let devices = service
        .peers()?
        .pairedPeers()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|peer| peer.nodeId)
        .filter(|nodeId| Some(nodeId.as_str()) != keepPeer)
        .collect::<Vec<_>>();
    leave(service)?;
    // Mutual forgetting happens now, not in a later sweep: a deferred delete
    // would also match the pairing the device just made to re-join.
    forgetPairingRecords(service, &devices);
    Ok(Some(previous))
}

/// Local ownership recovery is independent of direct pairing. Serialize it
/// with approval mutations so a decision cannot restore the Space being left.
pub(crate) fn leave(service: &dyn NodeSpaceContext) -> Result<CoreSpace, String> {
    let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    leaveLocked(service)
}

/// A crash can land between persisting the exit and switching the local Space.
/// Finish only that committed transition; historical exits never undo admission.
pub(crate) fn recoverLocalDeparture(service: &dyn NodeSpaceContext) -> Result<(), String> {
    let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if service.networkControlStore().committedLocalDeparture()?.is_some() { leaveLocked(service)?; }
    Ok(())
}

fn leaveLocked(service: &dyn NodeSpaceContext) -> Result<CoreSpace, String> {
    // Notification is best effort. A device owns its local exit even when the
    // old Space, its approval receipts or its peers cannot be read.
    if let Err(error) = super::super::fault_isolation::run("prepare Space departure", || space_departure::prepare(service)) {
        operit_util::AppLogger::AppLogger::w("SpaceJoin", &error);
        let _ = CoreNodeStateStore::newWithStorage(service.storage(), RUNTIME_SPACE_DEPARTURES_PATH).delete();
    }
    if let Err(error) = store(service).clearSpaceApprovals() {
        operit_util::AppLogger::AppLogger::w("SpaceJoin", &format!("Exit Space: clear approvals: {error}"));
    }
    let space = service.spaceStore().leave()?;
    for (operation, result) in [
        ("reset old control projection", super::super::fault_isolation::run("reset control projection", || service.networkControlStore().resetLocalProjection())),
        ("initialize new local policy", super::super::fault_isolation::run("initialize local policy", || service.networkControlStore().initializeCurrentSpace().map(|_| ()))) ,
    ] {
        if let Err(error) = result {
            operit_util::AppLogger::AppLogger::w("SpaceJoin", &format!("Space {}: {operation}: {error}", space.spaceId));
        }
    }
    // Once local membership has switched, ancillary failures cannot turn a
    // completed exit into an error or keep the user attached to the old Space.
    if let Err(error) = super::super::fault_isolation::run("refresh pairing generation after Space exit", || {
        let incarnation = store(service).renewPairingIncarnation()?;
        if let Some(profile) = service.spaceStore().deviceProfiles()?.remove(&service.localNodeId()) {
            if let Some(coreVersion) = profile.coreVersion {
                service.spaceStore().writeLocalDeviceProfile(profile.displayName, profile.platform,
                    profile.model, coreVersion, Some(incarnation))?;
            }
        }
        Ok(())
    }) {
        operit_util::AppLogger::AppLogger::w("SpaceJoin", &error);
    }
    Ok(space)
}

/// Completes preparation before polling any submission response or receipt decoder.
pub(crate) async fn request(service: &dyn NodeSpaceContext, deviceId: String) -> Result<SpaceJoinRequest, String> {
    let id = Box::pin(prepareRequest(service, deviceId)).await?;
    refresh(service, id).await
}

/// Resolves one immutable submission and releases its working snapshots before returning its identifier.
async fn prepareRequest(service: &dyn NodeSpaceContext, deviceId: String) -> Result<String, String> {
    let mut retiredStaleSplit = false;
    loop {
        paired(service, &deviceId, crate::NodeServices::PairingDirection::Either)?;
        if let Some(old) = outgoing(service)?.into_iter().find(|r| r.targetDeviceId == deviceId && active(&r.status)) {
            return Ok(old.requestId);
        }
        let snapshot: PeerSpaceSnapshot = Box::pin(joinSnapshot(service, &deviceId)).await?;
        if !snapshot.space.members.contains(&deviceId) { return Err("Target is not in its advertised Space".into()); }
        space_departure::observe(service, &deviceId, &snapshot.departures, &snapshot.controlOperations)?;
        let source = peerSpaceSnapshot(service)?;
        let local = source.space.clone();
        let localId = service.localNodeId();
        // One shared space identity is never joinable: either both sides already
        // agree on membership, or their member tables diverged. The divergent case
        // is reconciled on the spot - both control logs merge into one policy - so
        // the retried submission sees a single truth instead of a split.
        if local.spaceId == snapshot.space.spaceId {
            if snapshot.space.members.contains(&localId) {
                return Err("This device is already a member of the target Space".into());
            }
            let reconciled = space_reconcile::reconcile(service, &deviceId).await?;
            let converged: PeerSpaceSnapshot = Box::pin(joinSnapshot(service, &deviceId)).await?;
            if converged.space.members.contains(&localId) {
                return Err("This device is already a member of the target Space".into());
            }
            // The converged truth excludes this device: its records still named
            // the shared Space while the shared log does not admit it. The
            // reconcile already retires that stale membership; apply again as the
            // independent Space this device now is, so a removed device submits
            // its request on this attempt instead of retrying by hand. The
            // submission that follows carries the retired single-device source,
            // never the divergent projection.
            if !retiredStaleSplit {
                retiredStaleSplit = true;
                // The pairing this application arrived on is the transport of the
                // application itself, so the retirement keeps it.
                retireRemovedLocalDevice(service, Some(&deviceId))?;
                continue;
            }
            return Err(format!(
                "Membership split with {deviceId} reconciled; the shared Space control log admits {} \
                 and this device is {}. Submit a new join request from the reconciled state.",
                reconciled.members.join(", "),
                if reconciled.localIsMember { "a member" } else { "not a member" }
            ));
        }
        return prepareSubmission(service, deviceId, source, snapshot);
    }
}

/// Persists the immutable consent source and releases both snapshots before the network response phase.
fn prepareSubmission(service: &dyn NodeSpaceContext, deviceId: String, source: PeerSpaceSnapshot, snapshot: PeerSpaceSnapshot) -> Result<String, String> {
    let localId = service.localNodeId();
    let local = &source.space;
    let profile = source.deviceProfiles.iter().find(|p| p.nodeId == localId)
        .cloned().ok_or("Local device profile missing")?;
    let applicantName = source.deviceProfiles.iter()
        .filter(|p| source.space.members.iter().any(|node| node == &p.nodeId))
        .map(|p| p.displayName.clone()).collect::<Vec<_>>().join(", ");
    let now = currentTimeMillis();
    let record = Record {
        request: SpaceJoinRequest { requestId: uuid::Uuid::new_v4().to_string(), targetDeviceId: deviceId,
            applicantDeviceId: localId, applicantName, spaceName: snapshot.space.spaceName,
            status: SpaceJoinStatus::Pending, createdAt: now, expiresAt: now + LIFETIME_MS, canApprove: false,
            reviewerDeviceId: None, reviewerName: None, reviewerHops: None, assignmentVersion: 0, decisionApprove: None },
        sourceSpaceId: local.spaceId.clone(), sourceRevision: local.spaceRevision,
        targetSpaceId: snapshot.space.spaceId, profile, source, unavailableSince: None, approvedDecision: None, decisionRevision: None, cancelRequested: false, submitted: false,
    };
    saveRecord(service, OUTBOUND, &record.request.requestId, &record)?;
    Ok(record.request.requestId)
}

/// Encodes the immutable submission without cloning evidence into an async frame.
fn submissionValue(service: &dyn NodeSpaceContext, id: &str) -> Result<CoreValue, String> {
    #[derive(Serialize)]
    struct SubmissionView<'a> {
        requestId: &'a str,
        sourceSpaceId: &'a str,
        sourceRevision: i64,
        targetSpaceId: &'a str,
        profile: &'a CoreSpaceDeviceProfile,
        source: &'a PeerSpaceSnapshot,
    }
    let mut record = load(service, OUTBOUND, id)?;
    let operations = takeSnapshotOperations(&mut record.source);
    let mut value = toCoreValue(SubmissionView {
        requestId: &record.request.requestId, sourceSpaceId: &record.sourceSpaceId,
        sourceRevision: record.sourceRevision, targetSpaceId: &record.targetSpaceId,
        profile: &record.profile, source: &record.source,
    }).map_err(|error| error.to_string())?;
    let CoreValue::Map(fields) = &mut value else { return Err("Invalid join submission envelope".into()); };
    attachSnapshotOperations(fields.get_mut("source").ok_or("Missing join source")?, operations)?;
    Ok(value)
}

/// Correlates a full claimed record without projecting or copying its evidence.
fn responseIdentity(record: &Record) -> [&str; 5] {
    projection::identity(&record.request, &record.sourceSpaceId, &record.targetSpaceId)
}

/// Loads a consent read model without decoding source profiles or policy history.
fn consent(service: &dyn NodeSpaceContext, path: &str, id: &str) -> Result<ConsentProjection, String> {
    store(service).record(path, id)?.ok_or_else(|| "Join request not found".into())
}

/// Reads the outgoing list without materializing source evidence or membership receipts.
pub(crate) fn outgoing(service: &dyn NodeSpaceContext) -> Result<Vec<SpaceJoinRequest>, String> {
    let mut records: Vec<_> = store(service).independentRecords::<ConsentProjection>(OUTBOUND)?.into_values().map(|r| r.request).collect();
    // Do not expire locally: a lost reply may hide a completed remote approval.
    records.sort_by_key(|r| r.createdAt);
    Ok(records)
}

/// Retires application/review state when the pairing that authorized it is
/// explicitly forgotten. A re-paired peer must submit a new request, never
/// reuse an old admission that may already have been revoked by Space policy.
pub(crate) fn purgeDevice(service: &dyn NodeSpaceContext, deviceId: &str) -> Result<(), String> {
    purgePeerRecords(&store(service), deviceId)
}

pub(crate) fn purgePeerRecords(store: &PeerStateStore, deviceId: &str) -> Result<(), String> {
    #[derive(Deserialize)]
    struct RequestPeers {
        applicantDeviceId: String,
        targetDeviceId: String,
    }
    #[derive(Deserialize)]
    struct RecordPeers {
        request: RequestPeers,
    }

    let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    // Read only identities, not complete source snapshots. Validate every
    // request file before deleting anything, preserving unrelated requests.
    let records = [INBOUND, OUTBOUND, INBOX]
        .into_iter()
        .map(|path| {
            let ids = store
                .records::<RecordPeers>(path)?
                .into_iter()
                .filter(|(_, record)| {
                    record.request.applicantDeviceId == deviceId
                        || record.request.targetDeviceId == deviceId
                })
                .map(|(id, _)| id)
                .collect::<Vec<_>>();
            Ok((path, ids))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // Remove retry outcomes first: if a later write fails, the remaining
    // request still identifies the work to finish on an idempotent retry.
    for id in records
        .iter()
        .flat_map(|(_, ids)| ids)
        .collect::<BTreeSet<_>>()
    {
        store.deleteRecord(RESULTS, id)?;
    }
    for (path, ids) in records {
        for id in ids {
            store.deleteRecord(path, &id)?;
        }
    }
    Ok(())
}

/// Receives one applicant command and returns only its status and membership projection.
pub(crate) fn receive(service: &dyn NodeSpaceContext, peer: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
    let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    paired(service, peer, crate::NodeServices::PairingDirection::Either)?;
    let current = service.spaceStore().initialize()?;
    let summary = match request.methodName.as_str() {
        "requestJoin" | "cancelJoin" => {
            let input: Box<Submission> = fromCoreValue(request.args).map_err(|e| e.to_string())?;
            receiveCommand(service, peer, &current, input, request.methodName == "cancelJoin")?
        }
        "joinStatus" => receiveStatus(service, peer, &current, fromCoreValue(request.args).map_err(|e| e.to_string())?)?,
        _ => return Err("Unknown Space join method".into()),
    };
    toCoreValue(membershipReply(service, summary, &service.spaceStore().space()?)?).map_err(|error| error.to_string())
}

/// Decodes one command before taking the mutation lock and loading current authority.
pub(crate) fn receiveEncoded(service: &dyn NodeSpaceContext, peer: &str, bytes: &mut Vec<u8>, method: &str)
    -> Result<JoinReply, String> {
    let input = decodeApplicantCommand(bytes, method)?;
    let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    paired(service, peer, crate::NodeServices::PairingDirection::Either)?;
    let current = service.spaceStore().initialize()?;
    let summary = match input {
        ApplicantCommand::Submit(input) => receiveCommand(service, peer, &current, input, method == "cancelJoin")?,
        ApplicantCommand::Status(id) => receiveStatus(service, peer, &current, id)?,
    };
    membershipReply(service, summary, &service.spaceStore().space()?)
}

/// Typed applicant inputs keep immutable command evidence separate from status requests.
enum ApplicantCommand { Submit(Box<Submission>), Status(String) }

/// Releases the authenticated input frame after direct typed argument decoding.
fn decodeApplicantCommand(bytes: &mut Vec<u8>, method: &str) -> Result<ApplicantCommand, String> {
    let args = encodedSpaceArguments(bytes)?;
    let input = match method {
        "requestJoin" | "cancelJoin" => ApplicantCommand::Submit(decodeSubmission(args)?),
        "joinStatus" => ApplicantCommand::Status(operit_link::decodeLink(args).map_err(|e| e.to_string())?),
        _ => return Err("Unknown Space join method".into()),
    };
    drop(std::mem::take(bytes));
    Ok(input)
}

/// Decodes submission fields independently so source evidence does not nest decoder frames.
fn decodeSubmission(args: &[u8]) -> Result<Box<Submission>, String> {
    let source = operit_link::codec::linkStructField(args, "source", 5)
        .map_err(|error| error.to_string())?.ok_or("Missing join source")?;
    Ok(Box::new(Submission {
        requestId: decodeSpaceField(args, "requestId", 0)?,
        sourceSpaceId: decodeSpaceField(args, "sourceSpaceId", 1)?,
        sourceRevision: decodeSpaceField(args, "sourceRevision", 2)?,
        targetSpaceId: decodeSpaceField(args, "targetSpaceId", 3)?,
        profile: decodeSpaceField(args, "profile", 4)?,
        source: PeerSpaceSnapshot {
            space: decodeSpaceField(source, "space", 0)?,
            deviceProfiles: decodeSpaceField(source, "deviceProfiles", 1)?,
            controlOperations: decodeSpaceField(source, "controlOperations", 2)?,
            topology: decodeSpaceField(source, "topology", 3)?,
            departures: match operit_link::codec::linkStructField(source, "departures", 4).map_err(|error| error.to_string())? {
                Some(bytes) => operit_link::decodeLink(bytes).map_err(|error| error.to_string())?,
                None => Vec::new(),
            },
        },
    }))
}

/// Reconciles an applicant's selected status without decoding its submission evidence.
fn receiveStatus(service: &dyn NodeSpaceContext, peer: &str, current: &CoreSpace, id: String) -> Result<ConsentProjection, String> {
    let mut summary = consent(service, INBOUND, &id)?;
    if summary.request.applicantDeviceId != peer { return Err("Join request belongs to another device".into()); }
    reconcileProjection(service, &mut summary, current)?;
    Ok(summary)
}

/// Applies a typed command and releases evidence before constructing its reply.
fn receiveCommand(service: &dyn NodeSpaceContext, peer: &str, current: &CoreSpace, input: Box<Submission>,
    cancellation: bool) -> Result<ConsentProjection, String> {
    let mut record = receiveSubmission(service, peer, current, input, cancellation)?;
    if cancellation {
        if record.request.status == SpaceJoinStatus::Pending { record.request.status = SpaceJoinStatus::Cancelled; }
    } else {
        reconcile(service, &mut record)?;
    }
    saveRecord(service, INBOUND, &record.request.requestId, &record)?;
    Ok(ConsentProjection::from(record.as_ref()))
}

/// Validates immutable applicant evidence while retaining only headers for unrelated requests.
fn receiveSubmission(service: &dyn NodeSpaceContext, peer: &str, current: &CoreSpace,
    input: Box<Submission>, cancellation: bool) -> Result<Box<Record>, String> {
    if !cancellation {
        CoreSpaceStore::validateSpaceProfiles(&input.source.space, &input.source.deviceProfiles)?;
        if input.source.space.spaceId != input.sourceSpaceId || input.source.space.spaceRevision != input.sourceRevision
            || !input.source.space.members.iter().any(|node| node == peer)
            || !input.source.deviceProfiles.iter().any(|profile| profile == &input.profile) {
            return Err("Join source snapshot does not match its applicant".into());
        }
    }
    uuid::Uuid::parse_str(&input.requestId).map_err(|_| "Invalid join request id")?;
    if input.profile.nodeId != peer || input.sourceRevision <= 0 || (!cancellation && input.targetSpaceId != current.spaceId)
        || input.sourceSpaceId.is_empty() || input.profile.displayName.len() > 512 {
        return Err("Join request identity/Space mismatch".into());
    }
    if !cancellation && input.sourceSpaceId == current.spaceId {
        return Err("Join request must merge a distinct source Space, not the target itself".into());
    }
    space_departure::observe(service, peer, &[], &input.source.controlOperations)?;
    let mut records: Vec<_> = store(service).independentRecords::<ConsentProjection>(INBOUND)?.into_values().collect();
    pruneCompletedRequests(service, &mut records)?;
    if let Some(old) = records.iter().find(|record| record.request.requestId == input.requestId) {
        if old.request.applicantDeviceId != peer || old.targetSpaceId != input.targetSpaceId
            || old.sourceSpaceId != input.sourceSpaceId || old.sourceRevision != input.sourceRevision {
            return Err("Join request id belongs to another submission".into());
        }
        return load(service, INBOUND, &input.requestId);
    }
    let now = currentTimeMillis();
    if !cancellation {
        for summary in &mut records {
            if summary.request.applicantDeviceId == peer && summary.targetSpaceId == current.spaceId
                && active(&summary.request.status) && summary.sourceSpaceId != input.sourceSpaceId {
                let mut record = load(service, INBOUND, &summary.request.requestId)?;
                record.request.status = SpaceJoinStatus::Cancelled;
                saveRecord(service, INBOUND, &record.request.requestId, &record)?;
                summary.request.status = SpaceJoinStatus::Cancelled;
            }
        }
        if records.iter().filter(|r| active(&r.request.status) && r.request.expiresAt > now).count() >= 64 {
            return Err("Too many pending join requests".into());
        }
        if records.iter().any(|r| r.request.applicantDeviceId == peer && r.targetSpaceId == current.spaceId
            && active(&r.request.status) && (r.request.status != SpaceJoinStatus::Pending || r.request.expiresAt > now)) {
            return Err("An active join request already exists".into());
        }
    }
    Ok(Box::new(Record {
        request: SpaceJoinRequest { requestId: input.requestId, targetDeviceId: service.localNodeId(),
            applicantDeviceId: peer.into(), applicantName: input.source.deviceProfiles.iter()
                .filter(|p| input.source.space.members.iter().any(|node| node == &p.nodeId))
                .map(|p| p.displayName.clone()).collect::<Vec<_>>().join(", "),
            spaceName: current.spaceName.clone(), status: SpaceJoinStatus::Pending,
            createdAt: now, expiresAt: now + LIFETIME_MS, canApprove: false,
            reviewerDeviceId: None, reviewerName: None, reviewerHops: None, assignmentVersion: 0, decisionApprove: None },
        sourceSpaceId: input.sourceSpaceId, sourceRevision: input.sourceRevision,
        targetSpaceId: input.targetSpaceId, profile: input.profile, source: input.source,
        unavailableSince: None,
        approvedDecision: None, decisionRevision: None, cancelRequested: false, submitted: false,
    }))
}

/// Projects membership only after matching the decision to the current authoritative admission.
fn membershipReply(service: &dyn NodeSpaceContext, summary: ConsentProjection, current: &CoreSpace) -> Result<JoinReply, String> {
    let accepted = if summary.request.status == SpaceJoinStatus::Approved {
        if summary.approvedDecision != Some(true) || summary.decisionRevision.is_none()
            || summary.targetSpaceId != current.spaceId
            || !committedAdmission(service, &summary.request, &summary.sourceSpaceId, &summary.targetSpaceId, &summary.source.space.members)? {
            return Err("Approved join has no committed admission".into());
        }
        let state = service.networkControlStore().currentState()?;
        if !summary.source.space.members.iter().all(|node| state.memberNodeIds.contains(node)) {
            return Err("Approved target membership changed".into());
        }
        Some(MembershipReceipt { space: current.clone() })
    } else { None };
    Ok(JoinReply { request: summary.request, sourceSpaceId: summary.sourceSpaceId,
        sourceRevision: summary.sourceRevision, targetSpaceId: summary.targetSpaceId,
        accepted, approvedDecision: summary.approvedDecision, decisionRevision: summary.decisionRevision })
}

/// Reconciles a read model and loads evidence only for an actual state change or committed recovery.
fn reconcileProjection(service: &dyn NodeSpaceContext, summary: &mut ConsentProjection, current: &CoreSpace) -> Result<(), String> {
    let before = (summary.request.clone(), summary.unavailableSince);
    let now = currentTimeMillis();
    expire(&mut summary.request, &summary.targetSpaceId, now, &current.spaceId);
    let committed = summary.request.status == SpaceJoinStatus::Approving
        && committedAdmission(service, &summary.request, &summary.sourceSpaceId, &summary.targetSpaceId, &summary.source.space.members)?;
    if committed && summary.approvedDecision == Some(true) {
        let mut record = load(service, INBOUND, &summary.request.requestId)?;
        reconcile(service, &mut record)?;
        if record.request != before.0 || record.unavailableSince != before.1 {
            saveRecord(service, INBOUND, &record.request.requestId, &record)?;
        }
        *summary = ConsentProjection::from(record.as_ref());
        return Ok(());
    }
    assignReviewer(service, &mut summary.request, &mut summary.unavailableSince, committed, now)?;
    if summary.request != before.0 || summary.unavailableSince != before.1 {
        let mut record = load(service, INBOUND, &summary.request.requestId)?;
        record.request = summary.request.clone();
        record.unavailableSince = summary.unavailableSince;
        saveRecord(service, INBOUND, &record.request.requestId, &record)?;
    }
    Ok(())
}

/// Calls the review owner through the same typed protocol for local and remote decisions.
async fn approvalCall<T: serde::de::DeserializeOwned>(service: &dyn NodeSpaceContext, node: &str, method: &str, args: CoreValue) -> Result<T, String> {
    let request = CoreCallRequest::new(format!("space-approval-{}", uuid::Uuid::new_v4()), NODE_SPACE_APPROVAL_TARGET, method, args);
    let value = if node == service.localNodeId() {
        receiveApproval(service, node, request)?
    } else {
        service.callNode(node.to_owned(), request).await.result.map_err(|e| e.to_string())?
    };
    fromCoreValue(value).map_err(|e| e.to_string())
}
/// Every reviewer pulls only its assignments from reachable same-Space gateways.
/// No applicant data is replicated/broadcast and no other admin gets a popup.
pub(crate) fn hasInbound(service: &dyn NodeSpaceContext) -> Result<bool, String> {
    Ok(!store(service).independentRecords::<ConsentProjection>(INBOUND)?.is_empty())
}

/// Read-only review-state summary for diagnostics, judged through the shared
/// commit predicate so every replica describes stuck records identically.
pub(crate) fn doctorSummaries(service: &dyn NodeSpaceContext) -> Result<Vec<space_doctor::JoinHealth>, String> {
    let now = currentTimeMillis();
    let mut summaries = Vec::new();
    let mut seen = BTreeSet::new();
    for path in [INBOUND, INBOX] {
        for record in store(service).independentRecords::<ConsentProjection>(path)?.into_values() {
            if !seen.insert(record.request.requestId.clone()) { continue; }
            let committed = record.request.status == SpaceJoinStatus::Approving
                && record.approvedDecision == Some(true)
                && committedAdmission(service, &record.request, &record.sourceSpaceId, &record.targetSpaceId, &record.source.space.members)?;
            let recoverable = if committed {
                let state = service.networkControlStore().currentState()?;
                record.source.space.members.iter().all(|node| state.memberNodeIds.contains(node))
            } else { false };
            let reassignable = record.request.status == SpaceJoinStatus::Approving && !committed
                && record.unavailableSince.is_some_and(|since| now.saturating_sub(since) > OFFLINE_GRACE_MS);
            summaries.push(space_doctor::JoinHealth {
                requestId: record.request.requestId.clone(),
                applicantDeviceId: record.request.applicantDeviceId.clone(),
                status: record.request.status.clone(),
                reviewerDeviceId: record.request.reviewerDeviceId.clone(),
                assignmentVersion: record.request.assignmentVersion,
                approvedDecision: record.approvedDecision,
                committed,
                recoverable,
                reassignable,
            });
        }
    }
    Ok(summaries)
}
/// Collects public review requests from explicitly addressed assignment sources.
pub(crate) async fn incoming(service: &dyn NodeSpaceContext) -> Result<Vec<SpaceJoinRequest>, String> {
    let local = service.localNodeId();
    if !canReview(service, &local)? { return Ok(Vec::new()); }
    let mut result = Vec::new();
    for node in service.spaceStore().initialize()?.members {
        if !service.nodeIsReachable(&node)? { continue; }
        let fetched = if node == local {
            let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            assignedRequests(service, &local, &localDecisionContext(service)?)
        } else { approvalCall::<Vec<ConsentProjection>>(service, &node, "assigned", CoreValue::Null).await };
        let records = match fetched {
            Ok(records) => records,
            Err(error) => {
                operit_util::AppLogger::AppLogger::v_with_level("SpaceJoin", &format!("Assignment source unavailable {node}: {error}"), operit_util::AppLogger::VERBOSE_LEVEL_3);
                continue;
            }
        };
        for mut record in records {
            if record.request.targetDeviceId != node || record.request.reviewerDeviceId.as_deref() != Some(local.as_str())
                || record.targetSpaceId != service.spaceStore().initialize()?.spaceId { continue; }
            record.request.canApprove = true;
            if record.request.targetDeviceId != local {
                store(service).putRecord(INBOX, &record.request.requestId, &record)?;
            }
            result.push(record.request);
        }
    }
    result.sort_by_key(|r| r.createdAt);
    Ok(result)
}
/// Rejects a stale or unauthorized review claim.
fn validateAssignment(service: &dyn NodeSpaceContext, record: &Record, origin: &str, decision: &Decision) -> Result<(), String> {
    if record.request.reviewerDeviceId.as_deref() != Some(origin)
        || record.request.assignmentVersion != decision.assignmentVersion
        || record.targetSpaceId != service.spaceStore().initialize()?.spaceId
        || !canReview(service, origin)? {
        return Err("Join assignment changed or this device has no approval permission".into());
    }
    Ok(())
}
/// Endpoint NVS holds consent in flight and two recent terminal receipts, not
/// an unbounded application archive. Active claims and approved handoffs stay
/// durable; a retired request cannot be replayed without fresh user consent.
fn pruneCompletedRequests(service: &dyn NodeSpaceContext, records: &mut Vec<ConsentProjection>) -> Result<(), String> {
    let Some(limit) = service.storage().nodeLocalApplicationHistoryLimit() else { return Ok(()); };
    let mut terminal: Vec<_> = records.iter().filter(|record| !active(&record.request.status))
        .map(|record| (record.request.createdAt, record.request.requestId.clone())).collect();
    terminal.sort_by(|left, right| right.cmp(left));
    for (_, id) in terminal.into_iter().skip(limit) {
        store(service).deleteRecord(RESULTS, &id)?;
        store(service).deleteRecord(INBOUND, &id)?;
        records.retain(|record| record.request.requestId != id);
    }
    Ok(())
}
/// Reconciles live gateway records individually and returns only their review projections.
fn assignedRequests(service: &dyn NodeSpaceContext, origin: &str, current: &CoreSpace) -> Result<Vec<ConsentProjection>, String> {
    let mut summaries: Vec<_> = store(service).independentRecords::<ConsentProjection>(INBOUND)?.into_values().collect();
    pruneCompletedRequests(service, &mut summaries)?;
    let mut assigned = Vec::new();
    for mut summary in summaries {
        if !matches!(summary.request.status, SpaceJoinStatus::Pending | SpaceJoinStatus::Approving) { continue; }
        if let Err(error) = crate::fault_isolation::run("join assignment", || reconcileProjection(service, &mut summary, current)) {
            operit_util::AppLogger::AppLogger::w("SpaceJoin", &format!("request={}: {error}", summary.request.requestId));
            continue;
        }
        if summary.targetSpaceId == current.spaceId && summary.request.reviewerDeviceId.as_deref() == Some(origin)
            && matches!(summary.request.status, SpaceJoinStatus::Pending | SpaceJoinStatus::Approving)
            && canReview(service, origin)? {
            assigned.push(summary);
        }
    }
    Ok(assigned)
}

/// Dispatches reviewer commands while keeping assignment queries free of application evidence.
pub(crate) fn receiveApproval(service: &dyn NodeSpaceContext, origin: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
    let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let current = service.spaceStore().initialize()?;
    if !current.members.iter().any(|n| n == origin) || service.networkControlStore().nodeIsDisconnected(origin)? {
        return Err("Approval requires an active member of this Space".into());
    }
    match request.methodName.as_str() {
        "assigned" => toCoreValue(assignedRequests(service, origin, &current)?).map_err(|e| e.to_string()),
        "claim" => {
            let decision: Decision = fromCoreValue(request.args).map_err(|e| e.to_string())?;
            toCoreValue(claimDecision(service, origin, decision, current)?).map_err(|e| e.to_string())
        }
        "complete" => {
            let outcome: Outcome = fromCoreValue(request.args).map_err(|e| e.to_string())?;
            let record = completeDecision(service, origin, outcome, current)?;
            toCoreValue(membershipReply(service, ConsentProjection::from(record.as_ref()), &service.spaceStore().space()?)?).map_err(|e| e.to_string())
        }
        _ => Err("Unknown Space approval method".into()),
    }
}

/// Repair presentation bytes from this durable review only after its matching
/// admission is authoritative. Missing profiles must not prevent resuming a
/// membership prefix written before the final approval receipt.
fn restoreClaimedMemberProfiles(service: &dyn NodeSpaceContext, record: &Record, current: &CoreSpace) -> Result<(), String> {
    if record.approvedDecision != Some(true)
        || !matches!(record.request.status, SpaceJoinStatus::Approving | SpaceJoinStatus::Approved) {
        return Ok(());
    }
    let profiles = service.spaceStore().deviceProfiles()?;
    if current.members.iter().all(|node| profiles.contains_key(node)) { return Ok(()); }
    if !admissionCommitted(service, record)? { return Ok(()); }
    CoreSpaceStore::validateSpaceProfiles(&record.source.space, &record.source.deviceProfiles)?;
    service.spaceStore().importDeviceProfiles(record.source.deviceProfiles.iter()
        .filter(|profile| current.members.contains(&profile.nodeId) && !profiles.contains_key(&profile.nodeId))
        .cloned().collect())
}

/// Applies the same claim transition to local and authenticated remote reviewers.
fn claimDecision(service: &dyn NodeSpaceContext, origin: &str, decision: Decision, current: CoreSpace) -> Result<Claim, String> {
            let mut record = load(service, INBOUND, &decision.requestId)?;
            let before = (record.request.clone(), record.unavailableSince);
            reconcile(service, &mut record)?;
            // Persist real reassignment or expiry even for a stale reviewer.
            if record.request != before.0 || record.unavailableSince != before.1 {
                saveRecord(service, INBOUND, &decision.requestId, &record)?;
            }
            validateAssignment(service, &record, origin, &decision)?;
            if record.request.status == SpaceJoinStatus::Pending {
                paired(service, &record.request.applicantDeviceId, crate::NodeServices::PairingDirection::Either)?;
                record.request.status = SpaceJoinStatus::Approving;
                record.approvedDecision = Some(decision.approve);
                record.request.decisionApprove = Some(decision.approve);
                saveRecord(service, INBOUND, &decision.requestId, &record)?;
            } else if record.approvedDecision != Some(decision.approve)
                || !matches!(record.request.status, SpaceJoinStatus::Approving | SpaceJoinStatus::Approved | SpaceJoinStatus::Rejected) {
                return Err("Join request is no longer awaiting this decision".into());
            }
    if record.request.status == SpaceJoinStatus::Approving && record.decisionRevision.is_none() {
        record.decisionRevision = Some(current.spaceRevision.max(record.sourceRevision).checked_add(1).ok_or("Space revision overflow")?);
        saveRecord(service, INBOUND, &decision.requestId, &record)?;
    }
    restoreClaimedMemberProfiles(service, &record, &current)?;
    Ok(Claim { record, current: PeerSpaceJoin { space: current,
        deviceProfiles: service.spaceStore().deviceProfilesForCurrentSpace()?,
        topology: service.spaceStore().topologyRecords()?.into_values().collect(),
        // A local reviewer already owns this policy and never imports its own
        // claim history. Remote reviewers still need the complete authority.
        controlOperations: if origin == service.localNodeId() { Vec::new() }
            else { service.networkControlStore().currentSpaceOperations()? }, departures: Vec::new() } })
}
/// Completes one claimed decision and returns the owned durable application record.
fn completeDecision(service: &dyn NodeSpaceContext, origin: &str, outcome: Outcome, current: CoreSpace) -> Result<Box<Record>, String> {
            let mut record = load(service, INBOUND, &outcome.decision.requestId)?;
            validateAssignment(service, &record, origin, &outcome.decision)?;
            if record.approvedDecision != Some(outcome.decision.approve) { return Err("Decision does not match its claim".into()); }
            if matches!(record.request.status, SpaceJoinStatus::Approved | SpaceJoinStatus::Rejected) {
                return Ok(record);
            }
            if record.request.status != SpaceJoinStatus::Approving { return Err("Decision was not claimed".into()); }
            if outcome.decision.approve {
                let operation = outcome.admission.as_ref().ok_or("Approval has no admission operation")?;
                let command: NetworkControlCommandRecord = NetworkControlCommandRecord::deserialize(&operation.payload).map_err(|e| e.to_string())?;
                if command.spaceId != record.targetSpaceId || command.issuerNodeId != origin
                    || !matches!(command.command, NetworkControlCommand::AdmitSpace { ref sourceSpaceId, ref identities }
                        if sourceSpaceId == &record.sourceSpaceId && identities.keys().cloned().collect::<BTreeSet<_>>() == record.source.space.members.iter().cloned().collect()) {
                    return Err("Approval operation is not this reviewer's admission for this applicant".into());
                }
                let members = record.source.space.members.iter().cloned().collect();
                let mut operations = service.networkControlStore().currentSpaceOperations()?;
                if !operations.iter().any(|existing| existing.opId == operation.opId) { operations.push(operation.clone()); }
                service.networkControlStore().validateSpaceAdmission(&record.targetSpaceId,
                    &record.sourceSpaceId, &members, &operations)?;
                // This validated log is no longer needed when publishing the
                // member projection and returning the approval facts.
                drop(operations);
                service.spaceStore().importDeviceProfiles(record.source.deviceProfiles.clone())?;
                service.spaceStore().importTopologyRecords(record.source.topology.clone())?;
                service.networkControlStore().applyBootstrapOperation(operation)?;
                let mut joined = current;
                joined.members.extend(record.source.space.members.clone());
                joined.members.sort();
                joined.members.dedup();
                joined.spaceRevision = joined.spaceRevision.max(outcome.revision).max(record.decisionRevision.ok_or("Approval has no claimed revision")?);
                service.spaceStore().adoptAt(joined, record.request.createdAt)?;
                record.request.status = SpaceJoinStatus::Approved;
            } else {
                if outcome.admission.is_some() { return Err("Rejection must not contain an admission".into()); }
                record.request.status = SpaceJoinStatus::Rejected;
            }
            saveRecord(service, INBOUND, &record.request.requestId, &record)?;
    Ok(record)
}
/// Requires current local membership before constructing a decision context.
fn localDecisionContext(service: &dyn NodeSpaceContext) -> Result<CoreSpace, String> {
    let current = service.spaceStore().initialize()?;
    let local = service.localNodeId();
    if !current.members.contains(&local) || service.networkControlStore().nodeIsDisconnected(&local)? {
        return Err("Approval requires an active member of this Space".into());
    }
    Ok(current)
}

/// Resolves one unambiguous review owner and commits its authorized decision.
pub(crate) async fn decide(service: &dyn NodeSpaceContext, id: String, version: u64, approve: bool) -> Result<SpaceJoinRequest, String> {
    let local = service.localNodeId();
    if !canReview(service, &local)? { return Err("This device cannot approve Space join requests".into()); }
    let records = store(service);
    let gateway: Option<ConsentProjection> = records.record(INBOUND, &id)?;
    let assigned: Option<ConsentProjection> = records.record(INBOX, &id)?;
    let inbox = match (gateway, assigned) {
        (Some(record), None) if record.request.targetDeviceId == local => record,
        (None, Some(record)) if record.request.targetDeviceId != local => record,
        _ => return Err("Join review must have exactly one matching gateway or assignment record".into()),
    };
    let onGateway = inbox.request.targetDeviceId == local;
    let decision = Decision { requestId: id.clone(), assignmentVersion: version, approve };
    let claim: Claim = if onGateway {
        let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        claimDecision(service, &local, decision, localDecisionContext(service)?)?
    } else {
        approvalCall(service, &inbox.request.targetDeviceId, "claim", toCoreValue(decision).map_err(|e| e.to_string())?).await?
    };
    if inbox.identity() != responseIdentity(&claim.record) || inbox.sourceRevision != claim.record.sourceRevision {
        return Err("Join response identity mismatch".into());
    }
    if matches!(claim.record.request.status, SpaceJoinStatus::Approved | SpaceJoinStatus::Rejected) { return Ok(claim.record.request); }
    // Completion needs only correlation facts; command evidence is consumed in its own phase.
    let expected = inbox.identity().map(str::to_owned);
    let expectedRevision = inbox.sourceRevision;
    drop(inbox);
    let outcome = prepareOutcome(service, &id, version, approve, onGateway, claim)?;
    let reply: JoinReply = if onGateway {
        let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let record = completeDecision(service, &local, outcome, localDecisionContext(service)?)?;
        membershipReply(service, ConsentProjection::from(record.as_ref()), &service.spaceStore().space()?)?
    } else {
        approvalCall(service, &expected[2], "complete", toCoreValue(outcome).map_err(|e| e.to_string())?).await?
    };
    if expected.each_ref().map(String::as_str) != projection::identity(&reply.request, &reply.sourceSpaceId, &reply.targetSpaceId)
        || expectedRevision != reply.sourceRevision {
        return Err("Join response identity mismatch".into());
    }
    Ok(reply.request)
}

/// Consumes command evidence synchronously so completion retains only the decision outcome.
fn prepareOutcome(service: &dyn NodeSpaceContext, id: &str, version: u64, approve: bool,
    onGateway: bool, claim: Claim) -> Result<Outcome, String> {
    let saved = store(service).record::<Outcome>(RESULTS, id)?;
    if let Some(saved) = saved {
        if saved.decision.assignmentVersion != version || saved.decision.approve != approve { return Err("Stored decision differs from the claim".into()); }
        Ok(saved)
    } else {
        let revision = claim.record.decisionRevision.ok_or("Claim has no decision revision")?;
        let admission = if approve {
            CoreSpaceStore::validateSpaceProfiles(&claim.current.space, &claim.current.deviceProfiles)?;
            service.spaceStore().importDeviceProfiles(claim.current.deviceProfiles)?;
            service.spaceStore().importTopologyRecords(claim.current.topology)?;
            if !onGateway { for operation in claim.current.controlOperations { service.networkControlStore().applyBootstrapOperation(&operation)?; } }
            let mut joined = service.spaceStore().initialize()?;
            if joined.spaceId != claim.record.targetSpaceId { return Err("Reviewer changed Space".into()); }
            for node in claim.current.space.members { if !joined.members.contains(&node) { joined.members.push(node); } }
            CoreSpaceStore::validateSpaceProfiles(&claim.record.source.space, &claim.record.source.deviceProfiles)?;
            // Select initial identities from the approved immutable profiles;
            // policy persistence and replay only see the generic assignment map.
            let profiles = &claim.record.source.deviceProfiles;
            let identities = claim.record.source.space.members.iter()
                .map(|nodeId| {
                    let profile = profiles.iter().find(|profile| &profile.nodeId == nodeId)
                        .ok_or_else(|| format!("Space snapshot is missing a member profile: {nodeId}"))?;
                    Ok((nodeId.clone(), if profile.coreVersion.is_some() { "user" } else { "edge" }.into()))
                })
                .collect::<Result<_, String>>()?;
            service.spaceStore().importDeviceProfiles(claim.record.source.deviceProfiles)?;
            service.spaceStore().importTopologyRecords(claim.record.source.topology)?;
            let operation = service.networkControlStore().admitSpaceForReview(claim.record.sourceSpaceId,
                identities, &format!("{id}-{version}"))?;
            joined.members.extend(claim.record.source.space.members);
            joined.members.sort();
            joined.members.dedup();
            joined.spaceRevision = revision;
            if !onGateway { service.spaceStore().adoptAt(joined, claim.record.request.createdAt)?; }
            Some(operation)
        } else { None };
        let outcome = Outcome { decision: Decision { requestId: id.to_owned(), assignmentVersion: version, approve }, admission, revision };
        // Local retries recover their stable admission from the shared log;
        // only remote reviewers need a durable outbound completion result.
        if !onGateway { store(service).putRecord(RESULTS, id, &outcome)?; }
        Ok(outcome)
    }
}

/// Polls status through its consent projection; immutable evidence is sent only until acknowledged.
pub(crate) async fn refresh(service: &dyn NodeSpaceContext, id: String) -> Result<SpaceJoinRequest, String> {
    let mut local = consent(service, OUTBOUND, &id)?;
    if !active(&local.request.status) { return Ok(local.request); }
    if local.cancelRequested {
        let request = cancelRemote(service, &local).await?;
        if !active(&request.status) { return Ok(request); }
        local = consent(service, OUTBOUND, &id)?;
        if !active(&local.request.status) { return Ok(local.request); }
    }
    paired(service, &local.request.targetDeviceId, crate::NodeServices::PairingDirection::Either)?;
    let (method, args) = if local.submitted {
        ("joinStatus", toCoreValue(&id).map_err(|error| error.to_string())?)
    } else {
        ("requestJoin", submissionValue(service, &id)?)
    };
    let mut remote: JoinReply = callPeerSpace(service, &local.request.targetDeviceId, method, args).await?;
    remote.validate(&local)?;
    let latest = consent(service, OUTBOUND, &id)?;
    if !active(&latest.request.status) || latest.cancelRequested || latest.request.status != local.request.status {
        return Ok(latest.request);
    }
    if remote.request.status == SpaceJoinStatus::Approved {
        let accepted = remote.accepted.as_ref().ok_or("Approved join has no membership result")?;
        Box::pin(adoptApprovedMembership(service, &local, accepted)).await?;
        remote.request.status = SpaceJoinStatus::Joined;
    }
    remote.request.canApprove = false;
    let published = publishOutgoingResponse(service, &id, &remote, false)?;
    if published.status == SpaceJoinStatus::Joined {
        if let Err(e) = service.afterJoined(local.request.targetDeviceId).await {
            operit_util::AppLogger::AppLogger::w("SpaceJoin", &format!("Join approved; post-join synchronization failed: {e}"));
        }
    }
    Ok(published)
}

/// Verifies current target authority in its own phase before adopting any projected membership.
async fn adoptApprovedMembership(service: &dyn NodeSpaceContext, local: &ConsentProjection,
    accepted: &MembershipReceipt) -> Result<(), String> {
    let current = service.spaceStore().initialize()?;
    if accepted.space.spaceId != local.targetSpaceId
        || !accepted.space.members.contains(&service.localNodeId())
        || !accepted.space.members.contains(&local.request.targetDeviceId) {
        return Err("Approved membership does not match the request".into());
    }
    if current.spaceId != local.sourceSpaceId && current.spaceId != local.targetSpaceId {
        return Err("Local Space changed while waiting; join will not be applied".into());
    }
    // Read the current target revision, not an obsolete approval snapshot.
    let snapshot: PeerSpaceSnapshot = Box::pin(joinSnapshot(service, &local.request.targetDeviceId)).await?;
    if snapshot.space.spaceId != local.targetSpaceId || !snapshot.space.members.contains(&local.request.applicantDeviceId) {
        return Err("Approved target membership changed".into());
    }
    CoreSpaceStore::validateSpaceProfiles(&snapshot.space, &snapshot.deviceProfiles)?;
    let members = local.source.space.members.iter().cloned().collect();
    service.networkControlStore().validateSpaceAdmission(&snapshot.space.spaceId,
        &local.sourceSpaceId, &members, &snapshot.controlOperations)?;
    let currentMembers = current.members.iter().collect::<BTreeSet<_>>();
    let approvedMembers = snapshot.space.members.iter().collect::<BTreeSet<_>>();
    if !currentMembers.is_subset(&approvedMembers) {
        return Err("Source Space gained members after approval; submit a new merge request".into());
    }
    service.spaceStore().importDeviceProfiles(snapshot.deviceProfiles)?;
    service.spaceStore().importTopologyRecords(snapshot.topology)?;
    for operation in snapshot.controlOperations { service.networkControlStore().applyBootstrapOperation(&operation)?; }
    service.spaceStore().adopt(snapshot.space)?;
    Ok(())
}

/// Publishes a changed status while retaining only the applicant's original immutable evidence.
fn publishOutgoingResponse(service: &dyn NodeSpaceContext, id: &str, remote: &JoinReply, cancellationAcknowledged: bool) -> Result<SpaceJoinRequest, String> {
    let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let summary = consent(service, OUTBOUND, id)?;
    remote.validate(&summary)?;
    if preserveOutgoingStatus(&summary.request.status, &remote.request.status)
        || (summary.cancelRequested && !cancellationAcknowledged) {
        return Ok(summary.request);
    }
    if summary.submitted && summary.request == remote.request
        && summary.approvedDecision == remote.approvedDecision && summary.decisionRevision == remote.decisionRevision
        && !summary.cancelRequested {
        return Ok(summary.request);
    }
    let mut current = load(service, OUTBOUND, id)?;
    current.request = remote.request.clone();
    current.approvedDecision = remote.approvedDecision;
    current.decisionRevision = remote.decisionRevision;
    current.submitted = true;
    if cancellationAcknowledged { current.cancelRequested = false; }
    saveRecord(service, OUTBOUND, id, &current)?;
    Ok(current.request)
}

/// Prevents delayed responses from rolling a durable request back to an earlier state.
fn preserveOutgoingStatus(current: &SpaceJoinStatus, incoming: &SpaceJoinStatus) -> bool {
    !active(current)
        || (*current == SpaceJoinStatus::Approving && *incoming == SpaceJoinStatus::Pending)
        || (*current == SpaceJoinStatus::Approved
            && matches!(incoming, SpaceJoinStatus::Pending | SpaceJoinStatus::Approving))
}

/// Sends cancellation against the immutable local submission and accepts only its matching reply.
async fn cancelRemote(service: &dyn NodeSpaceContext, local: &ConsentProjection) -> Result<SpaceJoinRequest, String> {
    let remote: JoinReply = callPeerSpace(service, &local.request.targetDeviceId, "cancelJoin",
        submissionValue(service, &local.request.requestId)?).await?;
    remote.validate(local)?;
    publishOutgoingResponse(service, &local.request.requestId, &remote, true)
}

/// Persists cancellation intent before sending it to the gateway.
pub(crate) async fn cancel(service: &dyn NodeSpaceContext, id: String) -> Result<SpaceJoinRequest, String> {
    let local = {
        let _lock = MUTATION.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut current = load(service, OUTBOUND, &id)?;
        if !active(&current.request.status) { return Ok(current.request); }
        // Record intent without overwriting a concurrently published decision.
        if !current.cancelRequested {
            current.cancelRequested = true;
            saveRecord(service, OUTBOUND, &id, &current)?;
        }
        ConsentProjection::from(current.as_ref())
    };
    let request = cancelRemote(service, &local).await?;
    if active(&request.status) { return refresh(service, id).await; }
    Ok(request)
}

#[cfg(test)]
mod device_space_state_tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/device_space/join_state_machine.rs"));
}

/// Collects the same versioned authority projection for every applicant and target node.
async fn joinSnapshot(service: &dyn NodeSpaceContext, node: &str) -> Result<PeerSpaceSnapshot, String> {
    use super::space_projection::{self, SnapshotHeader, SnapshotCollector};
    let response = service.callNode(node.to_owned(), CoreCallRequest::new(
        format!("node-space-projectionProtocol-{}", currentTimeMillis()), NODE_SPACE_TARGET,
        "projectionProtocol", toCoreValue(serde_json::json!({"version": space_projection::VERSION})).map_err(|e| e.to_string())?,
    )).await;
    let protocol = space_projection::decodeProtocolResponse(node, response.result)?;
    if !protocol.snapshotPaging { return Err("SPACE_PROTOCOL_INCOMPATIBLE: Joining requires authority paging".into()); }
    // A departure/control update may race a paged read. Restart only this
    // read-only export; never retry admission writes or an uncertain decision.
    // The negotiated paging protocol stays valid across snapshot revisions.
    for attempt in 0..3 {
        let result = async {
            let header: SnapshotHeader = callPeerSpace(service, node, "snapshotHeader", CoreValue::Null).await?;
            let mut collector = SnapshotCollector::new(header)?;
            loop {
                let page = callPeerSpace(service, node, "snapshotControlPage",
                    toCoreValue(collector.request()).map_err(|e| e.to_string())?).await?;
                if collector.push(page)? { break; }
            }
            let current = callPeerSpace(service, node, "snapshotHeader", CoreValue::Null).await?;
            collector.finish(current)
        }.await;
        match result {
            Err(error) if attempt < 2 && error.starts_with("SPACE_EXPORT_CHANGED:") => continue,
            result => return result,
        }
    }
    unreachable!()
}

/// Calls one explicit Space method on the directly paired peer.
async fn callPeerSpace<T: serde::de::DeserializeOwned>(service: &dyn NodeSpaceContext, node: &str, method: &str, args: CoreValue) -> Result<T, String> {
    let response = service.callNode(node.to_owned(), CoreCallRequest::new(
        format!("node-space-{method}-{}", currentTimeMillis()), NODE_SPACE_TARGET, method, args,
    )).await;
    fromCoreValue(response.result.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[cfg(test)]
mod pairing_generation_tests {
    use super::removalForgetsPairing;

    /// One removal revokes exactly the generation it was issued against: a
    /// credential stamped with a later generation — the device left, reset and
    /// paired again to re-apply — always survives, while records and removals
    /// from older cores carry no generation and keep the previous semantics.
    #[test]
    fn removal_forgets_only_the_generation_it_revokes() {
        assert!(removalForgetsPairing(Some("inc-a"), &[Some("inc-a".into())]));
        assert!(!removalForgetsPairing(Some("inc-b"), &[Some("inc-a".into())]));
        assert!(removalForgetsPairing(None, &[Some("inc-a".into())]));
        assert!(removalForgetsPairing(Some("inc-a"), &[None]));
        assert!(removalForgetsPairing(None, &[None]));
        // Several removals arrive in one batch: any revoked generation locks
        // its own pairing, and the older revocation never shadows the newer.
        assert!(removalForgetsPairing(Some("inc-b"), &[Some("inc-a".into()), Some("inc-b".into())]));
        assert!(!removalForgetsPairing(Some("inc-c"), &[Some("inc-a".into()), Some("inc-b".into())]));
    }
}

#[cfg(test)]
mod snapshot_retry_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Export {
        changes: usize,
        terminal: bool,
        reads: AtomicUsize,
        protocols: AtomicUsize,
    }
    #[async_trait::async_trait(?Send)]
    impl NodeSpaceContext for Export {
        fn storage(&self) -> Arc<dyn operit_host_api::RuntimeStorageHost> { panic!("read-only export") }
        fn spaceStore(&self) -> &CoreSpaceStore { panic!("read-only export") }
        fn networkControlStore(&self) -> &NetworkControlStore { panic!("read-only export") }
        fn peers(&self) -> Result<Arc<dyn crate::RuntimePeerService::RuntimePeerService>, String> { panic!("read-only export") }
        fn localNodeId(&self) -> String { "local".into() }
        fn nodeIsReachable(&self, _: &str) -> Result<bool, String> { panic!("read-only export") }
        async fn callNode(&self, _: String, request: CoreCallRequest) -> CoreCallResponse {
            let value = match request.methodName.as_str() {
                "projectionProtocol" => {
                    self.protocols.fetch_add(1, Ordering::SeqCst);
                    if self.terminal { return CoreCallResponse::err(request.requestId,
                        operit_link::CoreLinkError::new("PEER_PERMISSION", "denied")); }
                    serde_json::json!({"version":1,"nodeId":"peer","endpoint":true,
                        "authorityMode":"localReplay","maxMessageBytes":8192,"snapshotPaging":true})
                }
                "snapshotHeader" => {
                    let read = self.reads.fetch_add(1, Ordering::SeqCst).min(self.changes);
                    serde_json::json!({"space":{"spaceId":"space","spaceName":"Space","spaceRevision":1,"members":["peer"]},
                        "deviceProfiles":[],"topology":[],"departures":[],"controlRevision":format!("{read:064x}"),"controlCount":0})
                }
                "snapshotControlPage" => {
                    let args: serde_json::Value = fromCoreValue(request.args).unwrap();
                    serde_json::json!({"revision":args["revision"],"count":0,"operation":null})
                }
                other => panic!("snapshot retry attempted a mutation: {other}"),
            };
            CoreCallResponse::ok(request.requestId, toCoreValue(value).unwrap())
        }
    }
    fn export(changes: usize, terminal: bool) -> Export {
        Export { changes, terminal, reads: AtomicUsize::new(0), protocols: AtomicUsize::new(0) }
    }
    #[tokio::test]
    async fn changed_export_restarts_from_a_fresh_header_without_writing_admission() {
        let source = export(1, false);
        let snapshot = joinSnapshot(&source, "peer").await.unwrap();
        assert_eq!(snapshot.space.spaceId, "space");
        assert_eq!(source.reads.load(Ordering::SeqCst), 4);
        assert_eq!(source.protocols.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn export_retry_is_bounded_and_permission_errors_are_not_retried() {
        let source = export(usize::MAX, false);
        assert!(joinSnapshot(&source, "peer").await.err().unwrap().starts_with("SPACE_EXPORT_CHANGED:"));
        assert_eq!(source.reads.load(Ordering::SeqCst), 6);
        let source = export(0, true);
        assert!(joinSnapshot(&source, "peer").await.err().unwrap().contains("PEER_PERMISSION"));
        assert_eq!(source.protocols.load(Ordering::SeqCst), 1);
    }
}
