//! Accessibility tools shared by every host, without platform dispatch.
use operit_host_api::UiAutomation::{UiAction, UiAutomationHost, UiNode, UiScope};
use operit_tools::tools::ToolResultDataClasses::{
    stringResultData, JsonResultData, SimplifiedUINode, ToolResultData, UIActionResultData, UIPageResultData,
};
use operit_tools::ConversationMarkupManager::ToolResult;
use operit_tools::ToolExecutionManager::{
    AITool, ToolAccessSpec, ToolBoundary, ToolEffect, ToolExecutor, ToolValidationResult,
};
use serde_json::{Map, Value};
use std::sync::Arc;

/// Executes all UI operations against the owning host's accessibility provider.
pub struct UiToolExecutor {
    pub host: Option<Arc<dyn UiAutomationHost>>,
}

impl UiToolExecutor {
    /// Validates arguments and dispatches a snapshot or input command.
    fn execute(&self, tool: &AITool) -> Result<ToolResultData, String> {
        let host = self
            .host
            .as_ref()
            .ok_or("This host does not support whole-device UI automation")?;
        if tool
            .parameters
            .iter()
            .any(|parameter| parameter.name == "display")
        {
            return Err("This UI interface addresses the active surface; explicit display selection is not supported".into());
        }
        if tool.name == "list_windows" {
            if !tool.parameters.is_empty() {
                return Err("list_windows accepts no parameters".into());
            }
            let windows = host.list_windows().map_err(|error| error.message)?;
            return Ok(ToolResultData::JsonResultData(JsonResultData {
                value: serde_json::json!({"windows": windows}),
            }));
        }
        if tool.name == "get_page_info" {
            if tool.parameters.len() > 1 || tool.parameters.iter().any(|p| p.name != "window_id" || p.value.trim().is_empty()) {
                return Err("get_page_info accepts only an optional nonempty window_id".into());
            }
            let window_id = tool.parameters.first().map(|p| p.value.as_str());
            let page = host.get_page_info(window_id).map_err(|error| error.message)?;
            let ui_elements = plugin_node(page.ui_elements);
            let summary = ui_elements.toTreeString(None);
            let data = UIPageResultData {
                window_id: page.window_id,
                summary,
                package_name: page.package_name,
                activity_name: page.activity_name,
                ui_elements,
                truncated: page.truncated,
                scope: match page.scope {
                    UiScope::Desktop => "desktop",
                }
                .into(),
            };
            return Ok(ToolResultData::UIPageResultData(data));
        }
        let mut parameters = Map::new();
        for parameter in &tool.parameters {
            parameters.insert(
                parameter.name.clone(),
                Value::String(parameter.value.clone()),
            );
        }
        let mut operation = tool.name.as_str();
        let allowed = match operation {
            "focus_window" => &["window_id"][..],
            "tap" | "long_press" => &["x", "y"][..],
            "swipe" => &["start_x", "start_y", "end_x", "end_y", "duration"][..],
            "set_input_text" => &["text", "resource_id"][..],
            "press_key" => &["key_code"][..],
            "click_element" => &[],
            _ => return Err(format!("Unknown UI operation: {operation}")),
        };
        if operation != "click_element" {
            for key in parameters.keys() {
                if !allowed.contains(&key.as_str()) {
                    return Err(format!("Unexpected {operation} parameter: {key}"));
                }
            }
        }
        if operation == "click_element" {
            let criteria = click_criteria(&parameters)?;
            let page = host.get_page_info(None).map_err(|error| error.message)?;
            let mut matches = Vec::new();
            collect_matches(&page.ui_elements, &criteria, &mut matches)?;
            let index = criteria
                .get("index")
                .map(parse_index)
                .transpose()?
                .unwrap_or(0);
            let node = matches
                .get(index)
                .ok_or("No accessible element matches the requested selector and index")?;
            let (x, y) = bounds_center(
                node.bounds
                    .as_deref()
                    .ok_or("Selected element has no bounds")?,
            )?;
            parameters =
                Map::from_iter([("x".into(), Value::from(x)), ("y".into(), Value::from(y))]);
            operation = "tap";
        }
        for key in match operation {
            "tap" | "long_press" => &["x", "y"][..],
            "swipe" => &["start_x", "start_y", "end_x", "end_y"][..],
            _ => &[],
        } {
            let number = text(
                parameters
                    .get(*key)
                    .ok_or_else(|| format!("{key} is required"))?,
            )
            .parse::<f64>()
            .map_err(|_| format!("Invalid coordinate: {key}"))?;
            if !number.is_finite() || number < i32::MIN as f64 || number > i32::MAX as f64 {
                return Err(format!("Coordinate out of range: {key}"));
            }
            parameters.insert((*key).into(), Value::from(number));
        }
        if operation == "swipe" {
            let duration = parameters
                .get("duration")
                .map(|v| text(v).parse::<u64>())
                .transpose()
                .map_err(|_| "duration must be an integer in milliseconds")?
                .unwrap_or(300);
            if !(1..=60_000).contains(&duration) {
                return Err("duration must be between 1 and 60000 ms".into());
            }
            parameters.insert("duration".into(), Value::from(duration));
        }
        if operation == "set_input_text" && !parameters.contains_key("text") {
            return Err("text is required".into());
        }
        if operation == "press_key" && !parameters.contains_key("key_code") {
            return Err("keyCode is required".into());
        }
        if operation == "focus_window" && !parameters.get("window_id").is_some_and(|value| !text(value).trim().is_empty()) {
            return Err("window_id is required and must not be empty".into());
        }
        let coordinates = parameters
            .get("x")
            .and_then(Value::as_f64)
            .zip(parameters.get("y").and_then(Value::as_f64));
        let action: UiAction = serde_json::from_value(serde_json::json!({
            "operation": operation, "parameters": parameters,
        }))
        .map_err(|error| format!("Invalid UI action: {error}"))?;
        host.perform_action(&action)
            .map_err(|error| error.message)?;
        Ok(ToolResultData::UIActionResultData(UIActionResultData {
            action_type: tool.name.clone(),
            action_description: format!("{} completed", tool.name),
            coordinates,
            element_id: None,
        }))
    }
}

/// Moves a host node into its public plugin representation without a JSON round trip.
fn plugin_node(node: UiNode) -> SimplifiedUINode {
    SimplifiedUINode {
        class_name: node.class_name,
        text: node.text,
        content_desc: node.content_desc,
        resource_id: node.resource_id,
        bounds: node.bounds,
        is_clickable: node.is_clickable,
        properties: node.properties,
        children: node.children.into_iter().map(plugin_node).collect(),
    }
}

impl ToolExecutor for UiToolExecutor {
    /// Performs semantic validation before any input is sent to the host.
    fn validateParameters(&self, _tool: &AITool) -> ToolValidationResult {
        ToolValidationResult {
            valid: true,
            errorMessage: String::new(),
        }
    }
    /// Marks inspection as read access and every input action as write access.
    fn accessSpec(&self, tool: &AITool) -> Result<ToolAccessSpec, String> {
        Ok(ToolAccessSpec {
            effect: if matches!(tool.name.as_str(), "get_page_info" | "list_windows") {
                ToolEffect::READ
            } else {
                ToolEffect::WRITE
            },
            boundary: ToolBoundary::None,
        })
    }
    /// Returns a typed result or an explicit provider error through the ordinary pipeline.
    fn invokeAndStream(&self, tool: &AITool) -> Vec<ToolResult> {
        vec![match self.execute(tool) {
            Ok(result) => ToolResult {
                toolName: tool.name.clone(),
                success: true,
                result,
                error: None,
            },
            Err(error) => ToolResult {
                toolName: tool.name.clone(),
                success: false,
                result: stringResultData(""),
                error: Some(error),
            },
        }]
    }
}

/// Reads a scalar without guessing whether a string contains serialized data.
fn text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

/// Parses a nonnegative match index without truncating fractional input.
fn parse_index(value: &Value) -> Result<usize, String> {
    text(value)
        .parse()
        .map_err(|_| "index must be a nonnegative integer".into())
}

/// Normalizes the documented click overloads into one exact selector object.
fn click_criteria(parameters: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    let mut criteria = parameters.clone();
    if let Some(param1) = criteria.remove("param1") {
        let first = text(&param1);
        let second = criteria.remove("param2");
        let third = criteria.remove("param3");
        match first.as_str() {
            "resourceId" | "className" | "bounds" => {
                criteria.insert(first, second.ok_or("Typed selector requires a value")?);
                if let Some(index) = third {
                    criteria.insert("index".into(), index);
                }
            }
            _ => {
                let key = if first.starts_with('[') {
                    bounds_center(&first)?;
                    "bounds"
                } else {
                    "resourceId"
                };
                criteria.insert(key.into(), Value::String(first));
                if let Some(index) = second {
                    criteria.insert("index".into(), index);
                }
            }
        }
    }
    if ![
        "resourceId",
        "className",
        "text",
        "contentDesc",
        "bounds",
        "isClickable",
    ]
    .iter()
    .any(|key| criteria.contains_key(*key))
    {
        return Err("An element selector is required".into());
    }
    for key in criteria.keys() {
        if ![
            "resourceId",
            "className",
            "text",
            "contentDesc",
            "bounds",
            "isClickable",
            "partialMatch",
            "index",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("Unknown element selector: {key}"));
        }
    }
    Ok(criteria)
}

/// Collects matches in stable tree order with explicit exact or partial string matching.
fn collect_matches<'a>(
    node: &'a UiNode,
    criteria: &Map<String, Value>,
    output: &mut Vec<&'a UiNode>,
) -> Result<(), String> {
    let partial = criteria
        .get("partialMatch")
        .map(|v| text(v).parse::<bool>())
        .transpose()
        .map_err(|_| "partialMatch must be boolean")?
        .unwrap_or(false);
    let mut matched = true;
    for (key, expected) in criteria {
        let actual = match key.as_str() {
            "className" => node.class_name.as_deref(),
            "text" => node.text.as_deref(),
            "contentDesc" => node.content_desc.as_deref(),
            "resourceId" => node.resource_id.as_deref(),
            "bounds" => node.bounds.as_deref(),
            "isClickable" => {
                matched &= node.is_clickable
                    == text(expected)
                        .parse::<bool>()
                        .map_err(|_| "isClickable must be boolean")?;
                continue;
            }
            "index" | "partialMatch" => continue,
            _ => return Err(format!("Unknown selector: {key}")),
        };
        let expected = text(expected);
        matched &= actual.is_some_and(|actual| {
            if partial && key != "bounds" {
                actual.contains(&expected)
            } else {
                actual == expected
            }
        });
    }
    if matched {
        output.push(node);
    }
    for child in &node.children {
        collect_matches(child, criteria, output)?;
    }
    Ok(())
}

/// Parses the canonical bounds grammar and rejects inverted or empty rectangles.
fn bounds_center(bounds: &str) -> Result<(f64, f64), String> {
    let pattern = regex::Regex::new(r"^\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]$")
        .expect("static bounds regex");
    let captures = pattern
        .captures(bounds)
        .ok_or("Invalid bounds; expected [x1,y1][x2,y2]")?;
    let values = (1..=4)
        .map(|index| {
            captures[index]
                .parse::<i32>()
                .map_err(|_| "Bounds coordinate out of range".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values[2] <= values[0] || values[3] <= values[1] {
        return Err("Element has empty or inverted bounds".into());
    }
    Ok((
        (values[0] as f64 + values[2] as f64) / 2.0,
        (values[1] as f64 + values[3] as f64) / 2.0,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{
        HostResult,
        UiAutomation::{UiPage, UiScope},
    };
    use operit_tools::ToolExecutionManager::ToolParameter;
    use std::sync::Mutex;

    struct FakeHost {
        actions: Mutex<Vec<(String, Value)>>,
    }
    impl UiAutomationHost for FakeHost {
        /// Supplies a stable native window identity for discovery tests.
        fn list_windows(&self) -> HostResult<Vec<operit_host_api::UiAutomation::UiWindow>> {
            Ok(vec![operit_host_api::UiAutomation::UiWindow {
                window_id: "editor:1".into(), title: "editor".into(), package_name: "test".into(),
                bounds: Some("[0,0][400,400]".into()), is_active: true, is_focused: true, is_minimized: false,
            }])
        }
        /// Supplies a real nested canonical snapshot to the common executor.
        fn get_page_info(&self, window_id: Option<&str>) -> HostResult<UiPage> {
            if window_id.is_some_and(|id| id != "editor:1") {
                return Err(operit_host_api::HostError::new("Window no longer exists"));
            }
            let node = |text: &str, bounds: &str| UiNode {
                class_name: Some("Button".into()),
                text: Some(text.into()),
                content_desc: None,
                resource_id: Some("save".into()),
                bounds: Some(bounds.into()),
                is_clickable: true,
                properties: std::collections::BTreeMap::from([("role".into(), Value::String("button".into()))]),
                children: vec![],
            };
            let mut root = node("Window", "[0,0][400,400]");
            root.children = vec![
                node("Save", "[10,20][30,40]"),
                node("Save copy", "[-20,40][0,60]"),
            ];
            Ok(UiPage {
                window_id: "editor:1".into(),
                package_name: "test".into(),
                activity_name: "editor".into(),
                ui_elements: root,
                scope: UiScope::Desktop,
                truncated: false,
            })
        }
        /// Records input without touching the user's desktop.
        fn perform_action(&self, action: &UiAction) -> HostResult<()> {
            let envelope = serde_json::to_value(action).unwrap();
            self.actions.lock().unwrap().push((
                envelope["operation"].as_str().unwrap().into(),
                envelope["parameters"].clone(),
            ));
            Ok(())
        }
    }

    /// Builds a legacy invocation exactly as the ordinary tool pipeline does.
    fn call(name: &str, parameters: &[(&str, &str)]) -> AITool {
        AITool {
            name: name.into(),
            parameters: parameters
                .iter()
                .map(|(name, value)| ToolParameter {
                    name: (*name).into(),
                    value: (*value).into(),
                })
                .collect(),
        }
    }

    /// Rejects every UI entry point when no whole-device provider is registered.
    #[test]
    fn unavailable_system_provider_is_an_explicit_error() {
        let executor = UiToolExecutor { host: None };
        for name in [
            "list_windows",
            "focus_window",
            "get_page_info",
            "tap",
            "long_press",
            "swipe",
            "click_element",
            "set_input_text",
            "press_key",
        ] {
            assert_eq!(
                executor.execute(&call(name, &[])).err().as_deref(),
                Some("This host does not support whole-device UI automation"),
            );
        }
    }

    /// Verifies scope metadata and recursive fields survive the host-to-plugin conversion.
    #[test]
    fn snapshot_keeps_scope_and_hierarchy() {
        let host = Arc::new(FakeHost {
            actions: Mutex::new(vec![]),
        });
        let executor = UiToolExecutor { host: Some(host) };
        let ToolResultData::UIPageResultData(page) =
            executor.execute(&call("get_page_info", &[])).unwrap()
        else {
            panic!("page result")
        };
        assert_eq!(page.scope, "desktop");
        assert!(!page.truncated);
        assert_eq!(page.ui_elements.children.len(), 2);
        assert_eq!(page.ui_elements.children[0].text.as_deref(), Some("Save"));
        assert_eq!(page.ui_elements.children[0].properties["role"], "button");
        assert_eq!(page.window_id, "editor:1");
    }

    /// Keeps discovery read-only, forwards focus and rejects stale explicit targets.
    #[test]
    fn windows_are_discoverable_focusable_and_explicitly_addressable() {
        let host = Arc::new(FakeHost { actions: Mutex::new(vec![]) });
        let executor = UiToolExecutor { host: Some(host.clone()) };
        let ToolResultData::JsonResultData(list) = executor.execute(&call("list_windows", &[])).unwrap() else {
            panic!("window list")
        };
        assert_eq!(list.value["windows"][0]["windowId"], "editor:1");
        assert_eq!(executor.accessSpec(&call("list_windows", &[])).unwrap().effect, ToolEffect::READ);
        executor.execute(&call("focus_window", &[("window_id", "editor:1")])).unwrap();
        assert_eq!(host.actions.lock().unwrap()[0], ("focus_window".into(), serde_json::json!({"window_id": "editor:1"})));
        assert!(executor.execute(&call("get_page_info", &[("window_id", "editor:1")])).is_ok());
        assert!(executor.execute(&call("get_page_info", &[("window_id", "closed")])).is_err());
        assert!(executor.execute(&call("focus_window", &[("window_id", "")])).is_err());
        assert!(executor.execute(&call("get_page_info", &[("window_id", "")])).is_err());
        assert!(executor.execute(&call("list_windows", &[("window_id", "editor:1")])).is_err());
    }

    /// Exercises exact selection, partial matching, indexes and negative monitor coordinates.
    #[test]
    fn click_selects_the_requested_match() {
        let host = Arc::new(FakeHost {
            actions: Mutex::new(vec![]),
        });
        let executor = UiToolExecutor {
            host: Some(host.clone()),
        };
        executor
            .execute(&call("click_element", &[("text", "Save")]))
            .unwrap();
        executor
            .execute(&call(
                "click_element",
                &[("text", "Save"), ("partialMatch", "true"), ("index", "1")],
            ))
            .unwrap();
        let actions = host.actions.lock().unwrap();
        assert_eq!(
            actions[0],
            ("tap".into(), serde_json::json!({"x":20.0,"y":30.0}))
        );
        assert_eq!(
            actions[1],
            ("tap".into(), serde_json::json!({"x":-10.0,"y":50.0}))
        );
    }

    /// Invalid input and unsupported display targets must never dispatch an action.
    #[test]
    fn invalid_actions_do_not_reach_the_provider() {
        let host = Arc::new(FakeHost {
            actions: Mutex::new(vec![]),
        });
        let executor = UiToolExecutor {
            host: Some(host.clone()),
        };
        for tool in [
            call("tap", &[("x", "NaN"), ("y", "2")]),
            call("click_element", &[("text", "missing")]),
            call("click_element", &[("resourceId", "save"), ("index", "0.5")]),
            call("tap", &[("x", "1"), ("y", "2"), ("display", "7")]),
        ] {
            assert!(executor.execute(&tool).is_err());
        }
        assert!(host.actions.lock().unwrap().is_empty());
    }

    /// Empty input remains a valid request to clear the selected text field.
    #[test]
    fn text_can_be_cleared() {
        let host = Arc::new(FakeHost {
            actions: Mutex::new(vec![]),
        });
        let executor = UiToolExecutor {
            host: Some(host.clone()),
        };
        executor
            .execute(&call("set_input_text", &[("text", "")]))
            .unwrap();
        assert_eq!(host.actions.lock().unwrap()[0].1["text"], "");
    }
}
