use std::ffi::{CStr, CString};

use operit_host_api::{HostError, HostResult};
use serde_json::Value;

unsafe extern "C" {
    fn operit_ios_ish_terminal_call(command: *const i8, request_json: *const i8) -> *mut i8;
    fn operit_ios_ish_terminal_free(value: *mut i8);
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
