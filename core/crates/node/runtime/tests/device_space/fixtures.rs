/// Exercises approvalService through isolated runtime stores.
fn approvalService(node: &str) -> (CoreNodeRouter, RuntimeRemoteLinkService) {
    let router = testCoreNodeRouterWithoutBinding(node);
    // Every fixture device mints its pairing generation up front, exactly like
    // a real runtime identity does before it announces itself.
    let incarnation = crate::PeerStateStore::PeerStateStore::new(router.localCore.runtimeStorageHost())
        .ensurePairingIncarnation()
        .unwrap();
    router
        .spaceStore
        .writeLocalDeviceProfile(node.into(), "test".into(), "test".into(), "1".into(), Some(incarnation))
        .unwrap();
    router.networkControlStore.initializeCurrentSpace().unwrap();
    let service =
        RuntimeRemoteLinkService::newWithRouter((*router.localCore).clone(), router.clone());
    (router, service)
}

/// Exercises controlOperationsForSpace through isolated runtime stores.
fn controlOperationsForSpace(router: &CoreNodeRouter, spaceId: &str) -> Vec<operit_store::SyncOperationStore::SyncOperation> {
    router.networkControlStore.spaceOperations(spaceId).unwrap()
}

struct ApprovalMeshPeer {
    local: String,
    endpoints: StdMutex<BTreeMap<String, Arc<dyn TestRouteTarget>>>,
    active: StdMutex<BTreeSet<String>>,
    /// The peer's announced pairing generation per paired node, captured at the
    /// modeled handshake exactly like a real pairing record does.
    incarnations: StdMutex<BTreeMap<String, Option<String>>>,
    changes: tokio::sync::broadcast::Sender<()>,
    faults: StdMutex<BTreeMap<String, CallFault>>,
    calls: StdMutex<Vec<(String, String)>>,
}

impl ApprovalMeshPeer {
    /// Exercises new through the device-space contract fixture.
    fn new(local: String) -> Arc<Self> {
        Arc::new(Self {
            local,
            endpoints: StdMutex::new(BTreeMap::new()),
            active: StdMutex::new(BTreeSet::new()),
            incarnations: StdMutex::new(BTreeMap::new()),
            changes: tokio::sync::broadcast::channel(16).0,
            faults: StdMutex::new(BTreeMap::new()),
            calls: StdMutex::new(Vec::new()),
        })
    }
    /// Exercises link through the device-space contract fixture. The handshake
    /// carries the peer's current pairing generation, so a re-pairing made
    /// after a reset is stamped with the peer's next generation.
    fn link(&self, router: &CoreNodeRouter) {
        let node = router.localNodeId();
        let incarnation = crate::PeerStateStore::PeerStateStore::new(router.localCore.runtimeStorageHost())
            .pairingIncarnation()
            .unwrap();
        self.incarnations.lock().unwrap().insert(node.clone(), incarnation);
        self.endpoints.lock().unwrap().insert(
            node,
            TestCoreNodeRouterEndpoint::new(router.clone()),
        );
        self.active.lock().unwrap().insert(router.localNodeId());
    }
}

#[async_trait(?Send)]
impl RuntimePeerService for ApprovalMeshPeer {
    /// Exercises discoverPeers through the device-space contract fixture.
    async fn discoverPeers(&self, _: u64) -> Result<Vec<DiscoveredPeer>, CoreLinkError> {
        unreachable!()
    }
    /// Exercises startPairing through the device-space contract fixture.
    async fn startPairing(
        &self,
        _: PeerEndpoint,
        _: Option<PeerTransport>,
    ) -> Result<PendingPairing, CoreLinkError> {
        unreachable!()
    }
    /// Exercises finishPairing through the device-space contract fixture.
    async fn finishPairing(&self, _: &str, _: &str) -> Result<PairedPeer, CoreLinkError> {
        unreachable!()
    }
    /// Exercises cancelPairing through the device-space contract fixture.
    async fn cancelPairing(&self, _: &str) -> Result<(), CoreLinkError> {
        unreachable!()
    }
    /// Declares that this routing fixture owns no listener or discovery Host.
    fn listenerCapabilities(&self) -> operit_peer_link::PeerListenerCapabilities {
        operit_peer_link::PeerListenerCapabilities {
            transports: Vec::new(),
            discoveryAdvertisement: false,
        }
    }
    /// Exercises startListening through the device-space contract fixture.
    async fn startListening(&self, _: &[PeerTransport]) -> Result<(), CoreLinkError> {
        unreachable!()
    }
    /// Exercises stop through the device-space contract fixture.
    async fn stop(&self) -> Result<(), CoreLinkError> {
        self.active.lock().unwrap().clear();
        Ok(())
    }
    /// Injects faults at exact delivery boundaries while executing the actual receiver router.
    async fn call(
        &self,
        node: &str,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let id = request.payload.requestId.clone();
        let method = request.payload.methodName.clone();
        self.calls
            .lock()
            .unwrap()
            .push((node.into(), method.clone()));
        if !self.active.lock().unwrap().contains(node) {
            return CoreCallResponse::err(
                id,
                CoreLinkError::new("TEST_OFFLINE", "Offline mesh link"),
            );
        }
        let fault = self.faults.lock().unwrap().remove(&method);
        if matches!(fault, Some(CallFault::BeforeDelivery)) {
            return CoreCallResponse::err(
                id,
                CoreLinkError::new("TEST_BEFORE_DELIVERY", "Request not delivered"),
            );
        }
        let endpoint = self.endpoints.lock().unwrap().get(node).cloned().unwrap();
        let response = endpoint.routedCall(self.local.clone(), request).await;
        match fault {
            Some(CallFault::AfterDelivery) => CoreCallResponse::err(
                id,
                CoreLinkError::new("TEST_LOST_RESPONSE", "Receiver committed; response lost"),
            ),
            Some(CallFault::PausedResponse { arrived, release }) => {
                arrived
                    .send(())
                    .expect("test controller must observe receiver completion");
                release
                    .await
                    .expect("test controller must release the response");
                response
            }
            _ => response,
        }
    }
    /// Exercises watchSnapshot through the device-space contract fixture.
    async fn watchSnapshot(
        &self,
        _: &str,
        _: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        unreachable!()
    }
    /// Exercises watch through the device-space contract fixture.
    async fn watch(
        &self,
        _: &str,
        _: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        unreachable!()
    }
    /// Exercises openPush through the device-space contract fixture.
    async fn openPush(
        &self,
        _: &str,
        _: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        unreachable!()
    }
    /// Exercises pairedPeers through the device-space contract fixture.
    fn pairedPeers(&self) -> Result<Vec<PairedPeer>, CoreLinkError> {
        Ok(self
            .endpoints
            .lock()
            .unwrap()
            .keys()
            .map(|node| PairedPeer {
                nodeId: node.clone(),
                displayName: node.clone(),
                inbound: true,
                outbound: true,
                pairingIncarnation: self.incarnations.lock().unwrap().get(node).cloned().flatten(),
            })
            .collect())
    }
    /// Exercises outboundPeerNodeIds through the device-space contract fixture.
    fn outboundPeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> {
        Ok(self.endpoints.lock().unwrap().keys().cloned().collect())
    }
    /// Exercises activePeerNodeIds through the device-space contract fixture.
    fn activePeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> {
        Ok(self.active.lock().unwrap().clone())
    }
    /// Exercises subscribePeerChanges through the device-space contract fixture.
    fn subscribePeerChanges(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.changes.subscribe()
    }
    /// Exercises pairingPrompts through the device-space contract fixture.
    fn pairingPrompts(&self) -> Result<Vec<PairingPrompt>, CoreLinkError> {
        Ok(vec![])
    }
    /// Exercises disconnectPeer through the device-space contract fixture.
    async fn disconnectPeer(&self, node: &str) -> Result<(), CoreLinkError> {
        self.active.lock().unwrap().remove(node);
        Ok(())
    }
    /// Exercises removePairedPeer through the device-space contract fixture.
    async fn removePairedPeer(&self, node: &str) -> Result<(), CoreLinkError> {
        // Unpairing drops both the pairing record and the live link.
        self.forgetPairingRecords(&[node.to_string()])?;
        Ok(())
    }
    /// Exercises forgetPairingRecords through the device-space contract fixture.
    fn forgetPairingRecords(&self, nodeIds: &[String]) -> Result<(), CoreLinkError> {
        for node in nodeIds {
            self.endpoints.lock().unwrap().remove(node);
            self.active.lock().unwrap().remove(node);
            self.incarnations.lock().unwrap().remove(node);
        }
        Ok(())
    }
}

/// Creates an authorized fixture Space with complete profiles and no synthetic direct links.
fn mergedChainFixture(routers: &[&CoreNodeRouter]) {
    let owner = routers[0];
    for router in routers.iter().skip(1) {
        owner
            .networkControlStore
            .admitMember(router.localNodeId())
            .unwrap();
    }
    let mut space = owner.spaceStore.space().unwrap();
    space.members = routers.iter().map(|router| router.localNodeId()).collect();
    space.members.sort();
    space.spaceRevision = 20;
    let profiles = routers
        .iter()
        .flat_map(|router| router.spaceStore.deviceProfiles().unwrap().into_values())
        .collect::<Vec<_>>();
    let operations = owner.networkControlStore.currentSpaceOperations().unwrap();
    for router in routers {
        router
            .spaceStore
            .importDeviceProfiles(profiles.clone())
            .unwrap();
        for operation in &operations {
            router
                .networkControlStore
                .applyBootstrapOperation(operation)
                .unwrap();
        }
        router.spaceStore.adopt(space.clone()).unwrap();
    }
}

/// Defines one explicit transport fault; no test synthesizes a successful protocol response.
enum CallFault {
    BeforeDelivery,
    AfterDelivery,
    PausedResponse {
        arrived: tokio::sync::oneshot::Sender<()>,
        release: tokio::sync::oneshot::Receiver<()>,
    },
}

impl ApprovalMeshPeer {
    /// Arms exactly one failure for one protocol method.
    fn failNext(&self, method: &str, afterDelivery: bool) {
        let fault = if afterDelivery {
            CallFault::AfterDelivery
        } else {
            CallFault::BeforeDelivery
        };
        assert!(self
            .faults
            .lock()
            .unwrap()
            .insert(method.into(), fault)
            .is_none());
    }

    /// Pauses a real receiver response until the test explicitly releases it.
    fn pauseResponse(
        &self,
        method: &str,
    ) -> (
        tokio::sync::oneshot::Receiver<()>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (arrived, received) = tokio::sync::oneshot::channel();
        let (resume, release) = tokio::sync::oneshot::channel();
        assert!(self
            .faults
            .lock()
            .unwrap()
            .insert(
                method.into(),
                CallFault::PausedResponse { arrived, release }
            )
            .is_none());
        (received, resume)
    }
}

const OUTBOUND_RECORDS: &str = "runtime/link_access/space_merge_outbound.preferences.json";
const INBOUND_RECORDS: &str = "runtime/link_access/space_merge_inbound.preferences.json";
const REVIEW_RECORDS: &str = "runtime/link_access/space_merge_review_inbox.preferences.json";
const RESULT_RECORDS: &str = "runtime/link_access/space_merge_review_results.preferences.json";

/// Owns two independent administrators, actual facades and controllable direct transport.
struct IndependentPair {
    a: CoreNodeRouter,
    b: CoreNodeRouter,
    applicant: RuntimeRemoteLinkService,
    receiver: RuntimeRemoteLinkService,
    aPeer: Arc<ApprovalMeshPeer>,
    bPeer: Arc<ApprovalMeshPeer>,
}

impl IndependentPair {
    /// Creates independent node identities and policy stores, connected in both directions.
    fn new(prefix: &str) -> Self {
        let (a, applicant) = approvalService(&format!("{prefix}-a"));
        let (b, receiver) = approvalService(&format!("{prefix}-b"));
        let aPeer = ApprovalMeshPeer::new(a.localNodeId());
        let bPeer = ApprovalMeshPeer::new(b.localNodeId());
        aPeer.link(&b);
        bPeer.link(&a);
        a.installNodeServices(NodeServices::new(aPeer.clone()))
            .unwrap();
        b.installNodeServices(NodeServices::new(bPeer.clone()))
            .unwrap();
        Self {
            a,
            b,
            applicant,
            receiver,
            aPeer,
            bPeer,
        }
    }

    /// Submits one real cross-Space request and checks the receiver's local admin assignment.
    async fn request(&self) -> crate::RuntimeRemoteLinkService::SpaceJoinRequest {
        let request = self
            .applicant
            .requestDeviceSpaceJoin(self.b.localNodeId())
            .await
            .unwrap();
        assert_eq!(request.status, SpaceJoinStatus::Pending);
        assert_eq!(request.reviewerDeviceId, Some(self.b.localNodeId()));
        assert_eq!(request.reviewerHops, Some(1));
        request
    }

    /// Recreates the public facade over the exact persisted applicant store.
    fn restartApplicant(&self) -> RuntimeRemoteLinkService {
        RuntimeRemoteLinkService::newWithRouter((*self.a.localCore).clone(), self.a.clone())
    }
}

/// Reads protocol records through the production datastore, including its serialization.
fn protocolRecords(router: &CoreNodeRouter, path: &str) -> BTreeMap<String, serde_json::Value> {
    crate::PeerStateStore::PeerStateStore::new(router.localCore.runtimeStorageHost())
        .records(path)
        .unwrap()
}

/// Captures byte-exact Space, business, identity and pairing files while excluding request bookkeeping.
fn durableFiles(router: &CoreNodeRouter) -> BTreeMap<String, Vec<u8>> {
    let host = router.localCore.runtimeStorageHost();
    ["runtime/space/", "runtime/data/", "runtime/link_access/"]
        .into_iter()
        .flat_map(|prefix| {
            host.list(prefix)
                .unwrap()
                .into_iter()
                .filter(|entry| {
                    !entry.isDirectory
                        && ![
                            OUTBOUND_RECORDS,
                            INBOUND_RECORDS,
                            REVIEW_RECORDS,
                            RESULT_RECORDS,
                        ]
                        .contains(&entry.path.as_str())
                })
                .map(|entry| (entry.path.clone(), host.readBytes(&entry.path).unwrap()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Verifies the persisted join record agrees with the expected user-visible status.
fn assertRecordStatus(router: &CoreNodeRouter, path: &str, id: &str, status: SpaceJoinStatus) {
    let records = protocolRecords(router, path);
    assert_eq!(
        records[id]["request"]["status"],
        serde_json::to_value(status).unwrap()
    );
}

/// Replays one recorded submission without changing its request identity or captured source snapshot.
fn submissionArgs(router: &CoreNodeRouter, id: &str) -> CoreValue {
    let records = protocolRecords(router, OUTBOUND_RECORDS);
    let record = &records[id];
    operit_link::toCoreValue(serde_json::json!({
        "requestId": id, "sourceSpaceId": record["sourceSpaceId"], "sourceRevision": record["sourceRevision"],
        "targetSpaceId": record["targetSpaceId"], "profile": record["profile"], "source": record["source"],
    })).unwrap()
}

/// Sends an exact direct protocol request through the authenticated router boundary.
async fn directCommand(
    router: &CoreNodeRouter,
    target: &str,
    method: &str,
    args: CoreValue,
) -> CoreCallResponse {
    router
        .callNode(
            target.into(),
            CoreCallRequest::new(
                format!("contract-{method}"),
                crate::RuntimeRemoteLinkService::NODE_SPACE_TARGET,
                method,
                args,
            ),
        )
        .await
}

/// Sends a local reviewer command through the same approval dispatcher used in production.
async fn reviewerCommand(
    pair: &IndependentPair,
    method: &str,
    args: serde_json::Value,
) -> CoreCallResponse {
    let request = CoreCallRequest::new(
        format!("review-{method}"),
        crate::RuntimeRemoteLinkService::NODE_SPACE_APPROVAL_TARGET,
        method,
        operit_link::toCoreValue(args).unwrap(),
    );
    let requestId = request.requestId.clone();
    let result = pair
        .receiver
        .acceptSpaceApprovalCall(&pair.b.localNodeId(), request)
        .await
        .map_err(CoreLinkError::internal);
    CoreCallResponse { requestId, result }
}

/// Publishes every device's live adjacency exactly like a sync round does,
/// then carries the records the way the Space snapshot exchange does.
fn publishRevocationFixtureAdjacency(routers: &[CoreNodeRouter], peers: &[Arc<ApprovalMeshPeer>]) {
    for (router, peer) in routers.iter().zip(peers) {
        let activePeers = peer.activePeerNodeIds().unwrap();
        crate::NodeSpaceService::space_topology::publishLocalTopology(
            &router.spaceStore,
            &router.networkControlStore,
            &router.localNodeId(),
            &activePeers,
        )
        .expect("production adjacency publication must succeed");
    }
    let topology = routers
        .iter()
        .map(|router| {
            router
                .spaceStore
                .topologyRecords()
                .unwrap()
                .remove(&router.localNodeId())
                .expect("every device publishes its own adjacency")
        })
        .collect::<Vec<_>>();
    for router in routers {
        router
            .spaceStore
            .importTopologyRecords(topology.clone())
            .unwrap();
    }
}

/// Builds one authorized Space over a mesh of `nodeCount` routers with the
/// listed direct links and published adjacency.
fn revocationMesh(
    prefix: &str,
    nodeCount: usize,
    links: &[(usize, usize)],
) -> (
    Vec<CoreNodeRouter>,
    Vec<RuntimeRemoteLinkService>,
    Vec<Arc<ApprovalMeshPeer>>,
) {
    let services = (0..nodeCount)
        .map(|index| approvalService(&format!("{prefix}-{index}")))
        .collect::<Vec<_>>();
    let routers = services
        .iter()
        .map(|(router, _)| router.clone())
        .collect::<Vec<_>>();
    let access = services
        .iter()
        .map(|(_, service)| service.clone())
        .collect::<Vec<_>>();
    mergedChainFixture(&routers.iter().collect::<Vec<_>>());
    let peers = routers
        .iter()
        .map(|router| ApprovalMeshPeer::new(router.localNodeId()))
        .collect::<Vec<_>>();
    for (a, b) in links {
        peers[*a].link(&routers[*b]);
        peers[*b].link(&routers[*a]);
    }
    for (router, peer) in routers.iter().zip(&peers) {
        router
            .installNodeServices(NodeServices::new(peer.clone()))
            .unwrap();
    }
    publishRevocationFixtureAdjacency(&routers, &peers);
    (routers, access, peers)
}

/// Verifies one online removal reaches the removed device, which retires
/// itself against its own replayed log, and that completing the retreat drops
/// every pairing credential: mutual forgetting.
#[tokio::test]
async fn direct_removal_retires_the_removed_device_and_drops_every_pairing() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = revocationMesh("revoke-direct", 2, &[(0, 1)]);
    let removed = routers[1].localNodeId();

    access[0]
        .removeDeviceSpaceMember(removed.clone())
        .await
        .unwrap();

    // The remover forgets the device entirely, exactly as before this change.
    assert!(!routers[0].spaceStore.contains(removed.clone()).unwrap());
    assert!(!routers[0]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));
    assert!(!peers[0]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == removed));

    // The notice retired the removed device: it stands as its own Space and
    // keeps no membership claim on the Space it was ejected from.
    let retiredSpace = routers[1].spaceStore.space().unwrap();
    assert_eq!(retiredSpace.members, vec![removed.clone()]);
    assert!(routers[1]
        .networkControlStore
        .currentState()
        .unwrap()
        .memberNodeIds
        .contains(&removed));

    // Forgetting is immediate: the pairing credentials are gone in the same
    // step that retired the device.
    assert!(peers[1].pairedPeers().unwrap().is_empty());
    // A later retirement check is idempotent and cannot fire again.
    let retired = RuntimeRemoteLinkService::newWithRouter(
        (*routers[1].localCore).clone(),
        routers[1].clone(),
    );
    assert!(!retired.retireRemovedDevice().await.unwrap());
    assert!(peers[1].pairedPeers().unwrap().is_empty());
}

/// Verifies removing a bridge still reaches the far side: the notice travels
/// along the route captured before the removal, so the leaf learns the bridge
/// is gone instead of staying in a stale view.
#[tokio::test]
async fn bridge_removal_reaches_the_far_side_through_the_captured_route() {
    let _guard = routeTestGlobalLock().lock().await;
    installTestRuntimeScheduler();
    let (routers, access, peers) = revocationMesh("revoke-bridge", 3, &[(0, 1), (1, 2)]);
    let bridge = routers[1].localNodeId();
    let leaf = routers[2].localNodeId();

    access[0]
        .removeDeviceSpaceMember(bridge.clone())
        .await
        .unwrap();

    // The far side applied the removal: its policy no longer admits the
    // bridge, and its own member table drops the bridge with it.
    let leafState = routers[2].networkControlStore.currentState().unwrap();
    assert!(!leafState.memberNodeIds.contains(&bridge));
    assert!(leafState.memberNodeIds.contains(&leaf));
    assert!(!routers[2].spaceStore.contains(bridge.clone()).unwrap());
    assert!(!routers[2]
        .spaceStore
        .space()
        .unwrap()
        .members
        .contains(&bridge));
    // The bridge itself retired against its own replayed log.
    assert_eq!(routers[1].spaceStore.space().unwrap().members, vec![bridge.clone()]);

    // Mutual forgetting is immediate: the far side dropped the bridge's
    // pairing credentials in the same step that applied the removal.
    assert!(!peers[2]
        .pairedPeers()
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == bridge));
}
