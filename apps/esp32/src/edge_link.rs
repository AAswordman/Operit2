#![allow(non_snake_case)]

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener as StdTcpListener, TcpStream as StdTcpStream};
use std::sync::Arc;

use async_trait::async_trait;
use esp_idf_hal::uart::UartDriver;
use esp_idf_svc::mdns::EspMdns;
use operit_edge_transport::{EdgePairingAuthority, EdgePairingStore, LinkChannel};
use operit_host_api::{HostError, HostResult};
use operit_link::{decodeLink, encodeLink, LinkDeviceInfo, LinkFrame};
use tokio::runtime::Builder;
use tokio::sync::Mutex;

use crate::edge_serial::Esp32UartLinkChannel;
use crate::edge_session::handleChannel;
use crate::status::FirmwareStatus;

const MAX_LINK_FRAME_BYTES: usize = 1024 * 1024;

/// ESP-IDF's mio reactor is unavailable, so the Wi-Fi carrier uses native
/// nonblocking BSD sockets while the current-thread Tokio runtime drives Link.
struct Esp32TcpLinkChannel {
    stream: Arc<StdTcpStream>,
    readLock: Mutex<()>,
    writeLock: Mutex<()>,
}

impl Esp32TcpLinkChannel {
    fn fromStream(stream: StdTcpStream) -> Result<Arc<Self>, String> {
        // ESP-IDF's socket wrapper does not implement TcpStream::try_clone().
        // Keep one socket and serialize reads/writes independently; Read/Write
        // are implemented for &TcpStream, so full-duplex operation remains safe.
        stream
            .set_nonblocking(true)
            .map_err(|error| format!("configure Edge TCP socket: {error}"))?;
        Ok(Arc::new(Self {
            stream: Arc::new(stream),
            readLock: Mutex::new(()),
            writeLock: Mutex::new(()),
        }))
    }
}
#[async_trait]
impl LinkChannel for Esp32TcpLinkChannel {
    async fn send(&self, frame: LinkFrame) -> Result<(), String> {
        let payload = encodeLink(&frame).map_err(|error| error.to_string())?;
        if payload.is_empty() || payload.len() > MAX_LINK_FRAME_BYTES {
            return Err("invalid Edge Link frame size".to_string());
        }
        let mut bytes = Vec::with_capacity(4 + payload.len());
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&payload);

        let _writeLock = self.writeLock.lock().await;
        let mut offset = 0;
        while offset < bytes.len() {
            match (&*self.stream).write(&bytes[offset..]) {
                Ok(0) => return Err("Edge TCP peer closed while writing".to_string()),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(format!("Edge TCP write: {error}")),
            }
        }
        Ok(())
    }

    async fn receive(&self) -> Result<Option<LinkFrame>, String> {
        let _readLock = self.readLock.lock().await;
        let mut header = [0u8; 4];
        let mut offset = 0;
        while offset < header.len() {
            match (&*self.stream).read(&mut header[offset..]) {
                Ok(0) if offset == 0 => return Ok(None),
                Ok(0) => return Err("Edge TCP peer closed during frame header".to_string()),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(format!("Edge TCP read: {error}")),
            }
        }
        let length = u32::from_be_bytes(header) as usize;
        if length == 0 || length > MAX_LINK_FRAME_BYTES {
            return Err(format!("invalid Edge Link frame length: {length}"));
        }
        let mut payload = vec![0u8; length];
        offset = 0;
        while offset < payload.len() {
            match (&*self.stream).read(&mut payload[offset..]) {
                Ok(0) => return Err("Edge TCP peer closed during frame payload".to_string()),
                Ok(count) => offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(format!("Edge TCP read: {error}")),
            }
        }
        decodeLink(&payload)
            .map(Some)
            .map_err(|error| format!("decode Edge Link frame: {error}"))
    }

    async fn close(&self) {
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

/// Runs the authenticated standard-Link listener on a dedicated lightweight
/// thread so network processing can progress independently of LVGL redraws.
pub struct Esp32EdgeLinkServer {
    _runtimeThread: Option<std::thread::JoinHandle<()>>,
    _mdns: Option<EspMdns>,
}

impl Esp32EdgeLinkServer {
    pub fn start(
        port: u16,
        token: String,
        status: Arc<FirmwareStatus>,
        store: Arc<dyn EdgePairingStore>,
        uart: Option<UartDriver<'static>>,
    ) -> HostResult<Option<Self>> {
        if token.trim().is_empty() {
            log::warn!("Edge Link disabled: OPERIT_EDGE_TOKEN is not configured");
            return Ok(None);
        }
        let tokenHash = operit_edge_transport::linkTokenHash(&token);
        crate::logRuntimeHealth("edge-runtime-ready");
        let authority = match EdgePairingAuthority::newWithStore(
            token,
            "esp32-edge".to_string(),
            LinkDeviceInfo {
                platform: "esp32".to_string(),
                model: "ESP32-2432S028".to_string(),
            },
            store,
            {
                let status = Arc::clone(&status);
                move |code| {
                    status.setPairingCode(code.clone());
                    log::info!("Edge Link pairing code: {code}");
                }
            },
        ) {
            Ok(authority) => Arc::new(authority),
            Err(error) => {
                return Err(HostError::new(format!(
                    "Edge Link persistent store: {error}"
                )))
            }
        };
        crate::logRuntimeHealth("edge-authority-ready");
        let serialChannel = match uart {
            Some(uart) => match Esp32UartLinkChannel::new(uart) {
                Ok(channel) => {
                    log::info!("Edge Link listening on UART0 GPIO1/GPIO3 at 115200 baud");
                    Some(channel)
                }
                Err(error) => {
                    log::error!("Edge UART listener: {}", error.message);
                    None
                }
            },
            None => None,
        };
        let listener = StdTcpListener::bind(format!("0.0.0.0:{port}"))
            .map_err(|error| HostError::new(format!("Edge Link listener: {error}")))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| HostError::new(format!("Edge Link nonblocking listener: {error}")))?;
        log::info!("Edge Link listening on TCP port {port}");
        crate::logRuntimeHealth("edge-before-mdns");
        let mdns = match EspMdns::take() {
            Ok(mut mdns) => {
                let txt = [
                    ("deviceId", "esp32-edge"),
                    ("displayName", "ESP32 Edge"),
                    ("platform", "esp32"),
                    ("model", "ESP32-2432S028"),
                    ("tokenHash", tokenHash.as_str()),
                    ("version", "edge-1"),
                ];
                // ESP-IDF requires a hostname before registering services.
                let registration = mdns.set_hostname("operit-edge-esp32").and_then(|()| {
                    mdns.add_service(
                        Some("operit-edge-esp32"),
                        "_operit-edge",
                        "_tcp",
                        port,
                        &txt,
                    )
                });
                match registration {
                    Ok(()) => {
                        log::info!("Edge mDNS discovery enabled (_operit-edge._tcp)");
                        Some(mdns)
                    }
                    Err(error) => {
                        log::warn!("Edge mDNS registration failed: {error}");
                        None
                    }
                }
            }
            Err(error) => {
                log::warn!("Edge mDNS initialization failed: {error}");
                None
            }
        };
        crate::logRuntimeHealth("edge-after-mdns");
        let runtimeThread = std::thread::Builder::new()
            .name("operit-edge-link".to_string())
            // Link MessagePack decoding and X25519 exceed a 6 KiB stack on
            // Xtensa. Keep headroom and measure the watermark after pairing.
            .stack_size(32 * 1024)
            .spawn(move || {
                let runtime = match Builder::new_current_thread().enable_time().build() {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        log::error!("Edge Link runtime: {error}");
                        return;
                    }
                };
                runtime.block_on(async move {
                    crate::logRuntimeHealth("edge-worker-ready");
                    let mut serialStarted = false;
                    loop {
                        if !serialStarted {
                            serialStarted = true;
                            if let Some(channel) = serialChannel.clone() {
                                let authority = Arc::clone(&authority);
                                tokio::spawn(async move {
                                    loop {
                                        match handleChannel(Arc::clone(&authority), channel.clone()).await {
                                            Ok(()) => {}
                                            Err(error) => {
                                                log::warn!("Edge UART session: {error}");
                                                if channel.isClosed() {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }
                        match listener.accept() {
                            Ok((stream, peer)) => {
                                log::info!("Edge Link connection from {peer}");
                                match Esp32TcpLinkChannel::fromStream(stream) {
                                    Ok(channel) => {
                                        let authority = Arc::clone(&authority);
                                        tokio::spawn(async move {
                                            if let Err(error) = handleChannel(authority, channel).await {
                                                log::warn!("Edge Link session: {error}");
                                            }
                                        });
                                    }
                                    Err(error) => log::warn!("Edge Link stream: {error}"),
                                }
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                            Err(error) => log::warn!("Edge Link accept: {error}"),
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                    }
                });
            })
            .map_err(|error| HostError::new(format!("Edge Link thread: {error}")))?;
        Ok(Some(Self {
            _runtimeThread: Some(runtimeThread),
            _mdns: mdns,
        }))
    }

    /// Clears all persisted Edge pairings without changing Wi-Fi or the token.
    pub fn clearPairings(&self) -> Result<(), String> {
        Err("Edge pairing reset requires restarting the device".to_string())
    }

    /// The listener is continuously driven by the dedicated runtime thread.
    pub fn poll(&mut self) {}
}
