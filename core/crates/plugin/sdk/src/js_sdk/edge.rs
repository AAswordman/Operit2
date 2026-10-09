//! Explicit-node hardware ports. Plugins execute on Core; hardware actions execute on Edge.
use super::results::EdgePortResultData;
use super::{JsAny, JsFuture, JsObject};
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

/// One received device action. The action identifies business meaning, not a callable method.
pub struct EdgeActionEvent {
    /// Monotonic producer sequence, safe as a JavaScript integer.
    pub seq: f64,
    /// Examples: target.tap, gpio.changed, sensor.sample, serial.message.
    pub action: String,
    /// Producer-defined JSON object; validated by that action's handler.
    pub data: JsObject,
}
/// Generic bounded event batch shared by all device event producers.
pub struct EdgeEventBatch {
    /// Protocol version, currently 1.
    pub v: f64,
    /// Firmware service that produced this batch.
    pub source: String,
    /// Producer stream identity; renewed after reset/re-subscription.
    pub stream: String,
    /// Up to four ordered action events.
    pub events: Vec<EdgeActionEvent>,
    /// Last sequence in this batch; acknowledge only after persistence.
    pub next: f64,
    /// Last irrecoverably lost sequence; gaps are explicit.
    pub lostBefore: f64,
}
/// Core attaches routing context to a generic Edge event batch.
pub struct EdgeEventPayload {
    /// Existing authorized chat Binding, never implicitly created.
    pub chatId: String,
    /// Device identity authenticated against the route origin.
    pub nodeId: String,
    /// Device-independent action envelope.
    pub batch: EdgeEventBatch,
}
/// Argument of the fixed on_edge_event export in the existing ToolPkg main runtime.
pub struct EdgeEventHookEvent {
    /// Dedicated generic event discriminator: edge_event.
    pub event: String,
    /// Repeats the event discriminator.
    pub eventName: String,
    /// Trusted route context and bounded device events.
    pub eventPayload: EdgeEventPayload,
}
/// Return after applying/persisting this batch; never acknowledge merely on reception.
pub struct EdgeEventAck {
    /// False rejects this receiver/source/stream or unhandled action.
    pub accepted: bool,
    /// Required for acceptance; must equal the delivered batch next.
    pub next: Option<f64>,
}
