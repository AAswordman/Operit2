use std::collections::BTreeMap;
use std::sync::Arc;

use operit_plugin_sdk::execution_result::decode_js_execution_result_value;
pub use operit_plugin_sdk::javascript::{
    JsExecutionEngine, JsExecutionProvider, JsPackageExecutor, JsPackageRuntime,
    JsPackageToolCallRequest, JsPackageToolCallResult, ToolPkgMainRegistrationCapture,
};
use operit_plugin_sdk::javascript::{
    JsToolNameResolutionRequest, JsToolPkgIpcRequest, JsToolPkgResourceRequest,
};

use crate::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use crate::tools::AIToolHandler::AIToolHandler;
use operit_util::LocaleUtils::LocaleUtils;
use operit_util::OperitPaths;
use serde_json::Value;

const TOOLPKG_SCRIPT_TIMEOUT_SECONDS: u64 = 60;
const TOOLPKG_IPC_DISPATCH_FUNCTION_NAME: &str = "__operit_toolpkg_runtime_dispatch__";
const TOOLPKG_IPC_DISPATCH_FUNCTION_SOURCE: &str = r#"
async function(params) {
    var dispatch = globalThis.__operitInvokeToolPkgIpcLocal;
    if (typeof dispatch !== 'function') {
        throw new Error('ToolPkg.ipc runtime is unavailable in target context');
    }
    var payload = JSON.parse(params.__operit_toolpkg_ipc_payload_json);
    var channel = params.__operit_toolpkg_ipc_channel.trim();
    if (!channel) {
        throw new Error('ToolPkg.ipc channel is required');
    }
    return await dispatch(channel, payload, {
        channel: channel,
        callerContextKey: params.__operit_toolpkg_ipc_caller_context_key,
        currentContextKey: params.__operit_execution_context_key,
        currentRuntime: params.__operit_toolpkg_runtime_kind,
        packageTarget: params.__operit_ui_package_name
    });
}
"#;

/// Adapts the Operit package manager to the SDK JavaScript package contract.
#[derive(Clone)]
pub struct PackageManagerJsRuntime {
    package_manager: Arc<std::sync::Mutex<RuntimePackageManager>>,
    tool_handler: AIToolHandler,
}

impl PackageManagerJsRuntime {
    /// Creates an SDK package runtime backed by the shared package manager.
    pub fn new(
        package_manager: Arc<std::sync::Mutex<RuntimePackageManager>>,
        tool_handler: AIToolHandler,
    ) -> Self {
        Self {
            package_manager,
            tool_handler,
        }
    }
}

impl JsPackageRuntime for PackageManagerJsRuntime {
    /// Returns the current host language for package JavaScript.
    fn package_language(&self) -> Result<String, String> {
        let language = LocaleUtils::getCurrentLanguage(&self.tool_handler.getContext(), "")?;
        let language = language.trim();
        if language.is_empty() {
            return Err("Host returned an empty package language".to_string());
        }
        Ok(language.to_string())
    }

    /// Returns one registered package definition.
    fn package(&self, package_name: &str) -> Option<operit_plugin_sdk::package::ToolPackage> {
        self.package_manager
            .lock()
            .expect("package manager mutex poisoned")
            .getPackageTools(package_name)
    }

    /// Returns the active conditional state id for one package.
    fn active_package_state_id(&self, package_name: &str) -> Option<String> {
        self.package_manager
            .lock()
            .expect("package manager mutex poisoned")
            .getActivePackageStateId(package_name)
    }

    /// Resolves ToolPkg runtime metadata for one executable subpackage.
    fn resolve_toolpkg_subpackage(
        &self,
        package_name: &str,
    ) -> Option<operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgSubpackageRuntime> {
        self.package_manager
            .lock()
            .expect("package manager mutex poisoned")
            .resolveToolPkgSubpackageRuntimeInternal(package_name)
    }

    /// Returns the shared ToolPkg engine for one explicitly owned execution context.
    fn toolpkg_execution_engine(
        &self,
        context_key: &str,
        container_package_name: &str,
    ) -> Arc<dyn JsExecutionEngine> {
        self.package_manager
            .lock()
            .expect("package manager mutex poisoned")
            .getToolPkgExecutionEngine(context_key, container_package_name)
    }
}

/// Materializes one ToolPkg resource through the concrete package manager.
#[allow(non_snake_case)]
pub fn materializeToolPkgResource(
    toolHandler: &AIToolHandler,
    request: JsToolPkgResourceRequest,
) -> Result<String, String> {
    let target = requiredText(
        &request.package_name_or_subpackage_id,
        "ToolPkg resource target",
    )?;
    let resourceKey = requiredText(&request.resource_key, "ToolPkg resource key")?;
    let packageManager = toolHandler.getOrCreatePackageManager();
    let guard = packageManager
        .lock()
        .expect("package manager mutex poisoned");
    let outputFileName = match request.output_file_name {
        Some(value) => requiredText(&value, "ToolPkg resource output file name")?,
        None => guard
            .getToolPkgResourceOutputFileName(&target, &resourceKey, true)
            .ok_or_else(|| {
                format!("ToolPkg resource output file name is not declared: {target}/{resourceKey}")
            })?,
    };
    let safeName = outputFileName
        .rsplit(['/', '\\'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "ToolPkg resource output file name is invalid".to_string())?;
    let fileSystemHost = toolHandler
        .getContext()
        .fileSystemHost
        .ok_or_else(|| "FileSystemHost is required for ToolPkg resource export".to_string())?;
    let outputDir = OperitPaths::toolPkgResourceExportsDir(request.internal)?;
    let outputDirPath = outputDir
        .to_str()
        .ok_or_else(|| "ToolPkg resource export directory is not valid UTF-8".to_string())?;
    fileSystemHost
        .makeDirectory(outputDirPath, true)
        .map_err(|error| error.to_string())?;
    let outputFile = outputDir.join(safeName);

    let copied = if guard.getToolPkgContainerRuntime(&target).is_some() {
        guard.copyToolPkgResourceToFile(&target, &resourceKey, &outputFile)
    } else if let Some(runtime) = guard.resolveToolPkgSubpackageRuntimeInternal(&target) {
        guard.copyToolPkgResourceToFileBySubpackageId(
            &runtime.subpackageId,
            &resourceKey,
            &outputFile,
            true,
        )
    } else if guard
        .findPreferredPackageNameForSubpackageId(&target, true)
        .is_some()
    {
        guard.copyToolPkgResourceToFileBySubpackageId(&target, &resourceKey, &outputFile, true)
    } else {
        return Err(format!("ToolPkg resource target not found: {target}"));
    };
    if !copied {
        return Err(format!(
            "ToolPkg resource not found: {target}/{resourceKey}"
        ));
    }
    Ok(outputFile.to_string_lossy().to_string())
}

/// Resolves one JavaScript tool name through concrete package state.
#[allow(non_snake_case)]
pub fn resolveJsToolName(
    toolHandler: &AIToolHandler,
    request: JsToolNameResolutionRequest,
) -> Result<String, String> {
    let toolName = requiredText(&request.tool_name, "Tool name")?;
    if toolName.contains(':') {
        return Ok(toolName);
    }
    let packageManager = toolHandler.getOrCreatePackageManager();
    let guard = packageManager
        .lock()
        .expect("package manager mutex poisoned");
    let packageName = match (request.package_name, request.subpackage_id) {
        (Some(packageName), None) => {
            let packageName = requiredText(&packageName, "Package name")?;
            if guard.getPackageTools(&packageName).is_none() {
                return Err(format!("Package not found: {packageName}"));
            }
            packageName
        }
        (None, Some(subpackageId)) => {
            let subpackageId = requiredText(&subpackageId, "ToolPkg subpackage id")?;
            guard
                .findPreferredPackageNameForSubpackageId(&subpackageId, request.prefer_imported)
                .ok_or_else(|| format!("ToolPkg subpackage not found: {subpackageId}"))?
        }
        (None, None) => return Ok(toolName),
        (Some(_), Some(_)) => {
            return Err("Tool resolution accepts either package_name or subpackage_id".to_string())
        }
    };
    Ok(format!("{packageName}:{toolName}"))
}

/// Invokes one ToolPkg IPC request through the concrete package manager.
#[allow(non_snake_case)]
pub fn invokeToolPkgIpc(
    toolHandler: &AIToolHandler,
    request: JsToolPkgIpcRequest,
) -> Result<Value, String> {
    let packageTarget = requiredText(&request.package_target, "ToolPkg IPC package target")?;
    let channel = requiredText(&request.channel, "ToolPkg IPC channel")?;
    let managerSnapshot = toolHandler
        .getOrCreatePackageManager()
        .lock()
        .expect("package manager mutex poisoned")
        .clone();
    let containerRuntime = managerSnapshot
        .getToolPkgContainerRuntime(&packageTarget)
        .ok_or_else(|| format!("ToolPkg container not found: {packageTarget}"))?;
    let requestedRuntime = request
        .target_runtime
        .map(|value| value.trim().to_ascii_lowercase());
    let targetContextKey = match request.target_context_key {
        Some(value) => requiredText(&value, "ToolPkg IPC target context key")?,
        None if requestedRuntime
            .as_deref()
            .is_none_or(|value| value == "main") =>
        {
            format!("toolpkg_main:{packageTarget}")
        }
        None => {
            return Err(format!(
                "ToolPkg IPC target context key is required for runtime {}",
                requestedRuntime.as_deref().unwrap_or_default()
            ))
        }
    };
    let inferredRuntime = inferToolPkgRuntime(&targetContextKey)?;
    let targetRuntime = match requestedRuntime {
        Some(value) => {
            if value != inferredRuntime {
                return Err(format!(
                    "ToolPkg IPC runtime does not match context key: {value} != {inferredRuntime}"
                ));
            }
            value
        }
        None => inferredRuntime,
    };
    let mainContextKey = format!("toolpkg_main:{packageTarget}");
    let isMainTarget = targetRuntime == "main";
    if isMainTarget && !targetContextKey.eq_ignore_ascii_case(&mainContextKey) {
        return Err(format!(
            "ToolPkg IPC main context key is invalid: {targetContextKey}"
        ));
    }
    let engine = if isMainTarget {
        managerSnapshot.getToolPkgExecutionEngine(&targetContextKey, &packageTarget)
    } else {
        managerSnapshot
            .findToolPkgExecutionEngine(&targetContextKey, &packageTarget)
            .ok_or_else(|| format!("ToolPkg runtime is not active: {targetContextKey}"))?
    };
    let (scriptPath, script) = if isMainTarget {
        let scriptPath = requiredText(&containerRuntime.mainEntry, "ToolPkg main entry")?;
        let script = managerSnapshot
            .getToolPkgMainScriptInternal(&packageTarget)
            .ok_or_else(|| format!("ToolPkg main script is unavailable: {packageTarget}"))?;
        (scriptPath, script)
    } else {
        (String::new(), String::new())
    };
    let mut params = BTreeMap::new();
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String(packageTarget.clone()),
    );
    params.insert(
        "toolPkgId".to_string(),
        Value::String(packageTarget.clone()),
    );
    params.insert(
        "containerPackageName".to_string(),
        Value::String(packageTarget),
    );
    params.insert(
        "__operit_execution_context_key".to_string(),
        Value::String(targetContextKey),
    );
    params.insert(
        "__operit_toolpkg_runtime_kind".to_string(),
        Value::String(targetRuntime),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String(scriptPath),
    );
    params.insert(
        "__operit_inline_function_name".to_string(),
        Value::String(TOOLPKG_IPC_DISPATCH_FUNCTION_NAME.to_string()),
    );
    params.insert(
        "__operit_inline_function_source".to_string(),
        Value::String(TOOLPKG_IPC_DISPATCH_FUNCTION_SOURCE.trim().to_string()),
    );
    params.insert(
        "__operit_toolpkg_ipc_channel".to_string(),
        Value::String(channel),
    );
    params.insert(
        "__operit_toolpkg_ipc_payload_json".to_string(),
        Value::String(request.payload.to_string()),
    );
    params.insert(
        "__operit_toolpkg_ipc_caller_context_key".to_string(),
        Value::String(request.caller_context_key.trim().to_string()),
    );
    let result = engine
        .execute_script_function(
            &script,
            TOOLPKG_IPC_DISPATCH_FUNCTION_NAME,
            &params,
            &BTreeMap::new(),
            None,
            true,
            TOOLPKG_SCRIPT_TIMEOUT_SECONDS,
        )
        .map_err(|error| error.to_string())?;
    decode_js_execution_result_value(result.as_deref()).map_err(|error| error.to_string())
}

/// Resolves the ToolPkg runtime kind encoded by a context key.
#[allow(non_snake_case)]
fn inferToolPkgRuntime(contextKey: &str) -> Result<String, String> {
    let normalized = contextKey.trim().to_ascii_lowercase();
    if normalized.starts_with("toolpkg_main:") {
        return Ok("main".to_string());
    }
    if normalized.starts_with("toolpkg_provider:") {
        return Ok("provider".to_string());
    }
    if normalized.starts_with("toolpkg_compose:")
        || normalized.starts_with("toolpkg_compose_dsl:")
        || normalized.starts_with("toolpkg_xml_render:")
    {
        return Ok("ui".to_string());
    }
    if normalized.starts_with("toolpkg_sandbox:") {
        return Ok("sandbox".to_string());
    }
    Err(format!(
        "ToolPkg context key has an unknown runtime: {contextKey}"
    ))
}

/// Returns one trimmed non-empty contract string.
#[allow(non_snake_case)]
fn requiredText(value: &str, fieldName: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{fieldName} is required"));
    }
    Ok(trimmed.to_string())
}
