// Test-only service injection. No socket, frames, transport registry or production fallback.
use super::*;

#[async_trait]
pub(super) trait TestRouteTarget: Send + Sync {
    fn installServices(&self, _: NodeServices) -> Result<(), String> { Ok(()) }
    async fn routedCall(&self, previousNodeId: String, request: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse;
    async fn routedWatchSnapshot(&self, previousNodeId: String, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent, CoreLinkError>;
    async fn routedWatch(&self, previousNodeId: String, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError>;
    async fn routedOpenPush(&self, previousNodeId: String, request: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;
}

pub(super) struct TestPeerService {
    localNodeId: String,
    targetNodeId: String,
    endpoint: StdMutex<Option<Arc<dyn TestRouteTarget>>>,
    changes: tokio::sync::broadcast::Sender<()>,
}
impl TestPeerService {
    pub fn new(localNodeId: String, targetNodeId: String, endpoint: Arc<dyn TestRouteTarget>) -> Arc<Self> {
        Arc::new(Self { localNodeId, targetNodeId, endpoint: StdMutex::new(Some(endpoint)), changes: tokio::sync::broadcast::channel(16).0 })
    }
    pub fn attach(&self, endpoint: Arc<dyn TestRouteTarget>) {
        *self.endpoint.lock().unwrap() = Some(endpoint);
        let _ = self.changes.send(());
    }
    pub fn close(&self) {
        self.endpoint.lock().unwrap().take();
        let _ = self.changes.send(());
    }
    fn endpoint(&self, nextNodeId: &str) -> Result<Arc<dyn TestRouteTarget>, CoreLinkError> {
        if nextNodeId != self.targetNodeId { return Err(CoreLinkError::new("TEST_PEER_NOT_FOUND", "Unexpected next hop")); }
        self.endpoint.lock().unwrap().clone().ok_or_else(|| CoreLinkError::new("TEST_PEER_CLOSED", "Test peer is disconnected"))
    }
    fn unsupported() -> CoreLinkError { CoreLinkError::new("TEST_NOT_IMPLEMENTED", "Routing fixture does not implement pairing or transport") }
}
#[async_trait(?Send)]
impl RuntimePeerService for TestPeerService {
    async fn discoverPeers(&self, _: u64) -> Result<Vec<DiscoveredPeer>, CoreLinkError> { Err(Self::unsupported()) }
    /// Rejects pairing requests outside this routing fixture contract.
    async fn startPairing(&self, _: PeerEndpoint, _: Option<PeerTransport>) -> Result<PendingPairing, CoreLinkError> { Err(Self::unsupported()) }
    async fn finishPairing(&self, _: &str, _: &str) -> Result<PairedPeer, CoreLinkError> { Err(Self::unsupported()) }
    async fn cancelPairing(&self, _: &str) -> Result<(), CoreLinkError> { Err(Self::unsupported()) }
    /// Declares that this routing fixture owns no listener or discovery Host.
    fn listenerCapabilities(&self) -> operit_peer_link::PeerListenerCapabilities {
        operit_peer_link::PeerListenerCapabilities { transports: Vec::new(), discoveryAdvertisement: false }
    }
    async fn startListening(&self, _: &[PeerTransport]) -> Result<(), CoreLinkError> { Err(Self::unsupported()) }
    async fn stop(&self) -> Result<(), CoreLinkError> { self.close(); Ok(()) }
    async fn call(&self, nextNodeId: &str, request: RoutedCoreRequest<CoreCallRequest>) -> CoreCallResponse {
        match self.endpoint(nextNodeId) {
            Ok(endpoint) => endpoint.routedCall(self.localNodeId.clone(), request).await,
            Err(error) => CoreCallResponse::err(request.payload.requestId, error),
        }
    }
    async fn watchSnapshot(&self, nextNodeId: &str, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEvent, CoreLinkError> {
        self.endpoint(nextNodeId)?.routedWatchSnapshot(self.localNodeId.clone(), request).await
    }
    async fn watch(&self, nextNodeId: &str, request: RoutedCoreRequest<CoreWatchRequest>) -> Result<CoreEventStream, CoreLinkError> {
        self.endpoint(nextNodeId)?.routedWatch(self.localNodeId.clone(), request).await
    }
    async fn openPush(&self, nextNodeId: &str, request: RoutedCoreRequest<CorePushRequest>) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.endpoint(nextNodeId)?.routedOpenPush(self.localNodeId.clone(), request).await
    }
    fn pairedPeers(&self) -> Result<Vec<PairedPeer>, CoreLinkError> {
        Ok(vec![PairedPeer { nodeId: self.targetNodeId.clone(), displayName: "Test peer".into(), inbound: true, outbound: true, pairingIncarnation: None }])
    }
    fn outboundPeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> { Ok([self.targetNodeId.clone()].into()) }
    fn activePeerNodeIds(&self) -> Result<BTreeSet<String>, CoreLinkError> {
        Ok(if self.endpoint.lock().unwrap().is_some() { [self.targetNodeId.clone()].into() } else { BTreeSet::new() })
    }
    fn subscribePeerChanges(&self) -> tokio::sync::broadcast::Receiver<()> { self.changes.subscribe() }
    fn pairingPrompts(&self) -> Result<Vec<PairingPrompt>, CoreLinkError> { Ok(Vec::new()) }
    async fn disconnectPeer(&self, peerNodeId: &str) -> Result<(), CoreLinkError> {
        if peerNodeId != self.targetNodeId { return Err(Self::unsupported()); } self.close(); Ok(())
    }
    async fn removePairedPeer(&self, _: &str) -> Result<(), CoreLinkError> { Err(Self::unsupported()) }
    fn forgetPairingRecords(&self, _: &[String]) -> Result<(), CoreLinkError> { Err(Self::unsupported()) }
}

pub(super) struct TestPeerFixture(Arc<TestPeerService>, Arc<TestPeerService>);
impl TestPeerFixture { pub fn close(&self) { self.0.close(); self.1.close(); } }
impl Drop for TestPeerFixture { fn drop(&mut self) { self.close(); } }

pub(super) fn installTestPeer(router: &CoreNodeRouter, targetNodeId: String, target: Arc<dyn TestRouteTarget>) -> Result<TestPeerFixture, String> {
    let reverse = TestPeerService::new(targetNodeId.clone(), router.localNodeId(), TestCoreNodeRouterEndpoint::new(router.clone()));
    target.installServices(NodeServices::new(reverse.clone()))?;
    let forward = TestPeerService::new(router.localNodeId(), targetNodeId, target);
    router.installNodeServices(NodeServices::new(forward.clone()))?;
    Ok(TestPeerFixture(forward, reverse))
}
