//! Explicit-node hardware ports. Plugins execute on Core; hardware actions execute on Edge.
use super::results::EdgePortResultData;
use super::{JsAny, JsFuture};
use serde::{Deserialize, Serialize};

/// Identifies one firmware-registered Edge plugin action, not a Core RPC target.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeInterfaceInfo {
    /// Stable Edge native plugin ID, as declared by the firmware.
    pub pluginId: String,
    /// Action explicitly declared by that native plugin.
    pub action: String,
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

/// Calls only firmware-declared native plugin actions on an explicit node.
pub trait EdgeHost: Send + Sync {
    /// Executes a declared action through authenticated node routing without adding application-level retries.
    fn execute(
        &self,
        nodeId: String,
        interfaceInfo: EdgeInterfaceInfo,
        args: Option<JsAny>,
    ) -> JsFuture<EdgePortResultData>;
}

/// Executes hardware I/O through authenticated node routing, not the host's local serial port.
pub trait IoHost: Send + Sync {
    /// Executes one supported port operation. Missing ports reject instead of pretending success.
    fn execute(
        &self,
        nodeId: String,
        interfaceInfo: IoInterfaceInfo,
        args: Option<JsAny>,
    ) -> JsFuture<EdgePortResultData>;
}
