use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::runtime_support::{RuntimePluginAsset, ToolRuntimeSupport};
use crate::tools::condition::ConditionEvaluator::{ConditionEvaluator, ConditionValue};
use crate::tools::mcp::MCPManager::MCPManager;
use crate::tools::mcp::MCPPackage::MCPPackage;
use crate::tools::mcp::MCPServerConfig::MCPServerConfig;
use crate::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use crate::tools::skill::SkillManager::SkillManager;
use crate::tools::ToolJsRuntime::{JsExecutionEngine, JsExecutionProvider};
use crate::tools::ToolResultDataClasses::stringResultData;
use crate::ConversationMarkupManager::ToolResult;
use operit_host_api::HostManager::HostManager;
use operit_plugin_sdk::package::{LocalizedText, PublishablePackageSource, ToolPackage};
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{ToolPkgHookDispatcher, ToolPkgHookInvocation};
use operit_plugin_sdk::toolpkg::ToolPkgLoader::ToolPkgLoader;
use operit_plugin_sdk::toolpkg::ToolPkgManager::{
    ToolPkgAssetSource, ToolPkgExecutionEngineFactory, ToolPkgManager, ToolPkgRuntimeChangeListener,
};
use operit_plugin_sdk::toolpkg::ToolPkgPackageModels::{
    ToolPkgContainerDetails, ToolPkgDesktopWidget, ToolPkgNavigationActionHook,
    ToolPkgNavigationEntry, ToolPkgSubpackageInfo, ToolPkgToolboxUiModule, ToolPkgUiRoute,
    ToolPkgWorkspaceTemplate, ToolPkgWorkspaceTemplateImportResult,
};
use operit_plugin_sdk::toolpkg::ToolPkgPackageService::{
    ToolPkgPackageHost, ToolPkgPackageService,
};
use operit_plugin_sdk::toolpkg::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgContainerRuntime, ToolPkgLoadResult, ToolPkgResourceRuntime,
    ToolPkgSourceType, ToolPkgSubpackageRuntime,
};
use operit_plugin_sdk::JsPackageLoader::JsPackageLoader;
use operit_plugin_sdk::PackageManager::{PackageStateResolver, PluginPackageManager};
use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::AppLogger::AppLogger;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ENABLED_PACKAGES_KEY: &str = "imported_packages";
const DISABLED_PACKAGES_KEY: &str = "disabled_packages";
const BUNDLED_EXTERNAL_IMPORTS_KEY: &str = "bundled_external_imports";
const TOOLPKG_SUBPACKAGE_STATES_KEY: &str = "toolpkg_subpackage_states";
const TOOLPKG_CACHE_SIGNATURE_FILE: &str = ".toolpkg-cache-signature";
const PACKAGE_MANAGER_LOG_TAG: &str = "ToolPkg";

/// Creates SDK-owned ToolPkg execution engines through the installed JavaScript bridge.
#[derive(Clone)]
struct RuntimeToolPkgExecutionEngineFactory {
    toolHandler: crate::tools::AIToolHandler::AIToolHandler,
    jsExecutionProvider: Arc<dyn JsExecutionProvider>,
}

impl ToolPkgExecutionEngineFactory for RuntimeToolPkgExecutionEngineFactory {
    /// Creates one ToolPkg engine bound to the current host context.
    #[allow(non_snake_case)]
    fn createToolPkgExecutionEngine(&self) -> Arc<dyn JsExecutionEngine> {
        self.jsExecutionProvider
            .create_execution_engine(Arc::new(self.toolHandler.clone()))
    }
}

/// Exposes runtime-owned embedded plugin assets to the SDK package manager.
#[derive(Clone)]
struct RuntimeToolPkgAssetSource {
    runtimeSupport: Arc<dyn ToolRuntimeSupport>,
}

impl ToolPkgAssetSource for RuntimeToolPkgAssetSource {
    /// Returns one embedded ToolPkg archive as owned bytes.
    #[allow(non_snake_case)]
    fn toolPkgAssetBytes(&self, assetName: &str) -> Option<Vec<u8>> {
        self.runtimeSupport
            .builtinPluginAssets()
            .iter()
            .chain(self.runtimeSupport.bundledExternalPluginAssets().iter())
            .find(|asset| asset.name == assetName)
            .map(|asset| asset.bytes.to_vec())
    }
}

/// Resolves package states from the runtime capability snapshot.
#[derive(Clone, Copy)]
struct RuntimePackageStateResolver;

impl PackageStateResolver for RuntimePackageStateResolver {
    /// Returns the first conditional state matching the current runtime capabilities.
    #[allow(non_snake_case)]
    fn resolvePackageStateId(&self, package: &ToolPackage) -> Option<String> {
        let capabilities = buildConditionCapabilitiesSnapshot();
        package
            .states
            .iter()
            .find(|state| ConditionEvaluator::evaluate(&state.condition, &capabilities))
            .map(|state| state.id.clone())
    }
}

/// Cached MCP tool metadata imported into package manager state.
pub type CachedMcpToolInfo = crate::tools::mcp_runtime::MCPLocalServer::CachedToolInfo;

/// Built-in external package candidate bundled with the runtime assets.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct BundledExternalPackageCandidate {
    pub packageName: String,
    pub displayName: LocalizedText,
    pub description: LocalizedText,
    pub author: Vec<String>,
    pub packageKind: String,
    pub sourcePath: String,
    pub sourceFileName: String,
    pub isToolPkg: bool,
    pub version: String,
    pub category: String,
    pub toolCount: usize,
    pub subpackageCount: usize,
}

#[derive(Clone, Default)]
struct PackageScanSnapshot {
    availablePackages: BTreeMap<String, ToolPackage>,
    toolPkgContainers: BTreeMap<String, ToolPkgContainerRuntime>,
    toolPkgSubpackages: BTreeMap<String, ToolPkgSubpackageRuntime>,
}

#[derive(Clone, Default)]
struct PackageScanCandidateResult {
    phase: String,
    toolPackage: Option<ToolPackage>,
    toolPkgLoadResult: Option<ToolPkgLoadResult>,
    sourcePath: String,
}

#[derive(Clone, Default)]
struct ExternalPackageScanCacheEntry {
    signature: String,
    result: PackageScanCandidateResult,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct BundledExternalImportRecord {
    packageName: String,
    sourceFileName: String,
    destinationFileName: String,
    sourceSignature: String,
}

#[derive(Clone)]
/// Manages JavaScript packages, ToolPkg containers, MCP packages, and package UI resources.
pub struct RuntimePackageManager {
    pluginPackageManager: PluginPackageManager,
    cachedMcpTools: BTreeMap<String, Vec<CachedMcpToolInfo>>,
    externalPackageScanCache: BTreeMap<String, ExternalPackageScanCacheEntry>,
    bundledExternalPackageScanCache: BTreeMap<String, ExternalPackageScanCacheEntry>,
    toolPkgCacheLock: Arc<Mutex<()>>,
    jsEngine: Arc<dyn JsExecutionEngine>,
    dataStore: PreferencesDataStore,
    storePaths: RuntimeStorePaths,
    context: HostManager,
    toolHandler: crate::tools::AIToolHandler::AIToolHandler,
    mcpManager: MCPManager,
}

impl RuntimePackageManager {
    /// Creates a package manager bound to one tool handler instance.
    pub fn new(
        paths: RuntimeStorePaths,
        toolHandler: crate::tools::AIToolHandler::AIToolHandler,
    ) -> Self {
        let context = toolHandler.getContext();
        let runtimeDependencies = toolHandler.runtimeDependencies();
        let mut manager = Self {
            pluginPackageManager: PluginPackageManager::new(
                Arc::new(RuntimeToolPkgExecutionEngineFactory {
                    toolHandler: toolHandler.clone(),
                    jsExecutionProvider: runtimeDependencies.shared_js_execution_provider(),
                }),
                Arc::new(RuntimeToolPkgAssetSource {
                    runtimeSupport: runtimeDependencies.shared_runtime_support(),
                }),
                Arc::new(RuntimePackageStateResolver),
            ),
            cachedMcpTools: BTreeMap::new(),
            externalPackageScanCache: BTreeMap::new(),
            bundledExternalPackageScanCache: BTreeMap::new(),
            toolPkgCacheLock: Arc::new(Mutex::new(())),
            jsEngine: runtimeDependencies
                .js_execution_provider()
                .create_execution_engine(Arc::new(toolHandler.clone())),
            dataStore: PreferencesDataStore::new(paths.package_manager_preferences_path()),
            storePaths: paths,
            mcpManager: MCPManager::getInstance(context.clone()),
            context,
            toolHandler,
        };
        manager.loadAvailablePackages();
        manager
    }

    /// Marks a package as active for the current prompt session.
    pub fn activatePackage(&mut self, packageName: &str) -> bool {
        self.pluginPackageManager.activatePackage(packageName)
    }

    #[allow(non_snake_case)]
    /// Releases the ToolPkg execution engine for a context key.
    pub fn releaseToolPkgExecutionEngine(&self, contextKey: &str) {
        self.toolPkgManager()
            .releaseToolPkgExecutionEngine(contextKey);
    }

    #[allow(non_snake_case)]
    /// Returns the ToolPkg execution engine for a context key.
    pub fn getToolPkgExecutionEngine(&self, contextKey: &str) -> Arc<dyn JsExecutionEngine> {
        self.toolPkgManager().getToolPkgExecutionEngine(contextKey)
    }

    #[allow(non_snake_case)]
    /// Executes a Compose DSL render script through the ToolPkg engine for a context.
    pub fn executeToolPkgComposeDslScript(
        &self,
        contextKey: &str,
        script: &str,
        runtimeOptions: BTreeMap<String, serde_json::Value>,
        envOverrides: BTreeMap<String, String>,
    ) -> Result<Option<String>, String> {
        self.getToolPkgExecutionEngine(contextKey)
            .execute_compose_dsl_script(script, &runtimeOptions, &envOverrides)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    /// Dispatches a Compose DSL action through the ToolPkg engine for a context.
    pub fn dispatchToolPkgComposeDslActionEvents(
        &self,
        contextKey: &str,
        actionId: &str,
        payload: Option<serde_json::Value>,
        runtimeOptions: BTreeMap<String, serde_json::Value>,
        envOverrides: BTreeMap<String, String>,
    ) -> Result<Vec<String>, String> {
        let events = Arc::new(Mutex::new(Vec::<String>::new()));
        let eventCollector = events.clone();
        let finalEvent = self
            .getToolPkgExecutionEngine(contextKey)
            .dispatch_compose_dsl_action(
                actionId,
                payload,
                &runtimeOptions,
                &envOverrides,
                Some(Arc::new(move |event| {
                    eventCollector
                        .lock()
                        .expect("compose dsl event collector mutex poisoned")
                        .push(event);
                })),
            );
        let mut output = events
            .lock()
            .expect("compose dsl event collector mutex poisoned")
            .clone();
        if let Some(event) = finalEvent.map_err(|error| error.to_string())? {
            output.push(event);
        }
        Ok(output)
    }

    #[allow(non_snake_case)]
    /// Finds an existing ToolPkg execution engine for a context key.
    pub fn findToolPkgExecutionEngine(
        &self,
        contextKey: &str,
    ) -> Option<Arc<dyn JsExecutionEngine>> {
        self.toolPkgManager().findToolPkgExecutionEngine(contextKey)
    }

    #[allow(non_snake_case)]
    pub(crate) fn contextInternal(&self) -> &HostManager {
        &self.context
    }

    #[allow(non_snake_case)]
    pub(crate) fn jsEngineInternal(&self) -> &dyn JsExecutionEngine {
        self.jsEngine.as_ref()
    }

    #[allow(non_snake_case)]
    pub(crate) fn ensureInitialized(&self) {}

    /// Returns the SDK-owned ToolPkg manager.
    #[allow(non_snake_case)]
    fn toolPkgManager(&self) -> &ToolPkgManager {
        self.pluginPackageManager.toolPkgManager()
    }

    /// Returns the SDK-owned ToolPkg manager for mutation.
    #[allow(non_snake_case)]
    fn toolPkgManagerMut(&mut self) -> &mut ToolPkgManager {
        self.pluginPackageManager.toolPkgManagerMut()
    }

    /// Returns the SDK-owned package definition map.
    #[allow(non_snake_case)]
    fn availablePackages(&self) -> &BTreeMap<String, ToolPackage> {
        self.pluginPackageManager.availablePackagesRef()
    }

    /// Returns the SDK-owned package definition map for mutation.
    #[allow(non_snake_case)]
    fn availablePackagesMut(&mut self) -> &mut BTreeMap<String, ToolPackage> {
        self.pluginPackageManager.availablePackagesMut()
    }

    #[allow(non_snake_case)]
    fn toolPkgCacheRootDir(&self) -> PathBuf {
        let dir = self.storePaths.toolpkg_cache_dir();
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        dir
    }

    #[allow(non_snake_case)]
    fn toolPkgCacheDirName(packageName: &str) -> String {
        let normalized = packageName.trim();
        let safeName = normalized
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>();
        let safeName = if safeName.trim().is_empty() {
            "toolpkg".to_string()
        } else {
            safeName
        };
        let hash = javaStringHashCodeHex(normalized);
        format!("{safeName}-{hash}")
    }

    #[allow(non_snake_case)]
    fn toolPkgCacheDir(&self, packageName: &str) -> PathBuf {
        self.toolPkgCacheRootDir()
            .join(Self::toolPkgCacheDirName(packageName))
    }

    #[allow(non_snake_case)]
    fn deleteToolPkgCacheDir(&self, packageName: &str) {
        let _guard = self
            .toolPkgCacheLock
            .lock()
            .expect("toolpkg cache mutex poisoned");
        self.deleteToolPkgCacheDirLocked(packageName);
    }

    #[allow(non_snake_case)]
    fn deleteToolPkgCacheDirLocked(&self, packageName: &str) {
        let dir = self.toolPkgCacheDir(packageName);
        if dir.exists() {
            let _ = fs::remove_dir_all(dir);
        }
    }

    #[allow(non_snake_case)]
    fn ensureToolPkgCacheDir<FExtractArchive>(
        &self,
        packageName: &str,
        signature: &str,
        mainEntry: &str,
        extractArchive: FExtractArchive,
    ) -> Option<PathBuf>
    where
        FExtractArchive: Fn(&Path) -> bool,
    {
        let _guard = self
            .toolPkgCacheLock
            .lock()
            .expect("toolpkg cache mutex poisoned");
        let cacheDir = self.toolPkgCacheDir(packageName);
        let signatureFile = cacheDir.join(TOOLPKG_CACHE_SIGNATURE_FILE);
        let cacheDirExists = cacheDir.exists();
        let signatureFileExists = signatureFile.exists();
        let signatureMatches = if signatureFileExists {
            fs::read_to_string(&signatureFile)
                .map(|text| text == signature)
                .unwrap_or(false)
        } else {
            false
        };
        let mainScriptFile = cacheDir.join(mainEntry);
        let mainScriptExists = mainScriptFile.exists();

        if cacheDirExists && signatureFileExists && signatureMatches && mainScriptExists {
            return Some(cacheDir);
        }

        self.deleteToolPkgCacheDirLocked(packageName);
        if fs::create_dir_all(&cacheDir).is_err() {
            return None;
        }
        if !extractArchive(&cacheDir) {
            self.deleteToolPkgCacheDirLocked(packageName);
            return None;
        }
        if fs::write(&signatureFile, signature).is_err() {
            self.deleteToolPkgCacheDirLocked(packageName);
            return None;
        }
        Some(cacheDir)
    }

    #[allow(non_snake_case)]
    fn buildToolPkgCacheSignature(
        &self,
        sourceType: &ToolPkgSourceType,
        sourcePath: &str,
        version: &str,
        mainEntry: &str,
    ) -> Option<String> {
        match sourceType {
            ToolPkgSourceType::EXTERNAL => {
                let sourceFile = PathBuf::from(sourcePath);
                if !sourceFile.exists() {
                    return None;
                }
                let metadata = fs::metadata(&sourceFile).ok()?;
                Some(format!(
                    "external|{}|{}|{}|{}|{}",
                    sourceFile.to_string_lossy(),
                    metadata.len(),
                    metadataModifiedMillis(&metadata),
                    version,
                    mainEntry
                ))
            }
            ToolPkgSourceType::ASSET => {
                let asset = bundledPluginAssetByName(
                    self.toolHandler.runtimeSupport().as_ref(),
                    sourcePath,
                )?;
                Some(format!(
                    "asset|{}|{}|{}|{}|{}",
                    sourcePath,
                    asset.bytes.len(),
                    sha256Hex(asset.bytes),
                    version,
                    mainEntry
                ))
            }
        }
    }

    #[allow(non_snake_case)]
    fn buildToolPkgCacheSignatureForRuntime(
        &self,
        runtime: &ToolPkgContainerRuntime,
    ) -> Option<String> {
        self.buildToolPkgCacheSignature(
            &runtime.sourceType,
            &runtime.sourcePath,
            &runtime.version,
            &runtime.mainEntry,
        )
    }

    #[allow(non_snake_case)]
    fn extractToolPkgArchive(
        &self,
        runtime: &ToolPkgContainerRuntime,
        destinationDir: &Path,
    ) -> bool {
        match runtime.sourceType {
            ToolPkgSourceType::EXTERNAL => {
                let sourcePath = PathBuf::from(&runtime.sourcePath);
                if sourcePath.is_dir() {
                    return copyDirectoryEntries(&sourcePath, destinationDir);
                }
                ToolPkgArchiveParser::extractZipEntriesFromExternal(
                    &runtime.sourcePath,
                    destinationDir,
                )
            }
            ToolPkgSourceType::ASSET => {
                let Some(asset) = bundledPluginAssetByName(
                    self.toolHandler.runtimeSupport().as_ref(),
                    &runtime.sourcePath,
                ) else {
                    return false;
                };
                ToolPkgArchiveParser::extractZipEntriesFromAssetBytes(asset.bytes, destinationDir)
            }
        }
    }

    #[allow(non_snake_case)]
    fn ensureToolPkgCache(&self, runtime: &ToolPkgContainerRuntime) -> Option<PathBuf> {
        let signature = self.buildToolPkgCacheSignatureForRuntime(runtime)?;
        self.ensureToolPkgCacheDir(
            &runtime.packageName,
            &signature,
            &runtime.mainEntry,
            |destinationDir| self.extractToolPkgArchive(runtime, destinationDir),
        )
    }

    #[allow(non_snake_case)]
    pub(crate) fn resolveToolPkgResourceFile(
        &self,
        runtime: &ToolPkgContainerRuntime,
        normalizedResourcePath: &str,
    ) -> Option<PathBuf> {
        let normalizedPath = ToolPkgArchiveParser::normalizeResourcePath(normalizedResourcePath)?;
        let cacheDir = self.ensureToolPkgCache(runtime)?;
        let resourceFile = cacheDir.join(normalizedPath);
        if !resourceFile.exists() {
            return None;
        }
        Some(resourceFile)
    }

    #[allow(non_snake_case)]
    pub(crate) fn exportToolPkgResource(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resource: &ToolPkgResourceRuntime,
        destinationFile: &Path,
    ) -> bool {
        let Some(resourceFile) = self.resolveToolPkgResourceFile(runtime, &resource.path) else {
            return false;
        };
        if let Some(parent) = destinationFile.parent() {
            if fs::create_dir_all(parent).is_err() {
                return false;
            }
        }
        if ToolPkgArchiveParser::isDirectoryResourceMime(Some(&resource.mime)) {
            if !resourceFile.is_dir() {
                return false;
            }
            zipToolPkgResourceDirectory(&resourceFile, destinationFile)
        } else {
            if !resourceFile.is_file() {
                return false;
            }
            fs::copy(resourceFile, destinationFile).is_ok()
        }
    }

    #[allow(non_snake_case)]
    pub(crate) fn toolPkgContainersInternal(&self) -> BTreeMap<String, ToolPkgContainerRuntime> {
        self.toolPkgManager().getToolPkgContainerRuntimeMap()
    }

    #[allow(non_snake_case)]
    pub(crate) fn toolPkgSubpackageByPackageNameInternal(
        &self,
    ) -> BTreeMap<String, ToolPkgSubpackageRuntime> {
        self.toolPkgManager().getToolPkgSubpackageRuntimeMap()
    }

    /// Returns whether a package has been activated for the current prompt session.
    pub fn isPackageActivated(&self, packageName: &str) -> bool {
        self.pluginPackageManager.isPackageActivated(packageName)
    }

    #[allow(non_snake_case)]
    /// Activates a package and returns its system prompt contribution.
    pub fn usePackage(&mut self, packageName: &str) -> String {
        let normalizedPackageName = self.normalizePackageName(packageName);

        if self.isToolPkgContainer(&normalizedPackageName) {
            return format!(
                "ToolPkg container '{}' is not a package and cannot be activated.",
                normalizedPackageName
            );
        }

        let enabledPackageNames = self.getEnabledPackageNames();
        if enabledPackageNames.contains(&normalizedPackageName) {
            let Some(toolPackage) = self
                .availablePackages()
                .get(&normalizedPackageName)
                .cloned()
            else {
                return format!("Failed to load package data for: {}", normalizedPackageName);
            };
            let selectedPackage = self.selectToolPackageState(&toolPackage);
            self.activatePackage(&normalizedPackageName);
            return self.generatePackageSystemPrompt(&selectedPackage);
        }

        let skillManager = SkillManager::fromDefaultPaths();
        if skillManager
            .getAvailableSkills()
            .contains_key(&normalizedPackageName)
            && !self
                .toolHandler
                .runtimeSupport()
                .isSkillVisibleToAi(&normalizedPackageName)
        {
            return format!("Skill '{}' is set to not show to AI", normalizedPackageName);
        }

        if let Some(skillPrompt) = skillManager.getSkillSystemPrompt(&normalizedPackageName) {
            return skillPrompt;
        }

        if self.isRegisteredMCPServer(&normalizedPackageName) {
            return self.useMCPServer(&normalizedPackageName);
        }

        format!(
            "Package not found: {}. Please import it first or register it as an MCP server.",
            normalizedPackageName
        )
    }

    #[allow(non_snake_case)]
    /// Executes the built-in package activation tool.
    pub fn executeUsePackageTool(&mut self, toolName: &str, packageName: &str) -> ToolResult {
        if packageName.trim().is_empty() {
            return ToolResult {
                toolName: toolName.to_string(),
                success: false,
                result: stringResultData(""),
                error: Some("Missing required parameter: package_name".to_string()),
            };
        }

        let normalizedPackageName = self.normalizePackageName(packageName);
        if self.isToolPkgContainer(&normalizedPackageName) {
            return ToolResult {
                toolName: toolName.to_string(),
                success: false,
                result: stringResultData(""),
                error: Some(format!(
                    "ToolPkg container '{}' is not a package and cannot be activated.",
                    normalizedPackageName
                )),
            };
        }

        let skillManager = SkillManager::fromDefaultPaths();
        if skillManager
            .getAvailableSkills()
            .contains_key(&normalizedPackageName)
            && !self
                .toolHandler
                .runtimeSupport()
                .isSkillVisibleToAi(&normalizedPackageName)
        {
            return ToolResult {
                toolName: toolName.to_string(),
                success: false,
                result: stringResultData(""),
                error: Some(format!(
                    "Skill '{}' is set to not show to AI",
                    normalizedPackageName
                )),
            };
        }

        ToolResult {
            toolName: toolName.to_string(),
            success: true,
            result: stringResultData(self.usePackage(&normalizedPackageName)),
            error: None,
        }
    }

    #[allow(non_snake_case)]
    /// Returns package names enabled in preferences after applying disabled package records.
    pub fn getEnabledPackageNames(&self) -> Vec<String> {
        let mut enabledPackageNames =
            BTreeSet::from_iter(self.decodeEnabledPackageNamesFromPrefs());
        let disabledPackageNames = BTreeSet::from_iter(self.decodeDisabledPackageNamesFromPrefs());
        for toolPackage in self.availablePackages().values() {
            if toolPackage.is_built_in
                && toolPackage.enabled_by_default
                && !disabledPackageNames.contains(&toolPackage.name)
            {
                enabledPackageNames.insert(toolPackage.name.clone());
            }
        }
        enabledPackageNames.into_iter().collect()
    }

    #[allow(non_snake_case)]
    pub(crate) fn getEnabledPackageNameSetInternal(&self) -> BTreeSet<String> {
        BTreeSet::from_iter(self.getEnabledPackageNames())
    }

    #[allow(non_snake_case)]
    pub(crate) fn getToolPkgSubpackageStatesInternal(&self) -> BTreeMap<String, bool> {
        self.normalizeToolPkgSubpackageStates(&self.decodeToolPkgSubpackageStatesFromPrefs())
    }

    #[allow(non_snake_case)]
    /// Returns whether a package is enabled and not disabled by ToolPkg subpackage state.
    pub fn isPackageEnabled(&self, packageName: &str) -> bool {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let enabledPackageSet = self.getEnabledPackageNameSetInternal();
        if !enabledPackageSet.contains(&normalizedPackageName) {
            return false;
        }
        if let Some(subpackageRuntime) = self
            .toolPkgManager()
            .resolveToolPkgSubpackageRuntimeInternal(&normalizedPackageName)
        {
            return enabledPackageSet.contains(&subpackageRuntime.containerPackageName);
        }
        true
    }

    #[allow(non_snake_case)]
    /// Returns package names currently active in the prompt session.
    pub fn getActivePackageNames(&self) -> Vec<String> {
        self.pluginPackageManager.activePackageNames()
    }

    #[allow(non_snake_case)]
    /// Enables a package and loads its tools into available package state.
    pub fn enablePackage(&mut self, packageName: &str) -> String {
        let normalizedPackageName = self.normalizePackageName(packageName);
        if normalizedPackageName.trim().is_empty() {
            return "Package name cannot be empty".to_string();
        }

        if !self
            .availablePackages()
            .contains_key(&normalizedPackageName)
        {
            return format!(
                "Package not found in available packages: {}",
                normalizedPackageName
            );
        }

        let mut enabledPackageNames = BTreeSet::from_iter(self.getEnabledPackageNames());
        let mut subpackageStates = self.getToolPkgSubpackageStatesInternal();

        if let Some(containerRuntime) = self
            .toolPkgManager()
            .getToolPkgContainerRuntime(&normalizedPackageName)
        {
            let containerAlreadyEnabled = enabledPackageNames.contains(&normalizedPackageName);
            enabledPackageNames.insert(normalizedPackageName.clone());
            for subpackage in &containerRuntime.subpackages {
                let shouldEnable = subpackageStates
                    .get(&subpackage.packageName)
                    .copied()
                    .unwrap_or(subpackage.enabledByDefault);
                subpackageStates
                    .entry(subpackage.packageName.clone())
                    .or_insert(shouldEnable);
                if shouldEnable {
                    enabledPackageNames.insert(subpackage.packageName.clone());
                } else {
                    enabledPackageNames.remove(&subpackage.packageName);
                }
            }
            let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
            if let Err(error) = self.saveEnabledPackageNames(&names) {
                return format!(
                    "Failed to enable ToolPkg container '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Err(error) = self.saveToolPkgSubpackageStates(&subpackageStates) {
                return format!(
                    "Failed to save ToolPkg subpackage states '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Err(error) = self.removeFromDisabledPackages(&normalizedPackageName) {
                return format!(
                    "Failed to enable ToolPkg container '{}': {}",
                    normalizedPackageName, error
                );
            }
            self.ensureToolPkgCache(&containerRuntime);
            self.notifyToolPkgRuntimeChangeListeners();
            if containerAlreadyEnabled {
                return format!(
                    "ToolPkg container '{}' is already enabled",
                    normalizedPackageName
                );
            }
            return format!(
                "Successfully enabled toolpkg container: {}",
                normalizedPackageName
            );
        }

        if let Some(subpackageRuntime) = self
            .toolPkgManager()
            .resolveToolPkgSubpackageRuntimeInternal(&normalizedPackageName)
        {
            enabledPackageNames.insert(subpackageRuntime.containerPackageName.clone());
            enabledPackageNames.insert(normalizedPackageName.clone());
            subpackageStates.insert(normalizedPackageName.clone(), true);
            let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
            if let Err(error) = self.saveEnabledPackageNames(&names) {
                return format!(
                    "Failed to enable ToolPkg subpackage '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Err(error) = self.saveToolPkgSubpackageStates(&subpackageStates) {
                return format!(
                    "Failed to save ToolPkg subpackage states '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Err(error) =
                self.removeFromDisabledPackages(&subpackageRuntime.containerPackageName)
            {
                return format!(
                    "Failed to enable ToolPkg subpackage '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Some(runtime) = self
                .toolPkgManager()
                .getToolPkgContainerRuntime(&subpackageRuntime.containerPackageName)
            {
                self.ensureToolPkgCache(&runtime);
            }
            self.notifyToolPkgRuntimeChangeListeners();
            return format!(
                "Successfully enabled toolpkg subpackage: {}",
                normalizedPackageName
            );
        }

        if enabledPackageNames.contains(&normalizedPackageName) {
            return format!("Package '{}' is already enabled", normalizedPackageName);
        }
        enabledPackageNames.insert(normalizedPackageName.clone());
        let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
        if let Err(error) = self.saveEnabledPackageNames(&names) {
            return format!(
                "Failed to enable package '{}': {}",
                normalizedPackageName, error
            );
        }
        if let Err(error) = self.removeFromDisabledPackages(&normalizedPackageName) {
            return format!(
                "Failed to enable package '{}': {}",
                normalizedPackageName, error
            );
        }
        self.notifyToolPkgRuntimeChangeListeners();
        format!("Successfully enabled package: {}", normalizedPackageName)
    }

    #[allow(non_snake_case)]
    /// Disables a package and removes its tools from active package state.
    pub fn disablePackage(&mut self, packageName: &str) -> String {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let mut enabledPackageNames = BTreeSet::from_iter(self.getEnabledPackageNames());
        let mut subpackageStates = self.getToolPkgSubpackageStatesInternal();
        self.pluginPackageManager
            .deactivatePackage(&normalizedPackageName);

        if let Some(containerRuntime) = self
            .toolPkgManager()
            .getToolPkgContainerRuntime(&normalizedPackageName)
        {
            let mut packageWasRemoved = enabledPackageNames.remove(&normalizedPackageName);
            self.unregisterPackageTools(&normalizedPackageName);
            for subpackage in containerRuntime.subpackages {
                packageWasRemoved =
                    enabledPackageNames.remove(&subpackage.packageName) || packageWasRemoved;
                self.unregisterPackageTools(&subpackage.packageName);
            }
            let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
            if let Err(error) = self.saveEnabledPackageNames(&names) {
                return format!(
                    "Failed to disable ToolPkg container '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Err(error) = self.saveToolPkgSubpackageStates(&subpackageStates) {
                return format!(
                    "Failed to save ToolPkg subpackage states '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Err(error) = self.addToDisabledIfDefaultEnabled(&normalizedPackageName) {
                return format!(
                    "Failed to disable ToolPkg container '{}': {}",
                    normalizedPackageName, error
                );
            }
            self.deleteToolPkgCacheDir(&normalizedPackageName);
            self.toolPkgManager()
                .destroyDefaultToolPkgExecutionEngine(&normalizedPackageName);
            self.notifyToolPkgRuntimeChangeListeners();
            if packageWasRemoved {
                return format!(
                    "Successfully disabled toolpkg container: {}",
                    normalizedPackageName
                );
            }
            return format!(
                "ToolPkg container is already disabled: {}",
                normalizedPackageName
            );
        }

        if self
            .toolPkgManager()
            .resolveToolPkgSubpackageRuntimeInternal(&normalizedPackageName)
            .is_some()
        {
            let packageWasRemoved = enabledPackageNames.remove(&normalizedPackageName);
            subpackageStates.insert(normalizedPackageName.clone(), false);
            self.unregisterPackageTools(&normalizedPackageName);
            let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
            if let Err(error) = self.saveEnabledPackageNames(&names) {
                return format!(
                    "Failed to disable ToolPkg subpackage '{}': {}",
                    normalizedPackageName, error
                );
            }
            if let Err(error) = self.saveToolPkgSubpackageStates(&subpackageStates) {
                return format!(
                    "Failed to save ToolPkg subpackage states '{}': {}",
                    normalizedPackageName, error
                );
            }
            self.notifyToolPkgRuntimeChangeListeners();
            if packageWasRemoved {
                return format!(
                    "Successfully disabled toolpkg subpackage: {}",
                    normalizedPackageName
                );
            }
            return format!(
                "ToolPkg subpackage is already disabled: {}",
                normalizedPackageName
            );
        }

        let packageWasRemoved = enabledPackageNames.remove(&normalizedPackageName);
        self.unregisterPackageTools(&normalizedPackageName);
        if let Err(error) = self.addToDisabledIfDefaultEnabled(&normalizedPackageName) {
            return format!(
                "Failed to disable package '{}': {}",
                normalizedPackageName, error
            );
        }
        if packageWasRemoved {
            let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
            if let Err(error) = self.saveEnabledPackageNames(&names) {
                return format!(
                    "Failed to disable package '{}': {}",
                    normalizedPackageName, error
                );
            }
            self.notifyToolPkgRuntimeChangeListeners();
            return format!("Successfully disabled package: {}", normalizedPackageName);
        }
        format!("Package is already disabled: {}", normalizedPackageName)
    }

    #[allow(non_snake_case)]
    /// Deletes an external package from storage and package state.
    pub fn deletePackage(&mut self, packageName: &str) -> bool {
        let normalizedPackageName = self.normalizePackageName(packageName);

        if let Some(subpackageRuntime) = self
            .toolPkgManager()
            .resolveToolPkgSubpackageRuntimeInternal(&normalizedPackageName)
        {
            return self.deletePackage(&subpackageRuntime.containerPackageName);
        }

        let containerRuntime = self
            .toolPkgManager()
            .getToolPkgContainerRuntime(&normalizedPackageName);
        if containerRuntime
            .as_ref()
            .is_some_and(|runtime| runtime.sourceType != ToolPkgSourceType::EXTERNAL)
        {
            return false;
        }
        if containerRuntime.is_none()
            && self
                .availablePackages()
                .get(&normalizedPackageName)
                .is_some_and(|package| package.is_built_in)
        {
            return false;
        }
        let isToolPkgContainer = containerRuntime.is_some();
        let packageFile = self.findPackageFile(&normalizedPackageName);

        if packageFile.as_ref().is_none_or(|file| !file.exists()) {
            if isToolPkgContainer {
                self.disableToolPkgContainer(&normalizedPackageName);
            } else {
                self.disablePackage(&normalizedPackageName);
            }
            self.removeFromCachesAfterDelete(&normalizedPackageName);
            if self
                .removeBundledExternalImportRecord(&normalizedPackageName)
                .is_err()
            {
                return false;
            }
            return true;
        }

        let packageFile = packageFile.expect("checked package file presence");
        match fs::remove_file(&packageFile) {
            Ok(_) => {
                if isToolPkgContainer {
                    self.disableToolPkgContainer(&normalizedPackageName);
                } else {
                    self.disablePackage(&normalizedPackageName);
                }
                self.removeFromCachesAfterDelete(&normalizedPackageName);
                if self
                    .removeBundledExternalImportRecord(&normalizedPackageName)
                    .is_err()
                {
                    return false;
                }
                true
            }
            Err(_) => false,
        }
    }

    #[allow(non_snake_case)]
    /// Enables a ToolPkg container through the normal package enable flow.
    pub fn enableToolPkgContainer(&mut self, containerPackageName: &str) -> String {
        self.enablePackage(containerPackageName)
    }

    #[allow(non_snake_case)]
    /// Disables a ToolPkg container through the normal package disable flow.
    pub fn disableToolPkgContainer(&mut self, containerPackageName: &str) -> String {
        self.disablePackage(containerPackageName)
    }

    #[allow(non_snake_case)]
    /// Returns whether a package name belongs to a ToolPkg container runtime.
    pub fn isToolPkgContainer(&self, packageName: &str) -> bool {
        ToolPkgPackageService::new(self).isToolPkgContainer(packageName)
    }

    #[allow(non_snake_case)]
    /// Returns whether a package name is a ToolPkg subpackage.
    pub fn isToolPkgSubpackage(&self, packageName: &str) -> bool {
        ToolPkgPackageService::new(self).isToolPkgSubpackage(packageName)
    }

    #[allow(non_snake_case)]
    /// Returns whether a package is visible as a top-level package.
    pub fn isTopLevelPackage(&self, packageName: &str) -> bool {
        self.ensureInitialized();
        self.resolveToolPkgSubpackageRuntimeInternal(packageName)
            .is_none()
    }

    #[allow(non_snake_case)]
    /// Returns available packages excluding ToolPkg subpackages.
    pub fn getTopLevelAvailablePackages(&self) -> BTreeMap<String, ToolPackage> {
        self.ensureInitialized();
        let toolPkgSubpackages = self.toolPkgSubpackageByPackageNameInternal();
        self.getAvailablePackages()
            .into_iter()
            .filter(|(packageName, _)| !toolPkgSubpackages.contains_key(packageName))
            .collect()
    }

    #[allow(non_snake_case)]
    /// Returns packages that can be executed directly as tools.
    pub fn getExecutableAvailablePackages(&self) -> BTreeMap<String, ToolPackage> {
        self.ensureInitialized();
        self.getAvailablePackages()
            .into_iter()
            .filter(|(packageName, _)| !self.isToolPkgContainer(packageName))
            .collect()
    }

    #[allow(non_snake_case)]
    /// Returns localized details for all registered ToolPkg containers.
    pub fn getToolPkgPluginContainerDetails(
        &self,
        useEnglish: bool,
    ) -> Vec<ToolPkgContainerDetails> {
        self.ensureInitialized();
        self.toolPkgContainersInternal()
            .keys()
            .filter_map(|packageName| self.getToolPkgContainerDetails(packageName, useEnglish))
            .collect()
    }

    #[allow(non_snake_case)]
    /// Returns ToolPkg container runtimes that are currently enabled.
    pub fn getEnabledToolPkgContainerRuntimes(&self) -> Vec<ToolPkgContainerRuntime> {
        self.toolPkgManager()
            .getEnabledToolPkgContainerRuntimes(&self.getEnabledPackageNames())
    }

    #[allow(non_snake_case)]
    /// Returns all registered ToolPkg container runtimes.
    pub fn getToolPkgContainerRuntimes(&self) -> Vec<ToolPkgContainerRuntime> {
        self.toolPkgManager().getToolPkgContainerRuntimes()
    }

    #[allow(non_snake_case)]
    /// Returns localized details for a ToolPkg container.
    pub fn getToolPkgContainerDetails(
        &self,
        packageName: &str,
        useEnglish: bool,
    ) -> Option<ToolPkgContainerDetails> {
        ToolPkgPackageService::new(self).getToolPkgContainerDetails(packageName, useEnglish)
    }

    #[allow(non_snake_case)]
    /// Returns UI routes exposed by ToolPkg modules for one runtime target.
    pub fn getToolPkgUiRoutes(&self, runtime: &str, useEnglish: bool) -> Vec<ToolPkgUiRoute> {
        ToolPkgPackageService::new(self).getToolPkgUiRoutes(runtime, useEnglish)
    }

    #[allow(non_snake_case)]
    /// Returns desktop widgets exposed by enabled ToolPkg containers.
    pub fn getToolPkgDesktopWidgets(&self, useEnglish: bool) -> Vec<ToolPkgDesktopWidget> {
        ToolPkgPackageService::new(self).getToolPkgDesktopWidgets(useEnglish)
    }

    #[allow(non_snake_case)]
    /// Returns navigation entries exposed by enabled ToolPkg containers.
    pub fn getToolPkgNavigationEntries(&self, useEnglish: bool) -> Vec<ToolPkgNavigationEntry> {
        ToolPkgPackageService::new(self).getToolPkgNavigationEntries(useEnglish)
    }

    #[allow(non_snake_case)]
    /// Returns workspace templates exposed by enabled ToolPkg containers.
    pub fn getToolPkgWorkspaceTemplates(&self, useEnglish: bool) -> Vec<ToolPkgWorkspaceTemplate> {
        ToolPkgPackageService::new(self).getToolPkgWorkspaceTemplates(useEnglish)
    }

    #[allow(non_snake_case)]
    /// Imports a ToolPkg workspace template into a destination directory.
    pub fn importToolPkgWorkspaceTemplate(
        &self,
        containerPackageName: &str,
        templateId: &str,
        destinationDir: &str,
    ) -> Result<ToolPkgWorkspaceTemplateImportResult, String> {
        ToolPkgPackageService::new(self).importToolPkgWorkspaceTemplate(
            containerPackageName,
            templateId,
            Path::new(destinationDir),
        )
    }

    #[allow(non_snake_case)]
    /// Updates the enabled state for a ToolPkg subpackage.
    pub fn setToolPkgSubpackageEnabled(
        &mut self,
        subpackagePackageName: &str,
        enabled: bool,
    ) -> bool {
        let normalizedPackageName = self.normalizePackageName(subpackagePackageName);
        let success = ToolPkgPackageService::new(self)
            .setToolPkgSubpackageEnabled(&normalizedPackageName, enabled);
        if success && !enabled {
            self.unregisterPackageTools(&normalizedPackageName);
        }
        if success {
            self.notifyToolPkgRuntimeChangeListeners();
        }
        success
    }

    #[allow(non_snake_case)]
    /// Resolves the best package name for a ToolPkg subpackage id.
    pub fn findPreferredPackageNameForSubpackageId(
        &self,
        subpackageId: &str,
        preferEnabled: bool,
    ) -> Option<String> {
        ToolPkgPackageService::new(self)
            .findPreferredPackageNameForSubpackageId(subpackageId, preferEnabled)
    }

    #[allow(non_snake_case)]
    /// Runs a ToolPkg navigation entry action hook.
    pub fn runToolPkgNavigationEntryAction(
        &self,
        containerPackageName: &str,
        entryId: &str,
        functionName: &str,
        inlineFunctionSource: Option<&str>,
        eventPayload: serde_json::Value,
    ) -> Result<Option<String>, String> {
        self.runToolPkgMainHook(
            containerPackageName,
            functionName,
            operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_NAVIGATION_ENTRY_ACTION,
            Some("navigation_entry_action"),
            Some(entryId),
            inlineFunctionSource,
            eventPayload,
            None,
            None,
            None,
        )
    }

    #[allow(non_snake_case)]
    /// Lists bundled external standalone packages that are not currently loaded.
    pub fn getBundledExternalPackageCandidates(&mut self) -> Vec<BundledExternalPackageCandidate> {
        let loadedPackageNames = self
            .availablePackages()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut candidates = self
            .scanBundledExternalPackageCandidates()
            .into_iter()
            .filter_map(Self::bundledExternalPackageCandidateFromScanResult)
            .filter(|candidate| !loadedPackageNames.contains(&candidate.packageName))
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.displayName
                .resolve(false)
                .cmp(&right.displayName.resolve(false))
                .then_with(|| left.packageName.cmp(&right.packageName))
        });
        candidates
    }

    #[allow(non_snake_case)]
    /// Lists bundled external ToolPkg containers that are not currently loaded.
    pub fn getBundledExternalToolPkgContainerRuntimes(&mut self) -> Vec<ToolPkgContainerRuntime> {
        let loadedContainerNames = self
            .toolPkgManager()
            .getToolPkgContainerRuntimes()
            .into_iter()
            .map(|runtime| runtime.packageName)
            .collect::<BTreeSet<_>>();
        let mut runtimes = self
            .scanBundledExternalToolPkgCandidates()
            .into_iter()
            .filter_map(|result| result.toolPkgLoadResult)
            .map(|loadResult| loadResult.containerRuntime)
            .filter(|runtime| !loadedContainerNames.contains(&runtime.packageName))
            .collect::<Vec<_>>();
        runtimes.sort_by(|left, right| left.packageName.cmp(&right.packageName));
        runtimes
    }

    #[allow(non_snake_case)]
    /// Imports a bundled external standalone package into local package storage.
    pub fn importBundledExternalPackage(&mut self, packageName: &str) -> String {
        let normalizedPackageName = self.normalizePackageName(packageName);
        for result in self.scanBundledExternalPackageCandidates() {
            if Self::packageNameFromScanResult(&result).as_deref()
                == Some(normalizedPackageName.as_str())
            {
                return self.importBundledExternalPackageCandidate(result, &normalizedPackageName);
            }
        }
        format!(
            "Bundled external package not found: {}",
            normalizedPackageName
        )
    }

    #[allow(non_snake_case)]
    /// Imports a bundled external ToolPkg container into local package storage.
    pub fn importBundledExternalToolPkgContainer(&mut self, containerPackageName: &str) -> String {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        for result in self.scanBundledExternalToolPkgCandidates() {
            let Some(loadResult) = &result.toolPkgLoadResult else {
                continue;
            };
            if loadResult.containerPackage.name == normalizedContainerPackageName {
                return self.importBundledExternalPackageCandidate(
                    result,
                    &normalizedContainerPackageName,
                );
            }
        }
        format!(
            "Bundled external ToolPkg container not found: {}",
            normalizedContainerPackageName
        )
    }

    #[allow(non_snake_case)]
    fn importBundledExternalPackageCandidate(
        &mut self,
        result: PackageScanCandidateResult,
        packageName: &str,
    ) -> String {
        let sourcePath = result.sourcePath;
        let Some(sourceAsset) = bundledExternalPluginAssetByName(
            self.toolHandler.runtimeSupport().as_ref(),
            &sourcePath,
        ) else {
            return format!("Bundled external package asset not found: {sourcePath}");
        };
        let sourceSignature = sha256Hex(sourceAsset.bytes);
        let sourceFileName = packageSourceFileName(&sourcePath);
        if let Err(error) = self.storePaths.ensure_packages_dir() {
            return format!("Error importing package: {error}");
        }
        let destinationFile = self.storePaths.packages_dir().join(&sourceFileName);
        if let Err(error) = fs::write(&destinationFile, sourceAsset.bytes) {
            return format!("Error importing package: {error}");
        }
        self.externalPackageScanCache
            .remove(&destinationFile.to_string_lossy().to_string());
        self.loadAvailablePackages();
        let record = BundledExternalImportRecord {
            packageName: packageName.to_string(),
            sourceFileName: sourceFileName.clone(),
            destinationFileName: sourceFileName,
            sourceSignature,
        };
        let message = format!(
            "Successfully imported package: {}\nStored at: {}",
            packageName,
            destinationFile.to_string_lossy()
        );
        match self.upsertBundledExternalImportRecord(record) {
            Ok(()) => message,
            Err(error) => format!("{message}\nFailed to record bundled external import: {error}"),
        }
    }

    #[allow(non_snake_case)]
    /// Returns the runtime metadata for a ToolPkg container.
    pub fn getToolPkgContainerRuntime(
        &self,
        containerPackageName: &str,
    ) -> Option<ToolPkgContainerRuntime> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        self.toolPkgManager()
            .getToolPkgContainerRuntime(&normalizedContainerPackageName)
    }

    #[allow(non_snake_case)]
    /// Resolves ToolPkg subpackage runtime metadata by package name.
    pub fn resolveToolPkgSubpackageRuntimeInternal(
        &self,
        packageName: &str,
    ) -> Option<ToolPkgSubpackageRuntime> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        self.toolPkgManager()
            .resolveToolPkgSubpackageRuntimeInternal(&normalizedPackageName)
    }

    #[allow(non_snake_case)]
    /// Returns package tools with active state applied.
    pub fn getEffectivePackageTools(&self, packageName: &str) -> Option<ToolPackage> {
        self.pluginPackageManager.effectivePackage(packageName)
    }

    #[allow(non_snake_case)]
    /// Returns raw package tool metadata by package name.
    pub fn getPackageTools(&self, packageName: &str) -> Option<ToolPackage> {
        self.pluginPackageManager.package(packageName)
    }

    #[allow(non_snake_case)]
    /// Returns the first tool script associated with a package.
    pub fn getPackageScript(&self, packageName: &str) -> Option<String> {
        self.pluginPackageManager
            .package(packageName)
            .as_ref()
            .and_then(|toolPackage| toolPackage.tools.first())
            .map(|tool| tool.script.clone())
    }

    #[allow(non_snake_case)]
    /// Returns the active state id selected for a package.
    pub fn getActivePackageStateId(&self, packageName: &str) -> Option<String> {
        self.pluginPackageManager.activePackageStateId(packageName)
    }

    #[allow(non_snake_case)]
    /// Returns all package definitions currently available to the manager.
    pub fn getAvailablePackages(&self) -> BTreeMap<String, ToolPackage> {
        self.pluginPackageManager.availablePackages()
    }

    #[allow(non_snake_case)]
    /// Returns MCP server packages registered with the package manager.
    pub fn getAvailableServerPackages(&self) -> BTreeMap<String, MCPServerConfig> {
        self.mcpManager.getRegisteredServers()
    }

    #[allow(non_snake_case)]
    /// Removes an MCP server package from package manager state.
    pub fn unregisterMCPServerPackage(&mut self, serverName: &str) -> bool {
        let normalizedServerName = self.normalizePackageName(serverName);
        let removed = self
            .availablePackagesMut()
            .remove(&normalizedServerName)
            .is_some();
        self.pluginPackageManager
            .deactivatePackage(&normalizedServerName);
        self.pluginPackageManager
            .clearActivePackageState(&normalizedServerName);
        self.cachedMcpTools.remove(&normalizedServerName);
        removed
    }

    #[allow(non_snake_case)]
    /// Returns cached MCP tool descriptions for a registered server.
    pub fn getCachedMcpTools(&self, serverName: &str) -> Vec<CachedMcpToolInfo> {
        MCPLocalServer::getInstance(&self.context)
            .getCachedTools(serverName)
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    /// Registers or replaces an available package definition.
    pub fn setAvailablePackage(&mut self, packageName: String, toolPackage: ToolPackage) {
        let normalizedPackageName = self.normalizePackageName(&packageName);
        self.availablePackagesMut()
            .insert(normalizedPackageName, toolPackage);
    }

    #[allow(non_snake_case)]
    /// Registers a loaded ToolPkg container and its subpackages.
    pub fn registerToolPkg(&mut self, loadResult: ToolPkgLoadResult) -> bool {
        let registered = self.pluginPackageManager.registerToolPkg(loadResult);
        if !registered {
            return false;
        }
        self.notifyToolPkgRuntimeChangeListeners();
        true
    }

    #[allow(non_snake_case)]
    /// Subscribes to ToolPkg runtime changes and immediately publishes current state.
    pub fn addToolPkgRuntimeChangeListener(&self, listener: ToolPkgRuntimeChangeListener) {
        self.toolPkgManager()
            .addToolPkgRuntimeChangeListener(listener, self.getEnabledToolPkgContainerRuntimes());
    }

    #[allow(non_snake_case)]
    pub(crate) fn notifyToolPkgRuntimeChangeListeners(&self) {
        self.toolPkgManager()
            .notifyToolPkgRuntimeChangeListeners(self.getEnabledToolPkgContainerRuntimes());
    }

    #[allow(non_snake_case)]
    fn unregisterPackageTools(&mut self, packageName: &str) {
        self.pluginPackageManager.deactivatePackage(packageName);
    }

    #[allow(non_snake_case)]
    /// Returns the external package storage path as display text.
    pub fn getExternalPackagesPath(&self) -> String {
        self.storePaths.packages_dir().to_string_lossy().to_string()
    }

    #[allow(non_snake_case)]
    /// Scans built-in, bundled external, and external package sources.
    pub fn loadAvailablePackages(&mut self) {
        let previousContainerNames = self
            .toolPkgManager()
            .getToolPkgContainerRuntimes()
            .into_iter()
            .map(|runtime| runtime.packageName)
            .collect::<BTreeSet<_>>();
        let assetSnapshot = self.scanBuiltInPackageAssets();
        self.syncBundledExternalImportRecords()
            .expect("Bundled external package sync must succeed before scanning external packages");
        let mergedSnapshot = self.scanExternalPackages(&assetSnapshot);
        let nextContainerNames = mergedSnapshot
            .toolPkgContainers
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        for packageName in previousContainerNames.union(&nextContainerNames) {
            self.toolPkgManager()
                .destroyDefaultToolPkgExecutionEngine(packageName);
        }
        self.applyPackageScanSnapshot(mergedSnapshot);
        self.notifyToolPkgRuntimeChangeListeners();
    }

    #[allow(non_snake_case)]
    /// Returns whether the ToolPkg protection secret is configured.
    pub fn isToolPkgProtectionSecretConfigured(&self) -> bool {
        operit_plugin_sdk::toolpkg::ToolPkgProtection::isSecretConfigured()
    }

    #[allow(non_snake_case)]
    /// Protects a local JS or ToolPkg artifact before marketplace upload.
    pub fn protectArtifactFile(
        &self,
        sourcePath: String,
        isToolPkg: bool,
    ) -> Result<Vec<u8>, String> {
        operit_plugin_sdk::toolpkg::ToolPkgProtection::protectArtifactFile(
            std::path::Path::new(&sourcePath),
            isToolPkg,
        )
    }

    #[allow(non_snake_case)]
    /// Returns package sources that can be exported or published.
    pub fn getPublishablePackageSources(&mut self) -> Vec<PublishablePackageSource> {
        let mut sources = Vec::new();

        for (packageName, toolPackage) in self.pluginPackageManager.availablePackages() {
            if toolPackage.is_built_in
                || self.toolPkgManager().hasSubpackage(&packageName)
                || self.toolPkgManager().isToolPkgContainer(&packageName)
            {
                continue;
            }
            let Some(sourceFile) = self.findPackageFile(&packageName) else {
                continue;
            };
            if !sourceFile.exists() || !sourceFile.is_file() {
                continue;
            }
            let displayName = {
                let resolved = toolPackage.display_name.resolve(false);
                if resolved.trim().is_empty() {
                    packageName.clone()
                } else {
                    resolved
                }
            };
            sources.push(PublishablePackageSource {
                packageName,
                displayName,
                description: toolPackage.description.resolve(false),
                author: toolPackage.author,
                sourcePath: sourceFile.to_string_lossy().to_string(),
                sourceFileName: sourceFile
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_default(),
                fileExtension: sourceFile
                    .extension()
                    .map(|value| value.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default(),
                isToolPkg: false,
                inferredVersion: None,
            });
        }

        for runtime in self.toolPkgManager().getToolPkgContainerRuntimes() {
            if runtime.sourceType != ToolPkgSourceType::EXTERNAL {
                continue;
            }
            let sourceFile = PathBuf::from(&runtime.sourcePath);
            if !sourceFile.exists() || !sourceFile.is_file() {
                continue;
            }
            let displayName = {
                let resolved = runtime.displayName.resolve(false);
                if resolved.trim().is_empty() {
                    runtime.packageName.clone()
                } else {
                    resolved
                }
            };
            sources.push(PublishablePackageSource {
                packageName: runtime.packageName,
                displayName,
                description: runtime.description.resolve(false),
                author: runtime.author,
                sourcePath: sourceFile.to_string_lossy().to_string(),
                sourceFileName: sourceFile
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_default(),
                fileExtension: sourceFile
                    .extension()
                    .map(|value| value.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default(),
                isToolPkg: true,
                inferredVersion: if runtime.version.trim().is_empty() {
                    None
                } else {
                    Some(runtime.version)
                },
            });
        }

        sources.sort_by(|left, right| {
            left.isToolPkg.cmp(&right.isToolPkg).then_with(|| {
                left.displayName
                    .to_lowercase()
                    .cmp(&right.displayName.to_lowercase())
            })
        });
        sources
    }

    #[allow(non_snake_case)]
    fn scanBuiltInPackageAssets(&self) -> PackageScanSnapshot {
        let results = self
            .toolHandler
            .runtimeSupport()
            .builtinPluginAssets()
            .iter()
            .map(|asset| self.parseBuiltInPackageAsset(asset))
            .collect::<Vec<_>>();
        self.mergePackageScanCandidateResults(results, None)
    }

    #[allow(non_snake_case)]
    fn scanExternalPackages(&mut self, baseSnapshot: &PackageScanSnapshot) -> PackageScanSnapshot {
        let packagesDir = self.storePaths.packages_dir();
        if let Err(error) = fs::create_dir_all(&packagesDir) {
            logPackageManagerError(format!(
                "External package directory creation failed: {}, error={error}",
                packagesDir.display()
            ));
            return baseSnapshot.clone();
        }
        let Ok(entries) = fs::read_dir(&packagesDir) else {
            logPackageManagerError(format!(
                "External package directory is unreadable: {}",
                packagesDir.display()
            ));
            return baseSnapshot.clone();
        };
        let mut files = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        files.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));

        let previousCache = self.externalPackageScanCache.clone();
        let mut nextCache = BTreeMap::new();
        let mut results = Vec::new();
        for file in files {
            let cacheKey = file.to_string_lossy().to_string();
            let signature = self.buildExternalPackageScanSignature(&file);
            let result = previousCache
                .get(&cacheKey)
                .filter(|entry| entry.signature == signature)
                .map(|entry| entry.result.clone())
                .unwrap_or_else(|| self.parseExternalPackageCandidate(&file));
            nextCache.insert(
                cacheKey,
                ExternalPackageScanCacheEntry {
                    signature,
                    result: result.clone(),
                },
            );
            results.push(result);
        }
        self.externalPackageScanCache = nextCache;
        self.mergePackageScanCandidateResults(results, Some(baseSnapshot))
    }

    #[allow(non_snake_case)]
    fn scanBundledExternalPackageCandidates(&mut self) -> Vec<PackageScanCandidateResult> {
        let previousCache = self.bundledExternalPackageScanCache.clone();
        let mut nextCache = BTreeMap::new();
        let mut results = Vec::new();
        for asset in self
            .toolHandler
            .runtimeSupport()
            .bundledExternalPluginAssets()
        {
            if !isPackageCandidateAssetName(asset.name) {
                continue;
            }
            let cacheKey = asset.name.to_string();
            let signature = buildBundledPluginAssetSignature(asset);
            let result = previousCache
                .get(&cacheKey)
                .filter(|entry| entry.signature == signature)
                .map(|entry| entry.result.clone())
                .unwrap_or_else(|| self.parseBundledExternalPackageAsset(asset));
            nextCache.insert(
                cacheKey,
                ExternalPackageScanCacheEntry {
                    signature,
                    result: result.clone(),
                },
            );
            results.push(result);
        }
        self.bundledExternalPackageScanCache = nextCache;
        results
    }

    #[allow(non_snake_case)]
    fn scanBundledExternalToolPkgCandidates(&mut self) -> Vec<PackageScanCandidateResult> {
        self.scanBundledExternalPackageCandidates()
            .into_iter()
            .filter(|result| result.toolPkgLoadResult.is_some())
            .collect()
    }

    #[allow(non_snake_case)]
    fn bundledExternalPackageCandidateFromScanResult(
        result: PackageScanCandidateResult,
    ) -> Option<BundledExternalPackageCandidate> {
        if let Some(package) = result.toolPackage {
            let toolCount = package.tools.len();
            return Some(BundledExternalPackageCandidate {
                packageName: package.name,
                displayName: package.display_name,
                description: package.description,
                author: package.author,
                packageKind: "script".to_string(),
                sourceFileName: packageSourceFileName(&result.sourcePath),
                sourcePath: result.sourcePath,
                isToolPkg: false,
                version: String::new(),
                category: package.category,
                toolCount,
                subpackageCount: 0,
            });
        }
        if let Some(loadResult) = result.toolPkgLoadResult {
            let runtime = loadResult.containerRuntime;
            let toolCount = loadResult.containerPackage.tools.len();
            let subpackageCount = runtime.subpackages.len();
            return Some(BundledExternalPackageCandidate {
                packageName: runtime.packageName,
                displayName: runtime.displayName,
                description: runtime.description,
                author: runtime.author,
                packageKind: "toolpkg".to_string(),
                sourceFileName: packageSourceFileName(&result.sourcePath),
                sourcePath: result.sourcePath,
                isToolPkg: true,
                version: runtime.version,
                category: "ToolPkg".to_string(),
                toolCount,
                subpackageCount,
            });
        }
        None
    }

    #[allow(non_snake_case)]
    fn parseBuiltInPackageAsset(&self, asset: &RuntimePluginAsset) -> PackageScanCandidateResult {
        let mut result = PackageScanCandidateResult {
            phase: "asset".to_string(),
            sourcePath: asset.name.to_string(),
            ..Default::default()
        };
        let lowerName = asset.name.to_ascii_lowercase();
        if lowerName.ends_with(".js") || lowerName.ends_with(".ts") {
            match std::str::from_utf8(asset.bytes)
                .map_err(|error| error.to_string())
                .and_then(|script| {
                    JsPackageLoader::parse(script).map(|package| ToolPackage {
                        is_built_in: true,
                        ..package
                    })
                }) {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => logPackageManagerError(format!(
                    "Built-in JavaScript package load error [{}]: {error}",
                    asset.name
                )),
            }
        } else if lowerName.ends_with(".hjson") || lowerName.ends_with(".json") {
            match std::str::from_utf8(asset.bytes)
                .map_err(|error| error.to_string())
                .and_then(|content| {
                    JsPackageLoader::parse_metadata(content, "").map(|package| ToolPackage {
                        is_built_in: true,
                        ..package
                    })
                }) {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => logPackageManagerError(format!(
                    "Built-in package metadata load error [{}]: {error}",
                    asset.name
                )),
            }
        } else if lowerName.ends_with(".toolpkg") {
            match self.loadToolPkgFromBuiltInAsset(asset.name, asset.bytes) {
                Ok(loadResult) => result.toolPkgLoadResult = Some(loadResult),
                Err(error) => logPackageManagerError(format!(
                    "Built-in ToolPkg package load error [{}]: {error}",
                    asset.name
                )),
            }
        }
        result
    }

    #[allow(non_snake_case)]
    fn parseBundledExternalPackageAsset(
        &self,
        asset: &RuntimePluginAsset,
    ) -> PackageScanCandidateResult {
        let mut result = PackageScanCandidateResult {
            phase: "bundled_external".to_string(),
            sourcePath: asset.name.to_string(),
            ..Default::default()
        };
        let lowerName = asset.name.to_ascii_lowercase();
        if lowerName.ends_with(".js") || lowerName.ends_with(".ts") {
            match std::str::from_utf8(asset.bytes)
                .map_err(|error| error.to_string())
                .and_then(|script| JsPackageLoader::parse(script))
            {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => logPackageManagerError(format!(
                    "Bundled external JavaScript package load error [{}]: {error}",
                    asset.name
                )),
            }
        } else if lowerName.ends_with(".hjson") || lowerName.ends_with(".json") {
            match std::str::from_utf8(asset.bytes)
                .map_err(|error| error.to_string())
                .and_then(|content| JsPackageLoader::parse_metadata(content, ""))
            {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => logPackageManagerError(format!(
                    "Bundled external package metadata load error [{}]: {error}",
                    asset.name
                )),
            }
        } else if lowerName.ends_with(".toolpkg") {
            match self.loadToolPkgFromBuiltInAsset(asset.name, asset.bytes) {
                Ok(loadResult) => result.toolPkgLoadResult = Some(loadResult),
                Err(error) => logPackageManagerError(format!(
                    "Bundled external ToolPkg package load error [{}]: {error}",
                    asset.name
                )),
            }
        }
        result
    }

    #[allow(non_snake_case)]
    fn parseExternalPackageCandidate(&self, path: &Path) -> PackageScanCandidateResult {
        let sourcePath = path.to_string_lossy().to_string();
        let mut result = PackageScanCandidateResult {
            phase: "external".to_string(),
            sourcePath: sourcePath.clone(),
            ..Default::default()
        };
        let lowerPath = sourcePath.to_ascii_lowercase();
        if lowerPath.ends_with(".js") || lowerPath.ends_with(".ts") {
            result.toolPackage = self.loadPackageFromJsFile(path);
        } else if lowerPath.ends_with(".hjson") {
            match fs::read_to_string(path).and_then(|content| {
                JsPackageLoader::parse_metadata(&content, "")
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            }) {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => logPackageManagerError(format!(
                    "External package metadata load error [{sourcePath}]: {error}"
                )),
            }
        } else if lowerPath.ends_with(".toolpkg") {
            match self.loadToolPkgFromExternalFile(path) {
                Ok(loadResult) => result.toolPkgLoadResult = Some(loadResult),
                Err(error) => logPackageManagerError(format!(
                    "External ToolPkg package load error [{sourcePath}]: {error}"
                )),
            }
        }
        result
    }

    #[allow(non_snake_case)]
    fn mergePackageScanCandidateResults(
        &self,
        candidateResults: Vec<PackageScanCandidateResult>,
        baseSnapshot: Option<&PackageScanSnapshot>,
    ) -> PackageScanSnapshot {
        let mut stagedAvailablePackages = baseSnapshot
            .map(|snapshot| snapshot.availablePackages.clone())
            .unwrap_or_default();
        let mut stagedToolPkgContainers = baseSnapshot
            .map(|snapshot| snapshot.toolPkgContainers.clone())
            .unwrap_or_default();
        let mut stagedToolPkgSubpackages = baseSnapshot
            .map(|snapshot| snapshot.toolPkgSubpackages.clone())
            .unwrap_or_default();

        for result in candidateResults {
            if let Some(packageMetadata) = result.toolPackage {
                if result.phase == "external"
                    && !Self::prepareExternalStandalonePackageOverride(
                        &packageMetadata.name,
                        &mut stagedAvailablePackages,
                        &mut stagedToolPkgContainers,
                        &mut stagedToolPkgSubpackages,
                    )
                {
                    logPackageManagerError(format!(
                        "Duplicate package name: {}, source={}",
                        packageMetadata.name, result.sourcePath
                    ));
                    continue;
                }
                if stagedAvailablePackages.contains_key(&packageMetadata.name) {
                    logPackageManagerError(format!(
                        "Duplicate package name: {}, source={}",
                        packageMetadata.name, result.sourcePath
                    ));
                } else {
                    stagedAvailablePackages.insert(packageMetadata.name.clone(), packageMetadata);
                }
            }
            if let Some(loadResult) = result.toolPkgLoadResult {
                if result.phase == "external"
                    && !Self::prepareExternalToolPkgOverride(
                        &loadResult,
                        &mut stagedAvailablePackages,
                        &mut stagedToolPkgContainers,
                        &mut stagedToolPkgSubpackages,
                    )
                {
                    logPackageManagerError(format!(
                        "Duplicate ToolPkg package name: {}, source={}",
                        loadResult.containerPackage.name, result.sourcePath
                    ));
                    continue;
                }
                if !Self::registerToolPkgInto(
                    loadResult,
                    &mut stagedAvailablePackages,
                    &mut stagedToolPkgContainers,
                    &mut stagedToolPkgSubpackages,
                ) {
                    logPackageManagerError(format!(
                        "ToolPkg package registration failed, source={}",
                        result.sourcePath
                    ));
                }
            }
        }

        PackageScanSnapshot {
            availablePackages: stagedAvailablePackages,
            toolPkgContainers: stagedToolPkgContainers,
            toolPkgSubpackages: stagedToolPkgSubpackages,
        }
    }

    #[allow(non_snake_case)]
    fn applyPackageScanSnapshot(&mut self, snapshot: PackageScanSnapshot) {
        self.pluginPackageManager
            .replaceAvailablePackages(snapshot.availablePackages);
        self.pluginPackageManager
            .replaceToolPkgRuntimes(snapshot.toolPkgContainers, snapshot.toolPkgSubpackages);
    }

    #[allow(non_snake_case)]
    fn registerToolPkgInto(
        loadResult: ToolPkgLoadResult,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) -> bool {
        let containerName = loadResult.containerPackage.name.clone();
        if availablePackagesTarget.contains_key(&containerName) {
            return false;
        }
        if loadResult
            .subpackagePackages
            .iter()
            .any(|subpackage| availablePackagesTarget.contains_key(&subpackage.name))
        {
            return false;
        }
        availablePackagesTarget.insert(containerName.clone(), loadResult.containerPackage);
        toolPkgContainersTarget.insert(containerName, loadResult.containerRuntime.clone());
        for subpackage in loadResult.subpackagePackages {
            availablePackagesTarget.insert(subpackage.name.clone(), subpackage);
        }
        for runtime in loadResult.containerRuntime.subpackages {
            toolPkgSubpackageByPackageNameTarget.insert(runtime.packageName.clone(), runtime);
        }
        true
    }

    #[allow(non_snake_case)]
    fn removeToolPkgContainerFromTargets(
        containerPackageName: &str,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) {
        let Some(runtime) = toolPkgContainersTarget.remove(containerPackageName) else {
            return;
        };
        availablePackagesTarget.remove(containerPackageName);
        for subpackage in runtime.subpackages {
            availablePackagesTarget.remove(&subpackage.packageName);
            toolPkgSubpackageByPackageNameTarget.remove(&subpackage.packageName);
        }
    }

    #[allow(non_snake_case)]
    fn prepareExternalStandalonePackageOverride(
        packageName: &str,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) -> bool {
        if let Some(existingContainer) = toolPkgContainersTarget.get(packageName).cloned() {
            if existingContainer.sourceType != ToolPkgSourceType::ASSET {
                return false;
            }
            Self::removeToolPkgContainerFromTargets(
                &existingContainer.packageName,
                availablePackagesTarget,
                toolPkgContainersTarget,
                toolPkgSubpackageByPackageNameTarget,
            );
            return true;
        }

        if let Some(existingSubpackage) = toolPkgSubpackageByPackageNameTarget
            .get(packageName)
            .cloned()
        {
            let Some(ownerContainer) = toolPkgContainersTarget
                .get(&existingSubpackage.containerPackageName)
                .cloned()
            else {
                return false;
            };
            if ownerContainer.sourceType != ToolPkgSourceType::ASSET {
                return false;
            }
            Self::removeToolPkgContainerFromTargets(
                &ownerContainer.packageName,
                availablePackagesTarget,
                toolPkgContainersTarget,
                toolPkgSubpackageByPackageNameTarget,
            );
            return true;
        }

        let Some(existingPackage) = availablePackagesTarget.get(packageName) else {
            return true;
        };
        if !existingPackage.is_built_in {
            return false;
        }
        availablePackagesTarget.remove(packageName);
        true
    }

    #[allow(non_snake_case)]
    fn prepareExternalToolPkgOverride(
        loadResult: &ToolPkgLoadResult,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) -> bool {
        let mut builtInContainersToRemove = BTreeSet::new();
        let mut builtInStandalonePackagesToRemove = BTreeSet::new();
        let mut conflictingNames = Vec::new();
        conflictingNames.push(loadResult.containerPackage.name.clone());
        conflictingNames.extend(
            loadResult
                .subpackagePackages
                .iter()
                .map(|subpackage| subpackage.name.clone()),
        );

        for packageName in conflictingNames {
            if let Some(existingContainer) = toolPkgContainersTarget.get(&packageName) {
                if existingContainer.sourceType != ToolPkgSourceType::ASSET {
                    return false;
                }
                builtInContainersToRemove.insert(existingContainer.packageName.clone());
                continue;
            }

            if let Some(existingSubpackage) = toolPkgSubpackageByPackageNameTarget.get(&packageName)
            {
                let Some(ownerContainer) =
                    toolPkgContainersTarget.get(&existingSubpackage.containerPackageName)
                else {
                    return false;
                };
                if ownerContainer.sourceType != ToolPkgSourceType::ASSET {
                    return false;
                }
                builtInContainersToRemove.insert(ownerContainer.packageName.clone());
                continue;
            }

            if let Some(existingPackage) = availablePackagesTarget.get(&packageName) {
                if !existingPackage.is_built_in {
                    return false;
                }
                builtInStandalonePackagesToRemove.insert(packageName);
            }
        }

        for containerPackageName in builtInContainersToRemove {
            Self::removeToolPkgContainerFromTargets(
                &containerPackageName,
                availablePackagesTarget,
                toolPkgContainersTarget,
                toolPkgSubpackageByPackageNameTarget,
            );
        }
        for packageName in builtInStandalonePackagesToRemove {
            availablePackagesTarget.remove(&packageName);
        }
        true
    }

    #[allow(non_snake_case)]
    fn buildExternalPackageScanSignature(&self, file: &Path) -> String {
        let metadata = fs::metadata(file).ok();
        format!(
            "{}|{}|{}",
            file.to_string_lossy(),
            metadata.as_ref().map(|value| value.len()).unwrap_or(0),
            metadata
                .and_then(|value| value.modified().ok())
                .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|value| value.as_millis())
                .unwrap_or(0)
        )
    }

    #[allow(non_snake_case)]
    fn syncBundledExternalImportRecords(&mut self) -> Result<(), String> {
        self.storePaths
            .ensure_packages_dir()
            .map_err(|error| error.to_string())?;
        let mut records = self.decodeBundledExternalImportRecordsFromPrefs()?;
        let bundledResults = self.scanBundledExternalPackageCandidates();
        let mut recordsChanged =
            self.adoptMatchingBundledExternalImportRecords(&mut records, &bundledResults)?;
        let mut bundledResultsByFileName = BTreeMap::new();
        for result in bundledResults {
            bundledResultsByFileName.insert(packageSourceFileName(&result.sourcePath), result);
        }

        let packagesDir = self.storePaths.packages_dir();
        let mut packageNamesToRemove = Vec::new();
        let packageNames = records.keys().cloned().collect::<Vec<_>>();
        for packageName in packageNames {
            let Some(record) = records.get(&packageName).cloned() else {
                continue;
            };
            let destinationFile = packagesDir.join(&record.destinationFileName);
            if !destinationFile.exists() || !destinationFile.is_file() {
                packageNamesToRemove.push(packageName);
                recordsChanged = true;
                continue;
            }
            let Some(result) = bundledResultsByFileName.get(&record.sourceFileName) else {
                packageNamesToRemove.push(packageName);
                recordsChanged = true;
                continue;
            };
            if Self::packageNameFromScanResult(result).as_deref()
                != Some(record.packageName.as_str())
            {
                packageNamesToRemove.push(packageName);
                recordsChanged = true;
                continue;
            }

            let Some(sourceAsset) = bundledExternalPluginAssetByName(
                self.toolHandler.runtimeSupport().as_ref(),
                &result.sourcePath,
            ) else {
                packageNamesToRemove.push(packageName);
                recordsChanged = true;
                continue;
            };
            let sourceSignature = sha256Hex(sourceAsset.bytes);
            if sourceSignature != record.sourceSignature {
                fs::write(&destinationFile, sourceAsset.bytes)
                    .map_err(|error| error.to_string())?;
                if let Some(recordToUpdate) = records.get_mut(&record.packageName) {
                    recordToUpdate.sourceSignature = sourceSignature;
                }
                self.externalPackageScanCache
                    .remove(&destinationFile.to_string_lossy().to_string());
                recordsChanged = true;
            }
        }

        for packageName in packageNamesToRemove {
            records.remove(&packageName);
        }
        if recordsChanged {
            self.saveBundledExternalImportRecords(&records)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn adoptMatchingBundledExternalImportRecords(
        &self,
        records: &mut BTreeMap<String, BundledExternalImportRecord>,
        bundledResults: &[PackageScanCandidateResult],
    ) -> Result<bool, String> {
        let packagesDir = self.storePaths.packages_dir();
        let mut changed = false;
        for result in bundledResults {
            let Some(packageName) = Self::packageNameFromScanResult(result) else {
                continue;
            };
            if records.contains_key(&packageName) {
                continue;
            }
            let sourceFileName = packageSourceFileName(&result.sourcePath);
            let destinationFile = packagesDir.join(&sourceFileName);
            if !destinationFile.exists() || !destinationFile.is_file() {
                continue;
            }
            let Some(sourceAsset) = bundledExternalPluginAssetByName(
                self.toolHandler.runtimeSupport().as_ref(),
                &result.sourcePath,
            ) else {
                continue;
            };
            let destinationBytes = fs::read(&destinationFile).map_err(|error| error.to_string())?;
            if destinationBytes != sourceAsset.bytes {
                continue;
            }
            let sourceSignature = sha256Hex(sourceAsset.bytes);
            records.insert(
                packageName.clone(),
                BundledExternalImportRecord {
                    packageName,
                    sourceFileName: sourceFileName.clone(),
                    destinationFileName: sourceFileName,
                    sourceSignature,
                },
            );
            changed = true;
        }
        Ok(changed)
    }

    #[allow(non_snake_case)]
    fn packageNameFromScanResult(result: &PackageScanCandidateResult) -> Option<String> {
        if let Some(package) = &result.toolPackage {
            return Some(package.name.clone());
        }
        if let Some(loadResult) = &result.toolPkgLoadResult {
            return Some(loadResult.containerPackage.name.clone());
        }
        None
    }

    #[allow(non_snake_case)]
    fn buildFileContentSignature(file: &Path) -> Result<String, std::io::Error> {
        let bytes = fs::read(file)?;
        Ok(format!("{:x}", Sha256::digest(&bytes)))
    }

    #[allow(non_snake_case)]
    fn filesHaveSameContent(left: &Path, right: &Path) -> Result<bool, std::io::Error> {
        Ok(fs::read(left)? == fs::read(right)?)
    }

    #[allow(non_snake_case)]
    /// Imports a package file from external storage into package storage.
    pub fn addPackageFileFromExternalStorage(&mut self, filePath: &str) -> String {
        let file = PathBuf::from(filePath);
        if !file.exists() || !file.is_file() {
            return format!("Cannot access file at path: {filePath}");
        }

        let lowerPath = filePath.to_ascii_lowercase();
        let isJsLike = lowerPath.ends_with(".js") || lowerPath.ends_with(".ts");
        let isHjson = lowerPath.ends_with(".hjson");
        let isToolPkg = lowerPath.ends_with(".toolpkg");
        if !isJsLike && !isHjson && !isToolPkg {
            return "Only HJSON, JavaScript (.js), TypeScript (.ts) and ToolPkg (.toolpkg) package files are supported"
                .to_string();
        }

        if isToolPkg {
            let loadResult = match self.loadToolPkgFromExternalFile(&file) {
                Ok(value) => value,
                Err(error) => return format!("Error importing package: {error}"),
            };
            let packageName = loadResult.containerPackage.name.clone();
            if !self
                .toolPkgManager()
                .canRegisterToolPkg(&loadResult, self.availablePackages())
            {
                return format!(
                    "A package with name '{}' already exists in available packages",
                    packageName
                );
            }
            if let Err(error) = self.storePaths.ensure_packages_dir() {
                return format!("Error importing package: {error}");
            }
            let Some(fileName) = file.file_name() else {
                return "Error importing package: invalid file name".to_string();
            };
            let destinationFile = self.storePaths.packages_dir().join(fileName);
            if file != destinationFile {
                if let Err(error) = fs::copy(&file, &destinationFile) {
                    return format!("Error importing package: {error}");
                }
            }
            let importedLoadResult = match self.loadToolPkgFromExternalFile(&destinationFile) {
                Ok(value) => value,
                Err(error) => return format!("Error importing package: {error}"),
            };
            if !self.registerToolPkg(importedLoadResult) {
                return format!(
                    "A package with name '{}' already exists in available packages",
                    packageName
                );
            }
            return format!(
                "Successfully imported package: {}\nStored at: {}",
                packageName,
                destinationFile.to_string_lossy()
            );
        }

        let packageMetadata = if isHjson {
            let content = match fs::read_to_string(&file) {
                Ok(value) => value,
                Err(error) => return format!("Error importing package: {error}"),
            };
            match JsPackageLoader::parse_metadata(&content, "") {
                Ok(value) => value,
                Err(error) => return format!("Error importing package: {error}"),
            }
        } else {
            match self.loadPackageFromJsFile(&file) {
                Some(value) => value,
                None => {
                    return format!(
                        "Failed to parse {} package file",
                        if lowerPath.ends_with(".ts") {
                            "TypeScript"
                        } else {
                            "JavaScript"
                        }
                    )
                }
            }
        };

        if self.availablePackages().contains_key(&packageMetadata.name)
            || self
                .toolPkgManager()
                .isToolPkgContainer(&packageMetadata.name)
            || self.toolPkgManager().hasSubpackage(&packageMetadata.name)
        {
            return format!(
                "A package with name '{}' already exists in available packages",
                packageMetadata.name
            );
        }

        if let Err(error) = self.storePaths.ensure_packages_dir() {
            return format!("Error importing package: {error}");
        }
        let Some(fileName) = file.file_name() else {
            return "Error importing package: invalid file name".to_string();
        };
        let destinationFile = self.storePaths.packages_dir().join(fileName);
        if file != destinationFile {
            if let Err(error) = fs::copy(&file, &destinationFile) {
                return format!("Error importing package: {error}");
            }
        }

        self.pluginPackageManager.registerPackage(ToolPackage {
            is_built_in: false,
            ..packageMetadata.clone()
        });
        format!(
            "Successfully imported package: {}\nStored at: {}",
            packageMetadata.name,
            destinationFile.to_string_lossy()
        )
    }

    #[allow(non_snake_case)]
    /// Persists the complete enabled package name list.
    pub fn setEnabledPackageNames(
        &self,
        packageNames: &[String],
    ) -> Result<(), PreferencesDataStoreError> {
        let result = self.saveEnabledPackageNames(packageNames);
        if result.is_ok() {
            self.notifyToolPkgRuntimeChangeListeners();
        }
        result
    }

    #[allow(non_snake_case)]
    /// Registers or replaces an MCP server package definition.
    pub fn setAvailableServerPackage(&mut self, serverName: String, serverConfig: MCPServerConfig) {
        self.mcpManager.registerServer(serverName, serverConfig);
    }

    #[allow(non_snake_case)]
    /// Stores cached MCP tool descriptions for a server package.
    pub fn setCachedMcpTools(&mut self, serverName: String, tools: Vec<CachedMcpToolInfo>) {
        self.cachedMcpTools.insert(serverName, tools);
    }

    #[allow(non_snake_case)]
    /// Returns the main script for an enabled ToolPkg container.
    pub fn getToolPkgMainScriptInternal(&self, containerPackageName: &str) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        self.toolPkgManager().getToolPkgMainScriptInternal(
            &normalizedContainerPackageName,
            &self.getEnabledPackageNames(),
        )
    }

    #[allow(non_snake_case)]
    pub(crate) fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        normalizedResourcePath: &str,
    ) -> Option<Vec<u8>> {
        let resourceFile = self.resolveToolPkgResourceFile(runtime, normalizedResourcePath)?;
        if !resourceFile.is_file() {
            return None;
        }
        let bytes = fs::read(resourceFile).ok()?;
        operit_plugin_sdk::toolpkg::ToolPkgProtection::decryptIfNeeded(&bytes).ok()
    }

    #[allow(non_snake_case)]
    /// Reads a text resource from a ToolPkg container or subpackage.
    pub fn readToolPkgTextResource(
        &self,
        packageNameOrSubpackageId: &str,
        resourcePath: &str,
        preferEnabledContainer: bool,
    ) -> Option<String> {
        let normalizedPackageName = self.normalizePackageName(packageNameOrSubpackageId);
        ToolPkgPackageService::new(self).readToolPkgTextResource(
            &normalizedPackageName,
            resourcePath,
            preferEnabledContainer,
        )
    }

    #[allow(non_snake_case)]
    /// Copies a ToolPkg resource selected by subpackage id to a file.
    pub fn copyToolPkgResourceToFileBySubpackageId(
        &self,
        subpackageId: &str,
        resourceKey: &str,
        destinationFile: &Path,
        preferEnabledContainer: bool,
    ) -> bool {
        ToolPkgPackageService::new(self).copyToolPkgResourceToFileBySubpackageId(
            subpackageId,
            resourceKey,
            destinationFile,
            preferEnabledContainer,
        )
    }

    #[allow(non_snake_case)]
    /// Copies a ToolPkg resource from a container to a file.
    pub fn copyToolPkgResourceToFile(
        &self,
        containerPackageName: &str,
        resourceKey: &str,
        destinationFile: &Path,
    ) -> bool {
        ToolPkgPackageService::new(self).copyToolPkgResourceToFile(
            containerPackageName,
            resourceKey,
            destinationFile,
        )
    }

    #[allow(non_snake_case)]
    /// Returns the output file name declared for a ToolPkg resource.
    pub fn getToolPkgResourceOutputFileName(
        &self,
        packageNameOrSubpackageId: &str,
        resourceKey: &str,
        preferEnabledContainer: bool,
    ) -> Option<String> {
        ToolPkgPackageService::new(self).getToolPkgResourceOutputFileName(
            packageNameOrSubpackageId,
            resourceKey,
            preferEnabledContainer,
        )
    }

    #[allow(non_snake_case)]
    /// Returns Compose DSL script text selected through a ToolPkg subpackage id.
    pub fn getToolPkgComposeDslScriptBySubpackageId(
        &self,
        subpackageId: &str,
        uiModuleId: Option<&str>,
        preferEnabledContainer: bool,
    ) -> Option<String> {
        let normalizedSubpackageId = self.normalizePackageName(subpackageId);
        ToolPkgPackageService::new(self).getToolPkgComposeDslScriptBySubpackageId(
            &normalizedSubpackageId,
            uiModuleId,
            preferEnabledContainer,
        )
    }

    #[allow(non_snake_case)]
    /// Returns Compose DSL script text for a ToolPkg UI module.
    pub fn getToolPkgComposeDslScript(
        &self,
        containerPackageName: &str,
        uiModuleId: Option<&str>,
    ) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        ToolPkgPackageService::new(self)
            .getToolPkgComposeDslScript(&normalizedContainerPackageName, uiModuleId)
    }

    #[allow(non_snake_case)]
    /// Returns the Compose DSL screen path for a ToolPkg UI module.
    pub fn getToolPkgComposeDslScreenPath(
        &self,
        containerPackageName: &str,
        uiModuleId: Option<&str>,
    ) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        ToolPkgPackageService::new(self)
            .getToolPkgComposeDslScreenPath(&normalizedContainerPackageName, uiModuleId)
    }

    #[allow(non_snake_case)]
    /// Runs a main hook exported by a ToolPkg container.
    pub fn runToolPkgMainHook(
        &self,
        containerPackageName: &str,
        functionName: &str,
        event: &str,
        eventName: Option<&str>,
        pluginId: Option<&str>,
        inlineFunctionSource: Option<&str>,
        eventPayload: serde_json::Value,
        executionContextKey: Option<&str>,
        runtimeKind: Option<&str>,
        onIntermediateResult: Option<std::sync::Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Result<Option<String>, String> {
        self.toolPkgManager().dispatchToolPkgHook(
            &self.getEnabledPackageNames(),
            ToolPkgHookInvocation {
                containerPackageName: self.normalizePackageName(containerPackageName),
                functionName: functionName.to_string(),
                event: event.to_string(),
                eventName: eventName.map(str::to_string),
                pluginId: pluginId.map(str::to_string),
                inlineFunctionSource: inlineFunctionSource.map(str::to_string),
                eventPayload,
                executionContextKey: executionContextKey.map(str::to_string),
                runtimeKind: runtimeKind.map(str::to_string),
                envOverrides: BTreeMap::new(),
                timestampMs: operit_host_api::TimeUtils::currentTimeMillis(),
                timeoutSec: 60,
                dispatchIntermediateOnMain: true,
                onIntermediateResult,
            },
        )
    }

    #[allow(non_snake_case)]
    /// Activates an MCP server package and returns its system prompt contribution.
    pub fn useMCPServer(&mut self, serverName: &str) -> String {
        if !self.isRegisteredMCPServer(serverName) {
            return format!(
                "MCP server '{}' does not exist or is not registered.",
                serverName
            );
        }
        let Some(serverConfig) = self
            .mcpManager
            .getRegisteredServers()
            .get(serverName)
            .cloned()
        else {
            return format!("Cannot get MCP server configuration: {}", serverName);
        };
        let mcpLoadResult = MCPPackage::loadFromServer(&self.context, serverConfig);
        let Some(mcpPackage) = mcpLoadResult.mcpPackage else {
            return mcpLoadResult
                .errorMessage
                .map(|message| {
                    format!("Cannot connect to MCP server '{}': {}", serverName, message)
                })
                .unwrap_or_else(|| format!("Cannot connect to MCP server: {}", serverName));
        };
        let toolPackage = mcpPackage.toToolPackage();
        self.pluginPackageManager
            .registerPackage(toolPackage.clone());
        self.activatePackage(serverName);
        self.generateMCPSystemPrompt(&toolPackage, serverName)
    }

    #[allow(non_snake_case)]
    fn isRegisteredMCPServer(&self, serverName: &str) -> bool {
        self.mcpManager.isServerRegistered(serverName)
    }

    #[allow(non_snake_case)]
    pub(crate) fn normalizePackageName(&self, packageName: &str) -> String {
        packageName.trim().to_string()
    }

    #[allow(non_snake_case)]
    fn normalizeEnabledPackageNames(&self, packageNames: &[String]) -> Vec<String> {
        let mut normalized = BTreeSet::new();
        for original in packageNames {
            let canonical = self.normalizePackageName(original);
            if !canonical.trim().is_empty() {
                normalized.insert(canonical);
            }
        }
        normalized.into_iter().collect()
    }

    #[allow(non_snake_case)]
    fn decodeEnabledPackageNamesFromPrefs(&self) -> Vec<String> {
        let key = stringPreferencesKey(ENABLED_PACKAGES_KEY);
        let preferences = match self.dataStore.data() {
            Ok(preferences) => preferences,
            Err(_) => return Vec::new(),
        };
        let Some(packagesJson) = preferences.get(&key) else {
            return Vec::new();
        };
        let rawPackages = match serde_json::from_str::<Vec<String>>(packagesJson) {
            Ok(rawPackages) => rawPackages,
            Err(_) => return Vec::new(),
        };
        self.normalizeEnabledPackageNames(&rawPackages)
    }

    #[allow(non_snake_case)]
    fn decodeDisabledPackageNamesFromPrefs(&self) -> Vec<String> {
        let key = stringPreferencesKey(DISABLED_PACKAGES_KEY);
        let preferences = match self.dataStore.data() {
            Ok(preferences) => preferences,
            Err(_) => return Vec::new(),
        };
        let Some(packagesJson) = preferences.get(&key) else {
            return Vec::new();
        };
        let rawPackages = match serde_json::from_str::<Vec<String>>(packagesJson) {
            Ok(rawPackages) => rawPackages,
            Err(_) => return Vec::new(),
        };
        self.normalizeEnabledPackageNames(&rawPackages)
    }

    #[allow(non_snake_case)]
    fn decodeToolPkgSubpackageStatesFromPrefs(&self) -> BTreeMap<String, bool> {
        let key = stringPreferencesKey(TOOLPKG_SUBPACKAGE_STATES_KEY);
        let preferences = match self.dataStore.data() {
            Ok(preferences) => preferences,
            Err(_) => return BTreeMap::new(),
        };
        let Some(statesJson) = preferences.get(&key) else {
            return BTreeMap::new();
        };
        serde_json::from_str::<BTreeMap<String, bool>>(statesJson).unwrap_or_default()
    }

    #[allow(non_snake_case)]
    fn normalizeToolPkgSubpackageStates(
        &self,
        states: &BTreeMap<String, bool>,
    ) -> BTreeMap<String, bool> {
        let mut normalized = BTreeMap::new();
        for (packageName, enabled) in states {
            let normalizedPackageName = self.normalizePackageName(packageName);
            if !normalizedPackageName.trim().is_empty() {
                normalized.insert(normalizedPackageName, *enabled);
            }
        }
        normalized
    }

    #[allow(non_snake_case)]
    pub(crate) fn saveToolPkgSubpackageStates(
        &self,
        states: &BTreeMap<String, bool>,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedStates = self.normalizeToolPkgSubpackageStates(states);
        let updatedJson = serde_json::to_string(&normalizedStates)?;
        self.dataStore.edit(|preferences| {
            preferences.set(
                &stringPreferencesKey(TOOLPKG_SUBPACKAGE_STATES_KEY),
                updatedJson,
            );
        })
    }

    #[allow(non_snake_case)]
    pub(crate) fn saveEnabledPackageNames(
        &self,
        enabledPackageNames: &[String],
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackages = self.normalizeEnabledPackageNames(enabledPackageNames);
        let updatedJson = serde_json::to_string(&normalizedPackages)?;
        self.dataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey(ENABLED_PACKAGES_KEY), updatedJson);
        })
    }

    #[allow(non_snake_case)]
    fn saveDisabledPackageNames(
        &self,
        disabledPackageNames: &[String],
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackages = self.normalizeEnabledPackageNames(disabledPackageNames);
        let updatedJson = serde_json::to_string(&normalizedPackages)?;
        self.dataStore.edit(|preferences| {
            preferences.set(&stringPreferencesKey(DISABLED_PACKAGES_KEY), updatedJson);
        })
    }

    #[allow(non_snake_case)]
    fn decodeBundledExternalImportRecordsFromPrefs(
        &self,
    ) -> Result<BTreeMap<String, BundledExternalImportRecord>, String> {
        let key = stringPreferencesKey(BUNDLED_EXTERNAL_IMPORTS_KEY);
        let preferences = self.dataStore.data().map_err(|error| error.to_string())?;
        let Some(recordsJson) = preferences.get(&key) else {
            return Ok(BTreeMap::new());
        };
        serde_json::from_str::<BTreeMap<String, BundledExternalImportRecord>>(recordsJson)
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn saveBundledExternalImportRecords(
        &self,
        records: &BTreeMap<String, BundledExternalImportRecord>,
    ) -> Result<(), String> {
        let updatedJson = serde_json::to_string(records).map_err(|error| error.to_string())?;
        self.dataStore
            .edit(|preferences| {
                preferences.set(
                    &stringPreferencesKey(BUNDLED_EXTERNAL_IMPORTS_KEY),
                    updatedJson,
                );
            })
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    fn upsertBundledExternalImportRecord(
        &self,
        record: BundledExternalImportRecord,
    ) -> Result<(), String> {
        let mut records = self.decodeBundledExternalImportRecordsFromPrefs()?;
        records.insert(record.packageName.clone(), record);
        self.saveBundledExternalImportRecords(&records)
    }

    #[allow(non_snake_case)]
    fn removeBundledExternalImportRecord(&self, packageName: &str) -> Result<(), String> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let mut records = self.decodeBundledExternalImportRecordsFromPrefs()?;
        if records.remove(&normalizedPackageName).is_some() {
            self.saveBundledExternalImportRecords(&records)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    pub(crate) fn removeFromDisabledPackages(
        &self,
        packageName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let mut disabledPackageNames =
            BTreeSet::from_iter(self.decodeDisabledPackageNamesFromPrefs());
        if disabledPackageNames.remove(&normalizedPackageName) {
            let names = disabledPackageNames.into_iter().collect::<Vec<_>>();
            self.saveDisabledPackageNames(&names)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    pub(crate) fn addToDisabledIfDefaultEnabled(
        &self,
        packageName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let Some(toolPackage) = self.availablePackages().get(&normalizedPackageName) else {
            return Ok(());
        };
        if !toolPackage.is_built_in || !toolPackage.enabled_by_default {
            return Ok(());
        }
        let mut disabledPackageNames =
            BTreeSet::from_iter(self.decodeDisabledPackageNamesFromPrefs());
        if disabledPackageNames.insert(normalizedPackageName) {
            let names = disabledPackageNames.into_iter().collect::<Vec<_>>();
            self.saveDisabledPackageNames(&names)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn removeFromCachesAfterDelete(&mut self, packageName: &str) {
        self.pluginPackageManager.removePackage(packageName);
    }

    #[allow(non_snake_case)]
    fn findPackageFile(&mut self, packageName: &str) -> Option<PathBuf> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let packagesDir = self.storePaths.packages_dir();
        if !packagesDir.exists() {
            return None;
        }

        if let Some(containerRuntime) = self
            .toolPkgManager()
            .getToolPkgContainerRuntime(&normalizedPackageName)
        {
            if containerRuntime.sourceType == ToolPkgSourceType::EXTERNAL {
                let candidate = PathBuf::from(containerRuntime.sourcePath);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        let jsFile = packagesDir.join(format!("{}.js", normalizedPackageName));
        if jsFile.exists() {
            return Some(jsFile);
        }

        let entries = fs::read_dir(&packagesDir).ok()?;
        for entry in entries.flatten() {
            let file = entry.path();
            if !file.is_file() {
                continue;
            }
            let lowerName = file.to_string_lossy().to_ascii_lowercase();
            if lowerName.ends_with(".js") {
                if let Some(loadedPackage) = self.loadPackageFromJsFile(&file) {
                    if loadedPackage.name == normalizedPackageName {
                        return Some(file);
                    }
                }
            }
        }

        None
    }

    #[allow(non_snake_case)]
    fn generatePackageSystemPrompt(&self, toolPackage: &ToolPackage) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("Using package: {}\n", toolPackage.name));
        prompt.push_str(&format!("Use Time: {}\n", currentUseTime()));
        prompt.push_str(&format!(
            "Description: {}\n\n",
            toolPackage.description.resolve(false)
        ));
        prompt.push_str("Available tools in this package:\n");

        for tool in &toolPackage.tools {
            if tool.advice {
                prompt.push_str(&format!(
                    "- (advice): {}\n",
                    tool.description.resolve(false)
                ));
            } else {
                prompt.push_str(&format!(
                    "- {}:{}: {}\n",
                    toolPackage.name,
                    tool.name,
                    tool.description.resolve(false)
                ));
            }
            if !tool.parameters.is_empty() {
                prompt.push_str("  Parameters:\n");
                for parameter in &tool.parameters {
                    let requiredText = if parameter.required {
                        "(required)"
                    } else {
                        "(optional)"
                    };
                    prompt.push_str(&format!(
                        "  - {} {}: {}\n",
                        parameter.name,
                        requiredText,
                        parameter.description.resolve(false)
                    ));
                }
            }
            prompt.push('\n');
        }

        prompt
    }

    #[allow(non_snake_case)]
    fn loadPackageFromJsFile(&self, file: &Path) -> Option<ToolPackage> {
        JsPackageLoader::load_from_file(file).ok()
    }

    #[allow(non_snake_case)]
    fn loadToolPkgFromExternalFile(&self, file: &Path) -> Result<ToolPkgLoadResult, String> {
        ToolPkgLoader::loadToolPkgFromExternalFile(
            file,
            self.jsEngine.as_ref(),
            |packageName, error| {
                AppLogger::e(
                    PACKAGE_MANAGER_LOG_TAG,
                    &format!("ToolPkg package load error [{packageName}]: {error}"),
                );
            },
        )
    }

    #[allow(non_snake_case)]
    fn loadToolPkgFromBuiltInAsset(
        &self,
        assetName: &str,
        bytes: &'static [u8],
    ) -> Result<ToolPkgLoadResult, String> {
        ToolPkgLoader::loadToolPkgFromBuiltInAsset(
            assetName,
            bytes,
            self.jsEngine.as_ref(),
            |packageName, error| {
                AppLogger::e(
                    PACKAGE_MANAGER_LOG_TAG,
                    &format!("Built-in ToolPkg package load error [{packageName}]: {error}"),
                );
            },
        )
    }

    #[allow(non_snake_case)]
    fn selectToolPackageState(&mut self, toolPackage: &ToolPackage) -> ToolPackage {
        self.pluginPackageManager
            .selectPackageState(&toolPackage.name)
            .expect("registered package must exist while selecting package state")
    }

    #[allow(non_snake_case)]
    fn generateMCPSystemPrompt(&self, toolPackage: &ToolPackage, serverName: &str) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!("Using MCP server: {}\n", serverName));
        prompt.push_str(&format!("Time: {}\n", currentUseTime()));
        prompt.push_str(&format!(
            "Description: {}\n\n",
            toolPackage.description.resolve(false)
        ));
        prompt.push_str("Available tools:\n");

        for tool in &toolPackage.tools {
            prompt.push_str(&format!(
                "- {}:{}: {}\n",
                serverName,
                tool.name,
                tool.description.resolve(false)
            ));
            if !tool.parameters.is_empty() {
                prompt.push_str("  Parameters:\n");
                for parameter in &tool.parameters {
                    let requiredText = if parameter.required {
                        "(required)"
                    } else {
                        "(optional)"
                    };
                    prompt.push_str(&format!(
                        "  - {} {}: {}\n",
                        parameter.name,
                        requiredText,
                        parameter.description.resolve(false)
                    ));
                }
            }
            prompt.push('\n');
        }

        prompt
    }
}

impl ToolPkgPackageHost for RuntimePackageManager {
    /// Ensures package state is initialized before an SDK ToolPkg query.
    #[allow(non_snake_case)]
    fn ensureInitialized(&self) {
        RuntimePackageManager::ensureInitialized(self);
    }

    /// Normalizes one package name using RuntimePackageManager rules.
    #[allow(non_snake_case)]
    fn normalizePackageName(&self, packageName: &str) -> String {
        RuntimePackageManager::normalizePackageName(self, packageName)
    }

    /// Returns registered ToolPkg container runtimes.
    #[allow(non_snake_case)]
    fn toolPkgContainersInternal(&self) -> BTreeMap<String, ToolPkgContainerRuntime> {
        RuntimePackageManager::toolPkgContainersInternal(self)
    }

    /// Returns registered ToolPkg subpackage runtimes.
    #[allow(non_snake_case)]
    fn toolPkgSubpackageByPackageNameInternal(&self) -> BTreeMap<String, ToolPkgSubpackageRuntime> {
        RuntimePackageManager::toolPkgSubpackageByPackageNameInternal(self)
    }

    /// Resolves one ToolPkg subpackage runtime.
    #[allow(non_snake_case)]
    fn resolveToolPkgSubpackageRuntimeInternal(
        &self,
        packageName: &str,
    ) -> Option<ToolPkgSubpackageRuntime> {
        RuntimePackageManager::resolveToolPkgSubpackageRuntimeInternal(self, packageName)
    }

    /// Returns enabled package names as a set.
    #[allow(non_snake_case)]
    fn getEnabledPackageNameSetInternal(&self) -> BTreeSet<String> {
        RuntimePackageManager::getEnabledPackageNameSetInternal(self)
    }

    /// Returns enabled package names in stable order.
    #[allow(non_snake_case)]
    fn getEnabledPackageNames(&self) -> Vec<String> {
        RuntimePackageManager::getEnabledPackageNames(self)
    }

    /// Returns persisted ToolPkg subpackage states.
    #[allow(non_snake_case)]
    fn getToolPkgSubpackageStatesInternal(&self) -> BTreeMap<String, bool> {
        RuntimePackageManager::getToolPkgSubpackageStatesInternal(self)
    }

    /// Persists enabled package names for SDK state changes.
    #[allow(non_snake_case)]
    fn saveEnabledPackageNames(&self, packageNames: &[String]) -> Result<(), String> {
        RuntimePackageManager::saveEnabledPackageNames(self, packageNames)
            .map_err(|error| error.to_string())
    }

    /// Persists ToolPkg subpackage states for SDK state changes.
    #[allow(non_snake_case)]
    fn saveToolPkgSubpackageStates(&self, states: &BTreeMap<String, bool>) -> Result<(), String> {
        RuntimePackageManager::saveToolPkgSubpackageStates(self, states)
            .map_err(|error| error.to_string())
    }

    /// Returns whether one package is enabled.
    #[allow(non_snake_case)]
    fn isPackageEnabled(&self, packageName: &str) -> bool {
        RuntimePackageManager::isPackageEnabled(self, packageName)
    }

    /// Resolves one ToolPkg resource to a cached host file.
    #[allow(non_snake_case)]
    fn resolveToolPkgResourceFile(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<PathBuf> {
        RuntimePackageManager::resolveToolPkgResourceFile(self, runtime, resourcePath)
    }

    /// Exports one ToolPkg resource to a host file.
    #[allow(non_snake_case)]
    fn exportToolPkgResource(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resource: &ToolPkgResourceRuntime,
        destinationFile: &Path,
    ) -> bool {
        RuntimePackageManager::exportToolPkgResource(self, runtime, resource, destinationFile)
    }

    /// Reads one ToolPkg resource as bytes.
    #[allow(non_snake_case)]
    fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<Vec<u8>> {
        RuntimePackageManager::readToolPkgResourceBytes(self, runtime, resourcePath)
    }
}

#[allow(non_snake_case)]
fn currentUseTime() -> String {
    chrono::Local::now()
        .naive_local()
        .format("%Y-%m-%dT%H:%M:%S%.f")
        .to_string()
}

#[allow(non_snake_case)]
fn bundledPluginAssetByName(
    runtimeSupport: &dyn ToolRuntimeSupport,
    assetName: &str,
) -> Option<&'static RuntimePluginAsset> {
    runtimeSupport
        .builtinPluginAssets()
        .iter()
        .chain(runtimeSupport.bundledExternalPluginAssets().iter())
        .find(|asset| asset.name == assetName)
}

#[allow(non_snake_case)]
fn bundledExternalPluginAssetByName(
    runtimeSupport: &dyn ToolRuntimeSupport,
    assetName: &str,
) -> Option<&'static RuntimePluginAsset> {
    runtimeSupport
        .bundledExternalPluginAssets()
        .iter()
        .find(|asset| asset.name == assetName)
}

#[allow(non_snake_case)]
fn buildBundledPluginAssetSignature(asset: &RuntimePluginAsset) -> String {
    format!(
        "{}|{}|{}",
        asset.name,
        asset.bytes.len(),
        sha256Hex(asset.bytes)
    )
}

#[allow(non_snake_case)]
fn sha256Hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[allow(non_snake_case)]
fn metadataModifiedMillis(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| {
            modified
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_millis())
        })
        .unwrap_or(0)
}

#[allow(non_snake_case)]
fn javaStringHashCodeHex(value: &str) -> String {
    let mut hash = 0i32;
    for unit in value.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(unit));
    }
    format!("{:x}", hash as u32)
}

#[allow(non_snake_case)]
fn copyDirectoryEntries(sourceDir: &Path, destinationDir: &Path) -> bool {
    if !sourceDir.exists() || !sourceDir.is_dir() {
        return false;
    }
    let mut pending = vec![sourceDir.to_path_buf()];
    while let Some(currentDir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&currentDir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if !path.is_file() {
                continue;
            }
            let Ok(relativePath) = path.strip_prefix(sourceDir) else {
                return false;
            };
            let relativePath = relativePath.to_string_lossy().replace('\\', "/");
            let Some(normalizedEntry) = ToolPkgArchiveParser::normalizeZipEntryPath(&relativePath)
            else {
                continue;
            };
            let outputFile = destinationDir.join(normalizedEntry);
            if let Some(parent) = outputFile.parent() {
                if fs::create_dir_all(parent).is_err() {
                    return false;
                }
            }
            if fs::copy(&path, &outputFile).is_err() {
                return false;
            }
        }
    }
    true
}

#[allow(non_snake_case)]
fn zipToolPkgResourceDirectory(sourceDirectory: &Path, destinationZip: &Path) -> bool {
    let Some(zipRootParent) = sourceDirectory.parent() else {
        return false;
    };
    let Ok(fileOutput) = fs::File::create(destinationZip) else {
        return false;
    };
    let mut zipOutput = zip::ZipWriter::new(fileOutput);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut files = Vec::<PathBuf>::new();
    collectDirectoryFiles(sourceDirectory, &mut files);
    files.sort();
    for file in files {
        let Ok(relativePath) = file.strip_prefix(zipRootParent) else {
            return false;
        };
        let relativePath = relativePath.to_string_lossy().replace('\\', "/");
        let Some(normalizedEntry) = ToolPkgArchiveParser::normalizeZipEntryPath(&relativePath)
        else {
            continue;
        };
        if zipOutput.start_file(normalizedEntry, options).is_err() {
            return false;
        }
        let Ok(mut input) = fs::File::open(&file) else {
            return false;
        };
        if std::io::copy(&mut input, &mut zipOutput).is_err() {
            return false;
        }
    }
    zipOutput.finish().is_ok()
}

#[allow(non_snake_case)]
fn collectDirectoryFiles(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collectDirectoryFiles(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

#[allow(non_snake_case)]
fn isExternalPackageCandidateFile(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    let normalized = extension.to_ascii_lowercase();
    matches!(normalized.as_str(), "js" | "ts" | "hjson" | "toolpkg")
}

#[allow(non_snake_case)]
fn isPackageCandidateAssetName(assetName: &str) -> bool {
    let Some(extension) = Path::new(assetName)
        .extension()
        .and_then(|value| value.to_str())
    else {
        return false;
    };
    let normalized = extension.to_ascii_lowercase();
    matches!(normalized.as_str(), "js" | "ts" | "hjson" | "toolpkg")
}

#[allow(non_snake_case)]
fn packageSourceFileName(sourcePath: &str) -> String {
    Path::new(sourcePath)
        .file_name()
        .expect("package source path must have file name")
        .to_string_lossy()
        .to_string()
}

#[allow(non_snake_case)]
fn buildConditionCapabilitiesSnapshot() -> BTreeMap<String, ConditionValue> {
    let platformName = std::env::consts::OS;
    BTreeMap::from([
        (
            "platform.name".to_string(),
            ConditionValue::Str(platformName.to_string()),
        ),
        (
            "platform.windows".to_string(),
            ConditionValue::Bool(platformName == "windows"),
        ),
        (
            "platform.linux".to_string(),
            ConditionValue::Bool(platformName == "linux"),
        ),
        (
            "platform.android".to_string(),
            ConditionValue::Bool(platformName == "android"),
        ),
        (
            "platform.macos".to_string(),
            ConditionValue::Bool(platformName == "macos"),
        ),
        (
            "ui.virtual_display".to_string(),
            ConditionValue::Bool(false),
        ),
        (
            "android.permission_level".to_string(),
            ConditionValue::Str("STANDARD".to_string()),
        ),
        (
            "android.shizuku_available".to_string(),
            ConditionValue::Bool(false),
        ),
        ("ui.shower_display".to_string(), ConditionValue::Bool(false)),
    ])
}

#[allow(non_snake_case)]
fn logPackageManagerError(message: impl AsRef<str>) {
    AppLogger::e(PACKAGE_MANAGER_LOG_TAG, message.as_ref());
}
