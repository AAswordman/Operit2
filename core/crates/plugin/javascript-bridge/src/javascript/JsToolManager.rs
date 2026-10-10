use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use serde_json::Value;

use operit_plugin_sdk::execution_result::{JsExecutionError, JsExecutionResult};
use operit_plugin_sdk::javascript::{
    JsExecutionCompletion, JsExecutionEngine, JsPackageExecutor, JsPackageRuntime,
    JsPackageToolCallRequest, JsPackageToolCallResult, JsPackageToolSelection,
};
use operit_plugin_sdk::toolpkg::ToolPkgManager::ToolPkgExecutionEngineFactory;

#[derive(Clone)]
/// Executes JavaScript-backed package tools through reusable JS engines.
pub struct JsToolManager {
    packageRuntime: Arc<dyn JsPackageRuntime>,
    enginePool: Arc<Mutex<Vec<Arc<dyn JsExecutionEngine>>>>,
    executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
    enginePermits: Arc<Semaphore>,
}

/// Returns an engine to its pool before releasing its asynchronous permit.
struct JsEngineLease {
    engine: Arc<dyn JsExecutionEngine>,
    pool: Arc<Mutex<Vec<Arc<dyn JsExecutionEngine>>>>,
    _permit: OwnedSemaphorePermit,
}

impl Drop for JsEngineLease {
    /// Returns the leased engine when execution completes or is cancelled.
    fn drop(&mut self) {
        self.pool
            .lock()
            .expect("JsToolManager engine pool mutex poisoned")
            .push(self.engine.clone());
    }
}

#[derive(Debug)]
struct ToolParameterConversionException {
    message: String,
}

const MAX_CONCURRENT_ENGINES: usize = 4;
impl JsToolManager {
    /// Creates a JavaScript tool manager from SDK package and engine contracts.
    pub(crate) fn new(
        packageRuntime: Arc<dyn JsPackageRuntime>,
        executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
    ) -> Self {
        Self {
            packageRuntime,
            enginePool: Arc::new(Mutex::new(Vec::new())),
            executionEngineFactory,
            enginePermits: Arc::new(Semaphore::new(MAX_CONCURRENT_ENGINES)),
        }
    }

    /// Acquires an engine slot asynchronously and keeps it leased across execution.
    #[allow(non_snake_case)]
    async fn withEngine<T>(
        &self,
        block: impl FnOnce(Arc<dyn JsExecutionEngine>) -> JsExecutionCompletion<JsExecutionResult<T>>,
    ) -> JsExecutionResult<T> {
        let permit = self
            .enginePermits
            .clone()
            .acquire_owned()
            .await
            .expect("JsToolManager engine semaphore must remain open");
        let idleEngine = self
            .enginePool
            .lock()
            .expect("JsToolManager engine pool mutex poisoned")
            .pop();
        let engine = match idleEngine {
            Some(engine) => engine,
            None => self.executionEngineFactory.createExecutionEngine()
                .map_err(JsExecutionError::initialization)?,
        };
        let lease = JsEngineLease {
            engine,
            pool: self.enginePool.clone(),
            _permit: permit,
        };
        block(lease.engine.clone()).await
    }

    /// Selects the package engine without holding the pool lock across await.
    #[allow(non_snake_case)]
    async fn withExecutionEngineForPackage<T>(
        &self,
        selection: &JsPackageToolSelection,
        block: impl FnOnce(Arc<dyn JsExecutionEngine>) -> JsExecutionCompletion<JsExecutionResult<T>>,
    ) -> JsExecutionResult<T> {
        if let Some(runtime) = selection.toolpkg_runtime.as_ref() {
            let contextKey = format!("toolpkg_main:{}", runtime.containerPackageName);
            let engine = self
                .packageRuntime
                .toolpkg_execution_engine(&contextKey, &runtime.containerPackageName)
                .map_err(JsExecutionError::initialization)?;
            return block(engine).await;
        }
        self.withEngine(block).await
    }

    #[allow(non_snake_case)]
    fn parseDotCall(toolName: &str) -> Option<(String, String)> {
        let separatorIndex = toolName.rfind('.')?;
        if separatorIndex == 0 || separatorIndex >= toolName.len() - 1 {
            return None;
        }
        Some((
            toolName[..separatorIndex].to_string(),
            toolName[separatorIndex + 1..].to_string(),
        ))
    }

    #[allow(non_snake_case)]
    fn parsePackageToolName(toolName: &str) -> Option<(String, String)> {
        let separatorIndex = toolName.find(':')?;
        if separatorIndex == 0 || separatorIndex >= toolName.len() - 1 {
            return None;
        }
        Some((
            toolName[..separatorIndex].to_string(),
            toolName[separatorIndex + 1..].to_string(),
        ))
    }

    #[allow(non_snake_case)]
    fn buildRuntimeParams(
        &self,
        packageName: &str,
        selection: &JsPackageToolSelection,
        params: BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, Value>, String> {
        let mut runtimeParams = params;
        let packageLanguage = self.packageRuntime.package_language()?;
        runtimeParams.insert(
            "__operit_package_lang".to_string(),
            Value::String(packageLanguage),
        );
        if let Some(stateId) = selection.active_state_id.as_ref() {
            runtimeParams.insert(
                "__operit_package_state".to_string(),
                Value::String(stateId.clone()),
            );
        }

        for key in [
            "__operit_package_caller_name",
            "__operit_package_chat_id",
            "__operit_package_caller_participant_id",
            "__operit_package_caller_owner",
        ] {
            let value = runtimeParams
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            match value {
                Some(value) => {
                    runtimeParams.insert(key.to_string(), Value::String(value));
                }
                None => {
                    runtimeParams.remove(key);
                }
            }
        }

        runtimeParams.insert(
            "__operit_package_name".to_string(),
            Value::String(packageName.to_string()),
        );
        runtimeParams.insert(
            "__operit_toolpkg_runtime_kind".to_string(),
            Value::String("sandbox".to_string()),
        );

        if let Some(runtime) = selection.toolpkg_runtime.as_ref() {
            runtimeParams.insert(
                "__operit_execution_context_key".to_string(),
                Value::String(format!("toolpkg_main:{}", runtime.containerPackageName)),
            );
            runtimeParams.insert(
                "__operit_toolpkg_subpackage_id".to_string(),
                Value::String(runtime.subpackageId.clone()),
            );
            runtimeParams.insert(
                "containerPackageName".to_string(),
                Value::String(runtime.containerPackageName.clone()),
            );
            runtimeParams.insert(
                "toolPkgId".to_string(),
                Value::String(runtime.containerPackageName.clone()),
            );
            runtimeParams.insert(
                "__operit_ui_package_name".to_string(),
                Value::String(runtime.containerPackageName.clone()),
            );
            runtimeParams.insert(
                "__operit_script_screen".to_string(),
                Value::String(runtime.entryPath.clone()),
            );
        } else {
            runtimeParams.remove("__operit_toolpkg_subpackage_id");
            runtimeParams.remove("containerPackageName");
            runtimeParams.remove("toolPkgId");
            runtimeParams.remove("__operit_ui_package_name");
            runtimeParams.remove("__operit_script_screen");
        }
        Ok(runtimeParams)
    }

    #[allow(non_snake_case)]
    fn convertToolParameters(
        &self,
        request: &JsPackageToolCallRequest,
        packageName: &str,
        selection: &JsPackageToolSelection,
    ) -> Result<BTreeMap<String, Value>, ToolParameterConversionException> {
        let definition = &selection.definition;
        let missingRequiredParameters = definition
            .parameters
            .iter()
            .filter(|parameter| {
                parameter.required && !request.parameters.contains_key(&parameter.name)
            })
            .map(|parameter| parameter.name.clone())
            .collect::<Vec<_>>();

        if !missingRequiredParameters.is_empty() {
            return Err(ToolParameterConversionException {
                message: format!(
                    "Missing required parameters: {}",
                    missingRequiredParameters.join(", ")
                ),
            });
        }

        let mut converted = BTreeMap::new();
        for (parameterName, rawValue) in &request.parameters {
            let parameterType = match definition
                .parameters
                .iter()
                .find(|item| item.name == *parameterName)
            {
                Some(parameter) => parameter.parameter_type.as_str(),
                None if matches!(
                    parameterName.as_str(),
                    "__operit_package_caller_name"
                        | "__operit_package_chat_id"
                        | "__operit_package_caller_participant_id"
                        | "__operit_package_caller_owner"
                ) =>
                {
                    "string"
                }
                None => {
                    return Err(ToolParameterConversionException {
                        message: format!(
                            "Unknown tool parameter: {}:{parameterName}",
                            request.tool_name
                        ),
                    })
                }
            };
            let value = self.convertToolParameterValue(
                &request.tool_name,
                parameterName,
                rawValue,
                &parameterType,
            )?;
            converted.insert(parameterName.clone(), value);
        }

        self.buildRuntimeParams(packageName, selection, converted)
            .map_err(|message| ToolParameterConversionException { message })
    }

    #[allow(non_snake_case)]
    fn convertToolParameterValue(
        &self,
        toolName: &str,
        parameterName: &str,
        rawValue: &str,
        parameterType: &str,
    ) -> Result<Value, ToolParameterConversionException> {
        let normalizedValue = rawValue.trim();
        match parameterType {
            "number" => normalizedValue
                .parse::<f64>()
                .map(|value| serde_json::json!(value))
                .map_err(|_| self.invalidParameterType(toolName, parameterName, parameterType)),
            "integer" => normalizedValue
                .parse::<i64>()
                .map(|value| serde_json::json!(value))
                .map_err(|_| self.invalidParameterType(toolName, parameterName, parameterType)),
            "boolean" => match normalizedValue.to_ascii_lowercase().as_str() {
                "true" | "1" => Ok(Value::Bool(true)),
                "false" | "0" => Ok(Value::Bool(false)),
                _ => Err(self.invalidParameterType(toolName, parameterName, parameterType)),
            },
            "array" | "object" => serde_json::from_str::<Value>(rawValue)
                .map_err(|_| self.invalidParameterType(toolName, parameterName, parameterType)),
            _ => Ok(Value::String(rawValue.to_string())),
        }
    }

    #[allow(non_snake_case)]
    fn invalidParameterType(
        &self,
        toolName: &str,
        parameterName: &str,
        expectedType: &str,
    ) -> ToolParameterConversionException {
        ToolParameterConversionException {
            message: format!(
                "Invalid parameter '{}' for tool '{}': expected {}",
                parameterName, toolName, expectedType
            ),
        }
    }

    /// Builds a successful SDK package execution result.
    fn success(toolName: &str, value: Option<String>) -> JsPackageToolCallResult {
        JsPackageToolCallResult {
            tool_name: toolName.to_string(),
            success: true,
            result: value.unwrap_or_else(|| "null".to_string()),
            error: None,
        }
    }

    /// Builds a failed SDK package execution result.
    fn failure(toolName: &str, message: String) -> JsPackageToolCallResult {
        JsPackageToolCallResult {
            tool_name: toolName.to_string(),
            success: false,
            result: String::new(),
            error: Some(message),
        }
    }

    /// Converts a JavaScript failure envelope into the SDK execution result.
    #[allow(non_snake_case)]
    /// Executes a package function by dotted package tool name.
    pub async fn executeScriptByName(
        &self,
        toolName: &str,
        params: BTreeMap<String, String>,
    ) -> JsExecutionResult<Option<String>> {
        let Some((packageName, functionName)) = Self::parseDotCall(toolName) else {
            return Err(JsExecutionError::invalid_request(format!(
                "Invalid tool name format: {toolName}. Expected format: packageName.functionName"
            )));
        };
        let selection = self
            .packageRuntime
            .select_tool(&packageName, &functionName)
            .map_err(JsExecutionError::invalid_request)?;
        let request = JsPackageToolCallRequest {
            tool_name: format!("{packageName}:{functionName}"),
            parameters: params,
        };
        let runtimeParams = self
            .convertToolParameters(&request, &packageName, &selection)
            .map_err(|error| JsExecutionError::invalid_request(error.message))?;
        self.withExecutionEngineForPackage(&selection, |engine| {
            engine.execute_script_function(
                &selection.definition.script,
                &functionName,
                &runtimeParams,
                &BTreeMap::new(),
                None,
                true,
                60,
            )
        })
        .await
    }

    #[allow(non_snake_case)]
    /// Executes a JavaScript package tool through the SDK execution contract.
    pub async fn executeScript(
        &self,
        request: &JsPackageToolCallRequest,
    ) -> JsPackageToolCallResult {
        let Some((packageName, functionName)) = Self::parsePackageToolName(&request.tool_name)
        else {
            return Self::failure(
                &request.tool_name,
                "Invalid tool name format. Expected 'packageName:toolName'".to_string(),
            );
        };

        let selection = match self.packageRuntime.select_tool(&packageName, &functionName) {
            Ok(selection) => selection,
            Err(error) => return Self::failure(&request.tool_name, error),
        };
        let runtimeParams = match self.convertToolParameters(request, &packageName, &selection) {
            Ok(value) => value,
            Err(error) => return Self::failure(&request.tool_name, error.message),
        };

        let result = self
            .withExecutionEngineForPackage(&selection, |engine| {
                engine.execute_script_function(
                    &selection.definition.script,
                    &functionName,
                    &runtimeParams,
                    &BTreeMap::new(),
                    None,
                    true,
                    60,
                )
            })
            .await;
        match result {
            Ok(value) => Self::success(&request.tool_name, value),
            Err(error) => Self::failure(&request.tool_name, error.message),
        }
    }

    /// Releases JavaScript tool manager resources.
    pub fn destroy(&self) {}
}

impl JsPackageExecutor for JsToolManager {
    /// Executes one package tool through this manager's bound runtime context.
    fn execute_package_tool(
        &self,
        request: &JsPackageToolCallRequest,
    ) -> JsExecutionCompletion<JsPackageToolCallResult> {
        let manager = self.clone();
        let request = request.clone();
        Box::pin(async move { manager.executeScript(&request).await })
    }
}

#[cfg(test)]
mod tests {
    use super::JsToolManager;
    use crate::javascript::JsEngine::JsEngine;
    use crate::javascript::TestJsToolsHost::{expect_js_output, register_test_runtime_storage};
    use operit_host_api::{
        FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
        GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult,
    };
    use operit_plugin_sdk::javascript::{
        JsExecutionEngine, JsExecutionHost, JsPackageRuntime, JsPackageToolSelection, JsToolCallRequest, JsToolCallResult,
        JsToolNameResolutionRequest, JsToolPkgIpcCompletion, JsToolPkgIpcRequest,
        JsToolPkgResourceRequest, JsToolPkgWasmRequest, JsToolPkgWasmResult, ToolPkgConfigScope,
        ToolPkgExecutionContext,
    };
    use operit_plugin_sdk::package::{PackageTool, ToolPackage};
    use operit_plugin_sdk::toolpkg::ToolPkgLoader::ToolPkgLoader;
    use operit_plugin_sdk::toolpkg::ToolPkgManager::{
        ToolPkgAssetSource, ToolPkgExecutionEngineFactory,
    };
    use operit_plugin_sdk::toolpkg::ToolPkgParser::{
        ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgMarketOrigin, ToolPkgSourceType,
        ToolPkgSubpackageRuntime,
    };
    use operit_plugin_sdk::toolpkg::ToolPkgProtection;
    use operit_plugin_sdk::JsPackageLoader::JsPackageLoader;
    use operit_plugin_sdk::PackageManager::{PackageStateResolver, PluginPackageManager};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::io::{Cursor, Read, Write};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Copy)]
    struct TestJsExecutionHost;

    crate::impl_rejecting_js_tools_host!(TestJsExecutionHost);

    impl JsExecutionHost for TestJsExecutionHost {
        /// Retains the explicit manager test host when an engine supplies a real package context.
        fn for_toolpkg_execution_context(
            &self,
            context: &operit_plugin_sdk::javascript::ToolPkgExecutionContext,
        ) -> Result<Arc<dyn JsExecutionHost>, String> {
            if context.container_package_name.trim().is_empty()
                || context.context_key.trim().is_empty()
            {
                return Err("Invalid manager test execution context".to_string());
            }
            Ok(Arc::new(*self))
        }

        /// Returns an empty catalog for tests that do not install runtime tools.
        fn get_tool_catalog(&self) -> Result<Value, String> {
            Ok(serde_json::json!({ "tools": [] }))
        }

        /// Rejects unexpected host tool calls from package execution tests.
        fn execute_tool_call(
            &self,
            request: JsToolCallRequest,
        ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<JsToolCallResult> {
            Box::pin(async move {
                JsToolCallResult {
                    success: false,
                    error: Some(format!(
                        "Unexpected host tool call: {}",
                        request.qualified_tool_name()
                    )),
                    ..JsToolCallResult::default()
                }
            })
        }

        /// Returns the package language used by manager tests.
        fn package_language(&self) -> Result<String, String> {
            Ok("en".to_string())
        }

        /// Reports no environment value in manager tests.
        fn read_environment_variable(&self, _key: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        /// Ignores environment writes in manager tests.
        fn write_environment_variable(&self, _key: &str, _value: &str) -> Result<(), String> {
            Ok(())
        }

        /// Rejects plugin configuration access in manager tests.
        fn plugin_config_dir(&self, _plugin_id: &str) -> Result<String, String> {
            Err("Plugin configuration is not part of this test".to_string())
        }

        /// Resolves scoped configuration through the explicit test contract.
        fn scoped_plugin_config_dir(
            &self,
            _owner_id: &str,
            plugin_id: &str,
        ) -> Result<String, String> {
            self.plugin_config_dir(plugin_id)
        }

        /// Rejects configuration creation in manager fixtures that do not provide storage.
        fn registration_plugin_config_dir(
            &self,
            _owner_id: &str,
            _plugin_id: &str,
            _scope: ToolPkgConfigScope,
        ) -> Result<String, String> {
            Err("Plugin configuration is not part of this test".to_string())
        }

        /// Rejects local data access in manager fixtures that do not provide storage.
        fn plugin_local_data_dir(&self, _owner_id: &str) -> Result<String, String> {
            Err("Local plugin data is not part of this test".to_string())
        }

        /// Rejects registration-local data access in manager fixtures that do not provide storage.
        fn registration_plugin_local_data_dir(&self, _owner_id: &str) -> Result<String, String> {
            Err("Local plugin data is not part of this test".to_string())
        }

        /// Rejects shared data access in manager fixtures without installed storage.
        fn plugin_space_data_dir(&self, _owner_id: &str) -> Result<String, String> {
            Err("Shared plugin data is not part of this test".to_string())
        }

        /// Rejects registration-shared data access in manager fixtures without installed storage.
        fn registration_plugin_space_data_dir(&self, _owner_id: &str) -> Result<String, String> {
            Err("Shared plugin data is not part of this test".to_string())
        }

        /// Rejects ToolPkg text resource access in manager tests.
        fn read_toolpkg_text_resource(
            &self,
            _package_name_or_subpackage_id: &str,
            _resource_path: &str,
        ) -> Result<Option<String>, String> {
            Err("ToolPkg text resources are not part of this test".to_string())
        }

        /// Rejects ToolPkg resource materialization in manager tests.
        fn materialize_toolpkg_resource(
            &self,
            _request: JsToolPkgResourceRequest,
        ) -> Result<String, String> {
            Err("ToolPkg resources are not part of this test".to_string())
        }

        /// Rejects ToolPkg WASM calls in manager tests.
        fn call_toolpkg_wasm(
            &self,
            _request: JsToolPkgWasmRequest,
        ) -> Result<JsToolPkgWasmResult, String> {
            Err("ToolPkg WASM is not part of this test".to_string())
        }

        /// Rejects Compose DSL controller commands in manager tests.
        fn handle_compose_webview_controller_command(
            &self,
            _payload: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            Err("Compose DSL WebView control is not part of this test".to_string())
        }

        /// Rejects Compose DSL file-picker requests in manager tests.
        fn open_compose_file_picker(
            &self,
            _payload: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            Err("Compose DSL file picking is not part of this test".to_string())
        }

        /// Reports no imported packages in manager tests.
        fn is_package_imported(&self, _package_name: &str) -> Result<bool, String> {
            Ok(false)
        }

        /// Rejects package import in manager tests.
        fn import_package(&self, _package_name: &str) -> Result<String, String> {
            Err("Package import is not part of this test".to_string())
        }

        /// Rejects package removal in manager tests.
        fn remove_package(&self, _package_name: &str) -> Result<String, String> {
            Err("Package removal is not part of this test".to_string())
        }

        /// Rejects package activation in manager tests.
        fn use_package(&self, _package_name: &str) -> Result<String, String> {
            Err("Package activation is not part of this test".to_string())
        }

        /// Returns the empty imported package list used by manager tests.
        fn list_imported_packages(&self) -> Result<Vec<String>, String> {
            Ok(Vec::new())
        }

        /// Returns the requested tool name in manager tests.
        fn resolve_tool_name(
            &self,
            request: JsToolNameResolutionRequest,
        ) -> Result<String, String> {
            Ok(request.tool_name)
        }

        /// Rejects ToolPkg IPC in manager tests.
        fn invoke_toolpkg_ipc_async(
            &self,
            _request: JsToolPkgIpcRequest,
            completion: JsToolPkgIpcCompletion,
        ) -> Result<(), String> {
            completion(Err("ToolPkg IPC is not part of this test".to_string()));
            Ok(())
        }

        /// Completes one JavaScript runtime turn immediately in manager tests.
        fn wait_for_javascript_runtime_turn(&self) -> operit_host_api::HostRuntimeTurnFuture {
            Box::pin(async { Ok(()) })
        }
    }

    #[derive(Clone, Copy)]
    struct TestExecutionEngineFactory;

    impl ToolPkgExecutionEngineFactory for TestExecutionEngineFactory {
        /// Creates a generic isolated JavaScript engine for manager tests.
        #[allow(non_snake_case)]
        fn createExecutionEngine(&self) -> Result<Arc<dyn JsExecutionEngine>, String> {
            JsEngine::new(Arc::new(TestJsExecutionHost)).map(|engine| Arc::new(engine) as Arc<dyn JsExecutionEngine>).map_err(|error| error.to_string())
        }

        /// Creates a ToolPkg JavaScript engine with its bound package context.
        #[allow(non_snake_case)]
        fn createToolPkgExecutionEngine(
            &self,
            context: ToolPkgExecutionContext,
        ) -> Result<Arc<dyn JsExecutionEngine>, String> {
            JsEngine::new_toolpkg_execution_engine(
                Arc::new(TestJsExecutionHost),
                context,
            ).map(|engine| Arc::new(engine) as Arc<dyn JsExecutionEngine>).map_err(|error| error.to_string())
        }
    }

    #[derive(Clone, Copy)]
    struct TestToolPkgAssetSource;

    impl ToolPkgAssetSource for TestToolPkgAssetSource {
        /// Returns no embedded assets because tests register packages directly.
        #[allow(non_snake_case)]
        fn toolPkgAssetBytes(&self, _assetName: &str) -> Option<Vec<u8>> {
            None
        }
    }

    #[derive(Clone, Copy)]
    struct RejectingFileSystemHost;

    impl RejectingFileSystemHost {
        /// Builds the unsupported operation result used by this file-system host.
        fn unsupported<T>() -> HostResult<T> {
            Err(HostError::new(
                "File-system host is not part of JsToolManager tests",
            ))
        }
    }

    impl FileSystemHost for RejectingFileSystemHost {
        /// Returns the test host label.
        fn envLabel(&self) -> &str {
            "test"
        }

        /// Returns the test environment descriptor.
        fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
            HostEnvironmentDescriptor::linux()
        }

        /// Rejects path validation.
        fn validatePath(&self, _path: &str, _param_name: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects directory listing.
        fn listFiles(&self, _path: &str) -> HostResult<Vec<FileEntry>> {
            Self::unsupported()
        }

        /// Rejects text reads.
        fn readFile(&self, _path: &str) -> HostResult<String> {
            Self::unsupported()
        }

        /// Rejects bounded text reads.
        fn readFileWithLimit(&self, _path: &str, _max_bytes: usize) -> HostResult<String> {
            Self::unsupported()
        }

        /// Rejects byte reads.
        fn readFileBytes(&self, _path: &str) -> HostResult<Vec<u8>> {
            Self::unsupported()
        }

        /// Rejects text writes.
        fn writeFile(&self, _path: &str, _content: &str, _append: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects byte writes.
        fn writeFileBytes(&self, _path: &str, _content: &[u8]) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects deletion.
        fn deleteFile(&self, _path: &str, _recursive: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects file existence checks.
        fn fileExists(&self, _path: &str) -> HostResult<FileExistence> {
            Self::unsupported()
        }

        /// Rejects moves.
        fn moveFile(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects copies.
        fn copyFile(&self, _source: &str, _destination: &str, _recursive: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects directory creation.
        fn makeDirectory(&self, _path: &str, _create_parents: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects file searching.
        fn findFiles(&self, _request: FindFilesRequest) -> HostResult<Vec<String>> {
            Self::unsupported()
        }

        /// Rejects file metadata reads.
        fn fileInfo(&self, _path: &str) -> HostResult<FileInfo> {
            Self::unsupported()
        }

        /// Rejects code searches.
        fn grepCode(&self, _request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
            Self::unsupported()
        }

        /// Rejects archive creation.
        fn zipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects archive extraction.
        fn unzipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects host file opening.
        fn openFile(&self, _path: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects host file sharing.
        fn shareFile(&self, _path: &str, _title: &str) -> HostResult<()> {
            Self::unsupported()
        }
    }

    #[derive(Clone)]
    struct DownloadedToolPkgFileSystemHost {
        source_path: String,
        source_bytes: Vec<u8>,
    }

    impl DownloadedToolPkgFileSystemHost {
        /// Creates a host that exposes one downloaded ToolPkg file by exact path.
        fn new(source_path: &str, source_bytes: Vec<u8>) -> Self {
            Self {
                source_path: source_path.to_string(),
                source_bytes,
            }
        }

        /// Builds the unsupported operation result used by this file-system host.
        fn unsupported<T>() -> HostResult<T> {
            Err(HostError::new(
                "Only downloaded ToolPkg byte reads are part of this test",
            ))
        }
    }

    impl FileSystemHost for DownloadedToolPkgFileSystemHost {
        /// Returns the test host label.
        fn envLabel(&self) -> &str {
            "downloaded-toolpkg-test"
        }

        /// Returns the test environment descriptor.
        fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
            HostEnvironmentDescriptor::linux()
        }

        /// Accepts the exact downloaded ToolPkg path.
        fn validatePath(&self, path: &str, _param_name: &str) -> HostResult<()> {
            if path == self.source_path {
                Ok(())
            } else {
                Self::unsupported()
            }
        }

        /// Rejects directory listing.
        fn listFiles(&self, _path: &str) -> HostResult<Vec<FileEntry>> {
            Self::unsupported()
        }

        /// Rejects text reads.
        fn readFile(&self, _path: &str) -> HostResult<String> {
            Self::unsupported()
        }

        /// Rejects bounded text reads.
        fn readFileWithLimit(&self, _path: &str, _max_bytes: usize) -> HostResult<String> {
            Self::unsupported()
        }

        /// Returns the exact downloaded ToolPkg bytes.
        fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            if path == self.source_path {
                Ok(self.source_bytes.clone())
            } else {
                Self::unsupported()
            }
        }

        /// Rejects text writes.
        fn writeFile(&self, _path: &str, _content: &str, _append: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects byte writes.
        fn writeFileBytes(&self, _path: &str, _content: &[u8]) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects deletion.
        fn deleteFile(&self, _path: &str, _recursive: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects file existence checks.
        fn fileExists(&self, _path: &str) -> HostResult<FileExistence> {
            Self::unsupported()
        }

        /// Rejects moves.
        fn moveFile(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects copies.
        fn copyFile(&self, _source: &str, _destination: &str, _recursive: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects directory creation.
        fn makeDirectory(&self, _path: &str, _create_parents: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects file searching.
        fn findFiles(&self, _request: FindFilesRequest) -> HostResult<Vec<String>> {
            Self::unsupported()
        }

        /// Rejects file metadata reads.
        fn fileInfo(&self, _path: &str) -> HostResult<FileInfo> {
            Self::unsupported()
        }

        /// Rejects code searches.
        fn grepCode(&self, _request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
            Self::unsupported()
        }

        /// Rejects archive creation.
        fn zipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects archive extraction.
        fn unzipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects host file opening.
        fn openFile(&self, _path: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects host file sharing.
        fn shareFile(&self, _path: &str, _title: &str) -> HostResult<()> {
            Self::unsupported()
        }
    }

    #[derive(Clone)]
    struct DownloadedStandaloneJsFileSystemHost {
        source_path: String,
        source_bytes: Vec<u8>,
    }

    impl DownloadedStandaloneJsFileSystemHost {
        /// Creates a host that exposes one downloaded standalone JavaScript file by exact path.
        fn new(source_path: &str, source_bytes: Vec<u8>) -> Self {
            Self {
                source_path: source_path.to_string(),
                source_bytes,
            }
        }

        /// Builds the unsupported operation result used by this file-system host.
        fn unsupported<T>() -> HostResult<T> {
            Err(HostError::new(
                "Only downloaded standalone JavaScript byte reads are part of this test",
            ))
        }
    }

    impl FileSystemHost for DownloadedStandaloneJsFileSystemHost {
        /// Returns the test host label.
        fn envLabel(&self) -> &str {
            "downloaded-standalone-js-test"
        }

        /// Returns the test environment descriptor.
        fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
            HostEnvironmentDescriptor::linux()
        }

        /// Accepts the exact downloaded standalone JavaScript path.
        fn validatePath(&self, path: &str, _param_name: &str) -> HostResult<()> {
            if path == self.source_path {
                Ok(())
            } else {
                Self::unsupported()
            }
        }

        /// Rejects directory listing.
        fn listFiles(&self, _path: &str) -> HostResult<Vec<FileEntry>> {
            Self::unsupported()
        }

        /// Rejects text reads.
        fn readFile(&self, _path: &str) -> HostResult<String> {
            Self::unsupported()
        }

        /// Rejects bounded text reads.
        fn readFileWithLimit(&self, _path: &str, _max_bytes: usize) -> HostResult<String> {
            Self::unsupported()
        }

        /// Returns the exact downloaded standalone JavaScript bytes.
        fn readFileBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            if path == self.source_path {
                Ok(self.source_bytes.clone())
            } else {
                Self::unsupported()
            }
        }

        /// Rejects text writes.
        fn writeFile(&self, _path: &str, _content: &str, _append: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects byte writes.
        fn writeFileBytes(&self, _path: &str, _content: &[u8]) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects deletion.
        fn deleteFile(&self, _path: &str, _recursive: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects file existence checks.
        fn fileExists(&self, _path: &str) -> HostResult<FileExistence> {
            Self::unsupported()
        }

        /// Rejects moves.
        fn moveFile(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects copies.
        fn copyFile(&self, _source: &str, _destination: &str, _recursive: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects directory creation.
        fn makeDirectory(&self, _path: &str, _create_parents: bool) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects file searching.
        fn findFiles(&self, _request: FindFilesRequest) -> HostResult<Vec<String>> {
            Self::unsupported()
        }

        /// Rejects file metadata reads.
        fn fileInfo(&self, _path: &str) -> HostResult<FileInfo> {
            Self::unsupported()
        }

        /// Rejects code searches.
        fn grepCode(&self, _request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
            Self::unsupported()
        }

        /// Rejects archive creation.
        fn zipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects archive extraction.
        fn unzipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects host file opening.
        fn openFile(&self, _path: &str) -> HostResult<()> {
            Self::unsupported()
        }

        /// Rejects host file sharing.
        fn shareFile(&self, _path: &str, _title: &str) -> HostResult<()> {
            Self::unsupported()
        }
    }

    /// Returns whether a byte slice contains one exact byte sequence.
    fn contains_bytes(bytes: &[u8], needle: &[u8]) -> bool {
        bytes
            .windows(needle.len())
            .any(|candidate| candidate == needle)
    }

    /// Reads one ZIP entry into memory for protection assertions.
    fn read_zip_entry_bytes(archive_bytes: &[u8], entry_name: &str) -> Vec<u8> {
        let cursor = Cursor::new(archive_bytes);
        let mut archive = zip::ZipArchive::new(cursor).expect("ToolPkg bytes should be a zip");
        let mut entry = archive
            .by_name(entry_name)
            .expect("expected ToolPkg entry should exist");
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .expect("expected ToolPkg entry should read");
        bytes
    }

    /// Builds a ToolPkg archive before publish-time AST minification.
    fn build_toolpkg_bytes() -> Vec<u8> {
        let manifest = br#"{
  "toolpkg_id": "market_flow_toolpkg",
  "version": "1.0.0",
  "main": "dist/main.js",
  "display_name": "Market Flow ToolPkg",
  "description": "Protected marketplace flow test",
  "author": ["Operit"],
  "subpackages": [
    {
      "id": "market_flow_sub",
      "entry": "dist/sub.js",
      "display_name": "Market Flow Sub",
      "description": "Protected subpackage"
    }
  ],
  "resources": [
    {
      "key": "payload",
      "path": "assets/payload.txt",
      "mime": "text/plain"
    }
  ]
}"#;
        let main_script = br#""use strict";exports.registerToolPkg=function(){return true};"#;
        let subpackage_script = br#"/* METADATA
{
  name: market_flow_sub_source
  displayName: Market Flow Source
  tools: [
    {
      name: inspect
      description: Inspect protected package flow
      parameters: [
        { name: text, description: Text to echo, type: string, required: true }
      ]
    }
  ]
}
*/"use strict";exports.inspect=function(t){return"protected-flow:"+t.text+"|"+t.__operit_toolpkg_subpackage_id+"|"+t.__operit_script_screen};"#;
        let asset_payload = b"asset-secret-value";

        let mut source_bytes = Vec::new();
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut source_bytes));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file("manifest.json", options)
            .expect("manifest entry should start");
        writer
            .write_all(manifest)
            .expect("manifest entry should write");
        writer
            .start_file("dist/main.js", options)
            .expect("main entry should start");
        writer
            .write_all(main_script)
            .expect("main entry should write");
        writer
            .start_file("dist/sub.js", options)
            .expect("subpackage entry should start");
        writer
            .write_all(subpackage_script)
            .expect("subpackage entry should write");
        writer
            .start_file("assets/payload.txt", options)
            .expect("asset entry should start");
        writer
            .write_all(asset_payload)
            .expect("asset entry should write");
        writer.finish().expect("ToolPkg zip should finish");
        source_bytes
    }

    /// Builds a standalone JavaScript package before publish-time AST minification.
    fn build_standalone_js_bytes() -> Vec<u8> {
        br#"/* METADATA
{
  name: standalone_market_flow
  displayName: Standalone Market Flow
  tools: [
    {
      name: inspect
      description: Inspect protected standalone flow
      parameters: [
        { name: text, description: Text to echo, type: string, required: true }
      ]
    }
  ]
}
*/"use strict";exports.inspect=function(t){return"standalone-protected:"+t.text};"#
            .to_vec()
    }

    /// Loads a downloaded ToolPkg through the standard external-file path.
    fn load_downloaded_toolpkg(downloaded_bytes: Vec<u8>) -> ToolPkgLoadResult {
        let source_path = "/marketplace/downloaded/market_flow_toolpkg.toolpkg";
        let host = DownloadedToolPkgFileSystemHost::new(source_path, downloaded_bytes);
        let registration_engine = JsEngine::new(Arc::new(TestJsExecutionHost)).expect("registration fixture must initialize");
        let load_errors = Mutex::new(Vec::<String>::new());
        let load_result = ToolPkgLoader::loadToolPkgFromExternalFile(
            &host,
            source_path,
            ToolPkgConfigScope::Device,
            &registration_engine,
            |package_name, error| {
                load_errors
                    .lock()
                    .expect("load error mutex poisoned")
                    .push(format!("{package_name}: {error}"));
            },
        )
        .expect("installed market ToolPkg should load");
        let load_errors = load_errors.lock().expect("load error mutex poisoned");
        assert!(
            load_errors.is_empty(),
            "downloaded external ToolPkg reported load errors: {load_errors:?}"
        );
        load_result
    }

    /// Loads downloaded standalone JavaScript bytes through the SDK package loader.
    fn load_downloaded_standalone_js(downloaded_bytes: Vec<u8>) -> ToolPackage {
        let source_path = "/marketplace/downloaded/standalone_market_flow.js";
        let host = DownloadedStandaloneJsFileSystemHost::new(source_path, downloaded_bytes);
        JsPackageLoader::load_from_file(&host, source_path)
            .expect("downloaded external standalone JavaScript should load")
    }

    #[derive(Clone, Copy)]
    struct TestPackageStateResolver;

    impl PackageStateResolver for TestPackageStateResolver {
        /// Leaves package state unselected for execution contract tests.
        #[allow(non_snake_case)]
        fn resolvePackageStateId(&self, _package: &ToolPackage) -> Option<String> {
            None
        }
    }

    struct TestPackageRuntime {
        package_manager: Mutex<PluginPackageManager>,
    }

    impl TestPackageRuntime {
        /// Creates a pure SDK package runtime from one ToolPkg load result.
        fn new(load_result: ToolPkgLoadResult) -> Arc<Self> {
            let mut package_manager = PluginPackageManager::new(
                Arc::new(TestExecutionEngineFactory),
                Arc::new(TestToolPkgAssetSource),
                Arc::new(RejectingFileSystemHost),
                Arc::new(TestPackageStateResolver),
            );
            assert!(package_manager.registerToolPkg(load_result));
            Arc::new(Self {
                package_manager: Mutex::new(package_manager),
            })
        }

        /// Creates a pure SDK package runtime from one standalone JavaScript package.
        fn new_with_package(package: ToolPackage) -> Arc<Self> {
            let mut package_manager = PluginPackageManager::new(
                Arc::new(TestExecutionEngineFactory),
                Arc::new(TestToolPkgAssetSource),
                Arc::new(RejectingFileSystemHost),
                Arc::new(TestPackageStateResolver),
            );
            package_manager.registerPackage(package);
            Arc::new(Self {
                package_manager: Mutex::new(package_manager),
            })
        }
    }

    impl JsPackageRuntime for TestPackageRuntime {
        /// Returns the language used by package execution tests.
        fn package_language(&self) -> Result<String, String> {
            Ok("en".to_string())
        }

        /// Selects a real test declaration and its SDK runtime metadata under one registry lock.
        fn select_tool(
            &self,
            package_name: &str,
            function_name: &str,
        ) -> Result<JsPackageToolSelection, String> {
            let manager = self
                .package_manager
                .lock()
                .map_err(|error| error.to_string())?;
            let package = manager
                .package(package_name)
                .ok_or_else(|| format!("Package not registered: {package_name}"))?;
            Ok(JsPackageToolSelection {
                definition: operit_plugin_sdk::javascript::select_registered_package_tool(
                    package,
                    function_name,
                )?,
                active_state_id: manager.activePackageStateId(package_name),
                toolpkg_runtime: manager
                    .toolPkgManager()
                    .resolveToolPkgSubpackageRuntimeInternal(package_name),
            })
        }

        /// Returns the shared execution engine for one explicitly owned ToolPkg context.
        fn toolpkg_execution_engine(
            &self,
            context_key: &str,
            container_package_name: &str,
        ) -> Result<Arc<dyn JsExecutionEngine>, String> {
            self.package_manager
                .lock()
                .expect("test package manager mutex poisoned")
                .toolPkgManager()
                .getToolPkgExecutionEngine(context_key, container_package_name)
        }
    }

    fn toolpkg_manager(script: &str) -> (JsToolManager, Arc<TestPackageRuntime>) {
        register_test_runtime_storage("js-tool-manager-tests");
        let load_result = ToolPkgLoadResult {
            containerPackage: ToolPackage {
                name: "test_toolpkg".to_string(),
                ..ToolPackage::default()
            },
            subpackagePackages: vec![ToolPackage {
                name: "test_toolpkg_sub".to_string(),
                tools: vec![PackageTool {
                    name: "inspect".to_string(),
                    script: script.to_string(),
                    ..PackageTool::default()
                }],
                ..ToolPackage::default()
            }],
            containerRuntime: ToolPkgContainerRuntime {
                packageName: "test_toolpkg".to_string(),
                mainEntry: "dist/main.js".to_string(),
                sourceType: ToolPkgSourceType::EXTERNAL,
                sourcePath: ".".to_string(),
                subpackages: vec![ToolPkgSubpackageRuntime {
                    packageName: "test_toolpkg_sub".to_string(),
                    containerPackageName: "test_toolpkg".to_string(),
                    subpackageId: "sub".to_string(),
                    entryPath: "dist/sub.js".to_string(),
                    ..ToolPkgSubpackageRuntime::default()
                }],
                ..ToolPkgContainerRuntime::default()
            },
            marketOrigin: None,
        };
        let package_runtime = TestPackageRuntime::new(load_result);
        let manager = JsToolManager::new(
            package_runtime.clone(),
            Arc::new(TestExecutionEngineFactory),
        );
        (manager, package_runtime)
    }

    #[tokio::test(flavor = "current_thread")]
    /// Verifies a minified ToolPkg downloads through the standard external path and executes.
    async fn minified_toolpkg_download_executes_registered_tool() {
        register_test_runtime_storage("js-tool-manager-marketplace-flow");
        let upload_source_bytes = build_toolpkg_bytes();
        let upload_subpackage_bytes = read_zip_entry_bytes(&upload_source_bytes, "dist/sub.js");
        let upload_asset_bytes = read_zip_entry_bytes(&upload_source_bytes, "assets/payload.txt");
        assert!(contains_bytes(&upload_subpackage_bytes, b"protected-flow:"));
        assert!(contains_bytes(&upload_asset_bytes, b"asset-secret-value"));

        let downloaded_bytes = ToolPkgProtection::protectArtifactBytes(&upload_source_bytes, true)
            .expect("ToolPkg upload bytes should be minified");
        assert!(downloaded_bytes.starts_with(b"PK"));
        assert!(!ToolPkgProtection::isMarketArchive(&downloaded_bytes));

        let manifest_bytes = read_zip_entry_bytes(&downloaded_bytes, "manifest.json");
        let main_bytes = read_zip_entry_bytes(&downloaded_bytes, "dist/main.js");
        let subpackage_bytes = read_zip_entry_bytes(&downloaded_bytes, "dist/sub.js");
        let asset_bytes = read_zip_entry_bytes(&downloaded_bytes, "assets/payload.txt");
        assert!(contains_bytes(&manifest_bytes, b"market_flow_toolpkg"));
        assert!(!contains_bytes(&manifest_bytes, b"market_only"));
        assert!(contains_bytes(
            &main_bytes,
            b"exports.registerToolPkg=function"
        ));
        assert!(subpackage_bytes.starts_with(b"/* METADATA"));
        assert!(contains_bytes(
            &subpackage_bytes,
            b"name: market_flow_sub_source"
        ));
        assert!(contains_bytes(
            &subpackage_bytes,
            b"exports.inspect=function(t){return\"protected-flow:\""
        ));
        assert_eq!(asset_bytes, b"asset-secret-value");

        let load_result = load_downloaded_toolpkg(downloaded_bytes);
        assert_eq!(
            load_result.containerRuntime.sourceType,
            ToolPkgSourceType::EXTERNAL
        );
        assert_eq!(
            load_result.containerRuntime.sourcePath,
            "/marketplace/downloaded/market_flow_toolpkg.toolpkg"
        );
        assert_eq!(
            load_result.marketOrigin,
            Some(ToolPkgMarketOrigin {
                market: "Operit".to_string(),
                toolpkgId: "market_flow_toolpkg".to_string(),
                version: "1.0.0".to_string(),
                author: vec!["Operit".to_string()],
            })
        );
        assert_eq!(load_result.subpackagePackages.len(), 1);
        assert_eq!(load_result.subpackagePackages[0].name, "market_flow_sub");
        assert_eq!(load_result.subpackagePackages[0].tools.len(), 1);
        assert_eq!(load_result.subpackagePackages[0].tools[0].name, "inspect");

        let package_runtime = TestPackageRuntime::new(load_result);
        let manager = JsToolManager::new(package_runtime, Arc::new(TestExecutionEngineFactory));
        let params = BTreeMap::from([("text".to_string(), "downloaded".to_string())]);
        let output = expect_js_output(
            manager
                .executeScriptByName("market_flow_sub.inspect", params)
                .await,
            "minified ToolPkg download execution",
        );

        assert_eq!(
            output,
            "\"protected-flow:downloaded|market_flow_sub|dist/sub.js\""
        );
    }

    #[tokio::test(flavor = "current_thread")]
    /// Verifies a minified standalone script downloads and executes without encryption.
    async fn minified_standalone_download_executes_registered_tool() {
        register_test_runtime_storage("js-tool-manager-standalone-marketplace-flow");
        let upload_source_bytes = build_standalone_js_bytes();
        assert!(contains_bytes(
            &upload_source_bytes,
            b"standalone-protected:"
        ));

        let downloaded_bytes = ToolPkgProtection::protectArtifactBytes(&upload_source_bytes, false)
            .expect("standalone upload bytes should be minified");
        assert!(!ToolPkgProtection::isProtected(&downloaded_bytes));
        let minified_script = String::from_utf8(downloaded_bytes.clone())
            .expect("minified standalone script should be UTF-8");
        assert!(minified_script.starts_with("/* METADATA"));
        assert!(minified_script.contains("name: standalone_market_flow"));
        assert!(
            minified_script.contains("exports.inspect=function(t){return\"standalone-protected:\"")
        );

        let package = load_downloaded_standalone_js(downloaded_bytes);
        assert_eq!(package.name, "standalone_market_flow");
        assert_eq!(package.tools.len(), 1);
        assert_eq!(package.tools[0].name, "inspect");
        assert!(package.tools[0].script.contains("standalone-protected:"));

        let package_runtime = TestPackageRuntime::new_with_package(package);
        let manager = JsToolManager::new(package_runtime, Arc::new(TestExecutionEngineFactory));
        let params = BTreeMap::from([("text".to_string(), "downloaded".to_string())]);
        let output = expect_js_output(
            manager
                .executeScriptByName("standalone_market_flow.inspect", params)
                .await,
            "minified standalone download execution",
        );

        assert_eq!(output, "\"standalone-protected:downloaded\"");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn subpackage_runtime_params_match_toolpkg_context() {
        let script = r#"
            exports.inspect = function(params) {
                return [
                    params.__operit_execution_context_key,
                    params.__operit_toolpkg_subpackage_id,
                    params.containerPackageName,
                    params.toolPkgId,
                    params.__operit_ui_package_name,
                    params.__operit_script_screen
                ].join('|');
            };
        "#;
        let (manager, _) = toolpkg_manager(script);

        let output = expect_js_output(
            manager
                .executeScriptByName("test_toolpkg_sub.inspect", BTreeMap::new())
                .await,
            "subpackage runtime params execution",
        );

        assert_eq!(
            output,
            "\"toolpkg_main:test_toolpkg|sub|test_toolpkg|test_toolpkg|test_toolpkg|dist/sub.js\""
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn subpackage_execution_uses_toolpkg_main_engine() {
        let script = r#"
            exports.inspect = function(_params) {
                return globalThis.__toolpkg_engine_marker;
            };
        "#;
        let (manager, package_runtime) = toolpkg_manager(script);
        let engine =
            package_runtime.toolpkg_execution_engine("toolpkg_main:test_toolpkg", "test_toolpkg").expect("fixture engine must initialize");
        let seed_script = r#"
            exports.seed = function(_params) {
                globalThis.__toolpkg_engine_marker = "same-engine";
                return "ok";
            };
        "#;
        let seed_params = BTreeMap::from([(
            "__operit_package_lang".to_string(),
            Value::String("en".to_string()),
        )]);
        let seed_output = engine
            .execute_script_function(
                seed_script,
                "seed",
                &seed_params,
                &BTreeMap::new(),
                None,
                true,
                60,
            )
            .await;

        assert_eq!(
            expect_js_output(seed_output, "ToolPkg main engine seed execution"),
            "\"ok\""
        );
        let output = expect_js_output(
            manager
                .executeScriptByName("test_toolpkg_sub.inspect", BTreeMap::new())
                .await,
            "subpackage shared-engine execution",
        );
        assert_eq!(output, "\"same-engine\"");
    }
}
