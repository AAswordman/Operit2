//! Platform-neutral accessibility snapshots and input commands.
use crate::HostResult;
use serde::{Deserialize, Serialize};

/// Identifies a system accessibility snapshot of the active device surface.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiScope {
    Desktop,
}

/// Preserves the plugin node wire format across accessibility backends.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiNode {
    pub class_name: Option<String>,
    pub text: Option<String>,
    pub content_desc: Option<String>,
    pub resource_id: Option<String>,
    pub bounds: Option<String>,
    pub is_clickable: bool,
    pub children: Vec<UiNode>,
}

/// Captures one bounded snapshot without embedding platform transport details.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiPage {
    pub package_name: String,
    pub activity_name: String,
    pub ui_elements: UiNode,
    pub scope: UiScope,
    pub truncated: bool,
}

/// Describes one validated input action without string-based platform dispatch.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "operation",
    content = "parameters",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum UiAction {
    Tap {
        x: f64,
        y: f64,
    },
    LongPress {
        x: f64,
        y: f64,
    },
    Swipe {
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
        duration: u64,
    },
    SetInputText {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        resource_id: Option<String>,
    },
    PressKey {
        key_code: String,
    },
}

/// Owns accessibility collection and input within the platform boundary.
pub trait UiAutomationHost: Send + Sync {
    /// Reads the system's active surface, including applications outside Operit.
    fn get_page_info(&self) -> HostResult<UiPage>;
    /// Executes one validated input operation using the platform provider.
    fn perform_action(&self, action: &UiAction) -> HostResult<()>;
}

/// Adapts an application owner's UI transport without leaking it into runtime code.
pub struct OwnerUiAutomationHost {
    pub invoke: std::sync::Arc<
        dyn Fn(&str, serde_json::Value) -> HostResult<serde_json::Value> + Send + Sync,
    >,
}

impl UiAutomationHost for OwnerUiAutomationHost {
    /// Decodes the canonical snapshot returned by the owner.
    fn get_page_info(&self) -> HostResult<UiPage> {
        serde_json::from_value((self.invoke)("get_page_info", serde_json::json!({}))?)
            .map_err(|error| crate::HostError::new(format!("Invalid UI snapshot: {error}")))
    }

    /// Forwards input to the same owner that supplied the snapshot.
    fn perform_action(&self, action: &UiAction) -> HostResult<()> {
        let envelope = serde_json::to_value(action)
            .map_err(|error| crate::HostError::new(error.to_string()))?;
        let operation = envelope["operation"]
            .as_str()
            .expect("UiAction operation is a string");
        (self.invoke)(operation, envelope["parameters"].clone()).map(|_| ())
    }
}
