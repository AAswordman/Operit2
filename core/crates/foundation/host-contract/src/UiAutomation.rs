//! Platform-neutral accessibility snapshots and input commands.
use crate::HostResult;
use serde::{Deserialize, Serialize};

/// Decodes complete native trees without imposing serde_json's generic 128-level limit.
pub fn decode_ui_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> HostResult<T> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    deserializer.disable_recursion_limit();
    let value = T::deserialize(&mut deserializer)
        .map_err(|error| crate::HostError::new(format!("Invalid UI response: {error}")))?;
    deserializer.end().map_err(|error| crate::HostError::new(error.to_string()))?;
    Ok(value)
}

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
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, serde_json::Value>,
    pub children: Vec<UiNode>,
}

/// Identifies one native window without depending on its title or list position.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiWindow {
    pub window_id: String,
    pub title: String,
    pub package_name: String,
    pub bounds: Option<String>,
    pub is_active: bool,
    pub is_focused: bool,
    pub is_minimized: bool,
}

/// Captures one bounded snapshot without embedding platform transport details.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiPage {
    pub window_id: String,
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
    FocusWindow {
        window_id: String,
    },
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
    /// Enumerates native windows, including windows outside Operit.
    fn list_windows(&self) -> HostResult<Vec<UiWindow>>;
    /// Reads the complete native subtree of the selected or active window.
    fn get_page_info(&self, window_id: Option<&str>) -> HostResult<UiPage>;
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
    /// Decodes the native windows exposed by the application owner.
    fn list_windows(&self) -> HostResult<Vec<UiWindow>> {
        let result = (self.invoke)("list_windows", serde_json::json!({}))?;
        serde_json::from_value(result["windows"].clone())
            .map_err(|error| crate::HostError::new(format!("Invalid UI window list: {error}")))
    }
    /// Decodes the canonical snapshot returned by the owner.
    fn get_page_info(&self, window_id: Option<&str>) -> HostResult<UiPage> {
        let parameters = match window_id {
            Some(id) => serde_json::json!({"window_id": id}),
            None => serde_json::json!({}),
        };
        serde_json::from_value((self.invoke)("get_page_info", parameters)?)
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

#[cfg(test)]
mod tests {
    /// Deep native trees must survive decoding instead of failing at the generic JSON limit.
    #[test]
    fn complete_ui_json_keeps_deep_descendants() {
        let json = format!("{}\"last-descendant\"{}", "{\"children\":[".repeat(80), "]}".repeat(80));
        let mut value: serde_json::Value = super::decode_ui_json(json.as_bytes()).unwrap();
        for _ in 0..80 { value = value["children"][0].take(); }
        assert_eq!(value, "last-descendant");
    }
}
