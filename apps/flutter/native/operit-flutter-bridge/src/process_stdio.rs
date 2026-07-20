use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use super::*;

const OP_CREATE: u8 = 1;
const OP_CALL: u8 = 2;
const OP_PUSH_OPEN: u8 = 3;
const OP_PUSH_ITEM: u8 = 4;
const OP_PUSH_CLOSE: u8 = 5;
const OP_WATCH_SNAPSHOT: u8 = 6;
const OP_WATCH_STREAM: u8 = 7;
const OP_CLOSE_WATCH_STREAM: u8 = 8;
const OP_START_WEB_ACCESS_SERVER: u8 = 9;
const OP_DISCOVER_DEVICES: u8 = 10;
const OP_STOP_WEB_ACCESS_SERVER: u8 = 11;
const OP_REMOTE_PAIR_START: u8 = 12;
const OP_REMOTE_PAIR_FINISH: u8 = 13;
const OP_EMIT_RUNTIME_EVENT: u8 = 14;

const FRAME_RESPONSE: u8 = 101;
const FRAME_WATCH_EVENT: u8 = 102;

const PROCESS_STATUS_OK: u8 = 0;
const PROCESS_STATUS_ERROR: u8 = 1;

const PAYLOAD_BYTES: u8 = 1;
const PAYLOAD_STRING: u8 = 2;

/// Holds the bridge instance owned by one stdio process.
struct BridgeProcessState {
    bridge: Mutex<Option<Arc<OperitFlutterBridge>>>,
}

impl BridgeProcessState {
    /// Creates empty process state before the owner supplies storage roots.
    fn new() -> Self {
        Self {
            bridge: Mutex::new(None),
        }
    }

    /// Creates the runtime bridge and starts the watch event pump.
    fn create(
        &self,
        runtime_root: String,
        workspace_root: String,
        output: Arc<Mutex<io::Stdout>>,
    ) -> Result<(), String> {
        let mut bridge_slot = self
            .bridge
            .lock()
            .map_err(|error| format!("bridge state lock poisoned: {error}"))?;
        if bridge_slot.is_some() {
            return Err("runtime bridge is already initialized".to_string());
        }
        let bridge = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            OperitFlutterBridge::new_with_storage_roots(
                PathBuf::from(runtime_root),
                PathBuf::from(workspace_root),
            )
        }))
        .map_err(|payload| {
            format!(
                "FATAL_CORE_PANIC: {}",
                panic_payload_message(payload.as_ref())
            )
        })?
        .map_err(|error| error.to_string())?;
        let bridge = Arc::new(bridge);
        start_watch_pump(bridge.clone(), output);
        *bridge_slot = Some(bridge);
        Ok(())
    }

    /// Returns the initialized bridge for one request.
    fn bridge(&self) -> Result<Arc<OperitFlutterBridge>, String> {
        self.bridge
            .lock()
            .map_err(|error| format!("bridge state lock poisoned: {error}"))?
            .clone()
            .ok_or_else(|| "runtime bridge is not initialized".to_string())
    }
}

/// Reads typed values from one request payload.
struct FrameCursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> FrameCursor<'a> {
    /// Creates a cursor over one request frame payload.
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    /// Reads one byte from the frame payload.
    fn read_u8(&mut self) -> Result<u8, String> {
        let bytes = self.read_slice(1)?;
        Ok(bytes[0])
    }

    /// Reads one little-endian u64 from the frame payload.
    fn read_u64(&mut self) -> Result<u64, String> {
        let bytes = self.read_slice(8)?;
        let mut value = [0u8; 8];
        value.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(value))
    }

    /// Reads one length-prefixed byte vector from the frame payload.
    fn read_bytes(&mut self) -> Result<Vec<u8>, String> {
        let len = self.read_u32()? as usize;
        Ok(self.read_slice(len)?.to_vec())
    }

    /// Reads one length-prefixed UTF-8 string from the frame payload.
    fn read_string(&mut self) -> Result<String, String> {
        String::from_utf8(self.read_bytes()?)
            .map_err(|error| format!("frame string is not valid UTF-8: {error}"))
    }

    /// Reads one little-endian u32 from the frame payload.
    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_slice(4)?;
        let mut value = [0u8; 4];
        value.copy_from_slice(bytes);
        Ok(u32::from_le_bytes(value))
    }

    /// Reads a fixed-size slice from the frame payload.
    fn read_slice(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "frame offset overflowed".to_string())?;
        if end > self.data.len() {
            return Err("frame ended before all fields were read".to_string());
        }
        let bytes = &self.data[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}

/// Runs the blocking stdio server loop.
pub fn run_stdio_server() -> io::Result<()> {
    let state = Arc::new(BridgeProcessState::new());
    let output = Arc::new(Mutex::new(io::stdout()));
    let mut input = io::stdin();
    while let Some(frame) = read_frame(&mut input)? {
        let state = state.clone();
        let output = output.clone();
        thread::spawn(move || {
            let response = handle_request(state, output.clone(), &frame);
            write_response(output, response);
        });
    }
    Ok(())
}

/// Handles one request frame and returns one response frame.
fn handle_request(
    state: Arc<BridgeProcessState>,
    output: Arc<Mutex<io::Stdout>>,
    frame: &[u8],
) -> ProcessResponse {
    let mut cursor = FrameCursor::new(frame);
    let operation = match cursor.read_u8() {
        Ok(value) => value,
        Err(error) => return ProcessResponse::request_error(0, error),
    };
    let request_id = match cursor.read_u64() {
        Ok(value) => value,
        Err(error) => return ProcessResponse::request_error(0, error),
    };
    let result = dispatch_request(operation, state, output, &mut cursor);
    match result {
        Ok(payload) => ProcessResponse::ok(request_id, payload),
        Err(error) => ProcessResponse::request_error(request_id, error),
    }
}

/// Dispatches one decoded request by operation id.
fn dispatch_request(
    operation: u8,
    state: Arc<BridgeProcessState>,
    output: Arc<Mutex<io::Stdout>>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    match operation {
        OP_CREATE => handle_create(state, output, cursor),
        OP_CALL => handle_bytes_request(state, cursor, bridge_native_call),
        OP_PUSH_OPEN => handle_bytes_request(state, cursor, bridge_push_open),
        OP_PUSH_ITEM => handle_bytes_request(state, cursor, bridge_push_item),
        OP_PUSH_CLOSE => handle_push_close(state, cursor),
        OP_WATCH_SNAPSHOT => handle_bytes_request(state, cursor, bridge_watch_snapshot),
        OP_WATCH_STREAM => handle_bytes_request(state, cursor, bridge_watch_stream),
        OP_CLOSE_WATCH_STREAM => handle_close_watch_stream(state, cursor),
        OP_START_WEB_ACCESS_SERVER => handle_start_web_access_server(state, cursor),
        OP_DISCOVER_DEVICES => handle_discover_devices(state, cursor),
        OP_STOP_WEB_ACCESS_SERVER => handle_stop_web_access_server(state),
        OP_REMOTE_PAIR_START => handle_remote_pair_start(state, cursor),
        OP_REMOTE_PAIR_FINISH => handle_remote_pair_finish(state, cursor),
        OP_EMIT_RUNTIME_EVENT => handle_emit_runtime_event(state, cursor),
        _ => Err(format!("unknown bridge process operation: {operation}")),
    }
}

/// Handles one byte-in and byte-out bridge request.
fn handle_bytes_request(
    state: Arc<BridgeProcessState>,
    cursor: &mut FrameCursor<'_>,
    handler: fn(&OperitFlutterBridge, &[u8]) -> Vec<u8>,
) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    let bytes = cursor.read_bytes()?;
    Ok(ProcessPayload::bytes(handler(&bridge, &bytes)))
}

/// Handles one push close request.
fn handle_push_close(
    state: Arc<BridgeProcessState>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    let push_id = cursor.read_string()?;
    Ok(ProcessPayload::bytes(native_result_vec(
        bridge.pushClose(&push_id),
    )))
}

/// Handles one watch stream close request.
fn handle_close_watch_stream(
    state: Arc<BridgeProcessState>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    let subscription_id = cursor.read_string()?;
    bridge.closeWatchStream(&subscription_id);
    Ok(ProcessPayload::bytes(native_result_vec(Ok::<
        (),
        CoreLinkError,
    >(()))))
}

/// Handles one web access server stop request.
fn handle_stop_web_access_server(state: Arc<BridgeProcessState>) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    bridge.stopWebAccessServer();
    Ok(ProcessPayload::string("{\"ok\":true}".to_string()))
}

/// Handles one runtime event ingestion request.
fn handle_emit_runtime_event(
    state: Arc<BridgeProcessState>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    let event_json = cursor.read_string()?;
    Ok(ProcessPayload::string(bridge.emitRuntimeEvent(&event_json)))
}

/// Handles one bridge creation request.
fn handle_create(
    state: Arc<BridgeProcessState>,
    output: Arc<Mutex<io::Stdout>>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    let runtime_root = cursor.read_string()?;
    let workspace_root = cursor.read_string()?;
    state.create(runtime_root, workspace_root, output)?;
    Ok(ProcessPayload::string("{\"ok\":true}".to_string()))
}

/// Handles one web access server start request.
fn handle_start_web_access_server(
    state: Arc<BridgeProcessState>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    let bind_address = cursor.read_string()?;
    let token = cursor.read_string()?;
    let shutdown_token = cursor.read_string()?;
    let web_root = cursor.read_string()?;
    let device_info_json = cursor.read_string()?;
    let enable_web_access = cursor.read_string()?;
    let enable_discovery = cursor.read_string()?;
    let device_info =
        serde_json::from_str::<RemoteDeviceInfo>(&device_info_json).map_err(|error| {
            core_error_string(CoreLinkError::new(
                "INVALID_ARGS",
                format!("device info is invalid: {error}"),
            ))
        })?;
    match bridge.startWebAccessServer(
        bind_address,
        token,
        shutdown_token,
        PathBuf::from(web_root),
        device_info,
        enable_web_access == "true",
        enable_discovery == "true",
    ) {
        Ok(device_id) => Ok(ProcessPayload::string(
            serde_json::json!({"ok": true, "deviceId": device_id}).to_string(),
        )),
        Err(error) => Ok(ProcessPayload::string(core_error_string(
            CoreLinkError::internal(error),
        ))),
    }
}

/// Handles one device discovery request.
fn handle_discover_devices(
    state: Arc<BridgeProcessState>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    let timeout_ms = cursor.read_string()?.parse::<u64>().map_err(|error| {
        core_error_string(CoreLinkError::new(
            "INVALID_ARGS",
            format!("timeout_ms is not a valid number: {error}"),
        ))
    })?;
    match bridge.discoverDevices(timeout_ms) {
        Ok(json) => Ok(ProcessPayload::string(json)),
        Err(error) => Ok(ProcessPayload::string(core_error_string(
            CoreLinkError::internal(error),
        ))),
    }
}

/// Handles one remote pairing start request.
fn handle_remote_pair_start(
    state: Arc<BridgeProcessState>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    let base_url = cursor.read_string()?;
    let token_hash = cursor.read_string()?;
    let device_info_json = cursor.read_string()?;
    let device_info =
        serde_json::from_str::<RemoteDeviceInfo>(&device_info_json).map_err(|error| {
            core_error_string(CoreLinkError::new(
                "INVALID_ARGS",
                format!("clientDeviceInfo is invalid: {error}"),
            ))
        })?;
    match bridge.remotePairStart(base_url, token_hash, device_info) {
        Ok(value) => Ok(ProcessPayload::string(value)),
        Err(error) => Ok(ProcessPayload::string(core_error_string(
            CoreLinkError::internal(error),
        ))),
    }
}

/// Handles one remote pairing finish request.
fn handle_remote_pair_finish(
    state: Arc<BridgeProcessState>,
    cursor: &mut FrameCursor<'_>,
) -> Result<ProcessPayload, String> {
    let bridge = state.bridge()?;
    let pairing_id = cursor.read_string()?;
    let pairing_code = cursor.read_string()?;
    match bridge.remotePairFinish(pairing_id, pairing_code) {
        Ok(value) => Ok(ProcessPayload::string(value)),
        Err(error) => Ok(ProcessPayload::string(core_error_string(
            CoreLinkError::internal(error),
        ))),
    }
}

/// Starts the thread that forwards watch events to stdout.
fn start_watch_pump(bridge: Arc<OperitFlutterBridge>, output: Arc<Mutex<io::Stdout>>) {
    thread::spawn(move || {
        while let Ok(event) = bridge.nextWatchChannelEvent() {
            let frame = build_watch_event_frame(event);
            if write_frame(output.clone(), &frame).is_err() {
                break;
            }
        }
    });
}

/// Represents one process response payload.
struct ProcessPayload {
    kind: u8,
    bytes: Vec<u8>,
}

impl ProcessPayload {
    /// Creates a byte response payload.
    fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            kind: PAYLOAD_BYTES,
            bytes,
        }
    }

    /// Creates a string response payload.
    fn string(value: String) -> Self {
        Self {
            kind: PAYLOAD_STRING,
            bytes: value.into_bytes(),
        }
    }
}

/// Represents one process response frame.
struct ProcessResponse {
    request_id: u64,
    status: u8,
    payload: ProcessPayload,
}

impl ProcessResponse {
    /// Creates a successful response frame.
    fn ok(request_id: u64, payload: ProcessPayload) -> Self {
        Self {
            request_id,
            status: PROCESS_STATUS_OK,
            payload,
        }
    }

    /// Creates a process-level error response frame.
    fn request_error(request_id: u64, error: String) -> Self {
        Self {
            request_id,
            status: PROCESS_STATUS_ERROR,
            payload: ProcessPayload::string(error),
        }
    }
}

/// Writes one response frame to stdout.
fn write_response(output: Arc<Mutex<io::Stdout>>, response: ProcessResponse) {
    let mut frame = Vec::new();
    frame.push(FRAME_RESPONSE);
    append_u64(&mut frame, response.request_id);
    frame.push(response.status);
    frame.push(response.payload.kind);
    append_bytes(&mut frame, &response.payload.bytes);
    let _ = write_frame(output, &frame);
}

/// Builds one watch event frame.
fn build_watch_event_frame(event: Vec<u8>) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.push(FRAME_WATCH_EVENT);
    append_bytes(&mut frame, &event);
    frame
}

/// Writes one length-prefixed frame to stdout.
fn write_frame(output: Arc<Mutex<io::Stdout>>, frame: &[u8]) -> io::Result<()> {
    let mut output = output
        .lock()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "stdout lock poisoned"))?;
    output.write_all(&(frame.len() as u32).to_le_bytes())?;
    output.write_all(frame)?;
    output.flush()
}

/// Reads one length-prefixed frame from stdin.
fn read_frame(input: &mut io::Stdin) -> io::Result<Option<Vec<u8>>> {
    let mut len_bytes = [0u8; 4];
    match input.read_exact(&mut len_bytes) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let len = u32::from_le_bytes(len_bytes) as usize;
    let mut frame = vec![0u8; len];
    input.read_exact(&mut frame)?;
    Ok(Some(frame))
}

/// Appends one little-endian u64 to a frame.
fn append_u64(frame: &mut Vec<u8>, value: u64) {
    frame.extend_from_slice(&value.to_le_bytes());
}

/// Appends one length-prefixed byte slice to a frame.
fn append_bytes(frame: &mut Vec<u8>, value: &[u8]) {
    frame.extend_from_slice(&(value.len() as u32).to_le_bytes());
    frame.extend_from_slice(value);
}

/// Serializes one Link error into the native string response shape.
fn core_error_string(error: CoreLinkError) -> String {
    serde_json::to_string(&error).expect("CoreLinkError must serialize")
}
