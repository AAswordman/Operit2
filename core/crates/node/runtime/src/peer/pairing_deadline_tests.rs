//! Discovered carriers may open successfully and then never answer pairing.
use super::*;
use operit_host_api::{HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost, HostRuntimeTurnFuture};
use std::sync::atomic::{AtomicBool, Ordering};

struct ElapsedScheduler;
impl HostRuntimeTaskSchedulerHost for ElapsedScheduler {
    fn monotonicTimeMillis(&self) -> HostResult<u64> { Ok(0) }
    fn scheduleHostRuntimeAsyncTask(&self, _: &str, _: HostRuntimeAsyncTask) -> HostResult<()> { unreachable!() }
    fn scheduleHostRuntimeCooperativeAsyncTask(&self, _: &str, _: HostRuntimeAsyncTask) -> HostResult<()> { unreachable!() }
    fn scheduleHostRuntimeTask(&self, _: &str, _: HostRuntimeTask) -> HostResult<()> { unreachable!() }
    fn scheduleDelayedHostRuntimeTask(&self, _: &str, _: u64, _: HostRuntimeTask) -> HostResult<()> { unreachable!() }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture { unreachable!() }
    fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture { Box::pin(async { Ok(()) }) }
}
struct SilentPeer { endpoint: PeerEndpoint, transport: PeerTransport, blockedSend: bool }
#[async_trait]
impl PeerConnection for SilentPeer {
    fn source(&self) -> &PeerEndpoint { &self.endpoint }
    fn target(&self) -> &PeerEndpoint { &self.endpoint }
    fn transport(&self) -> PeerTransport { self.transport }
    async fn send(&self, _: PeerMessage) -> Result<(), String> {
        if self.blockedSend { std::future::pending::<()>().await; }
        Ok(())
    }
    async fn receive(&self) -> Result<Option<PeerMessage>, String> { std::future::pending().await }
    async fn close(&self) {}
}
#[tokio::test]
async fn all_pairing_carriers_bound_both_send_and_response_waits() {
    let host = HostManager { hostRuntimeTaskSchedulerHost: Some(Arc::new(ElapsedScheduler)), ..HostManager::default() };
    for transport in [PeerTransport::Tcp, PeerTransport::Http, PeerTransport::WebSocket, PeerTransport::Bluetooth, PeerTransport::Serial] {
        for blockedSend in [false, true] {
            let raw: Arc<dyn PeerConnection> = Arc::new(SilentPeer {
                endpoint: PeerEndpoint { nodeId: "silent-peer".into(), address: "unused".into() }, transport, blockedSend,
            });
            let result = tokio::time::timeout(std::time::Duration::from_millis(100),
                rawCall::<_, CoreValue>(&host, &raw, "hello", &CoreValue::Null)).await
                .expect("pairing must finish when the Host deadline expires even if the carrier stays open");
            assert!(result.unwrap_err().message.contains("timed out"));
        }
    }
}
struct SilentTcp { closed: Arc<AtomicBool>, blockConnect: bool }
#[async_trait]
impl operit_host_api::TcpHost for SilentTcp {
    async fn connect(&self, _: &str) -> HostResult<Arc<dyn operit_host_api::TcpConnection>> {
        if self.blockConnect { std::future::pending::<()>().await; }
        Ok(Arc::new(SilentSocket(self.closed.clone())))
    }
    async fn bind(&self, _: &str) -> HostResult<Arc<dyn operit_host_api::TcpListener>> { unreachable!() }
}
struct SilentSocket(Arc<AtomicBool>);
#[async_trait]
impl operit_host_api::TcpConnection for SilentSocket {
    async fn write(&self, _: &[u8]) -> HostResult<()> { Ok(()) }
    async fn read(&self) -> HostResult<Option<Vec<u8>>> { std::future::pending().await }
    async fn close(&self) { self.0.store(true, Ordering::SeqCst); }
}
fn service(blockConnect: bool) -> (HostRuntimePeerService, Arc<AtomicBool>) {
    let mut service = super::single_channel_tests::serviceWithScheduler(Arc::new(ElapsedScheduler));
    let closed = Arc::new(AtomicBool::new(false));
    Arc::get_mut(&mut Arc::get_mut(&mut service.state).unwrap().host).unwrap().tcpHost = Some(Arc::new(SilentTcp { closed: closed.clone(), blockConnect }));
    (service, closed)
}
#[tokio::test]
async fn unreachable_pairing_endpoint_has_a_connection_deadline() {
    let (service, _) = service(true);
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), service.startPairing(
        PeerEndpoint { nodeId: "silent-peer".into(), address: "127.0.0.1:12345".into() }, Some(PeerTransport::Tcp),
    )).await.expect("opening the pairing carrier must also have a deadline");
    assert!(result.unwrap_err().message.contains("timed out"));
    assert!(service.state.store.records::<Pending>(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH).unwrap().is_empty());
}
#[tokio::test]
async fn stalled_start_closes_the_carrier_without_creating_pending_pairing() {
    let (service, closed) = service(false);
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), service.startPairing(
        PeerEndpoint { nodeId: "silent-peer".into(), address: "127.0.0.1:12345".into() }, Some(PeerTransport::Tcp),
    )).await.expect("silent handshake must return an actionable error");
    assert!(result.is_err());
    assert!(closed.load(Ordering::SeqCst));
    assert!(service.state.store.records::<Pending>(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH).unwrap().is_empty());
}
#[tokio::test]
async fn stalled_confirmation_closes_the_carrier_without_committing_credentials() {
    let (service, closed) = service(false);
    let pending = Pending {
        version: PAIRING_SERVICE_VERSION, id: "confirmation".into(), clientDeviceId: "self".into(), peerNodeId: "silent-peer".into(),
        endpoint: "127.0.0.1:12345".into(), transport: PeerTransport::Tcp, info: service.state.info.clone(), root: vec![7;32],
        expires: currentTimeMillis() + PAIRING_LIFETIME_MS, attempts: 0, confirmationCode: None, qrJoinConsent: None, invitationId: None,
    };
    service.state.store.putRecord(RUNTIME_LINK_ACCESS_PENDING_OUTBOUND_PAIRINGS_PATH, &pending.id, &pending).unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), service.confirmOutboundPairing(&pending.id, "123456"))
        .await.expect("confirmation must not leave the UI waiting forever");
    assert!(result.is_err());
    assert!(closed.load(Ordering::SeqCst));
    assert!(service.state.store.records::<StoredOutbound>(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH).unwrap().is_empty());
    assert_eq!(service.pending(false, &pending.id).unwrap().attempts, 1);
}
