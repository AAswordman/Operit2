use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use serde_json::Value;

use crate::api::chat::enhance::ConversationMarkupManager::ToolResult;
use crate::api::chat::enhance::ToolExecutionManager::AITool;
use crate::core::tools::javascript::JsEngine::JsEngine;
use crate::core::tools::javascript::JsExecutionResultProtocol::{
    extractJsExecutionFailure, JsExecutionFailure,
};
use crate::core::tools::packTool::PackageManager::PackageManager;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::core::tools::ToolResultDataClasses::stringResultData;

#[derive(Clone)]
pub struct JsToolManager {
    packageManager: Arc<Mutex<PackageManager>>,
    toolHandler: AIToolHandler,
    enginePool: Arc<(Mutex<Vec<JsEngine>>, Condvar)>,
}

#[derive(Debug)]
struct ToolParameterConversionException {
    message: String,
}

const MAX_CONCURRENT_ENGINES: usize = 4;
static INSTANCE: OnceLock<JsToolManager> = OnceLock::new();

impl JsToolManager {
    #[allow(non_snake_case)]
    pub fn getInstance(
        packageManager: Arc<Mutex<PackageManager>>,
        toolHandler: AIToolHandler,
    ) -> Self {
        INSTANCE
            .get_or_init(|| {
                let engines = (0..MAX_CONCURRENT_ENGINES)
                    .map(|_| JsEngine::new(toolHandler.clone()))
                    .collect::<Vec<_>>();
                Self {
                    packageManager,
                    toolHandler,
                    enginePool: Arc::new((Mutex::new(engines), Condvar::new())),
                }
            })
            .clone()
    }

    #[allow(non_snake_case)]
    fn withEngine<T>(&self, block: impl FnOnce(JsEngine) -> T) -> T {
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
        block: impl FnOnce(JsEngine) -> T,
    ) -> T {
        let toolPkgRuntime = self
            .packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .resolveToolPkgSubpackageRuntimeInternal(packageName);
        if let Some(runtime) = toolPkgRuntime {
            let contextKey = format!("toolpkg_main:{}", runtime.containerPackageName);
            let engine = self
                .packageManager
                .lock()
                .expect("package manager mutex poisoned")
                .getToolPkgExecutionEngine(&contextKey);
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
    ) -> BTreeMap<String, Value> {
        let mut runtimeParams = params;
        if let Some(stateId) = self
            .packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .getActivePackageStateId(packageName)
        {
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

        if let Some(runtime) = self
            .packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .resolveToolPkgSubpackageRuntimeInternal(packageName)
        {
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
        runtimeParams
    }

    #[allow(non_snake_case)]
    fn convertToolParameters(
        &self,
        tool: &AITool,
        packageName: &str,
        functionName: &str,
    ) -> Result<BTreeMap<String, Value>, ToolParameterConversionException> {
        let packageTools = self
            .packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .getPackageTools(packageName);
        let toolDefinition = packageTools
            .as_ref()
            .and_then(|package| package.tools.iter().find(|item| item.name == functionName));

        let missingRequiredParameters = toolDefinition
            .map(|definition| {
                definition
                    .parameters
                    .iter()
                    .filter(|parameter| {
                        parameter.required
                            && !tool
                                .parameters
                                .iter()
                                .any(|item| item.name == parameter.name)
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
        for parameter in &tool.parameters {
            let parameterType = toolDefinition
                .and_then(|definition| {
                    definition
                        .parameters
                        .iter()
                        .find(|item| item.name == parameter.name)
                })
                .map(|item| item.parameter_type.to_ascii_lowercase())
                .unwrap_or_else(|| "string".to_string());
            let value = self.convertToolParameterValue(
                &tool.name,
                &parameter.name,
                &parameter.value,
                &parameterType,
            )?;
            converted.insert(parameter.name.clone(), value);
        }

        Ok(self.buildRuntimeParams(packageName, converted))
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

    fn success(toolName: &str, value: Option<String>) -> ToolResult {
        ToolResult {
            toolName: toolName.to_string(),
            success: true,
            result: stringResultData(value.unwrap_or_else(|| "null".to_string())),
            error: None,
        }
    }

    fn failure(toolName: &str, message: String) -> ToolResult {
        ToolResult {
            toolName: toolName.to_string(),
            success: false,
            result: stringResultData(""),
            error: Some(message),
        }
    }

    #[allow(non_snake_case)]
    fn failureFromJs(toolName: &str, failure: JsExecutionFailure) -> ToolResult {
        ToolResult {
            toolName: toolName.to_string(),
            success: false,
            result: stringResultData(failure.dataText),
            error: Some(failure.message),
        }
    }

    #[allow(non_snake_case)]
    pub fn executeScriptByName(&self, toolName: &str, params: BTreeMap<String, String>) -> String {
        let Some((packageName, functionName)) = Self::parseDotCall(toolName) else {
            return format!(
                "Invalid tool name format: {toolName}. Expected format: packageName.functionName"
            );
        };
        let script = self
            .packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .getPackageScript(&packageName);
        let Some(script) = script else {
            return format!("Package not found: {packageName}");
        };
        let params = params
            .into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect::<BTreeMap<_, _>>();
        let runtimeParams = self.buildRuntimeParams(&packageName, params);
        self.withExecutionEngineForPackage(&packageName, |engine| {
            engine
                .executeScriptFunction(
                    &script,
                    &functionName,
                    &runtimeParams,
                    &BTreeMap::new(),
                    None,
                    true,
                    60,
                    None,
                )
                .unwrap_or_else(|| "null".to_string())
        })
    }

    #[allow(non_snake_case)]
    pub fn executeScript(&self, script: &str, tool: &AITool) -> Vec<ToolResult> {
        let Some((packageName, functionName)) = Self::parsePackageToolName(&tool.name) else {
            return vec![Self::failure(
                &tool.name,
                "Invalid tool name format. Expected 'packageName:toolName'".to_string(),
            )];
        };

        let runtimeParams = match self.convertToolParameters(tool, &packageName, &functionName) {
            Ok(value) => value,
            Err(error) => return vec![Self::failure(&tool.name, error.message)],
        };

        let result = self.withExecutionEngineForPackage(&packageName, |engine| {
            engine.executeScriptFunction(
                script,
                &functionName,
                &runtimeParams,
                &BTreeMap::new(),
                None,
                true,
                60,
                None,
            )
        });
        if let Some(failure) = extractJsExecutionFailure(result.as_deref()) {
            vec![Self::failureFromJs(&tool.name, failure)]
        } else {
            vec![Self::success(&tool.name, result)]
        }
    }

    pub fn destroy(&self) {}
}

#[cfg(test)]
mod tests {
    use super::JsToolManager;
    use crate::core::application::OperitApplicationContext::OperitApplicationContext;
    use crate::core::tools::packTool::PackageManager::PackageManager;
    use crate::core::tools::packTool::ToolPkgParser::{
        ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType, ToolPkgSubpackageRuntime,
    };
    use crate::core::tools::AIToolHandler::AIToolHandler;
    use crate::core::tools::ToolPackage::{PackageTool, ToolPackage};
    use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
    use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
    use operit_store::RuntimeStorePaths::{setDefaultRuntimeStoreRoot, RuntimeStorePaths};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::path::{Component, Path, PathBuf};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug)]
    struct TestRuntimeStorageHost {
        root: PathBuf,
    }

    impl TestRuntimeStorageHost {
        fn new(root: PathBuf) -> Self {
            Self { root }
        }

        fn resolve(&self, path: &str) -> HostResult<PathBuf> {
            let path = Path::new(path);
            if path.is_absolute() {
                return Err(HostError::new(format!(
                    "Runtime storage path must be relative: {}",
                    path.display()
                )));
            }
            let mut resolved = self.root.clone();
            for component in path.components() {
                match component {
                    Component::Normal(segment) => resolved.push(segment),
                    Component::CurDir => {}
                    _ => {
                        return Err(HostError::new(format!(
                            "Invalid runtime storage path: {}",
                            path.display()
                        )))
                    }
                }
            }
            Ok(resolved)
        }
    }

    impl RuntimeStorageHost for TestRuntimeStorageHost {
        fn rootDir(&self) -> Option<PathBuf> {
            Some(self.root.clone())
        }

        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            Ok(std::fs::read(self.resolve(path)?)?)
        }

        fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            let path = self.resolve(path)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)?;
            Ok(())
        }

        fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
            let path = self.resolve(path)?;
            if !path.exists() {
                return Ok(());
            }
            if path.is_dir() {
                if recursive {
                    std::fs::remove_dir_all(path)?;
                } else {
                    std::fs::remove_dir(path)?;
                }
            } else {
                std::fs::remove_file(path)?;
            }
            Ok(())
        }

        fn exists(&self, path: &str) -> HostResult<bool> {
            Ok(self.resolve(path)?.exists())
        }

        fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
            let directory = self.resolve(prefix)?;
            let mut entries = Vec::new();
            if !directory.exists() {
                return Ok(entries);
            }
            for entry in std::fs::read_dir(directory)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                let path = entry
                    .path()
                    .strip_prefix(&self.root)
                    .map_err(|error| HostError::new(error.to_string()))?
                    .to_string_lossy()
                    .replace('\\', "/");
                entries.push(RuntimeStorageEntry {
                    path,
                    isDirectory: metadata.is_dir(),
                    size: metadata.len() as i64,
                });
            }
            Ok(entries)
        }
    }

    fn test_paths(name: &str) -> RuntimeStorePaths {
        let root = std::env::temp_dir().join(format!(
            "operit-js-tool-manager-tests-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("test runtime root");
        let host = Arc::new(TestRuntimeStorageHost::new(root.clone()));
        setDefaultRuntimeStoreRoot(root.clone());
        setDefaultRuntimeStorageHost(host);
        RuntimeStorePaths::new(root)
    }

    fn toolpkg_manager(script: &str) -> (JsToolManager, Arc<Mutex<PackageManager>>) {
        let paths = test_paths("toolpkg-manager");
        let packageManager = Arc::new(Mutex::new(PackageManager::newWithContext(
            paths,
            OperitApplicationContext::new(),
        )));
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
        assert!(packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .registerToolPkg(loadResult));
        let manager = JsToolManager {
            packageManager: packageManager.clone(),
            toolHandler: AIToolHandler::getInstance(OperitApplicationContext::new()),
            enginePool: Arc::new((Mutex::new(Vec::new()), std::sync::Condvar::new())),
        };
        (manager, packageManager)
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
        let (manager, packageManager) = toolpkg_manager(script);
        let engine = packageManager
            .lock()
            .expect("package manager mutex poisoned")
            .getToolPkgExecutionEngine("toolpkg_main:test_toolpkg");
        let seedScript = r#"
            exports.seed = function(_params) {
                globalThis.__toolpkg_engine_marker = "same-engine";
                return "ok";
            };
        "#;
        let seedOutput = engine.executeScriptFunction(
            seedScript,
            "seed",
            &BTreeMap::new(),
            &BTreeMap::new(),
            None,
            true,
            60,
            None,
        );

        assert_eq!(seedOutput.as_deref(), Some("\"ok\""));
        assert_eq!(
            manager.executeScriptByName("test_toolpkg_sub.inspect", BTreeMap::new()),
            "\"same-engine\""
        );
    }
}
