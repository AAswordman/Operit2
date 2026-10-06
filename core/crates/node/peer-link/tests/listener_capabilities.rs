//! Capability declarations are operation-specific and never probe sockets or permissions.
use async_trait::async_trait;
use operit_host_api::{HostManager::HostManager, HttpServer::*, ServiceDiscovery::*, *};
use operit_peer_link::*;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

struct HttpFixture { upgrades: bool, binds: AtomicUsize }
#[async_trait]
impl HttpServerHost for HttpFixture {
    /// Returns the fixture's explicitly declared upgrade capability.
    fn supportsWebSocketUpgrade(&self) -> bool { self.upgrades }
    /// Counts unexpected server acquisition without opening a socket.
    async fn bind(&self, _: &str) -> HostResult<Arc<dyn HttpServerListener>> {
        self.binds.fetch_add(1, Ordering::SeqCst);
        Err(HostError::new("Capability fixture cannot open sockets"))
    }
}
struct TcpFixture;
#[async_trait]
impl TcpHost for TcpFixture {
    /// Rejects connection attempts during capability inspection.
    async fn connect(&self, _: &str) -> HostResult<Arc<dyn TcpConnection>> { panic!("Unexpected TCP connect") }
    /// Rejects bind attempts during capability inspection.
    async fn bind(&self, _: &str) -> HostResult<Arc<dyn TcpListener>> { panic!("Unexpected TCP bind") }
}
struct SerialFixture;
#[async_trait]
impl SerialPortHost for SerialFixture {
    /// Rejects serial I/O during capability inspection.
    async fn open(&self, _: &str, _: u32) -> HostResult<Arc<dyn SerialPortConnection>> { panic!("Unexpected serial open") }
}
struct DiscoveryFixture(bool);
impl ServiceDiscoveryHost for DiscoveryFixture {
    /// Returns advertisement capability independently of browsing support.
    fn supportsAdvertisement(&self) -> bool { self.0 }
    /// Rejects advertisement I/O during capability inspection.
    fn advertise(&self, _: ServiceAdvertisement) -> HostResult<Box<dyn DiscoveryAdvertisement>> { panic!("Unexpected advertise") }
    /// Rejects discovery I/O during capability inspection.
    fn discover(&self, _: &str, _: u64) -> HostResult<Vec<DiscoveredService>> { panic!("Unexpected discover") }
    /// Rejects subscription I/O during capability inspection.
    fn subscribe(&self, _: &str, _: DiscoveryCallback) -> HostResult<Box<dyn DiscoverySubscription>> { panic!("Unexpected subscribe") }
}
struct BluetoothFixture(bool);
impl BluetoothHost for BluetoothFixture {
    /// Returns classic listening capability independently of BLE support.
    fn supportsClassicListening(&self) -> bool { self.0 }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn requestBluetoothPermission(&self) -> HostResult<String> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothState(&self) -> HostResult<BluetoothStateData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn requestEnableBluetooth(&self) -> HostResult<String> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn listBluetoothBondedDevices(&self) -> HostResult<BluetoothBondedDevicesData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn scanBluetoothDevices(
        &self,
        request: BluetoothScanRequest,
    ) -> HostResult<BluetoothScanResultData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothConnect(
        &self,
        request: BluetoothClassicConnectRequest,
    ) -> HostResult<BluetoothSessionData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothListen(
        &self,
        request: BluetoothClassicListenRequest,
    ) -> HostResult<BluetoothSessionData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothAccept(
        &self,
        request: BluetoothClassicAcceptRequest,
    ) -> HostResult<BluetoothSessionData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothSend(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
    ) -> HostResult<BluetoothTransferData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothRead(&self, request: BluetoothReadRequest) -> HostResult<BluetoothReadData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothSendAndRead(
        &self,
        sessionId: &str,
        payload: BluetoothPayload,
        read: BluetoothReadRequest,
    ) -> HostResult<BluetoothReadData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothClose(&self, sessionId: &str) -> HostResult<String> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothBleConnect(
        &self,
        request: BluetoothBleConnectRequest,
    ) -> HostResult<BluetoothSessionData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothBleDiscoverServices(
        &self,
        sessionId: &str,
        timeoutMs: i64,
    ) -> HostResult<BluetoothBleServicesData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothBleReadCharacteristic(
        &self,
        address: BluetoothBleCharacteristicAddress,
    ) -> HostResult<BluetoothReadData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothBleWriteCharacteristic(
        &self,
        request: BluetoothBleWriteRequest,
    ) -> HostResult<BluetoothTransferData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothBleWriteAndReadCharacteristic(
        &self,
        request: BluetoothBleWriteAndReadRequest,
    ) -> HostResult<BluetoothReadData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothBleSubscribeCharacteristic(
        &self,
        request: BluetoothBleSubscribeRequest,
    ) -> HostResult<BluetoothTransferData> { panic!("Capability inspection performed Bluetooth I/O") }
    /// Rejects I/O because capability queries must not invoke Bluetooth operations.
    fn bluetoothBleReadNotifications(
        &self,
        sessionId: &str,
        limit: i64,
    ) -> HostResult<BluetoothBleNotificationData> { panic!("Capability inspection performed Bluetooth I/O") }
}

/// Keeps outgoing client Hosts independent of the inbound transport list.
#[test]
fn client_only_host_has_no_listeners() {
    let host = HostManager::new().withHostEnvironment(HostEnvironmentDescriptor::windows());
    assert_eq!(HostPeerLink::listenerCapabilities(&host), PeerListenerCapabilities { transports: vec![], discoveryAdvertisement: false });
}

/// Ensures a TCP-only Host cannot authorize HTTP or WebSocket listening.
#[test]
fn tcp_capability_does_not_enable_other_protocols() {
    let host = HostManager::new().withHostEnvironment(HostEnvironmentDescriptor::web()).withTcpHost(Arc::new(TcpFixture));
    assert_eq!(HostPeerLink::listenerCapabilities(&host).transports, vec![PeerTransport::Tcp]);
}

/// Separates HTTP serving from WebSocket upgrade support on the same Host.
#[test]
fn http_and_websocket_capabilities_are_independent() {
    for upgrades in [false, true] {
        let server = Arc::new(HttpFixture { upgrades, binds: AtomicUsize::new(0) });
        let host = HostManager::new().withHttpServerHost(server.clone());
        let capabilities = HostPeerLink::listenerCapabilities(&host);
        assert!(capabilities.transports.contains(&PeerTransport::Http));
        assert_eq!(capabilities.transports.contains(&PeerTransport::WebSocket), upgrades);
        assert_eq!(server.binds.load(Ordering::SeqCst), 0);
    }
}

/// Distinguishes BLE-only Hosts from RFCOMM listening Hosts without requesting permissions.
#[test]
fn bluetooth_classic_listening_requires_an_explicit_capability() {
    for classic in [false, true] {
        let host = HostManager::new().withBluetoothHost(Arc::new(BluetoothFixture(classic)));
        assert_eq!(HostPeerLink::listenerCapabilities(&host).transports.contains(&PeerTransport::Bluetooth), classic);
    }
}

/// Exposes a serial Host as the point-to-point inbound listener used by a device.
#[test]
fn installed_serial_host_declares_a_listener() {
    let host = HostManager::new().withSerialPortHost(Arc::new(SerialFixture));
    assert!(HostPeerLink::listenerCapabilities(&host).transports.contains(&PeerTransport::Serial));
}

/// Keeps advertisements separate from transport and service-browsing capabilities.
#[test]
fn discovery_advertisement_capability_is_independent() {
    for advertisement in [false, true] {
        let host = HostManager::new().withServiceDiscoveryHost(Arc::new(DiscoveryFixture(advertisement)));
        let capabilities = HostPeerLink::listenerCapabilities(&host);
        assert!(capabilities.transports.is_empty());
        assert_eq!(capabilities.discoveryAdvertisement, advertisement);
    }
}

/// Rejects unsupported WS before even binding an otherwise usable HTTP server.
#[tokio::test]
async fn unsupported_websocket_does_not_acquire_http_resources() {
    let server = Arc::new(HttpFixture { upgrades: false, binds: AtomicUsize::new(0) });
    let host = Arc::new(HostManager::new().withHttpServerHost(server.clone()));
    let result = HostPeerLink::default().listen(host, PeerEndpoint { nodeId: "fixture".into(), address: "127.0.0.1:0".into() }, PeerTransport::WebSocket).await;
    assert!(matches!(result, Err(message) if message == "Host does not support WebSocket peer listeners"));
    assert_eq!(server.binds.load(Ordering::SeqCst), 0);
}


#[test]
fn connection_policy_only_serializes_the_same_exclusive_endpoint() {
    let link = HostPeerLink::default();
    let endpoint = |address: &str| PeerEndpoint { nodeId: "peer".into(), address: address.into() };
    for transport in [PeerTransport::Tcp, PeerTransport::Http, PeerTransport::WebSocket, PeerTransport::Bluetooth] {
        assert_eq!(link.exclusiveEndpointKey(&endpoint("address"), transport), None);
    }
    assert_ne!(link.exclusiveEndpointKey(&endpoint("COM1"), PeerTransport::Serial),
        link.exclusiveEndpointKey(&endpoint("COM2"), PeerTransport::Serial));
    #[cfg(windows)]
    assert_eq!(link.exclusiveEndpointKey(&endpoint("COM1"), PeerTransport::Serial),
        link.exclusiveEndpointKey(&endpoint(r"\\.\com1"), PeerTransport::Serial));
}
