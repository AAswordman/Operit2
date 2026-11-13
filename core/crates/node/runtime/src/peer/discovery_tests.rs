use super::*;
use operit_host_api::{HostResult, ServiceDiscovery::*};
use operit_host_native_common::{NativeHostRuntimeTaskSchedulerHost, NativeTcpHost};
use crate::PeerStateStore::{PeerHostConfig, PeerHostPortMode};
use std::{sync::atomic::{AtomicUsize, Ordering}, time::Duration};

#[derive(Default)]
struct Discovery {
    callback: Arc<Mutex<Option<DiscoveryCallback>>>,
    records: Mutex<Vec<DiscoveredService>>,
    scans: AtomicUsize,
    subscriptionAttempts: AtomicUsize,
    subscriptionFailures: Mutex<usize>,
    scanGate: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
}
struct Subscription(Arc<Mutex<Option<DiscoveryCallback>>>);
impl DiscoverySubscription for Subscription {}
impl Drop for Subscription {
    fn drop(&mut self) { self.0.lock().unwrap().take(); }
}
impl ServiceDiscoveryHost for Discovery {
    fn supportsAdvertisement(&self) -> bool { false }
    fn discover(&self, service: &str, _: u64) -> HostResult<Vec<DiscoveredService>> {
        assert_eq!(service, "_operit-link._tcp.local.");
        self.scans.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = self.scanGate.lock().unwrap().as_ref() {
            gate.recv_timeout(Duration::from_secs(10)).expect("blocked test scan released");
        }
        Ok(self.records.lock().unwrap().clone())
    }
    fn subscribe(&self, service: &str, callback: DiscoveryCallback) -> HostResult<Box<dyn DiscoverySubscription>> {
        assert_eq!(service, "_operit-link._tcp.local.");
        self.subscriptionAttempts.fetch_add(1, Ordering::SeqCst);
        let mut failures = self.subscriptionFailures.lock().unwrap();
        if *failures > 0 {
            *failures -= 1;
            return Err(operit_host_api::HostError::new("Discovery temporarily unavailable"));
        }
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
    server.state.store.putRecord(RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH, "saved-session", &StoredInbound { qrJoinConsent: None,
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

/// A missed announcement must recover like opening the direct-link scan, without UI interaction.
#[tokio::test]
async fn mdns_refreshes_a_changed_port_when_the_browse_notification_is_missed() {
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

    server.stop().await.unwrap();
    let _previousPort = std::net::TcpListener::bind(&first).unwrap();
    let second = listen(&server).await;
    assert_ne!(first, second);
    drop(_previousPort);
    // The Host can resolve the new port on an active query, but no callback is delivered.
    *discovery.records.lock().unwrap() = vec![advertisement("server", &second)];
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if client.hasLiveChannel("server") && client.outbound("server").unwrap().endpoint == second { break; }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await.expect("a missed announcement must not require a manual direct-link scan");
    let saved = client.outbound("server").unwrap();
    assert_eq!(saved.sessionId, original.sessionId);
    assert_eq!(saved.sessionSecret, original.sessionSecret);
    assert!(discovery.scans.load(Ordering::SeqCst) > 0);
    client.stop().await.unwrap();
    server.stop().await.unwrap();
}

/// A temporary startup failure must not permanently disable background discovery.
#[tokio::test]
async fn mdns_retries_a_failed_background_subscription() {
    let discovery = Arc::new(Discovery::default());
    *discovery.subscriptionFailures.lock().unwrap() = 1;
    let client = service("client", Some(discovery.clone()));
    let server = service("server", None);
    outbound(&client, "server");
    inbound(&server, "client", 7);
    let address = listen(&server).await;
    client.startAvailabilityWorker().unwrap();
    waitUntil(|| discovery.callback.lock().unwrap().is_some()).await;
    assert!(discovery.subscriptionAttempts.load(Ordering::SeqCst) >= 2);
    discovery.publish(advertisement("server", &address));
    waitUntil(|| client.hasLiveChannel("server")).await;
    assert_eq!(client.outbound("server").unwrap().endpoint, address);
    client.stop().await.unwrap();
    server.stop().await.unwrap();
}

/// Multiple offline peers share one refresh, rather than one scan per peer or connection tick.
#[tokio::test]
async fn mdns_refreshes_are_shared_and_rate_limited() {
    let discovery = Arc::new(Discovery::default());
    let client = service("client", Some(discovery.clone()));
    let mut second = outbound(&client, "first");
    second.sessionId = "second-session".into();
    second.peerNodeId = "second".into();
    client.state.store.putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, &second.sessionId, &second).unwrap();
    client.startAvailabilityWorker().unwrap();
    waitUntil(|| discovery.scans.load(Ordering::SeqCst) > 0).await;
    tokio::time::sleep(Duration::from_millis(3_200)).await;
    assert_eq!(discovery.scans.load(Ordering::SeqCst), 1);
    client.stop().await.unwrap();
}

/// Healthy channels and non-network pairings do not need active LAN queries.
#[tokio::test]
async fn mdns_does_not_refresh_for_a_live_peer() {
    let discovery = Arc::new(Discovery::default());
    let client = service("client", Some(discovery.clone()));
    let server = service("server", None);
    outbound(&client, "server");
    inbound(&server, "client", 7);
    let address = listen(&server).await;
    client.rememberDiscoveryRecords(&[advertisement("server", &address)]);
    let channel = client.acquireChannel("server").await.unwrap();
    client.startAvailabilityWorker().unwrap();
    waitUntil(|| discovery.callback.lock().unwrap().is_some()).await;
    tokio::time::sleep(Duration::from_millis(3_200)).await;
    assert_eq!(discovery.scans.load(Ordering::SeqCst), 0);
    assert!(!channel.isFailed());
    client.stop().await.unwrap();
    server.stop().await.unwrap();
}

/// Stopping must not wait for a blocking scan or retain a subscription retried during that scan.
#[tokio::test]
async fn mdns_stop_during_refresh_releases_the_retried_subscription() {
    let discovery = Arc::new(Discovery::default());
    *discovery.subscriptionFailures.lock().unwrap() = 1;
    let (release, gate) = std::sync::mpsc::channel();
    *discovery.scanGate.lock().unwrap() = Some(gate);
    let client = service("client", Some(discovery.clone()));
    outbound(&client, "server");
    client.startAvailabilityWorker().unwrap();
    waitUntil(|| discovery.scans.load(Ordering::SeqCst) > 0).await;
    assert!(discovery.callback.lock().unwrap().is_some());
    let stopped = tokio::time::timeout(Duration::from_secs(1), client.stop()).await;
    release.send(()).unwrap();
    stopped.expect("stop must not wait for a blocking discovery scan").unwrap();
    waitUntil(|| discovery.callback.lock().unwrap().is_none()).await;
}
