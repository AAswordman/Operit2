use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ConversationMarkupManager::ToolResult;
use crate::ToolExecutionManager::AITool;
use crate::tools::AIToolHandler::AIToolHandler;

/// Captured metadata emitted by a package's main registration script.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgMainRegistrationCapture {
    #[serde(rename = "toolboxUiModules", default)]
    pub toolboxUiModules: Vec<String>,
    #[serde(rename = "uiRoutes", default)]
    pub uiRoutes: Vec<String>,
    #[serde(rename = "navigationEntries", default)]
    pub navigationEntries: Vec<String>,
    #[serde(rename = "desktopWidgets", default)]
    pub desktopWidgets: Vec<String>,
    #[serde(rename = "appLifecycleHooks", default)]
    pub appLifecycleHooks: Vec<String>,
    #[serde(rename = "messageProcessingPlugins", default)]
    pub messageProcessingPlugins: Vec<String>,
    #[serde(rename = "xmlRenderPlugins", default)]
    pub xmlRenderPlugins: Vec<String>,
    #[serde(rename = "inputMenuTogglePlugins", default)]
    pub inputMenuTogglePlugins: Vec<String>,
    #[serde(rename = "chatInputHooks", default)]
    pub chatInputHooks: Vec<String>,
    #[serde(rename = "chatViewHooks", default)]
    pub chatViewHooks: Vec<String>,
    #[serde(rename = "hostEventHooks", default)]
    pub hostEventHooks: Vec<String>,
    #[serde(rename = "toolLifecycleHooks", default)]
    pub toolLifecycleHooks: Vec<String>,
    #[serde(rename = "promptInputHooks", default)]
    pub promptInputHooks: Vec<String>,
    #[serde(rename = "promptHistoryHooks", default)]
    pub promptHistoryHooks: Vec<String>,
    #[serde(rename = "promptEstimateHistoryHooks", default)]
    pub promptEstimateHistoryHooks: Vec<String>,
    #[serde(rename = "systemPromptComposeHooks", default)]
    pub systemPromptComposeHooks: Vec<String>,
    #[serde(rename = "toolPromptComposeHooks", default)]
    pub toolPromptComposeHooks: Vec<String>,
    #[serde(rename = "promptFinalizeHooks", default)]
    pub promptFinalizeHooks: Vec<String>,
    #[serde(rename = "promptEstimateFinalizeHooks", default)]
    pub promptEstimateFinalizeHooks: Vec<String>,
    #[serde(rename = "summaryGenerateHooks", default)]
    pub summaryGenerateHooks: Vec<String>,
    #[serde(rename = "aiProviders", default)]
    pub aiProviders: Vec<String>,
}

/// JavaScript execution handle supplied by the JS bridge crate.
pub trait JsExecutionEngine: Send + Sync {
    /// Executes a named JavaScript function for ToolPkg runtime hooks.
    #[allow(non_snake_case)]
    fn executeScriptFunction(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatchIntermediateOnMain: bool,
        timeoutSec: u64,
    ) -> Option<String>;

    /// Executes a ToolPkg registration function and returns captured declarations.
    #[allow(non_snake_case)]
    fn executeToolPkgMainRegistrationFunctionWithTextResources(
        &self,
        script: &str,
        functionName: &str,
        params: &BTreeMap<String, Value>,
        textResources: Option<Arc<BTreeMap<String, String>>>,
    ) -> Result<ToolPkgMainRegistrationCapture, String>;

    /// Executes one Compose DSL render script.
    #[allow(non_snake_case)]
    fn executeComposeDslScript(
        &self,
        script: &str,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
    ) -> Option<String>;

    /// Dispatches one Compose DSL action and emits intermediate render events.
    #[allow(non_snake_case)]
    fn dispatchComposeDslAction(
        &self,
        actionId: &str,
        payload: Option<Value>,
        runtimeOptions: &BTreeMap<String, Value>,
        envOverrides: &BTreeMap<String, String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Option<String>;

    /// Destroys any engine resources owned by this handle.
    fn destroy(&self);
}

/// JavaScript support installed by the parent crate.
pub trait JsBridgeSupport: Send + Sync {
    /// Creates a general package execution engine.
    #[allow(non_snake_case)]
    fn newPackageExecutionEngine(&self, toolHandler: AIToolHandler) -> Arc<dyn JsExecutionEngine>;

    /// Creates a ToolPkg execution engine.
    #[allow(non_snake_case)]
    fn newToolPkgExecutionEngine(&self, toolHandler: AIToolHandler) -> Arc<dyn JsExecutionEngine>;

    /// Executes one JavaScript package tool.
    #[allow(non_snake_case)]
    fn executePackageTool(
        &self,
        packageManager: Arc<std::sync::Mutex<crate::tools::packTool::PackageManager::PackageManager>>,
        toolHandler: AIToolHandler,
        script: &str,
        tool: &AITool,
    ) -> Vec<ToolResult>;
}

static JS_BRIDGE_SUPPORT: OnceLock<Arc<dyn JsBridgeSupport>> = OnceLock::new();

/// Installs the JavaScript bridge implementation supplied by the JS crate.
#[allow(non_snake_case)]
pub fn setJsBridgeSupport(support: Arc<dyn JsBridgeSupport>) -> Result<(), String> {
    JS_BRIDGE_SUPPORT
        .set(support)
        .map_err(|_| "JavaScript bridge support is already installed".to_string())
}

/// Returns the active JavaScript bridge support.
#[allow(non_snake_case)]
pub fn jsBridgeSupport() -> &'static dyn JsBridgeSupport {
    JS_BRIDGE_SUPPORT
        .get()
        .map(|support| support.as_ref())
        .expect("JavaScript bridge support must be installed before JavaScript tools are used")
}
