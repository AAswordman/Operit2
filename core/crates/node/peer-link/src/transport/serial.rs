//! 串口适配；端口 I/O 由传入的 Host 提供。
use super::stream::{ByteConnection, FramedPeerConnection};
use crate::{PeerConnection, PeerEndpoint, PeerListener};
use async_trait::async_trait;
use operit_host_api::HostManager::HostManager;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, Weak};
use tokio::sync::Mutex;

// A UART is exclusive even within one process. Keep the lease for the whole
// framed session, not just open(): probes and application calls must queue.
pub(super) fn endpointKey(port: &str) -> String {
    if cfg!(windows) { port.trim_start_matches(r"\\.\").to_ascii_lowercase() }
    else { port.to_owned() }
}
fn portLease(port: &str) -> Arc<Mutex<()>> {
    static PORTS: OnceLock<std::sync::Mutex<BTreeMap<String, Weak<Mutex<()>>>>> = OnceLock::new();
    let mut ports = PORTS.get_or_init(Default::default).lock().unwrap();
    ports.retain(|_, lease| lease.strong_count() != 0);
    let key = endpointKey(port);
    if let Some(lease) = ports.get(&key).and_then(Weak::upgrade) {
        return lease;
    }
    let lease = Arc::new(Mutex::new(()));
    ports.insert(key, Arc::downgrade(&lease));
    lease
}
struct SerialBytes {
    connection: Arc<dyn operit_host_api::SerialPort::SerialPortConnection>,
    lease: Mutex<Option<tokio::sync::OwnedMutexGuard<()>>>,
}
impl SerialBytes {
    fn new(
        connection: Arc<dyn operit_host_api::SerialPort::SerialPortConnection>,
        lease: Option<tokio::sync::OwnedMutexGuard<()>>,
    ) -> Self {
        Self {
            connection,
            lease: Mutex::new(lease),
        }
    }
}
#[async_trait]
impl ByteConnection for SerialBytes {
    async fn write(&self, bytes: &[u8]) -> Result<(), String> {
        self.connection
            .write(bytes)
            .await
            .map_err(|e| e.to_string())
    }
    async fn read(&self) -> Result<Option<Vec<u8>>, String> {
        self.connection.read().await.map_err(|e| e.to_string())
    }
    async fn close(&self) {
        self.connection.close().await;
        self.lease.lock().await.take();
    }
}

pub(super) async fn connect(
    host: &HostManager,
    source: PeerEndpoint,
    target: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerConnection>, String> {
    let baudRate = host.peerSerialBaudRate.unwrap_or(115200);
    if baudRate == 0 { return Err("Serial baud rate must be positive".into()); }
    let provider = host
        .serialPortHost
        .as_ref()
        .ok_or("Serial Host is not installed")?;
    let lease = portLease(&target.address).lock_owned().await;
    let connection = provider
        .open(&target.address, baudRate)
        .await
        .map_err(|e| e.to_string())?;
    let peer = FramedPeerConnection::newSerial(
        source,
        target,
        Arc::new(SerialBytes::new(connection, Some(lease))),
        true,
        maxMessageBytes,
    );
    if let Err(error) = peer.initializeSerial().await {
        peer.close().await;
        return Err(error);
    }
    Ok(peer)
}
struct SerialListener {
    provider: Arc<dyn operit_host_api::SerialPort::SerialPortHost>,
    source: PeerEndpoint,
    maxMessageBytes: usize,
    baudRate: u32,
    first: Mutex<Option<Arc<dyn PeerConnection>>>,
    closed: AtomicBool,
}

#[async_trait]
impl PeerListener for SerialListener {
    async fn accept(&self) -> Result<Option<Arc<dyn PeerConnection>>, String> {
        if self.closed.load(Ordering::Acquire) {
            return Ok(None);
        }
        if let Some(connection) = self.first.lock().await.take() {
            return Ok(Some(connection));
        }
        // A serial port is one point-to-point stream. Once the previous Link
        // connection closes, wait for the host lease to become available and
        // accept the next pairing/session connection on the same COM port.
        loop {
            if self.closed.load(Ordering::Acquire) {
                return Ok(None);
            }
            match self.provider.open(&self.source.address, self.baudRate).await {
                Ok(connection) => {
                    let target = PeerEndpoint {
                        nodeId: String::new(),
                        address: self.source.address.clone(),
                    };
                    return Ok(Some(FramedPeerConnection::newSerial(
                        self.source.clone(),
                        target,
                        Arc::new(SerialBytes::new(connection, None)),
                        false,
                        self.maxMessageBytes,
                    )));
                }
                Err(_) => {
                    operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
                        .waitForHostRuntimeDelay(25)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
    async fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }
}

pub(super) async fn listen(
    host: &HostManager,
    source: PeerEndpoint,
    maxMessageBytes: usize,
) -> Result<Arc<dyn PeerListener>, String> {
    let baudRate = host.peerSerialBaudRate.unwrap_or(115200);
    if baudRate == 0 { return Err("Serial baud rate must be positive".into()); }
    let provider = host
        .serialPortHost
        .as_ref()
        .ok_or("Serial Host is not installed")?
        .clone();
    // Network and serial listeners share the persisted host config, so a board
    // may advertise TCP at 0.0.0.0:8765 while its local serial endpoint is
    // named uart0. Host-owned listenerPort keeps that mapping out of Core.
    let port = provider
        .listenerPort()
        .unwrap_or_else(|| source.address.clone());
    let source = PeerEndpoint {
        nodeId: source.nodeId,
        address: port,
    };
    // Open once during listener creation so invalid ports/configuration fail
    // synchronously instead of leaving a silent background listener.
    let connection = provider
        .open(&source.address, baudRate)
        .await
        .map_err(|e| e.to_string())?;
    let target = PeerEndpoint {
        nodeId: String::new(),
        address: source.address.clone(),
    };
    Ok(Arc::new(SerialListener {
        provider,
        source,
        maxMessageBytes,
        baudRate,
        first: Mutex::new(Some(FramedPeerConnection::newSerial(
            target.clone(),
            target,
            Arc::new(SerialBytes::new(connection, None)),
            false,
            maxMessageBytes,
        ))),
        closed: AtomicBool::new(false),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct StubPort;
    #[async_trait]
    impl operit_host_api::SerialPort::SerialPortConnection for StubPort {
        async fn write(&self, _: &[u8]) -> operit_host_api::HostResult<()> {
            Ok(())
        }
        async fn read(&self) -> operit_host_api::HostResult<Option<Vec<u8>>> {
            Ok(None)
        }
        async fn close(&self) {}
    }

    #[tokio::test]
    async fn serial_close_and_drop_both_release_session_lease() {
        let lease = portLease("serial-close-test");
        let bytes = SerialBytes::new(Arc::new(StubPort), Some(lease.clone().lock_owned().await));
        assert!(lease.try_lock().is_err());
        bytes.close().await;
        let held = lease
            .try_lock()
            .expect("explicit close releases port even while wrapper lives");
        drop(held);
        bytes.close().await;
        let bytes = SerialBytes::new(Arc::new(StubPort), Some(lease.clone().lock_owned().await));
        assert!(lease.try_lock().is_err());
        drop(bytes);
        assert!(
            lease.try_lock().is_ok(),
            "cancelled connection releases port on drop"
        );
    }

    #[tokio::test]
    async fn serial_sessions_share_an_exclusive_cancellation_safe_lease() {
        let first = portLease("COM-lease-test");
        let second = portLease(if cfg!(windows) {
            r"\\.\COM-lease-test"
        } else {
            "COM-lease-test"
        });
        assert!(Arc::ptr_eq(&first, &second));
        let held = first.lock_owned().await;
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(10),
            second.clone().lock_owned()
        )
        .await
        .is_err());
        // Cancelling a waiting availability probe cannot retain the lease.
        drop(held);
        let next = tokio::time::timeout(std::time::Duration::from_secs(1), second.lock_owned())
            .await
            .unwrap();
        drop(next);
    }
    struct BaudPort(std::sync::Mutex<Vec<u32>>);
    #[async_trait]
    impl operit_host_api::SerialPort::SerialPortHost for BaudPort {
        async fn open(&self, _: &str, baud: u32) -> operit_host_api::HostResult<Arc<dyn operit_host_api::SerialPort::SerialPortConnection>> {
            self.0.lock().unwrap().push(baud);
            Ok(Arc::new(StubPort))
        }
    }
    #[tokio::test]
    async fn serial_audio_baud_configuration_applies_to_listener_reopen() {
        let provider = Arc::new(BaudPort(std::sync::Mutex::new(Vec::new())));
        let host = HostManager { serialPortHost: Some(provider.clone()), ..HostManager::default() }.withPeerSerialBaudRate(921600);
        let endpoint = PeerEndpoint { nodeId: "audio-baud".into(), address: "audio-uart".into() };
        let listener = listen(&host, endpoint.clone(), 8192).await.unwrap();
        assert_eq!(*provider.0.lock().unwrap(), vec![921600]);
        listener.accept().await.unwrap().unwrap().close().await;
        listener.accept().await.unwrap().unwrap().close().await;
        assert_eq!(*provider.0.lock().unwrap(), vec![921600, 921600]);
        listener.close().await;
        let invalid = host.withPeerSerialBaudRate(0);
        assert!(listen(&invalid, endpoint, 8192).await.is_err());
    }

}
