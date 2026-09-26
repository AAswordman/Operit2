#![allow(non_snake_case)]
// Compile the actual firmware modules, including the same stream renderer.
#[path = "../../../../apps/esp32/src/edge_chat.rs"]
mod edge_chat;
#[path = "../../../../apps/esp32/src/edge_session.rs"]
mod edge_session;
#[cfg(test)]
mod tests;

use operit_edge_transport::{
    linkTokenHash, tcp::TcpLinkChannel, EdgePairingAuthority, EdgePairingPersistentState,
    EdgePairingStore,
};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncBufReadExt, BufReader};

struct FileStore(PathBuf);
impl EdgePairingStore for FileStore {
    fn load(&self) -> Result<Option<EdgePairingPersistentState>, String> {
        match std::fs::read(&self.0) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn save(&self, state: &EdgePairingPersistentState) -> Result<(), String> {
        let bytes = serde_json::to_vec(state).map_err(|e| e.to_string())?;
        let temporary = self.0.with_extension("tmp");
        std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(temporary, &self.0).map_err(|e| e.to_string())
    }
}

fn debug_node(
    id: &str,
    role: &str,
    text: impl Into<String>,
    action: &str,
    parent: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    visible: bool,
    clickable: bool,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "role": role,
        "text": text.into(),
        "action": action,
        "parent": parent,
        "rect": {"x": x, "y": y, "w": w, "h": h},
        "visible": visible,
        "enabled": true,
        "clickable": clickable,
    })
}

fn debug_tree(page: &str, sidebar_open: bool, sidebar_width: i32, pairing_code: &str, chat: &str, connected: bool) -> serde_json::Value {
    let mut nodes = Vec::new();
    if page == "Pairing" {
        nodes.push(debug_node("screen", "screen", "", "", "", 0, 0, 320, 240, true, false));
        nodes.push(debug_node("edge_chat", "button", "返回聊天", "edge_chat", "screen", 10, 8, 32, 30, true, true));
        nodes.push(debug_node("pairing_code", "label", if pairing_code.is_empty() { "------" } else { pairing_code }, "", "screen", 18, 112, 284, 30, true, false));
        nodes.push(debug_node("pairing_hint", "label", if pairing_code.is_empty() { "打开 Operit > 设备 > 添加边缘设备" } else { "请在 Operit 中输入此配对码" }, "", "screen", 18, 142, 284, 30, true, false));
        nodes.push(debug_node("edge_pair", "button", "刷新", "edge_pair", "screen", 18, 187, 136, 36, true, true));
    } else {
        let chat_x = if sidebar_open { sidebar_width } else { 0 };
        nodes.push(debug_node("chat", "screen", "", "", "", chat_x, 0, 320, 240, true, false));
        nodes.push(debug_node("sidebar_toggle", "button", "☰", "sidebar_toggle", "chat", chat_x + 8, 7, 32, 28, true, true));
        nodes.push(debug_node("connection_status", "label", if connected { "已连接" } else { "离线" }, "", "chat", chat_x + 224, 12, 82, 20, true, false));
        nodes.push(debug_node("chat_text", "label", chat, "", "chat", chat_x + 18, 58, 284, 110, true, false));
        nodes.push(debug_node("chat_input", "textbox", "", "", "chat", chat_x + 10, 181, 252, 38, true, true));
        nodes.push(debug_node("edge_send", "button", "发送", "edge_send", "chat", chat_x + 270, 181, 40, 38, true, true));
        if sidebar_open {
            nodes.push(debug_node("sidebar_scrim", "scrim", "", "sidebar_close", "", sidebar_width, 0, 320 - sidebar_width, 240, true, true));
            nodes.push(debug_node("sidebar", "navigation", "聊天", "", "", 0, 0, sidebar_width, 240, true, false));
            nodes.push(debug_node("edge_new", "button", "清空草稿", "edge_new", "sidebar", 10, 28, sidebar_width - 20, 30, true, true));
            nodes.push(debug_node("sidebar_close", "button", "当前对话", "sidebar_close", "sidebar", 10, 78, sidebar_width - 20, 30, true, true));
            nodes.push(debug_node("chat_preview", "label", chat, "", "sidebar", 20, 112, sidebar_width - 36, 30, true, false));
            nodes.push(debug_node("edge_pair", "button", "配对设备", "builtin:Pairing", "sidebar", 10, 144, sidebar_width - 20, 30, true, true));
            nodes.push(debug_node("tasks", "button", "任务", "builtin:Tasks", "sidebar", 10, 176, sidebar_width - 20, 30, true, true));
            nodes.push(debug_node("settings", "button", "设置", "builtin:Settings", "sidebar", 10, 208, sidebar_width - 20, 30, true, true));
        }
    }
    serde_json::json!({"page": page, "width": 320, "height": 240, "nodes": nodes})
}

fn debug_snapshot(page: &str, sidebar_open: bool, sidebar_width: i32, pairing_code: &str, chat: &str, connected: bool) -> serde_json::Value {
    let tree = debug_tree(page, sidebar_open, sidebar_width, pairing_code, chat, connected);
    serde_json::json!({
        "page": page,
        "width": 320,
        "height": 240,
        "sidebarOpen": sidebar_open,
        "sidebarWidth": sidebar_width,
        "pairingCode": pairing_code,
        "chat": chat,
        "tree": tree,
    })
}

fn apply_debug_action(action: &str, page: &mut String, sidebar_open: &mut bool, sidebar_width: &mut i32) {
    match action {
        "sidebar_open" => *sidebar_open = true,
        "sidebar_close" => *sidebar_open = false,
        "sidebar_toggle" => *sidebar_open = !*sidebar_open,
        "edge_chat" => { *sidebar_open = false; *page = "Chat".into(); }
        "edge_pair" | "edge_search" | "builtin:Pairing" => { *sidebar_open = false; *page = "Pairing".into(); }
        "builtin:Tasks" => { *sidebar_open = false; *page = "Tasks".into(); }
        "builtin:Settings" => { *sidebar_open = false; *page = "Settings".into(); }
        "sidebar_width_cycle" => {
            *sidebar_width = if *sidebar_width <= 180 { 213 } else if *sidebar_width <= 240 { 256 } else { 160 };
        }
        _ => {}
    }
}

fn emit(value: serde_json::Value) {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{value}").expect("editor IPC closed");
    stdout.flush().expect("editor IPC closed");
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("OPERIT_SIM_TOKEN")?;
    if token.is_empty() {
        return Err("Simulator token is empty".into());
    }
    let statePath = PathBuf::from(std::env::var("OPERIT_SIM_STATE")?);
    let tokenHash = linkTokenHash(&token);
    let code = Arc::new(Mutex::new(String::new()));
    let error = Arc::new(Mutex::new(String::new()));
    let lastAction = Arc::new(Mutex::new(String::new()));
    let mut debug_page = "Chat".to_string();
    let mut debug_sidebar_open = false;
    let mut debug_sidebar_width = 213_i32;
    let authority = Arc::new(EdgePairingAuthority::newWithStore(
        token,
        "esp32-edge-simulator",
        operit_link::LinkDeviceInfo {
            platform: "esp32".into(),
            model: "ESP32-2432S028".into(),
        },
        Arc::new(FileStore(statePath)),
        {
            let code = code.clone();
            move |value| {
                *code.lock().unwrap() = value;
            }
        },
    )?);
    // Match a normal LAN node: the editor IPC remains local, while the Edge
    // Link carrier is reachable by a Core on the same network by default.
    let address = std::env::var("OPERIT_SIM_BIND").unwrap_or_else(|_| "0.0.0.0:18765".into());
    let listener = TcpLinkChannel::bind(&address).await?;
    let address = listener.local_addr()?.to_string();
    // Advertise the raw TCP Edge listener separately from Core's HTTP service.
    // mDNS is discovery only; the token hash is not a substitute for pairing.
    let mdns = {
        let daemon = ServiceDaemon::new()?;
        let port = listener.local_addr()?.port();
        let pid = std::process::id();
        let serviceType = "_operit-edge._tcp.local.";
        let instance = format!("operit-edge-simulator-{pid}");
        let hostname = format!("operit-edge-simulator-{pid}.local.");
        let mut properties = HashMap::new();
        properties.insert("deviceId".to_string(), "esp32-edge-simulator".to_string());
        properties.insert("displayName".to_string(), "ESP32 Simulator".to_string());
        properties.insert("platform".to_string(), "esp32".to_string());
        properties.insert("model".to_string(), "ESP32-2432S028".to_string());
        properties.insert("tokenHash".to_string(), tokenHash);
        properties.insert("version".to_string(), "edge-1".to_string());
        let info = ServiceInfo::new(serviceType, &instance, &hostname, "", port, properties)?
            .enable_addr_auto();
        let fullname = info.get_fullname().to_string();
        daemon.register(info)?;
        Some((daemon, fullname))
    };
    let sessionError = error.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(value) => value,
                Err(e) => {
                    *sessionError.lock().unwrap() = e.to_string();
                    break;
                }
            };
            // A reconnect can arrive before the previous TCP task has observed
            // its close. Do not reject it: the real ESP32 listener has no
            // single-connection gate, and the authenticated session is owned
            // by the channel task below.
            let authority = authority.clone();
            let error = sessionError.clone();
            tokio::spawn(async move {
                *error.lock().unwrap() = String::new();
                if let Err(e) =
                    edge_session::handleChannel(authority, TcpLinkChannel::fromStream(stream)).await
                {
                    *error.lock().unwrap() = e;
                }
            });
        }
    });
    emit(serde_json::json!({"ready": true, "address": address, "discovery": "_operit-edge._tcp.local."}));
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        let request: serde_json::Value = serde_json::from_str(&line)?;
        let id = request["id"].clone();
        let chat_state = edge_chat::snapshot();
        let connected = chat_state.get("connected").and_then(|value| value.as_bool()).unwrap_or(false);
        let chat_screen = edge_chat::screenText();
        let pairing_code = code.lock().unwrap().clone();
        let result = match request["command"].as_str() {
            Some("state") => Ok(serde_json::json!({"address": address,
                "pairingCode": pairing_code, "error": *error.lock().unwrap(),
                "lastAction": *lastAction.lock().unwrap(), "page": debug_page,
                "sidebarOpen": debug_sidebar_open, "sidebarWidth": debug_sidebar_width,
                "chat": chat_state, "chatPreview": edge_chat::preview(), "chatScreen": chat_screen,
                "chatTask": edge_chat::taskStatus(),
                "chatSendResult": edge_chat::takeSendResult().map(|result| match result {
                    Ok(()) => serde_json::json!({"ok": true}),
                    Err(error) => serde_json::json!({"ok": false, "error": error}),
                })})),
            Some("debug_tree") => Ok(debug_tree(&debug_page, debug_sidebar_open, debug_sidebar_width, &pairing_code, &chat_screen, connected)),
            Some("debug_snapshot") => Ok(debug_snapshot(&debug_page, debug_sidebar_open, debug_sidebar_width, &pairing_code, &chat_screen, connected)),
            Some("debug_tap") => {
                let id = request["nodeId"].as_str().unwrap_or("");
                let action = match id {
                    "sidebar_toggle" | "sidebar_open" | "sidebar_close" | "edge_chat" | "edge_pair" | "edge_search" | "edge_new" | "tasks" | "settings" => id,
                    _ => "",
                };
                if action.is_empty() { Err("控件不存在或不可点击".into()) } else {
                    let route = match action { "tasks" => "builtin:Tasks", "settings" => "builtin:Settings", other => other };
                    apply_debug_action(route, &mut debug_page, &mut debug_sidebar_open, &mut debug_sidebar_width);
                    *lastAction.lock().unwrap() = route.to_string();
                    Ok(debug_snapshot(&debug_page, debug_sidebar_open, debug_sidebar_width, &pairing_code, &chat_screen, connected))
                }
            }
            Some("debug_swipe") => {
                let direction = request["direction"].as_str().unwrap_or("");
                if direction != "left" && direction != "right" && direction != "open" && direction != "close" {
                    Err("direction 必须是 left/right/open/close".into())
                } else {
                    apply_debug_action(if direction == "right" || direction == "open" { "sidebar_open" } else { "sidebar_close" }, &mut debug_page, &mut debug_sidebar_open, &mut debug_sidebar_width);
                    Ok(debug_snapshot(&debug_page, debug_sidebar_open, debug_sidebar_width, &pairing_code, &chat_screen, connected))
                }
            }
            Some("action") => {
                let action = request["action"].as_str().unwrap_or("");
                *lastAction.lock().unwrap() = action.to_string();
                apply_debug_action(action, &mut debug_page, &mut debug_sidebar_open, &mut debug_sidebar_width);
                Ok(serde_json::json!({"ok": true, "action": action}))
            }
            Some("send") => edge_chat::send(request["text"].as_str().unwrap_or("").into())
                .map(|_| serde_json::json!({"ok": true})),
            _ => Err("Unknown simulator command".into()),
        };
        match result {
            Ok(value) => emit(serde_json::json!({"id": id, "value": value})),
            Err(error) => emit(serde_json::json!({"id": id, "error": error})),
        }
    }
    if let Some((daemon, fullname)) = mdns {
        let _ = daemon.unregister(&fullname);
    }
    Ok(())
}
