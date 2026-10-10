use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::execution_result::JsExecutionResult;
use crate::package::ToolPackage;
use crate::toolpkg::ToolPkgParser::{ToolPkgMarketOrigin, ToolPkgSubpackageRuntime};

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

/// Describes one scalar argument passed from JavaScript into a ToolPkg WASM export.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsToolPkgWasmArg {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: Value,
}

/// Describes one ToolPkg WASM export call requested by JavaScript.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsToolPkgWasmRequest {
    pub package_target: String,
    pub module_id: String,
    pub export_name: String,
    pub args: Vec<JsToolPkgWasmArg>,
}

/// Contains one ToolPkg WASM export result returned to JavaScript.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JsToolPkgWasmResult {
    pub value_type: Option<String>,
    pub value: Value,
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
    /// Identifies a dependency caller injected by the engine, never by JavaScript arguments.
    pub dependency_caller: Option<String>,
    pub package_target: String,
    pub caller_context_key: String,
    pub target_context_key: Option<String>,
    pub target_runtime: Option<String>,
    pub channel: String,
    pub payload: Value,
}

/// Completes one asynchronous ToolPkg IPC request on the source execution host.
pub type JsToolPkgIpcCompletion = Box<dyn FnOnce(Result<Value, String>) + Send + 'static>;

/// Represents one locally polled JavaScript execution future.
pub type JsExecutionFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;

/// Represents a Send result while its local JavaScript session is driven by the owning Host.
pub type JsExecutionCompletion<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Identifies the storage scope selected before a ToolPkg registration is evaluated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolPkgConfigScope {
    Device,
    Space,
}

impl ToolPkgConfigScope {
    /// Returns the stable scope key understood by the runtime storage compatibility layer.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Device => "device",
            Self::Space => "space",
        }
    }
}

/// Defines the fixed Rust execution contract required by package JavaScript.
pub trait JsExecutionHost: crate::js_sdk::JsToolsHost + Send + Sync {
    /// Binds a host instance to an engine-owned authenticated ToolPkg execution context.
    fn for_toolpkg_execution_context(
        &self,
        context: &ToolPkgExecutionContext,
    ) -> Result<Arc<dyn JsExecutionHost>, String>;

    /// Returns the current executable tool catalog with parameter schemas.
    fn get_tool_catalog(&self) -> Result<serde_json::Value, String>;

    /// Executes one validated tool call through the embedding application's tool system.
    fn execute_tool_call(
        &self,
        request: JsToolCallRequest,
    ) -> JsExecutionCompletion<JsToolCallResult>;

    /// Returns the language code exposed to package JavaScript.
    fn package_language(&self) -> Result<String, String>;

    /// Reads one environment variable exposed to package JavaScript.
    fn read_environment_variable(&self, key: &str) -> Result<Option<String>, String>;

    /// Writes one environment variable exposed to package JavaScript.
    fn write_environment_variable(&self, key: &str, value: &str) -> Result<(), String>;

    /// Returns the writable configuration directory for one plugin.
    fn plugin_config_dir(&self, plugin_id: &str) -> Result<String, String>;

    /// Returns a named configuration directory owned by the executing package.
    fn scoped_plugin_config_dir(&self, owner_id: &str, plugin_id: &str) -> Result<String, String>;

    /// Creates package configuration in the explicit scope selected for registration.
    fn registration_plugin_config_dir(
        &self,
        owner_id: &str,
        plugin_id: &str,
        scope: ToolPkgConfigScope,
    ) -> Result<String, String>;

    /// Creates stable device-local data for the current installed package container.
    fn plugin_local_data_dir(&self, owner_id: &str) -> Result<String, String>;

    /// Creates stable device-local data for the owner selected before registration.
    fn registration_plugin_local_data_dir(&self, owner_id: &str) -> Result<String, String>;

    /// Creates persistent shared data independently of the package's installation scope.
    fn plugin_space_data_dir(&self, owner_id: &str) -> Result<String, String>;

    /// Creates persistent shared data for the package selected before registration.
    fn registration_plugin_space_data_dir(&self, owner_id: &str) -> Result<String, String>;

    /// Reads one UTF-8 ToolPkg resource.
    fn read_toolpkg_text_resource(
        &self,
        package_name_or_subpackage_id: &str,
        resource_path: &str,
    ) -> Result<Option<String>, String>;

    /// Materializes one ToolPkg resource and returns its output path.
    fn materialize_toolpkg_resource(
        &self,
        request: JsToolPkgResourceRequest,
    ) -> Result<String, String>;

    /// Calls one scalar ToolPkg WASM export.
    fn call_toolpkg_wasm(
        &self,
        request: JsToolPkgWasmRequest,
    ) -> Result<JsToolPkgWasmResult, String>;

    /// Handles one Compose DSL WebView controller command.
    fn handle_compose_webview_controller_command(&self, payload: &Value) -> Result<Value, String>;

    /// Opens one Compose DSL file picker through the embedding application's host UI.
    fn open_compose_file_picker(&self, payload: &Value) -> Result<Value, String>;

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

    /// Dispatches one ToolPkg IPC request to the selected runtime without blocking the source engine.
    fn invoke_toolpkg_ipc_async(
        &self,
        request: JsToolPkgIpcRequest,
        completion: JsToolPkgIpcCompletion,
    ) -> Result<(), String>;

    /// Waits for the host runtime to advance JavaScript work on a later turn.
    fn wait_for_javascript_runtime_turn(&self) -> operit_host_api::HostRuntimeTurnFuture;
}

/// Executes package JavaScript through one bound package and host context.
pub trait JsPackageExecutor: Send + Sync {
    /// Executes one package tool through the bound execution context.
    fn execute_package_tool(
        &self,
        request: &JsPackageToolCallRequest,
    ) -> JsExecutionCompletion<JsPackageToolCallResult>;
}

/// Supplies concrete JavaScript execution services to an embedding application.
pub trait JsExecutionProvider: Send + Sync {
    /// Creates one JavaScript engine bound to a caller-owned execution host.
    fn create_execution_engine(
        &self,
        execution_host: Arc<dyn JsExecutionHost>,
    ) -> Result<Arc<dyn JsExecutionEngine>, String>;

    /// Creates one JavaScript engine bound to a ToolPkg package environment.
    fn create_toolpkg_execution_engine(
        &self,
        execution_host: Arc<dyn JsExecutionHost>,
        context: ToolPkgExecutionContext,
    ) -> Result<Arc<dyn JsExecutionEngine>, String>;

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

/// Captures an exact executable declaration together with its immutable runtime selection.
#[derive(Clone, Debug)]
pub struct JsPackageToolSelection {
    /// Contains the selected registered declaration, including its authoritative script and parameter schema.
    pub definition: crate::package::PackageTool,
    /// Contains the selected conditional state when the registered package uses one.
    pub active_state_id: Option<String>,
    /// Contains the actual registered owning ToolPkg context for a subpackage.
    pub toolpkg_runtime: Option<ToolPkgSubpackageRuntime>,
}

/// Selects an exact executable declaration, rejecting unknown, duplicate, advisory and empty-script entries.
pub fn select_registered_package_tool(
    package: ToolPackage,
    function_name: &str,
) -> Result<crate::package::PackageTool, String> {
    let package_name = package.name.clone();
    let mut matches = package
        .tools
        .into_iter()
        .filter(|tool| tool.name == function_name);
    let definition = matches
        .next()
        .ok_or_else(|| format!("Tool not registered: {package_name}:{function_name}"))?;
    if matches.next().is_some() {
        return Err(format!(
            "Duplicate tool registration: {package_name}:{function_name}"
        ));
    }
    if definition.advice {
        return Err(format!(
            "Tool is advisory and cannot execute: {package_name}:{function_name}"
        ));
    }
    if definition.script.trim().is_empty() {
        return Err(format!(
            "Registered tool script is empty: {package_name}:{function_name}"
        ));
    }
    let mut parameters = std::collections::HashSet::new();
    for parameter in &definition.parameters {
        if !parameters.insert(&parameter.name) {
            return Err(format!(
                "Duplicate tool parameter: {package_name}:{function_name}:{}",
                parameter.name
            ));
        }
    }
    Ok(definition)
}

/// Supplies package state and ToolPkg engines to a JavaScript bridge.
pub trait JsPackageRuntime: Send + Sync {
    /// Returns the language code exposed to package JavaScript.
    fn package_language(&self) -> Result<String, String>;

    /// Selects one enabled executable declaration and its owning context from a single registry snapshot.
    fn select_tool(
        &self,
        package_name: &str,
        function_name: &str,
    ) -> Result<JsPackageToolSelection, String>;

    /// Returns the shared ToolPkg engine for one explicitly owned execution context.
    fn toolpkg_execution_engine(
        &self,
        context_key: &str,
        container_package_name: &str,
    ) -> Result<Arc<dyn JsExecutionEngine>, String>;
}

/// Captured metadata emitted by a package's main registration script.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgMainRegistrationCapture {
    #[serde(rename = "marketOrigin", default)]
    pub marketOrigin: Option<ToolPkgMarketOrigin>,
    #[serde(rename = "toolboxUiModules", default)]
    pub toolboxUiModules: Vec<String>,
    #[serde(rename = "uiRoutes", default)]
    pub uiRoutes: Vec<String>,
    #[serde(rename = "chatComposerSlots", default)]
    pub chatComposerSlots: Vec<String>,
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
    #[serde(rename = "chatMessageHooks", default)]
    pub chatMessageHooks: Vec<String>,
    /// Captures real package-owned lifecycle handler definitions for the strict registration parser.
    #[serde(rename = "chatLifecycleHooks", default)]
    pub chatLifecycleHooks: Vec<String>,
    #[serde(rename = "chatMessageMenuItems", default)]
    pub chatMessageMenuItems: Vec<String>,
    #[serde(rename = "chatRuntimeHooks", default)]
    pub chatRuntimeHooks: Vec<String>,
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
    #[serde(rename = "coreCommands", default)]
    pub coreCommands: Vec<String>,
    #[serde(rename = "aiProviders", default)]
    pub aiProviders: Vec<String>,
    #[serde(rename = "manifestExtensions", default)]
    pub manifestExtensions: Vec<String>,
    #[serde(rename = "publicApis", default)]
    pub publicApis: Vec<String>,
}

/// Resolves UTF-8 module resources for ToolPkg JavaScript execution contexts.
pub trait ToolPkgTextResourceHost: Send + Sync {
    /// Reads one UTF-8 resource from a registered ToolPkg container or subpackage.
    fn read_toolpkg_text_resource(
        &self,
        package_name_or_subpackage_id: &str,
        resource_path: &str,
    ) -> Result<Option<String>, String>;
}

/// Immutable package environment owned by one ToolPkg JavaScript execution context.
#[derive(Clone)]
pub struct ToolPkgExecutionContext {
    pub context_key: String,
    pub container_package_name: String,
    pub api_version: String,
    pub text_resource_host: Arc<dyn ToolPkgTextResourceHost>,
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
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>>;

    /// Executes a named JavaScript function with an exact millisecond timeout for ToolPkg runtime hooks.
    fn execute_script_function_with_timeout_millis(
        &self,
        script: &str,
        function_name: &str,
        params: &BTreeMap<String, Value>,
        env_overrides: &BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatch_intermediate_on_main: bool,
        timeout_millis: u64,
    ) -> JsExecutionCompletion<JsExecutionResult<Option<String>>>;

    /// Executes a named JavaScript function without blocking the caller runtime.
    fn execute_script_function_async(
        &self,
        script: String,
        function_name: String,
        params: BTreeMap<String, Value>,
        env_overrides: BTreeMap<String, String>,
        on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        dispatch_intermediate_on_main: bool,
        timeout_millis: u64,
    ) -> JsExecutionFuture<JsExecutionResult<Option<String>>>;

    /// Executes a ToolPkg registration function and returns captured declarations.
    fn execute_toolpkg_main_registration_function_with_text_resources(
        &self,
        script: &str,
        function_name: &str,
        params: &BTreeMap<String, Value>,
        text_resources: Option<Arc<BTreeMap<String, String>>>,
    ) -> JsExecutionResult<ToolPkgMainRegistrationCapture>;

    /// Executes a streamed render with a synchronous sink for every generated retained commit.
    fn execute_compose_dsl_script_stream_async(
        &self,
        script: String,
        runtime_options: BTreeMap<String, Value>,
        env_overrides: BTreeMap<String, String>,
        text_resources: Arc<BTreeMap<String, String>>,
        on_response: Arc<dyn Fn(Value) + Send + Sync>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>>;

    /// Dispatches one Compose DSL action without blocking the caller runtime.
    fn dispatch_compose_dsl_action_stream_async(
        &self,
        action_id: String,
        payload: Option<Value>,
        runtime_options: BTreeMap<String, Value>,
        env_overrides: BTreeMap<String, String>,
        on_intermediate_result: Arc<dyn Fn(Value) + Send + Sync>,
    ) -> JsExecutionFuture<JsExecutionResult<Option<Value>>>;

    /// Destroys any engine resources owned by this handle.
    fn destroy(&self);
}
