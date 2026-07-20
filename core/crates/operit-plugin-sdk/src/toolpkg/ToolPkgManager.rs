use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::sync::{Arc, Mutex};

use operit_host_api::FileSystemHost;
use serde_json::Value;

use crate::javascript::JsExecutionEngine;
use crate::package::ToolPackage;
use crate::toolpkg::ToolPkgHooks::{ToolPkgHookDispatcher, ToolPkgHookInvocation};
use crate::toolpkg::ToolPkgParser::{
    ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType, ToolPkgSubpackageRuntime,
};

/// Listener notified when the set of active ToolPkg containers changes.
pub type ToolPkgRuntimeChangeListener = Arc<dyn Fn(Vec<ToolPkgContainerRuntime>) + Send + Sync>;

/// Creates JavaScript engines used to execute ToolPkg hooks and UI scripts.
pub trait ToolPkgExecutionEngineFactory: Send + Sync {
    /// Creates one isolated JavaScript engine for a ToolPkg execution context.
    #[allow(non_snake_case)]
    fn createToolPkgExecutionEngine(&self) -> Arc<dyn JsExecutionEngine>;
}

/// Resolves embedded ToolPkg archive bytes owned by the embedding application.
pub trait ToolPkgAssetSource: Send + Sync {
    /// Returns the bytes of one embedded ToolPkg archive by asset name.
    #[allow(non_snake_case)]
    fn toolPkgAssetBytes(&self, assetName: &str) -> Option<Vec<u8>>;
}

/// Associates one cached JavaScript engine with its owning ToolPkg container.
#[derive(Clone)]
struct ToolPkgExecutionEngineEntry {
    containerPackageName: String,
    engine: Arc<dyn JsExecutionEngine>,
}

/// Manages loaded ToolPkg runtimes, resources, listeners, and execution engines.
#[derive(Clone)]
pub struct ToolPkgManager {
    containers: BTreeMap<String, ToolPkgContainerRuntime>,
    subpackageByPackageName: BTreeMap<String, ToolPkgSubpackageRuntime>,
    runtimeChangeListeners: Arc<Mutex<Vec<ToolPkgRuntimeChangeListener>>>,
    toolPkgExecutionEngines: Arc<Mutex<BTreeMap<String, ToolPkgExecutionEngineEntry>>>,
    executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
    assetSource: Arc<dyn ToolPkgAssetSource>,
    fileSystemHost: Arc<dyn FileSystemHost>,
}

impl ToolPkgManager {
    /// Creates a ToolPkg manager from application-supplied execution and asset interfaces.
    pub fn new(
        executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
        assetSource: Arc<dyn ToolPkgAssetSource>,
        fileSystemHost: Arc<dyn FileSystemHost>,
    ) -> Self {
        Self {
            containers: BTreeMap::new(),
            subpackageByPackageName: BTreeMap::new(),
            runtimeChangeListeners: Arc::new(Mutex::new(Vec::new())),
            toolPkgExecutionEngines: Arc::new(Mutex::new(BTreeMap::new())),
            executionEngineFactory,
            assetSource,
            fileSystemHost,
        }
    }

    /// Replaces the JavaScript engine factory and destroys cached engines.
    #[allow(non_snake_case)]
    pub fn updateExecutionEngineFactory(
        &mut self,
        executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
    ) {
        self.destroy();
        self.executionEngineFactory = executionEngineFactory;
    }

    /// Returns whether a package name belongs to a registered ToolPkg container.
    #[allow(non_snake_case)]
    pub fn isToolPkgContainer(&self, packageName: &str) -> bool {
        self.containers.contains_key(packageName.trim())
    }

    /// Returns whether a package name resolves to a ToolPkg subpackage.
    #[allow(non_snake_case)]
    pub fn hasSubpackage(&self, packageName: &str) -> bool {
        self.subpackageByPackageName
            .contains_key(packageName.trim())
    }

    /// Returns all registered ToolPkg container runtimes in stable name order.
    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntimes(&self) -> Vec<ToolPkgContainerRuntime> {
        let mut runtimes = self.containers.values().cloned().collect::<Vec<_>>();
        runtimes.sort_by(|left, right| left.packageName.cmp(&right.packageName));
        runtimes
    }

    /// Returns the registered ToolPkg container map keyed by package name.
    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntimeMap(&self) -> BTreeMap<String, ToolPkgContainerRuntime> {
        self.containers.clone()
    }

    /// Returns the registered ToolPkg subpackage map keyed by package name.
    #[allow(non_snake_case)]
    pub fn getToolPkgSubpackageRuntimeMap(&self) -> BTreeMap<String, ToolPkgSubpackageRuntime> {
        self.subpackageByPackageName.clone()
    }

    /// Returns one registered ToolPkg container runtime by package name.
    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntime(
        &self,
        containerPackageName: &str,
    ) -> Option<ToolPkgContainerRuntime> {
        self.containers.get(containerPackageName.trim()).cloned()
    }

    /// Resolves one ToolPkg subpackage runtime by package name.
    #[allow(non_snake_case)]
    pub fn resolveToolPkgSubpackageRuntimeInternal(
        &self,
        packageName: &str,
    ) -> Option<ToolPkgSubpackageRuntime> {
        self.subpackageByPackageName
            .get(packageName.trim())
            .cloned()
    }

    /// Validates whether a loaded ToolPkg can be registered without name conflicts.
    #[allow(non_snake_case)]
    pub fn canRegisterToolPkg(
        &self,
        loadResult: &ToolPkgLoadResult,
        availablePackages: &BTreeMap<String, ToolPackage>,
    ) -> bool {
        let containerName = loadResult.containerPackage.name.trim();
        if containerName.is_empty()
            || self.containers.contains_key(containerName)
            || availablePackages.contains_key(containerName)
        {
            return false;
        }
        for subpackage in &loadResult.subpackagePackages {
            let packageName = subpackage.name.trim();
            if packageName.is_empty()
                || self.containers.contains_key(packageName)
                || availablePackages.contains_key(packageName)
                || self.subpackageByPackageName.contains_key(packageName)
            {
                return false;
            }
        }
        true
    }

    /// Registers a loaded ToolPkg and returns its executable subpackages.
    #[allow(non_snake_case)]
    pub fn registerToolPkg(&mut self, loadResult: ToolPkgLoadResult) -> Vec<ToolPackage> {
        let containerName = loadResult.containerPackage.name.clone();
        self.containers
            .insert(containerName, loadResult.containerRuntime.clone());
        for runtime in loadResult.containerRuntime.subpackages {
            self.subpackageByPackageName
                .insert(runtime.packageName.clone(), runtime);
        }
        loadResult.subpackagePackages
    }

    /// Removes a ToolPkg container and its subpackage runtime mappings.
    #[allow(non_snake_case)]
    pub fn removeToolPkgContainer(
        &mut self,
        containerPackageName: &str,
    ) -> Option<ToolPkgContainerRuntime> {
        let runtime = self.containers.remove(containerPackageName.trim())?;
        for subpackage in &runtime.subpackages {
            self.subpackageByPackageName
                .remove(subpackage.packageName.trim());
        }
        self.destroyToolPkgExecutionEngines(&runtime.packageName);
        Some(runtime)
    }

    /// Replaces all registered container and subpackage runtime maps.
    #[allow(non_snake_case)]
    pub fn replaceRuntimeMaps(
        &mut self,
        containers: BTreeMap<String, ToolPkgContainerRuntime>,
        subpackageByPackageName: BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) {
        self.containers = containers;
        self.subpackageByPackageName = subpackageByPackageName;
    }

    /// Returns the ToolPkg containers enabled directly or through a subpackage.
    #[allow(non_snake_case)]
    pub fn getEnabledToolPkgContainerRuntimes(
        &self,
        enabledPackageNames: &[String],
    ) -> Vec<ToolPkgContainerRuntime> {
        let enabledPackageNames = BTreeSet::from_iter(enabledPackageNames.iter().cloned());
        let mut runtimes = self
            .containers
            .values()
            .filter(|runtime| {
                enabledPackageNames.contains(&runtime.packageName)
                    || runtime
                        .subpackages
                        .iter()
                        .any(|subpackage| enabledPackageNames.contains(&subpackage.packageName))
            })
            .cloned()
            .collect::<Vec<_>>();
        runtimes.sort_by(|left, right| left.packageName.cmp(&right.packageName));
        runtimes
    }

    /// Registers a listener and immediately sends the current active containers.
    #[allow(non_snake_case)]
    pub fn addToolPkgRuntimeChangeListener(
        &self,
        listener: ToolPkgRuntimeChangeListener,
        activeContainers: Vec<ToolPkgContainerRuntime>,
    ) {
        {
            let mut listeners = self
                .runtimeChangeListeners
                .lock()
                .expect("toolpkg runtime listener mutex poisoned");
            listeners.push(listener.clone());
        }
        listener(activeContainers);
    }

    /// Notifies every registered runtime change listener.
    #[allow(non_snake_case)]
    pub fn notifyToolPkgRuntimeChangeListeners(
        &self,
        activeContainers: Vec<ToolPkgContainerRuntime>,
    ) {
        let listeners = self
            .runtimeChangeListeners
            .lock()
            .expect("toolpkg runtime listener mutex poisoned")
            .clone();
        for listener in listeners {
            listener(activeContainers.clone());
        }
    }

    /// Returns the main script of an enabled ToolPkg container.
    #[allow(non_snake_case)]
    pub fn getToolPkgMainScriptInternal(
        &self,
        containerPackageName: &str,
        enabledPackageNames: &[String],
    ) -> Option<String> {
        let normalizedContainerPackageName = containerPackageName.trim();
        let runtime = self.containers.get(normalizedContainerPackageName)?;
        let enabledPackageNames = BTreeSet::from_iter(enabledPackageNames.iter().cloned());
        let enabled = runtime.packageName.eq(normalizedContainerPackageName)
            && enabledPackageNames.contains(&runtime.packageName)
            || runtime
                .subpackages
                .iter()
                .any(|subpackage| enabledPackageNames.contains(&subpackage.packageName));
        if !enabled || runtime.mainEntry.trim().is_empty() {
            return None;
        }
        self.readToolPkgResourceText(runtime, &runtime.mainEntry)
    }

    /// Reads a text resource from an enabled ToolPkg container or subpackage.
    #[allow(non_snake_case)]
    pub fn readToolPkgTextResource(
        &self,
        packageNameOrSubpackageId: &str,
        resourcePath: &str,
        enabledPackageNames: &[String],
    ) -> Option<String> {
        let normalizedPackageName = packageNameOrSubpackageId.trim();
        let enabledPackageNames = BTreeSet::from_iter(enabledPackageNames.iter().cloned());
        let runtime = self.containers.get(normalizedPackageName).or_else(|| {
            let subpackage = self.subpackageByPackageName.get(normalizedPackageName)?;
            self.containers.get(&subpackage.containerPackageName)
        })?;
        let enabled = enabledPackageNames.contains(&runtime.packageName)
            || runtime
                .subpackages
                .iter()
                .any(|subpackage| enabledPackageNames.contains(&subpackage.packageName));
        if !enabled {
            return None;
        }
        self.readToolPkgResourceText(runtime, resourcePath)
    }

    /// Returns a cached execution engine or creates one with explicit container ownership.
    #[allow(non_snake_case)]
    pub fn getToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> Arc<dyn JsExecutionEngine> {
        let normalizedKey = contextKey.trim();
        let normalizedContainer = containerPackageName.trim();
        assert!(
            !normalizedKey.is_empty(),
            "ToolPkg execution context key is required"
        );
        assert!(
            !normalizedContainer.is_empty(),
            "ToolPkg execution container is required"
        );
        let mut engines = self
            .toolPkgExecutionEngines
            .lock()
            .expect("toolpkg execution engine mutex poisoned");
        if let Some(entry) = engines.get(normalizedKey) {
            assert_eq!(
                entry.containerPackageName, normalizedContainer,
                "ToolPkg execution context belongs to a different container"
            );
            return entry.engine.clone();
        }
        let engine = self.executionEngineFactory.createToolPkgExecutionEngine();
        engines.insert(
            normalizedKey.to_string(),
            ToolPkgExecutionEngineEntry {
                containerPackageName: normalizedContainer.to_string(),
                engine: engine.clone(),
            },
        );
        engine
    }

    /// Finds a cached ToolPkg execution engine without creating one.
    #[allow(non_snake_case)]
    pub fn findToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> Option<Arc<dyn JsExecutionEngine>> {
        let normalizedKey = contextKey.trim();
        let normalizedContainer = containerPackageName.trim();
        if normalizedKey.is_empty() || normalizedContainer.is_empty() {
            return None;
        }
        let engines = self
            .toolPkgExecutionEngines
            .lock()
            .expect("toolpkg execution engine mutex poisoned");
        let entry = engines.get(normalizedKey)?;
        assert_eq!(
            entry.containerPackageName, normalizedContainer,
            "ToolPkg execution context belongs to a different container"
        );
        Some(entry.engine.clone())
    }

    /// Releases a cached ToolPkg JavaScript execution engine.
    #[allow(non_snake_case)]
    pub fn releaseToolPkgExecutionEngine(&self, contextKey: &str, containerPackageName: &str) {
        let normalizedKey = contextKey.trim();
        let normalizedContainer = containerPackageName.trim();
        if normalizedKey.is_empty() || normalizedContainer.is_empty() {
            return;
        }
        let removed = {
            let mut engines = self
                .toolPkgExecutionEngines
                .lock()
                .expect("toolpkg execution engine mutex poisoned");
            if let Some(entry) = engines.get(normalizedKey) {
                assert_eq!(
                    entry.containerPackageName, normalizedContainer,
                    "ToolPkg execution context belongs to a different container"
                );
            }
            engines.remove(normalizedKey)
        };
        if let Some(entry) = removed {
            entry.engine.destroy();
        }
    }

    /// Destroys every execution engine owned by one ToolPkg container.
    #[allow(non_snake_case)]
    pub fn destroyToolPkgExecutionEngines(&self, containerPackageName: &str) {
        let normalizedContainer = containerPackageName.trim();
        if normalizedContainer.is_empty() {
            return;
        }
        let removed = {
            let mut engines = self
                .toolPkgExecutionEngines
                .lock()
                .expect("toolpkg execution engine mutex poisoned");
            let keys = engines
                .iter()
                .filter(|(_, entry)| entry.containerPackageName == normalizedContainer)
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>();
            keys.into_iter()
                .filter_map(|key| engines.remove(&key))
                .collect::<Vec<_>>()
        };
        for entry in removed {
            entry.engine.destroy();
        }
    }

    /// Removes every registered ToolPkg runtime while preserving listeners.
    #[allow(non_snake_case)]
    pub fn clear(&mut self) {
        self.containers.clear();
        self.subpackageByPackageName.clear();
    }

    /// Destroys all cached ToolPkg JavaScript execution engines.
    pub fn destroy(&self) {
        let engines = {
            let mut stored = self
                .toolPkgExecutionEngines
                .lock()
                .expect("toolpkg execution engine mutex poisoned");
            std::mem::take(&mut *stored)
        };
        for (_, entry) in engines {
            entry.engine.destroy();
        }
    }

    /// Reads one ToolPkg resource as UTF-8 text.
    #[allow(non_snake_case)]
    fn readToolPkgResourceText(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<String> {
        let bytes = self.readToolPkgResourceBytes(runtime, resourcePath)?;
        crate::toolpkg::ToolPkgProtection::decodeUtf8(&bytes).ok()
    }

    /// Reads one ToolPkg resource as raw bytes.
    #[allow(non_snake_case)]
    pub fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<Vec<u8>> {
        let normalizedResourcePath = normalizeToolPkgEntryPath(resourcePath)?;
        match runtime.sourceType {
            ToolPkgSourceType::EXTERNAL | ToolPkgSourceType::MARKET => {
                let sourceInfo = self.fileSystemHost.fileExists(&runtime.sourcePath).ok()?;
                if sourceInfo.isDirectory {
                    let resourceFile =
                        joinToolPkgSourcePath(&runtime.sourcePath, &normalizedResourcePath);
                    let bytes = self.fileSystemHost.readFileBytes(&resourceFile).ok()?;
                    return crate::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok();
                }
                if sourceInfo.exists
                    && runtime
                        .sourcePath
                        .to_ascii_lowercase()
                        .ends_with(".toolpkg")
                {
                    let archiveBytes = self
                        .fileSystemHost
                        .readFileBytes(&runtime.sourcePath)
                        .ok()?;
                    let mut archive =
                        zip::ZipArchive::new(std::io::Cursor::new(archiveBytes)).ok()?;
                    let mut entry = archive.by_name(&normalizedResourcePath).ok()?;
                    let mut bytes = Vec::new();
                    entry.read_to_end(&mut bytes).ok()?;
                    return crate::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok();
                }
                None
            }
            ToolPkgSourceType::ASSET => {
                let assetBytes = self.assetSource.toolPkgAssetBytes(&runtime.sourcePath)?;
                let cursor = std::io::Cursor::new(assetBytes);
                let mut archive = zip::ZipArchive::new(cursor).ok()?;
                let mut entry = archive.by_name(&normalizedResourcePath).ok()?;
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).ok()?;
                crate::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok()
            }
        }
    }
}

/// Joins a normalized ToolPkg entry beneath one host-owned source directory.
fn joinToolPkgSourcePath(sourcePath: &str, entryPath: &str) -> String {
    format!("{}/{}", sourcePath.trim_end_matches(['/', '\\']), entryPath)
}

impl ToolPkgHookDispatcher for ToolPkgManager {
    /// Invokes one ToolPkg hook using its container main script and execution context.
    #[allow(non_snake_case)]
    fn dispatchToolPkgHook(
        &self,
        enabledPackageNames: &[String],
        invocation: ToolPkgHookInvocation,
    ) -> Result<Option<String>, String> {
        let containerPackageName = invocation.containerPackageName.trim();
        let runtime = self
            .containers
            .get(containerPackageName)
            .cloned()
            .ok_or_else(|| format!("ToolPkg container not found: {containerPackageName}"))?;
        let script = self
            .getToolPkgMainScriptInternal(&runtime.packageName, enabledPackageNames)
            .ok_or_else(|| {
                format!(
                    "ToolPkg main script is unavailable: {}",
                    runtime.packageName
                )
            })?;

        let eventName = invocation
            .eventName
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(invocation.event.as_str());
        let mut params = BTreeMap::<String, Value>::new();
        params.insert("event".to_string(), Value::String(eventName.to_string()));
        params.insert(
            "eventName".to_string(),
            Value::String(eventName.to_string()),
        );
        params.insert("eventPayload".to_string(), invocation.eventPayload.clone());
        params.insert(
            "timestampMs".to_string(),
            Value::Number(serde_json::Number::from(invocation.timestampMs)),
        );
        params.insert(
            "functionName".to_string(),
            Value::String(invocation.functionName.clone()),
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
        if let Some(pluginId) = invocation
            .pluginId
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert("pluginId".to_string(), Value::String(pluginId.to_string()));
        }
        if let Some(chatId) = invocation
            .eventPayload
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
        if let Some(functionSource) = invocation
            .inlineFunctionSource
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_inline_function_name".to_string(),
                Value::String(invocation.functionName.clone()),
            );
            params.insert(
                "__operit_inline_function_source".to_string(),
                Value::String(functionSource.to_string()),
            );
        }
        if let Some(contextKey) = invocation
            .executionContextKey
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_execution_context_key".to_string(),
                Value::String(contextKey.to_string()),
            );
        }
        if let Some(runtimeKind) = invocation
            .runtimeKind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            params.insert(
                "__operit_toolpkg_runtime_kind".to_string(),
                Value::String(runtimeKind.to_ascii_lowercase()),
            );
        }

        let contextKey = resolveToolPkgExecutionContextKey(&runtime.packageName, &params);
        let engine = self.getToolPkgExecutionEngine(&contextKey, &runtime.packageName);
        engine
            .execute_script_function(
                &script,
                &invocation.functionName,
                &params,
                &invocation.envOverrides,
                invocation.onIntermediateResult,
                invocation.dispatchIntermediateOnMain,
                invocation.timeoutSec,
            )
            .map_err(|error| error.to_string())
    }
}

/// Resolves the execution context key encoded in hook parameters.
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

/// Normalizes a ToolPkg entry path and rejects parent-directory traversal.
#[allow(non_snake_case)]
fn normalizeToolPkgEntryPath(rawPath: &str) -> Option<String> {
    let normalized = rawPath
        .trim()
        .replace('\\', "/")
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>()
        .join("/");
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.split('/').any(|segment| segment == "..")
    {
        return None;
    }
    Some(normalized)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;
    use crate::execution_result::JsExecutionResult;
    use crate::javascript::ToolPkgMainRegistrationCapture;
    use operit_host_api::{
        FileEntry, FileExistence, FileInfo, FindFilesRequest, GrepCodeRequest, GrepCodeResult,
        HostEnvironmentDescriptor, HostError, HostResult,
    };

    /// Rejects filesystem operations because these engine lifecycle tests do not access files.
    struct RejectingFileSystemHost;

    impl RejectingFileSystemHost {
        /// Returns the explicit error used for unsupported test filesystem operations.
        fn unsupported<T>() -> HostResult<T> {
            Err(HostError::new("filesystem access is not used by this test"))
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
        fn validatePath(&self, _path: &str, _paramName: &str) -> HostResult<()> {
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
        fn readFileWithLimit(&self, _path: &str, _maxBytes: usize) -> HostResult<String> {
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
        fn makeDirectory(&self, _path: &str, _createParents: bool) -> HostResult<()> {
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

    /// Records whether a test execution engine was destroyed.
    #[derive(Default)]
    struct RecordingExecutionEngine {
        destroyed: AtomicBool,
    }

    impl JsExecutionEngine for RecordingExecutionEngine {
        /// Returns no script result for registry tests.
        fn execute_script_function(
            &self,
            _script: &str,
            _function_name: &str,
            _params: &BTreeMap<String, Value>,
            _env_overrides: &BTreeMap<String, String>,
            _on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
            _dispatch_intermediate_on_main: bool,
            _timeout_sec: u64,
        ) -> JsExecutionResult<Option<String>> {
            Ok(None)
        }

        /// Returns an empty ToolPkg registration capture for registry tests.
        fn execute_toolpkg_main_registration_function_with_text_resources(
            &self,
            _script: &str,
            _function_name: &str,
            _params: &BTreeMap<String, Value>,
            _text_resources: Option<Arc<BTreeMap<String, String>>>,
        ) -> JsExecutionResult<ToolPkgMainRegistrationCapture> {
            Ok(ToolPkgMainRegistrationCapture::default())
        }

        /// Returns no Compose DSL result for registry tests.
        fn execute_compose_dsl_script(
            &self,
            _script: &str,
            _runtime_options: &BTreeMap<String, Value>,
            _env_overrides: &BTreeMap<String, String>,
        ) -> JsExecutionResult<Option<String>> {
            Ok(None)
        }

        /// Returns no Compose DSL action result for registry tests.
        fn dispatch_compose_dsl_action(
            &self,
            _action_id: &str,
            _payload: Option<Value>,
            _runtime_options: &BTreeMap<String, Value>,
            _env_overrides: &BTreeMap<String, String>,
            _on_intermediate_result: Option<Arc<dyn Fn(String) + Send + Sync>>,
        ) -> JsExecutionResult<Option<String>> {
            Ok(None)
        }

        /// Marks this test execution engine as destroyed.
        fn destroy(&self) {
            self.destroyed.store(true, Ordering::Release);
        }
    }

    /// Creates recording engines and retains them for lifecycle assertions.
    #[derive(Default)]
    struct RecordingExecutionEngineFactory {
        engines: Mutex<Vec<Arc<RecordingExecutionEngine>>>,
    }

    impl ToolPkgExecutionEngineFactory for RecordingExecutionEngineFactory {
        /// Creates one recording execution engine.
        #[allow(non_snake_case)]
        fn createToolPkgExecutionEngine(&self) -> Arc<dyn JsExecutionEngine> {
            let engine = Arc::new(RecordingExecutionEngine::default());
            self.engines
                .lock()
                .expect("recording engine factory mutex poisoned")
                .push(engine.clone());
            engine
        }
    }

    /// Supplies no assets because registry tests only exercise engine ownership.
    struct EmptyAssetSource;

    impl ToolPkgAssetSource for EmptyAssetSource {
        /// Returns no embedded ToolPkg archive.
        #[allow(non_snake_case)]
        fn toolPkgAssetBytes(&self, _assetName: &str) -> Option<Vec<u8>> {
            None
        }
    }

    /// Creates one manager and its recording engine factory.
    fn recordingManager() -> (ToolPkgManager, Arc<RecordingExecutionEngineFactory>) {
        let factory = Arc::new(RecordingExecutionEngineFactory::default());
        let manager = ToolPkgManager::new(
            factory.clone(),
            Arc::new(EmptyAssetSource),
            Arc::new(RejectingFileSystemHost),
        );
        (manager, factory)
    }

    /// Verifies an opaque context key cannot be reused under a different container owner.
    #[test]
    #[should_panic(expected = "belongs to a different container")]
    fn rejectsContextOwnershipMismatch() {
        let (manager, _) = recordingManager();
        manager.getToolPkgExecutionEngine("opaque-main-context", "package_a");
        manager.getToolPkgExecutionEngine("opaque-main-context", "package_b");
    }

    /// Verifies container cleanup covers main, provider, and UI execution contexts.
    #[test]
    fn destroysEveryContextOwnedByContainer() {
        let (manager, factory) = recordingManager();
        manager.getToolPkgExecutionEngine("package-a-main", "package_a");
        manager.getToolPkgExecutionEngine("package-a-xml-node", "package_a");
        manager.getToolPkgExecutionEngine("package-b-provider", "package_b");

        manager.destroyToolPkgExecutionEngines("package_a");

        let engines = factory
            .engines
            .lock()
            .expect("recording engine factory mutex poisoned");
        assert!(engines[0].destroyed.load(Ordering::Acquire));
        assert!(engines[1].destroyed.load(Ordering::Acquire));
        assert!(!engines[2].destroyed.load(Ordering::Acquire));
        assert!(manager
            .findToolPkgExecutionEngine("package-a-main", "package_a")
            .is_none());
        assert!(manager
            .findToolPkgExecutionEngine("package-b-provider", "package_b")
            .is_some());
    }
}
