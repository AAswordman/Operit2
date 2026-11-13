use super::*;
use crate::PeerStateStore::{PeerDiscoveryConfig, PeerHostConfig, PeerHostPortMode};
use operit_host_api::{HostResult, ServiceDiscovery::*};
use operit_host_native_common::{
    NativeHostRuntimeTaskSchedulerHost, NativeHttpHost, NativeHttpServerHost, NativeTcpHost,
};

const GROUP: &str = "test-discovery-group-key";
struct NoMulticast;
impl ServiceDiscoveryHost for NoMulticast {
    fn supportsAdvertisement(&self) -> bool {
        false
    }
    fn localAddresses(&self) -> HostResult<Vec<IpAddr>> {
        Ok(vec!["192.0.2.10".parse().unwrap()])
    }
    fn subscribe(&self, _: &str, _: DiscoveryCallback) -> HostResult<Box<dyn DiscoverySubscription>> {
        Err(operit_host_api::HostError::new("Multicast unavailable"))
    }
    fn discover(&self, _: &str, _: u64) -> HostResult<Vec<DiscoveredService>> {
        Err(operit_host_api::HostError::new("Multicast unavailable"))
    }
}
fn service(node: &str) -> HostRuntimePeerService {
    let mut service =
        super::super::single_channel_tests::serviceWithScheduler(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let state = Arc::get_mut(&mut service.state).unwrap();
    state.nodeId = node.into();
    let host = Arc::get_mut(&mut state.host).unwrap();
    host.httpHost = Some(Arc::new(NativeHttpHost::new()));
    host.httpServerHost = Some(Arc::new(NativeHttpServerHost));
    host.tcpHost = Some(Arc::new(NativeTcpHost));
    host.serviceDiscoveryHost = Some(Arc::new(NoMulticast));
    state
        .store
        .saveHostConfig(&PeerHostConfig {
            discovery: None,
            bindAddress: "0.0.0.0:0".into(),
            transports: vec![PeerTransport::Tcp],
            discoveryEnabled: true,
            portMode: PeerHostPortMode::Automatic,
            updatedAt: 1,
        })
        .unwrap();
    service
}
fn announcement(node: &str) -> Announcement {
    Announcement {
        version: 1,
        nodeId: node.into(),
        displayName: "Test peer".into(),
        endpoints: vec!["tcp://192.0.2.20:45000".into()],
        ttlMs: LEASE_MS,
    }
}
async fn directory() -> (HostRuntimePeerService, String, u16) {
    let service = service("directory");
    // Reserve an available test port; production uses its configured stable port.
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    saveConfig(
        &service,
        PeerDiscoveryConfig {
            groupKey: GROUP.into(),
            listenPort: port,
            ..Default::default()
        },
    );
    drop(reservation);
    service.startListening(&[PeerTransport::Tcp]).await.unwrap();
    assert!(service.state.discovery.directory.worker.lock().unwrap().is_some());
    (service, format!("http://127.0.0.1:{port}"), port)
}
fn saveConfig(service: &HostRuntimePeerService, discovery: PeerDiscoveryConfig) {
    let mut config = service.state.store.hostConfig().unwrap().unwrap();
    config.discovery = Some(discovery);
    service.state.store.saveHostConfig(&config).unwrap();
}
fn configure(client: &HostRuntimePeerService, url: &str, key: &str) {
    saveConfig(
        client,
        PeerDiscoveryConfig {
            directoryUrl: url.into(),
            groupKey: key.into(),
            ..Default::default()
        },
    );
}

#[test]
fn discovery_leases_expire_and_never_extend_a_forwarded_lease() {
    let mut leases = Registrations::default();
    let mut record = announcement("peer");
    record.ttlMs = u64::MAX;
    leases.put(100, record).unwrap();
    let forwarded = leases.snapshot(200).pop().unwrap();
    assert_eq!(forwarded.ttlMs, LEASE_MS - 100);
    let mut receiver = Registrations::default();
    receiver.put(200, forwarded).unwrap();
    assert!(receiver.snapshot(100 + LEASE_MS).is_empty());
    assert!(leases.snapshot(100 + LEASE_MS).is_empty());
}

#[test]
fn discovery_rejects_invalid_announcements_and_bounds_the_directory() {
    let mut leases = Registrations::default();
    for ip in ["0.0.0.0", "127.0.0.1", "224.0.0.1", "169.254.1.1", "::1", "fe80::1"] {
        let mut record = announcement("invalid");
        record.endpoints = vec![format!(
            "tcp://{}:45000",
            if ip.contains(':') { format!("[{ip}]") } else { ip.into() }
        )];
        assert!(leases.put(0, record).is_err(), "{ip}");
    }
    for endpoint in ["https://192.0.2.20:443", "wss://192.0.2.20:443", "tcp://example.com:45000"] {
        let mut record = announcement("invalid");
        record.endpoints = vec![endpoint.into()];
        assert!(leases.put(0, record).is_err());
    }
    for id in 0..MAX_PEERS - 1 {
        leases.put(0, announcement(&id.to_string())).unwrap();
    }
    assert!(leases.put(1, announcement("overflow")).is_err());
    assert!(leases.put(1, announcement("0")).is_ok());
    assert!(leases.put(LEASE_MS + 1, announcement("new")).is_ok());
}

#[test]
fn discovery_settings_are_saved_atomically_with_listener_settings() {
    let client = service("client");
    let store = &client.state.store;
    assert!(store.hostConfig().unwrap().unwrap().discovery.is_none());
    configure(&client, "https://discovery.example", GROUP);
    let original = store.hostConfig().unwrap().unwrap();
    for url in [
        "http://user:secret@example.com",
        "http://example.com/path",
        "ftp://example.com",
        "http://example.com?key=x",
    ] {
        let mut invalid = original.clone();
        invalid.bindAddress = "127.0.0.1:9999".into();
        invalid.discovery.as_mut().unwrap().directoryUrl = url.into();
        assert!(store.saveHostConfig(&invalid).is_err());
        let saved = store.hostConfig().unwrap().unwrap();
        assert_eq!(saved.bindAddress, original.bindAddress);
        assert_eq!(saved.discovery, original.discovery);
    }
    let mut disabled = original.clone();
    disabled.discovery = None;
    store.saveHostConfig(&disabled).unwrap();
    assert!(store.hostConfig().unwrap().unwrap().discovery.is_none());
}

#[tokio::test]
async fn directory_discovery_works_without_multicast_and_hides_paired_peers() {
    let (directory, url, _) = directory().await;
    let publisher = service("publisher");
    let client = service("client");
    let records = publisher
        .directoryHttp(
            format!("{url}{PATH}"),
            GROUP,
            serde_json::to_vec(&Some(announcement("publisher"))).unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(records.len(), 2);
    assert!(directory
        .discoverPeers(2000)
        .await
        .unwrap()
        .iter()
        .any(|peer| peer.nodeId == "publisher"));
    configure(&client, &url, GROUP);
    let peers = client.discoverPeers(2000).await.unwrap();
    assert!(peers.iter().any(|peer| peer.nodeId == "publisher"));
    assert!(peers.iter().any(|peer| peer.nodeId == "directory"));
    assert!(!client.discoveryRecords().is_empty());
    let record = StoredOutbound {
        endpoint: "192.0.2.20:1".into(),
        sessionId: "session".into(),
        deviceId: "client".into(),
        peerNodeId: "publisher".into(),
        peerDeviceInfo: client.state.info.clone(),
        pairingServiceVersion: PAIRING_SERVICE_VERSION,
        sessionSecret: BASE64.encode([7; 32]),
        transport: "tcp".into(),
    };
    client
        .state
        .store
        .putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "session", &record)
        .unwrap();
    assert!(client
        .discoverPeers(2000)
        .await
        .unwrap()
        .iter()
        .all(|peer| peer.nodeId != "publisher"));
    assert!(client
        .reconnectionEndpoints("publisher", &record)
        .unwrap()
        .iter()
        .any(|(address, _)| address == "192.0.2.20:45000"));
    directory.stop().await.unwrap();
}

#[tokio::test]
async fn directory_authentication_limits_registration_and_does_not_publish_client_cache() {
    let (directory, url, _) = directory().await;
    let client = service("client");
    for secret in ["", "another-discovery-key"] {
        let result = client
            .directoryHttp(format!("{url}{PATH}"), secret, b"null".to_vec())
            .await;
        assert!(result.err().unwrap().to_string().contains("401"));
    }
    assert!(directory.discoveryRecords().is_empty());
    let bad = client
        .directoryHttp(format!("{url}{PATH}"), GROUP, b"{invalid".to_vec())
        .await;
    assert!(bad.err().unwrap().to_string().contains("400"));
    let oversized = client
        .directoryHttp(format!("{url}{PATH}"), GROUP, vec![b' '; 4097])
        .await;
    assert!(oversized.err().unwrap().to_string().contains("413"));
    directory
        .rememberDirectoryRecords(&[announcement("not-registered")])
        .unwrap();
    let records = client
        .directoryHttp(format!("{url}{PATH}"), GROUP, b"null".to_vec())
        .await
        .unwrap();
    assert_eq!(records.len(), 1);
    directory.stop().await.unwrap();
}

#[tokio::test]
async fn directory_group_changes_drop_old_hints_and_listener_restarts() {
    let (directory, url, port) = directory().await;
    let client = service("client");
    configure(&client, &url, GROUP);
    client.refreshDirectory(2000).await.unwrap();
    assert!(!client.discoveryRecords().is_empty());
    configure(&client, &url, "replacement-group-key");
    assert!(client.refreshDirectory(2000).await.is_err());
    assert!(client.discoveryRecords().is_empty());
    directory.stop().await.unwrap();
    let reservation = std::net::TcpListener::bind(("0.0.0.0", port)).expect("stop releases discovery port");
    drop(reservation);
    directory.startListening(&[PeerTransport::Tcp]).await.unwrap();
    configure(&client, &url, GROUP);
    assert!(client.refreshDirectory(2000).await.is_ok());
    directory.stop().await.unwrap();
}

#[tokio::test]
async fn only_the_directory_host_opens_an_extra_listener_and_reports_port_conflicts() {
    let client = service("client");
    configure(&client, "https://discovery.example", GROUP);
    client.startListening(&[PeerTransport::Tcp]).await.unwrap();
    assert!(client.state.discovery.directory.worker.lock().unwrap().is_none());
    client.stop().await.unwrap();
    let server = service("server");
    let busy = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
    saveConfig(
        &server,
        PeerDiscoveryConfig {
            groupKey: GROUP.into(),
            listenPort: busy.local_addr().unwrap().port(),
            ..Default::default()
        },
    );
    assert!(server.startListening(&[PeerTransport::Tcp]).await.is_err());
    assert!(server.state.listeners.lock().await.is_empty());
    server.stop().await.unwrap();
}

/// A receiving node registers through the existing availability worker, without a UI scan.
#[tokio::test]
async fn directory_registration_runs_in_the_existing_background_worker() {
    let (directory, url, _) = directory().await;
    let publisher = service("publisher");
    configure(&publisher, &url, GROUP);
    publisher.startListening(&[PeerTransport::Tcp]).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if directory
                .discoveryRecords()
                .iter()
                .any(|record| record.properties.get("nodeId").is_some_and(|id| id == "publisher"))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("registration runs without a manual discovery request");
    publisher.stop().await.unwrap();
    directory.stop().await.unwrap();
}

/// Routes the documentation-only test subnet over real loopback sockets at the Host boundary.
struct RoutedTcp;
#[async_trait]
impl operit_host_api::TcpHost for RoutedTcp {
    async fn connect(&self, address: &str) -> operit_host_api::HostResult<Arc<dyn operit_host_api::TcpConnection>> {
        if address.starts_with("192.0.2.1:") {
            return Err(operit_host_api::HostError::new("Test route is unreachable"));
        }
        NativeTcpHost.connect(&address.replace("192.0.2.10", "127.0.0.1")).await
    }
    async fn bind(&self, address: &str) -> operit_host_api::HostResult<Arc<dyn operit_host_api::TcpListener>> {
        NativeTcpHost.bind(address).await
    }
}

#[tokio::test]
async fn directory_hint_recovers_the_business_port_only_after_session_authentication() {
    let (directory, url, _) = directory().await;
    let publisher = service("publisher");
    let mut client = service("client");
    Arc::get_mut(&mut Arc::get_mut(&mut client.state).unwrap().host)
        .unwrap()
        .tcpHost = Some(Arc::new(RoutedTcp));
    let original = StoredOutbound {
        endpoint: "127.0.0.1:1".into(),
        sessionId: "saved-session".into(),
        deviceId: "client".into(),
        peerNodeId: "publisher".into(),
        peerDeviceInfo: client.state.info.clone(),
        pairingServiceVersion: PAIRING_SERVICE_VERSION,
        sessionSecret: BASE64.encode([7; 32]),
        transport: "tcp".into(),
    };
    client
        .state
        .store
        .putRecord(RUNTIME_LINK_ACCESS_OUTBOUND_SESSIONS_PATH, "saved-session", &original)
        .unwrap();
    publisher
        .state
        .store
        .putRecord(
            RUNTIME_LINK_ACCESS_INBOUND_SESSIONS_PATH,
            "saved-session",
            &StoredInbound {
                qrJoinConsent: None,
                deviceId: "client".into(),
                deviceInfo: client.state.info.clone(),
                pairingServiceVersion: PAIRING_SERVICE_VERSION,
                sessionSecret: original.sessionSecret.clone(),
            },
        )
        .unwrap();
    configure(&publisher, &url, GROUP);
    publisher.startListening(&[PeerTransport::Tcp]).await.unwrap();
    publisher.refreshDirectory(2000).await.unwrap();
    configure(&client, &url, GROUP);
    client.refreshDirectory(2000).await.unwrap();
    // Merely learning the hint cannot overwrite the stored pairing address.
    assert_eq!(client.outbound("publisher").unwrap().endpoint, original.endpoint);
    client.acquireChannel("publisher").await.unwrap();
    let saved = client.outbound("publisher").unwrap();
    let expected = publisher.ownAnnouncement().await.unwrap().unwrap();
    assert_eq!(saved.endpoint, expected.endpoints[0].trim_start_matches("tcp://"));
    assert_eq!(saved.sessionSecret, original.sessionSecret);
    assert_eq!(saved.sessionId, original.sessionId);
    client.stop().await.unwrap();
    publisher.stop().await.unwrap();
    directory.stop().await.unwrap();
}

#[tokio::test]
async fn discovery_http_rejects_oversized_and_malformed_responses() {
    let client = service("client");
    for (body, message) in [
        (vec![b'x'; MAX_RESPONSE + 1], "size limit"),
        (b"not json".to_vec(), "expected"),
    ] {
        let listener = NativeHttpServerHost.bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}{PATH}", listener.localAddress().unwrap());
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            listener
                .serve(
                    Arc::new(move |_| {
                        let body = body.clone();
                        Box::pin(async move { response(200, body) })
                    }),
                    Box::pin(async move {
                        let _ = stopped.await;
                    }),
                )
                .await
                .unwrap();
        });
        let failure = client.directoryHttp(url, "", vec![]).await.err().unwrap();
        assert!(failure.to_string().contains(message), "{failure}");
        let _ = stop.send(());
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn first_pairing_falls_back_from_an_unreachable_discovered_address() {
    let server = service("server");
    server.startListening(&[PeerTransport::Tcp]).await.unwrap();
    let mut client = service("client");
    Arc::get_mut(&mut Arc::get_mut(&mut client.state).unwrap().host)
        .unwrap()
        .tcpHost = Some(Arc::new(RoutedTcp));
    let mut record = server.ownAnnouncement().await.unwrap().unwrap();
    let port = url::Url::parse(&record.endpoints[0]).unwrap().port().unwrap();
    record.endpoints.insert(0, format!("tcp://192.0.2.1:{port}"));
    client.rememberDirectoryRecords(&[record]).unwrap();
    let pending = client
        .startPairing(
            PeerEndpoint {
                nodeId: "server".into(),
                address: format!("192.0.2.1:{port}"),
            },
            Some(PeerTransport::Tcp),
        )
        .await
        .unwrap();
    assert_eq!(pending.peerNodeId, "server");
    assert_eq!(
        client.pending(false, &pending.pairingId).unwrap().endpoint,
        format!("192.0.2.10:{port}")
    );
    client.stop().await.unwrap();
    server.stop().await.unwrap();
}

#[tokio::test]
async fn default_discovery_does_not_hide_multicast_errors_or_open_a_directory() {
    let server = service("server");
    server.startListening(&[PeerTransport::Tcp]).await.unwrap();
    assert!(server.state.discovery.directory.worker.lock().unwrap().is_none());
    assert!(server.discoverPeers(100).await.is_err());
    server.stop().await.unwrap();
}

#[tokio::test]
async fn directory_cache_expires_and_retains_hints_during_a_temporary_outage() {
    let (directory, url, _) = directory().await;
    let client = service("client");
    configure(&client, &url, GROUP);
    assert!(!client.discoverPeers(2000).await.unwrap().is_empty());
    directory.stop().await.unwrap();
    assert!(!client.discoverPeers(100).await.unwrap().is_empty());
    for entry in client.state.discovery.records.lock().unwrap().values_mut() {
        entry.expires = Some(0);
    }
    assert!(client.discoveryRecords().is_empty());
    assert!(client.discoverPeers(100).await.is_err());
}
