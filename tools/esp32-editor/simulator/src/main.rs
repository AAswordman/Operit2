#![allow(non_snake_case)]
// Compile the actual firmware modules, including the same stream renderer.
#[path = "../../../../apps/esp32/src/edge_chat.rs"]
mod edge_chat;
mod memory;
use operit_link::protocol::LinkDeviceInfo;
use operit_node_edge::PeerRouter::EdgePeerRouter;
use operit_node_runtime::{
    HostRuntimePeerService::HostRuntimePeerService,
    NodeServices::{NodeServices, PeerTransport},
    PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore},
};
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncBufReadExt, BufReader};

struct SimulatorStatusPlugin {
    address: String,
}

impl operit_node_edge::EdgePlugin for SimulatorStatusPlugin {
    fn manifest(&self) -> operit_node_edge::EdgePluginManifest {
        operit_node_edge::EdgePluginManifest {
            id: "device.status".into(),
            name: "Device status".into(),
            actions: vec!["read".into()],
        }
    }

    fn invoke(
        &self,
        action: &str,
        _args: operit_link::CoreValue,
    ) -> Result<operit_link::CoreValue, operit_node_edge::EdgeServiceError> {
        if action != "read" {
            return Err(operit_node_edge::EdgeServiceError::new(
                "unsupported status action",
            ));
        }
        Ok(operit_link::CoreValue::Map(
            std::collections::BTreeMap::from([
                (
                    "boardId".into(),
                    operit_link::CoreValue::String("ESP32-2432S028-SIM".into()),
                ),
                (
                    "expression".into(),
                    operit_link::CoreValue::String("online".into()),
                ),
                (
                    "ipv4".into(),
                    operit_link::CoreValue::String(self.address.clone()),
                ),
                (
                    "wifiSsid".into(),
                    operit_link::CoreValue::String("simulator".into()),
                ),
            ]),
        ))
    }
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
    };
    let edgeRouter = EdgePeerRouter::new(nodeId.clone());
    let scenePlugin = Arc::new(operit_edge_scene::ScenePlugin::new(320, 240, 24, true));
    let mut edgeNode = operit_node_edge::EdgeNode::fromHostManager(hostManager.clone())
        .withPlugin(Arc::new(SimulatorStatusPlugin {
            address: configuredAddress.clone(),
        }))
        .map_err(|error| error.message)?;
    let (services, startListener) = match nodeServices {
        Some(services) => (services, false),
        None => {
            let peers = HostRuntimePeerService::newWithLimits(
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
                token: std::env::var("OPERIT_SIM_TOKEN")
                    .unwrap_or_else(|_| "operit-simulator-token".into()),
                transports: vec![PeerTransport::Tcp],
                discoveryEnabled: false,
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
    })?;
    edgeRouter.installSpace(spaceService.clone())?;
    edgeNode = edgeNode
        .withPlugin(scenePlugin.clone())
        .map_err(|error| error.message)?;
    edgeNode = edgeNode.withNodeServices(services.clone());
    let edgeNode = Arc::new(edgeNode);
    edgeRouter.install(edgeNode.clone())?;
    let _sceneEvents = edge_chat::startEdgeEvents(
        services.clone(), scenePlugin.clone(), nodeId.clone(),
    );
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
    let mut chatRoute = edge_chat::SpaceChatRoute::default();
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        let line = tokio::select! {
            _ = routePoll.tick() => {
                chatRoute.poll(services.clone());
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
        let paired = services
            .as_ref()
            .ok()
            .and_then(|s| s.peers().pairedPeers().ok())
            .is_some_and(|p| !p.is_empty());
        let chat_state = edge_chat::snapshot();
        let chat_screen = edge_chat::screenText();
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
        let result = match request["command"].as_str() {
            Some("state") => {
                let space_join = spaceService.incomingDeviceSpaceJoins().await
                    .ok()
                    .and_then(|requests| requests.into_iter().find(|request| request.canApprove));
                Ok(serde_json::json!({"address": address,
                "scene": scenePlugin.summary(),
                "memory": memory::snapshot(),
                "deviceId": "esp32-edge-simulator", "paired": paired,
                "peerServiceAvailable": services.is_ok(),
                "pairingCode": pairing_code,
                "spaceJoinPrompt": space_join.as_ref().map(|request| format!("申请加入空间\n{}\n{}", request.applicantName.chars().take(32).collect::<String>(), request.spaceName.chars().take(32).collect::<String>())).unwrap_or_default(),
                "spaceJoinRequestId": space_join.as_ref().map(|request| request.requestId.clone()).unwrap_or_default(),
                "spaceJoinAssignmentVersion": space_join.as_ref().map(|request| request.assignmentVersion).unwrap_or(0),
                "error": *error.lock().unwrap(),
                "lastAction": *lastAction.lock().unwrap(),
                "plugins": edge_chat::pluginsSnapshot(), "chat": chat_state, "chatPreview": edge_chat::preview(), "chatScreen": chat_screen,
                "chatTask": edge_chat::taskStatus(),
                "chatSendResult": edge_chat::takeSendResult().map(|result| match result {
                    Ok(()) => serde_json::json!({"ok": true}),
                    Err(error) => serde_json::json!({"ok": false, "error": error}),
                })}))
            }
            Some("sceneView") => Ok(scenePlugin.visual_snapshot()),
            Some("memory") => Ok(memory::snapshot()),
            // Decision/cleanup failures belong to this RPC, not to run(). In
            // particular a stale approval tap after cancellation must not
            // terminate the device or drop its authenticated TCP listener.
            Some("action") => async {
                let action = request["action"].as_str().unwrap_or("");
                *lastAction.lock().unwrap() = action.to_string();
                if action == "edge_scene_exit" {
                    scenePlugin.system_exit();
                } else if let Some(point) = action.strip_prefix("edge_scene_touch:") {
                    let (x, y) = point.split_once(':').ok_or("Invalid scene point")?;
                    if pairing_code.is_empty()
                        && spaceService.incomingDeviceSpaceJoins().await
                            .map_err(|error| error.to_string())?.is_empty()
                    {
                        scenePlugin.touch(
                            x.parse().map_err(|_| "Invalid scene x")?,
                            y.parse().map_err(|_| "Invalid scene y")?,
                        );
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
                            chatRoute.reset();
                        }
                        Ok::<_, String>(())
                    }
                    .await;
                    result?;
                } else if action == "edge_space_leave" {
                    spaceService.leaveDeviceSpace()?;
                    chatRoute.reset();
                } else if action == "edge_history_older" || action == "edge_history_newer" {
                    edge_chat::moveHistory(action == "edge_history_older")?;
                } else if action == "edge_new" {
                    edge_chat::newChat()?;
                } else if let Some(chatId) = action.strip_prefix("edge_select:") {
                    edge_chat::selectChat(chatId)?;
                } else {
                    return Err("Unknown simulator action".to_string());
                }
                Ok::<_, String>(serde_json::json!({"ok": true, "action": action}))
            }.await,
            Some("sendImage") => (|| {
                use base64::Engine;
                let encoded = request["bytes"].as_str().unwrap_or("");
                // Base64's Vec decoder reserves this upper bound, including padding.
                // Check BEFORE allocating. Errors stay within this RPC, not main().
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|_| "Invalid image bytes".to_string())?;
                edge_chat::sendImage(bytes, request["mimeType"].as_str().unwrap_or("").into())
                    .map(|_| serde_json::json!({"accepted":true}))
            })(),
            Some("send") => edge_chat::send(request["text"].as_str().unwrap_or("").into())
                .map(|_| serde_json::json!({"ok": true})),
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
