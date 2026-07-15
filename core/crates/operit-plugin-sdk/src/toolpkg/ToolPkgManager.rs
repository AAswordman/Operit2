use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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

/// Manages loaded ToolPkg runtimes, resources, listeners, and execution engines.
#[derive(Clone)]
pub struct ToolPkgManager {
    containers: BTreeMap<String, ToolPkgContainerRuntime>,
    subpackageByPackageName: BTreeMap<String, ToolPkgSubpackageRuntime>,
    runtimeChangeListeners: Arc<Mutex<Vec<ToolPkgRuntimeChangeListener>>>,
    toolPkgExecutionEngines: Arc<Mutex<BTreeMap<String, Arc<dyn JsExecutionEngine>>>>,
    executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
    assetSource: Arc<dyn ToolPkgAssetSource>,
}

impl ToolPkgManager {
    /// Creates a ToolPkg manager from application-supplied execution and asset interfaces.
    pub fn new(
        executionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
        assetSource: Arc<dyn ToolPkgAssetSource>,
    ) -> Self {
        Self {
            containers: BTreeMap::new(),
            subpackageByPackageName: BTreeMap::new(),
            runtimeChangeListeners: Arc::new(Mutex::new(Vec::new())),
            toolPkgExecutionEngines: Arc::new(Mutex::new(BTreeMap::new())),
            executionEngineFactory,
            assetSource,
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
        self.releaseToolPkgExecutionEngine(&format!("toolpkg_main:{}", runtime.packageName));
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

    /// Returns a cached execution engine or creates one for the supplied context key.
    #[allow(non_snake_case)]
    pub fn getToolPkgExecutionEngine(&self, contextKey: &str) -> Arc<dyn JsExecutionEngine> {
        let normalizedKey = match contextKey.trim() {
            "" => "toolpkg_main:default",
            value => value,
        };
        let mut engines = self
            .toolPkgExecutionEngines
            .lock()
            .expect("toolpkg execution engine mutex poisoned");
        if let Some(engine) = engines.get(normalizedKey) {
            return engine.clone();
        }
        let engine = self.executionEngineFactory.createToolPkgExecutionEngine();
        engines.insert(normalizedKey.to_string(), engine.clone());
        engine
    }

    /// Finds a cached ToolPkg execution engine without creating one.
    #[allow(non_snake_case)]
    pub fn findToolPkgExecutionEngine(
        &self,
        contextKey: &str,
    ) -> Option<Arc<dyn JsExecutionEngine>> {
        let normalizedKey = contextKey.trim();
        if normalizedKey.is_empty() {
            return None;
        }
        self.toolPkgExecutionEngines
            .lock()
            .expect("toolpkg execution engine mutex poisoned")
            .get(normalizedKey)
            .cloned()
    }

    /// Releases a cached ToolPkg JavaScript execution engine.
    #[allow(non_snake_case)]
    pub fn releaseToolPkgExecutionEngine(&self, contextKey: &str) {
        let normalizedKey = contextKey.trim();
        if normalizedKey.is_empty() {
            return;
        }
        if let Some(engine) = self
            .toolPkgExecutionEngines
            .lock()
            .expect("toolpkg execution engine mutex poisoned")
            .remove(normalizedKey)
        {
            engine.destroy();
        }
    }

    /// Destroys main and provider execution engines associated with one ToolPkg container.
    #[allow(non_snake_case)]
    pub fn destroyDefaultToolPkgExecutionEngine(&self, packageName: &str) {
        let normalizedPackageName = packageName.trim();
        if normalizedPackageName.is_empty() {
            return;
        }
        self.releaseToolPkgExecutionEngine(&format!("toolpkg_main:{normalizedPackageName}"));
        let providerPrefix = format!("toolpkg_provider:{normalizedPackageName}:");
        let keys = {
            let engines = self
                .toolPkgExecutionEngines
                .lock()
                .expect("toolpkg execution engine mutex poisoned");
            engines
                .keys()
                .filter(|key| key.starts_with(&providerPrefix))
                .cloned()
                .collect::<Vec<_>>()
        };
        for key in keys {
            self.releaseToolPkgExecutionEngine(&key);
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
        for (_, engine) in engines {
            engine.destroy();
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
            ToolPkgSourceType::EXTERNAL => {
                let sourcePath = PathBuf::from(&runtime.sourcePath);
                if sourcePath.is_dir() {
                    let bytes = fs::read(sourcePath.join(&normalizedResourcePath)).ok()?;
                    return crate::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok();
                }
                if sourcePath.is_file()
                    && sourcePath
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("toolpkg"))
                {
                    let file = fs::File::open(sourcePath).ok()?;
                    let mut archive = zip::ZipArchive::new(file).ok()?;
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
        let engine = self.getToolPkgExecutionEngine(&contextKey);
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
