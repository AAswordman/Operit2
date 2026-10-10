//! Space admission control plane, independent of Chat/Provider/Tool execution.
//! Both Core and Edge run peer/space_join.rs; only routing and post-join work differ.
#![allow(non_snake_case)]
use operit_host_api::{RuntimeStorageHost, TimeUtils::currentTimeMillis};
use operit_link::{fromCoreValue, toCoreValue, CoreCallRequest, CoreCallResponse, CoreValue, CoreLinkError, RoutedCoreRequest, RoutedCoreRequestKind};
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceDeviceProfile, CoreSpaceStore, CoreSpaceTopologyRecord};
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_store::NetworkControlStore::NetworkControlStore;
use operit_store::SyncOperationStore::SyncOperation;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::RuntimePeerService::RuntimePeerService;

/// Runtime 的 Space 业务对象；仍使用标准 Link Call，不新增握手消息或 HTTP 路径。
pub const NODE_SPACE_TARGET: &str = "node.space";
// Same-Space, authenticated routing only; never available to an unadmitted applicant.
pub const NODE_SPACE_APPROVAL_TARGET: &str = "node.space.approval";
/// Authority exchange remains available to admitted endpoints without storage grants.
pub const NODE_SPACE_CONTROL_TARGET: &str = "node.space.control";

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PeerSpaceSnapshot {
    pub(crate) space: CoreSpace,
    pub(crate) deviceProfiles: Vec<CoreSpaceDeviceProfile>,
    pub(crate) controlOperations: Vec<SyncOperation>,
    pub(crate) topology: Vec<CoreSpaceTopologyRecord>,
}
pub(crate) type PeerSpaceJoin = PeerSpaceSnapshot;

/// Join approval is separate from pairing and from synchronized membership.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpaceJoinStatus { Pending, Approving, Approved, Rejected, Cancelled, Expired, Joined }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceJoinRequest {
    pub requestId: String,
    pub targetDeviceId: String,
    pub applicantDeviceId: String,
    pub applicantName: String,
    pub spaceName: String,
    pub status: SpaceJoinStatus,
    pub createdAt: i64,
    pub expiresAt: i64,
    pub canApprove: bool,
    #[serde(default)]
    pub reviewerDeviceId: Option<String>,
    #[serde(default)]
    pub reviewerName: Option<String>,
    #[serde(default)]
    pub reviewerHops: Option<u32>,
    #[serde(default)]
    pub assignmentVersion: u64,
    #[serde(default)]
    pub decisionApprove: Option<bool>,
}


#[path = "peer/space_join.rs"]
pub(crate) mod space_join;
#[path = "peer/space_reconcile.rs"]
pub(crate) mod space_reconcile;
#[path = "peer/space_doctor.rs"]
pub(crate) mod space_doctor;
#[path = "peer/space_topology.rs"]
pub(crate) mod space_topology;

/// Hooks provided by the existing node runtime, not a second wire protocol.
#[async_trait::async_trait(?Send)]
pub trait NodeSpaceContext: Send + Sync {
    fn storage(&self) -> Arc<dyn RuntimeStorageHost>;
    fn spaceStore(&self) -> &CoreSpaceStore;
    fn networkControlStore(&self) -> &NetworkControlStore;
    fn peers(&self) -> Result<Arc<dyn RuntimePeerService>, String>;
    fn localNodeId(&self) -> String;
    fn nodeIsReachable(&self, node: &str) -> Result<bool, String>;
    async fn callNode(&self, node: String, request: CoreCallRequest) -> CoreCallResponse;
    async fn afterJoined(&self, _peer: String) -> Result<(), String> { Ok(()) }
}

pub(crate) fn peerSpaceSnapshot(service: &dyn NodeSpaceContext) -> Result<PeerSpaceSnapshot, String> {
    let space = service.spaceStore().initialize()?;
    peerSpaceSnapshotFor(service, space)
}
pub(crate) fn peerSpaceSnapshotFor(service: &dyn NodeSpaceContext, space: CoreSpace) -> Result<PeerSpaceSnapshot, String> {
    // Presentation history is local to the store. A Space snapshot carries only
    // the members of this projection, not devices retained from earlier Spaces.
    let deviceProfiles = service.spaceStore().deviceProfiles()?.into_iter()
        .filter(|(node, _)| space.members.contains(node)).map(|(_, profile)| profile).collect();
    let topology = service.spaceStore().topologyRecords()?.into_iter()
        .filter(|(node, _)| space.members.contains(node)).map(|(_, record)| record).collect();
    let snapshot = PeerSpaceSnapshot {
        space, deviceProfiles,
        controlOperations: service.networkControlStore().currentSpaceOperations()?,
        topology,
    };
    CoreSpaceStore::validateSpaceProfiles(&snapshot.space, &snapshot.deviceProfiles)?;
    Ok(snapshot)
}

pub(crate) fn observePeerSpaceSnapshot(service: &dyn NodeSpaceContext, peerNodeId: &str, snapshot: PeerSpaceSnapshot) -> Result<CoreSpace, String> {
        CoreSpaceStore::validateSpaceProfiles(&snapshot.space, &snapshot.deviceProfiles)?;
        if !snapshot.space.members.iter().any(|node| node == peerNodeId) {
            return Err("Paired device is not present in its announced device space".into());
        }
        let local = peerSpaceSnapshot(service)?.space;
        let crossing = local.spaceId != snapshot.space.spaceId;
        if crossing {
            let localOperations = service.networkControlStore().currentSpaceOperations()?;
            let localAdmittedSource = localOperations.iter().map(|operation| {
                serde_json::from_value::<operit_store::NetworkControlStore::NetworkControlCommandRecord>(operation.payload.clone())
                    .map_err(|error| error.to_string())
            }).collect::<Result<Vec<_>, _>>()?.iter().any(|record| {
                record.spaceId == local.spaceId && matches!(&record.command,
                    operit_store::NetworkControlStore::NetworkControlCommand::AdmitSpace { sourceSpaceId, .. }
                    if sourceSpaceId == &snapshot.space.spaceId)
            });
            if localAdmittedSource {
                service.networkControlStore().validateSpaceAdmission(&local.spaceId, &snapshot.space.spaceId,
                    &snapshot.space.members.iter().cloned().collect(), &localOperations)?;
                // This endpoint has already migrated; publish its target projection to the source peer.
                service.spaceStore().importDeviceProfiles(snapshot.deviceProfiles)?;
                return Ok(local);
            }
            if !snapshot.space.members.iter().any(|node| node == &service.localNodeId()) {
                // Independent paired Spaces do not merge without a target-side admission.
                return Ok(local);
            }
            if !local.members.iter().any(|node| node == peerNodeId) {
                return Err("Space migration must arrive through an existing source Space member".into());
            }
            service.networkControlStore().validateSpaceAdmission(&snapshot.space.spaceId,
                &local.spaceId, &local.members.iter().cloned().collect(), &snapshot.controlOperations)?;
        }
        let policy = service.networkControlStore().validateSpacePolicy(&snapshot.space.spaceId, &snapshot.controlOperations)?;
        for node in &snapshot.space.members {
            if !policy.memberNodeIds.contains(node) {
                return Err(format!("Space snapshot contains an unauthorized member: {node}"));
            }
        }
        // Only non-replicating endpoints discard obsolete cached projections;
        // authorization validation above still precedes every imported fact.
        service.spaceStore().pruneNodeLocalProjection()?;
        service.networkControlStore().pruneNodeLocalProjection()?;
        // Profiles and authorization must be available before any member reference is published.
        service.spaceStore().importDeviceProfiles(snapshot.deviceProfiles)?;
        service.spaceStore().importTopologyRecords(snapshot.topology)?;
        for operation in &snapshot.controlOperations {
            service.networkControlStore().applyBootstrapOperation(operation)?;
        }
        if crossing {
            service.spaceStore().adopt(snapshot.space)
        } else {
            service.spaceStore().observePairedDeviceSpace(peerNodeId.to_string(), snapshot.space)
        }
    }

/// Lightweight assembly for devices without the application execution runtime.
pub struct NodeSpaceService {
    storage: Arc<dyn RuntimeStorageHost>,
    space: CoreSpaceStore,
    control: NetworkControlStore,
    peers: Arc<dyn RuntimePeerService>,
    localNodeId: String,
}
impl NodeSpaceService {
    pub fn new(storage: Arc<dyn RuntimeStorageHost>, peers: Arc<dyn RuntimePeerService>) -> Result<Arc<Self>, String> {
        let localNodeId = CoreNodeIdentityStore::new(storage.clone()).initialize()?.nodeId;
        Ok(Arc::new(Self {
            space: CoreSpaceStore::newNodeLocal(storage.clone()), control: NetworkControlStore::newNodeLocal(storage.clone())?.withoutCommandCache(),
            storage, peers, localNodeId,
        }))
    }
    /// Initialize the same membership and creator policy, but keep this endpoint
    /// projection node-local rather than registering a business data replica.
    pub fn initialize(&self, profile: CoreSpaceDeviceProfile) -> Result<(), String> {
        if profile.nodeId != self.localNodeId { return Err("Local profile identity mismatch".into()); }
        self.space.initialize()?;
        self.control.migrateNodeLocalProjection(self.storage.as_ref())?;
        self.space.pruneNodeLocalProjection()?;
        self.control.pruneNodeLocalProjection()?;
        let existing = self.space.deviceProfiles()?.remove(&self.localNodeId);
        if !existing.is_some_and(|old| old.displayName == profile.displayName && old.userName == profile.userName
            && old.platform == profile.platform && old.model == profile.model && old.coreVersion == profile.coreVersion) {
            self.space.importDeviceProfiles(vec![profile])?;
        }
        self.control.initializeCurrentSpace()?;
        Ok(())
    }
    pub fn acceptPeerSpaceCall(&self, peer: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
        if !self.peers.pairedPeers().map_err(|e| e.to_string())?.iter().any(|p| p.nodeId == peer && p.inbound) {
            return Err("Node Space calls require direct inbound pairing".into());
        }
        match request.methodName.as_str() {
            "snapshot" => toCoreValue(peerSpaceSnapshot(self)?).map_err(|e| e.to_string()),
            "deviceSpace" => toCoreValue(self.space.initialize()?).map_err(|e| e.to_string()),
            "observeSpaceSnapshot" => {
                let snapshot: PeerSpaceSnapshot = fromCoreValue(request.args).map_err(|e| e.to_string())?;
                toCoreValue(observePeerSpaceSnapshot(self, peer, snapshot)?).map_err(|e| e.to_string())
            }
            "observePairedDeviceSpace" => {
                let space: CoreSpace = fromCoreValue(request.args).map_err(|e| e.to_string())?;
                toCoreValue(self.space.observePairedDeviceSpace(peer.to_string(), space)?).map_err(|e| e.to_string())
            }
            "requestJoin" | "joinStatus" | "cancelJoin" => space_join::receive(self, peer, request),
            "reconcileSharedSpace" => space_reconcile::receive(self, peer, request),
            "join" => Err("SPACE_JOIN_APPROVAL_REQUIRED: Submit a join request for local approval first".into()),
            _ => Err("Unknown node Space method".into()),
        }
    }
    pub async fn acceptSpaceApprovalCall(&self, origin: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
        space_join::receiveApproval(self, origin, request)
    }
    /// Local exit preserves identity and pairing, and bootstraps a new singleton
    /// policy through the same transition used by full Core.
    pub fn leaveDeviceSpace(&self) -> Result<CoreSpace, String> { space_join::leave(self) }
    pub async fn requestDeviceSpaceJoin(&self, peer: String) -> Result<SpaceJoinRequest, String> { space_join::request(self, peer).await }
    pub async fn incomingDeviceSpaceJoins(&self) -> Result<Vec<SpaceJoinRequest>, String> {
        // Before admission this singleton has no remote review sources. Avoid
        // materializing policy trees on every idle UI tick with an empty inbox.
        if self.space.space()?.members.len() == 1 && !space_join::hasInbound(self)? { return Ok(Vec::new()); }
        space_join::incoming(self).await
    }
    pub async fn decideDeviceSpaceJoin(&self, id: String, version: u64, approve: bool) -> Result<SpaceJoinRequest, String> { space_join::decide(self, id, version, approve).await }
    pub async fn refreshDeviceSpaceJoin(&self, id: String) -> Result<SpaceJoinRequest, String> { space_join::refresh(self, id).await }
    pub async fn cancelDeviceSpaceJoin(&self, id: String) -> Result<SpaceJoinRequest, String> { space_join::cancel(self, id).await }
    pub fn spaceChannelScope(&self, peer: &str) -> Result<Option<String>, CoreLinkError> {
        let space = self.space.space().map_err(CoreLinkError::internal)?;
        let control = self.control.currentState().map_err(CoreLinkError::internal)?;
        if peer == self.localNodeId || !control.initialized || control.spaceId != space.spaceId
            || [&self.localNodeId, &peer.to_string()].iter().any(|node| !space.members.contains(node)
                || !control.memberNodeIds.contains(*node)
                || control.disconnectedNodeIds.contains(*node)) { return Ok(None); }
        Ok(Some(space.spaceId))
    }
}
#[async_trait::async_trait(?Send)]
impl NodeSpaceContext for NodeSpaceService {
    fn storage(&self) -> Arc<dyn RuntimeStorageHost> { self.storage.clone() }
    fn spaceStore(&self) -> &CoreSpaceStore { &self.space }
    fn networkControlStore(&self) -> &NetworkControlStore { &self.control }
    fn peers(&self) -> Result<Arc<dyn RuntimePeerService>, String> { Ok(self.peers.clone()) }
    fn localNodeId(&self) -> String { self.localNodeId.clone() }
    fn nodeIsReachable(&self, node: &str) -> Result<bool, String> {
        Ok(node == self.localNodeId || self.peers.activePeerNodeIds().map_err(|e| e.to_string())?.contains(node))
    }
    async fn callNode(&self, node: String, request: CoreCallRequest) -> CoreCallResponse {
        let local = self.localNodeId.clone();
        let spaceId = if request.target == NODE_SPACE_APPROVAL_TARGET || request.target == NODE_SPACE_CONTROL_TARGET {
            match self.space.space() {
                Ok(space) => space.spaceId,
                Err(error) => return CoreCallResponse::err(request.requestId, CoreLinkError::internal(error)),
            }
        } else { String::new() };
        self.peers.call(&node, RoutedCoreRequest { spaceId, originNodeId: local,
            targetNodeId: node.clone(), ttl: 0, routeKind: RoutedCoreRequestKind::Target, payload: request }).await
    }
    async fn afterJoined(&self, peer: String) -> Result<(), String> {
        // Exchange authority through the shared authenticated session engine
        // after adopting membership. The lightweight endpoint never receives
        // a business replica or advances its replication clock.
        let request = CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId("space-control"),
            NODE_SPACE_CONTROL_TARGET, "exchange",
            toCoreValue(self.control.currentSpaceOperations()?).map_err(|error| error.to_string())?,
        );
        let response = self.callNode(peer.clone(), request).await;
        let operations = decodeControlExchange(response.result)?;
        self.control.projectPeerControlOperations(&peer, &operations)?;
        Ok(())
    }
}


/// Exchange only issuer-bound authority commands. Never receives chat, files,
/// profiles or a business replication clock; policy replay remains authoritative.
pub fn acceptSpaceControlCall(service: &dyn NodeSpaceContext, origin: &str, request: CoreCallRequest) -> Result<CoreValue, String> {
    let space = service.spaceStore().space()?;
    let control = service.networkControlStore().currentState()?;
    if !space.members.iter().any(|node| node == origin)
        || !control.memberNodeIds.contains(origin)
        || control.disconnectedNodeIds.contains(origin) {
        return Err("Space control exchange requires an admitted member".into());
    }
    if request.methodName != "exchange" { return Err("Unknown Space control method".into()); }
    let operations: Vec<SyncOperation> = fromCoreValue(request.args).map_err(|error| error.to_string())?;
    service.networkControlStore().projectPeerControlOperations(origin, &operations)?;
    toCoreValue(service.networkControlStore().currentSpaceOperations()?).map_err(|error| error.to_string())
}

/// Decode the required control-plane response. Missing methods and denied calls
/// must remain visible; neither can acknowledge an authority exchange.
pub(crate) fn decodeControlExchange(result: Result<CoreValue, CoreLinkError>) -> Result<Vec<SyncOperation>, String> {
    fromCoreValue(result.map_err(|error| error.to_string())?).map_err(|error| error.to_string())
}

#[cfg(test)]
mod control_exchange_tests {
    use super::*;
    #[test]
    fn control_exchange_requires_a_valid_acknowledgement() {
        for code in ["METHOD_NOT_FOUND", "SPACE_STORAGE_PERMISSION_DENIED", "PEER_SECURITY", "INTERNAL", "SPACE_ROUTE_NOT_FOUND"] {
            assert!(decodeControlExchange(Err(CoreLinkError::new(code, "failed"))).is_err());
        }
        assert!(decodeControlExchange(Ok(CoreValue::Null)).is_err());
        assert!(decodeControlExchange(Ok(toCoreValue(Vec::<SyncOperation>::new()).unwrap())).unwrap().is_empty());
    }
}
