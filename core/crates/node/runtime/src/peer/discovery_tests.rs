use super::*;
use operit_host_api::{HostResult, ServiceDiscovery::*};
use operit_host_native_common::{NativeHostRuntimeTaskSchedulerHost, NativeTcpHost};
use crate::PeerStateStore::{PeerHostConfig, PeerHostPortMode};
use std::time::Duration;

#[derive(Default)]
struct Discovery {
    callback: Arc<Mutex<Option<DiscoveryCallback>>>,
}
struct Subscription(Arc<Mutex<Option<DiscoveryCallback>>>);
impl DiscoverySubscription for Subscription {}
impl Drop for Subscription {
    fn drop(&mut self) { self.0.lock().unwrap().take(); }
}
impl ServiceDiscoveryHost for Discovery {
    fn supportsAdvertisement(&self) -> bool { false }
    fn discover(&self, _: &str, _: u64) -> HostResult<Vec<DiscoveredService>> { Ok(Vec::new()) }
    fn subscribe(&self, service: &str, callback: DiscoveryCallback) -> HostResult<Box<dyn DiscoverySubscription>> {
        assert_eq!(service, "_operit-link._tcp.local.");
        *self.callback.lock().unwrap() = Some(callback);
        Ok(Box::new(Subscription(self.callback.clone())))
    }
}
impl Discovery {
    fn publish(&self, record: DiscoveredService) {
        self.callback.lock().unwrap().clone().expect("background discovery subscribed")(record);
    }
}

fn service(node: &str, discovery: Option<Arc<Discovery>>) -> HostRuntimePeerService {
    let mut service = super::single_channel_tests::serviceWithScheduler(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let state = Arc::get_mut(&mut service.state).unwrap();
    state.nodeId = node.into();
    let host = Arc::get_mut(&mut state.host).unwrap();
    host.tcpHost = Some(Arc::new(NativeTcpHost));
    host.serviceDiscoveryHost = discovery.map(|host| host as Arc<dyn ServiceDiscoveryHost>);
    service
}
fn advertisement(node: &str, address: &str) -> DiscoveredService {
    let socket: std::net::SocketAddr = address.parse().unwrap();
    DiscoveredService {
        fullName: format!("{node}._operit-link._tcp.local."), hostname: "test.local.".into(),
        addresses: vec![socket.ip()], port: socket.port(),
        properties: [("nodeId".into(), node.into()), ("displayName".into(), "Peer".into()),
            ("transports".into(), "tcp".into())].into(),
    }
}
fn outbound(client: &HostRuntimePeerService, server: &str) -> StoredOutbound {
    let record = StoredOutbound {
        endpoint: "ws://127.0.0.1:1/link".into(), sessionId: "saved-session".into(),
        deviceId: client.state.nodeId.clone(), peerNodeId: server.into(),
        peerDeviceInfo: client.state.info.clone(), pairingServiceVersion: PAIRING_SERVICE_VERSION,
        sessionSecret: BASE64.encode([7; 32]), transport: "ws".into(),
    };
    client.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, &record.sessionId, &record).unwrap();
    record
}
fn inbound(server: &HostRuntimePeerService, client: &str, secret: u8) {
    server.state.store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "saved-session", &StoredInbound {
        deviceId: client.into(), deviceInfo: server.state.info.clone(),
        pairingServiceVersion: PAIRING_SERVICE_VERSION, sessionSecret: BASE64.encode([secret; 32]),
    }).unwrap();
}
async fn listen(server: &HostRuntimePeerService) -> String {
    server.state.store.saveHostConfig(&PeerHostConfig {
        bindAddress: "127.0.0.1:0".into(), transports: vec![PeerTransport::Tcp],
        discoveryEnabled: false, portMode: PeerHostPortMode::Automatic, updatedAt: 1,
    }).unwrap();
    server.startListening(&[PeerTransport::Tcp]).await.unwrap();
    server.state.store.hostConfig().unwrap().unwrap().bindAddress
}
async fn waitUntil(mut ready: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !ready() { tokio::time::sleep(Duration::from_millis(20)).await; }
    }).await.expect("background reconnection completed");
}

/// A new random listening port reconnects without opening discovery UI or entering another code.
#[tokio::test]
async fn mdns_reconnects_and_persists_the_authenticated_random_port() {
    let discovery = Arc::new(Discovery::default());
    let client = service("client", Some(discovery.clone()));
    let server = service("server", None);
    let original = outbound(&client, "server");
    inbound(&server, "client", 7);
    let first = listen(&server).await;
    client.startAvailabilityWorker().unwrap();
    waitUntil(|| discovery.callback.lock().unwrap().is_some()).await;
    discovery.publish(advertisement("server", &first));
    waitUntil(|| client.hasLiveChannel("server")).await;
    let saved = client.outbound("server").unwrap();
    assert_eq!(saved.endpoint, first);
    assert_eq!(saved.transport, "tcp");
    assert_eq!(saved.sessionSecret, original.sessionSecret);
    assert_eq!(saved.sessionId, original.sessionId);
    assert_eq!(client.channelCount("server").await, 1);

    server.stop().await.unwrap();
    client.disconnectPeer("server").await.unwrap();
    // Occupy the previous port so the replacement listener must receive a new one.
    let _previousPort = std::net::TcpListener::bind(&first).unwrap();
    let second = listen(&server).await;
    assert_ne!(first, second);
    discovery.publish(advertisement("server", &second));
    waitUntil(|| client.hasLiveChannel("server") && client.outbound("server").unwrap().endpoint == second).await;
    let saved = client.outbound("server").unwrap();
    assert_eq!(saved.sessionId, original.sessionId);
    assert_eq!(saved.sessionSecret, original.sessionSecret);
    client.stop().await.unwrap();
    assert!(discovery.callback.lock().unwrap().is_none());
    server.stop().await.unwrap();
}

/// Matching TXT node IDs cannot redirect a pairing without proof of the stored secret.
#[tokio::test]
async fn mdns_with_the_wrong_pairing_secret_does_not_change_the_saved_endpoint() {
    let client = service("client", None);
    let impostor = service("server", None);
    let original = outbound(&client, "server");
    inbound(&impostor, "client", 8);
    let address = listen(&impostor).await;
    client.rememberDiscoveryRecords(&[advertisement("server", &address)]);
    assert!(client.acquireChannel("server").await.is_err());
    let saved = client.outbound("server").unwrap();
    assert_eq!(saved.endpoint, original.endpoint);
    assert_eq!(saved.transport, original.transport);
    assert_eq!(saved.sessionSecret, original.sessionSecret);
    assert!(!client.hasLiveChannel("server"));
    client.stop().await.unwrap();
    impostor.stop().await.unwrap();
}

/// An unreachable advertisement does not hide another usable endpoint for the same node.
#[tokio::test]
async fn mdns_falls_back_between_discovered_addresses_before_persisting() {
    let client = service("client", None);
    let server = service("server", None);
    outbound(&client, "server");
    inbound(&server, "client", 7);
    let address = listen(&server).await;
    let valid = advertisement("server", &address);
    let mut unreachable = advertisement("server", "127.0.0.1:1");
    unreachable.fullName = "other-interface".into();
    client.rememberDiscoveryRecords(&[unreachable, valid]);
    let channel = client.acquireChannel("server").await.unwrap();
    assert!(!channel.isFailed());
    assert_eq!(client.outbound("server").unwrap().endpoint, address);
    client.stop().await.unwrap();
    server.stop().await.unwrap();
}

/// A connection in flight cannot recreate a removed pairing or overwrite a replacement session.
#[test]
fn authenticated_endpoint_update_respects_pairing_removal_and_operator_edits() {
    let client = service("client", None);
    let original = outbound(&client, "server");
    client.state.store.removePairedPeer("server").unwrap();
    assert!(client.saveAuthenticatedEndpoint("server", &original, "127.0.0.1:40000", PeerTransport::Tcp).is_err());
    assert!(client.outbound("server").is_err());
    let original = outbound(&client, "server");
    let endpoint = parseOperatorEndpoint("http://manual.example:1234/link").unwrap();
    client.updateOutboundEndpointLocked("server", &endpoint, None).unwrap();
    client.saveAuthenticatedEndpoint("server", &original, "127.0.0.1:40000", PeerTransport::Tcp).unwrap();
    assert_eq!(client.outbound("server").unwrap().endpoint, endpoint.rendered);
}
