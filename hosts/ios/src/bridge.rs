use std::ffi::{c_char, c_void, CStr, CString};

use operit_host_api::TerminalCommand::TerminalCommandBuffer;
use operit_host_api::{HostError, HostResult};
use serde_json::Value;
use std::sync::{Arc, Mutex};

type OutputCallback = extern "C" fn(*mut c_void, *const u8, usize);
type CompletionCallback = extern "C" fn(*mut c_void, *mut c_char);

unsafe extern "C" {
    #[cfg_attr(target_os = "macos", link_name = "operit_macos_arm64_terminal_call")]
    fn operit_ios_arm64_terminal_call(
        command: *const c_char,
        request_json: *const c_char,
    ) -> *mut c_char;
    #[cfg_attr(target_os = "macos", link_name = "operit_macos_arm64_terminal_free")]
    fn operit_ios_arm64_terminal_free(value: *mut c_char);
    #[cfg_attr(
        target_os = "macos",
        link_name = "operit_macos_arm64_terminal_execute_async"
    )]
    fn operit_ios_arm64_terminal_execute_async(
        command: *const c_char,
        request_json: *const c_char,
        context: *mut c_void,
        on_output: OutputCallback,
        on_complete: CompletionCallback,
    ) -> *mut c_char;
}

fn encodeRequest(request: Value) -> HostResult<CString> {
    let json = serde_json::to_string(&request).map_err(|error| {
        HostError::new(format!("ARM64 terminal request encode failed: {error}"))
    })?;
    CString::new(json).map_err(|_| HostError::new("ARM64 terminal request contains a NUL byte"))
}

/// Calls the ARM64 terminal owner and releases its malloc-owned JSON response.
pub(crate) fn callArm64Terminal(command: &str, request: Value) -> HostResult<Value> {
    let command = CString::new(command)
        .map_err(|_| HostError::new("ARM64 terminal command contains a NUL byte"))?;
    let request = encodeRequest(request)?;
    let response = unsafe { operit_ios_arm64_terminal_call(command.as_ptr(), request.as_ptr()) };
    decodeResponse(response)
}

/// Reads only the native configuration generation, without creating bookmarks
/// or reopening security-scoped resources in the terminal polling loop.
pub(crate) fn arm64RuntimeConfigurationId() -> HostResult<Option<String>> {
    let state = callArm64Terminal("arm64RuntimeState", serde_json::json!({}))?;
    if state.get("configured").and_then(Value::as_bool) == Some(false) {
        return Ok(None);
    }
    state
        .get("configurationId")
        .and_then(Value::as_str)
        .filter(|identifier| !identifier.is_empty())
        .map(|identifier| Some(identifier.to_string()))
        .ok_or_else(|| HostError::new("ARM64 runtime state has no configuration identifier"))
}

fn decodeResponse(response: *mut c_char) -> HostResult<Value> {
    if response.is_null() {
        return Err(HostError::new("ARM64 terminal bridge returned no response"));
    }
    // Copy before releasing; decoding errors must not leak the native allocation.
    let bytes = unsafe {
        let bytes = CStr::from_ptr(response).to_bytes().to_vec();
        operit_ios_arm64_terminal_free(response);
        bytes
    };
    let response: Value = serde_json::from_slice(&bytes).map_err(|error| {
        HostError::new(format!("ARM64 terminal response decode failed: {error}"))
    })?;
    if let Some(error) = response.get("error").and_then(Value::as_str) {
        return Err(HostError::new(error));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| HostError::new("ARM64 terminal bridge response has no result"))
}

/// Retains native callback ownership independently of the foreground Rust observer.
pub(crate) struct Arm64CommandEvents {
    pub(crate) output: TerminalCommandBuffer,
    result: Mutex<Option<HostResult<Value>>>,
}

impl Arm64CommandEvents {
    /// Reads a durable completion after draining the final output notification.
    pub(crate) fn completion(&self) -> HostResult<Option<HostResult<Value>>> {
        let state = self
            .result
            .lock()
            .map_err(|_| HostError::new("ARM64 command completion state poisoned"))?;
        Ok(state.as_ref().map(|result| match result {
            Ok(value) => Ok(value.clone()),
            Err(error) => Err(HostError::new(error.message.clone())),
        }))
    }
}

/// Publishes bytes from the session queue without borrowing the foreground output callback.
extern "C" fn receive(context: *mut c_void, bytes: *const u8, length: usize) {
    let state = unsafe { &*(context as *const Arm64CommandEvents) };
    if (length != 0 && bytes.is_null()) || length > isize::MAX as usize {
        state.output.fail(HostError::new(
            "ARM64 terminal returned an invalid output frame",
        ));
        return;
    }
    if length != 0 {
        state
            .output
            .append(unsafe { std::slice::from_raw_parts(bytes, length) });
    }
}

/// Releases the native-owned context exactly once after the final frame has been published.
extern "C" fn completed(context: *mut c_void, response: *mut c_char) {
    let state = unsafe { Arc::from_raw(context as *const Arm64CommandEvents) };
    let result = decodeResponse(response);
    *state
        .result
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(result);
    state.output.append(&[]);
}

/// Transfers one retained context to the native event producer, which completes it on close as well.
pub(crate) fn beginArm64TerminalCommand(
    sessionId: &str,
    command: &str,
    timeoutMs: u64,
) -> HostResult<Arc<Arm64CommandEvents>> {
    let request = encodeRequest(
        serde_json::json!({"sessionId": sessionId, "command": command, "timeoutMs": timeoutMs}),
    )?;
    let state = Arc::new(Arm64CommandEvents {
        output: TerminalCommandBuffer::new(if timeoutMs == 0 {
            1024 * 1024
        } else {
            16 * 1024 * 1024
        }),
        result: Mutex::new(None),
    });
    let context = Arc::into_raw(state.clone()) as *mut c_void;
    let response = unsafe {
        operit_ios_arm64_terminal_execute_async(
            c"terminalExecuteAsync".as_ptr(),
            request.as_ptr(),
            context,
            receive,
            completed,
        )
    };
    if let Err(error) = decodeResponse(response) {
        // A rejected submission never installs callbacks or takes ownership of this reference.
        unsafe {
            drop(Arc::from_raw(context as *const Arm64CommandEvents));
        }
        return Err(error);
    }
    Ok(state)
}

/// Receives one binary PTY event into an independently owned asynchronous read.
extern "C" fn receivePty(context: *mut c_void, bytes: *const u8, length: usize) {
    let output = unsafe { &*(context as *const operit_host_api::TerminalPtyOutput::TerminalPtyOutputBuffer) };
    if (length != 0 && bytes.is_null()) || length > isize::MAX as usize {
        output.fail(HostError::new("ARM64 terminal returned an invalid output frame"));
    } else if length != 0 {
        output.append(unsafe { std::slice::from_raw_parts(bytes, length) });
    }
}

/// Completes and releases a one-shot native output waiter on data, EOF, or error.
extern "C" fn completedPty(context: *mut c_void, response: *mut c_char) {
    let output = unsafe { Arc::from_raw(context as *const operit_host_api::TerminalPtyOutput::TerminalPtyOutputBuffer) };
    match decodeResponse(response) {
        Ok(_) => output.close(),
        Err(error) => output.fail(error),
    }
}

/// Waits for native output without serializing bytes into the JSON control response.
pub(crate) async fn readArm64TerminalOutput(sessionId: &str) -> HostResult<Option<Vec<u8>>> {
    let request = encodeRequest(serde_json::json!({"sessionId": sessionId}))?;
    let output = Arc::new(operit_host_api::TerminalPtyOutput::TerminalPtyOutputBuffer::new(16 * 1024 * 1024));
    let context = Arc::into_raw(output.clone()) as *mut c_void;
    let response = unsafe {
        operit_ios_arm64_terminal_execute_async(c"terminalReadAsync".as_ptr(), request.as_ptr(), context, receivePty, completedPty)
    };
    if let Err(error) = decodeResponse(response) {
        // Rejected reads never transfer callback ownership to the native session.
        unsafe { drop(Arc::from_raw(context as *const operit_host_api::TerminalPtyOutput::TerminalPtyOutputBuffer)); }
        return Err(error);
    }
    output.next().await
}
