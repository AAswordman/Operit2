#![allow(non_snake_case)]

use std::net::TcpListener as StdTcpListener;
use std::sync::Arc;

use esp_idf_hal::uart::UartDriver;
use esp_idf_svc::mdns::EspMdns;
use operit_edge_transport::{EdgePairingAuthority, EdgePairingStore, LinkChannel};
use operit_host_api::{HostError, HostResult};
use operit_link::LinkDeviceInfo;
use operit_board_esp32::link_channel::Esp32TcpLinkChannel;
use tokio::runtime::Builder;

use crate::edge_serial::Esp32UartLinkChannel;
use crate::edge_session::handleChannel;
use crate::status::FirmwareStatus;

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
