#![allow(non_snake_case)]
// Compile the actual firmware modules, including the same stream renderer.
mod core_access;
#[path = "../../../src/device_preferences.rs"]
mod device_preferences;
#[path = "../../../src/device_space_ui.rs"]
mod device_space_ui;
#[path = "../../../src/device_speech.rs"]
mod device_speech;
#[cfg(test)]
#[path = "../../../src/wav.rs"]
mod wav;
#[path = "../../../src/edge_chat.rs"]
mod edge_chat;
mod ipc;
mod memory;
#[path = "../../../src/runtime_storage_codec.rs"]
mod runtime_storage_codec;
#[path = "../../../src/runtime_storage_host.rs"]
mod runtime_storage_host;
#[path = "../../../src/runtime_storage_pack.rs"]
mod runtime_storage_pack;
mod send_receipt;
#[path = "../../../src/space_work.rs"]
mod space_work;
#[path = "../../../src/standalone_chat.rs"]
mod standalone_chat;
mod storage;
mod storage_pressure;
use operit_link::protocol::LinkDeviceInfo;
use operit_edge_sdk::device::PeerRouter::EdgePeerRouter;
use operit_edge_sdk::runtime::{
    NodeServices::{NodeServices, PeerTransport},
    PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore},
};
use std::sync::{Arc, Mutex};

/// Routes declared hardware calls through the shared device Host contracts.
fn nativeCallHandler(
    storage: Arc<dyn operit_host_api::RuntimeStorageHost>,
    nodeId: String,
    address: String,
    scene: Arc<operit_edge_sdk::scene::SceneService>,
) -> Arc<operit_edge_sdk::device::EdgeCallHandler> {
    Arc::new(move |request: operit_link::CoreCallRequest| {
        let address = address.clone();
        let scene = scene.clone();
        let storage = storage.clone();
        let nodeId = nodeId.clone();
        Box::pin(async move {
            if let Some(response) = device_preferences::call(storage, &nodeId, &request) {
                return Some(response);
            }
            let result = match request.target.as_str() {
                "edge.status" if request.methodName == "read" => {
                    operit_link::toCoreValue(serde_json::json!({
                        "boardId":"ESP32-2432S028-SIM", "expression":"online",
                        "ipv4":address, "wifiSsid":"simulator",
                    }))
                    .map_err(|error| operit_link::CoreLinkError::internal(error.to_string()))
                }
                "edge.scene" => scene
                    .call(&request.methodName, request.args)
                    .map_err(operit_link::CoreLinkError::internal),
                _ => return None,
            };
            Some(operit_link::CoreCallResponse {
                requestId: request.requestId,
                result,
            })
        })
    })
}

/// Schedules Space mutations on the firmware-equivalent worker and returns their actual result.
async fn onSpaceWorker<R: Send + 'static>(
    scheduler: Arc<dyn operit_host_api::HostRuntimeTaskSchedulerHost>,
    name: &str,
    task: impl FnOnce() -> std::pin::Pin<Box<dyn std::future::Future<Output = R>>> + Send + 'static,
) -> Result<R, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    scheduler
        .scheduleHostRuntimeBlockingAsyncTask(
            name,
            Box::new(move || {
                Box::pin(async move {
                    let _work = space_work::acquire().await;
                    let result = task().await;
                    let _ = sender.send(result);
                })
            }),
        )
        .map_err(|e| e.to_string())?;
    receiver
        .await
        .map_err(|_| "Space worker stopped without a result".into())
}

/// Boots the simulator on the firmware main-task budget after isolated limit probes.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = memory::probe(&std::env::args().skip(1).collect::<Vec<_>>()) {
        std::process::exit(code);
    }
    operit_util::AppLogger::AppLogger::set_retain_entries(false);
    operit_host_api::setHostConsoleLogSink(Arc::new(|_, _, line| {
        eprint!("{line}");
    }));
    let _ = operit_host_api::TimeUtils::currentLocalDateTime();
    let scheduler =
        Arc::new(operit_host_native_scheduler::LocalHostRuntimeTaskSchedulerHost::new()?);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_io_events_per_tick(16)
        .max_blocking_threads(1)
        .build()?;
    // Initialize the desktop socket driver before modeled device execution.
    drop(runtime.block_on(tokio::net::TcpListener::bind((
        std::net::Ipv4Addr::LOCALHOST,
        0,
    )))?);
    let backend = storage::FileSnapshotBackend::open(storage::stateDirectory())?;
    let (mut device, profile) =
        operit_host_native_scheduler::withStackBudget(memory::MAIN_STACK_BYTES, || {
            initialize(scheduler, backend)
        })??;
    operit_host_native_scheduler::withStackBudget(memory::MAIN_STACK_BYTES, || {
        device.spaceService.initialize(profile)
    })??;
    let network = Box::pin(device.start());
    operit_host_native_scheduler::withStackBudget(memory::MAIN_STACK_BYTES, || {
        runtime.block_on(network)
    })??;
    let device: std::pin::Pin<Box<dyn std::future::Future<Output = _>>> = Box::pin(run(device));
    operit_host_native_scheduler::withStackBudget(memory::MAIN_STACK_BYTES, || {
        runtime.block_on(device)
    })?
}

// The default simulator is a real TCP Edge node. Its UI still talks over IPC,
// but pairing uses the same HostRuntimePeerService as the ESP32 firmware.
/// Initializes device services before entering the main asynchronous event loop.
fn initialize(
    scheduler: Arc<operit_host_native_scheduler::LocalHostRuntimeTaskSchedulerHost>,
    backend: storage::FileSnapshotBackend,
) -> Result<
    (
        Simulator,
        operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceDeviceProfile,
    ),
    Box<dyn std::error::Error>,
> {
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(scheduler.clone());
    let error = Arc::new(Mutex::new(String::new()));
    let lastAction = Arc::new(Mutex::new(String::new()));
    let configuredAddress =
        std::env::var("OPERIT_SIM_BIND").unwrap_or_else(|_| "0.0.0.0:18765".into());
    let storageControl = backend.control();
    let storage = storage::DeviceStorage::open(backend)?;
    let discoveryEnabled = std::env::var("OPERIT_SIM_DISCOVERY")
        .map_or(true, |value| value != "false" && value != "0");
    let discovery = if discoveryEnabled {
        let address = std::env::var("OPERIT_SIM_DISCOVERY_IP")?.parse::<std::net::IpAddr>()?;
        if address.is_unspecified() {
            return Err("Simulator discovery requires one explicit interface IP".into());
        }
        operit_host_native_common::ServiceDiscovery::ServiceDiscoveryProvider::forInterface(address)
    } else {
        operit_host_native_common::ServiceDiscovery::ServiceDiscoveryProvider::default()
    };
    let hostManager = operit_host_api::HostManager::HostManager::default()
        .withTcpHost(Arc::new(operit_host_native_common::NativeTcpHost))
        .withHttpHost(Arc::new(
            operit_host_native_common::NativeHttpHost::default(),
        ))
        .withServiceDiscoveryHost(Arc::new(discovery))
        .withRuntimeStorageHost(storage.clone())
        .withHostRuntimeTaskSchedulerHost(scheduler.clone());
    let identityStore =
        operit_edge_sdk::storage::CoreNodeIdentityStore::CoreNodeIdentityStore::new(storage.clone());
    let nodeId =
        std::env::var("OPERIT_SIM_NODE_ID").unwrap_or_else(|_| "esp32-edge-simulator".into());
    identityStore.writeNodeId(nodeId.clone())?;
    let nodeId = identityStore.initialize()?.nodeId;
    let deviceInfo = LinkDeviceInfo {
        platform: "esp32".into(),
        model: "ESP32-2432S028-SIM".into(),
        pairingIncarnation: None,
    };
    let edgeRouter = EdgePeerRouter::new(nodeId.clone());
    let sceneService = Arc::new(
        operit_edge_sdk::scene::SceneService::new(320, 240, 24, true).with_storage(storage.clone())?,
    );
    let speech = Arc::new(device_speech::DeviceSpeech::default());
    let (speechOutput, speechInput) = speech.handlers();
    let mut edgeNode = operit_edge_sdk::device::EdgeNode::fromHostManager(hostManager.clone())
        .withCallHandler(nativeCallHandler(
            storage.clone(),
            nodeId.clone(),
            configuredAddress.clone(),
            sceneService.clone(),
        ))
        .withStreamHandlers(speechOutput, speechInput);
    let services = {
        let peers = operit_edge_sdk::device::peer::createPeerService(
            Arc::new(hostManager.clone()),
            &edgeRouter,
            deviceInfo.clone(),
            operit_edge_sdk::runtime::HostRuntimePeerService::PeerRuntimeLimits::constrained(),
        )?;
        let portMode = if configuredAddress.ends_with(":0") {
            PeerHostPortMode::Automatic
        } else {
            PeerHostPortMode::Fixed
        };
        PeerStateStore::new(storage.clone()).saveHostConfig(&PeerHostConfig {
            bindAddress: configuredAddress.clone(),
            transports: vec![PeerTransport::Tcp],
            discoveryEnabled,
            portMode,
            updatedAt: operit_host_api::TimeUtils::currentTimeMillis(),
        })?;
        NodeServices::new(peers)
    };
    let spaceService = operit_edge_sdk::runtime::NodeSpaceService::NodeSpaceService::new(
        storage.clone(),
        services.peers(),
    )?;
    let profile = operit_edge_sdk::storage::CoreSpaceStore::CoreSpaceDeviceProfile {
        nodeId: nodeId.clone(),
        displayName: deviceInfo.model.clone(),
        userName: String::new(),
        platform: deviceInfo.platform.clone(),
        model: deviceInfo.model.clone(),
        coreVersion: None,
        updatedAt: operit_host_api::TimeUtils::currentTimeMillis(),
        pairingIncarnation: None,
    };
    edgeRouter.installSpace(spaceService.clone())?;
    edgeNode = edgeNode.withNodeServices(services.clone());
    let edgeNode = Arc::new(edgeNode);
    edgeRouter.install(edgeNode.clone())?;
    let address = configuredAddress;
    let deviceSpaceUi = device_space_ui::DeviceSpaceUi::new(
        services.clone(),
        spaceService.clone(),
        scheduler.clone(),
    );
    Ok((
        Simulator {
            services,
            spaceService,
            edgeNode,
            _router: edgeRouter,
            sceneService,
            speech,
            hostManager,
            storage,
            storageControl,
            nodeId,
            address,
            error,
            lastAction,
            deviceSpaceUi,
            chatRoute: edge_chat::SpaceChatRoute::default(),
            coreAccess: Arc::new(tokio::sync::Mutex::new(core_access::CoreAccess::default())),
            review: Arc::new(Mutex::new(ReviewState::default())),
            sessionId: uuid::Uuid::new_v4().to_string(),
            sendReceipt: send_receipt::SendReceipt::default(),
        },
        profile,
    ))
}

/// Polls device services without retaining synchronous initialization temporaries on this stack.
async fn run(mut device: Simulator) -> Result<(), Box<dyn std::error::Error>> {
    let mut ipc = ipc::EditorIpc::connect().await?;
    ipc.send(
        serde_json::json!({"ready": true, "peerServiceAvailable": true,
        "memory": memory::snapshot(), "address": device.address, "deviceId": device.nodeId}),
    )
    .await?;
    // Match the firmware's 500 ms Space route polling. Compiling edge_chat
    // alone does not install a session; pairing and approval must not fake one.
    let mut routePoll = tokio::time::interval(std::time::Duration::from_millis(500));
    routePoll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let preferenceSpace = device.spaceService.clone();
    operit_host_api::HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(
        device
            .hostManager
            .hostRuntimeTaskSchedulerHost
            .as_deref()
            .expect("Simulator Host scheduler must be installed"),
        "device-preferences",
        Box::new(move || {
            Box::pin(async move {
                loop {
                    if let Some(_work) = space_work::try_acquire() {
                        let _ = preferenceSpace.synchronizePreferences().await;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            })
        }),
    )?;
    loop {
        let line = tokio::select! {
            _ = routePoll.tick() => { device.poll()?; continue; }
            line = ipc.receive() => match line? { Some(line) => line, None => break },
        };
        match ipc::decode_command(line)? {
            ipc::Command::Send { id, text, send_id } => {
                device.sendReceipt.capture(edge_chat::takeSendResult());
                let result = edge_chat::send(text).map(|_| {
                    device.sendReceipt.begin(send_id.as_deref());
                    serde_json::json!({"ok":true})
                });
                ipc.reply(&id, result).await?;
            }
            ipc::Command::Other(request) => {
                let id = &request["id"];
                if request["command"] == "state" {
                    let state = Box::pin(device.state()).await;
                    ipc.reply(id, state).await?;
                } else {
                    let result = device.command(&request).await;
                    ipc.reply(id, result).await?;
                }
            }
        }
    }
    device
        .services
        .peers()
        .stop()
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Retains only the latest review prompt produced by the shared Space service.
#[derive(Default)]
struct ReviewState {
    pending: Option<operit_edge_sdk::runtime::NodeSpaceService::SpaceJoinRequest>,
    polling: bool,
}

/// Owns persistent device handles separately from the active IPC command future.
struct Simulator {
    services: NodeServices,
    spaceService: Arc<operit_edge_sdk::runtime::NodeSpaceService::NodeSpaceService>,
    edgeNode: Arc<operit_edge_sdk::device::EdgeNode>,
    _router: Arc<EdgePeerRouter>,
    sceneService: Arc<operit_edge_sdk::scene::SceneService>,
    speech: Arc<device_speech::DeviceSpeech>,
    hostManager: operit_host_api::HostManager::HostManager,
    storage: Arc<storage::DeviceStorage>,
    storageControl: storage_pressure::StorageControl,
    nodeId: String,
    address: String,
    error: Arc<Mutex<String>>,
    lastAction: Arc<Mutex<String>>,
    chatRoute: edge_chat::SpaceChatRoute,
    coreAccess: Arc<tokio::sync::Mutex<core_access::CoreAccess>>,
    deviceSpaceUi: device_space_ui::DeviceSpaceUi,
    sessionId: String,
    sendReceipt: send_receipt::SendReceipt,
    review: Arc<Mutex<ReviewState>>,
}

/// Borrows stable identifiers and serializes each device snapshot without an outer JSON tree.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SimulatorState<'a> {
    address: &'a str,
    sessionId: &'a str,
    scene: serde_json::Value,
    speech: serde_json::Value,
    playback: Vec<serde_json::Value>,
    coreAccess: serde_json::Value,
    deviceSpace: serde_json::Value,
    memory: serde_json::Value,
    storage: serde_json::Value,
    deviceId: &'a str,
    paired: bool,
    activePeers: std::collections::BTreeSet<String>,
    peerServiceAvailable: bool,
    pairingCode: String,
    spaceJoinPrompt: String,
    spaceJoinRequestId: String,
    spaceJoinAssignmentVersion: u64,
    error: String,
    lastAction: String,
    plugins: serde_json::Value,
    chat: serde_json::Value,
    chatPreview: String,
    chatScreen: String,
    chatTask: String,
    chatSendResult: Option<&'a serde_json::Value>,
}

impl Simulator {
    /// Activates discovery and listening before the interactive main-loop poll frame exists.
    async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.services
            .peers()
            .startListening(&[PeerTransport::Tcp])
            .await?;
        self.address = PeerStateStore::new(self.storage.clone())
            .hostConfig()?
            .ok_or("Simulator listener configuration is missing")?
            .bindAddress;
        Ok(())
    }

    /// Applies the same periodic route and device-configuration updates as the firmware.
    fn poll(&mut self) -> Result<(), String> {
        let scope = operit_edge_sdk::runtime::NodeSpaceService::NodeSpaceContext::spaceStore(
            self.spaceService.as_ref(),
        )
        .space()?;
        let preferences =
            operit_edge_sdk::storage::DevicePreferencesStore::DevicePreferencesStore::new(self.storage.clone())
                .read(&scope.spaceId, &self.nodeId)?
                .values();
        if let Err(message) = standalone_chat::configure(
            self.hostManager.httpHost.clone(),
            &scope.spaceId,
            preferences.get("chat"),
        ) {
            *self.error.lock().unwrap() = message;
        }
        self.chatRoute.poll(self.services.clone());
        self.deviceSpaceUi.poll();
        self.pollReview()?;
        Ok(())
    }

    /// Runs review discovery on the same bounded Host worker used by firmware.
    fn pollReview(&self) -> Result<(), String> {
        let mut state = self.review.lock().unwrap();
        if state.polling {
            return Ok(());
        }
        state.polling = true;
        drop(state);
        let service = self.spaceService.clone();
        let review = self.review.clone();
        let errors = self.error.clone();
        let scheduled = self
            .hostManager
            .hostRuntimeTaskSchedulerHost
            .as_ref()
            .expect("Simulator Host scheduler must be installed")
            .scheduleHostRuntimeBlockingAsyncTask(
                "edge-space-incoming",
                Box::new(move || {
                    Box::pin(async move {
                        let _work = space_work::acquire().await;
                        let result = service.incomingDeviceSpaceJoins().await;
                        let mut state = review.lock().unwrap();
                        state.polling = false;
                        match result {
                            Ok(requests) => {
                                state.pending =
                                    requests.into_iter().find(|request| request.canApprove)
                            }
                            Err(error) => *errors.lock().unwrap() = error,
                        }
                    })
                }),
            );
        if let Err(error) = scheduled {
            self.review.lock().unwrap().polling = false;
            return Err(error.to_string());
        }
        Ok(())
    }

    /// Collects device views once and borrows stable fields until their IPC serialization.
    async fn state(&mut self) -> Result<SimulatorState<'_>, String> {
        let Self {
            sceneService,
            speech,
            nodeId,
            address,
            error,
            lastAction,
            coreAccess,
            deviceSpaceUi,
            sessionId,
            sendReceipt,
            ..
        } = self;
        let services = &self.services;

        let paired = !services
            .peers()
            .pairedPeers()
            .map_err(|e| e.to_string())?
            .is_empty();
        let mut playback = Vec::new();
        // One bounded audio block per observation. Draining the whole playback
        // queue here would duplicate several blocks as base64 plus the IPC JSON
        // buffer and exhaust the same device heap we are trying to simulate.
        if let Some(packet) = speech.takePlayback() {
            let mut packet = match packet {
                operit_link::CoreValue::Map(p) => p,
                _ => unreachable!(),
            };
            if let Some(operit_link::CoreValue::Bytes(bytes)) = packet.remove("audio") {
                use base64::Engine;
                packet.insert(
                    "audio".into(),
                    operit_link::CoreValue::String(
                        base64::engine::general_purpose::STANDARD.encode(bytes),
                    ),
                );
            }
            playback.push(
                operit_link::fromCoreValue::<serde_json::Value>(operit_link::CoreValue::Map(
                    packet,
                ))
                .map_err(|e| e.to_string())?,
            );
        }
        let chat_state = edge_chat::snapshot();
        let chat_screen = edge_chat::screenTextFromSnapshot(&chat_state);
        let chat_preview = edge_chat::previewFromSnapshot(&chat_state);
        sendReceipt.capture(edge_chat::takeSendResult());
        let pairing_code = services
            .peers()
            .pairingPrompts()
            .map_err(|e| e.to_string())?
            .into_iter()
            // Match firmware: one digits-only code, never a device-name prefix.
            .map(|p| p.confirmationCode)
            .find(|code| code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit()))
            .unwrap_or_default();
        let space_join = self.review.lock().unwrap().pending.clone();
        Ok(SimulatorState {
            address,
            sessionId,
            storage: self.storageControl.snapshot(),
            scene: sceneService.summary(),
            speech: speech.snapshot(),
            playback,
            coreAccess: coreAccess.lock().await.snapshot(),
            deviceSpace: deviceSpaceUi.snapshot(),
            memory: memory::snapshot(),
            deviceId: nodeId,
            paired,
            activePeers: services
                .peers()
                .activePeerNodeIds()
                .map_err(|e| e.to_string())?,
            peerServiceAvailable: true,
            pairingCode: pairing_code,
            spaceJoinPrompt: space_join
                .as_ref()
                .map(|request| {
                    format!(
                        "申请加入空间\n{}\n{}",
                        request.applicantName.chars().take(32).collect::<String>(),
                        request.spaceName.chars().take(32).collect::<String>(),
                    )
                })
                .unwrap_or_default(),
            spaceJoinRequestId: space_join
                .as_ref()
                .map(|request| request.requestId.clone())
                .unwrap_or_default(),
            spaceJoinAssignmentVersion: space_join
                .as_ref()
                .map(|request| request.assignmentVersion)
                .unwrap_or(0),
            error: error.lock().unwrap().clone(),
            lastAction: lastAction.lock().unwrap().clone(),
            plugins: edge_chat::pluginsSnapshot(),
            chat: chat_state,
            chatPreview: chat_preview,
            chatScreen: chat_screen,
            chatTask: edge_chat::taskStatus(),
            chatSendResult: sendReceipt.snapshot(),
        })
    }

    /// Executes one editor command before its result enters the IPC serializer.
    fn command<'a>(
        &'a mut self,
        request: &'a serde_json::Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + 'a>>
    {
        Box::pin(async move {
            let Self {
                spaceService,
                edgeNode,
                sceneService,
                speech,
                nodeId,
                address,
                error,
                lastAction,
                chatRoute,
                coreAccess,
                deviceSpaceUi,
                sessionId,
                sendReceipt,
                review,
                ..
            } = self;
            let scheduler = self
                .hostManager
                .hostRuntimeTaskSchedulerHost
                .as_ref()
                .expect("Simulator Host scheduler must be installed")
                .clone();
            let services = edgeNode.nodeServices();
            match request["command"].as_str() {
                Some("coreAccess") => match services.as_ref() {
                    Ok(services) => {
                        let access = coreAccess.clone();
                        let input = request.clone();
                        let services = services.clone();
                        let space = spaceService.clone();
                        onSpaceWorker(scheduler.clone(), "simulator-core-access", move || {
                            Box::pin(async move {
                                access.lock().await.handle(&input, &services, &space).await
                            })
                        })
                        .await?
                    }
                    Err(error) => Err(error.message.clone()),
                },
                Some("sceneView") => Ok(sceneService.visual_snapshot()),
                Some("memory") => Ok(memory::snapshot()),
                Some("storageControl") => self.storageControl.configure(request["configuration"].clone()).map_err(|e| e.to_string()),
                Some("storageRecords") => {
                    use operit_host_api::RuntimeStorageHost;
                    let records = self.storage.list("").map_err(|e| e.to_string())?;
                    serde_json::to_value(
                        records
                            .into_iter()
                            .map(|entry| entry.path)
                            .collect::<Vec<_>>(),
                    )
                    .map_err(|e| e.to_string())
                }
                // Decision/cleanup failures belong to this RPC, not to run(). In
                // particular a stale approval tap after cancellation must not
                // terminate the device or drop its authenticated TCP listener.
                Some("action") => {
                    async {
                        let action = request["action"].as_str().unwrap_or("");
                        *lastAction.lock().unwrap() = action.to_string();
                        if deviceSpaceUi.action(action) {
                            // The same bounded controller is used on actual firmware.
                        } else if action == "edge_scene_exit" {
                            sceneService.system_exit();
                        } else if let Some(point) = action.strip_prefix("edge_scene_touch:") {
                            let (x, y) = point.split_once(':').ok_or("Invalid scene point")?;
                            if let Some(target) = sceneService.touch(
                                x.parse().map_err(|_| "Invalid scene x")?,
                                y.parse().map_err(|_| "Invalid scene y")?,
                            ) {
                                if matches!(target.as_str(), "pet" | "treat") {
                                    sceneService.advance_target_frame(&target);
                                }
                            }
                        } else if action == "edge_space_approve" || action == "edge_space_reject" {
                            // Match firmware's captured pending request/version. A stale
                            // screen must never approve a different, newer submission.
                            let requestId = request["requestId"]
                                .as_str()
                                .filter(|id| !id.trim().is_empty())
                                .ok_or_else(|| "Missing Space review requestId".to_string())?;
                            let assignmentVersion =
                                request["assignmentVersion"].as_u64().ok_or_else(|| {
                                    "Missing Space review assignmentVersion".to_string()
                                })?;
                            let approve = action == "edge_space_approve";
                            let space = spaceService.clone();
                            let requestId = requestId.to_owned();
                            let decided = onSpaceWorker(
                                scheduler.clone(),
                                "edge-space-decision",
                                move || {
                                    Box::pin(async move {
                                        space
                                            .decideDeviceSpaceJoin(
                                                requestId,
                                                assignmentVersion,
                                                approve,
                                            )
                                            .await
                                    })
                                },
                            )
                            .await??;
                            let mut review = review.lock().unwrap();
                            if review.pending.as_ref().is_some_and(|pending| {
                                pending.requestId == decided.requestId
                                    && pending.assignmentVersion == decided.assignmentVersion
                            }) {
                                review.pending = None;
                            }
                        } else if action.starts_with("edge_plugins_")
                            || action.starts_with("edge_plugin_")
                        {
                            edge_chat::pluginsAction(action)?;
                        } else if action == "edge_pair" || action == "edge_unpair" {
                            if action == "edge_unpair" {
                                *coreAccess.lock().await = core_access::CoreAccess::default();
                            }
                            let result = async {
                                if action == "edge_pair" {
                                    let services =
                                        edgeNode.nodeServices().map_err(|error| error.message)?;
                                    services
                                        .peers()
                                        .startListening(&[PeerTransport::Tcp])
                                        .await
                                        .map_err(|e| e.to_string())?;
                                } else {
                                    // Standalone simulator runs without a core runtime service.
                                    // Unpair is intentionally idempotent in that mode.
                                    if let Ok(services) = edgeNode.nodeServices() {
                                        for peer in services
                                            .peers()
                                            .pairedPeers()
                                            .map_err(|e| e.to_string())?
                                        {
                                            services
                                                .peers()
                                                .removePairedPeer(&peer.nodeId)
                                                .await
                                                .map_err(|e| e.to_string())?;
                                        }
                                    }
                                    deviceSpaceUi.reset();
                                    chatRoute.reset();
                                }
                                Ok::<_, String>(())
                            }
                            .await;
                            result?;
                        } else if action == "edge_space_leave" {
                            let space = spaceService.clone();
                            onSpaceWorker(scheduler.clone(), "edge-space-leave", move || {
                                Box::pin(async move { space.leaveDeviceSpace() })
                            })
                            .await??;
                            deviceSpaceUi.reset();
                            *coreAccess.lock().await = core_access::CoreAccess::default();
                            chatRoute.reset();
                        } else if action == "edge_history_older" || action == "edge_history_newer" {
                            edge_chat::moveHistory(action == "edge_history_older")?;
                        } else if action == "edge_conversations_refresh" {
                            edge_chat::conversationsAction()?;
                        } else if action == "edge_new" {
                            edge_chat::newChat()?;
                        } else if let Some(chatId) = action.strip_prefix("edge_select:") {
                            edge_chat::selectChat(chatId)?;
                        } else {
                            return Err("Unknown simulator action".to_string());
                        }
                        Ok::<_, String>(serde_json::json!({"ok": true, "action": action}))
                    }
                    .await
                }
                Some("speech") => async {
                    if let Some(id) = request["requestId"].as_str() {
                        if speech.snapshot()["requestId"].as_str() != Some(id) {
                            return Err("Speech request expired".into());
                        }
                    }
                    match request["operation"].as_str().unwrap_or("") {
                        "begin" => speech
                            .beginRecognition()
                            .map(|id| serde_json::json!({"requestId":id})),
                        "pcm" => {
                            use base64::Engine;
                            let snapshot = speech.snapshot();
                            if snapshot["requestId"].as_str() != request["requestId"].as_str() {
                                return Err("Speech capture changed".into());
                            }
                            let pcm = base64::engine::general_purpose::STANDARD
                                .decode(request["audio"].as_str().unwrap_or(""))
                                .map_err(|e| e.to_string())?;
                            speech
                                .appendPcm(pcm).await
                                .map(|_| serde_json::json!({"ok":true}))
                        }
                        "finish" => speech
                            .finishRecognition().await
                            .map(|_| serde_json::json!({"ok":true})),
                        "cancel" => {
                            speech.cancel(
                                request["error"].as_str().unwrap_or("已取消语音任务").into(),
                            );
                            Ok(serde_json::json!({"ok":true}))
                        }
                        "tts" => speech
                            .synthesize(request["text"].as_str().unwrap_or("").into())
                            .map(|id| serde_json::json!({"requestId":id})),
                        _ => Err("Unknown speech operation".into()),
                    }
                }.await,
                Some("sendImage") => (|| {
                    use base64::Engine;
                    let encoded = request["bytes"].as_str().unwrap_or("");
                    // Base64's Vec decoder reserves this upper bound, including padding.
                    // Check BEFORE allocating. Errors stay within this RPC, not main().
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .map_err(|_| "Invalid image bytes".to_string())?;
                    sendReceipt.capture(edge_chat::takeSendResult());
                    edge_chat::sendImage(bytes, request["mimeType"].as_str().unwrap_or("").into())
                        .map(|_| {
                            sendReceipt.begin(request["sendId"].as_str());
                            serde_json::json!({"accepted":true})
                        })
                })(),
                Some("send") => {
                    sendReceipt.capture(edge_chat::takeSendResult());
                    edge_chat::send(request["text"].as_str().unwrap_or("").into()).map(|_| {
                        sendReceipt.begin(request["sendId"].as_str());
                        serde_json::json!({"ok": true})
                    })
                }
                _ => Err("Unknown simulator command".into()),
            }
        })
    }
}
