//! Native device services owned by the firmware application.
use std::sync::Arc;
use operit_link::{CoreCallRequest, CoreCallResponse, CoreLinkError, CoreValue};
use operit_node_edge::{EdgeCallHandler, EdgeServiceError};
use crate::status::FirmwareStatus;

pub fn callHandler(status: Arc<FirmwareStatus>, scene: Arc<operit_edge_scene::SceneService>) -> Arc<EdgeCallHandler> {
    Arc::new(move |request: CoreCallRequest| {
        let status = status.clone();
        let scene = scene.clone();
        Box::pin(async move {
            let result = match request.target.as_str() {
                "edge.status" => readStatus(&status, &request.methodName),
                "edge.scene" => scene.call(&request.methodName, request.args)
                    .map_err(EdgeServiceError::new),
                "edge.ui" => callUi(&request.methodName, request.args).await,
                _ => return None,
            };
            Some(CoreCallResponse {
                requestId: request.requestId,
                result: result.map_err(|error| CoreLinkError::new("DEVICE_SERVICE_ERROR", error.message)),
            })
        })
    })
}

fn readStatus(status: &FirmwareStatus, method: &str) -> Result<CoreValue, EdgeServiceError> {
    if method == "health" {
        return operit_link::toCoreValue(crate::runtimeHealthSnapshot())
            .map_err(|error| EdgeServiceError::new(error.to_string()));
    }
    if method != "read" { return Err(EdgeServiceError::new("Unsupported device status method")); }
    let status = status.snapshot();
    operit_link::toCoreValue(serde_json::json!({
        "boardId":status.boardId, "expression":status.expression,
        "ipv4":status.ipv4, "wifiSsid":status.wifiSsid,
        "wifiConnected":status.wifiConnected,
    })).map_err(|error| EdgeServiceError::new(error.to_string()))
}

struct UiRequest {
    action: String, argument: String, deadline: std::time::Instant,
    reply: tokio::sync::oneshot::Sender<Result<serde_json::Value, String>>,
}
static UI_REQUEST: std::sync::Mutex<Option<UiRequest>> = std::sync::Mutex::new(None);
async fn callUi(action: &str, args: CoreValue) -> Result<CoreValue, EdgeServiceError> {
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
pub fn pumpUi(ui: &mut crate::ui::Esp32Ui) {
    let request = UI_REQUEST.lock().unwrap().take();
    if let Some(r) = request {
        let result = if r.reply.is_closed() || std::time::Instant::now() >= r.deadline { Err("Expired UI request was not executed".into()) }
            else { ui.debugOperation(&r.action, &r.argument) };
        let _ = r.reply.send(result);
    }
}
