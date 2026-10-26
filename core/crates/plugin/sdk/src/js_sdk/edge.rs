//! Device-selected hardware ports. Plugins execute on Core; hardware actions execute on Edge.
use super::results::EdgePortResultData;
use super::{JsAny, JsFuture};
use serde::{Deserialize, Serialize};

/// Identifies one native service endpoint on the selected Edge device.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeInterfaceInfo {
    /// Native device service target in the edge namespace.
    pub target: String,
    /// Native service method handled by the device application.
    pub method: String,
}

/// Identifies an I/O port family and operation. Unknown ports fail explicitly.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IoInterfaceInfo {
    /// Currently `gpio`; serial/UART ports are not implemented yet.
    pub port: String,
    /// Currently `read` or `write`; GPIO arguments carry `pin` and optionally `level`.
    pub operation: String,
}

/// Calls native device services on an explicit node.
pub trait DeviceEdge: Send + Sync {
    /// Executes a native service method through authenticated node routing without adding application-level retries.
    fn execute(
        &self,
        interfaceInfo: EdgeInterfaceInfo,
        args: Option<JsAny>,
    ) -> JsFuture<EdgePortResultData>;
}

/// Executes hardware I/O through authenticated node routing, not the host's local serial port.
pub trait DeviceIo: Send + Sync {
    /// Executes one supported port operation. Missing ports reject instead of pretending success.
    fn execute(
        &self,
        interfaceInfo: IoInterfaceInfo,
        args: Option<JsAny>,
    ) -> JsFuture<EdgePortResultData>;
}


/// Opens the two directions of one native protocol with the same session arguments.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceTransportEndpoint {
    pub target: String,
    /// ReverseStream method, carried by Push.
    pub method: String,
    /// Stream property, carried by Watch.
    pub property: String,
    pub args: Option<JsAny>,
}
/// Carries the original Watch event kind and its vendor-defined value.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceTransportEvent {
    pub kind: String,
    /// Structured values may contain Uint8Array byte payloads at any depth.
    pub value: JsAny,
}
/// Represents a duplex channel; its receive direction is also an async iterable.
pub trait DeviceTransportChannel: Send + Sync {
    fn send(&self, value: JsAny) -> JsFuture<()>;
    /// Completes ReverseStream input while keeping Stream output readable.
    fn finish(&self) -> JsFuture<()>;
    /// Closes both directions and wakes pending I/O.
    fn close(&self) -> JsFuture<()>;
}
/// Transports structured values and bytes without interpreting their business meaning.
pub trait DeviceTransport: Send + Sync {
    fn call(&self, target: String, method: String, args: Option<JsAny>) -> JsFuture<JsAny>;
    fn open(&self, endpoint: DeviceTransportEndpoint) -> JsFuture<std::sync::Arc<dyn DeviceTransportChannel>>;
}
