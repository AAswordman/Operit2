//! Simulator onboarding uses the node's existing TCP pairing and Space consent.
use operit_edge_sdk::runtime::{
    NodeServices::{NodeServices, PeerEndpoint, PeerTransport, PendingPairing},
    NodeSpaceService::NodeSpaceService,
};
use serde_json::{json, Value};
use std::{future::Future, pin::Pin};

#[derive(Default)]
pub struct CoreAccess {
    pairing: Option<PendingPairing>,
    request: Option<Value>,
    joined: bool,
}

impl CoreAccess {
    /// Projects the current onboarding transaction for the editor.
    pub fn snapshot(&self) -> Value {
        json!({"pairing": self.pairing, "request": self.request, "joined": self.joined})
    }

    /// Dispatches one phase without retaining unrelated phase futures on its worker stack.
    pub fn handle<'a>(
        &'a mut self,
        input: &'a Value,
        services: &'a NodeServices,
        space: &'a NodeSpaceService,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + 'a>> {
        match input["step"].as_str() {
            Some("request") => Box::pin(self.request(input, services, space)),
            Some("confirm") => Box::pin(self.confirm(input, services, space)),
            Some("refresh") => Box::pin(self.refresh(input, space)),
            Some("cancel") => Box::pin(self.cancel(services, space)),
            _ => Box::pin(async { Err("Unknown Core access step".into()) }),
        }
    }

    /// Starts one explicit pairing or consent request through the shared node services.
    async fn request(&mut self, input: &Value, services: &NodeServices, space: &NodeSpaceService) -> Result<Value, String> {
        let node = required(input, "nodeId")?;
        if let Some(pairing) = &self.pairing {
            if pairing.peerNodeId != node { return Err("请先取消当前 Core 配对申请".into()); }
            return Ok(self.snapshot());
        }
        if space.spaceChannelScope(node).map_err(|e| e.to_string())?.is_some() {
            self.joined = true;
            self.request = None;
            return Ok(self.snapshot());
        }
        self.joined = false;
        let paired = services.peers().pairedPeers().map_err(|e| e.to_string())?
            .iter().any(|peer| peer.nodeId == node && peer.outbound);
        if paired {
            self.request = Some(serde_json::to_value(space.requestDeviceSpaceJoin(node.into()).await?)
                .map_err(|e| e.to_string())?);
        } else {
            let address = required(input, "address")?;
            self.pairing = Some(services.peers().startPairing(
                PeerEndpoint { nodeId: node.into(), address: address.into() }, Some(PeerTransport::Tcp),
            ).await.map_err(|e| e.to_string())?);
            self.request = None;
        }
        Ok(self.snapshot())
    }

    /// Confirms the captured pairing before submitting a separately authorized Space request.
    async fn confirm(&mut self, input: &Value, services: &NodeServices, space: &NodeSpaceService) -> Result<Value, String> {
        let id = required(input, "pairingId")?;
        let code = required(input, "confirmationCode")?;
        if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Err("请输入 Core 显示的六位配对码".into());
        }
        let node = self.pairing.as_ref().filter(|p| p.pairingId == id)
            .ok_or("Core 配对申请已变更，请重新申请")?.peerNodeId.clone();
        services.peers().finishPairing(id, code).await.map_err(|e| e.to_string())?;
        self.pairing = None;
        self.request = Some(serde_json::to_value(space.requestDeviceSpaceJoin(node).await?)
            .map_err(|e| e.to_string())?);
        Ok(self.snapshot())
    }

    /// Refreshes the exact durable consent request selected by the editor.
    async fn refresh(&mut self, input: &Value, space: &NodeSpaceService) -> Result<Value, String> {
        let id = required(input, "requestId")?;
        self.request = Some(serde_json::to_value(space.refreshDeviceSpaceJoin(id.into()).await?)
            .map_err(|e| e.to_string())?);
        Ok(self.snapshot())
    }

    /// Cancels the recorded pairing and consent transactions through their owning services.
    async fn cancel(&mut self, services: &NodeServices, space: &NodeSpaceService) -> Result<Value, String> {
        if let Some(pairing) = &self.pairing {
            services.peers().cancelPairing(&pairing.pairingId).await.map_err(|e| e.to_string())?;
            self.pairing = None;
        }
        if let Some(id) = self.request.as_ref().and_then(|r| r["requestId"].as_str()) {
            self.request = Some(serde_json::to_value(space.cancelDeviceSpaceJoin(id.into()).await?)
                .map_err(|e| e.to_string())?);
        }
        Ok(self.snapshot())
    }
}

/// Validates one explicit onboarding identifier supplied by the editor.
fn required<'a>(input: &'a Value, key: &str) -> Result<&'a str, String> {
    input[key].as_str().filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("Missing {key}"))
}
