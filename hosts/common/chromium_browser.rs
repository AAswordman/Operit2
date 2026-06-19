use std::env;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use operit_host_api::{
    BrowserAutomationHost, BrowserAutomationRequest, BrowserAutomationResponse, HostError,
    HostResult, WebVisitLinkData, WebVisitRequest, WebVisitResult,
};
use serde_json::{json, Map, Value};
use tungstenite::{connect, Message};

pub fn visit_with_chromium(
    request: WebVisitRequest,
    candidates: Vec<String>,
    extra_args: &[&str],
) -> HostResult<WebVisitResult> {
    let port = allocate_port()?;
    let profile_dir = create_profile_dir()?;
    let mut child = spawn_browser(&candidates, port, &profile_dir, extra_args)?;
    let result = run_cdp_visit(&request, port);
    cleanup_browser(&mut child, &profile_dir);
    result
}

pub struct ChromiumBrowserAutomationHost {
    candidates: Vec<String>,
    extra_args: Vec<String>,
    state: Mutex<Option<BrowserState>>,
}

impl ChromiumBrowserAutomationHost {
    pub fn new(candidates: Vec<String>, extra_args: Vec<String>) -> Self {
        Self {
            candidates,
            extra_args,
            state: Mutex::new(None),
        }
    }

    fn lock_state(&self) -> HostResult<std::sync::MutexGuard<'_, Option<BrowserState>>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("Browser automation state lock was poisoned"))
    }

    fn start_state(&self) -> HostResult<BrowserState> {
        let port = allocate_port()?;
        let profile_dir = create_profile_dir()?;
        let extra_args = self.extra_args.iter().map(String::as_str).collect::<Vec<_>>();
        let mut child = spawn_browser(&self.candidates, port, &profile_dir, &extra_args)?;
        match wait_for_browser(port) {
            Ok(()) => {}
            Err(error) => {
                cleanup_browser(&mut child, &profile_dir);
                return Err(error);
            }
        }
        match create_automation_tab(port) {
            Ok(tab) => Ok(BrowserState {
                port,
                profile_dir,
                child,
                tabs: vec![tab],
                active_index: 0,
            }),
            Err(error) => {
                cleanup_browser(&mut child, &profile_dir);
                Err(error)
            }
        }
    }

    fn with_state<R>(&self, action: impl FnOnce(&mut BrowserState) -> HostResult<R>) -> HostResult<R> {
        let mut guard = self.lock_state()?;
        if guard.is_none() {
            *guard = Some(self.start_state()?);
        }
        let state = guard
            .as_mut()
            .ok_or_else(|| HostError::new("Browser automation session was not created"))?;
        action(state)
    }

    fn with_existing_state<R>(
        &self,
        action: impl FnOnce(&mut BrowserState) -> HostResult<R>,
    ) -> HostResult<R> {
        let mut guard = self.lock_state()?;
        let state = guard
            .as_mut()
            .ok_or_else(|| HostError::new("No active browser session"))?;
        action(state)
    }

    fn handle_close(&self) -> HostResult<String> {
        let mut guard = self.lock_state()?;
        let Some(state) = guard.as_mut() else {
            return Ok("OK".to_string());
        };
        if state.tabs.len() <= 1 {
            let _ = guard.take();
            return Ok("OK".to_string());
        }
        state.close_tab(state.active_index)?;
        Ok("OK".to_string())
    }

    fn handle_close_all(&self) -> HostResult<String> {
        let mut guard = self.lock_state()?;
        let _ = guard.take();
        Ok("OK".to_string())
    }

    fn handle_tabs(&self, params: &BrowserToolParams) -> HostResult<String> {
        let action = params.required_trimmed("action")?;
        match action.as_str() {
            "list" => {
                let mut guard = self.lock_state()?;
                let Some(state) = guard.as_mut() else {
                    return Ok("[]".to_string());
                };
                state.tabs_json()
            }
            "create" => {
                let url = params.optional_trimmed("url")?;
                let mut guard = self.lock_state()?;
                if guard.is_none() {
                    *guard = Some(self.start_state()?);
                } else {
                    let state = guard
                        .as_mut()
                        .ok_or_else(|| HostError::new("No active browser session"))?;
                    let tab = create_automation_tab(state.port)?;
                    state.tabs.push(tab);
                    state.active_index = state.tabs.len() - 1;
                }
                let state = guard
                    .as_mut()
                    .ok_or_else(|| HostError::new("No active browser session"))?;
                if let Some(url) = url {
                    state.navigate(&url)?;
                }
                state.tabs_json()
            }
            "select" => self.with_existing_state(|state| {
                let index = params.required_usize("index")?;
                state.select_tab(index)?;
                state.tab_json(index)
            }),
            "close" => {
                let index = params.optional_usize("index")?;
                let mut guard = self.lock_state()?;
                let Some(state) = guard.as_mut() else {
                    return Ok("OK".to_string());
                };
                let close_index = index.unwrap_or(state.active_index);
                if state.tabs.len() <= 1 {
                    let _ = guard.take();
                    return Ok("OK".to_string());
                }
                state.close_tab(close_index)?;
                Ok("OK".to_string())
            }
            other => Err(HostError::new(format!(
                "Unsupported browser tab action: {other}"
            ))),
        }
    }
}

impl BrowserAutomationHost for ChromiumBrowserAutomationHost {
    fn executeBrowserTool(
        &self,
        request: BrowserAutomationRequest,
    ) -> HostResult<BrowserAutomationResponse> {
        let params = BrowserToolParams::parse(&request.parametersJson)?;
        let output = match request.toolName.as_str() {
            "browser_navigate" => self.with_state(|state| {
                let url = params.required_trimmed("url")?;
                state.navigate(&url)?;
                Ok(json!({ "url": url }).to_string())
            }),
            "browser_navigate_back" => self.with_existing_state(|state| {
                state.navigate_back()?;
                Ok("OK".to_string())
            }),
            "browser_close" => self.handle_close(),
            "browser_close_all" => self.handle_close_all(),
            "browser_tabs" => self.handle_tabs(&params),
            "browser_snapshot" => self.with_existing_state(|state| {
                let output = state.snapshot(
                    params.optional_trimmed("selector")?,
                    params.optional_i64("depth")?,
                )?;
                write_optional_text(&params, &output)
            }),
            "browser_console_messages" => self.with_existing_state(|state| {
                let output = state.console_messages(params.optional_trimmed("level")?)?;
                write_optional_text(&params, &output)
            }),
            "browser_network_requests" => self.with_existing_state(|state| {
                let output = state.network_requests(params.optional_bool("includeStatic")?)?;
                write_optional_text(&params, &output)
            }),
            "browser_click" => self.with_existing_state(|state| {
                state.click(
                    &target_from_params(&params)?,
                    params.optional_bool("doubleClick")?.unwrap_or(false),
                    params
                        .optional_trimmed("button")?
                        .unwrap_or_else(|| "left".to_string()),
                    params.optional_string_list("modifiers")?.unwrap_or_default(),
                )?;
                Ok("OK".to_string())
            }),
            "browser_type" => self.with_existing_state(|state| {
                state.type_text(
                    &target_from_params(&params)?,
                    &params.required_raw("text")?,
                    params.optional_bool("submit")?.unwrap_or(false),
                    params.optional_bool("slowly")?.unwrap_or(false),
                )?;
                Ok("OK".to_string())
            }),
            "browser_hover" => self.with_existing_state(|state| {
                state.hover(&target_from_params(&params)?)?;
                Ok("OK".to_string())
            }),
            "browser_drag" => self.with_existing_state(|state| {
                state.drag(
                    &params.required_trimmed("startRef")?,
                    &params.required_trimmed("endRef")?,
                )?;
                Ok("OK".to_string())
            }),
            "browser_fill_form" => self.with_existing_state(|state| {
                for field in params.required_fields("fields")? {
                    state.type_text(&field.target, &field.value, false, false)?;
                }
                Ok("OK".to_string())
            }),
            "browser_press_key" => self.with_existing_state(|state| {
                state.press_key(&params.required_trimmed("key")?)?;
                Ok("OK".to_string())
            }),
            "browser_select_option" => self.with_existing_state(|state| {
                state.select_option(
                    &params.required_trimmed("ref")?,
                    &params.required_string_list("values")?,
                )?;
                Ok("OK".to_string())
            }),
            "browser_evaluate" => self.with_existing_state(|state| {
                state.evaluate_function(
                    &params.required_raw("function")?,
                    params.optional_trimmed("ref")?,
                )
            }),
            "browser_run_code" => self.with_existing_state(|state| {
                state.run_code(&params.required_raw("code")?)
            }),
            "browser_wait_for" => self.with_existing_state(|state| {
                state.wait_for(
                    params.optional_f64("time")?,
                    params.optional_raw("text")?,
                    params.optional_raw("textGone")?,
                )
            }),
            "browser_resize" => self.with_existing_state(|state| {
                state.resize(params.required_i64("width")?, params.required_i64("height")?)?;
                Ok("OK".to_string())
            }),
            "browser_take_screenshot" => self.with_existing_state(|state| {
                state.take_screenshot(
                    params
                        .optional_trimmed("type")?
                        .unwrap_or_else(|| "png".to_string()),
                    params.optional_trimmed("ref")?,
                    params.optional_bool("fullPage")?.unwrap_or(false),
                    params.optional_trimmed("filename")?,
                )
            }),
            "browser_file_upload" => self.with_existing_state(|state| {
                state.file_upload(params.optional_string_list("paths")?.unwrap_or_default())?;
                Ok("OK".to_string())
            }),
            "browser_handle_dialog" => self.with_existing_state(|state| {
                state.handle_dialog(
                    params.required_bool("accept")?,
                    params.optional_raw("promptText")?,
                )?;
                Ok("OK".to_string())
            }),
            other => Err(HostError::new(format!(
                "Unknown browser automation tool: {other}"
            ))),
        }?;
        Ok(BrowserAutomationResponse { output })
    }
}

struct BrowserState {
    port: u16,
    profile_dir: PathBuf,
    child: Child,
    tabs: Vec<TabState>,
    active_index: usize,
}

impl Drop for BrowserState {
    fn drop(&mut self) {
        cleanup_browser(&mut self.child, &self.profile_dir);
    }
}

impl BrowserState {
    fn active_tab_mut(&mut self) -> HostResult<&mut TabState> {
        self.tabs
            .get_mut(self.active_index)
            .ok_or_else(|| HostError::new("No active browser tab"))
    }

    fn navigate(&mut self, url: &str) -> HostResult<()> {
        let tab = self.active_tab_mut()?;
        tab.session.command("Page.navigate", json!({ "url": url }))?;
        wait_for_document_ready(&mut tab.session)?;
        thread::sleep(Duration::from_millis(500));
        tab.collect_events()?;
        Ok(())
    }

    fn navigate_back(&mut self) -> HostResult<()> {
        let tab = self.active_tab_mut()?;
        let history = tab.session.command("Page.getNavigationHistory", json!({}))?;
        let current_index = history
            .get("currentIndex")
            .and_then(Value::as_i64)
            .ok_or_else(|| HostError::new("Browser history did not include currentIndex"))?;
        if current_index <= 0 {
            return Err(HostError::new("No previous browser history entry"));
        }
        let entries = history
            .get("entries")
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new("Browser history did not include entries"))?;
        let previous = entries
            .get((current_index - 1) as usize)
            .ok_or_else(|| HostError::new("Previous browser history entry was not found"))?;
        let entry_id = previous
            .get("id")
            .and_then(Value::as_i64)
            .ok_or_else(|| HostError::new("Previous browser history entry did not include id"))?;
        tab.session
            .command("Page.navigateToHistoryEntry", json!({ "entryId": entry_id }))?;
        wait_for_document_ready(&mut tab.session)?;
        tab.collect_events()?;
        Ok(())
    }

    fn select_tab(&mut self, index: usize) -> HostResult<()> {
        let target_id = self
            .tabs
            .get(index)
            .map(|tab| tab.target_id.clone())
            .ok_or_else(|| HostError::new(format!("Browser tab index out of bounds: {index}")))?;
        devtools_text(self.port, "GET", &format!("/json/activate/{target_id}"))?;
        self.active_index = index;
        Ok(())
    }

    fn close_tab(&mut self, index: usize) -> HostResult<()> {
        let target_id = self
            .tabs
            .get(index)
            .map(|tab| tab.target_id.clone())
            .ok_or_else(|| HostError::new(format!("Browser tab index out of bounds: {index}")))?;
        devtools_text(self.port, "GET", &format!("/json/close/{target_id}"))?;
        self.tabs.remove(index);
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len().saturating_sub(1);
        }
        Ok(())
    }

    fn tabs_json(&mut self) -> HostResult<String> {
        let mut tabs = Vec::with_capacity(self.tabs.len());
        for index in 0..self.tabs.len() {
            tabs.push(self.tab_value(index)?);
        }
        Ok(Value::Array(tabs).to_string())
    }

    fn tab_json(&mut self, index: usize) -> HostResult<String> {
        Ok(self.tab_value(index)?.to_string())
    }

    fn tab_value(&mut self, index: usize) -> HostResult<Value> {
        let active = index == self.active_index;
        let tab = self
            .tabs
            .get_mut(index)
            .ok_or_else(|| HostError::new(format!("Browser tab index out of bounds: {index}")))?;
        let page_state = tab.page_state()?;
        Ok(json!({
            "index": index,
            "active": active,
            "targetId": tab.target_id,
            "url": page_state.get("url").cloned().unwrap_or(Value::String(String::new())),
            "title": page_state.get("title").cloned().unwrap_or(Value::String(String::new()))
        }))
    }

    fn snapshot(&mut self, selector: Option<String>, depth: Option<i64>) -> HostResult<String> {
        if let Some(depth) = depth {
            if depth < 0 {
                return Err(HostError::new("browser_snapshot depth must be non-negative"));
            }
        }
        let script = snapshot_script(selector.as_deref(), depth);
        let tab = self.active_tab_mut()?;
        let output = evaluate_string(&mut tab.session, &script)?;
        tab.collect_events()?;
        Ok(output)
    }

    fn console_messages(&mut self, level: Option<String>) -> HostResult<String> {
        let tab = self.active_tab_mut()?;
        tab.collect_events()?;
        let threshold = level
            .as_deref()
            .map(console_level_threshold)
            .transpose()?;
        let items = tab
            .console_messages
            .iter()
            .filter(|item| match threshold {
                Some(threshold) => item
                    .get("level")
                    .and_then(Value::as_str)
                    .map(console_level_rank)
                    .transpose()
                    .map(|rank| rank.map_or(false, |r| r >= threshold))
                    .unwrap_or(false),
                None => true,
            })
            .cloned()
            .collect::<Vec<_>>();
        Ok(Value::Array(items).to_string())
    }

    fn network_requests(&mut self, include_static: Option<bool>) -> HostResult<String> {
        let include_static = include_static.unwrap_or(false);
        let tab = self.active_tab_mut()?;
        tab.collect_events()?;
        let items = tab
            .network_requests
            .iter()
            .filter(|item| include_static || !is_static_resource(item))
            .cloned()
            .collect::<Vec<_>>();
        Ok(Value::Array(items).to_string())
    }

    fn click(
        &mut self,
        target: &str,
        double_click: bool,
        button: String,
        modifiers: Vec<String>,
    ) -> HostResult<()> {
        let button = mouse_button(&button)?;
        let modifiers = modifier_bitmask(&modifiers)?;
        let rect = self.element_rect(target)?;
        let tab = self.active_tab_mut()?;
        tab.session.command(
            "Input.dispatchMouseEvent",
            json!({
                "type": "mouseMoved",
                "x": rect.x,
                "y": rect.y,
                "modifiers": modifiers
            }),
        )?;
        let count = if double_click { 2 } else { 1 };
        for click_count in 1..=count {
            tab.session.command(
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mousePressed",
                    "x": rect.x,
                    "y": rect.y,
                    "button": button,
                    "clickCount": click_count,
                    "modifiers": modifiers
                }),
            )?;
            tab.session.command(
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mouseReleased",
                    "x": rect.x,
                    "y": rect.y,
                    "button": button,
                    "clickCount": click_count,
                    "modifiers": modifiers
                }),
            )?;
        }
        tab.collect_events()?;
        Ok(())
    }

    fn hover(&mut self, target: &str) -> HostResult<()> {
        let rect = self.element_rect(target)?;
        let tab = self.active_tab_mut()?;
        tab.session.command(
            "Input.dispatchMouseEvent",
            json!({ "type": "mouseMoved", "x": rect.x, "y": rect.y }),
        )?;
        tab.collect_events()?;
        Ok(())
    }

    fn drag(&mut self, start_target: &str, end_target: &str) -> HostResult<()> {
        let start = self.element_rect(start_target)?;
        let end = self.element_rect(end_target)?;
        let tab = self.active_tab_mut()?;
        tab.session.command(
            "Input.dispatchMouseEvent",
            json!({ "type": "mouseMoved", "x": start.x, "y": start.y }),
        )?;
        tab.session.command(
            "Input.dispatchMouseEvent",
            json!({ "type": "mousePressed", "x": start.x, "y": start.y, "button": "left", "clickCount": 1 }),
        )?;
        for step in 1..=8 {
            let ratio = step as f64 / 8.0;
            tab.session.command(
                "Input.dispatchMouseEvent",
                json!({
                    "type": "mouseMoved",
                    "x": start.x + (end.x - start.x) * ratio,
                    "y": start.y + (end.y - start.y) * ratio,
                    "button": "left"
                }),
            )?;
        }
        tab.session.command(
            "Input.dispatchMouseEvent",
            json!({ "type": "mouseReleased", "x": end.x, "y": end.y, "button": "left", "clickCount": 1 }),
        )?;
        tab.collect_events()?;
        Ok(())
    }

    fn type_text(
        &mut self,
        target: &str,
        text: &str,
        submit: bool,
        slowly: bool,
    ) -> HostResult<()> {
        let script = type_script(target, text, slowly);
        let tab = self.active_tab_mut()?;
        evaluate_value(&mut tab.session, &script)?;
        if submit {
            self.press_key("Enter")?;
        } else {
            let tab = self.active_tab_mut()?;
            tab.collect_events()?;
        }
        Ok(())
    }

    fn press_key(&mut self, key: &str) -> HostResult<()> {
        let descriptor = key_descriptor(key);
        let tab = self.active_tab_mut()?;
        let mut down = json!({
            "type": "keyDown",
            "key": descriptor.key,
            "windowsVirtualKeyCode": descriptor.virtual_key_code,
            "nativeVirtualKeyCode": descriptor.virtual_key_code
        });
        if let Some(text) = descriptor.text.clone() {
            down["text"] = Value::String(text);
        }
        tab.session.command("Input.dispatchKeyEvent", down)?;
        tab.session.command(
            "Input.dispatchKeyEvent",
            json!({
                "type": "keyUp",
                "key": descriptor.key,
                "windowsVirtualKeyCode": descriptor.virtual_key_code,
                "nativeVirtualKeyCode": descriptor.virtual_key_code
            }),
        )?;
        tab.collect_events()?;
        Ok(())
    }

    fn select_option(&mut self, target: &str, values: &[String]) -> HostResult<()> {
        let script = select_option_script(target, values)?;
        let tab = self.active_tab_mut()?;
        evaluate_value(&mut tab.session, &script)?;
        tab.collect_events()?;
        Ok(())
    }

    fn evaluate_function(&mut self, function: &str, target: Option<String>) -> HostResult<String> {
        let script = evaluate_function_script(function, target.as_deref());
        let tab = self.active_tab_mut()?;
        let value = evaluate_value(&mut tab.session, &script)?;
        tab.collect_events()?;
        Ok(stringify_tool_value(value))
    }

    fn run_code(&mut self, code: &str) -> HostResult<String> {
        let tab = self.active_tab_mut()?;
        let value = evaluate_value(&mut tab.session, code)?;
        tab.collect_events()?;
        Ok(stringify_tool_value(value))
    }

    fn wait_for(
        &mut self,
        time: Option<f64>,
        text: Option<String>,
        text_gone: Option<String>,
    ) -> HostResult<String> {
        if let Some(seconds) = time {
            if seconds < 0.0 {
                return Err(HostError::new("browser_wait_for time must be non-negative"));
            }
            thread::sleep(Duration::from_secs_f64(seconds));
            return Ok("OK".to_string());
        }
        if let Some(text) = text {
            let tab = self.active_tab_mut()?;
            let value = evaluate_value(&mut tab.session, &wait_for_text_script(&text, false))?;
            tab.collect_events()?;
            return Ok(stringify_tool_value(value));
        }
        if let Some(text_gone) = text_gone {
            let tab = self.active_tab_mut()?;
            let value = evaluate_value(&mut tab.session, &wait_for_text_script(&text_gone, true))?;
            tab.collect_events()?;
            return Ok(stringify_tool_value(value));
        }
        Err(HostError::new("time, text, or textGone is required"))
    }

    fn resize(&mut self, width: i64, height: i64) -> HostResult<()> {
        if width <= 0 || height <= 0 {
            return Err(HostError::new("browser_resize width and height must be positive"));
        }
        let tab = self.active_tab_mut()?;
        tab.session.command(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "width": width,
                "height": height,
                "deviceScaleFactor": 1,
                "mobile": false
            }),
        )?;
        tab.collect_events()?;
        Ok(())
    }

    fn take_screenshot(
        &mut self,
        image_type: String,
        target: Option<String>,
        full_page: bool,
        filename: Option<String>,
    ) -> HostResult<String> {
        let format = match image_type.as_str() {
            "png" => "png",
            "jpeg" => "jpeg",
            other => {
                return Err(HostError::new(format!(
                    "browser_take_screenshot type must be png or jpeg, got {other}"
                )))
            }
        };
        let mut params = json!({ "format": format, "fromSurface": true });
        if let Some(target) = target.as_deref() {
            let rect = self.element_rect(target)?;
            params["clip"] = json!({
                "x": rect.left.max(0.0),
                "y": rect.top.max(0.0),
                "width": rect.width.max(1.0),
                "height": rect.height.max(1.0),
                "scale": 1
            });
        } else if full_page {
            let tab = self.active_tab_mut()?;
            let metrics = tab.session.command("Page.getLayoutMetrics", json!({}))?;
            let content = metrics
                .get("contentSize")
                .ok_or_else(|| HostError::new("Page.getLayoutMetrics did not return contentSize"))?;
            params["captureBeyondViewport"] = Value::Bool(true);
            params["clip"] = json!({
                "x": content.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                "y": content.get("y").and_then(Value::as_f64).unwrap_or(0.0),
                "width": content.get("width").and_then(Value::as_f64).unwrap_or(1.0).max(1.0),
                "height": content.get("height").and_then(Value::as_f64).unwrap_or(1.0).max(1.0),
                "scale": 1
            });
        }
        let tab = self.active_tab_mut()?;
        let result = tab.session.command("Page.captureScreenshot", params)?;
        tab.collect_events()?;
        let data = result
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| HostError::new("Page.captureScreenshot did not return image data"))?;
        if let Some(filename) = filename {
            let bytes = BASE64_STANDARD
                .decode(data.as_bytes())
                .map_err(|error| HostError::new(format!("Invalid screenshot base64: {error}")))?;
            return write_output_file(&filename, &bytes);
        }
        Ok(format!("data:image/{format};base64,{data}"))
    }

    fn file_upload(&mut self, paths: Vec<String>) -> HostResult<()> {
        for path in &paths {
            if !Path::new(path).is_absolute() {
                return Err(HostError::new(format!(
                    "browser_file_upload path must be absolute: {path}"
                )));
            }
        }
        let tab = self.active_tab_mut()?;
        tab.collect_events()?;
        let backend_node_id = tab
            .file_chooser_backend_node_id
            .take()
            .ok_or_else(|| HostError::new("No active browser file chooser"))?;
        tab.session.command(
            "DOM.setFileInputFiles",
            json!({ "files": paths, "backendNodeId": backend_node_id }),
        )?;
        tab.collect_events()?;
        Ok(())
    }

    fn handle_dialog(&mut self, accept: bool, prompt_text: Option<String>) -> HostResult<()> {
        let mut params = json!({ "accept": accept });
        if let Some(prompt_text) = prompt_text {
            params["promptText"] = Value::String(prompt_text);
        }
        let tab = self.active_tab_mut()?;
        tab.session.command("Page.handleJavaScriptDialog", params)?;
        tab.collect_events()?;
        Ok(())
    }

    fn element_rect(&mut self, target: &str) -> HostResult<ElementRect> {
        let script = element_rect_script(target);
        let tab = self.active_tab_mut()?;
        let raw = evaluate_string(&mut tab.session, &script)?;
        serde_json::from_str::<Value>(&raw)
            .map_err(|error| HostError::new(format!("Invalid element rect JSON: {error}")))
            .and_then(ElementRect::from_value)
    }
}

struct TabState {
    target_id: String,
    session: CdpSession,
    console_messages: Vec<Value>,
    network_requests: Vec<Value>,
    file_chooser_backend_node_id: Option<i64>,
}

impl TabState {
    fn collect_events(&mut self) -> HostResult<()> {
        self.session.drain_events(Duration::from_millis(80))?;
        let events = self.session.take_events();
        for event in events {
            self.record_event(event);
        }
        Ok(())
    }

    fn page_state(&mut self) -> HostResult<Value> {
        let raw = evaluate_string(
            &mut self.session,
            r#"JSON.stringify({ url: location.href, title: document.title || "" })"#,
        )?;
        self.collect_events()?;
        serde_json::from_str::<Value>(&raw)
            .map_err(|error| HostError::new(format!("Invalid browser page state JSON: {error}")))
    }

    fn record_event(&mut self, event: Value) {
        let method = event.get("method").and_then(Value::as_str);
        match method {
            Some("Runtime.consoleAPICalled") => {
                if let Some(item) = runtime_console_event(&event) {
                    push_limited(&mut self.console_messages, item);
                }
            }
            Some("Log.entryAdded") => {
                if let Some(item) = log_entry_event(&event) {
                    push_limited(&mut self.console_messages, item);
                }
            }
            Some("Network.requestWillBeSent") => {
                if let Some(item) = network_request_event(&event) {
                    push_limited(&mut self.network_requests, item);
                }
            }
            Some("Page.fileChooserOpened") => {
                self.file_chooser_backend_node_id = event
                    .get("params")
                    .and_then(|params| params.get("backendNodeId"))
                    .and_then(Value::as_i64);
            }
            _ => {}
        }
    }
}

fn create_automation_tab(port: u16) -> HostResult<TabState> {
    let target = devtools_json(port, "PUT", "/json/new")
        .map_err(|error| HostError::new(format!("Failed to create browser target: {error}")))?;
    let target_id = target
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::new("Browser target did not return id"))?
        .to_string();
    let ws_url = target
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::new("Browser target did not return a debugger WebSocket URL"))?;
    let mut session = CdpSession::connect(ws_url)?;
    session.command("Page.enable", json!({}))?;
    session.command("Runtime.enable", json!({}))?;
    session.command("Network.enable", json!({}))?;
    session.command("Log.enable", json!({}))?;
    session.command("DOM.enable", json!({}))?;
    session.command(
        "Page.setInterceptFileChooserDialog",
        json!({ "enabled": true }),
    )?;
    Ok(TabState {
        target_id,
        session,
        console_messages: Vec::new(),
        network_requests: Vec::new(),
        file_chooser_backend_node_id: None,
    })
}

fn allocate_port() -> HostResult<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| HostError::new(format!("Failed to allocate browser debug port: {error}")))?;
    listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|error| HostError::new(format!("Failed to read browser debug port: {error}")))
}

fn create_profile_dir() -> HostResult<PathBuf> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| HostError::new(error.to_string()))?
        .as_millis();
    let path = env::temp_dir().join(format!(
        "operit2_browser_{}_{}",
        std::process::id(),
        millis
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn spawn_browser(
    candidates: &[String],
    port: u16,
    profile_dir: &PathBuf,
    extra_args: &[&str],
) -> HostResult<Child> {
    let mut errors = Vec::new();
    for candidate in candidates {
        let mut command = Command::new(candidate);
        command
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-background-networking")
            .arg("--remote-debugging-address=127.0.0.1")
            .arg(format!("--remote-debugging-port={port}"))
            .arg(format!("--user-data-dir={}", profile_dir.display()))
            .args(extra_args)
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) => errors.push(format!("{candidate}: {error}")),
        }
    }
    Err(HostError::new(format!(
        "No Chromium-compatible browser could be launched. Tried: {}",
        errors.join(" | ")
    )))
}

fn cleanup_browser(child: &mut Child, profile_dir: &PathBuf) {
    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_dir_all(profile_dir);
}

fn run_cdp_visit(request: &WebVisitRequest, port: u16) -> HostResult<WebVisitResult> {
    wait_for_browser(port)?;
    let target = devtools_json(port, "PUT", "/json/new")
        .map_err(|error| HostError::new(format!("Failed to create browser target: {error}")))?;
    let ws_url = target
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::new("Browser target did not return a debugger WebSocket URL"))?;
    let mut session = CdpSession::connect(ws_url)?;
    session.command("Page.enable", json!({}))?;
    session.command("Runtime.enable", json!({}))?;
    if !request.headers.is_empty() {
        session.command("Network.enable", json!({}))?;
        let headers = request
            .headers
            .iter()
            .map(|(key, value)| (key.clone(), Value::String(value.clone())))
            .collect::<serde_json::Map<_, _>>();
        session.command("Network.setExtraHTTPHeaders", json!({ "headers": headers }))?;
    }
    if !request.userAgent.trim().is_empty() {
        session.command(
            "Network.setUserAgentOverride",
            json!({ "userAgent": request.userAgent }),
        )?;
    }
    session.command("Page.navigate", json!({ "url": request.url }))?;
    wait_for_document_ready(&mut session)?;
    thread::sleep(Duration::from_millis(900));
    scroll_page(&mut session)?;
    extract_page(&mut session, request.includeImageLinks)
}

fn wait_for_browser(port: u16) -> HostResult<()> {
    let started = Instant::now();
    loop {
        if devtools_json(port, "GET", "/json/version").is_ok() {
            return Ok(());
        }
        if started.elapsed() >= Duration::from_secs(10) {
            return Err(HostError::new("Timed out waiting for browser DevTools endpoint"));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn devtools_json(port: u16, method: &str, path: &str) -> HostResult<Value> {
    let body = devtools_text(port, method, path)?;
    serde_json::from_str::<Value>(&body)
        .map_err(|error| HostError::new(format!("Invalid browser DevTools JSON: {error}")))
}

fn devtools_text(port: u16, method: &str, path: &str) -> HostResult<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .map_err(|error| HostError::new(format!("Browser DevTools connection failed: {error}")))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| HostError::new(format!("Browser DevTools request failed: {error}")))?;
    let response = read_http_response(&mut stream)?;
    let response_text = String::from_utf8(response)
        .map_err(|error| HostError::new(format!("Browser DevTools response was not UTF-8: {error}")))?;
    let Some((headers, body)) = response_text.split_once("\r\n\r\n") else {
        return Err(HostError::new("Browser DevTools response was malformed"));
    };
    let status_line = headers.lines().next().unwrap_or_default();
    if parse_http_status(status_line) != Some(200) {
        return Err(HostError::new(format!(
            "Browser DevTools returned {status_line}"
        )));
    }
    Ok(body.to_string())
}

fn parse_http_status(status_line: &str) -> Option<u16> {
    let mut parts = status_line.split_whitespace();
    let _version = parts.next()?;
    parts.next()?.parse::<u16>().ok()
}

fn read_http_response(stream: &mut TcpStream) -> HostResult<Vec<u8>> {
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream
            .read(&mut buffer)
            .map_err(|error| HostError::new(format!("Browser DevTools response failed: {error}")))?;
        if count == 0 {
            return Ok(response);
        }
        response.extend_from_slice(&buffer[..count]);
        let Some(header_end) = find_bytes(&response, b"\r\n\r\n") else {
            continue;
        };
        let header_text = String::from_utf8_lossy(&response[..header_end]);
        let content_length = header_text.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        });
        if let Some(content_length) = content_length {
            if response.len() >= header_end + 4 + content_length {
                return Ok(response);
            }
        }
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

struct CdpSession {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    next_id: i64,
    pending_events: Vec<Value>,
}

impl CdpSession {
    fn connect(ws_url: &str) -> HostResult<Self> {
        let (mut socket, _) = connect(ws_url)
            .map_err(|error| HostError::new(format!("Failed to connect browser debugger: {error}")))?;
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_mut() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
        }
        Ok(Self {
            socket,
            next_id: 1,
            pending_events: Vec::new(),
        })
    }

    fn command(&mut self, method: &str, params: Value) -> HostResult<Value> {
        self.set_read_timeout(Duration::from_secs(8));
        let id = self.next_id;
        self.next_id += 1;
        self.socket
            .send(Message::Text(
                json!({ "id": id, "method": method, "params": params }).to_string().into(),
            ))
            .map_err(|error| HostError::new(format!("Failed to send browser command {method}: {error}")))?;
        loop {
            let message = self
                .socket
                .read()
                .map_err(|error| HostError::new(format!("Failed to read browser command {method}: {error}")))?;
            let Message::Text(text) = message else {
                continue;
            };
            let value = serde_json::from_str::<Value>(&text)
                .map_err(|error| HostError::new(format!("Invalid browser command response: {error}")))?;
            if value.get("id").and_then(Value::as_i64) == Some(id) {
                if let Some(error) = value.get("error") {
                    return Err(HostError::new(format!(
                        "Browser command {method} failed: {error}"
                    )));
                }
                return Ok(value.get("result").cloned().unwrap_or(Value::Null));
            }
            self.pending_events.push(value);
        }
    }

    fn drain_events(&mut self, duration: Duration) -> HostResult<()> {
        self.set_read_timeout(Duration::from_millis(40));
        let started = Instant::now();
        while started.elapsed() < duration {
            match self.socket.read() {
                Ok(Message::Text(text)) => {
                    let value = serde_json::from_str::<Value>(&text).map_err(|error| {
                        HostError::new(format!("Invalid browser event payload: {error}"))
                    })?;
                    self.pending_events.push(value);
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(error))
                    if error.kind() == ErrorKind::WouldBlock
                        || error.kind() == ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(tungstenite::Error::ConnectionClosed) | Err(tungstenite::Error::AlreadyClosed) => {
                    break;
                }
                Err(error) => {
                    self.set_read_timeout(Duration::from_secs(8));
                    return Err(HostError::new(format!(
                        "Failed to read browser event: {error}"
                    )));
                }
            }
        }
        self.set_read_timeout(Duration::from_secs(8));
        Ok(())
    }

    fn take_events(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.pending_events)
    }

    fn set_read_timeout(&mut self, timeout: Duration) {
        if let tungstenite::stream::MaybeTlsStream::Plain(stream) = self.socket.get_mut() {
            let _ = stream.set_read_timeout(Some(timeout));
        }
    }
}

fn wait_for_document_ready(session: &mut CdpSession) -> HostResult<()> {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        let ready_state = evaluate_string(session, "document.readyState")?;
        if ready_state == "complete" {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(HostError::new("Timed out waiting for document.readyState=complete"))
}

fn scroll_page(session: &mut CdpSession) -> HostResult<()> {
    for _ in 0..4 {
        let _ = session.command(
            "Runtime.evaluate",
            json!({
                "expression": "window.scrollTo(0, document.body ? document.body.scrollHeight : document.documentElement.scrollHeight);",
                "returnByValue": true
            }),
        )?;
        thread::sleep(Duration::from_millis(350));
    }
    Ok(())
}

fn evaluate_string(session: &mut CdpSession, expression: &str) -> HostResult<String> {
    let value = evaluate_value(session, expression)?;
    Ok(match value {
        Value::String(text) => text,
        other => other.to_string(),
    })
}

fn evaluate_value(session: &mut CdpSession, expression: &str) -> HostResult<Value> {
    let result = session.command(
        "Runtime.evaluate",
        json!({
            "expression": expression,
            "returnByValue": true,
            "awaitPromise": true,
            "userGesture": true
        }),
    )?;
    if let Some(details) = result.get("exceptionDetails") {
        return Err(HostError::new(format!(
            "Browser JavaScript evaluation failed: {details}"
        )));
    }
    let remote = result
        .get("result")
        .ok_or_else(|| HostError::new("Browser JavaScript evaluation did not return result"))?;
    if remote.get("subtype").and_then(Value::as_str) == Some("null") {
        return Ok(Value::Null);
    }
    if let Some(value) = remote.get("value") {
        return Ok(value.clone());
    }
    if remote.get("type").and_then(Value::as_str) == Some("undefined") {
        return Ok(Value::String("undefined".to_string()));
    }
    Ok(Value::String(
        remote
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    ))
}

fn extract_page(session: &mut CdpSession, include_images: bool) -> HostResult<WebVisitResult> {
    let script = extraction_script(include_images);
    let raw = evaluate_string(session, &script)?;
    let value = serde_json::from_str::<Value>(&raw)
        .map_err(|error| HostError::new(format!("Invalid extracted browser JSON: {error}")))?;
    let links = value
        .get("links")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(WebVisitLinkData {
                url: item.get("url")?.as_str()?.to_string(),
                text: item.get("text")?.as_str()?.to_string(),
            })
        })
        .collect::<Vec<_>>();
    let metadata = value
        .get("metadata")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|object| {
            object.iter().filter_map(|(key, value)| {
                value.as_str().map(|value| (key.clone(), value.to_string()))
            })
        })
        .collect::<Vec<_>>();
    let image_links = value
        .get("imageLinks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    Ok(WebVisitResult {
        url: value
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Web Page")
            .to_string(),
        content: value
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        metadata,
        links,
        imageLinks: image_links,
    })
}

fn extraction_script(include_images: bool) -> String {
    format!(
        r#"
JSON.stringify((() => {{
  const includeImages = {include_images};
  const absoluteUrl = (value) => {{
    try {{ return new URL(value, location.href).href; }} catch (_) {{ return ""; }}
  }};
  const cleanText = (value) => String(value || "")
    .replace(/\u00a0/g, " ")
    .split(/\r?\n/)
    .map(line => line.trim())
    .filter((line, index, arr) => line.length > 0 || (index > 0 && arr[index - 1].trim().length > 0))
    .join("\n")
    .trim();
  const metadata = {{}};
  for (const meta of Array.from(document.querySelectorAll("meta"))) {{
    const key = meta.getAttribute("name") || meta.getAttribute("property");
    const content = meta.getAttribute("content");
    if (key && content && !metadata[key]) metadata[key] = content;
  }}
  const seenLinks = new Set();
  const links = [];
  for (const anchor of Array.from(document.querySelectorAll("a[href]"))) {{
    const url = absoluteUrl(anchor.getAttribute("href"));
    const text = cleanText(anchor.innerText || anchor.getAttribute("aria-label") || anchor.getAttribute("title") || url);
    const signature = url + "\n" + text;
    if (!url || !text || seenLinks.has(signature)) continue;
    seenLinks.add(signature);
    links.push({{ url, text }});
  }}
  const imageLinks = [];
  if (includeImages) {{
    const seenImages = new Set();
    for (const image of Array.from(document.querySelectorAll("img"))) {{
      const src = image.currentSrc || image.getAttribute("src") || image.getAttribute("data-src") || "";
      const url = absoluteUrl(src);
      if (!url || url.startsWith("data:") || url.startsWith("blob:") || seenImages.has(url)) continue;
      seenImages.add(url);
      imageLinks.push(url);
    }}
  }}
  const title = cleanText(document.title || (document.querySelector("h1") && document.querySelector("h1").innerText) || "Web Page");
  const content = cleanText(document.body ? document.body.innerText : document.documentElement.innerText);
  return {{ url: location.href, title, content, metadata, links, imageLinks }};
}})())
"#
    )
}

fn resolver_script(target: &str) -> String {
    let encoded = js_string(target);
    format!(
        r#"
(function() {{
  const target = {encoded};
  const selector = 'a,button,input,textarea,select,[role],[contenteditable=""],[contenteditable="true"]';
  const refParts = String(target).split(':');
  if (refParts[0] === 'el') {{
    let doc = document;
    for (let index = 1; index < refParts.length - 1; index++) {{
      const frameMatch = /^f(\d+)$/.exec(refParts[index]);
      if (!frameMatch) throw new Error('Invalid browser_snapshot ref: ' + target);
      const frame = Array.from(doc.querySelectorAll('iframe'))[Number(frameMatch[1])];
      if (!frame || !frame.contentDocument) throw new Error('Frame is not available for ref: ' + target);
      doc = frame.contentDocument;
    }}
    const elementIndex = Number(refParts[refParts.length - 1]);
    if (!Number.isInteger(elementIndex)) throw new Error('Invalid browser_snapshot ref: ' + target);
    return Array.from(doc.querySelectorAll(selector))[elementIndex] || null;
  }}
  return document.querySelector(target);
}})()
"#
    )
}

fn snapshot_script(selector: Option<&str>, depth: Option<i64>) -> String {
    let selector_value = selector
        .map(js_string)
        .unwrap_or_else(|| "null".to_string());
    let depth_value = depth
        .map(|depth| depth.to_string())
        .unwrap_or_else(|| "null".to_string());
    format!(
        r#"
JSON.stringify((() => {{
  const interactiveSelector = 'a,button,input,textarea,select,[role],[contenteditable=""],[contenteditable="true"]';
  const rootSelector = {selector_value};
  const depthLimit = {depth_value};
  const output = [];
  const maxItems = 300;
  function textOf(el) {{
    const label = el.getAttribute('aria-label') || el.getAttribute('placeholder') || el.title || '';
    const rawText = 'value' in el ? el.value : (el.innerText || el.textContent || '');
    return String(rawText || label || '').trim().slice(0, 160);
  }}
  function isVisible(el) {{
    const rect = el.getBoundingClientRect();
    const style = getComputedStyle(el);
    return rect.width > 0 && rect.height > 0 && style.visibility !== 'hidden' && style.display !== 'none';
  }}
  function inRoot(root, el) {{
    return root.nodeType === 9 || root === el || root.contains(el);
  }}
  function withinDepth(root, el) {{
    if (depthLimit === null) return true;
    if (root.nodeType === 9) return true;
    let depth = 0;
    let node = el;
    while (node && node !== root) {{
      depth += 1;
      node = node.parentElement;
    }}
    return node === root && depth <= depthLimit;
  }}
  function collect(doc, root, framePath, originX, originY) {{
    const all = Array.from(doc.querySelectorAll(interactiveSelector));
    for (let index = 0; index < all.length && output.length < maxItems; index++) {{
      const el = all[index];
      if (!inRoot(root, el) || !withinDepth(root, el) || !isVisible(el)) continue;
      const rect = el.getBoundingClientRect();
      const label = el.getAttribute('aria-label') || el.getAttribute('placeholder') || el.title || '';
      output.push({{
        ref: framePath.concat([index]).join(':'),
        tag: el.tagName.toLowerCase(),
        role: el.getAttribute('role') || '',
        label: String(label).trim().slice(0, 160),
        text: textOf(el),
        x: Math.round(originX + rect.x),
        y: Math.round(originY + rect.y),
        width: Math.round(rect.width),
        height: Math.round(rect.height)
      }});
    }}
    const frames = Array.from(doc.querySelectorAll('iframe'));
    for (let frameIndex = 0; frameIndex < frames.length && output.length < maxItems; frameIndex++) {{
      const frame = frames[frameIndex];
      if (!inRoot(root, frame)) continue;
      try {{
        const rect = frame.getBoundingClientRect();
        if (frame.contentDocument) {{
          collect(frame.contentDocument, frame.contentDocument, framePath.concat(['f' + frameIndex]), originX + rect.x, originY + rect.y);
        }}
      }} catch (error) {{}}
    }}
  }}
  const root = rootSelector ? document.querySelector(rootSelector) : document;
  if (!root) throw new Error('browser_snapshot selector did not match: ' + rootSelector);
  collect(document, root, ['el'], 0, 0);
  return output;
}})())
"#
    )
}

fn element_rect_script(target: &str) -> String {
    let resolver = resolver_script(target);
    let encoded = js_string(target);
    format!(
        r#"
JSON.stringify((() => {{
  const el = {resolver};
  if (!el) throw new Error('Element not found: ' + {encoded});
  el.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }});
  const rect = el.getBoundingClientRect();
  return {{
    x: rect.left + rect.width / 2,
    y: rect.top + rect.height / 2,
    left: rect.left,
    top: rect.top,
    width: rect.width,
    height: rect.height
  }};
}})())
"#
    )
}

fn type_script(target: &str, text: &str, slowly: bool) -> String {
    let resolver = resolver_script(target);
    let text = js_string(text);
    if slowly {
        return format!(
            r#"
(async () => {{
  const el = {resolver};
  if (!el) throw new Error('Element not found');
  el.focus();
  if ('value' in el) el.value = '';
  else el.textContent = '';
  const text = {text};
  for (const ch of Array.from(text)) {{
    document.execCommand('insertText', false, ch);
    await new Promise(resolve => setTimeout(resolve, 25));
  }}
  el.dispatchEvent(new Event('input', {{ bubbles: true }}));
  el.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return true;
}})()
"#
        );
    }
    format!(
        r#"
(() => {{
  const el = {resolver};
  if (!el) throw new Error('Element not found');
  el.focus();
  const text = {text};
  if ('value' in el) el.value = text;
  else el.textContent = text;
  el.dispatchEvent(new Event('input', {{ bubbles: true }}));
  el.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return true;
}})()
"#
    )
}

fn select_option_script(target: &str, values: &[String]) -> HostResult<String> {
    let resolver = resolver_script(target);
    let values = serde_json::to_string(values)
        .map_err(|error| HostError::new(format!("Invalid select option values: {error}")))?;
    Ok(format!(
        r#"
(() => {{
  const el = {resolver};
  if (!el) throw new Error('Element not found');
  const values = {values};
  let selected = 0;
  for (const option of Array.from(el.options || [])) {{
    const shouldSelect = values.indexOf(option.value) >= 0 || values.indexOf(option.text) >= 0;
    option.selected = shouldSelect;
    if (shouldSelect) selected += 1;
  }}
  el.dispatchEvent(new Event('input', {{ bubbles: true }}));
  el.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return selected;
}})()
"#
    ))
}

fn evaluate_function_script(function: &str, target: Option<&str>) -> String {
    match target {
        Some(target) => {
            let resolver = resolver_script(target);
            format!(
                r#"
(async () => {{
  const fn = ({function});
  const el = {resolver};
  if (!el) throw new Error('Element not found');
  return await fn(el);
}})()
"#
            )
        }
        None => format!(
            r#"
(async () => {{
  const fn = ({function});
  return await fn();
}})()
"#
        ),
    }
}

fn wait_for_text_script(text: &str, gone: bool) -> String {
    let text = js_string(text);
    let condition = if gone {
        "!document.body || document.body.innerText.indexOf(target) < 0"
    } else {
        "document.body && document.body.innerText.indexOf(target) >= 0"
    };
    format!(
        r#"
new Promise(resolve => {{
  const target = {text};
  const startedAt = Date.now();
  const timer = setInterval(() => {{
    if ({condition}) {{
      clearInterval(timer);
      resolve(true);
      return;
    }}
    if (Date.now() - startedAt > 10000) {{
      clearInterval(timer);
      resolve(false);
    }}
  }}, 100);
}})
"#
    )
}

fn js_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string literal should not fail")
}

fn stringify_tool_value(value: Value) -> String {
    match value {
        Value::String(text) => text,
        other => other.to_string(),
    }
}

#[derive(Clone, Copy)]
struct ElementRect {
    x: f64,
    y: f64,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

impl ElementRect {
    fn from_value(value: Value) -> HostResult<Self> {
        Ok(Self {
            x: required_f64_field(&value, "x")?,
            y: required_f64_field(&value, "y")?,
            left: required_f64_field(&value, "left")?,
            top: required_f64_field(&value, "top")?,
            width: required_f64_field(&value, "width")?,
            height: required_f64_field(&value, "height")?,
        })
    }
}

fn required_f64_field(value: &Value, name: &str) -> HostResult<f64> {
    value
        .get(name)
        .and_then(Value::as_f64)
        .ok_or_else(|| HostError::new(format!("Element rect did not include {name}")))
}

struct BrowserToolParams {
    values: Map<String, Value>,
}

impl BrowserToolParams {
    fn parse(raw: &str) -> HostResult<Self> {
        let value = serde_json::from_str::<Value>(raw)
            .map_err(|error| HostError::new(format!("Invalid browser tool parameters JSON: {error}")))?;
        let values = value
            .as_object()
            .cloned()
            .ok_or_else(|| HostError::new("Browser tool parameters must be a JSON object"))?;
        Ok(Self { values })
    }

    fn optional_raw(&self, name: &str) -> HostResult<Option<String>> {
        let Some(value) = self.values.get(name) else {
            return Ok(None);
        };
        let Value::String(text) = value else {
            return Err(HostError::new(format!(
                "Browser tool parameter {name} must be a string"
            )));
        };
        if text.trim().is_empty() {
            return Ok(None);
        }
        Ok(Some(text.clone()))
    }

    fn required_raw(&self, name: &str) -> HostResult<String> {
        let value = self
            .optional_raw(name)?
            .ok_or_else(|| HostError::new(format!("Missing parameter: {name}")))?;
        Ok(value)
    }

    fn optional_trimmed(&self, name: &str) -> HostResult<Option<String>> {
        Ok(self.optional_raw(name)?.map(|value| value.trim().to_string()))
    }

    fn required_trimmed(&self, name: &str) -> HostResult<String> {
        let value = self
            .optional_trimmed(name)?
            .ok_or_else(|| HostError::new(format!("Missing parameter: {name}")))?;
        Ok(value)
    }

    fn optional_i64(&self, name: &str) -> HostResult<Option<i64>> {
        self.optional_trimmed(name)?
            .map(|value| {
                value.parse::<i64>().map_err(|error| {
                    HostError::new(format!("Browser tool parameter {name} must be an integer: {error}"))
                })
            })
            .transpose()
    }

    fn required_i64(&self, name: &str) -> HostResult<i64> {
        self.optional_i64(name)?
            .ok_or_else(|| HostError::new(format!("Missing parameter: {name}")))
    }

    fn optional_usize(&self, name: &str) -> HostResult<Option<usize>> {
        self.optional_trimmed(name)?
            .map(|value| {
                value.parse::<usize>().map_err(|error| {
                    HostError::new(format!(
                        "Browser tool parameter {name} must be a non-negative integer: {error}"
                    ))
                })
            })
            .transpose()
    }

    fn required_usize(&self, name: &str) -> HostResult<usize> {
        self.optional_usize(name)?
            .ok_or_else(|| HostError::new(format!("Missing parameter: {name}")))
    }

    fn optional_f64(&self, name: &str) -> HostResult<Option<f64>> {
        self.optional_trimmed(name)?
            .map(|value| {
                value.parse::<f64>().map_err(|error| {
                    HostError::new(format!("Browser tool parameter {name} must be a number: {error}"))
                })
            })
            .transpose()
    }

    fn optional_bool(&self, name: &str) -> HostResult<Option<bool>> {
        self.optional_trimmed(name)?
            .map(|value| parse_bool(name, &value))
            .transpose()
    }

    fn required_bool(&self, name: &str) -> HostResult<bool> {
        self.optional_bool(name)?
            .ok_or_else(|| HostError::new(format!("Missing parameter: {name}")))
    }

    fn optional_string_list(&self, name: &str) -> HostResult<Option<Vec<String>>> {
        self.optional_raw(name)?
            .map(|raw| parse_string_list(name, &raw))
            .transpose()
    }

    fn required_string_list(&self, name: &str) -> HostResult<Vec<String>> {
        self.optional_string_list(name)?
            .ok_or_else(|| HostError::new(format!("Missing parameter: {name}")))
    }

    fn required_fields(&self, name: &str) -> HostResult<Vec<FormField>> {
        let raw = self.required_raw(name)?;
        let value = serde_json::from_str::<Value>(&raw).map_err(|error| {
            HostError::new(format!("Browser tool parameter {name} must be a JSON array: {error}"))
        })?;
        let fields = value
            .as_array()
            .ok_or_else(|| HostError::new(format!("Browser tool parameter {name} must be a JSON array")))?;
        fields.iter().map(FormField::from_value).collect()
    }
}

fn parse_bool(name: &str, value: &str) -> HostResult<bool> {
    match value {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        other => Err(HostError::new(format!(
            "Browser tool parameter {name} must be true or false, got {other}"
        ))),
    }
}

fn parse_string_list(name: &str, raw: &str) -> HostResult<Vec<String>> {
    let value = serde_json::from_str::<Value>(raw).map_err(|error| {
        HostError::new(format!("Browser tool parameter {name} must be a JSON array: {error}"))
    })?;
    let array = value
        .as_array()
        .ok_or_else(|| HostError::new(format!("Browser tool parameter {name} must be a JSON array")))?;
    array
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| HostError::new(format!("Browser tool parameter {name} must contain strings")))
        })
        .collect()
}

struct FormField {
    target: String,
    value: String,
}

impl FormField {
    fn from_value(value: &Value) -> HostResult<Self> {
        let object = value
            .as_object()
            .ok_or_else(|| HostError::new("browser_fill_form field must be an object"))?;
        let target = object
            .get("ref")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                object
                    .get("selector")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            })
            .ok_or_else(|| HostError::new("browser_fill_form field is missing ref or selector"))?;
        let raw_value = object
            .get("value")
            .ok_or_else(|| HostError::new("browser_fill_form field is missing value"))?;
        let value = match raw_value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        Ok(Self { target, value })
    }
}

fn target_from_params(params: &BrowserToolParams) -> HostResult<String> {
    if let Some(target) = params.optional_trimmed("ref")? {
        return Ok(target);
    }
    if let Some(target) = params.optional_trimmed("selector")? {
        return Ok(target);
    }
    Err(HostError::new("ref or selector is required"))
}

fn write_optional_text(params: &BrowserToolParams, output: &str) -> HostResult<String> {
    if let Some(filename) = params.optional_trimmed("filename")? {
        write_output_file(&filename, output.as_bytes())
    } else {
        Ok(output.to_string())
    }
}

fn write_output_file(filename: &str, bytes: &[u8]) -> HostResult<String> {
    let path = output_path(filename)?;
    fs::write(&path, bytes)?;
    Ok(path.display().to_string())
}

fn output_path(filename: &str) -> HostResult<PathBuf> {
    let path = PathBuf::from(filename);
    if path.is_absolute() {
        return Ok(path);
    }
    env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|error| HostError::new(format!("Failed to resolve output path: {error}")))
}

fn push_limited(items: &mut Vec<Value>, item: Value) {
    items.insert(0, item);
    if items.len() > 300 {
        items.truncate(300);
    }
}

fn runtime_console_event(event: &Value) -> Option<Value> {
    let params = event.get("params")?;
    let level = params.get("type").and_then(Value::as_str).unwrap_or("info");
    let args = params
        .get("args")
        .and_then(Value::as_array)
        .map(|args| {
            args.iter()
                .map(console_arg_text)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    Some(json!({
        "level": normalize_console_level(level),
        "message": args,
        "timestamp": current_millis()
    }))
}

fn log_entry_event(event: &Value) -> Option<Value> {
    let entry = event.get("params")?.get("entry")?;
    let level = entry.get("level").and_then(Value::as_str).unwrap_or("info");
    let message = entry.get("text").and_then(Value::as_str).unwrap_or("");
    Some(json!({
        "level": normalize_console_level(level),
        "message": message,
        "timestamp": current_millis()
    }))
}

fn console_arg_text(value: &Value) -> String {
    if let Some(inner) = value.get("value") {
        match inner {
            Value::String(text) => return text.clone(),
            other => return other.to_string(),
        }
    }
    value
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn normalize_console_level(level: &str) -> &str {
    match level {
        "error" => "error",
        "warning" | "warn" => "warning",
        "debug" | "verbose" => "debug",
        _ => "info",
    }
}

fn console_level_threshold(level: &str) -> HostResult<i64> {
    console_level_rank(level)
}

fn console_level_rank(level: &str) -> HostResult<i64> {
    match normalize_console_level(level) {
        "debug" => Ok(0),
        "info" => Ok(1),
        "warning" => Ok(2),
        "error" => Ok(3),
        other => Err(HostError::new(format!(
            "Unsupported console level: {other}"
        ))),
    }
}

fn network_request_event(event: &Value) -> Option<Value> {
    let params = event.get("params")?;
    let request = params.get("request")?;
    Some(json!({
        "requestId": params.get("requestId").cloned().unwrap_or(Value::Null),
        "url": request.get("url").cloned().unwrap_or(Value::String(String::new())),
        "method": request.get("method").cloned().unwrap_or(Value::String(String::new())),
        "resourceType": params.get("type").cloned().unwrap_or(Value::String(String::new())),
        "timestamp": current_millis()
    }))
}

fn is_static_resource(item: &Value) -> bool {
    matches!(
        item.get("resourceType").and_then(Value::as_str),
        Some("Image" | "Stylesheet" | "Font" | "Media" | "Script")
    )
}

fn current_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn mouse_button(button: &str) -> HostResult<&'static str> {
    match button {
        "left" => Ok("left"),
        "right" => Ok("right"),
        "middle" => Ok("middle"),
        other => Err(HostError::new(format!(
            "Mouse button must be left, right, or middle, got {other}"
        ))),
    }
}

fn modifier_bitmask(modifiers: &[String]) -> HostResult<i64> {
    let mut mask = 0;
    for modifier in modifiers {
        mask |= match modifier.as_str() {
            "Alt" => 1,
            "Control" | "ControlOrMeta" => 2,
            "Meta" => 4,
            "Shift" => 8,
            other => {
                return Err(HostError::new(format!(
                    "Unsupported browser modifier: {other}"
                )))
            }
        };
    }
    Ok(mask)
}

struct KeyDescriptor {
    key: String,
    text: Option<String>,
    virtual_key_code: i64,
}

fn key_descriptor(key: &str) -> KeyDescriptor {
    match key {
        "Enter" => named_key(key, 13),
        "Tab" => named_key(key, 9),
        "Backspace" => named_key(key, 8),
        "Delete" => named_key(key, 46),
        "Escape" => named_key(key, 27),
        "ArrowLeft" => named_key(key, 37),
        "ArrowUp" => named_key(key, 38),
        "ArrowRight" => named_key(key, 39),
        "ArrowDown" => named_key(key, 40),
        "Home" => named_key(key, 36),
        "End" => named_key(key, 35),
        "PageUp" => named_key(key, 33),
        "PageDown" => named_key(key, 34),
        other => {
            let mut chars = other.chars();
            let first = chars.next();
            if let (Some(ch), None) = (first, chars.next()) {
                let code = if ch.is_ascii() {
                    ch.to_ascii_uppercase() as i64
                } else {
                    0
                };
                KeyDescriptor {
                    key: other.to_string(),
                    text: Some(other.to_string()),
                    virtual_key_code: code,
                }
            } else {
                KeyDescriptor {
                    key: other.to_string(),
                    text: None,
                    virtual_key_code: 0,
                }
            }
        }
    }
}

fn named_key(key: &str, virtual_key_code: i64) -> KeyDescriptor {
    KeyDescriptor {
        key: key.to_string(),
        text: None,
        virtual_key_code,
    }
}
