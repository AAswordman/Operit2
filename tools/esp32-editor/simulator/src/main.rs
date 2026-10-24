#![allow(non_snake_case)]
// Compile the actual firmware modules, including the same stream renderer.
#[path = "../../../../apps/esp32/src/device_speech.rs"]
mod device_speech;
#[path = "../../../../apps/esp32/src/standalone_chat.rs"]
mod standalone_chat;
#[path = "../../../../apps/esp32/src/device_preferences.rs"]
mod device_preferences;
#[path = "../../../../apps/esp32/src/edge_chat.rs"]
mod edge_chat;
mod memory;
mod core_access;
mod send_receipt;
#[path = "../../../../apps/esp32/src/device_space_ui.rs"]
mod device_space_ui;
use operit_link::protocol::LinkDeviceInfo;
use operit_node_edge::PeerRouter::EdgePeerRouter;
use operit_node_runtime::{
    NodeServices::{NodeServices, PeerTransport},
    PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore},
};
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncBufReadExt, BufReader};

fn nativeCallHandler(storage: Arc<dyn operit_host_api::RuntimeStorageHost>, nodeId: String, address: String, scene: Arc<operit_edge_scene::SceneService>) -> Arc<operit_node_edge::EdgeCallHandler> {
    Arc::new(move |request: operit_link::CoreCallRequest| {
        let address = address.clone();
        let scene = scene.clone();
        let storage = storage.clone();
        let nodeId = nodeId.clone();
        Box::pin(async move {
            if let Some(response) = device_preferences::call(storage, &nodeId, &request) { return Some(response); }
            let result = match request.target.as_str() {
                "edge.status" if request.methodName == "read" =>
                    operit_link::toCoreValue(serde_json::json!({
                        "boardId":"ESP32-2432S028-SIM", "expression":"online",
                        "ipv4":address, "wifiSsid":"simulator",
                    })).map_err(|error| operit_link::CoreLinkError::internal(error.to_string())),
                "edge.scene" => scene.call(&request.methodName, request.args)
                    .map_err(operit_link::CoreLinkError::internal),
                _ => return None,
            };
            Some(operit_link::CoreCallResponse { requestId: request.requestId, result })
        })
    })
}

fn emit(value: serde_json::Value) {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{value}").expect("editor IPC closed");
    stdout.flush().expect("editor IPC closed");
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(None).await
}

// The default simulator is a real TCP Edge node. Its UI still talks over IPC,
// but pairing uses the same HostRuntimePeerService as the ESP32 firmware.
async fn run(nodeServices: Option<NodeServices>) -> Result<(), Box<dyn std::error::Error>> {
    let scheduler =
        Arc::new(operit_host_native_scheduler::LocalHostRuntimeTaskSchedulerHost::new()?);
    operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(scheduler.clone());
    let error = Arc::new(Mutex::new(String::new()));
    let lastAction = Arc::new(Mutex::new(String::new()));
    let configuredAddress =
        std::env::var("OPERIT_SIM_BIND").unwrap_or_else(|_| "0.0.0.0:18765".into());
    let stateDir = std::env::var_os("OPERIT_SIM_STATE_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("OPERIT_SIM_STATE")
                .and_then(|path| PathBuf::from(path).parent().map(PathBuf::from))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("operit2-esp32-simulator"));
    std::fs::create_dir_all(&stateDir)?;
    let storage = Arc::new(operit_host_native_common::NativeRuntimeStorageHost::new(
        stateDir.join("runtime"),
        stateDir.join("workspaces"),
    ));
    let hostManager = operit_host_api::HostManager::HostManager::default()
        .withTcpHost(Arc::new(operit_host_native_common::NativeTcpHost))
        .withHttpHost(Arc::new(operit_host_native_common::NativeHttpHost::default()))
        .withServiceDiscoveryHost(Arc::new(
            operit_host_native_common::ServiceDiscovery::ServiceDiscoveryProvider::default(),
        ))
        .withRuntimeStorageHost(storage.clone())
        .withHostRuntimeTaskSchedulerHost(scheduler.clone());
    let identityStore = operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore::new(
        storage.clone(),
    );
    let nodeId = std::env::var("OPERIT_SIM_NODE_ID")
        .unwrap_or_else(|_| "esp32-edge-simulator".into());
    identityStore.writeNodeId(nodeId.clone())?;
    let nodeId = identityStore.initialize()?.nodeId;
    let deviceInfo = LinkDeviceInfo {
        platform: "esp32".into(),
        model: "ESP32-2432S028-SIM".into(),
    pairingIncarnation: None,
};
    let edgeRouter = EdgePeerRouter::new(nodeId.clone());
    let sceneService = Arc::new(operit_edge_scene::SceneService::new(320, 240, 24, true).with_storage(storage.clone())?);
    let speech = Arc::new(device_speech::DeviceSpeech::default());
    let (speechOutput, speechInput) = speech.handlers();
    let mut edgeNode = operit_node_edge::EdgeNode::fromHostManager(hostManager.clone())
        .withCallHandler(nativeCallHandler(storage.clone(), nodeId.clone(), configuredAddress.clone(), sceneService.clone()))
        .withStreamHandlers(speechOutput, speechInput);
    let (services, startListener) = match nodeServices {
        Some(services) => (services, false),
        None => {
            let peers = operit_node_edge::peer::createPeerService(
                Arc::new(hostManager.clone()),
                &edgeRouter,
                deviceInfo.clone(),
                operit_node_runtime::HostRuntimePeerService::PeerRuntimeLimits::constrained(),
            )?;
            let portMode = if configuredAddress.ends_with(":0") {
                PeerHostPortMode::Automatic
            } else {
                PeerHostPortMode::Fixed
            };
            PeerStateStore::new(storage.clone()).saveHostConfig(&PeerHostConfig {
                bindAddress: configuredAddress.clone(),
                transports: vec![PeerTransport::Tcp],
                discoveryEnabled: std::env::var("OPERIT_SIM_DISCOVERY")
                    .map_or(true, |value| value != "false" && value != "0"),
                portMode,
                updatedAt: operit_host_api::TimeUtils::currentTimeMillis(),
            })?;
            (NodeServices::new(peers), true)
        }
    };
    let spaceService = operit_node_runtime::NodeSpaceService::NodeSpaceService::new(
        storage.clone(),
        services.peers(),
    )?;
    spaceService.initialize(operit_store::CoreSpaceStore::CoreSpaceDeviceProfile {
        nodeId: nodeId.clone(),
        displayName: deviceInfo.model.clone(),
        userName: String::new(),
        platform: deviceInfo.platform.clone(),
        model: deviceInfo.model.clone(),
        coreVersion: None,
        updatedAt: operit_host_api::TimeUtils::currentTimeMillis(),
    pairingIncarnation: None,
})?;
    edgeRouter.installSpace(spaceService.clone())?;
    edgeNode = edgeNode.withNodeServices(services.clone());
    let edgeNode = Arc::new(edgeNode);
    edgeRouter.install(edgeNode.clone())?;
    if startListener {
        services.peers().startListening(&[PeerTransport::Tcp]).await?;
    }
    let address = PeerStateStore::new(storage.clone())
        .hostConfig()?
        .map(|config| config.bindAddress)
        .unwrap_or(configuredAddress);
    emit(
        serde_json::json!({"ready": true, "peerServiceAvailable": true, "memory": memory::snapshot(), "address": address, "deviceId": nodeId}),
    );
    // Match the firmware's 500 ms Space route polling. Compiling edge_chat
    // alone does not install a session; pairing and approval must not fake one.
    let mut routePoll = tokio::time::interval(std::time::Duration::from_millis(500));
    routePoll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let preferenceSpace = spaceService.clone();
    operit_host_api::HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(scheduler.as_ref(), "device-preferences", Box::new(move || Box::pin(async move {
        loop {
            let _ = preferenceSpace.synchronizePreferences().await;
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    })))?;
    let mut chatRoute = edge_chat::SpaceChatRoute::default();
    let mut coreAccess = core_access::CoreAccess::default();
    let mut deviceSpaceUi = device_space_ui::DeviceSpaceUi::new(services.clone(), spaceService.clone(), scheduler.clone());
    let sessionId = uuid::Uuid::new_v4().to_string();
    let mut sendReceipt = send_receipt::SendReceipt::default();
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        let line = tokio::select! {
            _ = routePoll.tick() => {
                let scope = operit_node_runtime::NodeSpaceService::NodeSpaceContext::spaceStore(spaceService.as_ref()).space()?;
                let preferences = operit_store::DevicePreferencesStore::DevicePreferencesStore::new(storage.clone()).read(&scope.spaceId, &nodeId)?.values();
                if let Err(message) = standalone_chat::configure(hostManager.httpHost.clone(), &scope.spaceId, preferences.get("chat")) {
                    *error.lock().unwrap() = message;
                }
                chatRoute.poll(services.clone());
                deviceSpaceUi.poll();
                continue;
            }
            line = lines.next_line() => match line? {
                Some(line) => line,
                None => break,
            },
        };
        let request: serde_json::Value = serde_json::from_str(&line)?;
        let id = request["id"].clone();
        let services = edgeNode.nodeServices();
        let result = match request["command"].as_str() {
            Some("state") => {
                let paired = services
                    .as_ref()
                    .ok()
                    .and_then(|s| s.peers().pairedPeers().ok())
                    .is_some_and(|p| !p.is_empty());
                let mut playback = Vec::new();
                while let Some(packet) = speech.takePlayback() {
                    let mut packet = match packet { operit_link::CoreValue::Map(p) => p, _ => unreachable!() };
                    if let Some(operit_link::CoreValue::Bytes(bytes)) = packet.remove("audio") {
                        use base64::Engine;
                        packet.insert("audio".into(), operit_link::CoreValue::String(base64::engine::general_purpose::STANDARD.encode(bytes)));
                    }
                    playback.push(operit_link::fromCoreValue::<serde_json::Value>(operit_link::CoreValue::Map(packet)).map_err(|e|e.to_string())?);
                }
                let chat_state = edge_chat::snapshot();
                let chat_screen = edge_chat::screenTextFromSnapshot(&chat_state);
                sendReceipt.capture(edge_chat::takeSendResult());
                let pairing_code = services
                    .as_ref()
                    .ok()
                    .and_then(|s| s.peers().pairingPrompts().ok())
                    .unwrap_or_default()
                    .into_iter()
                    // Match firmware: one digits-only code, never a device-name prefix.
                    .map(|p| p.confirmationCode)
                    .find(|code| code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit()))
                    .unwrap_or_default();
                let space_join = spaceService.incomingDeviceSpaceJoins().await
                    .ok()
                    .and_then(|requests| requests.into_iter().find(|request| request.canApprove));
                Ok(serde_json::json!({"address": address, "sessionId": sessionId,
                "scene": sceneService.summary(),
                "speech": speech.snapshot(), "playback":playback,
                "coreAccess": coreAccess.snapshot(),
                "deviceSpace": deviceSpaceUi.snapshot(),
                "memory": memory::snapshot(),
                "deviceId": nodeId, "paired": paired,
                "activePeers": services.as_ref().ok().and_then(|s| s.peers().activePeerNodeIds().ok()).unwrap_or_default(),
                "peerServiceAvailable": services.is_ok(),
                "pairingCode": pairing_code,
                "spaceJoinPrompt": space_join.as_ref().map(|request| format!("申请加入空间\n{}\n{}", request.applicantName.chars().take(32).collect::<String>(), request.spaceName.chars().take(32).collect::<String>())).unwrap_or_default(),
                "spaceJoinRequestId": space_join.as_ref().map(|request| request.requestId.clone()).unwrap_or_default(),
                "spaceJoinAssignmentVersion": space_join.as_ref().map(|request| request.assignmentVersion).unwrap_or(0),
                "error": *error.lock().unwrap(),
                "lastAction": *lastAction.lock().unwrap(),
                "plugins": edge_chat::pluginsSnapshot(), "chat": chat_state, "chatPreview": edge_chat::preview(), "chatScreen": chat_screen,
                "chatTask": edge_chat::taskStatus(),
                "chatSendResult": sendReceipt.snapshot()}))
            }
            Some("coreAccess") => match services.as_ref() {
                Ok(services) => coreAccess.handle(&request, services, &spaceService).await,
                Err(error) => Err(error.message.clone()),
            },
            Some("sceneView") => Ok(sceneService.visual_snapshot()),
            Some("memory") => Ok(memory::snapshot()),
            // Decision/cleanup failures belong to this RPC, not to run(). In
            // particular a stale approval tap after cancellation must not
            // terminate the device or drop its authenticated TCP listener.
            Some("action") => async {
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
                    let requestId = request["requestId"].as_str()
                        .filter(|id| !id.trim().is_empty())
                        .ok_or_else(|| "Missing Space review requestId".to_string())?;
                    let assignmentVersion = request["assignmentVersion"].as_u64()
                        .ok_or_else(|| "Missing Space review assignmentVersion".to_string())?;
                    let approve = action == "edge_space_approve";
                    spaceService.decideDeviceSpaceJoin(
                        requestId.to_string(),
                        assignmentVersion,
                        approve,
                    ).await.map_err(|error| error.to_string())?;
                } else if action.starts_with("edge_plugins_") || action.starts_with("edge_plugin_") {
                    edge_chat::pluginsAction(action)?;
                } else if action == "edge_pair" || action == "edge_unpair" {
                    if action == "edge_unpair" { coreAccess = core_access::CoreAccess::default(); }
                    let result = async {
                        if action == "edge_pair" {
                            let services = edgeNode.nodeServices().map_err(|error| error.message)?;
                            services
                                .peers()
                                .startListening(&[PeerTransport::Tcp])
                                .await
                                .map_err(|e| e.to_string())?;
                        } else {
                            // Standalone simulator runs without a core runtime service.
                            // Unpair is intentionally idempotent in that mode.
                            if let Ok(services) = edgeNode.nodeServices() {
                                for peer in services.peers().pairedPeers().map_err(|e| e.to_string())? {
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
                    spaceService.leaveDeviceSpace()?;
                    deviceSpaceUi.reset();
                    coreAccess = core_access::CoreAccess::default();
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
            }.await,
            Some("speech") => (|| {
                if let Some(id) = request["requestId"].as_str() {
                    if speech.snapshot()["requestId"].as_str() != Some(id) { return Err("Speech request expired".into()); }
                }
                match request["operation"].as_str().unwrap_or("") {
                    "begin" => speech.beginRecognition().map(|id|serde_json::json!({"requestId":id})),
                    "pcm" => {
                        use base64::Engine;
                        let snapshot = speech.snapshot();
                        if snapshot["requestId"].as_str() != request["requestId"].as_str() { return Err("Speech capture changed".into()); }
                        let pcm = base64::engine::general_purpose::STANDARD.decode(request["audio"].as_str().unwrap_or("")).map_err(|e|e.to_string())?;
                        speech.appendPcm(pcm).map(|_|serde_json::json!({"ok":true}))
                    }
                    "finish" => speech.finishRecognition().map(|_|serde_json::json!({"ok":true})),
                    "cancel" => { speech.cancel(request["error"].as_str().unwrap_or("已取消语音任务").into()); Ok(serde_json::json!({"ok":true})) },
                    "tts" => speech.synthesize(request["text"].as_str().unwrap_or("").into()).map(|id|serde_json::json!({"requestId":id})),
                    _ => Err("Unknown speech operation".into()),
                }
            })(),
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
        };
        match result {
            Ok(value) => emit(serde_json::json!({"id": id, "value": value})),
            Err(error) => emit(serde_json::json!({"id": id, "error": error})),
        }
    }
    if let Ok(services) = edgeNode.nodeServices() {
        services.peers().stop().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
