use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::HostManager;
use operit_tools::tools::javascript::{jsBridgeSupport, JsExecutionEngine};
use operit_tools::tools::packTool::ToolPkgParser::{
    ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgSourceType, ToolPkgSubpackageRuntime,
};
use operit_tools::tools::ToolPackage::ToolPackage;
use crate::runtime_support::{toolRuntimeSupport, RuntimePluginAsset};

pub type ToolPkgRuntimeChangeListener = Arc<dyn Fn(Vec<ToolPkgContainerRuntime>) + Send + Sync>;

#[derive(Clone)]
pub struct ToolPkgManager {
    context: HostManager,
    containers: BTreeMap<String, ToolPkgContainerRuntime>,
    subpackageByPackageName: BTreeMap<String, ToolPkgSubpackageRuntime>,
    runtimeChangeListeners: Arc<Mutex<Vec<ToolPkgRuntimeChangeListener>>>,
    toolPkgExecutionEngines: Arc<Mutex<BTreeMap<String, Arc<dyn JsExecutionEngine>>>>,
}

impl Default for ToolPkgManager {
    fn default() -> Self {
        Self::new(HostManager::new())
    }
}

impl ToolPkgManager {
    /// Creates a ToolPkg manager for an application context.
    pub fn new(context: HostManager) -> Self {
        Self {
            context,
            containers: BTreeMap::new(),
            subpackageByPackageName: BTreeMap::new(),
            runtimeChangeListeners: Arc::new(Mutex::new(Vec::new())),
            toolPkgExecutionEngines: Arc::new(Mutex::new(BTreeMap::new())),
        }
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

    #[allow(non_snake_case)]
    pub(crate) fn getToolPkgContainerRuntimeMap(
        &self,
    ) -> BTreeMap<String, ToolPkgContainerRuntime> {
        self.containers.clone()
    }

    #[allow(non_snake_case)]
    pub(crate) fn getToolPkgSubpackageRuntimeMap(
        &self,
    ) -> BTreeMap<String, ToolPkgSubpackageRuntime> {
        self.subpackageByPackageName.clone()
    }

    #[allow(non_snake_case)]
    pub fn getToolPkgContainerRuntime(
        &self,
        containerPackageName: &str,
    ) -> Option<ToolPkgContainerRuntime> {
        self.containers.get(containerPackageName.trim()).cloned()
    }

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

    #[allow(non_snake_case)]
    pub(crate) fn replaceRuntimeMaps(
        &mut self,
        containers: BTreeMap<String, ToolPkgContainerRuntime>,
        subpackageByPackageName: BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) {
        self.containers = containers;
        self.subpackageByPackageName = subpackageByPackageName;
    }

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

    #[allow(non_snake_case)]
    pub(crate) fn getToolPkgExecutionEngine(
        &self,
        contextKey: &str,
    ) -> Arc<dyn JsExecutionEngine> {
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
        let toolHandler =
            operit_tools::tools::AIToolHandler::AIToolHandler::getInstance(self.context.clone());
        let engine = jsBridgeSupport().newToolPkgExecutionEngine(toolHandler);
        engines.insert(normalizedKey.to_string(), engine.clone());
        engine
    }

    #[allow(non_snake_case)]
    pub(crate) fn findToolPkgExecutionEngine(
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

    #[allow(non_snake_case)]
    fn readToolPkgResourceText(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<String> {
        let bytes = self.readToolPkgResourceBytes(runtime, resourcePath)?;
        String::from_utf8(bytes).ok()
    }

    #[allow(non_snake_case)]
    pub(crate) fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<Vec<u8>> {
        let normalizedResourcePath = normalizeToolPkgEntryPath(resourcePath)?;
        match runtime.sourceType {
            ToolPkgSourceType::EXTERNAL => {
                let sourcePath = PathBuf::from(&runtime.sourcePath);
                if sourcePath.is_dir() {
                    return fs::read(sourcePath.join(&normalizedResourcePath)).ok();
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
                    return Some(bytes);
                }
                None
            }
            ToolPkgSourceType::ASSET => {
                let asset = bundledPluginAssetByName(&runtime.sourcePath)?;
                let cursor = std::io::Cursor::new(asset.bytes);
                let mut archive = zip::ZipArchive::new(cursor).ok()?;
                let mut entry = archive.by_name(&normalizedResourcePath).ok()?;
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).ok()?;
                Some(bytes)
            }
        }
    }
}

#[allow(non_snake_case)]
fn bundledPluginAssetByName(assetName: &str) -> Option<&'static RuntimePluginAsset> {
    toolRuntimeSupport()
        .builtinPluginAssets()
        .iter()
        .chain(toolRuntimeSupport().bundledExternalPluginAssets().iter())
        .find(|asset| asset.name == assetName)
}

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
