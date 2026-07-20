use std::ffi::{CStr, CString};
use std::thread;
use std::time::{Duration, Instant};

use operit_host_api::{
    HostError, HostResult, ManagedRuntimeHost, ManagedRuntimeProcess, ManagedRuntimeProgram,
    RuntimeCommandOutput, RuntimeProcessRequest,
};
use serde_json::{json, Value};

const RUNTIME_COMMAND_TIMEOUT: Duration = Duration::from_secs(180);

unsafe extern "C" {
    fn operit_ios_native_runtime_call(command: *const i8, request_json: *const i8) -> *mut i8;
    fn operit_ios_native_runtime_free(value: *mut i8);
}

#[derive(Clone, Default)]
pub struct IosManagedRuntimeHost;

impl IosManagedRuntimeHost {
    /// Creates the iOS-native managed runtime host.
    pub fn new() -> Self {
        Self
    }
}

struct IosManagedRuntimeProcess {
    id: String,
}

unsafe impl Send for IosManagedRuntimeProcess {}

impl ManagedRuntimeProcess for IosManagedRuntimeProcess {
    /// Writes one protocol line to the native iOS interpreter session.
    fn writeLine(&self, line: &str) -> HostResult<()> {
        callRuntime("writeLine", json!({"id": self.id, "line": line}))?;
        Ok(())
    }

    /// Writes several protocol lines to the native iOS interpreter session.
    fn writeLines(&self, lines: &[String]) -> HostResult<()> {
        callRuntime("writeLines", json!({"id": self.id, "lines": lines}))?;
        Ok(())
    }

    /// Reads one stdout protocol line from the native iOS interpreter session.
    fn readStdoutLine(&self, timeoutMs: u64) -> HostResult<Option<String>> {
        let response = callRuntime(
            "readStdoutLine",
            json!({"id": self.id, "timeoutMs": timeoutMs}),
        )?;
        optionalStringProperty(&response, "line")
    }

    /// Drains stderr captured by the native iOS interpreter session.
    fn drainStderr(&self) -> HostResult<String> {
        let response = callRuntime("drainStderr", json!({"id": self.id}))?;
        requiredStringProperty(&response, "stderr")
    }

    /// Reports whether the native iOS interpreter session remains active.
    fn isRunning(&self) -> HostResult<bool> {
        let response = callRuntime("isRunning", json!({"id": self.id}))?;
        response
            .get("running")
            .and_then(Value::as_bool)
            .ok_or_else(|| HostError::new("iOS native runtime running state is invalid"))
    }

    /// Closes the native iOS interpreter session input and output channels.
    fn kill(&self) -> HostResult<()> {
        callRuntime("close", json!({"id": self.id}))?;
        Ok(())
    }
}

impl ManagedRuntimeHost for IosManagedRuntimeHost {
    /// Returns the iOS App Support directory owned by the native interpreter bridge.
    fn runtimeWorkspaceDir(&self) -> HostResult<String> {
        let response = callRuntime("workspaceDir", Value::Null)?;
        requiredStringProperty(&response, "path")
    }

    /// Resolves one executable implemented by an embedded native iOS framework.
    fn resolveRuntimeExecutable(
        &self,
        program: ManagedRuntimeProgram,
        executablePath: Option<&str>,
    ) -> HostResult<String> {
        let response = callRuntime(
            "resolveExecutable",
            json!({
                "program": managedRuntimeProgramName(program),
                "executablePath": executablePath,
            }),
        )?;
        requiredStringProperty(&response, "executable")
    }

    /// Starts one persistent native iOS interpreter session with line-based stdio.
    fn startRuntimeProcess(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<Box<dyn ManagedRuntimeProcess>> {
        let response = callRuntime("start", runtimeRequestJson(&request))?;
        let id = requiredStringProperty(&response, "id")?;
        Ok(Box::new(IosManagedRuntimeProcess { id }))
    }

    /// Runs one finite native iOS interpreter command and captures all output.
    fn runRuntimeCommand(
        &self,
        request: RuntimeProcessRequest,
    ) -> HostResult<RuntimeCommandOutput> {
        let process = self.startRuntimeProcess(request)?;
        let deadline = Instant::now() + RUNTIME_COMMAND_TIMEOUT;
        let mut stdout = String::new();
        loop {
            if let Some(line) = process.readStdoutLine(100)? {
                stdout.push_str(&line);
                stdout.push('\n');
            }
            if !process.isRunning()? {
                return Ok(RuntimeCommandOutput {
                    exitCode: None,
                    stdout,
                    stderr: process.drainStderr()?,
                });
            }
            if Instant::now() >= deadline {
                process.kill()?;
                return Err(HostError::new("iOS native runtime command timed out"));
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}

/// Invokes the iOS Runner Objective-C++ native runtime bridge with one JSON request.
pub(crate) fn callRuntime(command: &str, request: Value) -> HostResult<Value> {
    let command = CString::new(command)
        .map_err(|_| HostError::new("iOS native runtime command contains a NUL byte"))?;
    let request_json = serde_json::to_string(&request).map_err(|error| {
        HostError::new(format!("iOS native runtime request encode failed: {error}"))
    })?;
    let request_json = CString::new(request_json)
        .map_err(|_| HostError::new("iOS native runtime request contains a NUL byte"))?;
    let response =
        unsafe { operit_ios_native_runtime_call(command.as_ptr(), request_json.as_ptr()) };
    if response.is_null() {
        return Err(HostError::new(
            "iOS native runtime bridge returned no response",
        ));
    }
    let response_json = unsafe {
        let value = CStr::from_ptr(response).to_string_lossy().into_owned();
        operit_ios_native_runtime_free(response);
        value
    };
    let response: Value = serde_json::from_str(&response_json).map_err(|error| {
        HostError::new(format!(
            "iOS native runtime response decode failed: {error}"
        ))
    })?;
    if let Some(error) = response.get("error").and_then(Value::as_str) {
        return Err(HostError::new(error));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| HostError::new("iOS native runtime bridge response has no result"))
}

/// Serializes one host runtime program enum into the native iOS bridge protocol name.
fn managedRuntimeProgramName(program: ManagedRuntimeProgram) -> &'static str {
    match program {
        ManagedRuntimeProgram::Node => "node",
        ManagedRuntimeProgram::Python => "python",
        ManagedRuntimeProgram::Uv => "uv",
        ManagedRuntimeProgram::Pnpm => "pnpm",
    }
}

/// Serializes one managed interpreter launch request for the native iOS bridge.
fn runtimeRequestJson(request: &RuntimeProcessRequest) -> Value {
    json!({
        "program": managedRuntimeProgramName(request.program.clone()),
        "executablePath": request.executablePath,
        "args": request.args,
        "cwd": request.cwd,
        "env": request.env,
    })
}

/// Reads a required string property from a native iOS runtime bridge result object.
fn requiredStringProperty(response: &Value, key: &str) -> HostResult<String> {
    response
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| HostError::new(format!("iOS native runtime response has invalid {key}")))
}

/// Reads an optional string property from a native iOS runtime bridge result object.
fn optionalStringProperty(response: &Value, key: &str) -> HostResult<Option<String>> {
    match response.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(HostError::new(format!(
            "iOS native runtime response has invalid {key}"
        ))),
    }
}
