//! Durable application records, not storage-engine changes. A paired gateway
//! coordinates one request; only its assigned, authorized reviewer may decide.
//! Applicant traffic stays direct. Reviewer traffic uses authenticated Space routes.
use super::*;
use crate::PeerStateStore::PeerStateStore;
use operit_store::CoreSpaceStore::CoreSpaceDeviceConnection;
use operit_store::NetworkControlStore::{NetworkControlCommand, NetworkControlCommandRecord};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Mutex;

// Group consent uses its own durable schema; single-device approval records are not group approvals.
pub(crate) const INBOUND: &str = "runtime/link_access/space_merge_inbound.preferences.json";
pub(crate) const OUTBOUND: &str = "runtime/link_access/space_merge_outbound.preferences.json";
pub(crate) const INBOX: &str = "runtime/link_access/space_merge_review_inbox.preferences.json";
pub(crate) const RESULTS: &str = "runtime/link_access/space_merge_review_results.preferences.json";
const LIFETIME_MS: i64 = 15 * 60 * 1000;
const OFFLINE_GRACE_MS: i64 = 30_000;
static MUTATION: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Record {
    pub(crate) request: SpaceJoinRequest,
    pub(crate) sourceSpaceId: String,
    pub(crate) sourceRevision: i64,
    pub(crate) targetSpaceId: String,
    pub(crate) profile: CoreSpaceDeviceProfile,
    pub(crate) source: PeerSpaceSnapshot,
    // Keep the uncommon receipt out of every pending record / BTreeMap node.
    // Box is serialization-transparent and avoids oversized Xtensa node moves.
    pub(crate) accepted: Option<Box<PeerSpaceJoin>>,
    #[serde(default)]
    pub(crate) unavailableSince: Option<i64>,
    #[serde(default)]
    pub(crate) approvedDecision: Option<bool>,
    #[serde(default)]
    pub(crate) decisionRevision: Option<i64>,
    #[serde(default)]
    pub(crate) cancelRequested: bool,
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
    record: Record,
    current: PeerSpaceJoin,
}
#[derive(Serialize, Deserialize)]
struct Outcome {
    decision: Decision,
    admission: Option<SyncOperation>,
    revision: i64,
}
fn store(service: &dyn NodeSpaceContext) -> PeerStateStore {
    PeerStateStore::new(service.storage())
}
fn active(status: &SpaceJoinStatus) -> bool {
    matches!(status, SpaceJoinStatus::Pending | SpaceJoinStatus::Approving | SpaceJoinStatus::Approved)
}
fn expire(record: &mut Record, now: i64, spaceId: &str) {
    // An approved admission is already durable membership, not an expiring offer.
    // A claimed decision is pinned for retry/recovery, never assigned twice.
    if record.request.status == SpaceJoinStatus::Pending {
        if record.targetSpaceId != spaceId {
            record.request.status = SpaceJoinStatus::Cancelled;
        } else if record.request.expiresAt <= now {
            record.request.status = SpaceJoinStatus::Expired;
        }
    }
}
fn load(service: &dyn NodeSpaceContext, path: &str, id: &str) -> Result<Record, String> {
    store(service).records::<Record>(path)?.remove(id).ok_or_else(|| "Join request not found".into())
}
// Accepted control history already lives in the common synchronization log.
// Persist only the decision/membership receipt, not another complete log per
// request. Rehydrate the wire response below; old full records remain readable.
pub(crate) fn saveRecord(service: &dyn NodeSpaceContext, path: &str, id: &str, record: &Record) -> Result<(), String> {
    let durable = Record {
        request: record.request.clone(), sourceSpaceId: record.sourceSpaceId.clone(),
        sourceRevision: record.sourceRevision, targetSpaceId: record.targetSpaceId.clone(),
        profile: record.profile.clone(), source: if active(&record.request.status) { record.source.clone() } else {
            // A terminal request is an identity/membership receipt, not another
            // replica of the applicant's profiles, topology and policy log.
            PeerSpaceSnapshot { space: record.source.space.clone(), deviceProfiles: Vec::new(), topology: Vec::new(), controlOperations: Vec::new() }
        }, unavailableSince: record.unavailableSince,
        approvedDecision: record.approvedDecision, decisionRevision: record.decisionRevision,
        cancelRequested: record.cancelRequested,
        accepted: record.accepted.as_ref().map(|accepted| Box::new(PeerSpaceJoin {
            space: accepted.space.clone(), deviceProfiles: Vec::new(), topology: Vec::new(), controlOperations: Vec::new(),
        })),
    };
    store(service).putRecord(path, id, &durable)
}
fn canReview(service: &dyn NodeSpaceContext, node: &str) -> Result<bool, String> {
    Ok(!service.networkControlStore().nodeIsDisconnected(node)?
        && service.networkControlStore().nodeHasCapability(node, "network.members.join", None)?
        && service.networkControlStore().nodeHasCapability(node, "network.approval", None)?)
}
fn paired(service: &dyn NodeSpaceContext, peer: &str, inbound: bool) -> Result<(), String> {
    if !service.peers()?.pairedPeers().map_err(|e| e.to_string())?.iter()
        .any(|p| p.nodeId == peer && if inbound { p.inbound } else { p.outbound }) {
        return Err("Join request requires a current directional pairing".into());
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
    let Some(reviewer) = record.request.reviewerDeviceId.as_deref() else { return Ok(false); };
    let id = format!("control-review-{}-{}", record.request.requestId, record.request.assignmentVersion);
    let members = record.source.space.members.iter().cloned().collect::<BTreeSet<_>>();
    Ok(service.networkControlStore().currentSpaceOperations()?.into_iter().any(|operation| {
        operation.entityId == id && operation.originDeviceId == reviewer
            && NetworkControlCommandRecord::deserialize(&operation.payload).is_ok_and(|command| {
                command.spaceId == record.targetSpaceId && command.issuerNodeId == operation.originDeviceId
                    && matches!(command.command, NetworkControlCommand::AdmitSpace { sourceSpaceId, nodeIds }
                        if sourceSpaceId == record.sourceSpaceId && nodeIds == members)
            })
    }))
}
pub(crate) fn assign(service: &dyn NodeSpaceContext, record: &mut Record, now: i64) -> Result<(), String> {
    // Only requests nobody claimed and decisions whose admission never reached
    // the control log may move between reviewers. A committed admission is
    // durable membership: it stays pinned to its reviewer for retry/recovery,
    // and recoverCommittedApproval completes it without a new decision.
    let reassignable = match record.request.status {
        SpaceJoinStatus::Pending => true,
        SpaceJoinStatus::Approving => !admissionCommitted(service, record)?,
        _ => false,
    };
    if !reassignable { return Ok(()); }
    let local = service.localNodeId();
    let space = service.spaceStore().initialize()?;
    let capable = |node: &str| -> Result<bool, String> {
        Ok(space.members.iter().any(|n| n == node) && canReview(service, node)?)
    };
    if let Some(node) = record.request.reviewerDeviceId.as_deref() {
        if capable(node)? {
            if service.nodeIsReachable(node)? {
                record.unavailableSince = None;
                return Ok(()); // Stable assignment: do not bounce between equally close peers.
            }
            let since = *record.unavailableSince.get_or_insert(now);
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
    if newId != record.request.reviewerDeviceId {
        record.request.assignmentVersion = record.request.assignmentVersion.checked_add(1).ok_or("Assignment overflow")?;
    }
    record.request.reviewerName = match newId.as_ref() {
        Some(node) => service.spaceStore().deviceProfiles()?.get(node).map(|p| p.displayName.clone()).or_else(|| Some(node.clone())),
        None => None,
    };
    record.request.reviewerDeviceId = newId;
    record.request.reviewerHops = chosen.map(|(hops, _)| hops + 1);
    record.unavailableSince = None;
    Ok(())
}
/// Completes a claimed approval locally when its admission is already durable
/// policy but the reviewer crashed between applying the operation and writing
/// the result record. Membership already exists in the replayed policy; this
/// only rebuilds the projection and the applicant-facing receipt.
fn recoverCommittedApproval(service: &dyn NodeSpaceContext, record: &mut Record, current: &CoreSpace) -> Result<(), String> {
    if record.request.status != SpaceJoinStatus::Approving || record.approvedDecision != Some(true) { return Ok(()); }
    if !admissionCommitted(service, record)? { return Ok(()); }
    // Only forge the receipt when replay still admits every source member. A
    // later policy change that excluded them leaves the record pinned for the
    // reviewer's own retry; doctor reports it instead of inventing approval.
    let state = service.networkControlStore().currentState()?;
    if !record.source.space.members.iter().all(|node| state.memberNodeIds.contains(node)) { return Ok(()); }
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
    record.accepted = Some(Box::new(peerSpaceSnapshotFor(service, service.spaceStore().adoptAt(joined, record.request.createdAt)?)?));
    record.request.status = SpaceJoinStatus::Approved;
    Ok(())
}
pub(crate) fn reconcile(service: &dyn NodeSpaceContext, record: &mut Record) -> Result<(), String> {
    let now = currentTimeMillis();
    let current = service.spaceStore().initialize()?;
    expire(record, now, &current.spaceId);
    recoverCommittedApproval(service, record, &current)?;
    assign(service, record, now)
}

/// Local ownership recovery is independent of direct pairing. Serialize it
/// with approval mutations so a decision cannot restore the Space being left.
pub(crate) fn leave(service: &dyn NodeSpaceContext) -> Result<CoreSpace, String> {
    let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
    // The retired single-device approval schema has no readers or active
    // requests. Explicit local exit deletes it without parsing or migration,
    // before allocating the new Space's state on bounded storage.
    for path in [
        "runtime/link_access/space_join_inbound.preferences.json",
        "runtime/link_access/space_join_outbound.preferences.json",
        "runtime/link_access/space_join_review_inbox.preferences.json",
        "runtime/link_access/space_join_review_results.preferences.json",
    ] {
        if service.storage().exists(path).map_err(|e| e.to_string())? {
            service.storage().delete(path, false).map_err(|e| e.to_string())?;
        }
    }
    let space = service.spaceStore().leave()?;
    service.spaceStore().pruneNodeLocalProjection()?;
    service.networkControlStore().pruneNodeLocalProjection()?;
    service.networkControlStore().initializeCurrentSpace()?;
    // Old target-Space submissions cannot be replayed in the new Space. Retire
    // their local review state rather than carrying complete source snapshots
    // into every later admission. Pairing, policy audit and business data stay.
    for path in [INBOUND, INBOX] {
        for (id, record) in store(service).records::<Record>(path)? {
            if record.targetSpaceId != space.spaceId {
                store(service).deleteRecord(RESULTS, &id)?;
                store(service).deleteRecord(path, &id)?;
            }
        }
    }
    for (id, record) in store(service).records::<Record>(OUTBOUND)? {
        if !active(&record.request.status) && record.targetSpaceId != space.spaceId
            && record.sourceSpaceId != space.spaceId {
            store(service).deleteRecord(OUTBOUND, &id)?;
        }
    }
    Ok(space)
}

pub(crate) async fn request(service: &dyn NodeSpaceContext, deviceId: String) -> Result<SpaceJoinRequest, String> {
    paired(service, &deviceId, false)?;
    if let Some(old) = outgoing(service)?.into_iter().find(|r| r.targetDeviceId == deviceId && active(&r.status)) {
        return refresh(service, old.requestId).await;
    }
    let snapshot: PeerSpaceSnapshot = callPeerSpace(service, &deviceId, "snapshot", CoreValue::Null).await?;
    if !snapshot.space.members.contains(&deviceId) { return Err("Target is not in its advertised Space".into()); }
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
        let converged: PeerSpaceSnapshot = callPeerSpace(service, &deviceId, "snapshot", CoreValue::Null).await?;
        if converged.space.members.contains(&localId) {
            return Err("This device is already a member of the target Space".into());
        }
        return Err(format!(
            "Membership split with {deviceId} reconciled; the shared Space control log admits {} \
             and this device is {}. Submit a new join request from the reconciled state.",
            reconciled.members.join(", "),
            if reconciled.localIsMember { "a member" } else { "not a member" }
        ));
    }
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
        sourceSpaceId: local.spaceId, sourceRevision: local.spaceRevision,
        targetSpaceId: snapshot.space.spaceId, profile, source, accepted: None, unavailableSince: None, approvedDecision: None, decisionRevision: None, cancelRequested: false,
    };
    saveRecord(service, OUTBOUND, &record.request.requestId, &record)?;
    // refresh also handles a result already approved after a lost initial reply.
    refresh(service, record.request.requestId).await
}
async fn sendSubmission(service: &dyn NodeSpaceContext, record: &Record) -> Result<Record, String> {
    callPeerSpace(service, &record.request.targetDeviceId, "requestJoin", toCoreValue(Submission {
        requestId: record.request.requestId.clone(), sourceSpaceId: record.sourceSpaceId.clone(),
        sourceRevision: record.sourceRevision, targetSpaceId: record.targetSpaceId.clone(), profile: record.profile.clone(), source: record.source.clone(),
    }).map_err(|e| e.to_string())?).await
}
fn validateResponse(local: &Record, remote: &Record) -> Result<(), String> {
    if local.request.requestId != remote.request.requestId
        || local.request.applicantDeviceId != remote.request.applicantDeviceId
        || local.request.targetDeviceId != remote.request.targetDeviceId
        || local.targetSpaceId != remote.targetSpaceId || local.sourceSpaceId != remote.sourceSpaceId {
        return Err("Join response identity mismatch".into());
    }
    Ok(())
}
pub(crate) fn outgoing(service: &dyn NodeSpaceContext) -> Result<Vec<SpaceJoinRequest>, String> {
    let mut records: Vec<_> = store(service).records::<Record>(OUTBOUND)?.into_values().map(|r| r.request).collect();
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

    let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
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

/// Receives applicant commands; cancellation never depends on reviewer discovery.
pub(crate) fn receive(service: &dyn NodeSpaceContext, peer: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
    let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
    paired(service, peer, true)?;
    let current = service.spaceStore().initialize()?;
    let now = currentTimeMillis();
    let cancellation = request.methodName == "cancelJoin";
    let mut record = if request.methodName == "requestJoin" || cancellation {
        let input: Submission = fromCoreValue(request.args).map_err(|e| e.to_string())?;
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
        // A merge whose source equals the gateway's own Space can never pass
        // admission validation; rejecting it here keeps it out of the durable
        // review state entirely. Cancellation of such a legacy record stays allowed.
        if !cancellation && input.sourceSpaceId == current.spaceId {
            return Err("Join request must merge a distinct source Space, not the target itself".into());
        }
        let mut records = store(service).records::<Record>(INBOUND)?;
        if let Some(old) = records.get(&input.requestId) {
            if old.request.applicantDeviceId != peer || old.targetSpaceId != input.targetSpaceId
                || old.sourceSpaceId != input.sourceSpaceId || old.sourceRevision != input.sourceRevision {
                return Err("Join request id belongs to another submission".into());
            }
            old.clone()
        } else {
            // A submission from a different source Space means the applicant
            // left and forked a new one; its previous application or admission
            // no longer describes this device. Retire it so the fresh
            // submission starts a real review instead of being blocked
            // forever. Resubmissions from the same source Space stay rejected.
            if !cancellation {
                for (id, record) in records.iter_mut() {
                    if record.request.applicantDeviceId == peer && record.targetSpaceId == current.spaceId
                        && active(&record.request.status) && record.sourceSpaceId != input.sourceSpaceId {
                        record.request.status = SpaceJoinStatus::Cancelled;
                        saveRecord(service, INBOUND, id, record)?;
                    }
                }
            }
            if !cancellation && records.values().filter(|r| active(&r.request.status) && r.request.expiresAt > now).count() >= 64 {
                return Err("Too many pending join requests".into());
            }
            if !cancellation && records.values().any(|r| r.request.applicantDeviceId == peer && r.targetSpaceId == current.spaceId
                && active(&r.request.status) && (r.request.status != SpaceJoinStatus::Pending || r.request.expiresAt > now)) {
                return Err("An active join request already exists".into());
            }
            Record {
                request: SpaceJoinRequest { requestId: input.requestId, targetDeviceId: service.localNodeId(),
                    applicantDeviceId: peer.into(), applicantName: input.source.deviceProfiles.iter()
                        .filter(|p| input.source.space.members.iter().any(|node| node == &p.nodeId))
                        .map(|p| p.displayName.clone()).collect::<Vec<_>>().join(", "),
                    spaceName: current.spaceName.clone(), status: SpaceJoinStatus::Pending,
                    createdAt: now, expiresAt: now + LIFETIME_MS, canApprove: false,
                    reviewerDeviceId: None, reviewerName: None, reviewerHops: None, assignmentVersion: 0, decisionApprove: None },
                sourceSpaceId: input.sourceSpaceId, sourceRevision: input.sourceRevision,
                targetSpaceId: input.targetSpaceId, profile: input.profile, source: input.source, accepted: None,
                unavailableSince: None, approvedDecision: None, decisionRevision: None, cancelRequested: false,
            }
        }
    } else {
        let id: String = fromCoreValue(request.args).map_err(|e| e.to_string())?;
        let record = load(service, INBOUND, &id)?;
        if record.request.applicantDeviceId != peer { return Err("Join request belongs to another device".into()); }
        record
    };
    if request.methodName == "cancelJoin" {
        if record.request.status == SpaceJoinStatus::Pending {
            record.request.status = SpaceJoinStatus::Cancelled;
        }
    } else {
        reconcile(service, &mut record)?;
    }
    saveRecord(service, INBOUND, &record.request.requestId, &record)?;
    if let Some(accepted) = &mut record.accepted {
        if accepted.space.spaceId != current.spaceId { return Err("Approved Space is no longer hosted here".into()); }
        accepted.controlOperations = service.networkControlStore().currentSpaceOperations()?;
    }
    toCoreValue(record).map_err(|e| e.to_string())
}

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
    Ok(!store(service).records::<Record>(INBOUND)?.is_empty())
}
pub(crate) async fn incoming(service: &dyn NodeSpaceContext) -> Result<Vec<SpaceJoinRequest>, String> {
    let local = service.localNodeId();
    if !canReview(service, &local)? { return Ok(Vec::new()); }
    let mut result = Vec::new();
    for node in service.spaceStore().initialize()?.members {
        if !service.nodeIsReachable(&node)? { continue; }
        let fetched = if node == local {
            let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
            assignedRequests(service, &local, &localDecisionContext(service)?)
        } else { approvalCall::<Vec<Record>>(service, &node, "assigned", CoreValue::Null).await };
        let records = match fetched {
            Ok(records) => records,
            Err(error) => {
                operit_util::AppLogger::AppLogger::d("SpaceJoin", &format!("Assignment source unavailable {node}: {error}"));
                continue;
            }
        };
        for mut record in records {
            if record.request.targetDeviceId != node || record.request.reviewerDeviceId.as_deref() != Some(local.as_str())
                || record.targetSpaceId != service.spaceStore().initialize()?.spaceId { continue; }
            record.request.canApprove = true;
            if record.request.targetDeviceId != local {
                saveRecord(service, INBOX, &record.request.requestId, &record)?;
            }
            result.push(record.request);
        }
    }
    result.sort_by_key(|r| r.createdAt);
    Ok(result)
}
fn validateAssignment(service: &dyn NodeSpaceContext, record: &Record, origin: &str, decision: &Decision) -> Result<(), String> {
    if record.request.reviewerDeviceId.as_deref() != Some(origin)
        || record.request.assignmentVersion != decision.assignmentVersion
        || record.targetSpaceId != service.spaceStore().initialize()?.spaceId
        || !canReview(service, origin)? {
        return Err("Join assignment changed or this device has no approval permission".into());
    }
    Ok(())
}
fn assignedRequests(service: &dyn NodeSpaceContext, origin: &str, current: &CoreSpace) -> Result<Vec<Record>, String> {
            let mut assigned = Vec::new();
            for mut record in store(service).records::<Record>(INBOUND)?.into_values() {
                reconcile(service, &mut record)?;
                saveRecord(service, INBOUND, &record.request.requestId, &record)?;
                if record.targetSpaceId == current.spaceId && record.request.reviewerDeviceId.as_deref() == Some(origin)
                    && matches!(record.request.status, SpaceJoinStatus::Pending | SpaceJoinStatus::Approving)
                    && canReview(service, origin)? { assigned.push(record); }
            }
    Ok(assigned)
}
pub(crate) fn receiveApproval(service: &dyn NodeSpaceContext, origin: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
    let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
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
            toCoreValue(completeDecision(service, origin, outcome, current)?).map_err(|e| e.to_string())
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

// Typed local calls and wire calls share the exact same transitions.
fn claimDecision(service: &dyn NodeSpaceContext, origin: &str, decision: Decision, current: CoreSpace) -> Result<Claim, String> {
            let mut record = load(service, INBOUND, &decision.requestId)?;
            reconcile(service, &mut record)?;
            // Persist reassignment/expiry even when a stale reviewer tries to claim.
            saveRecord(service, INBOUND, &decision.requestId, &record)?;
            validateAssignment(service, &record, origin, &decision)?;
            if record.request.status == SpaceJoinStatus::Pending {
                paired(service, &record.request.applicantDeviceId, true)?;
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
        controlOperations: service.networkControlStore().currentSpaceOperations()? } })
}
fn completeDecision(service: &dyn NodeSpaceContext, origin: &str, outcome: Outcome, current: CoreSpace) -> Result<Record, String> {
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
                    || !matches!(command.command, NetworkControlCommand::AdmitSpace { ref sourceSpaceId, ref nodeIds }
                        if sourceSpaceId == &record.sourceSpaceId && nodeIds == &record.source.space.members.iter().cloned().collect()) {
                    return Err("Approval operation is not this reviewer's admission for this applicant".into());
                }
                let members = record.source.space.members.iter().cloned().collect();
                let mut operations = service.networkControlStore().currentSpaceOperations()?;
                if !operations.iter().any(|existing| existing.opId == operation.opId) { operations.push(operation.clone()); }
                service.networkControlStore().validateSpaceAdmission(&record.targetSpaceId,
                    &record.sourceSpaceId, &members, &operations)?;
                service.spaceStore().importDeviceProfiles(record.source.deviceProfiles.clone())?;
                service.spaceStore().importTopologyRecords(record.source.topology.clone())?;
                service.networkControlStore().applyBootstrapOperation(operation)?;
                let mut joined = current;
                joined.members.extend(record.source.space.members.clone());
                joined.members.sort();
                joined.members.dedup();
                joined.spaceRevision = joined.spaceRevision.max(outcome.revision).max(record.decisionRevision.ok_or("Approval has no claimed revision")?);
                record.accepted = Some(Box::new(peerSpaceSnapshotFor(service, service.spaceStore().adoptAt(joined, record.request.createdAt)?)?));
                record.request.status = SpaceJoinStatus::Approved;
            } else {
                if outcome.admission.is_some() { return Err("Rejection must not contain an admission".into()); }
                record.request.status = SpaceJoinStatus::Rejected;
            }
            saveRecord(service, INBOUND, &record.request.requestId, &record)?;
    Ok(record)
}
fn localDecisionContext(service: &dyn NodeSpaceContext) -> Result<CoreSpace, String> {
    let current = service.spaceStore().initialize()?;
    let local = service.localNodeId();
    if !current.members.contains(&local) || service.networkControlStore().nodeIsDisconnected(&local)? {
        return Err("Approval requires an active member of this Space".into());
    }
    Ok(current)
}

pub(crate) async fn decide(service: &dyn NodeSpaceContext, id: String, version: u64, approve: bool) -> Result<SpaceJoinRequest, String> {
    let local = service.localNodeId();
    if !canReview(service, &local)? { return Err("This device cannot approve Space join requests".into()); }
    let inbox = match load(service, INBOUND, &id) {
        Ok(record) if record.request.targetDeviceId == local => record,
        _ => load(service, INBOX, &id)?,
    };
    let onGateway = inbox.request.targetDeviceId == local;
    let decision = Decision { requestId: id.clone(), assignmentVersion: version, approve };
    let claim: Claim = if onGateway {
        let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
        claimDecision(service, &local, decision, localDecisionContext(service)?)?
    } else {
        approvalCall(service, &inbox.request.targetDeviceId, "claim", toCoreValue(decision).map_err(|e| e.to_string())?).await?
    };
    validateResponse(&inbox, &claim.record)?;
    if matches!(claim.record.request.status, SpaceJoinStatus::Approved | SpaceJoinStatus::Rejected) { return Ok(claim.record.request); }
    let saved = store(service).records::<Outcome>(RESULTS)?.remove(&id);
    let outcome = if let Some(saved) = saved {
        if saved.decision.assignmentVersion != version || saved.decision.approve != approve { return Err("Stored decision differs from the claim".into()); }
        saved
    } else {
        let revision = claim.record.decisionRevision.unwrap_or(claim.current.space.spaceRevision);
        let admission = if approve {
            CoreSpaceStore::validateSpaceProfiles(&claim.current.space, &claim.current.deviceProfiles)?;
            service.spaceStore().importDeviceProfiles(claim.current.deviceProfiles.clone())?;
            service.spaceStore().importTopologyRecords(claim.current.topology.clone())?;
            if !onGateway { for operation in &claim.current.controlOperations { service.networkControlStore().applyBootstrapOperation(operation)?; } }
            let mut joined = service.spaceStore().initialize()?;
            if joined.spaceId != claim.record.targetSpaceId { return Err("Reviewer changed Space".into()); }
            for node in claim.current.space.members { if !joined.members.contains(&node) { joined.members.push(node); } }
            CoreSpaceStore::validateSpaceProfiles(&claim.record.source.space, &claim.record.source.deviceProfiles)?;
            service.spaceStore().importDeviceProfiles(claim.record.source.deviceProfiles.clone())?;
            service.spaceStore().importTopologyRecords(claim.record.source.topology.clone())?;
            let operation = service.networkControlStore().admitSpaceForReview(claim.record.sourceSpaceId.clone(),
                claim.record.source.space.members.iter().cloned().collect(), &format!("{id}-{version}"))?;
            joined.members.extend(claim.record.source.space.members.clone());
            joined.members.sort();
            joined.members.dedup();
            joined.spaceRevision = joined.spaceRevision.max(revision);
            joined.spaceRevision = revision;
            if !onGateway { service.spaceStore().adoptAt(joined, claim.record.request.createdAt)?; }
            Some(operation)
        } else { None };
        let outcome = Outcome { decision: Decision { requestId: id.clone(), assignmentVersion: version, approve }, admission, revision };
        // Local retries recover their stable admission from the shared log;
        // only remote reviewers need a durable outbound completion result.
        if !onGateway { store(service).putRecord(RESULTS, &id, &outcome)?; }
        outcome
    };
    let record: Record = if onGateway {
        let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
        completeDecision(service, &local, outcome, localDecisionContext(service)?)?
    } else {
        approvalCall(service, &inbox.request.targetDeviceId, "complete", toCoreValue(outcome).map_err(|e| e.to_string())?).await?
    };
    validateResponse(&inbox, &record)?;
    // Terminal gateway state wins subsequent claims; retain remote retry results.
    // Retain the completed claim outcome for duplicate decisions and restart recovery.
    Ok(record.request)
}

pub(crate) async fn refresh(service: &dyn NodeSpaceContext, id: String) -> Result<SpaceJoinRequest, String> {
    let mut local = load(service, OUTBOUND, &id)?;
    if !active(&local.request.status) { return Ok(local.request); }
    if local.cancelRequested {
        let request = cancelRemote(service, local).await?;
        if !active(&request.status) { return Ok(request); }
        local = load(service, OUTBOUND, &id)?;
        if !active(&local.request.status) { return Ok(local.request); }
    }
    paired(service, &local.request.targetDeviceId, false)?;
    let mut remote = sendSubmission(service, &local).await?;
    validateResponse(&local, &remote)?;
    // A user cancellation may have committed while this network request was in
    // flight (the background monitor and the dialog can call refresh together).
    // Never let that stale reply resurrect the request and restart retries.
    let latest = load(service, OUTBOUND, &id)?;
    if !active(&latest.request.status) || latest.cancelRequested || latest.request.status != local.request.status {
        return Ok(latest.request);
    }
    if remote.request.status == SpaceJoinStatus::Approved {
        let current = service.spaceStore().initialize()?;
        let accepted = remote.accepted.as_ref().ok_or("Approved join has no membership result")?;
        if accepted.space.spaceId != local.targetSpaceId
            || !accepted.space.members.contains(&service.localNodeId())
            || !accepted.space.members.contains(&local.request.targetDeviceId) {
            return Err("Approved membership does not match the request".into());
        }
        if current.spaceId != local.sourceSpaceId && current.spaceId != local.targetSpaceId {
            return Err("Local Space changed while waiting; join will not be applied".into());
        }
        // Read the current target revision, not an obsolete approval snapshot.
        let snapshot: PeerSpaceSnapshot = callPeerSpace(service, &local.request.targetDeviceId, "snapshot", CoreValue::Null).await?;
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
        for operation in &snapshot.controlOperations { service.networkControlStore().applyBootstrapOperation(operation)?; }
        service.spaceStore().adopt(snapshot.space)?;
        remote.request.status = SpaceJoinStatus::Joined;
    }
    remote.request.canApprove = false;
    let published = publishOutgoingResponse(service, &id, &remote, false)?;
    if published.status == SpaceJoinStatus::Joined {
        if let Err(e) = service.afterJoined(local.request.targetDeviceId).await {
            operit_util::AppLogger::AppLogger::w("SpaceJoin", &format!("Join approved; initial data sync failed: {e}"));
        }
    }
    Ok(published)
}

/// Publishes a response without allowing an older poll to resurrect a terminal request.
fn publishOutgoingResponse(service: &dyn NodeSpaceContext, id: &str, remote: &Record, cancellationAcknowledged: bool) -> Result<SpaceJoinRequest, String> {
    let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
    let current = load(service, OUTBOUND, id)?;
    if preserveOutgoingStatus(&current.request.status, &remote.request.status)
        || (current.cancelRequested && !cancellationAcknowledged) {
        return Ok(current.request);
    }
    // Only a cancellation receipt can clear durable intent; a concurrent poll cannot.
    saveRecord(service, OUTBOUND, id, remote)?;
    Ok(remote.request.clone())
}

/// Prevents delayed responses from rolling a durable request back to an earlier state.
fn preserveOutgoingStatus(current: &SpaceJoinStatus, incoming: &SpaceJoinStatus) -> bool {
    !active(current)
        || (*current == SpaceJoinStatus::Approving && *incoming == SpaceJoinStatus::Pending)
        || (*current == SpaceJoinStatus::Approved
            && matches!(incoming, SpaceJoinStatus::Pending | SpaceJoinStatus::Approving))
}

async fn cancelRemote(service: &dyn NodeSpaceContext, local: Record) -> Result<SpaceJoinRequest, String> {
    let mut remote: Record = callPeerSpace(service, &local.request.targetDeviceId, "cancelJoin", toCoreValue(Submission {
        requestId: local.request.requestId.clone(), sourceSpaceId: local.sourceSpaceId.clone(),
        sourceRevision: local.sourceRevision, targetSpaceId: local.targetSpaceId.clone(),
        profile: local.profile.clone(), source: local.source.clone(),
    }).map_err(|e| e.to_string())?).await?;
    validateResponse(&local, &remote)?;
    remote.cancelRequested = false;
    publishOutgoingResponse(service, &local.request.requestId, &remote, true)
}

pub(crate) async fn cancel(service: &dyn NodeSpaceContext, id: String) -> Result<SpaceJoinRequest, String> {
    let local = {
        let _lock = MUTATION.lock().map_err(|e| e.to_string())?;
        let mut current = load(service, OUTBOUND, &id)?;
        if !active(&current.request.status) { return Ok(current.request); }
        // Record intent without overwriting a concurrently published decision.
        if !current.cancelRequested {
            current.cancelRequested = true;
            saveRecord(service, OUTBOUND, &id, &current)?;
        }
        current
    };
    let request = cancelRemote(service, local).await?;
    if active(&request.status) { return refresh(service, id).await; }
    Ok(request)
}

#[cfg(test)]
mod device_space_state_tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/device_space/join_state_machine.rs"));
}

async fn callPeerSpace<T: serde::de::DeserializeOwned>(service: &dyn NodeSpaceContext, node: &str, method: &str, args: CoreValue) -> Result<T, String> {
    let response = service.callNode(node.to_owned(), CoreCallRequest::new(
        format!("node-space-{method}-{}", currentTimeMillis()), NODE_SPACE_TARGET, method, args,
    )).await;
    fromCoreValue(response.result.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
