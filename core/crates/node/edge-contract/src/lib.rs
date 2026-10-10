use serde::{Deserialize, Serialize};
use operit_link::CoreValue;

pub mod image_preview;
pub mod events;

/// Link target reserved for the Edge device I/O service namespace.
pub const EDGE_DEVICE_IO_OBJECT_ID: &str = "edge.deviceIo";

/// Link property name used by the digital-output state watch.
pub const EDGE_DEVICE_IO_STATE_PROPERTY: &str = "digitalOutputState";

/// Link target reserved for the Edge robot face service namespace.
pub const EDGE_ROBOT_FACE_OBJECT_ID: &str = "edge.robotFace";

/// Link property name used by the robot face expression watch.
pub const EDGE_ROBOT_FACE_STATE_PROPERTY: &str = "robotFaceState";

/// Link target reserved for the generic lightweight display service.
pub const EDGE_SCREEN_OBJECT_ID: &str = "edge.screen";

/// Link target reserved for device-owned native plugins.
pub const EDGE_PLUGIN_TARGET: &str = "edge.plugins";

/// Link property name reserved for future display state watches.
pub const EDGE_SCREEN_STATE_PROPERTY: &str = "screenState";

/// One complete display snapshot transported through the standard Link value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeScreenSnapshot {
    pub width: u16,
    pub height: u16,
    pub format: String,
    pub pixels: Vec<u8>,
}

/// A generic input event accepted by an Edge display service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeScreenInputRequest {
    pub action: String,
    pub x: u16,
    pub y: u16,
    pub endX: Option<u16>,
    pub endY: Option<u16>,
}

/// Reports whether a display input event was accepted by the Edge service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeScreenInputState {
    pub accepted: bool,
    pub action: String,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgePluginManifest {
    pub id: String,
    pub name: String,
    pub actions: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgePluginCall {
    pub pluginId: String,
    pub action: String,
    #[serde(default = "CoreValue::emptyMap")]
    pub args: CoreValue,
}

