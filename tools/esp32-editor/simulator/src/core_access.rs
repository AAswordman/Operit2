//! Simulator onboarding uses the node's existing TCP pairing and Space consent.
use operit_node_runtime::{
    NodeServices::{NodeServices, PeerEndpoint, PeerTransport, PendingPairing},
    NodeSpaceService::NodeSpaceService,
};
use serde_json::{json, Value};

#[derive(Default)]
pub struct CoreAccess {
    pairing: Option<PendingPairing>,
    request: Option<Value>,
    joined: bool,
}

impl CoreAccess {
    pub fn snapshot(&self) -> Value {
        json!({"pairing": self.pairing, "request": self.request, "joined": self.joined})
    }

    pub async fn handle(
        &mut self,
        input: &Value,
        services: &NodeServices,
        space: &NodeSpaceService,
    ) -> Result<Value, String> {
        match input["step"].as_str() {
            Some("request") => {
                let node = required(input, "nodeId")?;
                if let Some(pairing) = &self.pairing {
                    if pairing.peerNodeId != node {
                        return Err("请先取消当前 Core 配对申请".into());
                    }
                    return Ok(self.snapshot());
                }
                if space
                    .spaceChannelScope(node)
                    .map_err(|e| e.to_string())?
                    .is_some()
                {
                    self.joined = true;
                    self.request = None;
                    return Ok(self.snapshot());
                }
                self.joined = false;
                if services
                    .peers()
                    .pairedPeers()
                    .map_err(|e| e.to_string())?
                    .iter()
                    .any(|peer| peer.nodeId == node && peer.outbound)
                {
                    self.request = Some(
                        serde_json::to_value(space.requestDeviceSpaceJoin(node.into()).await?)
                            .map_err(|e| e.to_string())?,
                    );
                } else {
                    let address = required(input, "address")?;
                    let token = input["token"].as_str().filter(|token| !token.is_empty());
                    self.pairing = Some(
                        services
                            .peers()
                            .startPairing(
                                PeerEndpoint {
                                    nodeId: node.into(),
                                    address: address.into(),
                                },
                                PeerTransport::Tcp,
                                token,
                            )
                            .await
                            .map_err(|e| e.to_string())?,
                    );
                    self.request = None;
                }
            }
            Some("confirm") => {
                let id = required(input, "pairingId")?;
                let code = required(input, "confirmationCode")?;
                if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("请输入 Core 显示的六位配对码".into());
                }
                let pending = self
                    .pairing
                    .as_ref()
                    .filter(|p| p.pairingId == id)
                    .ok_or("Core 配对申请已变更，请重新申请")?;
                let node = pending.peerNodeId.clone();
                services
                    .peers()
                    .finishPairing(id, code)
                    .await
                    .map_err(|e| e.to_string())?;
                self.pairing = None;
                // Consent is still required after successful pairing. The Core is
                // the target Space reviewer; the simulator never approves itself.
                self.request = Some(
                    serde_json::to_value(space.requestDeviceSpaceJoin(node).await?)
                        .map_err(|e| e.to_string())?,
                );
            }
            Some("refresh") => {
                let id = required(input, "requestId")?;
                self.request = Some(
                    serde_json::to_value(space.refreshDeviceSpaceJoin(id.into()).await?)
                        .map_err(|e| e.to_string())?,
                );
            }
            Some("cancel") => {
                if let Some(pairing) = &self.pairing {
                    services
                        .peers()
                        .cancelPairing(&pairing.pairingId)
                        .await
                        .map_err(|e| e.to_string())?;
                    self.pairing = None;
                }
                if let Some(id) = self.request.as_ref().and_then(|r| r["requestId"].as_str()) {
                    self.request = Some(
                        serde_json::to_value(space.cancelDeviceSpaceJoin(id.into()).await?)
                            .map_err(|e| e.to_string())?,
                    );
                }
            }
            _ => return Err("Unknown Core access step".into()),
        }
        Ok(self.snapshot())
    }
}

fn required<'a>(input: &'a Value, key: &str) -> Result<&'a str, String> {
    input[key]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Missing {key}"))
}
