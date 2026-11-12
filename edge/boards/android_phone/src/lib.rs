//! Standalone old-phone Edge product. No Android Core Host or Flutter runtime.
#[cfg(target_os = "android")]
mod android;
mod storage;
#[cfg(test)]
mod tests;

use operit_edge_sdk::{
    device::{createRobotFaceService, EdgeNode},
    host::{
        HostError, HostManager::HostManager, HostResult, RobotFaceExpressionRequest, RobotFaceHost,
        RobotFaceState,
    },
    link::protocol::LinkDeviceInfo,
    runtime::{
        NodeServices::{PeerEndpoint, PeerTransport, PendingPairing},
        PeerStateStore::{PeerHostConfig, PeerHostPortMode, PeerStateStore},
    },
    space::{NodeSpaceContext, PeerRuntimeLimits, SpaceDevice},
};
use operit_host_native_common::NativeTcpHost;
use operit_host_native_scheduler::LocalHostRuntimeTaskSchedulerHost;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

struct Face(Mutex<String>);
impl RobotFaceHost for Face {
    /// Accepts only expressions painted by the phone's local Canvas renderer.
    fn setExpression(&self, request: RobotFaceExpressionRequest) -> HostResult<RobotFaceState> {
        if !matches!(
            request.expression.as_str(),
            "neutral" | "happy" | "listening" | "thinking" | "speaking" | "sleeping"
        ) {
            return Err(HostError::new("Unsupported phone expression"));
        }
        *self.0.lock().map_err(|e| HostError::new(e.to_string()))? = request.expression.clone();
        Ok(RobotFaceState {
            expression: request.expression,
        })
    }
    /// Returns the same state consumed by the local UI and remote Edge callers.
    fn getExpression(&self) -> HostResult<RobotFaceState> {
        let expression = self.0.lock().map_err(|e| HostError::new(e.to_string()))?;
        Ok(RobotFaceState {
            expression: expression.clone(),
        })
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum Command {
    Status,
    StartPairing { address: String },
    FinishPairing { id: String, code: String },
    CancelPairing { id: String },
    Join { node: String },
    RefreshJoin { id: String },
    CancelJoin { id: String },
    Leave,
    Unpair { node: String },
    Expression { expression: String },
    Stop,
}

pub struct PhoneNode {
    device: SpaceDevice,
    face: Arc<Face>,
    pending_pairing: Mutex<Option<PendingPairing>>,
}
impl PhoneNode {
    /// Starts one genuine Edge identity over TCP with SDK-owned authentication.
    pub async fn start(root: PathBuf, model: String, bind: String) -> Result<Self, String> {
        let face = Arc::new(Face(Mutex::new("happy".into())));
        let host = HostManager::new()
            .withRuntimeStorageHost(Arc::new(
                storage::PhoneStorage::open(root).map_err(|e| e.to_string())?,
            ))
            .withTcpHost(Arc::new(NativeTcpHost))
            .withHostRuntimeTaskSchedulerHost(Arc::new(
                LocalHostRuntimeTaskSchedulerHost::new().map_err(|e| e.to_string())?,
            ))
            .withRobotFaceHost(face.clone());
        let node = EdgeNode::headless().withRobotFaceService(createRobotFaceService(host.clone()));
        let device = SpaceDevice::assemble(
            host,
            LinkDeviceInfo {
                platform: "android-edge".into(),
                model,
                pairingIncarnation: None,
            },
            node,
            PeerRuntimeLimits::constrained(),
        )?;
        let store = PeerStateStore::new(device.space.storage());
        let mut config = match store.hostConfig()? {
            Some(config) => config,
            None => PeerHostConfig {
                bindAddress: bind.clone(),
                transports: vec![PeerTransport::Tcp],
                discoveryEnabled: false,
                portMode: PeerHostPortMode::Automatic,
                updatedAt: 0,
            },
        };
        // The SDK's generic defaults include HTTP. This product explicitly owns
        // a TCP-only listener while retaining its durably assigned port.
        let mut address = bind
            .parse::<std::net::SocketAddr>()
            .map_err(|e| e.to_string())?;
        address.set_port(
            config
                .bindAddress
                .parse::<std::net::SocketAddr>()
                .map_err(|e| e.to_string())?
                .port(),
        );
        config.bindAddress = address.to_string();
        config.transports = vec![PeerTransport::Tcp];
        config.discoveryEnabled = false;
        config.portMode = PeerHostPortMode::Automatic;
        if let Err(error) = device.listen(config).await {
            device.stop().await?;
            return Err(error);
        }
        Ok(Self {
            device,
            face,
            pending_pairing: Mutex::new(None),
        })
    }

    /// Executes only typed local UI operations; remote requests use the SDK router.
    pub async fn execute(&self, command: Command) -> Result<Value, String> {
        let peers = self.device.services.peers();
        match command {
            Command::Status => self.status(),
            Command::StartPairing { address } => {
                if self
                    .pending_pairing
                    .lock()
                    .map_err(|e| e.to_string())?
                    .is_some()
                {
                    return Err("请先完成或取消当前配对".into());
                }
                let address = address.trim();
                address
                    .parse::<std::net::SocketAddr>()
                    .map_err(|_| "请输入 TCP IP:端口（IPv6 地址使用方括号）".to_string())?;
                let pairing = peers
                    .startPairing(
                        PeerEndpoint {
                            nodeId: String::new(),
                            address: address.to_owned(),
                        },
                        Some(PeerTransport::Tcp),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                *self.pending_pairing.lock().map_err(|e| e.to_string())? = Some(pairing.clone());
                Ok(json!({"pairing": pairing}))
            }
            Command::FinishPairing { id, code } => {
                if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("请输入六位数字配对码".into());
                }
                let peer = peers
                    .finishPairing(&id, &code)
                    .await
                    .map_err(|e| e.to_string())?;
                *self.pending_pairing.lock().map_err(|e| e.to_string())? = None;
                Ok(json!({"peer": peer}))
            }
            Command::CancelPairing { id } => {
                peers.cancelPairing(&id).await.map_err(|e| e.to_string())?;
                *self.pending_pairing.lock().map_err(|e| e.to_string())? = None;
                Ok(json!({}))
            }
            Command::Join { node } => {
                Ok(json!({"request": self.device.space.requestDeviceSpaceJoin(node).await?}))
            }
            Command::RefreshJoin { id } => {
                Ok(json!({"request": self.device.space.refreshDeviceSpaceJoin(id).await?}))
            }
            Command::CancelJoin { id } => {
                Ok(json!({"request": self.device.space.cancelDeviceSpaceJoin(id).await?}))
            }
            Command::Leave => {
                self.device.space.leaveDeviceSpace()?;
                self.status()
            }
            Command::Unpair { node } => {
                peers
                    .removePairedPeer(&node)
                    .await
                    .map_err(|e| e.to_string())?;
                self.status()
            }
            Command::Expression { expression } => {
                self.face
                    .setExpression(RobotFaceExpressionRequest { expression })
                    .map_err(|e| e.to_string())?;
                self.status()
            }
            Command::Stop => {
                self.device.stop().await?;
                Ok(json!({}))
            }
        }
    }

    /// Exposes bounded local management state, including codes only to this app.
    pub fn status(&self) -> Result<Value, String> {
        let peers = self.device.services.peers();
        let config = PeerStateStore::new(self.device.space.storage())
            .hostConfig()?
            .ok_or("Edge listener configuration is missing")?;
        let space = self.device.space.spaceStore().space()?;
        Ok(
            json!({"nodeId": self.device.node_id, "address": config.bindAddress,
            "spaceId": space.spaceId, "spaceName": space.spaceName,
            "expression": self.face.getExpression().map_err(|e| e.to_string())?.expression,
            "peers": peers.pairedPeers().map_err(|e| e.to_string())?,
            "pendingPairing": *self.pending_pairing.lock().map_err(|e| e.to_string())?,
            "prompts": peers.pairingPrompts().map_err(|e| e.to_string())?,
            "joins": self.device.space.outgoingDeviceSpaceJoins()?}),
        )
    }
}
