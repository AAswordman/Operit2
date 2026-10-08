/// Exercises approvalService through isolated runtime stores.
fn approvalService(node: &str) -> (CoreNodeRouter, RuntimeRemoteLinkService) {
    let router = testCoreNodeRouterWithoutBinding(node);
    router
        .spaceStore
        .writeLocalDeviceProfile(node.into(), "test".into(), "test".into(), "1".into())
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
            changes: tokio::sync::broadcast::channel(16).0,
            faults: StdMutex::new(BTreeMap::new()),
            calls: StdMutex::new(Vec::new()),
        })
    }
    /// Exercises link through the device-space contract fixture.
    fn link(&self, router: &CoreNodeRouter) {
        self.endpoints.lock().unwrap().insert(
            router.localNodeId(),
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
        _: PeerTransport,
        _: Option<&str>,
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
    async fn removePairedPeer(&self, _: &str) -> Result<(), CoreLinkError> {
        unreachable!()
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
