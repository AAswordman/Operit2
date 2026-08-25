use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgXmlRenderHookComposeDslResult {
    pub screen: String,
    #[serde(default)]
    pub state: HashMap<String, Value>,
    #[serde(default)]
    pub memo: HashMap<String, Value>,
    #[serde(rename = "moduleSpec", default)]
    pub moduleSpec: Option<HashMap<String, Value>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgXmlRenderHookObjectResult {
    #[serde(default)]
    pub handled: Option<bool>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(rename = "composeDsl", default)]
    pub composeDsl: Option<ToolPkgXmlRenderHookComposeDslResult>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgToolLifecycleEventPayload {
    #[serde(rename = "toolName")]
    pub toolName: String,
    #[serde(default)]
    pub parameters: HashMap<String, String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub granted: Option<bool>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(rename = "errorMessage", default)]
    pub errorMessage: Option<String>,
    #[serde(rename = "resultText", default)]
    pub resultText: Option<String>,
    #[serde(rename = "resultJson", default)]
    pub resultJson: Option<HashMap<String, Value>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgPromptTurn {
    pub kind: String,
    pub content: String,
    #[serde(rename = "toolName", default)]
    pub toolName: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgPromptHookObjectResult {
    #[serde(rename = "rawInput", default)]
    pub rawInput: Option<String>,
    #[serde(rename = "processedInput", default)]
    pub processedInput: Option<String>,
    #[serde(rename = "chatHistory", default)]
    pub chatHistory: Option<Vec<ToolPkgPromptTurn>>,
    #[serde(rename = "preparedHistory", default)]
    pub preparedHistory: Option<Vec<ToolPkgPromptTurn>>,
    #[serde(rename = "systemPrompt", default)]
    pub systemPrompt: Option<String>,
    #[serde(rename = "toolPrompt", default)]
    pub toolPrompt: Option<String>,
    #[serde(rename = "availableTools", default)]
    pub availableTools: Option<Vec<HashMap<String, Value>>>,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}
