use std::ffi::{c_void, CStr, CString};

use operit_host_api::{HostError, HostResult};
use serde_json::Value;

unsafe extern "C" {
    fn operit_ios_ish_terminal_call(command: *const i8, request_json: *const i8) -> *mut i8;
    fn operit_ios_ish_terminal_free(value: *mut i8);
    fn operit_ios_ish_terminal_call_stream(command: *const i8, request_json: *const i8,
        context: *mut c_void, on_output: extern "C" fn(*mut c_void, *const u8, usize) -> bool) -> *mut i8;
}

/// Invokes the iSH Objective-C++ terminal bridge with one JSON request.
pub(crate) fn callIshTerminal(command: &str, request: Value) -> HostResult<Value> {
    let command = CString::new(command)
        .map_err(|_| HostError::new("iSH terminal command contains a NUL byte"))?;
    let request_json = serde_json::to_string(&request)
        .map_err(|error| HostError::new(format!("iSH terminal request encode failed: {error}")))?;
    let request_json = CString::new(request_json)
        .map_err(|_| HostError::new("iSH terminal request contains a NUL byte"))?;
    let response = unsafe { operit_ios_ish_terminal_call(command.as_ptr(), request_json.as_ptr()) };
    decodeIshResponse(response)
}

/// Decodes and releases a response for both finite and streamed terminal calls.
fn decodeIshResponse(response: *mut i8) -> HostResult<Value> {
    if response.is_null() {
        return Err(HostError::new("iSH terminal bridge returned no response"));
    }
    let response_json = unsafe {
        let value = CStr::from_ptr(response).to_string_lossy().into_owned();
        operit_ios_ish_terminal_free(response);
        value
    };
    let response: Value = serde_json::from_str(&response_json)
        .map_err(|error| HostError::new(format!("iSH terminal response decode failed: {error}")))?;
    if let Some(error) = response.get("error").and_then(Value::as_str) {
        return Err(HostError::new(error));
    }
    response
        .get("result")
        .cloned()
        .ok_or_else(|| HostError::new("iSH terminal bridge response has no result"))
}

/// Keeps the Rust callback borrowed only for the synchronous native execution call.
pub(crate) fn callIshTerminalStreaming(request: Value, callback: &mut dyn FnMut(&[u8]) -> HostResult<()>) -> HostResult<Value> {
    struct OutputContext<'a> { callback: &'a mut dyn FnMut(&[u8]) -> HostResult<()>, error: Option<HostError> }
    extern "C" fn receive(context: *mut c_void, bytes: *const u8, length: usize) -> bool {
        let state = unsafe { &mut *(context as *mut OutputContext<'_>) };
        let frame = if length == 0 { &[] } else { unsafe { std::slice::from_raw_parts(bytes, length) } };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (state.callback)(frame))) {
            Ok(Ok(())) => true,
            Ok(Err(error)) => { state.error = Some(error); false }
            Err(_) => { state.error = Some(HostError::new("Terminal output callback panicked")); false }
        }
    }
    let request = CString::new(serde_json::to_string(&request).map_err(|error| HostError::new(error.to_string()))?)
        .map_err(|_| HostError::new("iSH terminal request contains a NUL byte"))?;
    let mut state = OutputContext { callback, error: None };
    let response = unsafe { operit_ios_ish_terminal_call_stream(c"terminalExecute".as_ptr(), request.as_ptr(), &mut state as *mut _ as *mut c_void, receive) };
    let result = decodeIshResponse(response);
    if let Some(error) = state.error { return Err(error); }
    result
}
