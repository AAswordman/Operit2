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
        let output =
            engine.executeScriptFunction(&script, functionName, &params, onIntermediateResult);
        Ok(output)
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

#[cfg(test)]
mod tests {
    use super::PackageManagerToolPkgFacade;
    use crate::core::application::OperitApplicationContext::OperitApplicationContext;
    use crate::core::tools::packTool::ToolPkgManager::ToolPkgManager;
    use crate::core::tools::packTool::ToolPkgParser::{
        ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType,
    };
    use crate::core::tools::ToolPackage::ToolPackage;
    use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
    use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
    use operit_store::RuntimeStorePaths::setDefaultRuntimeStoreRoot;
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::path::{Component, Path, PathBuf};
    use std::sync::Arc;

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

    fn test_runtime_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "operit-toolpkg-facade-tests-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("test runtime root");
        let host = Arc::new(TestRuntimeStorageHost::new(root.clone()));
        setDefaultRuntimeStoreRoot(root.clone());
        setDefaultRuntimeStorageHost(host);
        root
    }

    #[test]
    fn run_toolpkg_main_hook_executes_inline_function_source() {
        let root = test_runtime_root("inline-hook");
        let sourceDir = root.join("toolpkg");
        let distDir = sourceDir.join("dist");
        std::fs::create_dir_all(&distDir).expect("toolpkg dist dir");
        std::fs::write(
            distDir.join("main.js"),
            r#"
                exports.registeredOnly = function(_params) {
                    return "registered";
                };
            "#,
        )
        .expect("toolpkg main script");
        let mut toolPkgManager = ToolPkgManager::new();
        let loadResult = ToolPkgLoadResult {
            containerPackage: ToolPackage {
                name: "inline_hook_pkg".to_string(),
                ..ToolPackage::default()
            },
            subpackagePackages: Vec::new(),
            containerRuntime: ToolPkgContainerRuntime {
                packageName: "inline_hook_pkg".to_string(),
                mainEntry: "dist/main.js".to_string(),
                sourceType: ToolPkgSourceType::EXTERNAL,
                sourcePath: sourceDir.to_string_lossy().to_string(),
                ..ToolPkgContainerRuntime::default()
            },
        };
        assert!(toolPkgManager.canRegisterToolPkg(&loadResult, &BTreeMap::new()));
        toolPkgManager.registerToolPkg(loadResult);

        let output = PackageManagerToolPkgFacade::runToolPkgMainHook(
            &toolPkgManager,
            &OperitApplicationContext::new(),
            &["inline_hook_pkg".to_string()],
            "inline_hook_pkg",
            "inlinePromptHook",
            "system_prompt_compose",
            Some("system_prompt_compose"),
            Some("hook-id"),
            Some(
                r#"function(params) {
                    return {
                        systemPrompt: [
                            params.eventName,
                            params.eventPayload.chatId,
                            params.toolPkgId,
                            params.__operit_script_screen,
                            params.pluginId
                        ].join('|')
                    };
                }"#,
            ),
            json!({ "chatId": "chat-1" }),
            None,
            Some("sandbox"),
            None,
        )
        .expect("toolpkg main hook")
        .expect("hook result");

        assert_eq!(
            output,
            r#"{"systemPrompt":"system_prompt_compose|chat-1|inline_hook_pkg|dist/main.js|hook-id"}"#
        );
    }
}
