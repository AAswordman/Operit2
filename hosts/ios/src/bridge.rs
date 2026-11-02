use std::ffi::{c_char, c_void, CStr, CString};

use operit_host_api::{HostError, HostResult};
use serde_json::Value;

type OutputCallback = extern "C" fn(*mut c_void, *const u8, usize) -> bool;

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
        link_name = "operit_macos_arm64_terminal_execute_streaming"
    )]
    fn operit_ios_arm64_terminal_execute_streaming(
        command: *const c_char,
        request_json: *const c_char,
        context: *mut c_void,
        on_output: OutputCallback,
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

struct OutputContext<'a> {
    callback: &'a mut dyn FnMut(&[u8]) -> HostResult<()>,
    error: Option<HostError>,
}

extern "C" fn receive(context: *mut c_void, bytes: *const u8, length: usize) -> bool {
    if context.is_null() {
        return false;
    }
    let state = unsafe { &mut *(context as *mut OutputContext<'_>) };
    if state.error.is_some() {
        return false;
    }
    if (length != 0 && bytes.is_null()) || length > isize::MAX as usize {
        state.error = Some(HostError::new(
            "ARM64 terminal returned an invalid output frame",
        ));
        return false;
    }
    let frame = if length == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(bytes, length) }
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (state.callback)(frame))) {
        Ok(Ok(())) => true,
        Ok(Err(error)) => {
            state.error = Some(error);
            false
        }
        Err(_) => {
            state.error = Some(HostError::new("ARM64 terminal output callback panicked"));
            false
        }
    }
}

/// Native callbacks run synchronously on this caller's thread. The session's
/// queue owns only output buffers, never the borrowed Rust callback or context.
pub(crate) fn callArm64TerminalStreaming(
    request: Value,
    callback: &mut dyn FnMut(&[u8]) -> HostResult<()>,
) -> HostResult<Value> {
    let request = encodeRequest(request)?;
    let mut state = OutputContext {
        callback,
        error: None,
    };
    let response = unsafe {
        operit_ios_arm64_terminal_execute_streaming(
            c"terminalExecute".as_ptr(),
            request.as_ptr(),
            &mut state as *mut OutputContext<'_> as *mut c_void,
            receive,
        )
    };
    let result = decodeResponse(response);
    match state.error {
        Some(error) => Err(error),
        None => result,
    }
}
