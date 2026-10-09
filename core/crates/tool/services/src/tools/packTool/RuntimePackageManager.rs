#[path = "PackageRegistryReadiness.rs"]
mod package_registry_readiness;
pub use package_registry_readiness::{PackageRegistryLoadState, PackageRegistryReadiness};

#[path = "ToolPkgCoreCommandCatalog.rs"]
mod core_command_catalog;
pub use core_command_catalog::{ResolvedToolPkgCoreCommand, ToolPkgCoreCommandCatalog, ToolPkgCoreCommandRegistrationInfo};

use crate::tools::packTool::ToolPkgPublicApiUiCatalog::{discover_public_api_owners, ToolPkgPublicApiOwner};

use operit_store::ExtensionStore::{ExtensionRecord, ExtensionStore};
use operit_store::PreferencesDataStore::CoreNodeStateStore;
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use crate::tools::packTool::ToolPkgComposeDslSession::ToolPkgComposeDslSession;
use crate::tools::packTool::ToolPkgDesktopWidgetService::ToolPkgDesktopWidgetSnapshot;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::runtime_support::{RuntimePluginAsset, ToolRuntimeSupport};
use crate::tools::condition::ConditionEvaluator::{ConditionEvaluator, ConditionValue};
use crate::tools::mcp::MCPManager::MCPManager;
use crate::tools::mcp::MCPPackage::MCPPackage;
use crate::tools::mcp::MCPServerConfig::MCPServerConfig;
use crate::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use crate::tools::skill::SkillManager::SkillManager;
use crate::tools::PackageLoadingProgress::{
    ensurePluginLoadingItem, markPluginLoadingItemFailed, markPluginLoadingItemLoading,
    markPluginLoadingItemSuccess, pluginLoadingSessionActive, PLUGIN_LOAD_KIND_PACKAGE,
};
use crate::tools::ToolJsRuntime::{JsExecutionEngine, JsExecutionProvider};
use crate::tools::ToolResultDataClasses::stringResultData;
use crate::ConversationMarkupManager::ToolResult;
use operit_host_api::{
    FileSystemHost, HostEnvironmentDescriptor, HostManager::HostManager, HostPlatform,
    TimeUtils::currentTimeMillis,
};
use operit_plugin_sdk::javascript::{
    JsToolPkgWasmRequest, JsToolPkgWasmResult, ToolPkgConfigScope, ToolPkgExecutionContext,
};
use operit_plugin_sdk::package::{LocalizedText, PublishablePackageSource, ToolPackage};
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{ToolPkgHookDispatcher, ToolPkgHookInvocation};
use operit_plugin_sdk::toolpkg::ToolPkgLoader::ToolPkgLoader;
use operit_plugin_sdk::toolpkg::ToolPkgManager::{
    ToolPkgAssetSource, ToolPkgExecutionEngineFactory, ToolPkgManager, ToolPkgRuntimeChangeListener,
};
use operit_plugin_sdk::toolpkg::ToolPkgPackageModels::{
    ToolPkgChatMessageMenuItem, ToolPkgContainerDetails, ToolPkgDesktopWidget, ToolPkgLogoBytes,
    ToolPkgNavigationActionHook, ToolPkgNavigationEntry, ToolPkgSubpackageInfo,
    ToolPkgToolboxUiModule, ToolPkgUiRoute, ToolPkgWorkspaceTemplate,
    ToolPkgWorkspaceTemplateImportResult,
};
use operit_plugin_sdk::toolpkg::ToolPkgPackageService::{
    ToolPkgPackageHost, ToolPkgPackageService,
};
use operit_plugin_sdk::toolpkg::ToolPkgParser::{
    ToolPkgArchiveParser, ToolPkgContainerRuntime, ToolPkgDependencyIssue, ToolPkgLoadResult,
    ToolPkgManifestRequirement, ToolPkgMarketOrigin, ToolPkgResourceRuntime, ToolPkgSourceType,
    ToolPkgSubpackageRuntime,
};
use operit_plugin_sdk::toolpkg::ToolPkgProtection;
use operit_plugin_sdk::JsPackageLoader::JsPackageLoader;
use operit_plugin_sdk::PackageManager::{PackageStateResolver, PluginPackageManager};
use operit_store::PreferencesDataStore::{stringPreferencesKey, PreferencesDataStoreError};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_util::AppLogger::AppLogger;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const BUNDLED_EXTERNAL_IMPORTS_KEY: &str = "bundled_external_imports";
const MARKET_TOOLPKG_INSTALLATION_ID_KEY: &str = "toolpkg_market_installation_id";
const TOOLPKG_CACHE_SIGNATURE_FILE: &str = ".toolpkg-cache-signature";
const MARKET_TOOLPKG_FILE_PREFIX: &str = "market-";
const PACKAGE_MANAGER_LOG_TAG: &str = "ToolPkg";

/// Describes one slash command contributed by an enabled ToolPkg package.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgCoreCommandInfo {
    pub containerPackageName: String,
    pub commandId: String,
    pub name: String,
    pub title: String,
    pub description: String,
    pub usage: String,
}

/// Contains the text and structured output returned by a ToolPkg command.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolPkgCoreCommandExecutionResult {
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub json: Option<serde_json::Value>,
}

/// Creates SDK-owned ToolPkg execution engines through the installed JavaScript bridge.
#[derive(Clone)]
struct RuntimeToolPkgExecutionEngineFactory {
    toolHandler: crate::tools::AIToolHandler::AIToolHandler,
    jsExecutionProvider: Arc<dyn JsExecutionProvider>,
}

impl ToolPkgExecutionEngineFactory for RuntimeToolPkgExecutionEngineFactory {
    /// Creates one JavaScript engine without a ToolPkg package environment.
    #[allow(non_snake_case)]
    fn createExecutionEngine(&self) -> Arc<dyn JsExecutionEngine> {
        self.jsExecutionProvider
            .create_execution_engine(Arc::new(self.toolHandler.clone()))
    }

    /// Creates one ToolPkg engine bound to the current host context.
    #[allow(non_snake_case)]
    fn createToolPkgExecutionEngine(
        &self,
        context: ToolPkgExecutionContext,
    ) -> Arc<dyn JsExecutionEngine> {
        self.jsExecutionProvider
            .create_toolpkg_execution_engine(Arc::new(self.toolHandler.clone()), context)
    }
}

/// Destroys one temporary ToolPkg registration engine when its load scope ends.
struct ToolPkgRegistrationEngineGuard {
    engine: Arc<dyn JsExecutionEngine>,
}

impl Drop for ToolPkgRegistrationEngineGuard {
    /// Interrupts and destroys the temporary registration engine.
    fn drop(&mut self) {
        self.engine.destroy();
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
#[derive(Clone)]
struct RuntimePackageStateResolver {
    hostEnvironment: HostEnvironmentDescriptor,
}

impl PackageStateResolver for RuntimePackageStateResolver {
    /// Returns the first conditional state matching the current runtime capabilities.
    #[allow(non_snake_case)]
    fn resolvePackageStateId(&self, package: &ToolPackage) -> Option<String> {
        let capabilities = buildConditionCapabilitiesSnapshot(&self.hostEnvironment);
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

/// Structured result returned after importing one external package file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ExternalPackageImportResult {
    pub packageName: String,
    pub packageFormat: String,
    pub storedPath: String,
    pub sourceNotice: Option<String>,
}

/// Describes one package load or import failure shown by the package manager UI.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgLoadIssue {
    pub sourcePath: String,
    pub packageName: Option<String>,
    pub displayName: String,
    pub code: String,
    pub message: String,
    pub packageKind: String,
}

#[derive(Clone, Default)]
struct PackageScanSnapshot {
    availablePackages: BTreeMap<String, ToolPackage>,
    toolPkgContainers: BTreeMap<String, ToolPkgContainerRuntime>,
    toolPkgSubpackages: BTreeMap<String, ToolPkgSubpackageRuntime>,
    toolPkgLoadIssues: Vec<ToolPkgLoadIssue>,
}

#[derive(Clone, Default)]
struct PackageScanCandidateResult {
    phase: String,
    toolPackage: Option<ToolPackage>,
    toolPkgLoadResult: Option<ToolPkgLoadResult>,
    sourcePath: String,
    issues: Vec<ToolPkgLoadIssue>,
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
    toolPkgLoadIssues: Vec<ToolPkgLoadIssue>,
    manualToolPkgLoadIssues: Vec<ToolPkgLoadIssue>,
    toolPkgCacheLock: Arc<Mutex<()>>,
    packageRegistryReadiness: PackageRegistryReadiness,
    scopeState: Arc<Mutex<BTreeMap<String, ExtensionRecord>>>,
    toolPkgExecutionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory>,
    composeDslSessions: Arc<Mutex<BTreeMap<(String, String), ToolPkgComposeDslSession>>>,
    dataStore: CoreNodeStateStore,
    storePaths: RuntimeStorePaths,
    fileSystemHost: Arc<dyn FileSystemHost>,
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
        let toolPkgExecutionEngineFactory: Arc<dyn ToolPkgExecutionEngineFactory> =
            Arc::new(RuntimeToolPkgExecutionEngineFactory {
                toolHandler: toolHandler.clone(),
                jsExecutionProvider: runtimeDependencies.shared_js_execution_provider(),
            });
        let fileSystemHost = context
            .fileSystemHost
            .clone()
            .expect("RuntimePackageManager requires a FileSystemHost");
        let mut manager = Self {
            pluginPackageManager: PluginPackageManager::new(
                toolPkgExecutionEngineFactory.clone(),
                Arc::new(RuntimeToolPkgAssetSource {
                    runtimeSupport: runtimeDependencies.shared_runtime_support(),
                }),
                fileSystemHost.clone(),
                Arc::new(RuntimePackageStateResolver {
                    hostEnvironment: context.hostEnvironment.clone(),
                }),
            ),
            cachedMcpTools: BTreeMap::new(),
            externalPackageScanCache: BTreeMap::new(),
            bundledExternalPackageScanCache: BTreeMap::new(),
            toolPkgLoadIssues: Vec::new(),
            manualToolPkgLoadIssues: Vec::new(),
            toolPkgCacheLock: Arc::new(Mutex::new(())),
            packageRegistryReadiness: PackageRegistryReadiness::new(),
            scopeState: Arc::new(Mutex::new(BTreeMap::new())),
            toolPkgExecutionEngineFactory,
            composeDslSessions: Arc::new(Mutex::new(BTreeMap::new())),
            dataStore: CoreNodeStateStore::newWithStorage(
                defaultRuntimeStorageHost(),
                "runtime/extensions/device/manager.preferences.json",
            ),
            storePaths: paths,
            fileSystemHost,
            mcpManager: MCPManager::getInstance(context.clone()),
            context,
            toolHandler,
        };
        manager
    }

    /// Claims the single initial package load before the application submits its background task.
    #[allow(non_snake_case)]
    pub fn claimInitialPackageRegistryLoad(&self) -> Result<bool, String> {
        self.packageRegistryReadiness.claim_initial_load()
    }

    /// Records an original startup failure so commands waiting for the registry reject immediately.
    #[allow(non_snake_case)]
    pub fn failPackageRegistryLoad(&self, error: String) {
        self.packageRegistryReadiness.fail(error);
    }

    /// Returns the shared lifecycle handle without cloning an incomplete registration snapshot.
    #[allow(non_snake_case)]
    pub fn packageRegistryReadiness(&self) -> PackageRegistryReadiness {
        self.packageRegistryReadiness.clone()
    }

    /// Waits outside the manager lock and clones only a completed canonical registration snapshot.
    #[allow(non_snake_case)]
    pub async fn readySnapshot(manager: Arc<Mutex<Self>>) -> Result<Self, String> {
        let readiness = {
            let manager = manager.lock().map_err(|error| error.to_string())?;
            manager.packageRegistryReadiness()
        };
        readiness.wait_until_ready().await?;
        let manager = manager.lock().map_err(|error| error.to_string())?;
        manager.packageRegistryReadiness.require_ready()?;
        Ok(manager.clone())
    }

    /// Finds exact registerApi owners from the ready catalog without invoking their handlers or building domain UI inputs.
    #[allow(non_snake_case)]
    pub fn getToolPkgPublicApiOwners(&self, apiName: &str) -> Result<Vec<ToolPkgPublicApiOwner>, String> {
        self.packageRegistryReadiness.require_ready()?;
        let registered = self.toolPkgManager().getToolPkgContainerRuntimes();
        let enabled = self.getEnabledToolPkgContainerRuntimes();
        discover_public_api_owners(apiName, &registered, &enabled)
    }

    /// Lists only methods actually published by one enabled ToolPkg in this ready snapshot.
    #[allow(non_snake_case)]
    pub fn getToolPkgPublicApis(&self, packageName: &str) -> Result<Vec<String>, String> {
        let runtime = self.requireToolPkgPublicApiRuntime(packageName)?;
        let mut names = BTreeSet::new();
        for declaration in runtime.publicApis {
            if !names.insert(declaration.id.clone()) {
                return Err(format!("Duplicate public API: {}/{}", runtime.packageName, declaration.id));
            }
        }
        Ok(names.into_iter().collect())
    }

    /// Calls one registerApi export in its owning main runtime without dispatching an AI tool.
    #[allow(non_snake_case)]
    pub async fn invokeToolPkgPublicApi(
        &self,
        packageName: &str,
        methodName: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let runtime = self.requireToolPkgPublicApiRuntime(packageName)?;
        let declaration = selectToolPkgPublicApi(&runtime, methodName)?.clone();
        let script = self.getRegisteredToolPkgMainScript(&runtime.packageName)
            .ok_or_else(|| format!("ToolPkg main script is unavailable: {}", runtime.packageName))?;
        let contextKey = format!("toolpkg_main:{}", runtime.packageName);
        let engine = self.getToolPkgExecutionEngine(&contextKey, &runtime.packageName);
        let event = operit_plugin_sdk::js_sdk::toolpkg::ToolPkgPublicApiEvent {
            payload,
            callerPackage: "host".to_string(),
        };
        let serde_json::Value::Object(event) = serde_json::to_value(event)
            .map_err(|error| error.to_string())? else {
            return Err("ToolPkg public API event must serialize to an object".to_string());
        };
        let mut params = event.into_iter().collect::<BTreeMap<_, _>>();
        params.insert("__operit_ui_package_name".to_string(), serde_json::Value::String(runtime.packageName.clone()));
        params.insert("toolPkgId".to_string(), serde_json::Value::String(runtime.packageName.clone()));
        params.insert("containerPackageName".to_string(), serde_json::Value::String(runtime.packageName));
        params.insert("__operit_execution_context_key".to_string(), serde_json::Value::String(contextKey));
        params.insert("__operit_toolpkg_runtime_kind".to_string(), serde_json::Value::String("main".to_string()));
        params.insert("__operit_script_screen".to_string(), serde_json::Value::String(runtime.mainEntry));
        params.insert("__operit_toolpkg_api_version".to_string(), serde_json::Value::String(runtime.apiVersion));
        if let Some(source) = declaration.functionSource {
            params.insert("__operit_inline_function_name".to_string(), serde_json::Value::String(declaration.function.clone()));
            params.insert("__operit_inline_function_source".to_string(), serde_json::Value::String(source));
        }
        let raw = engine.execute_script_function_async(
            script, declaration.function, params, BTreeMap::new(), None, true, 60_000,
        ).await.map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Public API returned no JSON result: {packageName}/{methodName}"))?;
        serde_json::from_str(&raw).map_err(|error| error.to_string())
    }

    /// Requires a real enabled public API owner after the canonical registry has finished loading.
    #[allow(non_snake_case)]
    fn requireToolPkgPublicApiRuntime(&self, packageName: &str) -> Result<ToolPkgContainerRuntime, String> {
        self.packageRegistryReadiness.require_ready()?;
        let packageName = packageName.trim();
        if packageName.is_empty() {
            return Err("Public API package name is required".to_string());
        }
        let runtime = self.getToolPkgContainerRuntime(packageName)
            .ok_or_else(|| format!("ToolPkg container not found: {packageName}"))?;
        if !self.getEnabledToolPkgContainerRuntimes().iter()
            .any(|enabled| enabled.packageName == runtime.packageName) {
            return Err(format!("ToolPkg public API owner is disabled: {packageName}"));
        }
        if runtime.publicApi.is_none() {
            return Err(format!("{packageName} does not publish public_api"));
        }
        Ok(runtime)
    }

    /// Marks a package as active for the current prompt session.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn activatePackage(&mut self, packageName: &str) -> bool {
        self.pluginPackageManager.activatePackage(packageName)
    }

    #[allow(non_snake_case)]
    /// Releases one explicitly owned ToolPkg execution engine.
    pub fn releaseToolPkgExecutionEngine(&self, contextKey: &str, containerPackageName: &str) {
        let key = (contextKey.to_string(), containerPackageName.to_string());
        if let Some(session) = self.composeDslSessions.lock().expect("Compose session registry mutex poisoned").remove(&key) { session.close(); }
        self.toolPkgManager()
            .releaseToolPkgExecutionEngine(contextKey, containerPackageName);
    }

    #[allow(non_snake_case)]
    /// Acquires one explicit owner lease for a ToolPkg execution engine.
    pub fn acquireToolPkgExecutionEngine(&self, contextKey: &str, containerPackageName: &str) {
        self.toolPkgManager()
            .acquireToolPkgExecutionEngine(contextKey, containerPackageName);
    }

    #[allow(non_snake_case)]
    /// Returns the ToolPkg execution engine for one explicitly owned context.
    pub fn getToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> Arc<dyn JsExecutionEngine> {
        self.toolPkgManager()
            .getToolPkgExecutionEngine(contextKey, containerPackageName)
    }

    /// Resolves the same application session for every automatic object-proxy access.
    pub fn openComposeDslSession(&self, contextKey: &str, containerPackageName: &str) -> Result<String, String> {
        let key = (contextKey.to_string(), containerPackageName.to_string());
        let mut sessions = self.composeDslSessions.lock().expect("Compose session registry mutex poisoned");
        if let Some(session) = sessions.get(&key) { return Ok(session.id()); }
        let session = ToolPkgComposeDslSession::new(
            self.getToolPkgExecutionEngine(contextKey, containerPackageName),
            self.toolPkgTextResources(containerPackageName)?,
        );
        sessions.insert(key, session.clone());
        Ok(session.id())
    }

    #[allow(non_snake_case)]
    /// Finds an existing ToolPkg execution engine for a context key.
    pub fn findToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> Option<Arc<dyn JsExecutionEngine>> {
        self.toolPkgManager()
            .findToolPkgExecutionEngine(contextKey, containerPackageName)
    }

    #[allow(non_snake_case)]
    pub(crate) fn contextInternal(&self) -> &HostManager {
        &self.context
    }

    /// Returns the file-system host required by package-adjacent services.
    pub fn fileSystemHost(&self) -> Arc<dyn FileSystemHost> {
        self.fileSystemHost.clone()
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
        let _ = self.fileSystemHost.makeDirectory(&hostPath(&dir), true);
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

    /// Returns the stable local identifier that binds installed market ToolPkg seals to this client.
    #[allow(non_snake_case)]
    fn marketToolPkgInstallationId(
        &self,
    ) -> Result<[u8; ToolPkgProtection::MARKET_INSTALLATION_ID_SIZE], String> {
        let key = stringPreferencesKey(MARKET_TOOLPKG_INSTALLATION_ID_KEY);
        self.dataStore
            .try_edit_result(|preferences| {
                let existing = preferences.get(&key).cloned();
                match existing {
                    Some(encoded) => decodeMarketToolPkgInstallationId(&encoded)
                        .map_err(PreferencesDataStoreError::Message),
                    None => {
                        let installationId = ToolPkgProtection::createMarketInstallationId();
                        preferences.set(&key, encodeMarketToolPkgInstallationId(&installationId));
                        Ok(installationId)
                    }
                }
            })
            .map_err(|error| error.to_string())
    }

    /// Verifies an archive using the seal identity of its exact installation owner.
    #[allow(non_snake_case)]
    fn isInstalledMarketToolPkg(&self, file: &Path) -> Result<bool, String> {
        let root = self.storePaths.runtime_storage_key(file)?;
        let records = ExtensionStore::default().records("package")?;
        let record = records.iter().find(|record| {
            ExtensionStore::root("package", &record.scope)
                .is_ok_and(|directory| format!("{directory}/{}", record.sourceName) == root)
        });
        let installationId = match record {
            Some(record) => decodeMarketToolPkgInstallationId(
                record.settings["installationId"]
                    .as_str()
                    .ok_or("Registered installation seal is missing")?,
            )?,
            None => self.marketToolPkgInstallationId()?,
        };
        let bytes = self
            .fileSystemHost
            .readFileBytes(&hostPath(file))
            .map_err(|e| e.to_string())?;
        Ok(matches!(
            ToolPkgProtection::verifyMarketInstallSeal(&bytes, &installationId),
            Ok(true)
        ))
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
        let _ = self.fileSystemHost.deleteFile(&hostPath(&dir), true);
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
        let cacheDirExists = self
            .fileSystemHost
            .fileExists(&hostPath(&cacheDir))
            .map(|info| info.exists && info.isDirectory)
            .unwrap_or(false);
        let signatureFileExists = self
            .fileSystemHost
            .fileExists(&hostPath(&signatureFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false);
        let signatureMatches = if signatureFileExists {
            self.fileSystemHost
                .readFile(&hostPath(&signatureFile))
                .map(|text| text == signature)
                .unwrap_or(false)
        } else {
            false
        };
        let mainScriptFile = cacheDir.join(mainEntry);
        let mainScriptExists = self
            .fileSystemHost
            .fileExists(&hostPath(&mainScriptFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false);

        if cacheDirExists && signatureFileExists && signatureMatches && mainScriptExists {
            return Some(cacheDir);
        }

        self.deleteToolPkgCacheDirLocked(packageName);
        if self
            .fileSystemHost
            .makeDirectory(&hostPath(&cacheDir), true)
            .is_err()
        {
            return None;
        }
        if !extractArchive(&cacheDir) {
            self.deleteToolPkgCacheDirLocked(packageName);
            return None;
        }
        if self
            .fileSystemHost
            .writeFile(&hostPath(&signatureFile), signature, false)
            .is_err()
        {
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
            ToolPkgSourceType::EXTERNAL | ToolPkgSourceType::MARKET => {
                let sourceFile = PathBuf::from(sourcePath);
                let metadata = self.fileSystemHost.fileInfo(sourcePath).ok()?;
                if !metadata.exists {
                    return None;
                }
                Some(format!(
                    "{}|{}|{}|{}|{}|{}",
                    if *sourceType == ToolPkgSourceType::MARKET {
                        "market"
                    } else {
                        "external"
                    },
                    sourceFile.to_string_lossy(),
                    metadata.size,
                    metadata.lastModified,
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
            ToolPkgSourceType::EXTERNAL | ToolPkgSourceType::MARKET => {
                if self
                    .fileSystemHost
                    .fileExists(&runtime.sourcePath)
                    .map(|info| info.exists && info.isDirectory)
                    .unwrap_or(false)
                {
                    return self
                        .fileSystemHost
                        .copyFile(&runtime.sourcePath, &hostPath(destinationDir), true)
                        .is_ok();
                }
                ToolPkgArchiveParser::extractZipEntriesFromExternal(
                    self.fileSystemHost.as_ref(),
                    &runtime.sourcePath,
                    &destinationDir.to_string_lossy(),
                )
            }
            ToolPkgSourceType::ASSET => {
                let Some(asset) = bundledPluginAssetByName(
                    self.toolHandler.runtimeSupport().as_ref(),
                    &runtime.sourcePath,
                ) else {
                    return false;
                };
                ToolPkgArchiveParser::extractZipEntriesFromAssetBytes(
                    asset.bytes,
                    self.fileSystemHost.as_ref(),
                    &destinationDir.to_string_lossy(),
                )
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

    /// Collects immutable UTF-8 ToolPkg entries for module resolution without host reentry.
    #[allow(non_snake_case)]
    pub fn toolPkgTextResources(
        &self,
        containerPackageName: &str,
    ) -> Result<Arc<BTreeMap<String, String>>, String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let runtime = self
            .toolPkgManager()
            .getToolPkgContainerRuntime(&normalizedContainerPackageName)
            .ok_or_else(|| {
                format!(
                    "ToolPkg container package is not registered: {normalizedContainerPackageName}"
                )
            })?;
        let cacheDir = self.ensureToolPkgCache(&runtime).ok_or_else(|| {
            format!("ToolPkg package cache is unavailable: {normalizedContainerPackageName}")
        })?;
        let entryIndex = ToolPkgArchiveParser::buildDirectoryEntryIndex(
            self.fileSystemHost.as_ref(),
            &hostPath(&cacheDir),
        );
        let mut textResources = BTreeMap::new();
        for entryName in entryIndex.entryNames {
            let Some(bytes) = self.readToolPkgResourceBytes(&runtime, &entryName) else {
                continue;
            };
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            textResources.insert(entryName.to_ascii_lowercase(), text);
        }
        Ok(Arc::new(textResources))
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
        if !self
            .fileSystemHost
            .fileExists(&hostPath(&resourceFile))
            .map(|info| info.exists)
            .unwrap_or(false)
        {
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
            if self
                .fileSystemHost
                .makeDirectory(&hostPath(parent), true)
                .is_err()
            {
                return false;
            }
        }
        if ToolPkgArchiveParser::isDirectoryResourceMime(Some(&resource.mime)) {
            if !self
                .fileSystemHost
                .fileExists(&hostPath(&resourceFile))
                .map(|info| info.exists && info.isDirectory)
                .unwrap_or(false)
            {
                return false;
            }
            self.fileSystemHost
                .zipFiles(&hostPath(&resourceFile), &hostPath(destinationFile))
                .is_ok()
        } else {
            if !self
                .fileSystemHost
                .fileExists(&hostPath(&resourceFile))
                .map(|info| info.exists && !info.isDirectory)
                .unwrap_or(false)
            {
                return false;
            }
            self.fileSystemHost
                .copyFile(&hostPath(&resourceFile), &hostPath(destinationFile), false)
                .is_ok()
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
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn isPackageActivated(&self, packageName: &str) -> bool {
        self.pluginPackageManager.isPackageActivated(packageName)
    }

    #[allow(non_snake_case)]
    /// Activates a package and returns its system prompt contribution.
    #[operit_route_macros::operit_plugin_sdk_expose]
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

        let skillManager = SkillManager::fromDefaultPaths(self.fileSystemHost());
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
    #[operit_route_macros::operit_plugin_sdk_expose]
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

        let skillManager = SkillManager::fromDefaultPaths(self.fileSystemHost());
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
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getEnabledPackageNames(&self) -> Vec<String> {
        let availablePackages = self.availablePackages();
        let mut enabledPackageNames = self
            .decodeEnabledPackageNamesFromPrefs()
            .into_iter()
            .filter(|packageName| availablePackages.contains_key(packageName))
            .collect::<BTreeSet<_>>();
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
    #[operit_route_macros::operit_plugin_sdk_expose]
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
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getActivePackageNames(&self) -> Vec<String> {
        self.pluginPackageManager.activePackageNames()
    }

    #[allow(non_snake_case)]
    /// Enables a package and loads its tools into available package state.
    #[operit_route_macros::operit_plugin_sdk_expose]
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
    #[operit_route_macros::operit_plugin_sdk_expose]
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
                .destroyToolPkgExecutionEngines(&normalizedPackageName);
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
        let name = match self
            .toolPkgManager()
            .resolveToolPkgSubpackageRuntimeInternal(packageName)
        {
            Some(child) => child.containerPackageName,
            None => self.normalizePackageName(packageName),
        };
        let record = match ExtensionStore::default().record("package", &name) {
            Ok(record) => record,
            Err(error) => {
                self.recordManualToolPkgLoadIssue(
                    &name,
                    Some(&name),
                    "delete_failed",
                    &error,
                    "package",
                );
                return false;
            }
        };
        if record.settings["builtin"] == true {
            return false;
        }
        self.disablePackage(&name);
        if let Err(error) = ExtensionStore::default().delete("package", &name) {
            self.recordManualToolPkgLoadIssue(
                &name,
                Some(&name),
                "delete_failed",
                &error,
                "package",
            );
            return false;
        }
        if let Err(error) = self.removeBundledExternalImportRecord(&name) {
            self.recordManualToolPkgLoadIssue(
                &name,
                Some(&name),
                "delete_failed",
                &error,
                "package",
            );
            return false;
        }
        self.loadAvailablePackages();
        true
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
    #[operit_route_macros::operit_plugin_sdk_expose]
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
        let runtimes = self
            .toolPkgManager()
            .getEnabledToolPkgContainerRuntimes(&self.getEnabledPackageNames());
        self.orderToolPkgRuntimes(runtimes)
    }

    #[allow(non_snake_case)]
    /// Orders active ToolPkg runtimes exactly as persisted by the user.
    fn orderToolPkgRuntimes(
        &self,
        mut runtimes: Vec<ToolPkgContainerRuntime>,
    ) -> Vec<ToolPkgContainerRuntime> {
        let storedOrder = self.decodeToolPkgContainerOrderFromPrefs();
        let mut byName = runtimes
            .drain(..)
            .map(|runtime| (runtime.packageName.clone(), runtime))
            .collect::<BTreeMap<_, _>>();
        let mut ordered = Vec::new();
        for packageName in storedOrder {
            if let Some(runtime) = byName.remove(&packageName) {
                ordered.push(runtime);
            }
        }
        ordered.extend(byName.into_values());
        ordered
    }

    #[allow(non_snake_case)]
    /// Returns all registered ToolPkg container runtimes.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getToolPkgContainerRuntimes(&self) -> Vec<ToolPkgContainerRuntime> {
        let enabledPackageNames = self.getEnabledPackageNameSetInternal();
        let order = self
            .getToolPkgContainerOrder()
            .into_iter()
            .enumerate()
            .map(|(index, packageName)| (packageName, index))
            .collect::<BTreeMap<_, _>>();
        let runtimes = self.toolPkgManager().getToolPkgContainerRuntimes();
        runtimes
            .into_iter()
            .map(|runtime| self.withToolPkgDependencyIssues(runtime, &enabledPackageNames, &order))
            .collect()
    }

    #[allow(non_snake_case)]
    /// Returns the persisted visual order of ToolPkg containers followed by newly discovered containers.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getToolPkgContainerOrder(&self) -> Vec<String> {
        let stored = self.decodeToolPkgContainerOrderFromPrefs();
        let mut installed = self.toolPkgManager().getToolPkgContainerRuntimes();
        let mut byName = installed
            .drain(..)
            .map(|runtime| (runtime.packageName.clone(), runtime))
            .collect::<BTreeMap<_, _>>();
        let mut ordered = Vec::new();
        for packageName in stored {
            if let Some(runtime) = byName.remove(&packageName) {
                ordered.push(runtime.packageName);
            }
        }
        ordered.extend(byName.into_values().map(|runtime| runtime.packageName));
        ordered
    }

    #[allow(non_snake_case)]
    /// Persists the complete visual order of ToolPkg containers.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn setToolPkgContainerOrder(
        &self,
        packageNames: Vec<String>,
    ) -> Result<(), PreferencesDataStoreError> {
        let installed = self
            .toolPkgManager()
            .getToolPkgContainerRuntimes()
            .into_iter()
            .map(|runtime| runtime.packageName)
            .collect::<BTreeSet<_>>();
        let mut ordered = Vec::new();
        let mut seen = BTreeSet::new();
        for packageName in packageNames {
            let normalized = self.normalizePackageName(&packageName);
            if installed.contains(&normalized) && seen.insert(normalized.clone()) {
                ordered.push(normalized);
            }
        }
        for packageName in installed {
            if seen.insert(packageName.clone()) {
                ordered.push(packageName);
            }
        }
        let result = (|| -> Result<(), PreferencesDataStoreError> {
            for (rank, name) in ordered.iter().enumerate() {
                let mut record = ExtensionStore::default()
                    .record("package", name)
                    .map_err(PreferencesDataStoreError::Message)?;
                record.settings["order"] = serde_json::json!(rank);
                self.writeScopeSettings(name, record.settings)
                    .map_err(PreferencesDataStoreError::Message)?;
            }
            Ok(())
        })();
        if result.is_ok() {
            self.notifyToolPkgRuntimeChangeListeners();
        }
        result
    }

    #[allow(non_snake_case)]
    /// Returns localized details for a ToolPkg container.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getToolPkgContainerDetails(
        &self,
        packageName: &str,
        useEnglish: bool,
    ) -> Option<ToolPkgContainerDetails> {
        ToolPkgPackageService::new(self).getToolPkgContainerDetails(packageName, useEnglish)
    }

    #[allow(non_snake_case)]
    /// Reads the manifest-declared logo bytes for one ToolPkg container.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn readToolPkgLogoBytes(&self, packageName: &str) -> Option<ToolPkgLogoBytes> {
        self.ensureInitialized();
        let normalizedPackageName = self.normalizePackageName(packageName);
        let runtime = self
            .toolPkgManager()
            .getToolPkgContainerRuntime(&normalizedPackageName)?;
        let logoResource = runtime.logoResource.clone()?;
        let fileName = Path::new(&logoResource.path)
            .file_name()?
            .to_str()?
            .to_string();
        let bytes = self.readToolPkgResourceBytes(&runtime, &logoResource.path)?;
        Some(ToolPkgLogoBytes {
            resourceKey: logoResource.key,
            mimeType: logoResource.mime,
            fileName,
            bytes,
        })
    }

    #[allow(non_snake_case)]
    /// Returns enabled ToolPkg context-menu items applicable to one message sender.
    pub fn getToolPkgChatMessageMenuItems(
        &self,
        sender: &str,
        useEnglish: bool,
    ) -> Vec<ToolPkgChatMessageMenuItem> {
        let normalizedSender = sender.trim().to_ascii_lowercase();
        let mut items = self
            .getEnabledToolPkgContainerRuntimes()
            .into_iter()
            .flat_map(|runtime| {
                let normalizedSender = normalizedSender.clone();
                runtime
                    .chatMessageMenuItems
                    .into_iter()
                    .filter(move |item| {
                        item.senders.is_empty()
                            || item
                                .senders
                                .iter()
                                .any(|allowed| allowed == &normalizedSender)
                    })
                    .map(move |item| ToolPkgChatMessageMenuItem {
                        containerPackageName: runtime.packageName.clone(),
                        itemId: item.id,
                        title: item.title.resolve(useEnglish),
                        icon: item.icon,
                        order: item.order,
                        dialog: item.dialog.map(|dialog| {
                            operit_plugin_sdk::toolpkg::ToolPkgPackageModels::ToolPkgChatMessageMenuDialog {
                                screen: dialog.screen,
                                title: dialog.title.resolve(useEnglish),
                            }
                        }),
                    })
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.order
                .cmp(&right.order)
                .then(left.title.cmp(&right.title))
                .then(left.containerPackageName.cmp(&right.containerPackageName))
                .then(left.itemId.cmp(&right.itemId))
        });
        items
    }

    #[allow(non_snake_case)]
    /// Executes one enabled ToolPkg chat message context-menu item.
    pub async fn invokeToolPkgChatMessageMenuItem(
        &self,
        containerPackageName: &str,
        itemId: &str,
        chatId: &str,
        messageIndex: i32,
        message: serde_json::Value,
    ) -> Result<Option<String>, String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        let normalizedItemId = itemId.trim();
        let runtime = self
            .getEnabledToolPkgContainerRuntimes()
            .into_iter()
            .find(|runtime| runtime.packageName == normalizedContainerPackageName)
            .ok_or_else(|| {
                format!("Enabled ToolPkg container not found: {normalizedContainerPackageName}")
            })?;
        let item = runtime
            .chatMessageMenuItems
            .iter()
            .find(|item| item.id == normalizedItemId)
            .ok_or_else(|| {
                format!(
                    "ToolPkg chat message menu item not found: {}:{}",
                    runtime.packageName, normalizedItemId
                )
            })?;
        self.runToolPkgMainHook(
            &runtime.packageName,
            &item.function,
            operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_MESSAGE_MENU_ITEM,
            Some("chat_message_menu_item_click"),
            Some(&item.id),
            item.functionSource.as_deref(),
            serde_json::json!({
                "action": "click",
                "chatId": chatId,
                "messageIndex": messageIndex,
                "menuItemId": item.id,
                "message": message,
            }),
            None,
            None,
            None,
        ).await
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
    /// Returns slash commands contributed by enabled ToolPkg packages.
    pub fn getToolPkgCoreCommands(
        &self,
        useEnglish: bool,
    ) -> Result<Vec<ToolPkgCoreCommandInfo>, String> {
        self.packageRegistryReadiness.require_ready()?;
        let mut owners = BTreeMap::new();
        let mut commands = Vec::new();
        for runtime in self.getEnabledToolPkgContainerRuntimes() {
            for command in runtime.coreCommands {
                let key = command.name.to_lowercase();
                if let Some(existingOwner) = owners.insert(key, runtime.packageName.clone()) {
                    return Err(format!(
                        "duplicate enabled plugin command /{}: {} and {}",
                        command.name, existingOwner, runtime.packageName
                    ));
                }
                commands.push(ToolPkgCoreCommandInfo {
                    containerPackageName: runtime.packageName.clone(),
                    commandId: command.id,
                    name: command.name,
                    title: command.title.resolve(useEnglish),
                    description: command.description.resolve(useEnglish),
                    usage: command.usage,
                });
            }
        }
        commands.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        Ok(commands)
    }

    /// Builds a generic directory of all registered command declarations after canonical loading completes.
    #[allow(non_snake_case)]
    pub fn getToolPkgCoreCommandCatalog(&self, useEnglish: bool) -> Result<ToolPkgCoreCommandCatalog, String> {
        self.packageRegistryReadiness.require_ready()?;
        let enabled = self.getEnabledToolPkgContainerRuntimes().into_iter()
            .map(|runtime| runtime.packageName).collect::<BTreeSet<_>>();
        Ok(ToolPkgCoreCommandCatalog::new(
            self.toolPkgManager().getToolPkgContainerRuntimes(), enabled, useEnglish,
        ))
    }

    /// Resolves one plugin command from its directory and invokes only that selected declaration.
    #[allow(non_snake_case)]
    pub async fn executeToolPkgCoreCommand(
        &self,
        commandName: &str,
        args: &[String],
        jsonMode: bool,
    ) -> Result<ToolPkgCoreCommandExecutionResult, String> {
        self.packageRegistryReadiness.wait_until_ready().await?;
        let resolved = self.getToolPkgCoreCommandCatalog(false)?.resolve(commandName, &[])?
            .ok_or_else(|| format!("plugin command not found: /{commandName}"))?;
        self.executeResolvedToolPkgCoreCommand(&resolved, args, jsonMode).await
    }

    /// Executes an immutable resolved command token without performing another name lookup.
    #[allow(non_snake_case)]
    pub async fn executeResolvedToolPkgCoreCommand(
        &self,
        resolved: &ResolvedToolPkgCoreCommand,
        args: &[String],
        jsonMode: bool,
    ) -> Result<ToolPkgCoreCommandExecutionResult, String> {
        self.packageRegistryReadiness.wait_until_ready().await?;
        let info = resolved.info();
        if !self.getEnabledToolPkgContainerRuntimes().iter().any(|runtime| runtime.packageName == info.containerPackageName) {
            return Err(format!("plugin command is disabled: /{} ({})", info.name, info.containerPackageName));
        }
        let command = resolved.declaration();
        let raw = self
            .runToolPkgMainHook(
                &info.containerPackageName,
                &command.function,
                operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CORE_COMMAND,
                Some("core_command"),
                Some(&command.id),
                command.functionSource.as_deref(),
                serde_json::json!({
                    "commandId": command.id,
                    "commandName": command.name,
                    "args": args,
                    "json": jsonMode,
                }),
                None,
                None,
                None,
            ).await?
            .ok_or_else(|| format!("plugin command /{} returned no result", info.name))?;
        serde_json::from_str::<ToolPkgCoreCommandExecutionResult>(&raw).map_err(|error| {
            format!("plugin command /{} returned an invalid result: {error}", info.name)
        })
    }

    /// Renders a registered desktop widget and settles its initial load action for any host.
    #[allow(non_snake_case)]
    pub async fn renderToolPkgDesktopWidget(
        &self,
        containerPackageName: &str,
        widgetId: &str,
        instanceId: &str,
        useEnglish: bool,
    ) -> Result<ToolPkgDesktopWidgetSnapshot, String> {
        super::ToolPkgDesktopWidgetService::render(
            self,
            containerPackageName,
            widgetId,
            instanceId,
            useEnglish,
        )
        .await
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
    /// Runs a navigation hook asynchronously so proxy dispatch releases the package manager lock.
    pub async fn runToolPkgNavigationEntryAction(
        &self,
        containerPackageName: &str,
        entryId: &str,
        functionName: &str,
        inlineFunctionSource: Option<&str>,
        eventPayload: serde_json::Value,
    ) -> Result<Option<String>, String> {
        let actionContext =
            format!("package={containerPackageName}, entryId={entryId}, function={functionName}");
        AppLogger::i(
            PACKAGE_MANAGER_LOG_TAG,
            &format!("Navigation action started: {actionContext}"),
        );
        let result = self
            .toolPkgManager()
            .dispatchToolPkgHookAsync(
                &self.getEnabledPackageNames(),
                ToolPkgHookInvocation {
                    containerPackageName: self.normalizePackageName(containerPackageName),
                    functionName: functionName.to_string(),
                    event: operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_NAVIGATION_ENTRY_ACTION.to_string(),
                    eventName: Some("navigation_entry_action".to_string()),
                    pluginId: Some(entryId.to_string()),
                    inlineFunctionSource: inlineFunctionSource.map(str::to_string),
                    eventPayload,
                    executionContextKey: None,
                    runtimeKind: None,
                    envOverrides: BTreeMap::new(),
                    timestampMs: currentTimeMillis(),
                    timeoutMillis: 60_000,
                    dispatchIntermediateOnMain: true,
                    onIntermediateResult: None,
                },
            )
            .await;
        match &result {
            Ok(_) => AppLogger::i(
                PACKAGE_MANAGER_LOG_TAG,
                &format!("Navigation action completed: {actionContext}"),
            ),
            Err(error) => AppLogger::e(
                PACKAGE_MANAGER_LOG_TAG,
                &format!("Navigation action failed: {actionContext}, error={error}"),
            ),
        };
        result
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
        self.recordAndReturnPackageError(
            &normalizedPackageName,
            "bundled_import_failed",
            &format!(
                "Bundled external package not found: {}",
                normalizedPackageName
            ),
            "bundled_package",
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
        self.recordAndReturnPackageError(
            &normalizedContainerPackageName,
            "bundled_import_failed",
            &format!(
                "Bundled external ToolPkg container not found: {}",
                normalizedContainerPackageName
            ),
            "bundled_toolpkg",
        )
    }

    #[allow(non_snake_case)]
    fn importBundledExternalPackageCandidate(
        &mut self,
        result: PackageScanCandidateResult,
        packageName: &str,
    ) -> String {
        let sourcePath = result.sourcePath;
        let packageKind = if result.toolPkgLoadResult.is_some() {
            "bundled_toolpkg"
        } else {
            "bundled_package"
        };
        let Some(sourceAsset) = bundledExternalPluginAssetByName(
            self.toolHandler.runtimeSupport().as_ref(),
            &sourcePath,
        ) else {
            return self.recordAndReturnPackageError(
                &sourcePath,
                "bundled_import_failed",
                &format!("Bundled external package asset not found: {sourcePath}"),
                packageKind,
            );
        };
        let sourceSignature = sha256Hex(sourceAsset.bytes);
        let sourceFileName = packageSourceFileName(&sourcePath);
        let packagesDir = self.storePaths.packages_dir();
        if let Err(error) = self
            .fileSystemHost
            .makeDirectory(&hostPath(&packagesDir), true)
        {
            return self.recordAndReturnPackageError(
                &sourcePath,
                "bundled_import_failed",
                &format!("Error importing package: {error}"),
                packageKind,
            );
        }
        let destinationFile = self.storePaths.packages_dir().join(&sourceFileName);
        if let Err(error) = self
            .fileSystemHost
            .writeFileBytes(&hostPath(&destinationFile), sourceAsset.bytes)
        {
            return self.recordAndReturnPackageError(
                &sourcePath,
                "bundled_import_failed",
                &format!("Error importing package: {error}"),
                packageKind,
            );
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
            Err(error) => self.recordAndReturnPackageError(
                &sourcePath,
                "bundled_import_failed",
                &format!("{message}\nFailed to record bundled external import: {error}"),
                packageKind,
            ),
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
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getEffectivePackageTools(&self, packageName: &str) -> Option<ToolPackage> {
        self.pluginPackageManager.effectivePackage(packageName)
    }

    #[allow(non_snake_case)]
    /// Returns raw package tool metadata by package name.
    #[operit_route_macros::operit_plugin_sdk_expose]
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
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getAvailablePackages(&self) -> BTreeMap<String, ToolPackage> {
        self.pluginPackageManager.availablePackages()
    }

    /// Returns every package load or import issue currently shown by the manager.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getToolPkgLoadIssues(&self) -> Vec<ToolPkgLoadIssue> {
        let mut issues = self.toolPkgLoadIssues.clone();
        for entry in self.bundledExternalPackageScanCache.values() {
            appendUniqueToolPkgLoadIssues(&mut issues, &entry.result.issues);
        }
        appendUniqueToolPkgLoadIssues(&mut issues, &self.manualToolPkgLoadIssues);
        issues
    }

    /// Records one failed package operation for the next package-manager snapshot.
    #[allow(non_snake_case)]
    fn recordManualToolPkgLoadIssue(
        &mut self,
        sourcePath: &str,
        packageName: Option<&str>,
        code: &str,
        message: &str,
        packageKind: &str,
    ) {
        let displayName = match packageName {
            Some(name) => name.to_string(),
            None => sourcePath.to_string(),
        };
        let issue = newToolPkgLoadIssue(
            sourcePath.to_string(),
            packageName.map(str::to_string),
            displayName,
            code,
            message.to_string(),
            packageKind,
        );
        appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &[issue]);
    }

    /// Removes manual import issues resolved by a successful package operation.
    #[allow(non_snake_case)]
    fn clearManualToolPkgLoadIssues(&mut self, sourcePath: &str, packageName: Option<&str>) {
        self.manualToolPkgLoadIssues.retain(|issue| {
            issue.sourcePath != sourcePath
                && packageName.is_none_or(|name| issue.packageName.as_deref() != Some(name))
        });
    }

    /// Records an operation error and returns its original text to string callers.
    #[allow(non_snake_case)]
    fn recordAndReturnPackageError(
        &mut self,
        sourcePath: &str,
        code: &str,
        message: &str,
        packageKind: &str,
    ) -> String {
        self.recordManualToolPkgLoadIssue(sourcePath, None, code, message, packageKind);
        message.to_string()
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
        let mut members = vec![loadResult.containerPackage.name.clone()];
        members.extend(
            loadResult
                .subpackagePackages
                .iter()
                .map(|child| child.name.clone()),
        );
        let source = Path::new(&loadResult.containerRuntime.sourcePath)
            .file_name()
            .expect("Validated package source must have a filename")
            .to_string_lossy();
        if let Err(error) =
            self.registerScope(&loadResult.containerPackage.name, &source, false, members)
        {
            self.recordManualToolPkgLoadIssue(
                &loadResult.containerRuntime.sourcePath,
                Some(&loadResult.containerPackage.name),
                "scope_failed",
                &error,
                "toolpkg",
            );
            return false;
        }
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
    /// Schedules ToolPkg runtime listener delivery after the package manager call returns.
    pub(crate) fn notifyToolPkgRuntimeChangeListeners(&self) {
        let toolPkgManager = self.toolPkgManager().clone();
        let activeContainers = self.getEnabledToolPkgContainerRuntimes();
        let scheduler = self
            .context
            .hostRuntimeTaskSchedulerHost
            .clone()
            .expect("HostRuntimeTaskSchedulerHost is required for ToolPkg runtime changes");
        scheduler
            .scheduleHostRuntimeTask(
                "operit-toolpkg-runtime-change",
                Box::new(move || {
                    toolPkgManager.notifyToolPkgRuntimeChangeListeners(activeContainers);
                }),
            )
            .expect("ToolPkg runtime change task must be scheduled");
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
        self.packageRegistryReadiness.begin_scan();
        let previousContainerNames = self
            .toolPkgManager()
            .getToolPkgContainerRuntimes()
            .into_iter()
            .map(|runtime| runtime.packageName)
            .collect::<BTreeSet<_>>();
        let assetSnapshot = self.scanBuiltInPackageAssets();
        let syncError = self.syncBundledExternalImportRecords().err();
        let mut mergedSnapshot = self.scanExternalPackages(&assetSnapshot);
        if let Some(error) = syncError {
            mergedSnapshot.toolPkgLoadIssues.push(newToolPkgLoadIssue(
                self.storePaths.packages_dir().to_string_lossy().to_string(),
                None,
                "插件导入记录".to_string(),
                "scan_failed",
                format!("Bundled external package sync failed: {error}"),
                "package_manager",
            ));
        }
        let nextContainerNames = mergedSnapshot
            .toolPkgContainers
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        for packageName in previousContainerNames.union(&nextContainerNames) {
            self.toolPkgManager()
                .destroyToolPkgExecutionEngines(packageName);
        }
        self.applyPackageScanSnapshot(mergedSnapshot);
        self.notifyToolPkgRuntimeChangeListeners();
        let outcome = match self.getToolPkgLoadIssues().into_iter().next() {
            Some(issue) => Err(issue.message),
            None => Ok(()),
        };
        self.packageRegistryReadiness.complete_scan(outcome);
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
        packageId: String,
        version: String,
        author: Vec<String>,
        minifyArtifact: bool,
    ) -> Result<Vec<u8>, String> {
        let fileSystemHost = self.context.fileSystemHost.as_ref().ok_or_else(|| {
            "FileSystemHost is required for ToolPkg artifact protection".to_string()
        })?;
        let sourceBytes = fileSystemHost
            .readFileBytes(&sourcePath)
            .map_err(|error| error.to_string())?;
        let marketOrigin = ToolPkgMarketOrigin {
            market: "Operit".to_string(),
            toolpkgId: packageId,
            version,
            author,
        };
        operit_plugin_sdk::toolpkg::ToolPkgProtection::processArtifactNamedBytesWithMarketOrigin(
            &sourceBytes,
            &sourcePath,
            isToolPkg,
            &marketOrigin,
            minifyArtifact,
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
            if !self
                .fileSystemHost
                .fileExists(&hostPath(&sourceFile))
                .map(|info| info.exists && !info.isDirectory)
                .unwrap_or(false)
            {
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
                apiVersion: None,
            });
        }

        for runtime in self.toolPkgManager().getToolPkgContainerRuntimes() {
            if runtime.sourceType != ToolPkgSourceType::EXTERNAL {
                continue;
            }
            let sourceFile = PathBuf::from(&runtime.sourcePath);
            if !self
                .fileSystemHost
                .fileExists(&hostPath(&sourceFile))
                .map(|info| info.exists && !info.isDirectory)
                .unwrap_or(false)
            {
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
                apiVersion: Some(runtime.apiVersion),
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
        let mut results = Vec::new();
        for asset in self.toolHandler.runtimeSupport().builtinPluginAssets() {
            if isProgressTrackedPackageSource(asset.name) {
                beginPackageLoadProgress(asset.name);
            }
            let result = self.parseBuiltInPackageAsset(asset);
            if isProgressTrackedPackageSource(asset.name) {
                reportPackageScanProgress(&result);
            }
            results.push(result);
        }
        self.mergePackageScanCandidateResults(results, None)
    }

    #[allow(non_snake_case)]
    fn scanExternalPackages(&mut self, baseSnapshot: &PackageScanSnapshot) -> PackageScanSnapshot {
        let mut files = Vec::new();
        for scope in ["device", "space"] {
            let root =
                ExtensionStore::root("package", scope).expect("Package scope constants are valid");
            let directory = self.storePaths.runtime_storage_path(&root);
            let scan = (|| -> Result<(), String> {
                self.fileSystemHost
                    .makeDirectory(&hostPath(&directory), true)
                    .map_err(|e| e.to_string())?;
                // Upgrade old bundles before scanning; a file arriving before its descriptor
                // is staged data, not an unregistered device-local installation.
                let sharedSources = if scope == "space" {
                    Some(ExtensionStore::default().records("package")?.into_iter()
                        .filter(|record| record.scope == "space")
                        .map(|record| record.sourceName).collect::<BTreeSet<_>>())
                } else { None };
                let entries = self
                    .fileSystemHost
                    .listFiles(&hostPath(&directory))
                    .map_err(|e| e.to_string())?;
                files.extend(
                    entries
                        .into_iter()
                        .filter(|entry| !entry.isDirectory && sharedSources.as_ref()
                            .is_none_or(|sources| sources.contains(&entry.name)))
                        .map(|entry| directory.join(entry.name)),
                );
                Ok(())
            })();
            if let Err(error) = scan {
                let mut snapshot = baseSnapshot.clone();
                snapshot.toolPkgLoadIssues.push(newToolPkgLoadIssue(
                    hostPath(&directory),
                    None,
                    scope.to_string(),
                    "scan_failed",
                    error,
                    "external",
                ));
                return snapshot;
            }
        }
        files.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));

        let previousCache = self.externalPackageScanCache.clone();
        let mut nextCache = BTreeMap::new();
        let mut results = Vec::new();
        for file in files {
            let cacheKey = file.to_string_lossy().to_string();
            let isMarketToolPkg = match self.isInstalledMarketToolPkg(&file) {
                Ok(value) => value,
                Err(error) => {
                    let mut result = PackageScanCandidateResult::default();
                    result.sourcePath = cacheKey.clone();
                    result.phase = "external".to_string();
                    result.issues.push(newToolPkgLoadIssue(
                        cacheKey,
                        None,
                        hostPath(&file),
                        "scope_failed",
                        error,
                        "package",
                    ));
                    results.push(result);
                    continue;
                }
            };
            let signature = format!(
                "{}|{}",
                self.buildExternalPackageScanSignature(&file),
                if isMarketToolPkg {
                    "market"
                } else {
                    "external"
                }
            );
            if isExternalPackageCandidateFile(&file) {
                beginPackageLoadProgress(&cacheKey);
            }
            let result = previousCache
                .get(&cacheKey)
                .filter(|entry| entry.signature == signature)
                .map(|entry| entry.result.clone())
                .unwrap_or_else(|| {
                    if isMarketToolPkg {
                        self.parseMarketToolPkgCandidate(&file)
                    } else {
                        self.parseExternalPackageCandidate(&file)
                    }
                });
            if isExternalPackageCandidateFile(&file) {
                reportPackageScanProgress(&result);
            }
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
            match ToolPkgProtection::decodeUtf8(asset.bytes).and_then(|script| {
                JsPackageLoader::parse(&script).map(|package| ToolPackage {
                    is_built_in: true,
                    ..package
                })
            }) {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    asset.name.to_string(),
                    None,
                    packageProgressDisplayName(asset.name),
                    "package_parse",
                    error,
                    "javascript",
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
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    asset.name.to_string(),
                    None,
                    packageProgressDisplayName(asset.name),
                    "package_parse",
                    error,
                    "metadata",
                )),
            }
        } else if lowerName.ends_with(".toolpkg") {
            match self.loadToolPkgFromBuiltInAssetWithIssues(asset.name, asset.bytes) {
                Ok((loadResult, issues)) => {
                    result.toolPkgLoadResult = Some(loadResult);
                    result.issues.extend(issues);
                }
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    asset.name.to_string(),
                    None,
                    packageProgressDisplayName(asset.name),
                    "toolpkg_load",
                    error,
                    "toolpkg",
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
            match ToolPkgProtection::decodeUtf8(asset.bytes)
                .and_then(|script| JsPackageLoader::parse(&script))
            {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    asset.name.to_string(),
                    None,
                    packageProgressDisplayName(asset.name),
                    "package_parse",
                    error,
                    "javascript",
                )),
            }
        } else if lowerName.ends_with(".hjson") || lowerName.ends_with(".json") {
            match std::str::from_utf8(asset.bytes)
                .map_err(|error| error.to_string())
                .and_then(|content| JsPackageLoader::parse_metadata(content, ""))
            {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    asset.name.to_string(),
                    None,
                    packageProgressDisplayName(asset.name),
                    "package_parse",
                    error,
                    "metadata",
                )),
            }
        } else if lowerName.ends_with(".toolpkg") {
            match self.loadToolPkgFromBundledExternalAssetWithIssues(asset.name, asset.bytes) {
                Ok((loadResult, issues)) => {
                    result.toolPkgLoadResult = Some(loadResult);
                    result.issues.extend(issues);
                }
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    asset.name.to_string(),
                    None,
                    packageProgressDisplayName(asset.name),
                    "toolpkg_load",
                    error,
                    "toolpkg",
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
            match self.loadPackageFromJsFileResult(path) {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    sourcePath.clone(),
                    None,
                    packageProgressDisplayName(&sourcePath),
                    "package_parse",
                    error,
                    "javascript",
                )),
            }
        } else if lowerPath.ends_with(".hjson") {
            let parseResult = self
                .context
                .fileSystemHost
                .as_ref()
                .ok_or_else(|| {
                    "FileSystemHost is required for external package loading".to_string()
                })
                .and_then(|fileSystemHost| {
                    fileSystemHost
                        .readFile(&sourcePath)
                        .map_err(|error| error.to_string())
                })
                .and_then(|content| JsPackageLoader::parse_metadata(&content, ""));
            match parseResult {
                Ok(package) => result.toolPackage = Some(package),
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    sourcePath.clone(),
                    None,
                    packageProgressDisplayName(&sourcePath),
                    "package_parse",
                    error,
                    "metadata",
                )),
            }
        } else if lowerPath.ends_with(".toolpkg") {
            match self.installedPackageConfigScope(path).and_then(|scope| {
                self.loadToolPkgFromExternalFileWithIssues(path, scope)
            }) {
                Ok((loadResult, issues)) => {
                    result.toolPkgLoadResult = Some(loadResult);
                    result.issues.extend(issues);
                }
                Err(error) => result.issues.push(newToolPkgLoadIssue(
                    sourcePath.clone(),
                    None,
                    packageProgressDisplayName(&sourcePath),
                    "toolpkg_load",
                    error,
                    "toolpkg",
                )),
            }
        }
        result
    }

    /// Parses one locally sealed marketplace ToolPkg from the normal package storage directory.
    #[allow(non_snake_case)]
    fn parseMarketToolPkgCandidate(&self, path: &Path) -> PackageScanCandidateResult {
        let sourcePath = path.to_string_lossy().to_string();
        let mut result = PackageScanCandidateResult {
            phase: "external".to_string(),
            sourcePath: sourcePath.clone(),
            ..Default::default()
        };
        if !sourcePath.to_ascii_lowercase().ends_with(".toolpkg") {
            return result;
        }
        match self.installedPackageConfigScope(path).and_then(|scope| {
            self.loadToolPkgFromMarketFileWithIssues(path, scope)
        }) {
            Ok((loadResult, issues)) => {
                result.toolPkgLoadResult = Some(loadResult);
                result.issues.extend(issues);
            }
            Err(error) => result.issues.push(newToolPkgLoadIssue(
                sourcePath.clone(),
                None,
                packageProgressDisplayName(&sourcePath),
                "toolpkg_load",
                error,
                "market_toolpkg",
            )),
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
        let mut stagedToolPkgLoadIssues = baseSnapshot
            .map(|snapshot| snapshot.toolPkgLoadIssues.clone())
            .unwrap_or_default();

        for result in candidateResults {
            appendUniqueToolPkgLoadIssues(&mut stagedToolPkgLoadIssues, &result.issues);
            if let Some(packageMetadata) = result.toolPackage {
                if result.phase == "external"
                    && !Self::prepareExternalStandalonePackageOverride(
                        &packageMetadata.name,
                        &mut stagedAvailablePackages,
                        &mut stagedToolPkgContainers,
                        &mut stagedToolPkgSubpackages,
                    )
                {
                    stagedToolPkgLoadIssues.push(newToolPkgLoadIssue(
                        result.sourcePath.clone(),
                        Some(packageMetadata.name.clone()),
                        packageMetadata.display_name.resolve(false),
                        "duplicate_package",
                        format!(
                            "A package with name '{}' is already registered",
                            packageMetadata.name
                        ),
                        "package",
                    ));
                    continue;
                }
                if stagedAvailablePackages.contains_key(&packageMetadata.name) {
                    stagedToolPkgLoadIssues.push(newToolPkgLoadIssue(
                        result.sourcePath.clone(),
                        Some(packageMetadata.name.clone()),
                        packageMetadata.display_name.resolve(false),
                        "duplicate_package",
                        format!(
                            "A package with name '{}' is already registered",
                            packageMetadata.name
                        ),
                        "package",
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
                    stagedToolPkgLoadIssues.push(newToolPkgLoadIssue(
                        result.sourcePath.clone(),
                        Some(loadResult.containerPackage.name.clone()),
                        loadResult.containerRuntime.displayName.resolve(false),
                        "duplicate_package",
                        format!(
                            "A ToolPkg package with name '{}' is already registered",
                            loadResult.containerPackage.name
                        ),
                        "toolpkg",
                    ));
                    continue;
                }
                let packageName = loadResult.containerPackage.name.clone();
                let displayName = loadResult.containerRuntime.displayName.resolve(false);
                if !Self::registerToolPkgInto(
                    loadResult,
                    &mut stagedAvailablePackages,
                    &mut stagedToolPkgContainers,
                    &mut stagedToolPkgSubpackages,
                ) {
                    stagedToolPkgLoadIssues.push(newToolPkgLoadIssue(
                        result.sourcePath.clone(),
                        Some(packageName),
                        displayName,
                        "toolpkg_registration",
                        "ToolPkg package registration failed".to_string(),
                        "toolpkg",
                    ));
                }
            }
        }

        PackageScanSnapshot {
            availablePackages: stagedAvailablePackages,
            toolPkgContainers: stagedToolPkgContainers,
            toolPkgSubpackages: stagedToolPkgSubpackages,
            toolPkgLoadIssues: stagedToolPkgLoadIssues,
        }
    }

    /// Refreshes synchronized flags without destroying running JavaScript engines.
    pub fn refreshExtensionSettings(&mut self) -> Result<(), String> {
        let records = ExtensionStore::default().records("package")?;
        let mut next = BTreeMap::new();
        for record in records {
            let id = record.id.clone();
            if next.insert(id.clone(), record).is_some() {
                return Err(format!("Extension ownership conflict: {id}"));
            }
        }
        *self.scopeState.lock().map_err(|e| e.to_string())? = next;
        self.notifyToolPkgRuntimeChangeListeners();
        Ok(())
    }

    /// Returns validated in-memory ownership state for the currently loaded catalog.
    fn scopeRecords(&self) -> Vec<ExtensionRecord> {
        self.scopeState
            .lock()
            .expect("package scope state mutex poisoned")
            .values()
            .filter(|record| self.availablePackages().contains_key(&record.id))
            .cloned()
            .collect()
    }

    /// Persists scope-owned settings before updating the validated runtime snapshot.
    fn writeScopeSettings(&self, id: &str, settings: serde_json::Value) -> Result<(), String> {
        let store = ExtensionStore::default();
        store.setSettings("package", id, settings)?;
        let record = store.record("package", id)?;
        self.scopeState
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id.to_string(), record);
        Ok(())
    }

    /// Registers a concrete imported source without changing its plugin API version.
    fn registerScope(
        &self,
        id: &str,
        source: &str,
        builtin: bool,
        members: Vec<String>,
    ) -> Result<(), String> {
        let store = ExtensionStore::default();
        store.registerDevice("package", id, source, serde_json::json!({
            "builtin": builtin, "members": members, "enabledNames": [], "disabledNames": [],
            "subpackageStates": {}, "order": 0,
            "installationId": encodeMarketToolPkgInstallationId(&self.marketToolPkgInstallationId()?)
        }))?;
        let record = store.record("package", id)?;
        self.scopeState
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id.to_string(), record);
        Ok(())
    }

    /// Moves the parent installation and all child-package state to the selected scope.
    pub fn setPackageScope(&mut self, packageName: &str, scope: &str) -> Result<(), String> {
        let name = match self
            .toolPkgManager()
            .resolveToolPkgSubpackageRuntimeInternal(packageName)
        {
            Some(child) => child.containerPackageName,
            None => self.normalizePackageName(packageName),
        };
        ExtensionStore::default().moveScope("package", &name, scope)?;
        self.loadAvailablePackages();
        Ok(())
    }

    #[allow(non_snake_case)]
    fn applyPackageScanSnapshot(&mut self, mut snapshot: PackageScanSnapshot) {
        self.scopeState
            .lock()
            .expect("package scope state mutex poisoned")
            .clear();
        let mut rejected = Vec::new();
        for (name, package) in &snapshot.availablePackages {
            if snapshot.toolPkgSubpackages.contains_key(name) {
                continue;
            }
            let registration = (|| -> Result<(), String> {
                let (builtin, sourceName, members) = match snapshot.toolPkgContainers.get(name) {
                    Some(container) => {
                        let builtin = container.sourceType == ToolPkgSourceType::ASSET;
                        let sourceName = if builtin {
                            String::new()
                        } else {
                            Path::new(&container.sourcePath)
                                .file_name()
                                .ok_or("Package source filename is missing")?
                                .to_string_lossy()
                                .to_string()
                        };
                        let mut members = vec![name.clone()];
                        members.extend(
                            container
                                .subpackages
                                .iter()
                                .map(|child| child.packageName.clone()),
                        );
                        (builtin, sourceName, members)
                    }
                    None => {
                        let sourceName = if package.is_built_in {
                            String::new()
                        } else {
                            let entry = self
                                .externalPackageScanCache
                                .values()
                                .find(|entry| {
                                    entry
                                        .result
                                        .toolPackage
                                        .as_ref()
                                        .is_some_and(|candidate| candidate.name == *name)
                                })
                                .ok_or("Scanned package lost its concrete source")?;
                            Path::new(&entry.result.sourcePath)
                                .file_name()
                                .ok_or("Package source filename is missing")?
                                .to_string_lossy()
                                .to_string()
                        };
                        (package.is_built_in, sourceName, vec![name.clone()])
                    }
                };
                let store = ExtensionStore::default();
                let existing = store
                    .records("package")?
                    .into_iter()
                    .any(|record| record.id == *name);
                if !existing {
                    self.registerScope(name, &sourceName, builtin, members)?;
                }
                let record = store.record("package", name)?;
                if record.sourceName != sourceName
                    || record.settings["builtin"].as_bool() != Some(builtin)
                {
                    return Err(format!("Package source does not match its owner: {name}"));
                }
                if !builtin {
                    let sourcePath = match snapshot.toolPkgContainers.get(name) {
                        Some(container) => container.sourcePath.clone(),
                        None => self
                            .externalPackageScanCache
                            .values()
                            .find(|entry| {
                                entry
                                    .result
                                    .toolPackage
                                    .as_ref()
                                    .is_some_and(|candidate| candidate.name == *name)
                            })
                            .ok_or("Package source is missing")?
                            .result
                            .sourcePath
                            .clone(),
                    };
                    let actual = self
                        .storePaths
                        .runtime_storage_key(Path::new(&sourcePath))?;
                    let expected = format!(
                        "{}/{}",
                        ExtensionStore::root("package", &record.scope)?,
                        record.sourceName
                    );
                    if actual != expected {
                        return Err(format!("Package location conflicts with its owner: {name}"));
                    }
                }
                self.scopeState
                    .lock()
                    .map_err(|e| e.to_string())?
                    .insert(name.clone(), record);
                Ok(())
            })();
            if let Err(error) = registration {
                snapshot.toolPkgLoadIssues.push(newToolPkgLoadIssue(
                    name.clone(),
                    Some(name.clone()),
                    name.clone(),
                    "scope_failed",
                    error,
                    "package",
                ));
                rejected.push(name.clone());
            }
        }
        for name in rejected {
            if snapshot.toolPkgContainers.contains_key(&name) {
                Self::removeToolPkgContainerFromTargets(
                    &name,
                    &mut snapshot.availablePackages,
                    &mut snapshot.toolPkgContainers,
                    &mut snapshot.toolPkgSubpackages,
                );
            } else {
                snapshot.availablePackages.remove(&name);
            }
        }
        self.pluginPackageManager
            .replaceAvailablePackages(snapshot.availablePackages);
        self.pluginPackageManager
            .replaceToolPkgRuntimes(snapshot.toolPkgContainers, snapshot.toolPkgSubpackages);
        self.toolPkgLoadIssues = snapshot.toolPkgLoadIssues;
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
        let metadata = self.fileSystemHost.fileInfo(&hostPath(file)).ok();
        format!(
            "{}|{}|{}",
            file.to_string_lossy(),
            metadata.as_ref().map(|value| value.size).unwrap_or(0),
            metadata.map(|value| value.lastModified).unwrap_or_default()
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
            if !self
                .fileSystemHost
                .fileExists(&hostPath(&destinationFile))
                .map(|info| info.exists && !info.isDirectory)
                .unwrap_or(false)
            {
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
                self.fileSystemHost
                    .writeFileBytes(&hostPath(&destinationFile), sourceAsset.bytes)
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
            self.saveBundledExternalImportRecordsLocally(&records)?;
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
            if !self
                .fileSystemHost
                .fileExists(&hostPath(&destinationFile))
                .map(|info| info.exists && !info.isDirectory)
                .unwrap_or(false)
            {
                continue;
            }
            let Some(sourceAsset) = bundledExternalPluginAssetByName(
                self.toolHandler.runtimeSupport().as_ref(),
                &result.sourcePath,
            ) else {
                continue;
            };
            let destinationBytes = self
                .fileSystemHost
                .readFileBytes(&hostPath(&destinationFile))
                .map_err(|error| error.to_string())?;
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
    fn buildFileContentSignature(&self, file: &Path) -> Result<String, String> {
        let bytes = self
            .fileSystemHost
            .readFileBytes(&hostPath(file))
            .map_err(|error| error.to_string())?;
        Ok(format!("{:x}", Sha256::digest(&bytes)))
    }

    #[allow(non_snake_case)]
    fn filesHaveSameContent(&self, left: &Path, right: &Path) -> Result<bool, String> {
        let leftBytes = self
            .fileSystemHost
            .readFileBytes(&hostPath(left))
            .map_err(|error| error.to_string())?;
        let rightBytes = self
            .fileSystemHost
            .readFileBytes(&hostPath(right))
            .map_err(|error| error.to_string())?;
        Ok(leftBytes == rightBytes)
    }

    /// Imports a package file from external storage into package storage.
    #[allow(non_snake_case)]
    pub fn addPackageFileFromExternalStorage(&mut self, filePath: &str) -> String {
        match self.addPackageFileFromExternalStorageResult(filePath) {
            Ok(result) => formatExternalPackageImportResult(&result),
            Err(error) => error,
        }
    }

    /// Imports a package file from external storage and returns structured metadata.
    #[allow(non_snake_case)]
    pub fn addPackageFileFromExternalStorageResult(
        &mut self,
        filePath: &str,
    ) -> Result<ExternalPackageImportResult, String> {
        let result = self.addPackageFileFromExternalStorageResultInternal(filePath);
        match &result {
            Ok(_importResult) => self.clearManualToolPkgLoadIssues(filePath, None),
            Err(error) => {
                self.recordManualToolPkgLoadIssue(filePath, None, "import_failed", error, "package")
            }
        }
        result
    }

    /// Performs package-file validation and registration without changing issue state.
    #[allow(non_snake_case)]
    fn addPackageFileFromExternalStorageResultInternal(
        &mut self,
        filePath: &str,
    ) -> Result<ExternalPackageImportResult, String> {
        let file = PathBuf::from(filePath);
        let downloadedFileExists = matches!(
            self.fileSystemHost.fileExists(filePath),
            Ok(info) if info.exists && !info.isDirectory
        );
        if !downloadedFileExists {
            return Err(format!("Cannot access file at path: {filePath}"));
        }

        let lowerPath = filePath.to_ascii_lowercase();
        let isJsLike = lowerPath.ends_with(".js") || lowerPath.ends_with(".ts");
        let isHjson = lowerPath.ends_with(".hjson");
        let isToolPkg = lowerPath.ends_with(".toolpkg");
        if !isJsLike && !isHjson && !isToolPkg {
            return Err("Only HJSON, JavaScript (.js), TypeScript (.ts) and ToolPkg (.toolpkg) package files are supported"
                .to_string());
        }

        if isToolPkg {
            let (loadResult, loadIssues) = self
                .loadToolPkgFromExternalFileWithIssues(&file, ToolPkgConfigScope::Device)
                .map_err(|error| format!("Error importing package: {error}"))?;
            appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &loadIssues);
            let packageName = loadResult.containerPackage.name.clone();
            let marketOrigin = loadResult.marketOrigin.clone();
            if !self
                .toolPkgManager()
                .canRegisterToolPkg(&loadResult, self.availablePackages())
            {
                return Err(format!(
                    "A package with name '{}' already exists in available packages",
                    packageName
                ));
            }
            self.storePaths
                .ensure_packages_dir()
                .map_err(|error| format!("Error importing package: {error}"))?;
            let fileName = file
                .file_name()
                .ok_or_else(|| "Error importing package: invalid file name".to_string())?;
            let destinationFile = self.storePaths.packages_dir().join(fileName);
            if file != destinationFile {
                self.fileSystemHost
                    .copyFile(&hostPath(&file), &hostPath(&destinationFile), false)
                    .map_err(|error| format!("Error importing package: {error}"))?;
            }
            let (importedLoadResult, importedLoadIssues) = self
                .loadToolPkgFromExternalFileWithIssues(&destinationFile, ToolPkgConfigScope::Device)
                .map_err(|error| format!("Error importing package: {error}"))?;
            appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &importedLoadIssues);
            if !self.registerToolPkg(importedLoadResult) {
                return Err(format!(
                    "A package with name '{}' already exists in available packages",
                    packageName
                ));
            }
            let sourceNotice = marketOrigin.map(|origin| {
                format!(
                    "this is the {} marketplace ToolPkg '{}' (version: {}, author: {}). Please support the original author and beware of resales.",
                    origin.market,
                    packageName,
                    origin.version,
                    origin.author.join(", ")
                )
            });
            return Ok(ExternalPackageImportResult {
                packageName,
                packageFormat: "toolpkg".to_string(),
                storedPath: destinationFile.to_string_lossy().to_string(),
                sourceNotice,
            });
        }

        let packageFormat = if isHjson {
            "hjson"
        } else if lowerPath.ends_with(".ts") {
            "typescript"
        } else {
            "javascript"
        };
        let packageMetadata = if isHjson {
            let content = self
                .fileSystemHost
                .readFile(&hostPath(&file))
                .map_err(|error| format!("Error importing package: {error}"))?;
            JsPackageLoader::parse_metadata(&content, "")
                .map_err(|error| format!("Error importing package: {error}"))?
        } else {
            self.loadPackageFromJsFileResult(&file)
                .map_err(|error| format!("Failed to parse {packageFormat} package file: {error}"))?
        };

        if self.availablePackages().contains_key(&packageMetadata.name)
            || self
                .toolPkgManager()
                .isToolPkgContainer(&packageMetadata.name)
            || self.toolPkgManager().hasSubpackage(&packageMetadata.name)
        {
            return Err(format!(
                "A package with name '{}' already exists in available packages",
                packageMetadata.name
            ));
        }

        self.storePaths
            .ensure_packages_dir()
            .map_err(|error| format!("Error importing package: {error}"))?;
        let fileName = file
            .file_name()
            .ok_or_else(|| "Error importing package: invalid file name".to_string())?;
        let destinationFile = self.storePaths.packages_dir().join(fileName);
        if file != destinationFile {
            self.fileSystemHost
                .copyFile(&hostPath(&file), &hostPath(&destinationFile), false)
                .map_err(|error| format!("Error importing package: {error}"))?;
        }

        let scriptMarketOrigin = if isJsLike {
            let script = self
                .fileSystemHost
                .readFile(&hostPath(&file))
                .map_err(|error| format!("Error importing package: {error}"))?;
            ToolPkgProtection::readScriptMarketOrigin(&script, &packageMetadata.name)
        } else {
            None
        };
        self.registerScope(
            &packageMetadata.name,
            &fileName.to_string_lossy(),
            false,
            vec![packageMetadata.name.clone()],
        )?;
        self.pluginPackageManager.registerPackage(ToolPackage {
            is_built_in: false,
            ..packageMetadata.clone()
        });
        let sourceNotice = scriptMarketOrigin.map(|origin| {
            format!(
                "this is the {} marketplace script '{}' (version: {}, author: {}). Please support the original author and beware of resales.",
                origin.market,
                packageMetadata.name,
                origin.version,
                origin.author.join(", ")
            )
        });
        Ok(ExternalPackageImportResult {
            packageName: packageMetadata.name,
            packageFormat: packageFormat.to_string(),
            storedPath: destinationFile.to_string_lossy().to_string(),
            sourceNotice,
        })
    }

    /// Imports one marketplace artifact from bytes after validating its declared digest.
    #[allow(non_snake_case)]
    pub fn addMarketArtifactBytes(
        &mut self,
        bytes: Vec<u8>,
        fileName: String,
        expectedSha256: String,
    ) -> String {
        let normalizedSha256 = expectedSha256.trim().to_ascii_lowercase();
        if !isSha256Hex(&normalizedSha256) {
            return self.recordAndReturnPackageError(
                &fileName,
                "market_import_failed",
                "Market artifact SHA-256 is invalid",
                "market_artifact",
            );
        }
        if sha256Hex(&bytes) != normalizedSha256 {
            return self.recordAndReturnPackageError(
                &fileName,
                "market_import_failed",
                "Market artifact SHA-256 mismatch",
                "market_artifact",
            );
        }
        let fileName = fileName.trim();
        if fileName.is_empty()
            || Path::new(fileName)
                .file_name()
                .and_then(|name| name.to_str())
                != Some(fileName)
        {
            return self.recordAndReturnPackageError(
                &fileName,
                "market_import_failed",
                "Market artifact file name is invalid",
                "market_artifact",
            );
        }
        if let Err(error) = self.storePaths.ensure_packages_dir() {
            return self.recordAndReturnPackageError(
                &fileName,
                "market_import_failed",
                &format!("Error importing market artifact: {error}"),
                "market_artifact",
            );
        }
        let temporaryDirectory = self.storePaths.toolpkg_cache_dir().join(format!(
            ".market-download-{}-{normalizedSha256}",
            currentTimeMillis()
        ));
        if let Err(error) = self
            .fileSystemHost
            .makeDirectory(&hostPath(&temporaryDirectory), true)
        {
            return self.recordAndReturnPackageError(
                fileName,
                "market_import_failed",
                &format!("Error preparing market artifact download: {error}"),
                "market_artifact",
            );
        }
        // Keep staging outside scanned packages and preserve the final archive name.
        let temporaryFile = temporaryDirectory.join(fileName);
        if let Err(error) = self
            .fileSystemHost
            .writeFileBytes(&hostPath(&temporaryFile), &bytes)
        {
            let _ = self
                .fileSystemHost
                .deleteFile(&hostPath(&temporaryDirectory), true);
            return self.recordAndReturnPackageError(
                &fileName,
                "market_import_failed",
                &format!("Error importing market artifact: {error}"),
                "market_artifact",
            );
        }
        let result =
            self.addPackageFileFromExternalStorageResultInternal(&temporaryFile.to_string_lossy());
        let _ = self
            .fileSystemHost
            .deleteFile(&hostPath(&temporaryDirectory), true);
        match result {
            Ok(importResult) => {
                self.clearManualToolPkgLoadIssues(fileName, None);
                formatExternalPackageImportResult(&importResult)
            }
            Err(error) => self.recordAndReturnPackageError(
                fileName,
                "market_import_failed",
                &error,
                if fileName.to_ascii_lowercase().ends_with(".toolpkg") {
                    "market_toolpkg"
                } else {
                    "market_artifact"
                },
            ),
        }
    }

    /// Creates a package-manager-owned staging path for one market artifact.
    #[allow(non_snake_case)]
    pub fn prepareMarketArtifactDownload(&self, fileName: &str) -> Result<String, String> {
        let fileName = fileName.trim();
        if fileName.is_empty()
            || Path::new(fileName)
                .file_name()
                .and_then(|name| name.to_str())
                != Some(fileName)
        {
            return Err("Market artifact file name is invalid".to_string());
        }
        let directory = self
            .storePaths
            .toolpkg_cache_dir()
            .join(format!(".market-download-{}", currentTimeMillis()));
        self.fileSystemHost
            .makeDirectory(&hostPath(&directory), true)
            .map_err(|error| format!("Error preparing market artifact download: {error}"))?;
        Ok(directory.join(fileName).to_string_lossy().to_string())
    }

    /// Imports a downloaded market artifact after checking its declared SHA-256.
    #[allow(non_snake_case)]
    pub fn addMarketArtifactFile(&mut self, filePath: String, expectedSha256: String) -> String {
        let result = (|| {
            let normalizedSha256 = expectedSha256.trim().to_ascii_lowercase();
            if !isSha256Hex(&normalizedSha256) {
                return Err("Market artifact SHA-256 is invalid".to_string());
            }
            let bytes = self
                .fileSystemHost
                .readFileBytes(&filePath)
                .map_err(|error| format!("Error reading market artifact: {error}"))?;
            if sha256Hex(&bytes) != normalizedSha256 {
                return Err("Market artifact SHA-256 mismatch".to_string());
            }
            self.addPackageFileFromExternalStorageResultInternal(&filePath)
        })();
        match result {
            Ok(importResult) => {
                self.clearManualToolPkgLoadIssues(&filePath, None);
                formatExternalPackageImportResult(&importResult)
            }
            Err(error) => self.recordAndReturnPackageError(
                &filePath,
                "market_import_failed",
                &error,
                "market_artifact",
            ),
        }
    }

    /// Imports one signed marketplace ToolPkg as a locally authenticated package archive.
    #[allow(non_snake_case)]
    pub fn addMarketToolPkgFileFromExternalStorage(
        &mut self,
        filePath: &str,
        expectedMarketAssetSha256: &str,
    ) -> String {
        let downloadedFileExists = matches!(
            self.fileSystemHost.fileExists(filePath),
            Ok(info) if info.exists && !info.isDirectory
        );
        if !downloadedFileExists {
            return self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                &format!("Cannot access market ToolPkg at path: {filePath}"),
                "market_toolpkg",
            );
        }
        if !filePath.to_ascii_lowercase().ends_with(".toolpkg") {
            return self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                "Market ToolPkg install only supports .toolpkg files",
                "market_toolpkg",
            );
        }
        let normalizedAssetSha256 = expectedMarketAssetSha256.trim().to_ascii_lowercase();
        if !isSha256Hex(&normalizedAssetSha256) {
            return self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                "Market ToolPkg asset SHA-256 is invalid",
                "market_toolpkg",
            );
        }
        let downloadedBytes = match self.fileSystemHost.readFileBytes(filePath) {
            Ok(bytes) => bytes,
            Err(error) => {
                return self.recordAndReturnPackageError(
                    filePath,
                    "market_import_failed",
                    &format!("Error installing market ToolPkg: {error}"),
                    "market_toolpkg",
                )
            }
        };
        if !ToolPkgProtection::isMarketArchive(&downloadedBytes) {
            return self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                "Market ToolPkg archive is missing marketplace authentication",
                "market_toolpkg",
            );
        }
        if sha256Hex(&downloadedBytes) != normalizedAssetSha256 {
            return self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                "Market ToolPkg asset SHA-256 mismatch",
                "market_toolpkg",
            );
        }
        let rawArchive =
            match ToolPkgProtection::unwrapMarketArchive(&downloadedBytes) {
                Ok(bytes) => bytes,
                Err(_) => return self.recordAndReturnPackageError(
                    filePath,
                    "market_import_failed",
                    "Encrypted ToolPkg decryption failed. The plugin is damaged or not authorized",
                    "market_toolpkg",
                ),
            };
        match ToolPkgProtection::toolPkgArchiveContainsProtectedEntries(&rawArchive) {
            Ok(true) => {}
            Ok(false) => {
                return self.recordAndReturnPackageError(
                    filePath,
                    "market_import_failed",
                    "Market ToolPkg archive does not contain protected entries",
                    "market_toolpkg",
                )
            }
            Err(error) => {
                return self.recordAndReturnPackageError(
                    filePath,
                    "market_import_failed",
                    &format!("Error installing market ToolPkg: {error}"),
                    "market_toolpkg",
                )
            }
        }
        let installationId = match self.marketToolPkgInstallationId() {
            Ok(value) => value,
            Err(error) => {
                return self.recordAndReturnPackageError(
                    filePath,
                    "market_import_failed",
                    &format!("Error installing market ToolPkg: {error}"),
                    "market_toolpkg",
                )
            }
        };
        let installedArchive =
            match ToolPkgProtection::attachMarketInstallSeal(&rawArchive, &installationId) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return self.recordAndReturnPackageError(
                        filePath,
                        "market_import_failed",
                        &format!("Error installing market ToolPkg: {error}"),
                        "market_toolpkg",
                    )
                }
            };
        if let Err(error) = self.storePaths.ensure_packages_dir() {
            return self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                &format!("Error installing market ToolPkg: {error}"),
                "market_toolpkg",
            );
        }
        let packagesDir = self.storePaths.packages_dir();
        let stagingFile =
            packagesDir.join(format!(".market-install-{normalizedAssetSha256}.toolpkg"));
        let destinationFile = packagesDir.join(format!(
            "{MARKET_TOOLPKG_FILE_PREFIX}{normalizedAssetSha256}.toolpkg"
        ));
        let mut installed = false;
        let mut destinationStored = false;
        let outcome = (|| -> Result<String, String> {
            self.fileSystemHost
                .writeFileBytes(&hostPath(&stagingFile), &installedArchive)
                .map_err(|error| error.to_string())?;
            if !self.isInstalledMarketToolPkg(&stagingFile)? {
                return Err("ToolPkg market installation authentication failed".to_string());
            }
            let (preview, previewIssues) =
                self.loadToolPkgFromMarketFileWithIssues(&stagingFile, ToolPkgConfigScope::Device)?;
            appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &previewIssues);
            let packageName = preview.containerPackage.name.clone();
            if !self
                .toolPkgManager()
                .canRegisterToolPkg(&preview, self.availablePackages())
            {
                return Err(format!(
                    "A package with name '{}' already exists in available packages",
                    packageName
                ));
            }
            let destinationExists = matches!(
                self.fileSystemHost.fileExists(&hostPath(&destinationFile)),
                Ok(info) if info.exists
            );
            if destinationExists {
                self.fileSystemHost
                    .deleteFile(&hostPath(&destinationFile), false)
                    .map_err(|error| error.to_string())?;
            }
            self.fileSystemHost
                .moveFile(&hostPath(&stagingFile), &hostPath(&destinationFile))
                .map_err(|error| error.to_string())?;
            destinationStored = true;
            let (loaded, loadedIssues) =
                self.loadToolPkgFromMarketFileWithIssues(&destinationFile, ToolPkgConfigScope::Device)?;
            appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &loadedIssues);
            if !self.registerToolPkg(loaded) {
                return Err(format!(
                    "Failed to register toolpkg '{}' due to naming conflict",
                    packageName
                ));
            }
            installed = true;
            Ok(format!("Successfully imported toolpkg: {packageName}"))
        })();
        let stagingExists = matches!(
            self.fileSystemHost.fileExists(&hostPath(&stagingFile)),
            Ok(info) if info.exists
        );
        if stagingExists {
            let _ = self
                .fileSystemHost
                .deleteFile(&hostPath(&stagingFile), false);
        }
        let destinationExists = matches!(
            self.fileSystemHost.fileExists(&hostPath(&destinationFile)),
            Ok(info) if info.exists
        );
        if !installed && destinationStored && destinationExists {
            let _ = self
                .fileSystemHost
                .deleteFile(&hostPath(&destinationFile), false);
        }
        match outcome {
            Ok(message) => message,
            Err(error) => self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                &format!("Error installing market ToolPkg: {error}"),
                "market_toolpkg",
            ),
        }
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

    /// Returns the registered main script for runtime-owned ToolPkg IPC dispatch.
    #[allow(non_snake_case)]
    pub fn getRegisteredToolPkgMainScript(&self, containerPackageName: &str) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        self.toolPkgManager()
            .getRegisteredToolPkgMainScript(&normalizedContainerPackageName)
    }

    #[allow(non_snake_case)]
    pub(crate) fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        normalizedResourcePath: &str,
    ) -> Option<Vec<u8>> {
        let resourceFile = self.resolveToolPkgResourceFile(runtime, normalizedResourcePath)?;
        if !self
            .fileSystemHost
            .fileExists(&hostPath(&resourceFile))
            .map(|info| info.exists && !info.isDirectory)
            .unwrap_or(false)
        {
            return None;
        }
        let bytes = self
            .fileSystemHost
            .readFileBytes(&hostPath(&resourceFile))
            .ok()?;
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
    /// Calls one declared ToolPkg WASM export.
    pub fn callToolPkgWasm(
        &self,
        request: JsToolPkgWasmRequest,
        preferEnabledContainer: bool,
    ) -> Result<JsToolPkgWasmResult, String> {
        let bytes = ToolPkgPackageService::new(self).readToolPkgWasmModuleBytes(
            &request.package_target,
            &request.module_id,
            &request.export_name,
            preferEnabledContainer,
        )?;
        operit_plugin_sdk::toolpkg::ToolPkgWasmRuntime::callWasmExport(
            &bytes,
            &request.export_name,
            &request.args,
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
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<Result<Option<String>, String>> {
        self.runToolPkgMainHookWithTimeoutMillis(
            containerPackageName,
            functionName,
            event,
            eventName,
            pluginId,
            inlineFunctionSource,
            eventPayload,
            executionContextKey,
            runtimeKind,
            onIntermediateResult,
            60_000,
        )
    }

    #[allow(non_snake_case)]
    /// Runs a main hook exported by a ToolPkg container with one explicit millisecond timeout.
    pub fn runToolPkgMainHookWithTimeoutMillis(
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
        timeoutMillis: u64,
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<Result<Option<String>, String>> {
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
                timeoutMillis,
                dispatchIntermediateOnMain: true,
                onIntermediateResult,
            },
        )
    }

    /// Performs MCP discovery before synchronous package metadata/activation, without holding its lock.
    pub async fn prepareMcpPackage(
        manager: &Arc<Mutex<Self>>, packageName: &str,
    ) -> Result<(), String> {
        let mcpManager = {
            let guard = manager.lock().expect("package manager mutex poisoned");
            if !guard.isRegisteredMCPServer(packageName) { return Ok(()); }
            guard.mcpManager.clone()
        };
        let client = mcpManager.getOrCreateClient(packageName).await.ok_or_else(|| {
            mcpManager.getLastConnectionFailureReason(packageName)
                .unwrap_or_else(|| format!("Cannot connect to MCP server: {packageName}"))
        })?;
        if !client.ping() { return Err(format!("MCP server is not ready: {packageName}")); }
        Ok(())
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
        let tools = crate::tools::mcp_runtime::plugins::MCPBridgeClient::MCPBridgeClient::new(
            self.context.clone(), serverConfig.name.clone(),
        ).getTools();
        let mcpLoadResult = MCPPackage::fromToolMetadata(serverConfig, tools);
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
    /// Reads state from each installed extension's exact owner.
    fn decodeEnabledPackageNamesFromPrefs(&self) -> Vec<String> {
        let mut names = Vec::new();
        for record in self.scopeRecords() {
            let values: Vec<String> =
                serde_json::from_value(record.settings["enabledNames"].clone())
                    .expect("Registered package names must have the declared schema");
            names.extend(values);
        }
        self.normalizeEnabledPackageNames(&names)
    }

    #[allow(non_snake_case)]
    /// Reads the persisted ToolPkg container order.
    fn decodeToolPkgContainerOrderFromPrefs(&self) -> Vec<String> {
        let mut records = self.scopeRecords();
        records.sort_by_key(|record| {
            (
                record.settings["order"]
                    .as_u64()
                    .expect("Registered package rank must be an integer"),
                record.id.clone(),
            )
        });
        records.into_iter().map(|record| record.id).collect()
    }

    #[allow(non_snake_case)]
    /// Calculates prerequisite issues for one ToolPkg runtime from installed package state.
    fn withToolPkgDependencyIssues(
        &self,
        mut runtime: ToolPkgContainerRuntime,
        enabledPackageNames: &BTreeSet<String>,
        order: &BTreeMap<String, usize>,
    ) -> ToolPkgContainerRuntime {
        runtime.dependencyIssues = runtime
            .requires
            .iter()
            .filter_map(|requirement| {
                self.toolPkgDependencyIssue(
                    &runtime.packageName,
                    requirement,
                    enabledPackageNames,
                    order,
                )
            })
            .collect();
        runtime
    }

    #[allow(non_snake_case)]
    /// Resolves one manifest prerequisite against installed and enabled ToolPkg containers.
    fn toolPkgDependencyIssue(
        &self,
        packageName: &str,
        requirement: &ToolPkgManifestRequirement,
        enabledPackageNames: &BTreeSet<String>,
        order: &BTreeMap<String, usize>,
    ) -> Option<ToolPkgDependencyIssue> {
        let dependencyId = self.normalizePackageName(&requirement.id);
        let dependency = self
            .toolPkgManager()
            .getToolPkgContainerRuntime(&dependencyId);
        let Some(dependency) = dependency else {
            return Some(ToolPkgDependencyIssue {
                id: dependencyId,
                code: "missing".to_string(),
                requiredMinVersion: requirement.minVersion.clone(),
                requiredMaxVersion: requirement.maxVersion.clone(),
                installedVersion: None,
                enabled: false,
            });
        };
        let versionCompatible = requirement.minVersion.as_ref().map_or(true, |minimum| {
            operit_util::GithubReleaseUtil::GithubReleaseUtil::compareVersions(
                &dependency.version,
                minimum,
            )
            .map(|order| order >= 0)
            .unwrap_or(false)
        }) && requirement.maxVersion.as_ref().map_or(true, |maximum| {
            operit_util::GithubReleaseUtil::GithubReleaseUtil::compareVersions(
                &dependency.version,
                maximum,
            )
            .map(|order| order <= 0)
            .unwrap_or(false)
        });
        let enabled = enabledPackageNames.contains(&dependencyId);
        if !versionCompatible {
            return Some(ToolPkgDependencyIssue {
                id: dependencyId,
                code: "version_incompatible".to_string(),
                requiredMinVersion: requirement.minVersion.clone(),
                requiredMaxVersion: requirement.maxVersion.clone(),
                installedVersion: Some(dependency.version),
                enabled,
            });
        }
        if !enabled {
            return Some(ToolPkgDependencyIssue {
                id: dependencyId,
                code: "disabled".to_string(),
                requiredMinVersion: requirement.minVersion.clone(),
                requiredMaxVersion: requirement.maxVersion.clone(),
                installedVersion: Some(dependency.version),
                enabled,
            });
        }
        let packageOrder = order
            .get(packageName)
            .copied()
            .expect("registered ToolPkg container must have a persisted order");
        let dependencyOrder = order
            .get(&dependencyId)
            .copied()
            .expect("registered ToolPkg prerequisite must have a persisted order");
        if dependencyOrder > packageOrder {
            return Some(ToolPkgDependencyIssue {
                id: dependencyId,
                code: "load_order".to_string(),
                requiredMinVersion: requirement.minVersion.clone(),
                requiredMaxVersion: requirement.maxVersion.clone(),
                installedVersion: Some(dependency.version),
                enabled,
            });
        }
        None
    }

    #[allow(non_snake_case)]
    /// Reads state from each installed extension's exact owner.
    fn decodeDisabledPackageNamesFromPrefs(&self) -> Vec<String> {
        let mut names = Vec::new();
        for record in self.scopeRecords() {
            let values: Vec<String> =
                serde_json::from_value(record.settings["disabledNames"].clone())
                    .expect("Registered package names must have the declared schema");
            names.extend(values);
        }
        self.normalizeEnabledPackageNames(&names)
    }

    #[allow(non_snake_case)]
    /// Reads state from each installed extension's exact owner.
    fn decodeToolPkgSubpackageStatesFromPrefs(&self) -> BTreeMap<String, bool> {
        let mut states = BTreeMap::new();
        for record in self.scopeRecords() {
            let values: BTreeMap<String, bool> =
                serde_json::from_value(record.settings["subpackageStates"].clone())
                    .expect("Registered package flags must have the declared schema");
            states.extend(values);
        }
        states
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
    /// Writes subpackage flags with their parent container's ownership.
    pub(crate) fn saveToolPkgSubpackageStates(
        &self,
        states: &BTreeMap<String, bool>,
    ) -> Result<(), PreferencesDataStoreError> {
        for mut record in self.scopeRecords() {
            let members: Vec<String> = serde_json::from_value(record.settings["members"].clone())?;
            record.settings["subpackageStates"] = serde_json::to_value(
                states
                    .iter()
                    .filter(|(name, _)| members.iter().any(|member| member == *name))
                    .map(|(name, enabled)| (name.clone(), *enabled))
                    .collect::<BTreeMap<_, _>>(),
            )?;
            self.writeScopeSettings(&record.id, record.settings)
                .map_err(PreferencesDataStoreError::Message)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Writes independently owned package flags without replacing another scope's state.
    pub(crate) fn saveEnabledPackageNames(
        &self,
        names: &[String],
    ) -> Result<(), PreferencesDataStoreError> {
        let names = self.normalizeEnabledPackageNames(names);
        for mut record in self.scopeRecords() {
            let members: Vec<String> = serde_json::from_value(record.settings["members"].clone())?;
            record.settings["enabledNames"] = serde_json::to_value(
                names
                    .iter()
                    .filter(|name| members.iter().any(|member| member == *name))
                    .cloned()
                    .collect::<Vec<_>>(),
            )?;
            self.writeScopeSettings(&record.id, record.settings)
                .map_err(PreferencesDataStoreError::Message)?;
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Writes independently owned package flags without replacing another scope's state.
    fn saveDisabledPackageNames(&self, names: &[String]) -> Result<(), PreferencesDataStoreError> {
        let names = self.normalizeEnabledPackageNames(names);
        for mut record in self.scopeRecords() {
            let members: Vec<String> = serde_json::from_value(record.settings["members"].clone())?;
            record.settings["disabledNames"] = serde_json::to_value(
                names
                    .iter()
                    .filter(|name| members.iter().any(|member| member == *name))
                    .cloned()
                    .collect::<Vec<_>>(),
            )?;
            self.writeScopeSettings(&record.id, record.settings)
                .map_err(PreferencesDataStoreError::Message)?;
        }
        Ok(())
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

    /// Persists startup package-record reconciliation without publishing a Space mutation.
    #[allow(non_snake_case)]
    fn saveBundledExternalImportRecordsLocally(
        &self,
        records: &BTreeMap<String, BundledExternalImportRecord>,
    ) -> Result<(), String> {
        let updatedJson = serde_json::to_string(records).map_err(|error| error.to_string())?;
        self.dataStore
            .migrate(|preferences| {
                preferences.set(
                    &stringPreferencesKey(BUNDLED_EXTERNAL_IMPORTS_KEY),
                    updatedJson,
                );
                Ok::<(), operit_store::PreferencesDataStore::PreferencesDataStoreError>(())
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
    /// Resolves the exact installed source, including HJSON, TS and ToolPkg archives.
    fn findPackageFile(&mut self, packageName: &str) -> Option<PathBuf> {
        let record = ExtensionStore::default()
            .record("package", &self.normalizePackageName(packageName))
            .ok()?;
        if record.sourceName.is_empty() {
            return None;
        }
        let root = ExtensionStore::root("package", &record.scope).ok()?;
        Some(
            self.storePaths
                .runtime_storage_path(&root)
                .join(record.sourceName),
        )
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
        let fileSystemHost = self.context.fileSystemHost.as_ref()?;
        JsPackageLoader::load_from_file(fileSystemHost.as_ref(), &file.to_string_lossy()).ok()
    }

    /// Loads one JavaScript package and preserves the parser error for the UI.
    #[allow(non_snake_case)]
    fn loadPackageFromJsFileResult(&self, file: &Path) -> Result<ToolPackage, String> {
        let fileSystemHost =
            self.context.fileSystemHost.as_ref().ok_or_else(|| {
                "FileSystemHost is required for external package loading".to_string()
            })?;
        JsPackageLoader::load_from_file(fileSystemHost.as_ref(), &file.to_string_lossy())
            .map_err(|error| error.to_string())
    }

    /// Resolves installed archive scope from its exact host-backed package directory.
    #[allow(non_snake_case)]
    fn installedPackageConfigScope(&self, path: &Path) -> Result<ToolPkgConfigScope, String> {
        for scope in [ToolPkgConfigScope::Device, ToolPkgConfigScope::Space] {
            let root = ExtensionStore::root("package", scope.as_str())?;
            let directory = self.storePaths.runtime_storage_path(&root);
            if path.parent() == Some(directory.as_path()) {
                return Ok(scope);
            }
        }
        Err(format!("Package source is outside installed scope directories: {}", hostPath(path)))
    }

    /// Loads one external ToolPkg and collects partial registration diagnostics.
    #[allow(non_snake_case)]
    fn loadToolPkgFromExternalFileWithIssues(
        &self,
        file: &Path,
        configScope: ToolPkgConfigScope,
    ) -> Result<(ToolPkgLoadResult, Vec<ToolPkgLoadIssue>), String> {
        let fileSystemHost =
            self.context.fileSystemHost.as_ref().ok_or_else(|| {
                "FileSystemHost is required for external ToolPkg loading".to_string()
            })?;
        let sourcePath = file.to_string_lossy().to_string();
        let issues = RefCell::new(Vec::new());
        let loadResult = self.withToolPkgRegistrationEngine(|registrationEngine| {
            ToolPkgLoader::loadToolPkgFromExternalFile(
                fileSystemHost.as_ref(),
                &sourcePath,
                configScope,
                registrationEngine,
                |packageName, error| {
                    issues.borrow_mut().push(newToolPkgLoadIssue(
                        sourcePath.clone(),
                        nonEmptyString(&packageName),
                        packageName.to_string(),
                        "toolpkg_registration",
                        error.to_string(),
                        "toolpkg",
                    ));
                },
            )
        })?;
        Ok((loadResult, issues.into_inner()))
    }

    /// Loads one market ToolPkg and collects partial registration diagnostics.
    #[allow(non_snake_case)]
    fn loadToolPkgFromMarketFileWithIssues(
        &self,
        file: &Path,
        configScope: ToolPkgConfigScope,
    ) -> Result<(ToolPkgLoadResult, Vec<ToolPkgLoadIssue>), String> {
        if !self.isInstalledMarketToolPkg(file)? {
            return Err("ToolPkg market installation authentication failed".to_string());
        }
        let fileSystemHost =
            self.context.fileSystemHost.as_ref().ok_or_else(|| {
                "FileSystemHost is required for market ToolPkg loading".to_string()
            })?;
        let sourcePath = file.to_string_lossy().to_string();
        let issues = RefCell::new(Vec::new());
        let loadResult = self.withToolPkgRegistrationEngine(|registrationEngine| {
            ToolPkgLoader::loadToolPkgFromMarketFile(
                fileSystemHost.as_ref(),
                &sourcePath,
                configScope,
                registrationEngine,
                |packageName, error| {
                    issues.borrow_mut().push(newToolPkgLoadIssue(
                        sourcePath.clone(),
                        nonEmptyString(&packageName),
                        packageName.to_string(),
                        "toolpkg_registration",
                        error.to_string(),
                        "market_toolpkg",
                    ));
                },
            )
        })?;
        Ok((loadResult, issues.into_inner()))
    }

    /// Loads one built-in ToolPkg and collects partial registration diagnostics.
    #[allow(non_snake_case)]
    fn loadToolPkgFromBuiltInAssetWithIssues(
        &self,
        assetName: &str,
        bytes: &'static [u8],
    ) -> Result<(ToolPkgLoadResult, Vec<ToolPkgLoadIssue>), String> {
        let issues = RefCell::new(Vec::new());
        let loadResult = self.withToolPkgRegistrationEngine(|registrationEngine| {
            ToolPkgLoader::loadToolPkgFromBuiltInAsset(
                assetName,
                bytes,
                registrationEngine,
                |packageName, error| {
                    issues.borrow_mut().push(newToolPkgLoadIssue(
                        assetName.to_string(),
                        nonEmptyString(&packageName),
                        packageName.to_string(),
                        "toolpkg_registration",
                        error.to_string(),
                        "toolpkg",
                    ));
                },
            )
        })?;
        Ok((loadResult, issues.into_inner()))
    }

    /// Loads one bundled external ToolPkg and collects partial registration diagnostics.
    #[allow(non_snake_case)]
    fn loadToolPkgFromBundledExternalAssetWithIssues(
        &self,
        assetName: &str,
        bytes: &'static [u8],
    ) -> Result<(ToolPkgLoadResult, Vec<ToolPkgLoadIssue>), String> {
        let issues = RefCell::new(Vec::new());
        let loadResult = self.withToolPkgRegistrationEngine(|registrationEngine| {
            ToolPkgLoader::loadToolPkgFromBundledExternalAsset(
                assetName,
                bytes,
                registrationEngine,
                |packageName, error| {
                    issues.borrow_mut().push(newToolPkgLoadIssue(
                        assetName.to_string(),
                        nonEmptyString(&packageName),
                        packageName.to_string(),
                        "toolpkg_registration",
                        error.to_string(),
                        "toolpkg",
                    ));
                },
            )
        })?;
        Ok((loadResult, issues.into_inner()))
    }

    /// Executes one ToolPkg load with a dedicated registration engine.
    #[allow(non_snake_case)]
    fn withToolPkgRegistrationEngine<T>(
        &self,
        operation: impl FnOnce(&dyn JsExecutionEngine) -> T,
    ) -> T {
        // Registration runs package-owned code; per-load ownership prevents one timed-out
        // package from retaining state or blocking registration of the next package.
        let registrationEngine = ToolPkgRegistrationEngineGuard {
            engine: self.toolPkgExecutionEngineFactory.createExecutionEngine(),
        };
        operation(registrationEngine.engine.as_ref())
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

    /// Returns the filesystem host owned by this package manager context.
    #[allow(non_snake_case)]
    fn fileSystemHost(&self) -> Arc<dyn FileSystemHost> {
        self.context
            .fileSystemHost
            .clone()
            .expect("RuntimePackageManager requires a FileSystemHost")
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
/// Formats the host-provided Unix clock for package prompt metadata.
fn currentUseTime() -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(currentTimeMillis())
        .expect("host time must be a valid chrono timestamp")
        .naive_utc()
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

/// Formats one structured external package import result for text callers.
#[allow(non_snake_case)]
fn formatExternalPackageImportResult(result: &ExternalPackageImportResult) -> String {
    let mut message = format!(
        "Successfully imported package: {}\nStored at: {}",
        result.packageName, result.storedPath
    );
    if let Some(sourceNotice) = &result.sourceNotice {
        message.push('\n');
        message.push_str("Source notice: ");
        message.push_str(sourceNotice);
    }
    message
}

#[allow(non_snake_case)]
fn sha256Hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Returns whether text is one normalized lowercase SHA-256 hexadecimal digest.
fn isSha256Hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Encodes one local market installation identifier for the existing package preferences store.
fn encodeMarketToolPkgInstallationId(
    installationId: &[u8; ToolPkgProtection::MARKET_INSTALLATION_ID_SIZE],
) -> String {
    installationId
        .iter()
        .map(|value| format!("{value:02x}"))
        .collect()
}

/// Decodes one persisted local market installation identifier without accepting malformed values.
fn decodeMarketToolPkgInstallationId(
    encoded: &str,
) -> Result<[u8; ToolPkgProtection::MARKET_INSTALLATION_ID_SIZE], String> {
    if encoded.len() != ToolPkgProtection::MARKET_INSTALLATION_ID_SIZE * 2 {
        return Err("ToolPkg market installation identifier is invalid".to_string());
    }
    let mut installationId = [0u8; ToolPkgProtection::MARKET_INSTALLATION_ID_SIZE];
    for (index, target) in installationId.iter_mut().enumerate() {
        let offset = index * 2;
        *target = u8::from_str_radix(&encoded[offset..offset + 2], 16)
            .map_err(|_| "ToolPkg market installation identifier is invalid".to_string())?;
    }
    Ok(installationId)
}

#[allow(non_snake_case)]
fn javaStringHashCodeHex(value: &str) -> String {
    let mut hash = 0i32;
    for unit in value.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(unit));
    }
    format!("{:x}", hash as u32)
}

/// Converts a package metadata path into the corresponding host path string.
fn hostPath(path: &Path) -> String {
    path.to_string_lossy().to_string()
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

/// Builds conditional package capabilities from the active Host environment.
#[allow(non_snake_case)]
fn buildConditionCapabilitiesSnapshot(
    hostEnvironment: &HostEnvironmentDescriptor,
) -> BTreeMap<String, ConditionValue> {
    let platformName = match &hostEnvironment.platform {
        HostPlatform::Android => "android",
        HostPlatform::Ohos => "ohos",
        HostPlatform::Windows => "windows",
        HostPlatform::Linux => "linux",
        HostPlatform::Macos => "macos",
        HostPlatform::Ios => "ios",
        HostPlatform::Web => "web",
        HostPlatform::Esp32 => "esp32",
        HostPlatform::Other => "other",
    };
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
            "platform.ios".to_string(),
            ConditionValue::Bool(platformName == "ios"),
        ),
        (
            "platform.web".to_string(),
            ConditionValue::Bool(platformName == "web"),
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

fn isProgressTrackedPackageSource(sourcePath: &str) -> bool {
    let Some(extension) = Path::new(sourcePath)
        .extension()
        .and_then(|value| value.to_str())
    else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "js" | "ts" | "hjson" | "json" | "toolpkg"
    )
}

fn packageProgressDisplayName(sourcePath: &str) -> String {
    Path::new(sourcePath)
        .file_stem()
        .or_else(|| Path::new(sourcePath).file_name())
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| sourcePath.to_string())
}

/// Builds one structured issue with the canonical package-manager field names.
#[allow(non_snake_case)]
fn newToolPkgLoadIssue(
    sourcePath: String,
    packageName: Option<String>,
    displayName: String,
    code: &str,
    message: String,
    packageKind: &str,
) -> ToolPkgLoadIssue {
    ToolPkgLoadIssue {
        sourcePath,
        packageName,
        displayName: if displayName.trim().is_empty() {
            "插件加载失败".to_string()
        } else {
            displayName
        },
        code: code.to_string(),
        message,
        packageKind: packageKind.to_string(),
    }
}

/// Appends only issues that are not already present in a package-manager list.
#[allow(non_snake_case)]
fn appendUniqueToolPkgLoadIssues(target: &mut Vec<ToolPkgLoadIssue>, issues: &[ToolPkgLoadIssue]) {
    for issue in issues {
        if target.iter().any(|existing| {
            existing.sourcePath == issue.sourcePath
                && existing.packageName == issue.packageName
                && existing.code == issue.code
                && existing.message == issue.message
        }) {
            continue;
        }
        target.push(issue.clone());
    }
}

/// Converts a non-empty package identifier into an optional model field.
#[allow(non_snake_case)]
fn nonEmptyString(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn beginPackageLoadProgress(sourcePath: &str) {
    if !pluginLoadingSessionActive() {
        return;
    }
    ensurePluginLoadingItem(
        sourcePath,
        &packageProgressDisplayName(sourcePath),
        PLUGIN_LOAD_KIND_PACKAGE,
    );
    markPluginLoadingItemLoading(sourcePath, None);
}

fn reportPackageScanProgress(result: &PackageScanCandidateResult) {
    if !pluginLoadingSessionActive() {
        return;
    }
    let displayName = if let Some(package) = &result.toolPackage {
        let resolved = package.display_name.resolve(false);
        if resolved.trim().is_empty() {
            package.name.clone()
        } else {
            resolved
        }
    } else if let Some(loadResult) = &result.toolPkgLoadResult {
        let resolved = loadResult.containerRuntime.displayName.resolve(false);
        if resolved.trim().is_empty() {
            loadResult.containerRuntime.packageName.clone()
        } else {
            resolved
        }
    } else {
        packageProgressDisplayName(&result.sourcePath)
    };
    if result.issues.is_empty()
        && (result.toolPackage.is_some() || result.toolPkgLoadResult.is_some())
    {
        markPluginLoadingItemSuccess(&result.sourcePath, Some(&displayName));
    } else {
        if let Some(issue) = result.issues.first() {
            markPluginLoadingItemFailed(&result.sourcePath, "failed", &issue.message);
        } else {
            markPluginLoadingItemFailed(&result.sourcePath, "failed", "");
        }
    }
}



/// Selects one published method exactly once and rejects absent or duplicate declarations.
#[allow(non_snake_case)]
fn selectToolPkgPublicApi<'a>(
    runtime: &'a ToolPkgContainerRuntime,
    methodName: &str,
) -> Result<&'a operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredFunctionHook, String> {
    let methodName = methodName.trim();
    if methodName.is_empty() {
        return Err("Public API method name is required".to_string());
    }
    let declarations = runtime.publicApis.iter()
        .filter(|declaration| declaration.id == methodName).collect::<Vec<_>>();
    match declarations.as_slice() {
        [] => Err(format!("Public API not found: {}/{methodName}", runtime.packageName)),
        [declaration] => {
            if declaration.function.trim().is_empty() {
                return Err(format!("Public API handler is empty: {}/{methodName}", runtime.packageName));
            }
            Ok(*declaration)
        }
        _ => Err(format!("Duplicate public API: {}/{methodName}", runtime.packageName)),
    }
}

#[cfg(test)]
mod public_api_selection_tests {
    use super::*;
    use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredFunctionHook;

    /// Creates a public-method declaration with an explicit exported handler.
    fn declaration(name: &str, function: &str) -> ToolPkgRegisteredFunctionHook {
        ToolPkgRegisteredFunctionHook {
            id: name.to_string(), function: function.to_string(), functionSource: None,
        }
    }

    /// Selects arbitrary published names without applying a domain or command-root whitelist.
    #[test]
    fn selects_exact_registered_export() {
        let runtime = ToolPkgContainerRuntime {
            packageName: "demo".to_string(), publicApis: vec![declaration("configuration.read", "readConfiguration")],
            ..Default::default()
        };
        assert_eq!(selectToolPkgPublicApi(&runtime, "configuration.read").unwrap().function, "readConfiguration");
        assert!(selectToolPkgPublicApi(&runtime, "Configuration.read").is_err());
        assert!(selectToolPkgPublicApi(&runtime, "missing").is_err());
        assert!(selectToolPkgPublicApi(&runtime, " ").is_err());
    }

    /// Rejects duplicate publications and empty handlers before entering a JavaScript runtime.
    #[test]
    fn rejects_duplicate_and_empty_exports() {
        let mut runtime = ToolPkgContainerRuntime {
            packageName: "demo".to_string(), publicApis: vec![declaration("read", "first"), declaration("read", "second")],
            ..Default::default()
        };
        assert_eq!(selectToolPkgPublicApi(&runtime, "read").unwrap_err(), "Duplicate public API: demo/read");
        runtime.publicApis = vec![declaration("read", " ")];
        assert_eq!(selectToolPkgPublicApi(&runtime, "read").unwrap_err(), "Public API handler is empty: demo/read");
    }
}
