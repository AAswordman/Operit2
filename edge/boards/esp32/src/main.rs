#![allow(non_snake_case)]

mod config;
mod runtime_profile;
mod edge_chat;
mod device_preferences;
mod device_speech;
#[cfg(target_os = "espidf")]
mod audio;
#[cfg(target_os = "espidf")]
mod wav;
mod device_space_ui;
mod space_work;
#[cfg(target_os = "espidf")]
mod device_services;
#[cfg(target_os = "espidf")]
mod ui;
#[cfg(target_os = "espidf")]
mod runtime_host;
mod runtime_storage_codec;
mod runtime_storage_host;
mod runtime_storage_pack;
#[cfg(target_os = "espidf")]
mod settings;
#[cfg(target_os = "espidf")]
mod space_join_ui;
mod status;
#[cfg(target_os = "espidf")]
mod ui_capabilities;

#[cfg(target_os = "espidf")]
mod wifi;

// Fixed-size allocation failure evidence survives a panic restart in RTC RAM.
// The failure hook must not allocate, acquire locks, log, or write Flash.
#[cfg(target_os = "espidf")]
#[link_section = ".rtc_noinit"]
static ALLOCATION_FAILURE: [std::sync::atomic::AtomicU32; 3] =
    [const { std::sync::atomic::AtomicU32::new(0) }; 3];
#[cfg(target_os = "espidf")]
static PREVIOUS_ALLOCATION_FAILURE: std::sync::OnceLock<Option<(u32, u32)>> = std::sync::OnceLock::new();
#[cfg(target_os = "espidf")]
unsafe extern "C" fn recordAllocationFailure(size: usize, caps: u32, _: *const std::ffi::c_char) {
    use std::sync::atomic::Ordering;
    ALLOCATION_FAILURE[1].store(size as u32, Ordering::Relaxed);
    ALLOCATION_FAILURE[2].store(caps, Ordering::Relaxed);
    ALLOCATION_FAILURE[0].store(0x4f4f4d31, Ordering::Release);
}
#[cfg(target_os = "espidf")]
fn initializeAllocationDiagnostic() {
    unsafe {
        use std::sync::atomic::Ordering;
        let previous = if esp_idf_svc::sys::esp_reset_reason() == esp_idf_svc::sys::esp_reset_reason_t_ESP_RST_PANIC
            && ALLOCATION_FAILURE[0].load(Ordering::Acquire) == 0x4f4f4d31 {
            Some((ALLOCATION_FAILURE[1].load(Ordering::Relaxed), ALLOCATION_FAILURE[2].load(Ordering::Relaxed)))
        } else { None };
        let _ = PREVIOUS_ALLOCATION_FAILURE.set(previous);
        for slot in &ALLOCATION_FAILURE { slot.store(0, Ordering::Relaxed); }
        esp_idf_svc::sys::heap_caps_register_failed_alloc_callback(Some(recordAllocationFailure));
    }
}

/// Starts the ESP32-2432S028 Operit Edge firmware.
#[cfg(target_os = "espidf")]
fn main() {
    if let Err(error) = runFirmware() {
        // Startup has not handed UART to Link yet. Use the ROM formatter rather
        // than linking Rust's buffered stderr solely for this fatal diagnostic.
        unsafe { esp_idf_hal::sys::esp_rom_printf(
            b"operit-esp32 startup failed: %.*s\n\0".as_ptr().cast(),
            error.message.len().min(512) as i32, error.message.as_ptr().cast::<std::ffi::c_char>()); }
        // A recoverable storage/configuration error must not become a rapid
        // reboot loop that prevents diagnostics or replacement firmware.
        loop {
            esp_idf_hal::delay::FreeRtos::delay_ms(1000);
        }
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

    use crate::config::Esp32FirmwareConfig;
    use crate::ui::{updateStatus, Esp32Ui};
    use crate::runtime_host::{Esp32RuntimeStorageHost, Esp32ServiceDiscoveryHost};
    use crate::settings::{Esp32SettingsStore, Esp32SetupServer};
    use crate::status::FirmwareStatus;
    use crate::wifi::Esp32Wifi;
    use esp_idf_hal::delay::FreeRtos;
    use esp_idf_hal::peripherals::Peripherals;
    use esp_idf_svc::nvs::EspDefaultNvsPartition;
    use operit_board_esp32::{Esp32Board, INITIAL_EXPRESSION, LED_GREEN_PIN, LED_RED_PIN};
    use operit_host_api::TimeUtils::currentTimeMillis;
    use operit_host_api::{HostError, HostRuntimeTaskSchedulerHost};
    use operit_link::protocol::LinkDeviceInfo;
    use operit_edge_sdk::device::PeerRouter::EdgePeerRouter;
    use operit_edge_sdk::runtime::NodeServices::NodeServices;
    use operit_edge_sdk::runtime::PeerStateStore::PeerStateStore;
    use operit_edge_sdk::runtime::RuntimePeerService::RuntimePeerService;
    use operit_edge_sdk::storage::CoreNodeIdentityStore::CoreNodeIdentityStore;

    esp_idf_svc::sys::link_patches();
    initializeAllocationDiagnostic();

    // Mio/Tokio wake their device executors through eventfd. ESP-IDF returns
    // EACCES until its EventFD VFS is registered; keep the mount alive for the
    // entire firmware lifetime, including the Host worker and UI executor.
    let _eventfs = esp_idf_svc::io::vfs::MountedEventfs::mount(8)
        .map_err(|error| HostError::new(format!("mount EventFD VFS: {error}")))?;

    let peripherals = Peripherals::take().map_err(|error| HostError::new(error.to_string()))?;
    let modem = peripherals.modem;
    let nvsPartition =
        EspDefaultNvsPartition::take().map_err(|error| HostError::new(format!("nvs: {error}")))?;
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
    let serialConfig = esp_idf_hal::uart::config::Config::new()
        .baudrate(esp_idf_hal::units::Hertz(115_200))
        // Preserve a bounded frame during brief crypto/UI/flash stalls.
        .rx_fifo_size(8192)
        .queue_size(0);
    let serialUart = esp_idf_hal::uart::UartDriver::new(
        peripherals.uart0,
        peripherals.pins.gpio1,
        peripherals.pins.gpio3,
        Option::<esp_idf_hal::gpio::AnyIOPin>::None,
        Option::<esp_idf_hal::gpio::AnyIOPin>::None,
        &serialConfig,
    )
    .map_err(|error| HostError::new(error.to_string()))?;
    let serialHost = Arc::new(operit_board_esp32::serial::Esp32SerialPortHost::new("uart0", serialUart));
    let serialDebug = serialHost.debug_io();
    let faceHost = board.robotFaceHost();
    board.activateUi();
    let mut ui = Esp32Ui::new(&board)?;
    let scheduler =
        Arc::new(operit_host_native_scheduler::LocalHostRuntimeTaskSchedulerHost::new()?);
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(scheduler.clone());
    let runtimeStorage = Esp32RuntimeStorageHost::new(nvsPartition.clone())?;
    let sceneService = Arc::new(operit_edge_sdk::scene::SceneService::new_with_limits(
        320, 240, 24, true, runtime_profile::SCENE_LIMITS,
    ).map_err(HostError::new)?.with_storage(runtimeStorage.clone()).map_err(HostError::new)?);
    ui.attachScene(Arc::clone(&sceneService));
    let identity = CoreNodeIdentityStore::new(runtimeStorage.clone())
        .initialize()
        .map_err(HostError::new)?;
    let deviceInfo = PeerStateStore::new(runtimeStorage.clone())
        .deviceInfo(
            LinkDeviceInfo {
                platform: "esp32".into(),
                model: "ESP32-2432S028".into(),
            pairingIncarnation: None,
        },
            false,
        )
        .map_err(HostError::new)?;
    // Build the host before Wi-Fi so the board services are available to the
    // UI. mDNS itself is installed only after the station/AP network exists.
    let mut hostManager = board
        .installIntoHostManager()
        .withSerialPortHost(serialHost)
        .withRuntimeStorageHost(runtimeStorage.clone())
        .withHostRuntimeTaskSchedulerHost(scheduler.clone());

    let status = Arc::new(FirmwareStatus::new(INITIAL_EXPRESSION));
    let setExpression = |expression: &str| -> operit_host_api::HostResult<()> {
        let state = faceHost.setExpression(operit_host_api::RobotFaceExpressionRequest {
            expression: expression.to_string(),
        })?;
        status.setExpression(state.expression);
        Ok(())
    };
    let deviceIo = hostManager
        .deviceIoHost
        .as_ref()
        .ok_or_else(|| HostError::new("Board digital I/O is unavailable"))?;
    setExpression("booting")?;
    // Show the launcher even if the configured network is unavailable.
    ui.pump(1);
    let (_wifi, wifiMode) = Esp32Wifi::connectOrSetup(modem, &config, nvsPartition.clone())?;
    status.setWifiConnected(wifiMode == crate::wifi::Esp32WifiMode::Station && _wifi.stationConnected());
    let peerTransports = Esp32FirmwareConfig::peerTransports(wifiMode == crate::wifi::Esp32WifiMode::Station);
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
            deviceIo.setDigitalOutput(operit_host_api::DeviceDigitalOutputRequest {
                pin: LED_GREEN_PIN,
                level: true,
            })?;
            None
        }
        crate::wifi::Esp32WifiMode::SetupAccessPoint => {
            status.setWifiSsid("Operit-ESP32-Setup");
            status.setIpv4("192.168.4.1");
            setExpression("error")?;
            deviceIo.setDigitalOutput(operit_host_api::DeviceDigitalOutputRequest {
                pin: LED_RED_PIN,
                level: true,
            })?;
            Some(Esp32SetupServer::start(
                Arc::clone(&status),
                Arc::clone(&settingsStore),
                config.httpPort,
            )?)
        }
    };

    // ESP-IDF mDNS must be initialized after the Wi-Fi netif is up. Keep the
    // same HostManager and inject this host capability before constructing the
    // shared peer service; no Edge-specific pairing path is introduced.
    let discovery = Esp32ServiceDiscoveryHost::new()?;
    hostManager.serviceDiscoveryHost = Some(discovery);
    // AP/offline boots have no wall clock: previous-boot uptime deadlines cannot
    // expire safely. Invalidate only unfinished pairings, not identities/peers.
    PeerStateStore::new(runtimeStorage.clone())
        .discardPendingPairingsForUnsynchronizedClock(currentTimeMillis())
        .map_err(HostError::new)?;
    let edgeRouter = EdgePeerRouter::new(identity.nodeId.clone());
    let peerService = runtime_profile::create_peer_service(
        Arc::new(hostManager.clone()), &edgeRouter, deviceInfo.clone(),
        runtime_profile::peer_limits(),
    ).map_err(HostError::new)?;
    let peers = PeerStateStore::new(runtimeStorage.clone());
    let mut saved = peers.hostConfig().map_err(HostError::new)?
        .ok_or_else(|| HostError::new("Node listener preferences are not initialized"))?;
    if saved.transports != peerTransports {
        saved.transports = peerTransports.clone();
        saved.updatedAt = currentTimeMillis();
        peers.saveHostConfig(&saved).map_err(HostError::new)?;
    }
    let spaceService = operit_edge_sdk::runtime::NodeSpaceService::NodeSpaceService::new(
        runtimeStorage.clone(),
        peerService.clone(),
    )
    .map_err(HostError::new)?;
    let mut spaceJoinUi = crate::space_join_ui::SpaceJoinUi::new(
        spaceService.clone(),
        edgeRouter.clone(),
        operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceDeviceProfile {
            protocolVersion: None,
            nodeId: identity.nodeId.clone(),
            displayName: deviceInfo.model.clone(),
            userName: String::new(),
            platform: deviceInfo.platform.clone(),
            model: deviceInfo.model.clone(),
            coreVersion: None,
            pairingIncarnation: deviceInfo.pairingIncarnation.clone(),
            updatedAt: currentTimeMillis(),
    },
        scheduler.clone(),
    )?;
    let mut deviceSpaceUi = crate::device_space_ui::DeviceSpaceUi::new(
        NodeServices::new(peerService.clone()), spaceService.clone(), scheduler.clone(),
    );

    let speech = {
        let speech = Arc::new(device_speech::DeviceSpeech::default());
        audio::install(scheduler.clone(), speech.clone());
        speech
    };
    let preferenceSpace = spaceService.clone();
    scheduler.scheduleHostRuntimeAsyncTask("device-preferences", Box::new(move || Box::pin(async move {
        loop {
            if let Some(_work) = space_work::try_acquire() {
                let _ = preferenceSpace.synchronizePreferences().await;
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    })))?;
    let nativeHandler = device_services::callHandler(status.clone(), sceneService.clone());
    let configStorage = runtimeStorage.clone(); let configNode = identity.nodeId.clone();
    let handler: Arc<operit_edge_sdk::device::EdgeCallHandler> = Arc::new(move |request| {
        let storage = configStorage.clone(); let node = configNode.clone(); let native = nativeHandler.clone();
        Box::pin(async move {
            if let Some(response) = device_preferences::call(storage, &node, &request) { return Some(response); }
            native(request).await
        })
    });
    let nodeServices = NodeServices::new(peerService.clone());

    {
        let (speechOutput, speechInput) = speech.handlers();
        let edgeNode = operit_edge_sdk::device::EdgeNode::fromHostManager(hostManager.clone())
            .withCallHandler(handler)
            .withStreamHandlers(speechOutput, speechInput)
            .withNodeServices(nodeServices.clone());
        edgeRouter.install(Arc::new(edgeNode)).map_err(HostError::new)?;
    }
    let mut peerChanges = peerService.subscribePeerChanges();
    let mut pairingCodeSelection = crate::status::PairingCodeSelection::default();
    let mut pairingState = || -> Result<(bool, String), String> {
        let services = nodeServices.clone();
        let paired = !services
            .peers()
            .pairedPeers()
            .map_err(|error| error.to_string())?
            .is_empty();
        let prompts = services.peers().pairingPrompts().map_err(|error| error.to_string())?;
        // A restored stale request must not hide the code of a new USB pairing.
        let codes = pairingCodeSelection.select(&prompts);
        Ok((paired, codes))
    };
    // Reuse the already-running Host scheduler. Creating a second Tokio
    // runtime on the ESP32 main task causes a LoadProhibited panic after mDNS
    // initialization, which looks like the board disappears during discovery.
    // After boot UART0 belongs to framed Link/debug traffic. Console writers
    // do not share its frame lock; diagnostics use explicit health/UI requests.
    log::set_max_level(log::LevelFilter::Off);
    unsafe { esp_idf_hal::sys::esp_log_level_set(b"*\0".as_ptr().cast(), esp_idf_hal::sys::esp_log_level_t_ESP_LOG_NONE); }
    let listenService = peerService.clone();
    let initialTransports = peerTransports.clone();
    scheduler
        .scheduleHostRuntimeAsyncTask(
            "peer-listen-start",
            Box::new(move || {
                Box::pin(async move {
                    let _ = listenService.startListening(&initialTransports).await;
                })
            }),
        )
        .map_err(|error| HostError::new(error.to_string()))?;

    // The listener being enabled is not the same as having a live Space
    // route. The display must start offline until Core has admitted the Edge.
    let paired = match pairingState() {
        Ok((paired, codes)) => {
            status.setPairingCode(codes);
            paired
        }
        Err(_) => false,
    };
    updateStatus(&mut ui, &status, false, paired);
    ui.pump(1);
    let mut lastSpeechResult=0;
    let mut pendingSpeechResult = None;
    let mut lastExpression = status.snapshot().expression;
    let mut lastStatusRevision = status.revision();
    let mut lastEdgeReady = false;
    let mut lastPaired = paired;
    let mut nextPairingPoll = std::time::Instant::now();
    let mut nextWifiPoll = std::time::Instant::now();
    let mut lastChatRevision = u32::MAX;
    let mut lastChatConnected = false;
    let mut chatRoute = crate::edge_chat::SpaceChatRoute::default();
    let mut nextRoutePoll = std::time::Instant::now();
    loop {
        if std::time::Instant::now() >= nextWifiPoll {
            status.setWifiConnected(wifiMode == crate::wifi::Esp32WifiMode::Station && _wifi.stationConnected());
            nextWifiPoll = std::time::Instant::now() + std::time::Duration::from_secs(1);
        }
        let point = board.pollTouch().unwrap_or(None);
        // The shared UI runtime receives the sampled touch directly.
        ui.setTouch(point.map(|sample| (sample.x, sample.y)));
        ui.pump(20);
        if !spaceJoinUi.isBusy() {
            deviceSpaceUi.poll();
            ui.setDeviceSpaceState(&deviceSpaceUi.snapshot());
        }
        spaceJoinUi.pump(&mut ui);
        crate::device_services::pumpUi(&mut ui);
        for mut action in ui.drainActions() {
            if spaceJoinUi.action(&action, &mut ui) {
                if action == "edge_space_leave" { deviceSpaceUi.reset(); }
                continue;
            }
            if deviceSpaceUi.action(&action) {
                continue;
            }
            if action == "edge_scene_exit" {
                sceneService.system_exit();
                continue;
            }
            if let Some(point) = action.strip_prefix("edge_scene_touch:") {
                let target = point.split_once(':').and_then(|(x, y)| {
                    sceneService.touch(x.parse::<u16>().ok()?, y.parse::<u16>().ok()?)
                });
                match target.as_deref() {
                    Some("pet" | "treat") => {
                        sceneService.advance_target_frame(target.as_deref().unwrap());
                        continue;
                    }
                    Some(target) => action = target.to_owned(),
                    None => continue,
                }
            }
            match action.as_str() {
                "face_online" => {
                    setExpression("online")?;
                }
                "face_neutral" => {
                    setExpression("neutral")?;
                }
                "run_node" | "edge_search" | "edge_pair" | "edge_unpair" => {
                    let unpair = action == "edge_unpair";
                    let result = (|| -> Result<(), String> {
                        let services = nodeServices.clone();
                        let (sender, receiver) = std::sync::mpsc::channel();
                        let transports = if unpair { Vec::new() } else { peerTransports.clone() };
                        scheduler.scheduleHostRuntimeAsyncTask("edge-peer-ui", Box::new(move || {
                            Box::pin(async move {
                                let result = async {
                                    let peers = services.peers();
                                    if unpair {
                                        for peer in peers.pairedPeers().map_err(|error| error.to_string())? {
                                            peers.removePairedPeer(&peer.nodeId).await
                                                .map_err(|error| error.to_string())?;
                                        }
                                    } else {
                                        peers.startListening(&transports).await
                                            .map_err(|error| error.to_string())?;
                                    }
                                    Ok::<_, String>(())
                                }.await;
                                let _ = sender.send(result);
                            })
                        })).map_err(|error| error.to_string())?;
                        receiver.recv_timeout(std::time::Duration::from_secs(5))
                            .map_err(|error| format!("Host task timeout: {error}"))?
                    })();
                    match result {
                        Ok(()) if unpair => {
                            deviceSpaceUi.reset();
                            chatRoute.reset();
                            status.setPairingCode("");
                            setExpression("neutral")?;
                        }
                        Ok(()) => setExpression("listening")?,
                        Err(error) => ui.actionError(&error),
                    }
                }
                "edge_chat" => {}
                "edge_new" => {
                    if let Err(error) = crate::edge_chat::newChat() {
                        ui.actionError(&error);
                    }
                }
                "edge_voice" => {
                    if let Err(error)=audio::toggleRecording() {ui.actionError(&error);}
                }
                "edge_send" => {
                    let draft = ui.chatDraft();
                    if let Err(error) = crate::edge_chat::send(draft) {
                        ui.chatSendResult(Err(error));
                    }
                }
                _ => {}
            }
        }

        {
            audio::pump();
            if let Some(result)=audio::takeResult() {
                if let Err(error)=result {ui.actionError(&error);}
            }
            // Scalar speech state remains responsive while native audio owns the
            // device work guard. Defer chat materialization until that guard releases.
            let (phase, result) = speech.takeNativeUpdate(&mut lastSpeechResult);
            ui.setSpeechState(phase);
            if result.is_some() { pendingSpeechResult = result; }
            if !spaceJoinUi.isBusy() && !deviceSpaceUi.isBusy() {
                if let Some(result)=pendingSpeechResult.take() {
                    match result {
                        Ok(text)=>if let Err(error)=crate::edge_chat::send(text) {ui.actionError(&error);},
                        Err(error)=>ui.actionError(&error),
                    }
                }
            }
            if let Some(result) = crate::edge_chat::takeSendResult() {
                ui.chatSendResult(result);
            }
        }
        if let Ok(face) = faceHost.getExpression() {
            if face.expression != lastExpression {
                lastExpression = face.expression.clone();
                status.setExpression(face.expression);
            }
        }
        // Connection status is presentation state: poll it with route refresh,
        // not twice per rendered frame. Every routed operation still validates
        // its current Space entry and authorization before dispatch.
        let mut edgeReady = lastEdgeReady;
        let spaceStateBusy = spaceJoinUi.isBusy() || deviceSpaceUi.isBusy();
        // A review/merge can hold a source and a target snapshot. Display-only
        // checks must not allocate another policy replay beside that work.
        // Keep the previous indicators until the shared operation completes.
        // Poll first so a newly accepted Core carrier is reflected on the
        // display in the same loop iteration.
        if !spaceStateBusy && std::time::Instant::now() >= nextRoutePoll {
            nextRoutePoll = std::time::Instant::now() + std::time::Duration::from_millis(500);

            chatRoute.poll(nodeServices.clone());
            edgeReady = crate::edge_chat::isConnected();
        }
        let mut pairingChanged = false;
        loop {
            match peerChanges.try_recv() {
                Ok(()) | Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                    pairingChanged = true
                }
                Err(_) => break,
            }
        }
        let now = std::time::Instant::now();
        // Changes are immediate; a slow fallback poll also expires displayed codes.
        // Never deserialize the entire node database every 20ms UI frame.
        let paired = if !spaceStateBusy && (pairingChanged || now >= nextPairingPoll) {
            nextPairingPoll = now + std::time::Duration::from_secs(1);
            match pairingState() {
                Ok((paired, codes)) => {
                    status.setPairingCode(codes);
                    paired
                }
                Err(_) => {
                    status.setPairingCode("");
                    lastPaired
                }
            }
        } else {
            lastPaired
        };
        let statusRevision = status.revision();
        if statusRevision != lastStatusRevision
            || edgeReady != lastEdgeReady
            || paired != lastPaired
        {
            updateStatus(&mut ui, &status, edgeReady, paired);
            lastStatusRevision = statusRevision;
            lastEdgeReady = edgeReady;
            lastPaired = paired;
        }

        {
            let chatRevision = crate::edge_chat::revision();
            let chatConnected = edgeReady;
            if chatRevision != lastChatRevision || chatConnected != lastChatConnected {
                let chatState = crate::edge_chat::snapshot();
                ui.setChatState(&chatState);
                ui.setChatScreen(crate::edge_chat::screenTextFromSnapshot(&chatState));
                ui.setChatTask(&crate::edge_chat::taskStatus());
                lastChatRevision = chatRevision;
                lastChatConnected = chatConnected;
            }
        }
        // Health is now polled via the UART mailbox under the Link write lock.
        // Unsolicited console telemetry can interleave with a binary OPS1 frame.
        // Inspect/click the REAL display state only on its owning UI thread.
        ui.pollSerialDebug(&serialDebug);
        FreeRtos::delay_ms(1);
    }
}

/// Bounded diagnostics: no history buffer or framebuffer copies.
#[cfg(target_os = "espidf")]
pub(crate) fn runtimeHealthSnapshot() -> serde_json::Value {
    use esp_idf_svc::sys;
    unsafe {
        let mut stats = sys::nvs_stats_t::default();
        let nvs = if sys::nvs_get_stats(std::ptr::null(), &mut stats) == sys::ESP_OK as i32 {
            serde_json::json!({"usedEntries": stats.used_entries, "freeEntries": stats.free_entries,
                "availableEntries": stats.available_entries, "totalEntries": stats.total_entries})
        } else { serde_json::Value::Null };
        serde_json::json!({
            "uptimeMs": sys::esp_timer_get_time() / 1000,
            "resetReason": sys::esp_reset_reason(),
            "previousAllocationFailure": PREVIOUS_ALLOCATION_FAILURE.get().copied().flatten().map(|(size, caps)| serde_json::json!({"bytes": size, "caps": caps})),
            "heapFreeBytes": sys::esp_get_free_heap_size(),
            "heapMinimumFreeBytes": sys::esp_get_minimum_free_heap_size(),
            "largest8bitBlockBytes": sys::heap_caps_get_largest_free_block(sys::MALLOC_CAP_8BIT),
            // This is the task servicing the request, not always the UI/main task.
            "currentTaskStackHighWaterMark": sys::uxTaskGetStackHighWaterMark(std::ptr::null_mut()),
            "nvs": nvs,
            "serialFrameCounters": operit_peer_link::serialFrameCounters(),
            "peerSessionError": operit_edge_sdk::runtime::HostRuntimePeerService::peerSessionDiagnostic(),
        })
    }
}
