use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslNode {
    #[serde(rename = "type")]
    pub r#type: String,
    pub props: BTreeMap<String, Value>,
    pub children: Vec<ToolPkgComposeDslNode>,
    #[serde(default)]
    pub slots: BTreeMap<String, Vec<ToolPkgComposeDslNode>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslRenderResult {
    pub tree: ToolPkgComposeDslNode,
    pub state: BTreeMap<String, Value>,
    pub memo: BTreeMap<String, Value>,
}

pub struct ToolPkgComposeDslParser;

impl ToolPkgComposeDslParser {
    #[allow(non_snake_case)]
    pub fn parseRenderResult(rawResult: Option<Value>) -> Option<ToolPkgComposeDslRenderResult> {
        let root = parseRootObject(rawResult)?;
        let treeNode = parseNode(root.get("tree").cloned())?;
        Some(ToolPkgComposeDslRenderResult {
            tree: treeNode,
            state: asMap(root.get("state").cloned()),
            memo: asMap(root.get("memo").cloned()),
        })
    }

    #[allow(non_snake_case)]
    pub fn extractActionId(value: Option<Value>) -> Option<String> {
        match value? {
            Value::Object(object) => object
                .get("__actionId")
                .and_then(kotlinOptString)
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            Value::String(value) => {
                let normalized = value.trim();
                if let Some(actionId) = normalized.strip_prefix("__action:") {
                    let actionId = actionId.trim();
                    if actionId.is_empty() {
                        None
                    } else {
                        Some(actionId.to_string())
                    }
                } else if normalized.is_empty() {
                    None
                } else {
                    Some(normalized.to_string())
                }
            }
            _ => None,
        }
    }
}

#[allow(non_snake_case)]
fn kotlinOptString(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        value => Some(value.to_string()),
    }
}

#[allow(non_snake_case)]
fn parseRootObject(rawResult: Option<Value>) -> Option<serde_json::Map<String, Value>> {
    match rawResult? {
        Value::Object(object) => Some(object),
        Value::String(raw) => parseRootObjectFromString(&raw),
        value => parseRootObjectFromString(&value.to_string()),
    }
}

#[allow(non_snake_case)]
fn parseRootObjectFromString(raw: &str) -> Option<serde_json::Map<String, Value>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        return None;
    }
    let first = serde_json::from_str::<Value>(trimmed).ok()?;
    match first {
        Value::Object(object) => Some(object),
        Value::String(nested) => {
            let nested = nested.trim();
            if nested.starts_with('{') && nested.ends_with('}') {
                serde_json::from_str::<Value>(nested)
                    .ok()
                    .and_then(|value| match value {
                        Value::Object(object) => Some(object),
                        _ => None,
                    })
            } else {
                None
            }
        }
        _ => None,
    }
}

#[allow(non_snake_case)]
fn parseNode(value: Option<Value>) -> Option<ToolPkgComposeDslNode> {
    let Value::Object(nodeObj) = value? else {
        return None;
    };
    let nodeType = nodeObj
        .get("type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?
        .to_string();
    let props = asMap(nodeObj.get("props").cloned());
    let mut children = Vec::new();
    let mut slots = BTreeMap::<String, Vec<ToolPkgComposeDslNode>>::new();

    match nodeObj.get("children") {
        Some(Value::Array(rawChildren)) => {
            for child in rawChildren {
                if let Some(node) = parseNode(Some(child.clone())) {
                    children.push(node);
                }
            }
        }
        Some(Value::Object(rawChild)) => {
            if let Some(node) = parseNode(Some(Value::Object(rawChild.clone()))) {
                children.push(node);
            }
        }
        _ => {}
    }

    if let Some(Value::Object(rawSlots)) = nodeObj.get("slots") {
        for (slotName, rawSlotValue) in rawSlots {
            let mut slotChildren = Vec::new();
            match rawSlotValue {
                Value::Array(rawChildren) => {
                    for child in rawChildren {
                        if let Some(node) = parseNode(Some(child.clone())) {
                            slotChildren.push(node);
                        }
                    }
                }
                Value::Object(rawChild) => {
                    if let Some(node) = parseNode(Some(Value::Object(rawChild.clone()))) {
                        slotChildren.push(node);
                    }
                }
                _ => {}
            }
            if !slotChildren.is_empty() {
                slots.insert(slotName.clone(), slotChildren);
            }
        }
    }

    Some(ToolPkgComposeDslNode {
        r#type: nodeType,
        props,
        children,
        slots,
    })
}

#[allow(non_snake_case)]
fn asMap(value: Option<Value>) -> BTreeMap<String, Value> {
    match value {
        Some(Value::Object(object)) => object.into_iter().collect(),
        _ => BTreeMap::new(),
    }
}
