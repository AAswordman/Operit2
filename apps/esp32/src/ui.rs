#![allow(non_snake_case)]

use std::collections::VecDeque;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Mutex;

use operit_board_esp32::{logicalDisplaySize, FaceRect, DISPLAY_ROTATION_DEGREES};
use operit_host_api::{HostError, HostResult};

use crate::status::FirmwareStatus;

#[repr(C)]
struct UiArea {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

type FlushCallback = unsafe extern "C" fn(*const UiArea, *const u8, usize, *mut c_void);
type ActionCallback = unsafe extern "C" fn(*const c_char, *mut c_void);

unsafe extern "C" {
    fn operit_mini_static_bytes() -> usize;
    fn operit_mini_draw_bytes() -> usize;
    fn operit_ui_init(
        width: u16,
        height: u16,
        flush_cb: Option<FlushCallback>,
        touch_cb: Option<unsafe extern "C" fn(*mut u16, *mut u16, *mut c_void) -> bool>,
        action_cb: Option<ActionCallback>,
        user_data: *mut c_void,
    ) -> bool;
    fn operit_ui_pump(elapsed_ms: u32);
    fn operit_ui_set_touch(x: u16, y: u16, pressed: bool);
    fn operit_ui_set_connection(wifi_ready: bool, edge_ready: bool);
    fn operit_ui_set_paired(paired: bool);
    fn operit_ui_set_expression(expression: *const c_char);
    fn operit_ui_set_pairing_code(code: *const c_char);
    fn operit_ui_set_space_state(state: *const c_char);
    fn operit_ui_set_space_join_prompt(text: *const c_char, busy: bool);
    fn operit_ui_set_chat_screen(text: *const c_char);
    fn operit_ui_set_chat_identity(id: *const c_char, character: *const c_char);
    fn operit_ui_set_message(index: u32, user: bool, text: *const c_char);
    fn operit_ui_set_chat_history(older: bool, newer: bool);
    fn operit_ui_finish_messages(count: u32);
    fn operit_ui_set_conversation(
        index: u32,
        id: *const c_char,
        title: *const c_char,
        character: *const c_char,
        selected: bool,
    );
    fn operit_ui_finish_conversations(count: u32);
    fn operit_ui_action_error(error: *const c_char);
    fn operit_ui_set_chat_task(text: *const c_char);
    fn operit_ui_set_plugin(index: u32, id: *const c_char, name: *const c_char, status: u32, latency_ms: u32);
    fn operit_ui_set_plugin_test(index: u32, status: u32, ms: u32, error: *const c_char);
    fn operit_ui_set_plugin_testing(testing: bool);
    fn operit_ui_set_plugin_category(exclusive: bool);
    fn operit_ui_set_plugin_details(id: *const c_char, description: *const c_char, tools: *const c_char,
        offset: u32, total: u32, loading: bool, error: *const c_char);
    fn operit_ui_finish_plugins(count: u32, offset: u32, total: u32, loading: bool, error: *const c_char);
    fn operit_ui_chat_send_result(ok: bool, error: *const c_char);
    fn operit_ui_set_chat_draft(value: *const c_char);
    fn operit_ui_chat_draft() -> *const c_char;
    fn operit_ui_debug_tree() -> *const c_char;
    fn operit_ui_debug_snapshot() -> *const c_char;
    fn operit_ui_debug_tap(id: *const c_char) -> bool;
    fn operit_ui_debug_swipe(direction: *const c_char) -> bool;
}

/// Owns the self-drawn UI runtime and the small action queue emitted by app buttons.
pub struct Esp32Ui {
    context: Box<UiContext>,
    lastTouch: Option<(u16, u16)>,
}

struct UiContext {
    board: *const operit_board_esp32::Esp32Board,
    actions: Mutex<VecDeque<String>>,
}

unsafe impl Send for UiContext {}

impl Esp32Ui {
    /// Initializes the self-drawn UI with the board's rotated logical display dimensions.
    pub fn new(board: &operit_board_esp32::Esp32Board) -> HostResult<Self> {
        let runtime = Self {
            context: Box::new(UiContext {
                board,
                actions: Mutex::new(VecDeque::new()),
            }),
            lastTouch: None,
        };
        let userData = runtime.context.as_ref() as *const UiContext as *mut c_void;
        let (width, height) = logicalDisplaySize(DISPLAY_ROTATION_DEGREES);
        let initialized = unsafe {
            operit_ui_init(
                width,
                height,
                Some(flushCallback),
                None,
                Some(actionCallback),
                userData,
            )
        };
        if !initialized {
            return Err(HostError::new("the self-drawn UI initialization failed"));
        }
        unsafe {
            log::info!("operit-esp32 UI renderer=mini static_bytes={} draw_bytes={} ui_heap=0",
                operit_mini_static_bytes(), operit_mini_draw_bytes());
        }
        Ok(runtime)
    }

    /// Feeds one sampled touch point to the self-drawn UI's pointer input device.
    pub fn setTouch(&mut self, point: Option<(u16, u16)>) {
        let (x, y, pressed) = match point {
            Some((x, y)) => {
                self.lastTouch = Some((x, y));
                (x, y, true)
            }
            // Keep the final sample on release so horizontal swipe recognition sees
            // the actual finger-up coordinate instead of (0, 0).
            None => self
                .lastTouch
                .map(|(x, y)| (x, y, false))
                .unwrap_or((0, 0, false)),
        };
        unsafe { operit_ui_set_touch(x, y, pressed) };
    }

    /// Advances the self-drawn UI animations and flushes pending display regions.
    pub fn pump(&mut self, elapsedMs: u32) {
        unsafe { operit_ui_pump(elapsedMs) };
    }

    /// Updates the connection indicators shown by the launcher and settings app.
    pub fn setConnection(&mut self, wifiReady: bool, edgeReady: bool) {
        unsafe { operit_ui_set_connection(wifiReady, edgeReady) };
    }

    pub fn setPaired(&mut self, paired: bool) {
        unsafe { operit_ui_set_paired(paired) };
    }

    /// Updates the face app's expression label.
    pub fn setExpression(&mut self, expression: &str) {
        let mut bytes = expression.as_bytes().to_vec();
        bytes.retain(|byte| *byte != 0);
        bytes.push(0);
        unsafe { operit_ui_set_expression(bytes.as_ptr() as *const c_char) };
    }

    pub fn setPairingCode(&mut self, code: &str) {
        setText(code, operit_ui_set_pairing_code);
    }
    pub fn setSpaceJoinPrompt(&mut self, text: &str, busy: bool) {
        let text = CString::new(text.replace('\0', "")).unwrap();
        unsafe { operit_ui_set_space_join_prompt(text.as_ptr(), busy); }
    }
    pub fn setSpaceState(&mut self, state: &str) {
        setText(state, operit_ui_set_space_state);
    }
    pub fn setChatScreen(&mut self, text: &str) {
        setText(text, operit_ui_set_chat_screen);
    }
    pub fn actionError(&mut self, error: &str) {
        setText(error, operit_ui_action_error);
    }

    /// Transfers only the six visible plugin summaries to the fixed renderer.
    pub fn setPlugins(&mut self, state: &serde_json::Value) {
        let string = |value: &str| CString::new(value.replace('\0', "")).unwrap();
        unsafe { operit_ui_set_plugin_category(state["category"] == "exclusive"); }
        let items = state["items"].as_array().into_iter().flatten().take(6).collect::<Vec<_>>();
        for (index, item) in items.iter().enumerate() {
            let status = match item["status"].as_str() {
                Some("probing") => 1, Some("success") => 2, Some("failure") => 3, _ => 0,
            };
            let toolStatus = match item["toolStatus"].as_str() {
                Some("probing") => 1, Some("success") => 2, Some("failure") => 3, _ => 0,
            };
            unsafe { operit_ui_set_plugin(index as u32, string(item["id"].as_str().unwrap_or("")).as_ptr(),
                string(item["name"].as_str().unwrap_or("")).as_ptr(), status,
                item["latencyMs"].as_u64().unwrap_or(0).min(99999) as u32); }
            unsafe { operit_ui_set_plugin_test(index as u32, toolStatus,
                item["toolLatencyMs"].as_u64().unwrap_or(0).min(99999) as u32,
                string(item["testError"].as_str().unwrap_or("")).as_ptr()); }
        }
        let details = &state["details"];
        let tools = details["tools"].as_array().into_iter().flatten().take(3).filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n");
        unsafe { operit_ui_set_plugin_details(string(details["id"].as_str().unwrap_or("")).as_ptr(),
            string(details["description"].as_str().unwrap_or("")).as_ptr(), string(&tools).as_ptr(),
            details["toolOffset"].as_u64().unwrap_or(0) as u32, details["toolTotal"].as_u64().unwrap_or(0) as u32,
            details["loading"] == true, string(details["error"].as_str().unwrap_or("")).as_ptr()); }
        unsafe { operit_ui_set_plugin_testing(state["testing"] == true); }
        unsafe { operit_ui_finish_plugins(items.len() as u32, state["offset"].as_u64().unwrap_or(0) as u32,
            state["total"].as_u64().unwrap_or(0) as u32, state["loading"] == true,
            string(state["error"].as_str().unwrap_or("")).as_ptr()); }
    }

    pub fn setChatState(&mut self, state: &serde_json::Value) {
        unsafe { operit_ui_set_chat_history(state["hasOlder"].as_bool().unwrap_or(false), state["hasNewer"].as_bool().unwrap_or(false)); }
        let string = |value: &str| CString::new(value.replace('\0', "")).unwrap();
        let id = state["chatId"].as_str().unwrap_or("");
        let name = state["conversations"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|chat| chat["id"].as_str() == Some(id))
            .and_then(|chat| chat["characterCardName"].as_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("Operit");
        unsafe {
            operit_ui_set_chat_identity(string(id).as_ptr(), string(&name).as_ptr());
        }
        let rows = state["messages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|row| {
                row["text"]
                    .as_str()
                    .is_some_and(|text| !text.trim().is_empty())
            })
            .collect::<Vec<_>>();
        let rows = &rows[rows.len().saturating_sub(12)..];
        for (index, row) in rows.iter().enumerate() {
            unsafe {
                operit_ui_set_message(
                    index as u32,
                    row["sender"] == "user",
                    string(row["text"].as_str().unwrap_or("")).as_ptr(),
                );
            }
        }
        unsafe {
            operit_ui_finish_messages(rows.len() as u32);
        }
        let rows = state["conversations"]
            .as_array()
            .into_iter()
            .flatten()
            .take(24)
            .collect::<Vec<_>>();
        for (index, row) in rows.iter().enumerate() {
            unsafe {
                operit_ui_set_conversation(
                    index as u32,
                    string(row["id"].as_str().unwrap_or("")).as_ptr(),
                    string(row["title"].as_str().unwrap_or("")).as_ptr(),
                    string(row["characterCardName"].as_str().unwrap_or("")).as_ptr(),
                    row["id"] == id,
                );
            }
        }
        unsafe {
            operit_ui_finish_conversations(rows.len() as u32);
        }
    }

    pub fn setChatTask(&mut self, text: &str) {
        setText(text, operit_ui_set_chat_task);
    }

    /// Returns the bounded draft captured by the the self-drawn UI text area.
    pub fn chatSendResult(&mut self, result: Result<(), String>) {
        let error = CString::new(
            result
                .as_ref()
                .err()
                .map(String::as_str)
                .unwrap_or("")
                .replace('\0', ""),
        )
        .unwrap();
        unsafe {
            operit_ui_chat_send_result(result.is_ok(), error.as_ptr());
        }
    }

    pub fn chatDraft(&self) -> String {
        unsafe {
            CStr::from_ptr(operit_ui_chat_draft())
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn debugOperation(&mut self, action: &str, argument: &str) -> Result<serde_json::Value, String> {
        let value = CString::new(argument).map_err(|e| e.to_string())?;
        unsafe {
            match action {
                "screen" | "tree" => {
                    let text = if action == "screen" { operit_ui_debug_snapshot() } else { operit_ui_debug_tree() };
                    if text.is_null() { return Err("UI unavailable".into()); }
                    serde_json::from_str(&CStr::from_ptr(text).to_string_lossy()).map_err(|e| e.to_string())
                }
                "draft" => { operit_ui_set_chat_draft(value.as_ptr()); Ok(serde_json::json!({"ok": true})) }
                "tap" | "swipe" => {
                    let ok = if action == "tap" { operit_ui_debug_tap(value.as_ptr()) } else { operit_ui_debug_swipe(value.as_ptr()) };
                    if ok { Ok(serde_json::json!({"ok": true})) } else { Err("Unknown, hidden, disabled or unsupported UI target".into()) }
                }
                _ => Err("Invalid debug operation".into()),
            }
        }
    }

    /// USB-only diagnostics, never called from the UART/peer worker thread.
    pub fn pollSerialDebug(&mut self, io: &operit_board_esp32::serial_debug::SerialDebugIo) {
        let Some(request) = io.take_request() else { return; };
        match request.operation() {
            1 | 2 if request.argument().is_empty() => unsafe {
                let text = if request.operation() == 1 { operit_ui_debug_snapshot() }
                    else { operit_ui_debug_tree() };
                if text.is_null() {
                    io.respond(request.id(), b"{\"error\":\"UI inspection unavailable\"}");
                } else {
                    io.respond(request.id(), CStr::from_ptr(text).to_bytes());
                }
            },
            5 if request.argument().is_empty() => {
                let snapshot = crate::runtimeHealthSnapshot().to_string();
                io.respond(request.id(), snapshot.as_bytes());
            }
            6 => {
                let result = std::str::from_utf8(request.argument()).map_err(|e| e.to_string())
                    .and_then(|argument| self.debugOperation("draft", argument));
                let value = result.unwrap_or_else(|error| serde_json::json!({"error": error})).to_string();
                io.respond(request.id(), value.as_bytes());
            }
            3 | 4 => {
                let argument = std::str::from_utf8(request.argument()).ok()
                    .and_then(|value| CString::new(value).ok());
                let ok = argument.as_ref().is_some_and(|value| unsafe {
                    if request.operation() == 3 { operit_ui_debug_tap(value.as_ptr()) }
                    else { operit_ui_debug_swipe(value.as_ptr()) }
                });
                io.respond(request.id(), if ok { b"{\"ok\":true}" }
                    else { b"{\"error\":\"Unknown, hidden, disabled or unsupported UI target\"}" });
            }
            _ => io.respond(request.id(), b"{\"error\":\"Invalid debug operation\"}"),
        }
    }

    /// Drains actions requested by the self-drawn UI app buttons.
    pub fn drainActions(&self) -> Vec<String> {
        self.context
            .actions
            .lock()
            .map(|mut actions| actions.drain(..).collect())
            .unwrap_or_default()
    }
}

fn setText(value: &str, setter: unsafe extern "C" fn(*const c_char)) {
    let mut bytes = value.as_bytes().to_vec();
    bytes.retain(|byte| *byte != 0);
    bytes.push(0);
    unsafe { setter(bytes.as_ptr() as *const c_char) };
}

unsafe extern "C" fn flushCallback(
    area: *const UiArea,
    pixels: *const u8,
    length: usize,
    userData: *mut c_void,
) {
    if area.is_null() || pixels.is_null() || userData.is_null() {
        return;
    }
    let context = &*(userData as *const UiContext);
    let area = &*area;
    let rect = FaceRect {
        x: area.x1.max(0) as u16,
        y: area.y1.max(0) as u16,
        width: (area.x2 - area.x1 + 1).max(0) as u16,
        height: (area.y2 - area.y1 + 1).max(0) as u16,
    };
    let pixels = std::slice::from_raw_parts(pixels, length);
    let board = &*context.board;
    if let Err(error) = board.flushUi(rect, pixels) {
        log::error!("operit-esp32 the self-drawn UI flush: {}", error.message);
    }
}

unsafe extern "C" fn actionCallback(action: *const c_char, userData: *mut c_void) {
    if action.is_null() || userData.is_null() {
        return;
    }
    let Ok(action) = CStr::from_ptr(action).to_str() else {
        return;
    };
    let context = &*(userData as *const UiContext);
    if let Ok(mut actions) = context.actions.lock() {
        actions.push_back(action.to_string());
    }
}

/// Keeps status-to-the self-drawn UI updates in one place for the firmware loop.
pub fn updateStatus(
    runtime: &mut Esp32Ui,
    status: &FirmwareStatus,
    edgeReady: bool,
    paired: bool,
) {
    let snapshot = status.snapshot();
    // `edgeReady` is the live authenticated Space route state. It must not be
    // derived from the TCP listener, which is enabled even before pairing.
    runtime.setConnection(snapshot.wifiConnected, edgeReady);
    // Install a newly received code before credentials refresh can navigate.
    runtime.setPairingCode(&snapshot.pairingCode);
    runtime.setPaired(paired);
    runtime.setExpression(&snapshot.expression);
    runtime.setSpaceState(if edgeReady {
        "Connected to Space"
    } else {
        "Waiting for Space"
    });
}
