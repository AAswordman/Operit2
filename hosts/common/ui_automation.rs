//! Bounded subprocess transport for platform-owned accessibility providers.
use operit_host_api::UiAutomation::{UiAction, UiAutomationHost, UiPage, UiWindow};
use operit_host_api::{HostError, HostResult};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Runs one fixed provider script with structured input on stdin.
pub struct ScriptUiAutomationHost {
    program: &'static str,
    arguments: &'static [&'static str],
    operation: std::sync::Mutex<()>,
}

impl ScriptUiAutomationHost {
    /// Creates one serialized input channel for the platform's fixed provider.
    pub const fn new(program: &'static str, arguments: &'static [&'static str]) -> Self {
        Self {
            program,
            arguments,
            operation: std::sync::Mutex::new(()),
        }
    }

    /// Executes the provider with a deadline and drains both pipes concurrently.
    fn invoke(
        &self,
        operation: &str,
        parameters: &serde_json::Value,
    ) -> HostResult<serde_json::Value> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| HostError::new("UI automation operation lock is poisoned"))?;
        let mut command = Command::new(self.program);
        command
            .args(self.arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().map_err(|e| {
            HostError::new(format!("UI provider {} could not start: {e}", self.program))
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
        let input =
            serde_json::json!({ "operation": operation, "parameters": parameters }).to_string();
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
        serde_json::from_value(self.invoke("list_windows", &serde_json::json!({}))?)
            .map_err(|e| HostError::new(e.to_string()))
    }
    /// Decodes the canonical accessibility snapshot produced by this platform.
    fn get_page_info(&self, window_id: Option<&str>) -> HostResult<UiPage> {
        let parameters = match window_id {
            Some(id) => serde_json::json!({"window_id": id}),
            None => serde_json::json!({}),
        };
        serde_json::from_value(self.invoke("get_page_info", &parameters)?)
            .map_err(|e| HostError::new(e.to_string()))
    }
    /// Sends one input command to the same platform provider.
    fn perform_action(&self, action: &UiAction) -> HostResult<()> {
        let envelope =
            serde_json::to_value(action).map_err(|error| HostError::new(error.to_string()))?;
        let operation = envelope["operation"]
            .as_str()
            .expect("UiAction operation is a string");
        self.invoke(operation, &envelope["parameters"]).map(|_| ())
    }
}
