#![allow(non_snake_case)]

mod edge_chat;
mod edge_session;
mod config;
#[cfg(target_os = "espidf")]
mod edge_link;
#[cfg(target_os = "espidf")]
mod edge_screen;
#[cfg(target_os = "espidf")]
mod edge_serial;
#[cfg(target_os = "espidf")]
mod edge_store;
#[cfg(target_os = "espidf")]
mod lvgl;
#[cfg(target_os = "espidf")]
mod ui_deploy;
#[cfg(target_os = "espidf")]
mod settings;
mod status;
mod ui;

#[cfg(target_os = "espidf")]
mod web;
#[cfg(target_os = "espidf")]
mod wifi;

/// Starts the ESP32-2432S028 Operit Edge firmware.
#[cfg(target_os = "espidf")]
fn main() {
    if let Err(error) = runFirmware() {
        log::error!("operit-esp32 failed: {}", error.message);
        panic!("operit-esp32 failed: {}", error.message);
    }
}

/// Stops non-ESP-IDF targets from launching the firmware binary.
#[cfg(not(target_os = "espidf"))]
fn main() {
    eprintln!("operit-esp32 requires the xtensa-esp32-espidf target");
    std::process::exit(1);
}

#[cfg(target_os = "espidf")]
fn runFirmware() -> operit_host_api::HostResult<()> {
    use std::sync::Arc;

    use esp_idf_hal::delay::FreeRtos;
    use esp_idf_hal::gpio::Gpio0;
    use esp_idf_hal::peripherals::Peripherals;
    use esp_idf_hal::uart::{config::Config as UartConfig, UartDriver};
    use esp_idf_hal::units::Hertz;
    use esp_idf_svc::nvs::EspDefaultNvsPartition;
    use operit_board_esp32::{Esp32Board, INITIAL_EXPRESSION, LED_GREEN_PIN, LED_RED_PIN};
    use operit_host_api::{HostError, RobotFaceHost};
    use crate::config::Esp32FirmwareConfig;
    use crate::edge_screen::Esp32ScreenService;
    use crate::edge_store::Esp32EdgePairingStore;
    use crate::lvgl::{updateStatus, Esp32Lvgl};
    use crate::settings::{Esp32SettingsStore, Esp32SetupServer};
    use crate::status::FirmwareStatus;
    use crate::web::Esp32WebHome;
    use crate::wifi::Esp32Wifi;

    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("operit-esp32 stability diagnostics v1; reset_reason={}", unsafe {
        esp_idf_svc::sys::esp_reset_reason()
    });
    logRuntimeHealth("boot");

    let peripherals = Peripherals::take().map_err(|error| HostError::new(error.to_string()))?;
    let modem = peripherals.modem;
    let nvsPartition =
        EspDefaultNvsPartition::take().map_err(|error| HostError::new(format!("nvs: {error}")))?;
    let edgeStore = Arc::new(Esp32EdgePairingStore::new(nvsPartition.clone())?);
    let settingsStore = Esp32SettingsStore::new(nvsPartition.clone())?;
    let settings = settingsStore.load()?;
    let config = Esp32FirmwareConfig::fromSettings(&settings);
    let mut board = Esp32Board::new(
        peripherals.spi2,
        peripherals.pins.gpio2,
        peripherals.pins.gpio12,
        peripherals.pins.gpio13,
        peripherals.pins.gpio14,
        peripherals.pins.gpio15,
        peripherals.pins.gpio21,
        peripherals.pins.gpio4,
        peripherals.pins.gpio16,
        peripherals.pins.gpio17,
        peripherals.spi3,
        peripherals.pins.gpio25,
        peripherals.pins.gpio32,
        peripherals.pins.gpio39,
        peripherals.pins.gpio33,
        peripherals.pins.gpio36,
    )?;
    let faceHost = board.robotFaceHost();
    board.activateLvgl();
    let mut lvgl = Esp32Lvgl::new(&board)?;
    let hostManager = board.installIntoHostManager();
    let screenMirror = board.screenMirror();
    let screenService = Arc::new(Esp32ScreenService::new(Arc::clone(&screenMirror)));
    let status = Arc::new(FirmwareStatus::new(INITIAL_EXPRESSION));
    let setExpression = |expression: &str| -> operit_host_api::HostResult<()> {
        let state = faceHost.setExpression(operit_host_api::RobotFaceExpressionRequest {
            expression: expression.to_string(),
        })?;
        status.setExpression(state.expression);
        Ok(())
    };
    let deviceIo = hostManager.deviceIoHost.as_ref()
        .ok_or_else(|| HostError::new("Board digital I/O is unavailable"))?;
    setExpression("booting")?;
    // Show the launcher even if the configured network is unavailable.
    lvgl.pump(1);
    let (_wifi, wifiMode) = Esp32Wifi::connectOrSetup(modem, &config, nvsPartition.clone())?;
    let _sntp = if wifiMode == crate::wifi::Esp32WifiMode::Station {
        Some(Esp32Wifi::startTimeSync()?)
    } else {
        None
    };
    let _setupServer = match wifiMode {
        crate::wifi::Esp32WifiMode::Station => {
            let ip = _wifi.ipv4()?;
            status.setWifiSsid(config.wifiSsid.clone());
            status.setIpv4(ip.to_string());
            setExpression("online")?;
            deviceIo.setDigitalOutput(operit_host_api::DeviceDigitalOutputRequest { pin: LED_GREEN_PIN, level: true })?;
            log::info!("operit-esp32 online at http://{ip}/");
            None
        }
        crate::wifi::Esp32WifiMode::SetupAccessPoint => {
            status.setWifiSsid("Operit-ESP32-Setup");
            status.setIpv4("192.168.4.1");
            setExpression("error")?;
            deviceIo.setDigitalOutput(operit_host_api::DeviceDigitalOutputRequest { pin: LED_RED_PIN, level: true })?;
            log::info!("operit-esp32 setup AP ready: Operit-ESP32-Setup / http://192.168.4.1/");
            Some(Esp32SetupServer::start(
                Arc::clone(&status),
                Arc::clone(&settingsStore),
                config.httpPort,
            )?)
        }
    };
    let edgeLink = edge_link::Esp32EdgeLinkServer::start(
        config.edgePort,
        config.edgeToken.clone(),
        Arc::clone(&status),
        edgeStore,
        if config.hasEdgeToken() {
            Some(
                UartDriver::new(
                    peripherals.uart0,
                    peripherals.pins.gpio1,
                    peripherals.pins.gpio3,
                    Option::<Gpio0<'static>>::None,
                    Option::<Gpio0<'static>>::None,
                    &UartConfig::new().baudrate(Hertz(115_200)),
                )
                .map_err(|error| HostError::new(format!("Edge UART: {error}")))?,
            )
        } else {
            None
        },
    )?;
    let edgeReady = edgeLink.is_some();
    let _home =
        if wifiMode == crate::wifi::Esp32WifiMode::Station && !status.snapshot().ipv4.is_empty() {
            Some(Esp32WebHome::start(
                Arc::clone(&status),
                screenMirror,
                Arc::clone(&screenService),
                config.httpPort,
                config.edgeToken.clone(),
            )?)
        } else {
            None
        };

    updateStatus(&mut lvgl, &status, edgeReady);
    lvgl.pump(1);
    let mut lastExpression = status.snapshot().expression;
    let mut swipe = crate::ui::SwipeTracker::new();
    logRuntimeHealth("ready");
    let mut nextHealth = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        for input in screenService.drainInputs() {
            match input.action.as_str() {
                "down" => {
                    lvgl.setTouch(Some((input.x, input.y)));
                }
                "up" => {
                    lvgl.setTouch(None);
                }
                "tap" => {
                    lvgl.setTouch(Some((input.x, input.y)));
                    lvgl.pump(1);
                    lvgl.setTouch(None);
                }
                "swipe" => {
                    if let (Some(endX), Some(endY)) = (input.endX, input.endY) {
                        lvgl.setTouch(Some((input.x, input.y)));
                        lvgl.pump(1);
                        lvgl.setTouch(Some((endX, endY)));
                        lvgl.pump(1);
                        lvgl.setTouch(None);
                    }
                }
                _ => {}
            }
            log::debug!("operit-esp32 remote screen input: {}", input.action);
        }
        let point = match board.pollTouch() {
            Ok(point) => point,
            Err(error) => {
                log::warn!("operit-esp32 touch: {}", error.message);
                None
            }
        };
        if let Some(crate::ui::UiGesture::Back) =
            swipe.onSample(point.map(|sample| (sample.x, sample.y)))
        {
            lvgl.goHome();
        }
        lvgl.setTouch(point.map(|sample| (sample.x, sample.y)));
        lvgl.pump(20);
        for action in lvgl.drainActions() {
            match action.as_str() {
                "face_online" => {
                    setExpression("online")?;
                }
                "face_neutral" => {
                    setExpression("neutral")?;
                }
                "run_node" => {
                    // The Edge listener is intentionally started at boot. This action
                    // gives the local terminal a visible confirmation without creating
                    // a duplicate listener.
                    setExpression("listening")?;
                }
                "edge_search" => setExpression("listening")?,
                "edge_pair" => setExpression("listening")?,
                "edge_chat" => {}
                _ => log::debug!("operit-esp32 LVGL action: {action}"),
            }
        }
        if let Ok(face) = faceHost.getExpression() {
            if face.expression != lastExpression {
                lastExpression = face.expression.clone();
                status.setExpression(face.expression);
            }
        }
        updateStatus(&mut lvgl, &status, edgeReady);
        lvgl.setChatPreview(&crate::edge_chat::preview());
        if std::time::Instant::now() >= nextHealth {
            logRuntimeHealth("running");
            nextHealth = std::time::Instant::now() + std::time::Duration::from_secs(30);
        }
        FreeRtos::delay_ms(1);
    }
}

/// Fixed-size diagnostics: no history buffer or framebuffer copies.
#[cfg(target_os = "espidf")]
fn logRuntimeHealth(stage: &str) {
    use esp_idf_svc::sys;
    unsafe {
        log::info!(
            "health {stage}: heap_free={} heap_min={} largest_8bit={} main_stack_free={}",
            sys::esp_get_free_heap_size(),
            sys::esp_get_minimum_free_heap_size(),
            sys::heap_caps_get_largest_free_block(sys::MALLOC_CAP_8BIT),
            sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()),
        );
    }
}
