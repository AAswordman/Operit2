use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::tools::packTool::ToolPkgManager::ToolPkgManager;

pub struct PackageManagerToolPkgFacade;

impl PackageManagerToolPkgFacade {
    #[allow(non_snake_case)]
    pub fn runToolPkgMainHook(
        toolPkgManager: &ToolPkgManager,
        context: &OperitApplicationContext,
        enabledPackageNames: &[String],
        containerPackageName: &str,
        functionName: &str,
        event: &str,
        eventName: Option<&str>,
        pluginId: Option<&str>,
        inlineFunctionSource: Option<&str>,
        eventPayload: Value,
        executionContextKey: Option<&str>,
        runtimeKind: Option<&str>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Result<Option<String>, String> {
        let normalizedContainerPackageName = containerPackageName.trim().to_string();
        let runtime = toolPkgManager
            .getToolPkgContainerRuntime(&normalizedContainerPackageName)
            .ok_or_else(|| {
                format!("ToolPkg container not found: {normalizedContainerPackageName}")
            })?;
        let script = toolPkgManager
            .getToolPkgMainScriptInternal(&normalizedContainerPackageName, enabledPackageNames)
            .ok_or_else(|| {
                format!("ToolPkg main script is unavailable: {normalizedContainerPackageName}")
            })?;

        let resolvedEventName = eventName
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(event);
        let mut params = BTreeMap::<String, Value>::new();
        params.insert(
            "event".to_string(),
            Value::String(resolvedEventName.to_string()),
        );
        params.insert(
            "eventName".to_string(),
            Value::String(resolvedEventName.to_string()),
        );
        params.insert("eventPayload".to_string(), eventPayload.clone());
        params.insert(
            "timestampMs".to_string(),
            Value::Number(serde_json::Number::from(
                operit_host_api::TimeUtils::currentTimeMillis(),
            )),
        );
        params.insert(
            "functionName".to_string(),
            Value::String(functionName.to_string()),
        );
        params.insert(
            "toolPkgId".to_string(),
            Value::String(normalizedContainerPackageName.clone()),
        );
        params.insert(
            "containerPackageName".to_string(),
            Value::String(normalizedContainerPackageName.clone()),
        );
        params.insert(
            "__operit_ui_package_name".to_string(),
            Value::String(normalizedContainerPackageName.clone()),
        );
        params.insert(
            "__operit_script_screen".to_string(),
            Value::String(runtime.mainEntry),
        );
        if let Some(pluginId) = pluginId.map(str::trim).filter(|value| !value.is_empty()) {
            params.insert("pluginId".to_string(), Value::String(pluginId.to_string()));
        }
        if let Some(chatId) = eventPayload
            .get("chatId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_package_chat_id".to_string(),
                Value::String(chatId.to_string()),
            );
        }
        if let Some(functionSource) = inlineFunctionSource
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_inline_function_name".to_string(),
                Value::String(functionName.to_string()),
            );
            params.insert(
                "__operit_inline_function_source".to_string(),
                Value::String(functionSource.to_string()),
            );
        }
        if let Some(contextKey) = executionContextKey
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_execution_context_key".to_string(),
                Value::String(contextKey.to_string()),
            );
        }
        if let Some(kind) = runtimeKind.map(str::trim).filter(|value| !value.is_empty()) {
            params.insert(
                "__operit_toolpkg_runtime_kind".to_string(),
                Value::String(kind.to_ascii_lowercase()),
            );
        }

        let resolvedContextKey =
            resolveToolPkgExecutionContextKey(&normalizedContainerPackageName, &params);
        let engine = toolPkgManager.getToolPkgExecutionEngine(context, &resolvedContextKey);
        Ok(engine.executeScriptFunction(&script, functionName, &params, onIntermediateResult))
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
