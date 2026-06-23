use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::core::tools::AIToolHandler::{AIToolHandler, ExternalToolInvocationBridge};
use serde_json::Value;

use crate::ipc::{inferToolPkgIpcRuntimeFromContextKey, toolPkgIpcDispatchFunctionSource};
use crate::javascript::JsEngine::JsEngine;
use crate::loader::{loadToolPkgFileWithEngine, readToolPkgTextResourceFromMap, LoadedToolPkg};
use crate::models::{
    ToolPkgExecutionOutcome, ToolPkgFunctionCall, ToolPkgIpcCall, ToolPkgLoadOutcome,
    ToolPkgMainHookCall, ToolPkgRuntimeOptions,
};
const TOOLPKG_SCRIPT_TIMEOUT_SECONDS: u64 = 60;

#[derive(Clone)]
pub struct ToolPkgRuntime {
    registrationEngine: JsEngine,
    toolHandler: AIToolHandler,
    executionEngines: Arc<Mutex<BTreeMap<String, JsEngine>>>,
    loadedPackages: Arc<Mutex<BTreeMap<String, LoadedToolPkg>>>,
    languageCode: String,
}

impl ToolPkgRuntime {
    pub fn new(options: ToolPkgRuntimeOptions) -> Self {
        Self {
            registrationEngine: JsEngine::newToolPkgRegistrationEngine(),
            toolHandler: AIToolHandler::new(),
            executionEngines: Arc::new(Mutex::new(BTreeMap::new())),
            loadedPackages: Arc::new(Mutex::new(BTreeMap::new())),
            languageCode: options.languageCode,
        }
    }

    #[allow(non_snake_case)]
    pub fn setExternalToolBridge(&self, bridge: Option<Arc<dyn ExternalToolInvocationBridge>>) {
        let mut toolHandler = self.toolHandler.clone();
        toolHandler.setExternalToolBridge(bridge);
    }

    #[allow(non_snake_case)]
    pub fn loadToolPkgFile(&self, path: impl AsRef<Path>) -> Result<ToolPkgLoadOutcome, String> {
        let loaded =
            loadToolPkgFileWithEngine(path.as_ref(), &self.registrationEngine, &self.languageCode)?;
        let packageName = loaded.outcome.package.containerRuntime.packageName.clone();
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .insert(packageName, loaded.clone());
        Ok(loaded.outcome)
    }

    #[allow(non_snake_case)]
    pub fn readToolPkgTextResource(
        &self,
        containerPackageName: &str,
        resourcePath: &str,
    ) -> Option<String> {
        let loaded = self.loadedPackage(containerPackageName).ok()?;
        readToolPkgTextResourceFromMap(&loaded.textResources, resourcePath)
    }

    #[allow(non_snake_case)]
    pub fn runFunction(
        &self,
        call: ToolPkgFunctionCall,
    ) -> Result<ToolPkgExecutionOutcome, String> {
        let contextKey = call
            .executionContextKey
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("toolpkg_function:default");
        let engine = self.executionEngine(contextKey);
        let timeoutSeconds = call
            .timeoutSeconds
            .unwrap_or(TOOLPKG_SCRIPT_TIMEOUT_SECONDS);
        let value = engine.executeScriptFunction(
            &call.script,
            &call.functionName,
            &call.params,
            &call.envOverrides,
            None,
            true,
            timeoutSeconds,
            None,
        );
        Ok(ToolPkgExecutionOutcome { value })
    }

    #[allow(non_snake_case)]
    pub fn runMainHook(
        &self,
        call: ToolPkgMainHookCall,
    ) -> Result<ToolPkgExecutionOutcome, String> {
        let loaded = self.loadedPackage(&call.containerPackageName)?;
        let packageName = loaded.outcome.package.containerRuntime.packageName.clone();
        let mut params = BTreeMap::<String, Value>::new();
        let resolvedEventName = call
            .eventName
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(call.event.trim());

        params.insert(
            "event".to_string(),
            Value::String(resolvedEventName.to_string()),
        );
        params.insert(
            "eventName".to_string(),
            Value::String(resolvedEventName.to_string()),
        );
        params.insert("eventPayload".to_string(), call.eventPayload.clone());
        params.insert(
            "timestampMs".to_string(),
            Value::Number(serde_json::Number::from(
                operit_host_api::TimeUtils::currentTimeMillis(),
            )),
        );
        params.insert(
            "functionName".to_string(),
            Value::String(call.functionName.clone()),
        );
        params.insert("toolPkgId".to_string(), Value::String(packageName.clone()));
        params.insert(
            "containerPackageName".to_string(),
            Value::String(packageName.clone()),
        );
        params.insert(
            "__operit_ui_package_name".to_string(),
            Value::String(packageName.clone()),
        );
        params.insert(
            "__operit_script_screen".to_string(),
            Value::String(loaded.mainScriptPath),
        );
        params.insert(
            "__operit_package_lang".to_string(),
            Value::String(self.languageCode.trim().to_string()),
        );
        if let Some(pluginId) = nonBlankOption(call.pluginId.as_deref()) {
            params.insert("pluginId".to_string(), Value::String(pluginId));
        }
        if let Some(chatId) = call
            .eventPayload
            .get("chatId")
            .and_then(Value::as_str)
            .and_then(|value| nonBlankOption(Some(value)))
        {
            params.insert(
                "__operit_package_chat_id".to_string(),
                Value::String(chatId),
            );
        }
        if let Some(functionSource) = nonBlankOption(call.functionSource.as_deref()) {
            params.insert(
                "__operit_inline_function_name".to_string(),
                Value::String(call.functionName.clone()),
            );
            params.insert(
                "__operit_inline_function_source".to_string(),
                Value::String(functionSource),
            );
        }
        if let Some(contextKey) = nonBlankOption(call.executionContextKey.as_deref()) {
            params.insert(
                "__operit_execution_context_key".to_string(),
                Value::String(contextKey),
            );
        }
        if let Some(kind) = nonBlankOption(call.runtimeKind.as_deref()) {
            params.insert(
                "__operit_toolpkg_runtime_kind".to_string(),
                Value::String(kind.to_ascii_lowercase()),
            );
        }

        let resolvedContextKey = resolveToolPkgExecutionContextKey(&packageName, &params);
        let engine = self.executionEngine(&resolvedContextKey);
        let value = engine.executeScriptFunction(
            &loaded.mainScript,
            &call.functionName,
            &params,
            &BTreeMap::new(),
            None,
            true,
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
            None,
        );
        Ok(ToolPkgExecutionOutcome { value })
    }

    #[allow(non_snake_case)]
    pub fn dispatchIpc(&self, call: ToolPkgIpcCall) -> Result<ToolPkgExecutionOutcome, String> {
        let normalizedTarget = nonBlankOption(Some(call.packageTarget.as_str()))
            .ok_or_else(|| "ToolPkg.ipc package target is empty".to_string())?;
        let normalizedChannel = nonBlankOption(Some(call.channel.as_str()))
            .ok_or_else(|| "ToolPkg.ipc channel is required".to_string())?;
        let requestedRuntime = call
            .targetRuntime
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if !requestedRuntime.is_empty()
            && requestedRuntime != "main"
            && requestedRuntime != "ui"
            && requestedRuntime != "sandbox"
            && requestedRuntime != "provider"
        {
            return Err(format!(
                "ToolPkg.ipc targetRuntime is invalid: {requestedRuntime}"
            ));
        }
        let explicitContextKey = call
            .targetContextKey
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        let resolvedContextKey = if !explicitContextKey.is_empty() {
            explicitContextKey
        } else if requestedRuntime.is_empty() || requestedRuntime == "main" {
            format!("toolpkg_main:{normalizedTarget}")
        } else {
            return Err(format!(
                "ToolPkg.ipc targetContextKey is required for targetRuntime={requestedRuntime}"
            ));
        };
        let inferredRuntime = inferToolPkgIpcRuntimeFromContextKey(&resolvedContextKey);
        if !requestedRuntime.is_empty()
            && !inferredRuntime.is_empty()
            && requestedRuntime != inferredRuntime
        {
            return Err(format!(
                "ToolPkg.ipc targetRuntime does not match targetContextKey: {requestedRuntime} != {inferredRuntime}"
            ));
        }
        let resolvedRuntime = if !requestedRuntime.is_empty() {
            requestedRuntime
        } else if !inferredRuntime.is_empty() {
            inferredRuntime
        } else {
            return Err(format!(
                "ToolPkg.ipc targetRuntime is required for targetContextKey={resolvedContextKey}"
            ));
        };
        let isMainTarget = resolvedRuntime == "main";
        let loaded = self.loadedPackage(&normalizedTarget)?;
        let script = if isMainTarget {
            loaded.mainScript.clone()
        } else {
            String::new()
        };
        let scriptPath = if isMainTarget {
            loaded.mainScriptPath.clone()
        } else {
            String::new()
        };

        let dispatchFunctionName = "__operit_toolpkg_runtime_dispatch__";
        let dispatchFunctionSource = toolPkgIpcDispatchFunctionSource();
        let mut params = BTreeMap::new();
        params.insert(
            "__operit_ui_package_name".to_string(),
            Value::String(normalizedTarget.clone()),
        );
        params.insert(
            "toolPkgId".to_string(),
            Value::String(normalizedTarget.clone()),
        );
        params.insert(
            "containerPackageName".to_string(),
            Value::String(normalizedTarget),
        );
        params.insert(
            "__operit_execution_context_key".to_string(),
            Value::String(resolvedContextKey.clone()),
        );
        params.insert(
            "__operit_toolpkg_runtime_kind".to_string(),
            Value::String(resolvedRuntime),
        );
        params.insert(
            "__operit_script_screen".to_string(),
            Value::String(scriptPath),
        );
        params.insert(
            "__operit_inline_function_name".to_string(),
            Value::String(dispatchFunctionName.to_string()),
        );
        params.insert(
            "__operit_inline_function_source".to_string(),
            Value::String(dispatchFunctionSource),
        );
        params.insert(
            "__operit_toolpkg_ipc_channel".to_string(),
            Value::String(normalizedChannel),
        );
        params.insert(
            "__operit_toolpkg_ipc_payload_json".to_string(),
            Value::String(serde_json::to_string(&call.payload).map_err(|error| error.to_string())?),
        );
        params.insert(
            "__operit_toolpkg_ipc_caller_context_key".to_string(),
            Value::String(
                call.callerContextKey
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .to_string(),
            ),
        );

        let engine = self.executionEngine(&resolvedContextKey);
        let value = engine.executeScriptFunction(
            &script,
            dispatchFunctionName,
            &params,
            &BTreeMap::new(),
            None,
            true,
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
            None,
        );
        Ok(ToolPkgExecutionOutcome { value })
    }

    #[allow(non_snake_case)]
    pub fn destroyContext(&self, contextKey: &str) -> bool {
        let engine = self
            .executionEngines
            .lock()
            .expect("toolpkg execution engines mutex poisoned")
            .remove(contextKey.trim());
        if let Some(engine) = engine {
            engine.destroy();
            true
        } else {
            false
        }
    }

    pub fn destroy(&self) {
        self.registrationEngine.destroy();
        let engines = self
            .executionEngines
            .lock()
            .expect("toolpkg execution engines mutex poisoned")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for engine in engines {
            engine.destroy();
        }
        self.executionEngines
            .lock()
            .expect("toolpkg execution engines mutex poisoned")
            .clear();
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .clear();
    }

    /// Get a loaded toolpkg package by name.
    pub fn getLoadedPackage(&self, containerPackageName: &str) -> Result<LoadedToolPkg, String> {
        self.loadedPackage(containerPackageName)
    }

    /// Get all loaded toolpkg packages.
    pub fn getLoadedPackages(&self) -> BTreeMap<String, LoadedToolPkg> {
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .clone()
    }

    /// Register a pre-parsed ToolPkgLoadResult (from a snapshot or bulk scan).
    /// Requires the main script text and text resources for hook/IPC execution support.
    pub fn registerSnapshot(
        &self,
        loadResult: operit_tools::packTool::ToolPkgParser::ToolPkgLoadResult,
        mainScript: String,
        mainScriptPath: String,
        textResources: Arc<BTreeMap<String, String>>,
    ) -> ToolPkgLoadOutcome {
        let packageName = loadResult.containerRuntime.packageName.clone();
        let outcome = ToolPkgLoadOutcome {
            package: loadResult.clone(),
            packageLoadErrors: Vec::new(),
        };
        let loaded = LoadedToolPkg {
            outcome: outcome.clone(),
            mainScript,
            mainScriptPath,
            textResources,
        };
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .insert(packageName.clone(), loaded);
        outcome
    }

    /// Remove a loaded toolpkg package and its execution engines.
    pub fn removePackage(&self, containerPackageName: &str) -> bool {
        let name = containerPackageName.trim();
        let removed = self
            .loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .remove(name)
            .is_some();
        self.destroyContext(&format!("toolpkg_main:{name}"));
        removed
    }

    /// Get the main registration script text of a loaded package.
    pub fn getToolPkgMainScript(&self, containerPackageName: &str) -> Result<String, String> {
        let loaded = self.loadedPackage(containerPackageName)?;
        Ok(loaded.mainScript)
    }

    /// Clear all loaded package data (engines are not affected).
    pub fn clearPackages(&self) {
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .clear();
    }

    /// Get or create an execution engine for a context key.
    /// Core code that needs direct engine access (e.g. JsToolManager) uses this.
    pub fn getOrCreateExecutionEngine(&self, contextKey: &str) -> JsEngine {
        self.executionEngine(contextKey)
    }

    /// Find an existing execution engine without creating one.
    pub fn findExecutionEngine(&self, contextKey: &str) -> Option<JsEngine> {
        let normalizedKey = match contextKey.trim() {
            "" => "toolpkg_main:default".to_string(),
            value => value.to_string(),
        };
        self.executionEngines
            .lock()
            .expect("toolpkg execution engines mutex poisoned")
            .get(&normalizedKey)
            .cloned()
    }

    /// Release (destroy) an execution engine by context key.
    pub fn releaseExecutionEngine(&self, contextKey: &str) -> bool {
        self.destroyContext(contextKey)
    }

    /// Destroy the default execution engine for a toolpkg package.
    pub fn destroyDefaultToolPkgExecutionEngine(&self, packageName: &str) -> bool {
        let normalizedName = packageName.trim();
        self.destroyContext(&format!("toolpkg_main:{normalizedName}"))
    }

    /// Return the number of loaded toolpkg packages.
    pub fn getLoadedPackageCount(&self) -> usize {
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .len()
    }

    /// Check if a package name refers to a loaded container.
    pub fn isToolPkgContainer(&self, packageName: &str) -> bool {
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .contains_key(packageName.trim())
    }

    /// Check if a package name is a subpackage of any loaded container.
    pub fn hasSubpackage(&self, packageName: &str) -> bool {
        let packageName = packageName.trim();
        let loaded = self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned");
        loaded.values().any(|pkg|
            pkg.outcome.package.containerRuntime.subpackages.iter().any(|sp| sp.packageName == packageName)
        )
    }

    /// Get a container runtime by package name.
    pub fn getToolPkgContainerRuntime(&self, packageName: &str) -> Option<operit_tools::packTool::ToolPkgParser::ToolPkgContainerRuntime> {
        let packageName = packageName.trim();
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .get(packageName)
            .map(|pkg| pkg.outcome.package.containerRuntime.clone())
    }

    /// Get all container runtimes, sorted by name.
    pub fn getToolPkgContainerRuntimes(&self) -> Vec<operit_tools::packTool::ToolPkgParser::ToolPkgContainerRuntime> {
        let mut runtimes: Vec<_> = self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .values()
            .map(|pkg| pkg.outcome.package.containerRuntime.clone())
            .collect();
        runtimes.sort_by(|left, right| left.packageName.cmp(&right.packageName));
        runtimes
    }

    /// Find a subpackage runtime by subpackage or container name across all loaded packages.
    pub fn resolveToolPkgSubpackageRuntime(&self, packageName: &str) -> Option<operit_tools::packTool::ToolPkgParser::ToolPkgSubpackageRuntime> {
        let packageName = packageName.trim();
        let loaded = self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned");
        for pkg in loaded.values() {
            for subpackage in &pkg.outcome.package.containerRuntime.subpackages {
                if subpackage.packageName == packageName {
                    return Some(subpackage.clone());
                }
            }
        }
        None
    }

    /// Register a pre-parsed ToolPkgLoadResult with minimal data (no mainScript/textResources).
    /// For full registration with scripts, use registerSnapshot() instead.
    pub fn registerToolPkg(&self, loadResult: operit_tools::packTool::ToolPkgParser::ToolPkgLoadResult) {
        let packageName = loadResult.containerRuntime.packageName.clone();
        let loaded = LoadedToolPkg {
            outcome: ToolPkgLoadOutcome {
                package: loadResult,
                packageLoadErrors: Vec::new(),
            },
            mainScript: String::new(),
            mainScriptPath: String::new(),
            textResources: Arc::new(BTreeMap::new()),
        };
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .insert(packageName, loaded);
    }

    /// Remove a loaded toolpkg container and return its runtime, if found.
    pub fn removeToolPkgContainer(&self, packageName: &str) -> Option<operit_tools::packTool::ToolPkgParser::ToolPkgContainerRuntime> {
        let packageName = packageName.trim();
        let runtime = {
            let loaded = self.loadedPackages
                .lock()
                .expect("loaded toolpkg mutex poisoned");
            loaded.get(packageName).map(|pkg| pkg.outcome.package.containerRuntime.clone())
        };
        if let Some(_) = runtime {
            self.removePackage(packageName);
        }
        runtime
    }

    /// Check whether a ToolPkgLoadResult can be registered (no name conflicts with loaded packages or the given available set).
    pub fn canRegisterToolPkg(
        &self,
        loadResult: &operit_tools::packTool::ToolPkgParser::ToolPkgLoadResult,
        availablePackages: &std::collections::BTreeMap<String, operit_tools::ToolPackage::ToolPackage>,
    ) -> bool {
        let loaded = self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned");
        let containerName = loadResult.containerRuntime.packageName.trim();
        if containerName.is_empty()
            || loaded.contains_key(containerName)
            || availablePackages.contains_key(containerName)
        {
            return false;
        }
        for subpackage in &loadResult.containerRuntime.subpackages {
            let subName = subpackage.packageName.trim();
            if subName.is_empty()
                || loaded.contains_key(subName)
                || availablePackages.contains_key(subName)
            {
                return false;
            }
        }
        true
    }

    /// Bulk replace internal data from a container runtimes map (used during scan/refresh).
    pub fn replaceRuntimeMaps(&self, containers: std::collections::BTreeMap<String, operit_tools::packTool::ToolPkgParser::ToolPkgContainerRuntime>) {
        let mut newLoaded = std::collections::BTreeMap::new();
        for (name, runtime) in containers {
            let loadResult = operit_tools::packTool::ToolPkgParser::ToolPkgLoadResult {
                containerRuntime: runtime,
                ..Default::default()
            };
            newLoaded.insert(name, LoadedToolPkg {
                outcome: ToolPkgLoadOutcome {
                    package: loadResult,
                    packageLoadErrors: Vec::new(),
                },
                mainScript: String::new(),
                mainScriptPath: String::new(),
                textResources: Arc::new(BTreeMap::new()),
            });
        }
        *self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned") = newLoaded;
    }

    fn loadedPackage(&self, containerPackageName: &str) -> Result<LoadedToolPkg, String> {
        let name = containerPackageName.trim();
        self.loadedPackages
            .lock()
            .expect("loaded toolpkg mutex poisoned")
            .get(name)
            .cloned()
            .ok_or_else(|| format!("ToolPkg container not loaded: {name}"))
    }

    #[allow(non_snake_case)]
    fn executionEngine(&self, contextKey: &str) -> JsEngine {
        let normalizedKey = match contextKey.trim() {
            "" => "toolpkg_main:default".to_string(),
            value => value.to_string(),
        };
        let mut engines = self
            .executionEngines
            .lock()
            .expect("toolpkg execution engines mutex poisoned");
        engines
            .entry(normalizedKey)
            .or_insert_with(|| JsEngine::new(self.toolHandler.clone()))
            .clone()
    }
}

#[allow(non_snake_case)]
fn resolveToolPkgExecutionContextKey(
    containerPackageName: &str,
    params: &BTreeMap<String, Value>,
) -> String {
    params
        .get("__operit_execution_context_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("toolpkg_main:{containerPackageName}"))
}

#[allow(non_snake_case)]
fn nonBlankOption(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[allow(non_snake_case)]
pub fn loadToolPkgFile(
    path: impl AsRef<Path>,
    languageCode: &str,
) -> Result<ToolPkgLoadOutcome, String> {
    let runtime = ToolPkgRuntime::new(ToolPkgRuntimeOptions {
        languageCode: languageCode.to_string(),
    });
    let result = runtime.loadToolPkgFile(path);
    runtime.destroy();
    result
}

#[allow(non_snake_case)]
pub fn loadToolPkgSnapshotJson(
    path: impl AsRef<Path>,
    languageCode: &str,
) -> Result<String, String> {
    let outcome = loadToolPkgFile(path, languageCode)?;
    serde_json::to_string_pretty(&outcome).map_err(|error| error.to_string())
}
