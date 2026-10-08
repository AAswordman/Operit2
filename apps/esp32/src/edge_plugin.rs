//! Firmware-owned native plugin actions exposed over authenticated PeerLink.
use std::sync::Arc;

use operit_link::CoreValue;
use operit_node_edge::{EdgePlugin, EdgePluginManifest, EdgeServiceError};

use crate::status::FirmwareStatus;

pub struct DeviceStatusPlugin {
    status: Arc<FirmwareStatus>,
}

impl DeviceStatusPlugin {
    pub fn new(status: Arc<FirmwareStatus>) -> Self {
        Self { status }
    }
}

impl EdgePlugin for DeviceStatusPlugin {
    fn manifest(&self) -> EdgePluginManifest {
        EdgePluginManifest {
            id: "device.status".into(),
            name: "Device status".into(),
            actions: vec!["read".into(), "health".into()],
        }
    }

    fn invoke(&self, action: &str, _args: CoreValue) -> Result<CoreValue, EdgeServiceError> {
        if action == "health" {
            return operit_link::toCoreValue(crate::runtimeHealthSnapshot())
                .map_err(|error| EdgeServiceError::new(error.to_string()));
        }
        if action != "read" {
            return Err(EdgeServiceError::new("unsupported status action"));
        }
        let status = self.status.snapshot();
        Ok(CoreValue::Map(std::collections::BTreeMap::from([
            ("boardId".into(), CoreValue::String(status.boardId)),
            ("expression".into(), CoreValue::String(status.expression)),
            ("ipv4".into(), CoreValue::String(status.ipv4)),
            ("wifiSsid".into(), CoreValue::String(status.wifiSsid)),
            ("wifiConnected".into(), CoreValue::Bool(status.wifiConnected)),
        ])))
    }
}

struct UiRequest {
    action: String, argument: String, deadline: std::time::Instant,
    reply: tokio::sync::oneshot::Sender<Result<serde_json::Value, String>>,
}
static UI_REQUEST: std::sync::Mutex<Option<UiRequest>> = std::sync::Mutex::new(None);
/// Authenticated debug, dispatched on the UI task, never a second UART reader.
pub struct DeviceUiPlugin;
#[async_trait::async_trait(?Send)]
impl EdgePlugin for DeviceUiPlugin {
    fn manifest(&self) -> EdgePluginManifest {
        EdgePluginManifest { id: "device.ui".into(), name: "Device UI debug".into(),
            actions: ["screen", "tree", "tap", "swipe", "draft", "chat"].map(String::from).to_vec() }
    }
    fn invoke(&self, action: &str, _args: CoreValue) -> Result<CoreValue, EdgeServiceError> {
        if action == "chat" { return operit_link::toCoreValue(crate::edge_chat::snapshot()).map_err(|e| EdgeServiceError::new(e.to_string())); }
        Err(EdgeServiceError::new("UI actions require the asynchronous Link runtime"))
    }
    async fn invokeAsync(&self, action: &str, args: CoreValue) -> Result<CoreValue, EdgeServiceError> {
        if action == "chat" { return operit_link::toCoreValue(crate::edge_chat::snapshot()).map_err(|e| EdgeServiceError::new(e.to_string())); }
        if !["screen", "tree", "tap", "swipe", "draft"].contains(&action) { return Err(EdgeServiceError::new("Unsupported UI action")); }
        let argument = match args {
            CoreValue::Map(fields) => match fields.get("argument") { Some(CoreValue::String(s)) => s.clone(), None => String::new(), _ => return Err(EdgeServiceError::new("Expected string argument")) },
            _ => return Err(EdgeServiceError::new("Expected object arguments")),
        };
        if argument.len() > 120 || argument.contains('\0') { return Err(EdgeServiceError::new("UI argument exceeds bounded capacity")); }
        let (reply, receiver) = tokio::sync::oneshot::channel();
        {
            let mut pending = UI_REQUEST.lock().unwrap();
            if pending.is_some() { return Err(EdgeServiceError::new("UI debug mailbox busy")); }
            *pending = Some(UiRequest { action: action.into(), argument, reply,
                deadline: std::time::Instant::now() + std::time::Duration::from_secs(2) });
        }
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), receiver).await
            .map_err(|_| EdgeServiceError::new("UI did not acknowledge; action is not retried"))?
            .map_err(|_| EdgeServiceError::new("UI mailbox closed"))?
            .map_err(EdgeServiceError::new)?;
        operit_link::toCoreValue(result).map_err(|e| EdgeServiceError::new(e.to_string()))
    }
}
pub fn pumpUi(ui: &mut crate::ui::Esp32Ui) {
    let request = UI_REQUEST.lock().unwrap().take();
    if let Some(r) = request {
        let result = if r.reply.is_closed() || std::time::Instant::now() >= r.deadline { Err("Expired UI request was not executed".into()) }
            else { ui.debugOperation(&r.action, &r.argument) };
        let _ = r.reply.send(result);
    }
}
