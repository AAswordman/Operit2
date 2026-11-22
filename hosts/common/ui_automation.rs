//! Bounded subprocess transport for platform-owned accessibility providers.
use operit_host_api::UiAutomation::{UiAction, UiAutomationHost, UiPage, UiWindow};
use operit_host_api::{HostError, HostResult};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Keeps the native lifetime pipe together with its exact automation source.
struct NativeWindowPreview {
    child: Child,
    window_id: String,
}

/// Runs one fixed provider script with structured input on stdin.
pub struct ScriptUiAutomationHost {
    provider_command: std::sync::Arc<dyn Fn() -> HostResult<Command> + Send + Sync>,
    selection: std::sync::Mutex<Option<String>>,
    preview_command: Option<std::sync::Arc<dyn Fn() -> HostResult<Command> + Send + Sync>>,
    preview: std::sync::Mutex<Option<NativeWindowPreview>>,
}

impl ScriptUiAutomationHost {
    /// Creates one serialized input channel for the platform's fixed provider.
    pub fn new(program: &'static str, arguments: &'static [&'static str]) -> Self {
        Self::from_command(std::sync::Arc::new(move || {
            let mut command = Command::new(program);
            command.args(arguments);
            Ok(command)
        }))
    }

    /// Creates a provider whose native assets are materialized by its owning host.
    pub fn from_command(command: std::sync::Arc<dyn Fn() -> HostResult<Command> + Send + Sync>) -> Self {
        Self {
            provider_command: command,
            selection: std::sync::Mutex::new(None),
            preview_command: None,
            preview: std::sync::Mutex::new(None),
        }
    }

    /// Enables a native preview process whose stdin lifetime owns its capture resources.
    pub fn with_window_previews(mut self, command: std::sync::Arc<dyn Fn() -> HostResult<Command> + Send + Sync>) -> Self {
        self.preview_command = Some(command);
        self
    }

    /// Closes the previous preview and releases its capture resources before switching.
    fn stop_preview(&self) -> HostResult<()> {
        let mut preview = self.preview.lock().map_err(|_| HostError::new("UI preview lock is poisoned"))?;
        let mut failed_exit = None;
        if let Some(state) = preview.as_mut() {
            let child = &mut state.child;
            drop(child.stdin.take());
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if let Some(status) = child.try_wait().map_err(|error| HostError::new(error.to_string()))? {
                    if !status.success() { failed_exit = Some(status); }
                    break;
                }
                if Instant::now() >= deadline { return Err(HostError::new("Native window preview did not release its resources before closing")); }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        *preview = None;
        match failed_exit {
            Some(status) => Err(HostError::new(format!("Native preview exited unsuccessfully: {status}"))),
            None => Ok(()),
        }
    }

    /// Starts the platform's live preview and waits for successful source capture before returning.
    fn start_preview(&self, window_id: &str) -> HostResult<()> {
        let Some(make_command) = self.preview_command.as_ref() else { return Ok(()); };
        self.stop_preview()?;
        let mut command = make_command()?;
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().map_err(|error| HostError::new(format!("Could not start native window preview: {error}")))?;
        let input = serde_json::json!({"operation": "preview_window", "parameters": {"window_id": window_id}});
        let written = writeln!(child.stdin.as_mut().expect("piped preview input"), "{input}");
        let stdout = child.stdout.take().expect("piped preview output");
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = send.send(result);
        });
        *self.preview.lock().map_err(|_| HostError::new("UI preview lock is poisoned"))? = Some(NativeWindowPreview { child, window_id: window_id.to_owned() });
        let startup = (|| {
            written.map_err(|error| HostError::new(error.to_string()))?;
            let ready = receive.recv_timeout(Duration::from_secs(12))
                .map_err(|error| HostError::new(format!("Native preview startup failed: {error}")))?
                .map_err(|error| HostError::new(error.to_string()))?;
            let result: serde_json::Value = serde_json::from_str(&ready)
                .map_err(|error| HostError::new(format!("Invalid native preview response: {error}")))?;
            if result["ready"] != true {
                return Err(HostError::new(format!("Native preview could not capture the selected window: {}", result["error"])));
            }
            Ok(())
        })();
        if startup.is_err() { self.stop_preview()?; }
        startup
    }

    /// Observes whether the explicitly selected native preview remains alive.
    fn preview_running(&self) -> HostResult<bool> {
        if self.preview_command.is_none() { return Ok(true); }
        let mut preview = self.preview.lock().map_err(|_| HostError::new("UI preview lock is poisoned"))?;
        match preview.as_mut() {
            Some(state) => Ok(state.child.try_wait().map_err(|error| HostError::new(error.to_string()))?.is_none()),
            None => Ok(false),
        }
    }

    /// Rejects input after the user closed its preview instead of sending it to another surface.
    fn check_preview(&self) -> HostResult<()> {
        if !self.preview_running()? {
            return Err(HostError::new("The window preview was closed; select a window again or clear selection"));
        }
        Ok(())
    }

    /// Executes the provider with a deadline and drains both pipes concurrently.
    fn invoke(
        &self,
        operation: &str,
        parameters: &serde_json::Value,
    ) -> HostResult<serde_json::Value> {
        let mut command = (self.provider_command)()?;
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().map_err(|e| {
            HostError::new(format!("UI provider could not start: {e}"))
        })?;
        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut stderr = child.stderr.take().expect("piped stderr");
        let output = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout
                .take(32 * 1024 * 1024)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        });
        let errors = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });
        let mut parameters = parameters.clone();
        if self.preview_command.is_some() && parameters.get("window_id").is_some() {
            parameters["hosted_window"] = true.into();
        }
        if let Some(state) = self.preview.lock().map_err(|_| HostError::new("UI preview lock is poisoned"))?.as_ref() {
            parameters["preview_process_id"] = state.child.id().into();
            parameters["preview_window_id"] = state.window_id.clone().into();
        }
        let input = serde_json::json!({ "operation": operation, "parameters": parameters }).to_string();
        if let Err(error) = child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(input.as_bytes())
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HostError::new(format!("UI provider input failed: {error}")));
        }
        let deadline = Instant::now() + Duration::from_secs(75);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(HostError::new(
                        "UI provider exceeded its deadline or could not be observed",
                    ));
                }
            }
        };
        let stdout = output
            .join()
            .map_err(|_| HostError::new("UI output reader panicked"))?
            .map_err(|e| HostError::new(e.to_string()))?;
        let stderr = errors
            .join()
            .map_err(|_| HostError::new("UI error reader panicked"))?
            .map_err(|e| HostError::new(e.to_string()))?;
        if !status.success() {
            return Err(HostError::new(format!(
                "UI provider failed: {}",
                String::from_utf8_lossy(&stderr)
            )));
        }
        operit_host_api::UiAutomation::decode_ui_json(&stdout)
    }
}

impl UiAutomationHost for ScriptUiAutomationHost {
    /// Lists native windows without changing foreground focus.
    fn list_windows(&self) -> HostResult<Vec<UiWindow>> {
        let _selection = self.selection.lock().map_err(|_| HostError::new("UI selection lock is poisoned"))?;
        serde_json::from_value(self.invoke("list_windows", &serde_json::json!({}))?)
            .map_err(|e| HostError::new(e.to_string()))
    }
    /// Decodes the canonical accessibility snapshot produced by this platform.
    fn get_page_info(&self, window_id: Option<&str>) -> HostResult<UiPage> {
        let mut selection = self.selection.lock().map_err(|_| HostError::new("UI selection lock is poisoned"))?;
        let parameters = match window_id.or(selection.as_deref()) {
            Some(id) => serde_json::json!({"window_id": id}),
            None => serde_json::json!({}),
        };
        let page = serde_json::from_value(self.invoke("get_page_info", &parameters)?)
            .map_err(|e| HostError::new(e.to_string()))?;
        if let Some(id) = window_id {
            let already_running = selection.as_deref() == Some(id) && self.preview_running()?;
            *selection = Some(id.to_owned());
            if !already_running { self.start_preview(id)?; }
        } else if selection.is_some() { self.check_preview()?; }
        Ok(page)
    }
    /// Captures through the provider while holding the same selection used by input.
    fn capture_screenshot(&self) -> HostResult<Vec<u8>> {
        let selection = self.selection.lock().map_err(|_| HostError::new("UI selection lock is poisoned"))?;
        if selection.is_some() { self.check_preview()?; }
        let parameters = match selection.as_deref() {
            Some(id) => serde_json::json!({"window_id": id}),
            None => serde_json::json!({}),
        };
        operit_host_api::UiAutomation::decode_ui_screenshot(&self.invoke("capture_screenshot", &parameters)?)
    }
    /// Sends one input command to the same platform provider.
    fn perform_action(&self, action: &UiAction) -> HostResult<()> {
        let mut selection = self.selection.lock().map_err(|_| HostError::new("UI selection lock is poisoned"))?;
        if matches!(action, UiAction::FocusWindow { window_id: None, .. }) {
            self.stop_preview()?;
            *selection = None;
            return Ok(());
        }
        let envelope =
            serde_json::to_value(action).map_err(|error| HostError::new(error.to_string()))?;
        let operation = envelope["operation"]
            .as_str()
            .expect("UiAction operation is a string");
        let mut parameters = envelope["parameters"].clone();
        if operation != "focus_window" && selection.is_some() { self.check_preview()?; }
        if let Some(id) = selection.as_deref() {
            if operation != "focus_window" { parameters["window_id"] = id.into(); }
        }
        self.invoke(operation, &parameters)?;
        if let UiAction::FocusWindow { window_id, .. } = action {
            *selection = window_id.clone();
            if let Some(id) = window_id { self.start_preview(id)?; }
        }
        Ok(())
    }
}

impl Drop for ScriptUiAutomationHost {
    /// Releases the lifetime pipe and waits for capture resource cleanup on owner destruction.
    fn drop(&mut self) {
        if let Err(error) = self.stop_preview() { eprintln!("Could not close native window preview: {error}"); }
    }
}
