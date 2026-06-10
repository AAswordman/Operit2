use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use crate::core::tools::packTool::PackageManager::PackageManager;
use crate::core::tools::packTool::ToolPkgCommonPluginConstants::TOOLPKG_RUNTIME_COMPOSE_DSL;
use crate::core::tools::packTool::ToolPkgParser::{ToolPkgArchiveParser, ToolPkgSubpackageRuntime};

pub struct PackageManagerToolPkgFacade<'a> {
    packageManager: &'a PackageManager,
}

impl<'a> PackageManagerToolPkgFacade<'a> {
    #[allow(non_snake_case)]
    pub fn new(packageManager: &'a PackageManager) -> Self {
        Self { packageManager }
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgComposeDslScriptBySubpackageId(
        &self,
        subpackageId: &str,
        uiModuleId: Option<&str>,
        preferEnabledContainer: bool,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        if subpackageId.trim().is_empty() {
            return None;
        }

        let directSubpackage = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(subpackageId);
        let subpackages = if let Some(directSubpackage) = directSubpackage {
            vec![directSubpackage]
        } else {
            self.packageManager
                .toolPkgSubpackageByPackageNameInternal()
                .values()
                .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(subpackageId))
                .cloned()
                .collect::<Vec<_>>()
        };

        if subpackages.is_empty() {
            return None;
        }

        let candidateContainers = if preferEnabledContainer {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            distinctContainerNames(
                subpackages
                    .iter()
                    .filter(|subpackage| enabledSet.contains(&subpackage.containerPackageName)),
            )
        } else {
            distinctContainerNames(subpackages.iter())
        };

        for containerName in candidateContainers {
            let script = self.getToolPkgComposeDslScript(&containerName, uiModuleId);
            if let Some(script) = script.filter(|script| !script.trim().is_empty()) {
                return Some(script);
            }
        }

        None
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgComposeDslScript(
        &self,
        containerPackageName: &str,
        uiModuleId: Option<&str>,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        let normalizedContainerPackageName = self
            .packageManager
            .normalizePackageName(containerPackageName);
        let runtime = self
            .packageManager
            .toolPkgContainersInternal()
            .get(&normalizedContainerPackageName)
            .cloned()?;
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        if !enabledSet.contains(&runtime.packageName) {
            return None;
        }

        let uiModule =
            if let Some(uiModuleId) = uiModuleId.map(str::trim).filter(|value| !value.is_empty()) {
                runtime.uiModules.iter().find(|module| {
                    module.id.eq_ignore_ascii_case(uiModuleId)
                        && module
                            .runtime
                            .eq_ignore_ascii_case(TOOLPKG_RUNTIME_COMPOSE_DSL)
                })
            } else {
                runtime.uiModules.iter().find(|module| {
                    module
                        .runtime
                        .eq_ignore_ascii_case(TOOLPKG_RUNTIME_COMPOSE_DSL)
                })
            }?;

        if uiModule.screen.trim().is_empty() {
            return None;
        }

        let bytes = self
            .packageManager
            .readToolPkgResourceBytes(&runtime, &uiModule.screen)?;
        String::from_utf8(bytes).ok()
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgComposeDslScreenPath(
        &self,
        containerPackageName: &str,
        uiModuleId: Option<&str>,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        let normalizedContainerPackageName = self
            .packageManager
            .normalizePackageName(containerPackageName);
        let runtime = self
            .packageManager
            .toolPkgContainersInternal()
            .get(&normalizedContainerPackageName)
            .cloned()?;
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        if !enabledSet.contains(&runtime.packageName) {
            return None;
        }

        let uiModule =
            if let Some(uiModuleId) = uiModuleId.map(str::trim).filter(|value| !value.is_empty()) {
                runtime.uiModules.iter().find(|module| {
                    module.id.eq_ignore_ascii_case(uiModuleId)
                        && module
                            .runtime
                            .eq_ignore_ascii_case(TOOLPKG_RUNTIME_COMPOSE_DSL)
                })
            } else {
                runtime.uiModules.iter().find(|module| {
                    module
                        .runtime
                        .eq_ignore_ascii_case(TOOLPKG_RUNTIME_COMPOSE_DSL)
                })
            }?;

        let screen = uiModule.screen.trim().to_string();
        if screen.is_empty() {
            return None;
        }
        Some(screen)
    }

    #[allow(non_snake_case)]
    pub fn runToolPkgMainHook(
        &self,
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
        let normalizedContainerPackageName = self
            .packageManager
            .normalizePackageName(containerPackageName);
        let runtime = self
            .packageManager
            .toolPkgContainersInternal()
            .get(&normalizedContainerPackageName)
            .cloned()
            .ok_or_else(|| {
                format!("ToolPkg container not found: {normalizedContainerPackageName}")
            })?;
        let script = self
            .packageManager
            .getToolPkgMainScriptInternal(&runtime.packageName)
            .ok_or_else(|| {
                format!(
                    "ToolPkg main script is unavailable: {}",
                    runtime.packageName
                )
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
            Value::String(runtime.packageName.clone()),
        );
        params.insert(
            "containerPackageName".to_string(),
            Value::String(runtime.packageName.clone()),
        );
        params.insert(
            "__operit_ui_package_name".to_string(),
            Value::String(runtime.packageName.clone()),
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

        let resolvedContextKey = resolveToolPkgExecutionContextKey(&runtime.packageName, &params);
        let engine = self
            .packageManager
            .getToolPkgExecutionEngine(&resolvedContextKey);
        let output = engine.executeScriptFunction(
            &script,
            functionName,
            &params,
            &BTreeMap::new(),
            onIntermediateResult,
            true,
            60,
            None,
        );
        Ok(output)
    }

    #[allow(non_snake_case)]
    pub fn copyToolPkgResourceToFileBySubpackageId(
        &self,
        subpackageId: &str,
        resourceKey: &str,
        destinationFile: &Path,
        preferEnabledContainer: bool,
    ) -> bool {
        self.packageManager.ensureInitialized();
        if subpackageId.trim().is_empty() || resourceKey.trim().is_empty() {
            return false;
        }

        let directSubpackage = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(subpackageId);
        let subpackages = if let Some(directSubpackage) = directSubpackage {
            vec![directSubpackage]
        } else {
            self.packageManager
                .toolPkgSubpackageByPackageNameInternal()
                .values()
                .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(subpackageId))
                .cloned()
                .collect::<Vec<_>>()
        };

        if subpackages.is_empty() {
            return false;
        }

        let candidateContainers = if preferEnabledContainer {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            let enabledContainers = distinctContainerNames(
                subpackages
                    .iter()
                    .filter(|subpackage| enabledSet.contains(&subpackage.containerPackageName)),
            );
            if !enabledContainers.is_empty() {
                enabledContainers
            } else {
                distinctContainerNames(subpackages.iter())
            }
        } else {
            distinctContainerNames(subpackages.iter())
        };

        for containerName in candidateContainers {
            if self.copyToolPkgResourceToFile(&containerName, resourceKey, destinationFile) {
                return true;
            }
        }

        false
    }

    #[allow(non_snake_case)]
    pub fn copyToolPkgResourceToFile(
        &self,
        containerPackageName: &str,
        resourceKey: &str,
        destinationFile: &Path,
    ) -> bool {
        self.packageManager.ensureInitialized();
        let normalizedContainerPackageName = self
            .packageManager
            .normalizePackageName(containerPackageName);
        let toolPkgContainers = self.packageManager.toolPkgContainersInternal();
        let Some(runtime) = toolPkgContainers.get(&normalizedContainerPackageName) else {
            return false;
        };
        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        if !enabledSet.contains(&runtime.packageName) {
            return false;
        }
        let Some(resource) = runtime
            .resources
            .iter()
            .find(|resource| resource.key.eq_ignore_ascii_case(resourceKey))
        else {
            return false;
        };

        self.packageManager
            .exportToolPkgResource(runtime, resource, destinationFile)
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgResourceOutputFileName(
        &self,
        packageNameOrSubpackageId: &str,
        resourceKey: &str,
        preferEnabledContainer: bool,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        let target = packageNameOrSubpackageId.trim();
        let key = resourceKey.trim();
        if target.is_empty() || key.is_empty() {
            return None;
        }

        let toolPkgContainers = self.packageManager.toolPkgContainersInternal();
        let resolveFromContainer = |containerName: &str| -> Option<String> {
            let normalizedContainerName = self.packageManager.normalizePackageName(containerName);
            let runtime = toolPkgContainers.get(&normalizedContainerName)?;
            let resource = runtime
                .resources
                .iter()
                .find(|resource| resource.key.eq_ignore_ascii_case(key))?;
            let baseName = resource
                .path
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or_default()
                .trim();
            if baseName.is_empty() {
                return None;
            }
            if ToolPkgArchiveParser::isDirectoryResourceMime(Some(&resource.mime)) {
                if baseName.to_ascii_lowercase().ends_with(".zip") {
                    Some(baseName.to_string())
                } else {
                    Some(format!("{baseName}.zip"))
                }
            } else {
                Some(baseName.to_string())
            }
        };

        if let Some(outputFileName) = resolveFromContainer(target) {
            return Some(outputFileName);
        }

        let directSubpackage = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(target);
        if let Some(directSubpackage) = directSubpackage {
            if let Some(outputFileName) =
                resolveFromContainer(&directSubpackage.containerPackageName)
            {
                return Some(outputFileName);
            }
        }

        let subpackages = self
            .packageManager
            .toolPkgSubpackageByPackageNameInternal()
            .values()
            .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(target))
            .cloned()
            .collect::<Vec<_>>();
        if subpackages.is_empty() {
            return None;
        }

        let candidateContainers = if preferEnabledContainer {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            let enabledContainers = distinctContainerNames(
                subpackages
                    .iter()
                    .filter(|subpackage| enabledSet.contains(&subpackage.containerPackageName)),
            );
            if !enabledContainers.is_empty() {
                enabledContainers
            } else {
                distinctContainerNames(subpackages.iter())
            }
        } else {
            distinctContainerNames(subpackages.iter())
        };

        for containerName in candidateContainers {
            if let Some(outputFileName) = resolveFromContainer(&containerName) {
                return Some(outputFileName);
            }
        }

        None
    }

    #[allow(non_snake_case)]
    pub fn readToolPkgTextResource(
        &self,
        packageNameOrSubpackageId: &str,
        resourcePath: &str,
    ) -> Option<String> {
        self.packageManager.ensureInitialized();
        let target = packageNameOrSubpackageId.trim();
        let normalizedPath = resourcePath
            .trim()
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string();

        if target.is_empty() || normalizedPath.is_empty() {
            return None;
        }

        let toolPkgContainers = self.packageManager.toolPkgContainersInternal();
        if let Some(containerRuntime) = toolPkgContainers.get(target).cloned() {
            let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
            if !enabledSet.contains(&containerRuntime.packageName) {
                return None;
            }
            return self
                .packageManager
                .readToolPkgResourceBytes(&containerRuntime, &normalizedPath)
                .and_then(|bytes| String::from_utf8(bytes).ok());
        }

        let directSubpackageRuntime = self
            .packageManager
            .resolveToolPkgSubpackageRuntimeInternal(target);
        if let Some(directSubpackageRuntime) = directSubpackageRuntime {
            if let Some(directContainer) = toolPkgContainers
                .get(&directSubpackageRuntime.containerPackageName)
                .cloned()
            {
                let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
                if !enabledSet.contains(&directContainer.packageName) {
                    return None;
                }
                return self
                    .packageManager
                    .readToolPkgResourceBytes(&directContainer, &normalizedPath)
                    .and_then(|bytes| String::from_utf8(bytes).ok());
            }
        }

        let subpackages = self
            .packageManager
            .toolPkgSubpackageByPackageNameInternal()
            .values()
            .filter(|subpackage| subpackage.subpackageId.eq_ignore_ascii_case(target))
            .cloned()
            .collect::<Vec<_>>();
        if subpackages.is_empty() {
            return None;
        }

        let enabledSet = self.packageManager.getEnabledPackageNameSetInternal();
        let candidateContainers = distinctContainerNames(
            subpackages
                .iter()
                .filter(|subpackage| enabledSet.contains(&subpackage.containerPackageName)),
        );

        for containerName in candidateContainers {
            let Some(runtime) = toolPkgContainers.get(&containerName) else {
                continue;
            };
            let text = self
                .packageManager
                .readToolPkgResourceBytes(runtime, &normalizedPath)
                .and_then(|bytes| String::from_utf8(bytes).ok());
            if let Some(text) = text.filter(|text| !text.is_empty()) {
                return Some(text);
            }
        }

        None
    }
}

#[allow(non_snake_case)]
fn distinctContainerNames<'a, I>(subpackages: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a ToolPkgSubpackageRuntime>,
{
    let mut names = Vec::<String>::new();
    for subpackage in subpackages {
        if !names.contains(&subpackage.containerPackageName) {
            names.push(subpackage.containerPackageName.clone());
        }
    }
    names
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
    use crate::core::tools::packTool::PackageManager::PackageManager;
    use crate::core::tools::packTool::ToolPkgParser::{
        ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType,
    };
    use crate::core::tools::ToolPackage::ToolPackage;
    use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
    use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
    use operit_store::RuntimeStorePaths::setDefaultRuntimeStoreRoot;
    use operit_store::RuntimeStorePaths::RuntimeStorePaths;
    use serde_json::json;
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
        let mut packageManager = PackageManager::new(RuntimeStorePaths::new(root.clone()));
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
        assert!(packageManager.registerToolPkg(loadResult));
        packageManager
            .setEnabledPackageNames(&["inline_hook_pkg".to_string()])
            .expect("enabled package names");

        let output = PackageManagerToolPkgFacade::new(&packageManager)
            .runToolPkgMainHook(
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
