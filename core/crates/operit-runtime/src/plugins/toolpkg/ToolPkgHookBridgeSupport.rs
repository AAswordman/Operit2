use serde_json::Value;

use crate::core::application::OperitApplication::OperitApplication;
use operit_tools::tools::packTool::PackageManager::PackageManager;
use operit_tools::tools::AIToolHandler::AIToolHandler;

#[derive(Clone, Debug)]
pub struct ToolPkgAppLifecycleHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub event: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolPkgMessageProcessingHookRegistration {
    pub containerPackageName: String,
    pub pluginId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolPkgXmlRenderHookRegistration {
    pub containerPackageName: String,
    pub pluginId: String,
    pub tag: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolPkgInputMenuToggleHookRegistration {
    pub containerPackageName: String,
    pub pluginId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolPkgChatInputHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolPkgChatViewHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolPkgToolLifecycleHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolPkgPromptHookRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub functionName: String,
    pub functionSource: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ToolPkgAiProviderRegistration {
    pub containerPackageName: String,
    pub providerId: String,
    pub displayName: String,
    pub description: String,
    pub listModelsFunctionName: String,
    pub listModelsFunctionSource: Option<String>,
    pub sendMessageFunctionName: String,
    pub sendMessageFunctionSource: Option<String>,
    pub testConnectionFunctionName: String,
    pub testConnectionFunctionSource: Option<String>,
    pub calculateInputTokensFunctionName: String,
    pub calculateInputTokensFunctionSource: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ToolPkgHostEventRegistration {
    pub containerPackageName: String,
    pub hookId: String,
    pub source: String,
    pub trigger: serde_json::Value,
    pub functionName: String,
    pub functionSource: Option<String>,
    pub enabled: bool,
}

#[allow(non_snake_case)]
pub fn toolPkgPackageManager() -> PackageManager {
    let hostManager = OperitApplication::hostManager();
    AIToolHandler::getInstance(hostManager)
        .getOrCreatePackageManager()
        .lock()
        .expect("package manager mutex poisoned")
        .clone()
}

#[allow(non_snake_case)]
pub fn toolPkgToolHandler() -> AIToolHandler {
    let hostManager = OperitApplication::hostManager();
    AIToolHandler::getInstance(hostManager)
}

#[allow(non_snake_case)]
pub fn decodeToolPkgHookResult(raw: Option<String>) -> Option<Value> {
    let text = raw?;
    let normalized = text.trim();
    if normalized.is_empty() {
        return Some(Value::String(text));
    }
    match serde_json::from_str::<Value>(normalized) {
        Ok(value) => Some(value),
        Err(_) => Some(Value::String(text)),
    }
}
