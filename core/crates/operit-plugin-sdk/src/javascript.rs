use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::execution_result::JsExecutionResult;
use crate::package::ToolPackage;
use crate::toolpkg::ToolPkgParser::ToolPkgSubpackageRuntime;

/// Describes one tool call issued by JavaScript package code.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsToolCallRequest {
    pub tool_type: String,
    pub tool_name: String,
    pub parameters: BTreeMap<String, Value>,
}

impl JsToolCallRequest {
    /// Returns the package-qualified tool name understood by the Rust host.
    pub fn qualified_tool_name(&self) -> String {
        let tool_name = self.tool_name.trim();
        let tool_type = self.tool_type.trim();
        if tool_type.is_empty() || tool_type == "default" {
            tool_name.to_string()
        } else {
            format!("{tool_type}:{tool_name}")
        }
    }
}

/// Represents tool output returned to JavaScript package code.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum JsToolCallResultData {
    Binary(Vec<u8>),
    Value(Value),
}

impl Default for JsToolCallResultData {
    /// Creates an empty JSON result value.
    fn default() -> Self {
        Self::Value(Value::Null)
    }
}

/// Contains the stable result envelope returned by Rust tool implementations.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsToolCallResult {
    pub success: bool,
    pub data: JsToolCallResultData,
    pub error: Option<String>,
}

/// Describes one ToolPkg resource materialization request from JavaScript.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsToolPkgResourceRequest {
    pub package_name_or_subpackage_id: String,
    pub resource_key: String,
    pub output_file_name: Option<String>,
    pub internal: bool,
}

/// Describes one package-aware tool name resolution request.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsToolNameResolutionRequest {
    pub package_name: Option<String>,
    pub subpackage_id: Option<String>,
    pub tool_name: String,
    pub prefer_imported: bool,
}

/// Describes one ToolPkg runtime-to-runtime IPC request.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsToolPkgIpcRequest {
    pub package_target: String,
    pub caller_context_key: String,
    pub target_context_key: Option<String>,
    pub target_runtime: Option<String>,
    pub channel: String,
    pub payload: Value,
}

/// Defines the fixed Rust execution contract required by package JavaScript.
pub trait JsExecutionHost: crate::js_sdk::JsToolsHost + Send + Sync {
    /// Executes one validated tool call through the embedding application's tool system.
    fn execute_tool_call(&self, request: JsToolCallRequest) -> JsToolCallResult;

    /// Returns the language code exposed to package JavaScript.
    fn package_language(&self) -> Result<String, String>;

    /// Reads one environment variable exposed to package JavaScript.
    fn read_environment_variable(&self, key: &str) -> Result<Option<String>, String>;

    /// Returns the writable configuration directory for one plugin.
    fn plugin_config_dir(&self, plugin_id: &str) -> Result<String, String>;

    /// Reads one UTF-8 ToolPkg resource.
    fn read_toolpkg_text_resource(
        &self,
        package_name_or_subpackage_id: &str,
        resource_path: &str,
    ) -> Result<String, String>;

    /// Materializes one ToolPkg resource and returns its output path.
    fn materialize_toolpkg_resource(
        &self,
        request: JsToolPkgResourceRequest,
    ) -> Result<String, String>;

    /// Handles one Compose DSL WebView controller command.
    fn handle_compose_webview_controller_command(
        &self,
        payload_json: &str,
    ) -> Result<String, String>;

    /// Returns whether one package is currently imported.
    fn is_package_imported(&self, package_name: &str) -> Result<bool, String>;

    /// Imports one package into the active package set.
    fn import_package(&self, package_name: &str) -> Result<String, String>;

    /// Removes one package from the active package set.
    fn remove_package(&self, package_name: &str) -> Result<String, String>;

    /// Activates one imported package for the current execution.
    fn use_package(&self, package_name: &str) -> Result<String, String>;

    /// Lists all currently imported package names.
    fn list_imported_packages(&self) -> Result<Vec<String>, String>;

    /// Resolves one package-aware JavaScript tool name.
    fn resolve_tool_name(&self, request: JsToolNameResolutionRequest) -> Result<String, String>;

    /// Dispatches one ToolPkg IPC request to the selected runtime.
    fn invoke_toolpkg_ipc(&self, request: JsToolPkgIpcRequest) -> Result<Value, String>;
}

/// Executes package JavaScript through one bound package and host context.
pub trait JsPackageExecutor: Send + Sync {
    /// Executes one package tool through the bound execution context.
    fn execute_package_tool(
        &self,
        script: &str,
        request: &JsPackageToolCallRequest,
    ) -> JsPackageToolCallResult;
}

/// Supplies concrete JavaScript execution services to an embedding application.
pub trait JsExecutionProvider: Send + Sync {
    /// Creates one JavaScript engine bound to a caller-owned execution host.
    fn create_execution_engine(
        &self,
        execution_host: Arc<dyn JsExecutionHost>,
    ) -> Arc<dyn JsExecutionEngine>;

    /// Creates one package executor bound to caller-owned runtime contracts.
    fn create_package_executor(
        &self,
        package_runtime: Arc<dyn JsPackageRuntime>,
        execution_host: Arc<dyn JsExecutionHost>,
    ) -> Arc<dyn JsPackageExecutor>;
}
/// Describes one Rust-to-JavaScript package tool invocation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsPackageToolCallRequest {
    pub tool_name: String,
    pub parameters: BTreeMap<String, String>,
}

/// Contains the stable result returned by JavaScript package execution.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsPackageToolCallResult {
    pub tool_name: String,
    pub success: bool,
    pub result: String,
    pub error: Option<String>,
}

/// Supplies package state and ToolPkg engines to a JavaScript bridge.
pub trait JsPackageRuntime: Send + Sync {
    /// Returns the language code exposed to package JavaScript.
    fn package_language(&self) -> Result<String, String>;

    /// Returns one registered package definition.
    fn package(&self, package_name: &str) -> Option<ToolPackage>;

    /// Returns the active conditional state id for one package.
    fn active_package_state_id(&self, package_name: &str) -> Option<String>;

    /// Resolves ToolPkg runtime metadata for one executable subpackage.
    fn resolve_toolpkg_subpackage(&self, package_name: &str) -> Option<ToolPkgSubpackageRuntime>;

    /// Returns the shared ToolPkg engine for one execution context.
    fn toolpkg_execution_engine(&self, context_key: &str) -> Arc<dyn JsExecutionEngine>;
}

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
    fn execute_script_function(
        &self,
        script: &str,
        function_name: &str,
        params: &BTreeMap<String, Value>,
        env_overrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatch_intermediate_on_main: bool,
        timeout_sec: u64,
    ) -> JsExecutionResult<Option<String>>;

    /// Executes a ToolPkg registration function and returns captured declarations.
    fn execute_toolpkg_main_registration_function_with_text_resources(
        &self,
        script: &str,
        function_name: &str,
        params: &BTreeMap<String, Value>,
        text_resources: Option<Arc<BTreeMap<String, String>>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture>;

    /// Executes one Compose DSL render script.
    fn execute_compose_dsl_script(
        &self,
        script: &str,
        runtime_options: &BTreeMap<String, Value>,
        env_overrides: &BTreeMap<String, String>,
    ) -> JsExecutionResult<Option<String>>;

    /// Dispatches one Compose DSL action and emits intermediate render events.
    fn dispatch_compose_dsl_action(
        &self,
        action_id: &str,
        payload: Option<Value>,
        runtime_options: &BTreeMap<String, Value>,
        env_overrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> JsExecutionResult<Option<String>>;

    /// Destroys any engine resources owned by this handle.
    fn destroy(&self);
}
