use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex};

use serde_json::Value;

use operit_plugin_sdk::execution_result::{JsExecutionError, JsExecutionResult};
use operit_plugin_sdk::javascript::{
    JsExecutionEngine, JsPackageExecutor, JsPackageRuntime, JsPackageToolCallRequest,
    JsPackageToolCallResult,
};
use operit_plugin_sdk::toolpkg::ToolPkgManager::ToolPkgExecutionEngineFactory;

#[derive(Clone)]
/// Executes JavaScript-backed package tools through reusable JS engines.
pub struct JsToolManager {
    packageRuntime: Arc<dyn JsPackageRuntime>,
    enginePool: Arc<(Mutex<Vec<Arc<dyn JsExecutionEngine>>>, Condvar)>,
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
        let engines = (0..MAX_CONCURRENT_ENGINES)
            .map(|_| executionEngineFactory.createToolPkgExecutionEngine())
            .collect::<Vec<_>>();
        Self {
            packageRuntime,
            enginePool: Arc::new((Mutex::new(engines), Condvar::new())),
        }
    }

    #[allow(non_snake_case)]
    fn withEngine<T>(&self, block: impl FnOnce(Arc<dyn JsExecutionEngine>) -> T) -> T {
        let (pool, available) = &*self.enginePool;
        let mut guard = pool
            .lock()
            .expect("JsToolManager engine pool mutex poisoned");
        while guard.is_empty() {
            guard = available
                .wait(guard)
                .expect("JsToolManager engine pool mutex poisoned");
        }
        let engine = guard
            .pop()
            .expect("JsToolManager engine pool must contain engine");
        drop(guard);
        let output = block(engine.clone());
        pool.lock()
            .expect("JsToolManager engine pool mutex poisoned")
            .push(engine);
        available.notify_one();
        output
    }

    #[allow(non_snake_case)]
    fn withExecutionEngineForPackage<T>(
        &self,
        packageName: &str,
        block: impl FnOnce(Arc<dyn JsExecutionEngine>) -> T,
    ) -> T {
        let toolPkgRuntime = self.packageRuntime.resolve_toolpkg_subpackage(packageName);
        if let Some(runtime) = toolPkgRuntime {
            let contextKey = format!("toolpkg_main:{}", runtime.containerPackageName);
            let engine = self.packageRuntime.toolpkg_execution_engine(&contextKey);
            return block(engine);
        }
        self.withEngine(block)
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
        params: BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, Value>, String> {
        let mut runtimeParams = params;
        let packageLanguage = self.packageRuntime.package_language()?;
        runtimeParams.insert(
            "__operit_package_lang".to_string(),
            Value::String(packageLanguage),
        );
        if let Some(stateId) = self.packageRuntime.active_package_state_id(packageName) {
            runtimeParams.insert("__operit_package_state".to_string(), Value::String(stateId));
        }

        for key in [
            "__operit_package_caller_name",
            "__operit_package_chat_id",
            "__operit_package_caller_card_id",
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

        if let Some(runtime) = self.packageRuntime.resolve_toolpkg_subpackage(packageName) {
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
                Value::String(runtime.containerPackageName),
            );
            runtimeParams.insert(
                "__operit_script_screen".to_string(),
                Value::String(runtime.entryPath),
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
        functionName: &str,
    ) -> Result<BTreeMap<String, Value>, ToolParameterConversionException> {
        let packageTools = self.packageRuntime.package(packageName);
        let toolDefinition = packageTools
            .as_ref()
            .and_then(|package| package.tools.iter().find(|item| item.name == functionName));

        let missingRequiredParameters = toolDefinition
            .map(|definition| {
                definition
                    .parameters
                    .iter()
                    .filter(|parameter| {
                        parameter.required && !request.parameters.contains_key(&parameter.name)
                    })
                    .map(|parameter| parameter.name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

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
            let parameterType = toolDefinition
                .and_then(|definition| {
                    definition
                        .parameters
                        .iter()
                        .find(|item| item.name == *parameterName)
                })
                .map(|item| item.parameter_type.to_ascii_lowercase())
                .unwrap_or_else(|| "string".to_string());
            let value = self.convertToolParameterValue(
                &request.tool_name,
                parameterName,
                rawValue,
                &parameterType,
            )?;
            converted.insert(parameterName.clone(), value);
        }

        self.buildRuntimeParams(packageName, converted)
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
    pub fn executeScriptByName(
        &self,
        toolName: &str,
        params: BTreeMap<String, String>,
    ) -> JsExecutionResult<Option<String>> {
        let Some((packageName, functionName)) = Self::parseDotCall(toolName) else {
            return Err(JsExecutionError::invalid_request(format!(
                "Invalid tool name format: {toolName}. Expected format: packageName.functionName"
            )));
        };
        let script = self
            .packageRuntime
            .package(&packageName)
            .and_then(|package| package.tools.first().map(|tool| tool.script.clone()));
        let Some(script) = script else {
            return Err(JsExecutionError::invalid_request(format!(
                "Package not found: {packageName}"
            )));
        };
        let params = params
            .into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect::<BTreeMap<_, _>>();
        let runtimeParams = match self.buildRuntimeParams(&packageName, params) {
            Ok(runtimeParams) => runtimeParams,
            Err(error) => return Err(JsExecutionError::runtime(error)),
        };
        self.withExecutionEngineForPackage(&packageName, |engine| {
            engine.execute_script_function(
                &script,
                &functionName,
                &runtimeParams,
                &BTreeMap::new(),
                None,
                true,
                60,
            )
        })
    }

    #[allow(non_snake_case)]
    /// Executes a JavaScript package tool through the SDK execution contract.
    pub fn executeScript(
        &self,
        script: &str,
        request: &JsPackageToolCallRequest,
    ) -> JsPackageToolCallResult {
        let Some((packageName, functionName)) = Self::parsePackageToolName(&request.tool_name)
        else {
            return Self::failure(
                &request.tool_name,
                "Invalid tool name format. Expected 'packageName:toolName'".to_string(),
            );
        };

        let runtimeParams = match self.convertToolParameters(request, &packageName, &functionName) {
            Ok(value) => value,
            Err(error) => return Self::failure(&request.tool_name, error.message),
        };

        let result = self.withExecutionEngineForPackage(&packageName, |engine| {
            engine.execute_script_function(
                script,
                &functionName,
                &runtimeParams,
                &BTreeMap::new(),
                None,
                true,
                60,
            )
        });
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
        script: &str,
        request: &JsPackageToolCallRequest,
    ) -> JsPackageToolCallResult {
        self.executeScript(script, request)
    }
}

#[cfg(test)]
mod tests {
    use super::JsToolManager;
    use crate::javascript::JsEngine::JsEngine;
    use operit_plugin_sdk::javascript::{
        JsExecutionEngine, JsExecutionHost, JsPackageRuntime, JsToolCallRequest, JsToolCallResult,
        JsToolNameResolutionRequest, JsToolPkgIpcRequest, JsToolPkgResourceRequest,
    };
    use operit_plugin_sdk::package::{PackageTool, ToolPackage};
    use operit_plugin_sdk::toolpkg::ToolPkgManager::{
        ToolPkgAssetSource, ToolPkgExecutionEngineFactory,
    };
    use operit_plugin_sdk::toolpkg::ToolPkgParser::{
        ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType, ToolPkgSubpackageRuntime,
    };
    use operit_plugin_sdk::PackageManager::{PackageStateResolver, PluginPackageManager};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Copy)]
    struct TestJsExecutionHost;

    impl JsExecutionHost for TestJsExecutionHost {
        /// Rejects unexpected host tool calls from package execution tests.
        fn execute_tool_call(&self, request: JsToolCallRequest) -> JsToolCallResult {
            JsToolCallResult {
                success: false,
                error: Some(format!(
                    "Unexpected host tool call: {}",
                    request.qualified_tool_name()
                )),
                ..JsToolCallResult::default()
            }
        }

        /// Returns the package language used by manager tests.
        fn package_language(&self) -> Result<String, String> {
            Ok("en".to_string())
        }

        /// Reports no environment value in manager tests.
        fn read_environment_variable(&self, _key: &str) -> Result<Option<String>, String> {
            Ok(None)
        }

        /// Rejects plugin configuration access in manager tests.
        fn plugin_config_dir(&self, _plugin_id: &str) -> Result<String, String> {
            Err("Plugin configuration is not part of this test".to_string())
        }

        /// Rejects ToolPkg text resource access in manager tests.
        fn read_toolpkg_text_resource(
            &self,
            _package_name_or_subpackage_id: &str,
            _resource_path: &str,
        ) -> Result<String, String> {
            Err("ToolPkg text resources are not part of this test".to_string())
        }

        /// Rejects ToolPkg resource materialization in manager tests.
        fn materialize_toolpkg_resource(
            &self,
            _request: JsToolPkgResourceRequest,
        ) -> Result<String, String> {
            Err("ToolPkg resources are not part of this test".to_string())
        }

        /// Rejects Compose DSL controller commands in manager tests.
        fn handle_compose_webview_controller_command(
            &self,
            _payload_json: &str,
        ) -> Result<String, String> {
            Err("Compose DSL WebView control is not part of this test".to_string())
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
        fn invoke_toolpkg_ipc(&self, _request: JsToolPkgIpcRequest) -> Result<Value, String> {
            Err("ToolPkg IPC is not part of this test".to_string())
        }
    }

    #[derive(Clone, Copy)]
    struct TestExecutionEngineFactory;

    impl ToolPkgExecutionEngineFactory for TestExecutionEngineFactory {
        /// Creates an isolated JavaScript engine for one test context.
        #[allow(non_snake_case)]
        fn createToolPkgExecutionEngine(&self) -> Arc<dyn JsExecutionEngine> {
            Arc::new(JsEngine::new(Arc::new(TestJsExecutionHost)))
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
        fn new(loadResult: ToolPkgLoadResult) -> Arc<Self> {
            let mut package_manager = PluginPackageManager::new(
                Arc::new(TestExecutionEngineFactory),
                Arc::new(TestToolPkgAssetSource),
                Arc::new(TestPackageStateResolver),
            );
            assert!(package_manager.registerToolPkg(loadResult));
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

        /// Returns one package definition from the SDK package manager.
        fn package(&self, package_name: &str) -> Option<ToolPackage> {
            self.package_manager
                .lock()
                .expect("test package manager mutex poisoned")
                .package(package_name)
        }

        /// Returns the selected package state from the SDK package manager.
        fn active_package_state_id(&self, package_name: &str) -> Option<String> {
            self.package_manager
                .lock()
                .expect("test package manager mutex poisoned")
                .activePackageStateId(package_name)
        }

        /// Resolves one ToolPkg subpackage from the SDK package manager.
        fn resolve_toolpkg_subpackage(
            &self,
            package_name: &str,
        ) -> Option<ToolPkgSubpackageRuntime> {
            self.package_manager
                .lock()
                .expect("test package manager mutex poisoned")
                .toolPkgManager()
                .resolveToolPkgSubpackageRuntimeInternal(package_name)
        }

        /// Returns the shared execution engine for one ToolPkg context.
        fn toolpkg_execution_engine(&self, context_key: &str) -> Arc<dyn JsExecutionEngine> {
            self.package_manager
                .lock()
                .expect("test package manager mutex poisoned")
                .toolPkgManager()
                .getToolPkgExecutionEngine(context_key)
        }
    }

    fn toolpkg_manager(script: &str) -> (JsToolManager, Arc<TestPackageRuntime>) {
        let loadResult = ToolPkgLoadResult {
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
        };
        let packageRuntime = TestPackageRuntime::new(loadResult);
        let manager =
            JsToolManager::new(packageRuntime.clone(), Arc::new(TestExecutionEngineFactory));
        (manager, packageRuntime)
    }

    #[test]
    fn subpackage_runtime_params_match_toolpkg_context() {
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

        let output = manager.executeScriptByName("test_toolpkg_sub.inspect", BTreeMap::new());

        assert_eq!(
            output,
            "\"toolpkg_main:test_toolpkg|sub|test_toolpkg|test_toolpkg|test_toolpkg|dist/sub.js\""
        );
    }

    #[test]
    fn subpackage_execution_uses_toolpkg_main_engine() {
        let script = r#"
            exports.inspect = function(_params) {
                return globalThis.__toolpkg_engine_marker;
            };
        "#;
        let (manager, packageRuntime) = toolpkg_manager(script);
        let engine = packageRuntime.toolpkg_execution_engine("toolpkg_main:test_toolpkg");
        let seedScript = r#"
            exports.seed = function(_params) {
                globalThis.__toolpkg_engine_marker = "same-engine";
                return "ok";
            };
        "#;
        let seedParams = BTreeMap::from([(
            "__operit_package_lang".to_string(),
            Value::String("en".to_string()),
        )]);
        let seedOutput = engine.execute_script_function(
            seedScript,
            "seed",
            &seedParams,
            &BTreeMap::new(),
            None,
            true,
            60,
        );

        assert_eq!(seedOutput.as_deref(), Some("\"ok\""));
        assert_eq!(
            manager.executeScriptByName("test_toolpkg_sub.inspect", BTreeMap::new()),
            "\"same-engine\""
        );
    }
}
