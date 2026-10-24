#[path = "PackageRegistryReadiness.rs"]
mod package_registry_readiness;
pub use package_registry_readiness::{PackageRegistryLoadState, PackageRegistryReadiness};

#[path = "ToolPkgCoreCommandCatalog.rs"]
mod core_command_catalog;
pub use core_command_catalog::{
    ResolvedToolPkgCoreCommand, ToolPkgCoreCommandCatalog, ToolPkgCoreCommandRegistrationInfo,
};

#[path = "ToolPkgTextResourceCache.rs"]
mod text_resource_cache;
use text_resource_cache::ToolPkgTextResourceCache;
#[path = "ToolPkgResourceReader.rs"]
mod resource_reader;

use crate::tools::packTool::ToolPkgPublicApiUiCatalog::{
    discover_public_api_owners, ToolPkgPublicApiOwner,
};

use crate::tools::packTool::ToolPkgComposeDslSession::ToolPkgComposeDslSession;
use crate::tools::packTool::ToolPkgDesktopWidgetService::ToolPkgDesktopWidgetSnapshot;
use operit_store::ExtensionStore::{ExtensionRecord, ExtensionStore};
use operit_store::PreferencesDataStore::CoreNodeStateStore;
use operit_store::RuntimeStorageHost::defaultRuntimeStorageHost;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
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
    markPluginLoadingItemSuccess, pluginLoadingSessionActive, setPluginLoadingItemKind,
    PLUGIN_LOAD_KIND_PACKAGE, PLUGIN_LOAD_KIND_PLUGIN,
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
    fn createExecutionEngine(&self) -> Result<Arc<dyn JsExecutionEngine>, String> {
        self.jsExecutionProvider
            .create_execution_engine(Arc::new(self.toolHandler.clone()))
    }

    /// Creates one ToolPkg engine bound to the current host context.
    #[allow(non_snake_case)]
    fn createToolPkgExecutionEngine(
        &self,
        context: ToolPkgExecutionContext,
    ) -> Result<Arc<dyn JsExecutionEngine>, String> {
        self.jsExecutionProvider
            .create_toolpkg_execution_engine(Arc::new(self.toolHandler.clone()), context)
    }

    /// Acquires a prepared page engine through the shared execution provider.
    #[allow(non_snake_case)]
    fn acquireToolPkgExecutionEngine(
        &self,
        context: ToolPkgExecutionContext,
    ) -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Arc<dyn JsExecutionEngine>, String>>
    {
        self.jsExecutionProvider
            .acquire_toolpkg_execution_engine(Arc::new(self.toolHandler.clone()), context)
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
    /// Manifest-declared installation restriction; absent when users may choose a scope.
    pub requiredInstallationScope: Option<String>,
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
    toolPkgTextResourceCache: Arc<ToolPkgTextResourceCache>,
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
            toolPkgTextResourceCache: Arc::new(ToolPkgTextResourceCache::default()),
            packageRegistryReadiness: PackageRegistryReadiness::new(),
            scopeState: Arc::new(Mutex::new(BTreeMap::new())),
            toolPkgExecutionEngineFactory,
            composeDslSessions: Arc::new(Mutex::new(BTreeMap::new())),
            dataStore: CoreNodeStateStore::newWithStorage(
                defaultRuntimeStorageHost(),
                "runtime/extensions/device/manager.preferences.json",
            )
            .withSchema(3, |version, preferences| match version {
                0 | 1 => Ok(()),
                2 => {
                    preferences.remove(&stringPreferencesKey("bundled_external_imports"));
                    Ok(())
                }
                from => Err(PreferencesDataStoreError::MissingMigration { from, to: from + 1 }),
            }),
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
    #[operit_route_macros::operit_core_internal]
    pub fn packageRegistryReadiness(&self) -> PackageRegistryReadiness {
        self.packageRegistryReadiness.clone()
    }

    /// Waits outside the manager lock and clones only a completed canonical registration snapshot.
    #[allow(non_snake_case)]
    pub async fn readySnapshot(manager: Arc<Mutex<Self>>) -> Result<Self, String> {
        Self::withReady(manager, |manager| Ok(manager.clone())).await
    }

    /// Captures only request-owned state after readiness without holding the owner across await.
    pub(crate) async fn withReady<T>(
        manager: Arc<Mutex<Self>>,
        capture: impl FnOnce(&Self) -> Result<T, String>,
    ) -> Result<T, String> {
        Self::withReadyObserved(manager, capture, |_| Ok(())).await
    }

    /// Observes readiness and lock boundaries without holding the registry across an await.
    #[allow(non_snake_case)]
    pub(crate) async fn withReadyObserved<T>(
        manager: Arc<Mutex<Self>>,
        capture: impl FnOnce(&Self) -> Result<T, String>,
        mut observe: impl FnMut(&'static str) -> Result<(), String>,
    ) -> Result<T, String> {
        let readiness = {
            let manager = manager.lock().map_err(|error| error.to_string())?;
            observe("registryProbeLockMs")?;
            manager.packageRegistryReadiness()
        };
        observe("registryProbeMs")?;
        readiness.wait_until_ready().await?;
        observe("registryReadyMs")?;
        let manager = manager.lock().map_err(|error| error.to_string())?;
        observe("registryOwnerLockMs")?;
        manager.packageRegistryReadiness.require_ready()?;
        let captured = capture(&manager)?;
        drop(manager);
        observe("registryCaptureMs")?;
        Ok(captured)
    }

    /// Finds exact registerApi owners from the ready catalog without invoking their handlers or building domain UI inputs.
    #[allow(non_snake_case)]
    pub fn getToolPkgPublicApiOwners(
        &self,
        apiName: &str,
    ) -> Result<Vec<ToolPkgPublicApiOwner>, String> {
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
                return Err(format!(
                    "Duplicate public API: {}/{}",
                    runtime.packageName, declaration.id
                ));
            }
        }
        Ok(names.into_iter().collect())
    }

    /// Calls one registerApi export in its owning main runtime without dispatching an AI tool.
    #[allow(non_snake_case)]
    pub fn invokeToolPkgPublicApi(
        &self,
        packageName: &str,
        methodName: &str,
        payload: serde_json::Value,
    ) -> impl Future<Output = Result<serde_json::Value, String>> + 'static {
        let startedAt = currentTimeMillis();
        let manager = self.toolPkgManager().clone();
        let reader = self.resourceReader();
        let apiName = format!("{packageName}/{methodName}");
        let prepared = (|| {
            let runtime = self.requireToolPkgPublicApiRuntime(packageName)?;
            let declaration = selectToolPkgPublicApi(&runtime, methodName)?.clone();
            Ok::<_, String>((runtime, declaration))
        })();
        async move {
            let (runtime, declaration) = prepared?;
            let contextKey = format!("toolpkg_main:{}", runtime.packageName);
            let (engine, _lease) = manager.leaseToolPkgMainExecutionEngine(&runtime.packageName)?;
            let script = reader.mainScript(&runtime)?;
            if !manager
                .findToolPkgExecutionEngine(&contextKey, &runtime.packageName)
                .is_some_and(|current| Arc::ptr_eq(&current, &engine))
            {
                return Err("ToolPkg execution context has been released or replaced".to_string());
            }
            let event = operit_plugin_sdk::js_sdk::toolpkg::ToolPkgPublicApiEvent {
                payload,
                callerPackage: "host".to_string(),
            };
            let serde_json::Value::Object(event) =
                serde_json::to_value(event).map_err(|error| error.to_string())?
            else {
                return Err("ToolPkg public API event must serialize to an object".to_string());
            };
            let mut params = event.into_iter().collect::<BTreeMap<_, _>>();
            params.insert(
                "__operit_ui_package_name".to_string(),
                serde_json::Value::String(runtime.packageName.clone()),
            );
            params.insert(
                "toolPkgId".to_string(),
                serde_json::Value::String(runtime.packageName.clone()),
            );
            params.insert(
                "containerPackageName".to_string(),
                serde_json::Value::String(runtime.packageName.clone()),
            );
            params.insert(
                "__operit_execution_context_key".to_string(),
                serde_json::Value::String(contextKey),
            );
            params.insert(
                "__operit_toolpkg_runtime_kind".to_string(),
                serde_json::Value::String("main".to_string()),
            );
            params.insert(
                "__operit_script_screen".to_string(),
                serde_json::Value::String(runtime.mainEntry),
            );
            params.insert(
                "__operit_toolpkg_api_version".to_string(),
                serde_json::Value::String(runtime.apiVersion),
            );
            if let Some(source) = declaration.functionSource {
                params.insert(
                    "__operit_inline_function_name".to_string(),
                    serde_json::Value::String(declaration.function.clone()),
                );
                params.insert(
                    "__operit_inline_function_source".to_string(),
                    serde_json::Value::String(source),
                );
            }
            let executeAt = currentTimeMillis();
            let result = engine
                .execute_script_function_async(
                    script,
                    declaration.function.clone(),
                    params,
                    BTreeMap::new(),
                    None,
                    true,
                )
                .await;
            let completedAt = currentTimeMillis();
            AppLogger::v_with_level(
                "ToolPkgTiming",
                &format!(
                    "kind=public_api package={} function={} api={} prepareMs={} executeMs={} totalMs={} success={}",
                    runtime.packageName, declaration.function, apiName,
                    executeAt.saturating_sub(startedAt), completedAt.saturating_sub(executeAt),
                    completedAt.saturating_sub(startedAt), result.is_ok(),
                ),
                operit_util::AppLogger::VERBOSE_LEVEL_3,
            );
            let raw = result
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("Public API returned no JSON result: {apiName}"))?;
            serde_json::from_str(&raw).map_err(|error| error.to_string())
        }
    }

    /// Requires a real enabled public API owner after the canonical registry has finished loading.
    #[allow(non_snake_case)]
    fn requireToolPkgPublicApiRuntime(
        &self,
        packageName: &str,
    ) -> Result<ToolPkgContainerRuntime, String> {
        self.packageRegistryReadiness.require_ready()?;
        let packageName = packageName.trim();
        if packageName.is_empty() {
            return Err("Public API package name is required".to_string());
        }
        let runtime = self
            .getToolPkgContainerRuntime(packageName)
            .ok_or_else(|| format!("ToolPkg container not found: {packageName}"))?;
        if !self
            .toolPkgManager()
            .isEnabledToolPkgContainer(&runtime.packageName, &self.getEnabledPackageNames())
        {
            return Err(format!(
                "ToolPkg public API owner is disabled: {packageName}"
            ));
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
        let mut sessions = self
            .composeDslSessions
            .lock()
            .expect("Compose session registry mutex poisoned");
        if let Some(session) = sessions.remove(&key) {
            session.close();
        }
        self.toolPkgManager()
            .releaseToolPkgExecutionEngine(contextKey, containerPackageName);
    }

    #[allow(non_snake_case)]
    /// Acquires a lease outside the proxy's package-manager lock. Worker startup
    /// authenticates against that same manager, so synchronous proxy dispatch
    /// would deadlock while waiting for the worker to finish initialization.
    pub fn acquireToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> impl Future<Output = Result<(), String>> + 'static {
        let manager = self.toolPkgManager().clone();
        let contextKey = contextKey.to_string();
        let containerPackageName = containerPackageName.to_string();
        async move {
            manager
                .acquireToolPkgExecutionEngineAsync(&contextKey, &containerPackageName)
                .await
        }
    }

    #[allow(non_snake_case)]
    /// Returns the ToolPkg execution engine for one explicitly owned context.
    pub fn getToolPkgExecutionEngine(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> Result<Arc<dyn JsExecutionEngine>, String> {
        self.toolPkgManager()
            .getToolPkgExecutionEngine(contextKey, containerPackageName)
    }

    /// Captures session state and loads resources after releasing the package manager lock.
    pub fn openComposeDslSession(
        &self,
        contextKey: &str,
        containerPackageName: &str,
    ) -> impl Future<Output = Result<String, String>> + 'static {
        let key = (contextKey.to_string(), containerPackageName.to_string());
        let sessions = self.composeDslSessions.clone();
        let engines = self.toolPkgManager().clone();
        let resources = self.toolPkgTextResources(containerPackageName);
        let engine = engines.findToolPkgExecutionEngine(&key.0, &key.1);
        async move {
            let engine = engine
                .ok_or("Compose execution context has not been acquired or has been released")?;
            let require_owner = || {
                if engines
                    .findToolPkgExecutionEngine(&key.0, &key.1)
                    .is_some_and(|current| Arc::ptr_eq(&current, &engine))
                {
                    Ok(())
                } else {
                    Err("Compose execution context has been released or replaced".to_string())
                }
            };
            {
                let sessions = sessions
                    .lock()
                    .expect("Compose session registry mutex poisoned");
                require_owner()?;
                if let Some(session) = sessions.get(&key) {
                    if !session.isClosed() {
                        return Ok(session.id());
                    }
                }
            }
            let resources = resources.await?;
            let mut sessions = sessions
                .lock()
                .expect("Compose session registry mutex poisoned");
            // Recheck ownership after I/O while serialized with context release.
            require_owner()?;
            openComposeDslSessionForContext(&mut sessions, key, || {
                Ok(ToolPkgComposeDslSession::new(engine, resources))
            })
        }
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
    pub(crate) fn toolPkgManager(&self) -> &ToolPkgManager {
        self.pluginPackageManager.toolPkgManager()
    }

    /// Captures teardown independently of the package-manager lock: destroying a
    /// worker can complete host calls which consult that same manager.
    #[allow(non_snake_case)]
    pub fn executionEngineShutdown(&self) -> impl Fn() + Send + Sync + 'static {
        let manager = self.toolPkgManager().clone();
        move || manager.shutdownExecutionEngines()
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

    /// Returns the stable local identifier that binds installed market ToolPkg seals to this client.
    #[allow(non_snake_case)]
    fn marketToolPkgInstallationId(
        &self,
    ) -> Result<[u8; ToolPkgProtection::MARKET_INSTALLATION_ID_SIZE], String> {
        let key = stringPreferencesKey(MARKET_TOOLPKG_INSTALLATION_ID_KEY);
        self.dataStore
            .try_edit_result(|preferences| {
                let existing = preferences.getTyped::<String>(&key)?;
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

    /// Captures only the capabilities and shared caches needed for resource I/O.
    fn resourceReader(&self) -> resource_reader::ToolPkgResourceReader {
        resource_reader::ToolPkgResourceReader {
            storePaths: self.storePaths.clone(),
            fileSystemHost: self.fileSystemHost.clone(),
            runtimeSupport: self.toolHandler.runtimeSupport(),
            toolPkgCacheLock: self.toolPkgCacheLock.clone(),
            toolPkgTextResourceCache: self.toolPkgTextResourceCache.clone(),
        }
    }

    /// Invalidates the shared resources for one package.
    fn deleteToolPkgCacheDir(&self, packageName: &str) {
        self.resourceReader().deleteToolPkgCacheDir(packageName);
    }

    /// Ensures resources exist for synchronous package installation operations.
    fn ensureToolPkgCache(&self, runtime: &ToolPkgContainerRuntime) -> Option<PathBuf> {
        self.resourceReader().ensureToolPkgCache(runtime)
    }

    /// Captures the package revision before loading text resources outside the manager lock.
    pub fn toolPkgTextResources(
        &self,
        containerPackageName: &str,
    ) -> impl Future<Output = Result<Arc<BTreeMap<String, String>>, String>> + 'static {
        let packageName = self.normalizePackageName(containerPackageName);
        let runtime = self
            .toolPkgManager()
            .getToolPkgContainerRuntime(&packageName);
        let reader = self.resourceReader();
        async move {
            let runtime = runtime.ok_or_else(|| {
                format!("ToolPkg container package is not registered: {packageName}")
            })?;
            reader.toolPkgTextResources(&runtime)
        }
    }

    /// Resolves a resource for synchronous export operations.
    pub(crate) fn resolveToolPkgResourceFile(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<PathBuf> {
        self.resourceReader()
            .resolveToolPkgResourceFile(runtime, resourcePath)
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
    /// Projects enabled packages from container activation and each child's declared state.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getEnabledPackageNames(&self) -> Vec<String> {
        let availablePackages = self.availablePackages();
        let subpackages = self.toolPkgSubpackageByPackageNameInternal();
        let mut enabledPackageNames = self
            .decodeEnabledPackageNamesFromPrefs()
            .into_iter()
            .filter(|name| availablePackages.contains_key(name) && !subpackages.contains_key(name))
            .collect::<BTreeSet<_>>();
        let disabledPackageNames = BTreeSet::from_iter(self.decodeDisabledPackageNamesFromPrefs());
        for package in availablePackages.values().filter(|package| !subpackages.contains_key(&package.name)) {
            if package.is_built_in && package.enabled_by_default && !disabledPackageNames.contains(&package.name) {
                enabledPackageNames.insert(package.name.clone());
            }
        }
        let states = self.getToolPkgSubpackageStatesInternal();
        for child in subpackages.values() {
            if enabledPackageNames.contains(&child.containerPackageName) && states[&child.packageName] {
                enabledPackageNames.insert(child.packageName.clone());
            }
        }
        enabledPackageNames.into_iter().collect()
    }

    #[allow(non_snake_case)]
    pub(crate) fn getEnabledPackageNameSetInternal(&self) -> BTreeSet<String> {
        BTreeSet::from_iter(self.getEnabledPackageNames())
    }

    /// Applies persisted child preferences to their manifest-declared initial states.
    #[allow(non_snake_case)]
    pub(crate) fn getToolPkgSubpackageStatesInternal(&self) -> BTreeMap<String, bool> {
        let mut states: BTreeMap<_, _> = self.toolPkgSubpackageByPackageNameInternal()
            .into_iter().map(|(name, child)| (name, child.enabledByDefault)).collect();
        states.extend(self.normalizeToolPkgSubpackageStates(&self.decodeToolPkgSubpackageStatesFromPrefs()));
        states
    }

    #[allow(non_snake_case)]
    /// Returns the package's effective state from the shared enabled-package projection.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn isPackageEnabled(&self, packageName: &str) -> bool {
        self.getEnabledPackageNameSetInternal().contains(&self.normalizePackageName(packageName))
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
            let names = enabledPackageNames.into_iter().collect::<Vec<_>>();
            if let Err(error) = self.saveEnabledPackageNames(&names) {
                return format!(
                    "Failed to enable ToolPkg container '{}': {}",
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
            if let Err(error) = self.addToDisabledIfDefaultEnabled(&normalizedPackageName) {
                return format!(
                    "Failed to disable ToolPkg container '{}': {}",
                    normalizedPackageName, error
                );
            }
            self.deleteToolPkgCacheDir(&normalizedPackageName);
            self.toolPkgManager()
                .revokeContainerExecutionContexts(&normalizedPackageName);
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
    /// Removes custom code from built-in identities or deletes the selected package installation.
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
        if record.settings["customized"] == true {
            if let Err(error) = ExtensionStore::default().removePackageCustomization(&name) {
                self.recordManualToolPkgLoadIssue(&name, Some(&name), "delete_failed", &error, "package");
                return false;
            }
            self.loadAvailablePackages();
            return true;
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
        self.loadAvailablePackages();
        true
    }

    /// Deletes the persisted source of a current load failure without requiring a loaded package.
    #[allow(non_snake_case)]
    pub fn deleteToolPkgLoadIssue(&mut self, issue: ToolPkgLoadIssue) -> Result<(), String> {
        let issue = self
            .getToolPkgLoadIssues()
            .into_iter()
            .find(|current| {
                current.sourcePath == issue.sourcePath
                    && current.packageName == issue.packageName
                    && current.code == issue.code
                    && current.packageKind == issue.packageKind
            })
            .ok_or("Package load issue is no longer present; refresh the package list")?;

        let source = Path::new(&issue.sourcePath);
        let installedScope = ["device", "space"].into_iter().find(|scope| {
            let root = ExtensionStore::root("package", scope).expect("valid package scope");
            source.parent() == Some(self.storePaths.runtime_storage_path(&root).as_path())
        });
        // Even an unreadable archive can still have a registered installation owner.
        let owner = if issue.packageName.is_none() {
            ExtensionStore::default()
                .records("package")?
                .into_iter()
                .find(|record| {
                    Some(record.scope.as_str()) == installedScope
                        && source.file_name().and_then(|name| name.to_str())
                            == Some(record.sourceName.as_str())
                })
                .map(|record| record.id)
        } else {
            issue.packageName.clone()
        };
        let sourceOnly = issue.code == "duplicate_package"
            && owner
                .as_ref()
                .is_some_and(|name| self.availablePackages().contains_key(name));
        let mut removed = false;
        if let Some(name) = owner.as_deref().filter(|_| !sourceOnly) {
            self.disablePackage(name);
            removed = ExtensionStore::default().deleteFailedPackageInstallations(name)?;
        }
        // Parse failures may have no identity or installation record. Only remove a file
        // directly inside an installation directory, never an asset or a scan directory.
        if installedScope.is_some() {
            let info = self
                .fileSystemHost
                .fileExists(&issue.sourcePath)
                .map_err(|error| error.to_string())?;
            if info.exists && !info.isDirectory {
                self.fileSystemHost
                    .deleteFile(&issue.sourcePath, false)
                    .map_err(|error| error.to_string())?;
                removed = true;
            }
        }
        if !removed {
            return Err("No installed package source was found for this load issue".into());
        }
        self.clearManualToolPkgLoadIssues(&issue.sourcePath, issue.packageName.as_deref());
        self.loadAvailablePackages();
        operit_store::ExtensionStore::notifyCatalogChanged();
        Ok(())
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

    /// Requires one registered active container without copying or ordering the active catalog.
    #[allow(non_snake_case)]
    pub(crate) fn requireEnabledToolPkgContainer(&self, packageName: &str) -> Result<(), String> {
        self.packageRegistryReadiness.require_ready()?;
        let manager = self.toolPkgManager();
        if !manager.isToolPkgContainer(packageName) {
            return Err(format!(
                "Plugin execution owner is not registered: {packageName}"
            ));
        }
        if !manager.isEnabledToolPkgContainer(packageName, &self.getEnabledPackageNames()) {
            return Err(format!("Plugin execution owner is disabled: {packageName}"));
        }
        Ok(())
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
    /// Orders Space containers before device containers, preserving user order within each scope.
    fn orderToolPkgRuntimes(
        &self,
        mut runtimes: Vec<ToolPkgContainerRuntime>,
    ) -> Vec<ToolPkgContainerRuntime> {
        let ranks = self.decodeToolPkgContainerOrderFromPrefs().into_iter()
            .enumerate().map(|(rank, name)| (name, rank)).collect::<BTreeMap<_, _>>();
        runtimes.sort_by_key(|runtime| *ranks.get(&runtime.packageName)
            .expect("Registered ToolPkg container must have an owner rank"));
        runtimes
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
        let runtimes = self.orderToolPkgRuntimes(self.toolPkgManager().getToolPkgContainerRuntimes());
        runtimes
            .into_iter()
            .map(|runtime| self.withToolPkgDependencyIssues(runtime, &enabledPackageNames, &order))
            .collect()
    }

    #[allow(non_snake_case)]
    /// Returns the same scope and user order used for plugin execution.
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn getToolPkgContainerOrder(&self) -> Vec<String> {
        self.orderToolPkgRuntimes(self.toolPkgManager().getToolPkgContainerRuntimes())
            .into_iter().map(|runtime| runtime.packageName).collect()
    }

    #[allow(non_snake_case)]
    /// Persists independent user ranks for Space and device containers.
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
            let mut ranks = BTreeMap::from([("space", 0usize), ("device", 0usize)]);
            for name in &ordered {
                let mut record = ExtensionStore::default()
                    .record("package", name)
                    .map_err(PreferencesDataStoreError::Message)?;
                let rank = ranks.get_mut(record.scope.as_str())
                    .expect("Registered package must have a valid scope");
                record.settings["order"] = serde_json::json!(*rank);
                *rank += 1;
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
    pub fn invokeToolPkgChatMessageMenuItem(
        &self,
        containerPackageName: &str,
        itemId: &str,
        chatId: &str,
        messageIndex: i32,
        message: serde_json::Value,
    ) -> impl Future<Output = Result<Option<String>, String>> + 'static {
        let manager = self.toolPkgManager().clone();
        let enabled = self.getEnabledPackageNames();
        let prepared = (|| {
            let packageName = self.normalizePackageName(containerPackageName);
            let runtime = self
                .getEnabledToolPkgContainerRuntimes()
                .into_iter()
                .find(|runtime| runtime.packageName == packageName)
                .ok_or_else(|| format!("Enabled ToolPkg container not found: {packageName}"))?;
            let item = runtime
                .chatMessageMenuItems
                .iter()
                .find(|item| item.id == itemId.trim())
                .ok_or_else(|| {
                    format!(
                        "ToolPkg chat message menu item not found: {}:{}",
                        runtime.packageName,
                        itemId.trim()
                    )
                })?;
            Ok::<_, String>(ToolPkgHookInvocation {
                containerPackageName: runtime.packageName.clone(),
                functionName: item.function.clone(),
                event: operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CHAT_MESSAGE_MENU_ITEM.to_string(),
                eventName: Some("chat_message_menu_item_click".into()),
                pluginId: Some(item.id.clone()), inlineFunctionSource: item.functionSource.clone(),
                eventPayload: serde_json::json!({"action": "click", "chatId": chatId,
                    "messageIndex": messageIndex, "menuItemId": item.id, "message": message}),
                executionContextKey: None, runtimeKind: None, envOverrides: BTreeMap::new(),
                timestampMs: currentTimeMillis(), timeoutMillis: None,
                dispatchIntermediateOnMain: true, onIntermediateResult: None,
            })
        })();
        async move {
            manager
                .dispatchToolPkgHook(&enabled, prepared?)
                .await
                .map_err(|error| error.to_string())
        }
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
    pub fn getToolPkgCoreCommandCatalog(
        &self,
        useEnglish: bool,
    ) -> Result<ToolPkgCoreCommandCatalog, String> {
        self.packageRegistryReadiness.require_ready()?;
        let enabled = self
            .getEnabledToolPkgContainerRuntimes()
            .into_iter()
            .map(|runtime| runtime.packageName)
            .collect::<BTreeSet<_>>();
        Ok(ToolPkgCoreCommandCatalog::new(
            self.toolPkgManager().getToolPkgContainerRuntimes(),
            enabled,
            useEnglish,
        ))
    }

    /// Resolves one plugin command from its directory and invokes only that selected declaration.
    #[allow(non_snake_case)]
    pub fn executeToolPkgCoreCommand(
        &self,
        commandName: &str,
        args: &[String],
        jsonMode: bool,
    ) -> impl Future<Output = Result<ToolPkgCoreCommandExecutionResult, String>> + 'static {
        let manager = self.toolHandler.getOrCreatePackageManager();
        let readiness = self.packageRegistryReadiness.clone();
        let commandName = commandName.to_string();
        let args = args.to_vec();
        async move {
            readiness.wait_until_ready().await?;
            let pending = {
                let manager = manager.lock().map_err(|error| error.to_string())?;
                let resolved = manager
                    .getToolPkgCoreCommandCatalog(false)?
                    .resolve(&commandName, &[])?
                    .ok_or_else(|| format!("plugin command not found: /{commandName}"))?;
                manager.prepareResolvedToolPkgCoreCommand(&resolved, &args, jsonMode)
            };
            pending.await
        }
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
        self.prepareResolvedToolPkgCoreCommand(resolved, args, jsonMode)
            .await
    }

    /// Captures one resolved command after registry readiness has been established.
    #[allow(non_snake_case)]
    fn prepareResolvedToolPkgCoreCommand(
        &self,
        resolved: &ResolvedToolPkgCoreCommand,
        args: &[String],
        jsonMode: bool,
    ) -> impl Future<Output = Result<ToolPkgCoreCommandExecutionResult, String>> + 'static {
        let manager = self.toolPkgManager().clone();
        let enabled = self.getEnabledPackageNames();
        let info = resolved.info().clone();
        let command = resolved.declaration();
        let invocation = ToolPkgHookInvocation {
            containerPackageName: info.containerPackageName.clone(),
            functionName: command.function.clone(),
            event:
                operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_CORE_COMMAND
                    .to_string(),
            eventName: Some("core_command".into()),
            pluginId: Some(command.id.clone()),
            inlineFunctionSource: command.functionSource.clone(),
            eventPayload: serde_json::json!({"commandId": command.id, "commandName": command.name,
                "args": args, "json": jsonMode}),
            executionContextKey: None,
            runtimeKind: None,
            envOverrides: BTreeMap::new(),
            timestampMs: currentTimeMillis(),
            timeoutMillis: None,
            dispatchIntermediateOnMain: true,
            onIntermediateResult: None,
        };
        async move {
            if !manager
                .getEnabledToolPkgContainerRuntimes(&enabled)
                .iter()
                .any(|runtime| runtime.packageName == info.containerPackageName)
            {
                return Err(format!(
                    "plugin command is disabled: /{} ({})",
                    info.name, info.containerPackageName
                ));
            }
            let raw = manager
                .dispatchToolPkgHook(&enabled, invocation)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("plugin command /{} returned no result", info.name))?;
            serde_json::from_str::<ToolPkgCoreCommandExecutionResult>(&raw).map_err(|error| {
                format!(
                    "plugin command /{} returned an invalid result: {error}",
                    info.name
                )
            })
        }
    }

    /// Renders a registered desktop widget and settles its initial load action for any host.
    #[allow(non_snake_case)]
    pub fn renderToolPkgDesktopWidget(
        &self,
        containerPackageName: &str,
        widgetId: &str,
        instanceId: &str,
        useEnglish: bool,
    ) -> impl Future<Output = Result<ToolPkgDesktopWidgetSnapshot, String>> + 'static {
        super::ToolPkgDesktopWidgetService::render(
            self,
            containerPackageName,
            widgetId,
            instanceId,
            useEnglish,
        )
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
    pub fn runToolPkgNavigationEntryAction(
        &self,
        containerPackageName: &str,
        entryId: &str,
        functionName: &str,
        inlineFunctionSource: Option<&str>,
        eventPayload: serde_json::Value,
    ) -> impl Future<Output = Result<Option<String>, String>> + 'static {
        let actionContext =
            format!("package={containerPackageName}, entryId={entryId}, function={functionName}");
        let manager = self.toolPkgManager().clone();
        let enabled = self.getEnabledPackageNames();
        let invocation = ToolPkgHookInvocation {
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
                    timeoutMillis: None,
                    dispatchIntermediateOnMain: true,
                    onIntermediateResult: None,
                };
        async move {
            AppLogger::v_with_level(
                PACKAGE_MANAGER_LOG_TAG,
                &format!("Navigation action started: {actionContext}"),
                operit_util::AppLogger::VERBOSE_LEVEL_2,
            );
            let result = manager
                .dispatchToolPkgHookAsync(&enabled, invocation)
                .await
                .map_err(|error| error.to_string());
            match &result {
                Ok(_) => AppLogger::v_with_level(
                    PACKAGE_MANAGER_LOG_TAG,
                    &format!("Navigation action completed: {actionContext}"),
                    operit_util::AppLogger::VERBOSE_LEVEL_2,
                ),
                Err(error) => AppLogger::e(
                    PACKAGE_MANAGER_LOG_TAG,
                    &format!("Navigation action failed: {actionContext}, error={error}"),
                ),
            };
            result
        }
    }

    #[allow(non_snake_case)]
    /// Lists bundled packages available for installation or an explicit source update.
    pub fn getBundledExternalPackageCandidates(&mut self) -> Vec<BundledExternalPackageCandidate> {
        let scanned = self.scanBundledExternalPackageCandidates();
        let mut candidates = Vec::new();
        for candidate in scanned
            .into_iter()
            .filter_map(Self::bundledExternalPackageCandidateFromScanResult)
        {
            match self.bundledCandidateNeedsInstallation(&candidate) {
                Ok(true) => candidates.push(candidate),
                Ok(false) => {}
                Err(error) => {
                    self.recordAndReturnPackageError(
                        &candidate.sourcePath,
                        "bundled_update_check_failed",
                        &error,
                        &candidate.packageKind,
                    );
                }
            }
        }
        candidates.sort_by(|left, right| {
            left.displayName
                .resolve(false)
                .cmp(&right.displayName.resolve(false))
                .then_with(|| left.packageName.cmp(&right.packageName))
        });
        candidates
    }

    /// Compares the installed source with its bundled asset without replacing custom implementations.
    fn bundledCandidateNeedsInstallation(
        &self,
        candidate: &BundledExternalPackageCandidate,
    ) -> Result<bool, String> {
        let records = self.scopeState.lock().map_err(|error| error.to_string())?;
        let Some(record) = records.get(&candidate.packageName) else {
            return Ok(true);
        };
        if record.settings["builtin"] == true || record.settings["customized"] == true {
            return Ok(false);
        }
        if record.sourceName != candidate.sourceFileName {
            return Ok(false);
        }
        let root = ExtensionStore::root("package", &record.scope)?;
        let path = self
            .storePaths
            .runtime_storage_path(&root)
            .join(&record.sourceName);
        let installed = self
            .fileSystemHost
            .readFileBytes(&hostPath(&path))
            .map_err(|error| {
                format!(
                    "Cannot read installed package {}: {error}",
                    candidate.packageName
                )
            })?;
        let asset = bundledExternalPluginAssetByName(
            self.toolHandler.runtimeSupport().as_ref(),
            &candidate.sourcePath,
        )
        .ok_or_else(|| format!("Bundled package asset not found: {}", candidate.sourcePath))?;
        Ok(installed.as_slice() != asset.bytes)
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

    /// Updates an existing installation in its scope or installs into the manifest scope.
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
        let sourceFileName = packageSourceFileName(&sourcePath);
        let scope = match self.bundledInstallationScope(
            packageName,
            &sourceFileName,
            &result.toolPkgLoadResult,
        ) {
            Ok(scope) => scope,
            Err(error) => {
                return self.recordAndReturnPackageError(
                    &sourcePath,
                    "bundled_import_failed",
                    &error,
                    packageKind,
                )
            }
        };
        let packagesDir = self.storePaths.runtime_storage_path(
            &ExtensionStore::root("package", scope.as_str()).expect("Validated package scope"),
        );
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
        let destinationFile = packagesDir.join(&sourceFileName);
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
        // Publish the concrete source owner before scanning scope-owned package files.
        let registration = (|| -> Result<(), String> {
            let mut members = vec![packageName.to_string()];
            let requiredScope = if let Some(loaded) = &result.toolPkgLoadResult {
                members.extend(
                    loaded
                        .containerRuntime
                        .subpackages
                        .iter()
                        .map(|child| child.packageName.clone()),
                );
                Self::requiredInstallationScope(&loaded.containerRuntime)?
            } else {
                None
            };
            self.registerScope(
                packageName,
                &sourceFileName,
                false,
                members.clone(),
                scope,
                requiredScope,
            )?;
            ExtensionStore::default().installBundledPackageSource(
                packageName,
                &sourceFileName,
                members,
                scope.as_str(),
            )
        })();
        if let Err(error) = registration {
            return self.recordAndReturnPackageError(
                &sourcePath,
                "bundled_import_failed",
                &error,
                packageKind,
            );
        }
        self.externalPackageScanCache
            .remove(&destinationFile.to_string_lossy().to_string());
        self.loadAvailablePackages();
        format!(
            "Successfully imported package: {}\nStored at: {}",
            packageName,
            destinationFile.to_string_lossy()
        )
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
        let registration = (|| -> Result<(), String> {
            self.registerScope(
                &loadResult.containerPackage.name,
                &source,
                false,
                members,
                self.installedPackageConfigScope(Path::new(
                    &loadResult.containerRuntime.sourcePath,
                ))?,
                Self::requiredInstallationScope(&loadResult.containerRuntime)?,
            )
        })();
        if let Err(error) = registration {
            self.recordManualToolPkgLoadIssue(
                &loadResult.containerRuntime.sourcePath,
                Some(&loadResult.containerPackage.name),
                "scope_failed",
                &error,
                "toolpkg",
            );
            return false;
        }
        let packageName = loadResult.containerRuntime.packageName.clone();
        let registered = self.pluginPackageManager.registerToolPkg(loadResult);
        if !registered {
            return false;
        }
        self.toolPkgTextResourceCache.invalidate(&packageName);
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
        self.toolPkgTextResourceCache.retain_packages(
            &activeContainers
                .iter()
                .map(|runtime| runtime.packageName.clone())
                .collect(),
        );
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
    /// Scans package sources while preserving execution engines leased by active owners.
    pub fn loadAvailablePackages(&mut self) {
        self.packageRegistryReadiness.begin_scan();
        let assetSnapshot = self.scanBuiltInPackageAssets();
        let mergedSnapshot = self.scanExternalPackages(&assetSnapshot);
        self.applyPackageScanSnapshot(mergedSnapshot);
        self.notifyToolPkgRuntimeChangeListeners();
        self.packageRegistryReadiness.complete_scan();
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
    /// Selects bundled assets from their current installation owner.
    fn scanBuiltInPackageAssets(&self) -> PackageScanSnapshot {
        let mut results = Vec::new();
        for asset in self.toolHandler.runtimeSupport().builtinPluginAssets() {
            if isProgressTrackedPackageSource(asset.name) {
                beginPackageLoadProgress(asset.name);
            }
            let mut result = self.parseBuiltInPackageAsset(asset);
            let packageName = Self::packageNameFromScanResult(&result);
            let selected = match packageName.as_deref() {
                Some(name) => ExtensionStore::default().acceptsBuiltinPackage(name),
                None => Ok(true),
            };
            match selected {
                Ok(false) => continue,
                Ok(true) => {}
                Err(error) => {
                    result.toolPackage = None;
                    result.toolPkgLoadResult = None;
                    result.issues.push(newToolPkgLoadIssue(
                        asset.name.to_string(), packageName, packageProgressDisplayName(asset.name),
                        "scope_failed", error, "package",
                    ));
                }
            }
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
                    Some(
                        ExtensionStore::default()
                            .records("package")?
                            .into_iter()
                            .filter(|record| record.scope == "space")
                            .map(|record| record.sourceName)
                            .collect::<BTreeSet<_>>(),
                    )
                } else {
                    None
                };
                let entries = self
                    .fileSystemHost
                    .listFiles(&hostPath(&directory))
                    .map_err(|e| e.to_string())?;
                files.extend(
                    entries
                        .into_iter()
                        .filter(|entry| {
                            !entry.isDirectory
                                && sharedSources
                                    .as_ref()
                                    .is_none_or(|sources| sources.contains(&entry.name))
                        })
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
                        cacheKey.clone(),
                        None,
                        hostPath(&file),
                        "scope_failed",
                        error,
                        packageLoadIssueKind(&cacheKey, false, false),
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
    /// Lists bundled sources for explicit installation, including uninstalled built-in packages.
    fn scanBundledExternalPackageCandidates(&mut self) -> Vec<PackageScanCandidateResult> {
        let previousCache = self.bundledExternalPackageScanCache.clone();
        let mut nextCache = BTreeMap::new();
        let mut results = Vec::new();
        let support = self.toolHandler.runtimeSupport();
        for asset in support.bundledExternalPluginAssets().iter().chain(support.builtinPluginAssets()) {
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

    /// Scans optional bundled archives for explicit installation.
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
                requiredInstallationScope: None,
            });
        }
        if let Some(loadResult) = result.toolPkgLoadResult {
            let runtime = loadResult.containerRuntime;
            let toolCount = loadResult.containerPackage.tools.len();
            let subpackageCount = runtime.subpackages.len();
            let requiredInstallationScope = runtime
                .manifestExtensions
                .get("installation_scope")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
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
                requiredInstallationScope,
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
            match self
                .installedPackageConfigScope(path)
                .and_then(|scope| self.loadToolPkgFromExternalFileWithIssues(path, scope))
            {
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
        match self
            .installedPackageConfigScope(path)
            .and_then(|scope| self.loadToolPkgFromMarketFileWithIssues(path, scope))
        {
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

    /// Merges candidate registrations while preserving explicit conflict diagnostics.
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
                if result.phase == "external" {
                    if let Err(error) = Self::prepareExternalToolPkgOverride(
                        &loadResult,
                        &mut stagedAvailablePackages,
                        &mut stagedToolPkgContainers,
                        &mut stagedToolPkgSubpackages,
                    ) {
                        stagedToolPkgLoadIssues.push(newToolPkgLoadIssue(
                            result.sourcePath.clone(),
                            Some(loadResult.containerPackage.name.clone()),
                            loadResult.containerRuntime.displayName.resolve(false),
                            "duplicate_package",
                            error,
                            "toolpkg",
                        ));
                        continue;
                    }
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

    /// Reads the parser-validated installation restriction retained in manifest metadata.
    fn requiredInstallationScope(
        runtime: &ToolPkgContainerRuntime,
    ) -> Result<Option<ToolPkgConfigScope>, String> {
        runtime
            .manifestExtensions
            .get("installation_scope")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|error| format!("Invalid installation scope: {error}"))
    }

    /// Registers a concrete imported source without changing its plugin API version.
    fn registerScope(
        &self,
        id: &str,
        source: &str,
        builtin: bool,
        members: Vec<String>,
        scope: ToolPkgConfigScope,
        requiredScope: Option<ToolPkgConfigScope>,
    ) -> Result<(), String> {
        if requiredScope.is_some_and(|required| required != scope) {
            return Err(format!(
                "Package {id} cannot be installed in {} scope",
                scope.as_str()
            ));
        }
        let store = ExtensionStore::default();
        let mut settings = serde_json::json!({
            "builtin": builtin, "members": members, "enabledNames": [], "disabledNames": [],
            "subpackageStates": {}, "order": 0,
            "installationId": encodeMarketToolPkgInstallationId(&self.marketToolPkgInstallationId()?)
        });
        if let Some(required) = requiredScope {
            settings["installationScope"] =
                serde_json::to_value(required).map_err(|error| error.to_string())?;
        }
        store.registerInScope("package", id, source, settings, scope.as_str())?;
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
        if let Some(runtime) = self.getToolPkgContainerRuntime(&name) {
            if let Some(required) = Self::requiredInstallationScope(&runtime)? {
                if required.as_str() != scope {
                    return Err(format!(
                        "Package {name} only supports {} installation scope",
                        required.as_str()
                    ));
                }
            }
        }
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
                let requiredScope = snapshot
                    .toolPkgContainers
                    .get(name)
                    .map(Self::requiredInstallationScope)
                    .transpose()?
                    .flatten();
                let existing = store
                    .records("package")?
                    .into_iter()
                    .any(|record| record.id == *name);
                if !existing {
                    let scope = match snapshot.toolPkgContainers.get(name) {
                        Some(container) if !builtin => {
                            self.installedPackageConfigScope(Path::new(&container.sourcePath))?
                        }
                        _ => match requiredScope {
                            Some(scope) => scope,
                            None => ToolPkgConfigScope::Device,
                        },
                    };
                    self.registerScope(name, &sourceName, builtin, members.clone(), scope, requiredScope)?;
                }
                let mut record = store.record("package", name)?;
                if requiredScope.is_some_and(|required| required.as_str() != record.scope) {
                    return Err(format!(
                        "Package {name} is installed outside its required scope"
                    ));
                }
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
                let mut settings = record.settings.clone();
                settings["members"] =
                    serde_json::to_value(members).map_err(|error| error.to_string())?;
                let fields = settings
                    .as_object_mut()
                    .ok_or("Package settings must be an object")?;
                match requiredScope {
                    Some(scope) => {
                        fields.insert(
                            "installationScope".into(),
                            serde_json::to_value(scope).map_err(|error| error.to_string())?,
                        );
                    }
                    None => {
                        fields.remove("installationScope");
                    }
                }
                if record.settings != settings {
                    store.setSettings("package", name, settings.clone())?;
                    record.settings = settings;
                }
                self.scopeState
                    .lock()
                    .map_err(|e| e.to_string())?
                    .insert(name.clone(), record);
                Ok(())
            })();
            if let Err(error) = registration {
                let ownsToolPkg = snapshot.toolPkgContainers.contains_key(name);
                let market = snapshot
                    .toolPkgContainers
                    .get(name)
                    .is_some_and(|container| container.sourceType == ToolPkgSourceType::MARKET);
                snapshot.toolPkgLoadIssues.push(newToolPkgLoadIssue(
                    name.clone(),
                    Some(name.clone()),
                    name.clone(),
                    "scope_failed",
                    error,
                    packageLoadIssueKind(name, ownsToolPkg, market),
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
        // A reload also invalidates directory sources whose child files can change
        // without changing the parent directory metadata or declared package version.
        self.toolPkgTextResourceCache.clear();
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

    /// Validates all external names before removing any replaced built-in registrations.
    #[allow(non_snake_case)]
    fn prepareExternalToolPkgOverride(
        loadResult: &ToolPkgLoadResult,
        availablePackagesTarget: &mut BTreeMap<String, ToolPackage>,
        toolPkgContainersTarget: &mut BTreeMap<String, ToolPkgContainerRuntime>,
        toolPkgSubpackageByPackageNameTarget: &mut BTreeMap<String, ToolPkgSubpackageRuntime>,
    ) -> Result<(), String> {
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
                    return Err(format!(
                        "ToolPkg member '{}' is already registered by '{}' at '{}'",
                        packageName, existingContainer.packageName, existingContainer.sourcePath
                    ));
                }
                builtInContainersToRemove.insert(existingContainer.packageName.clone());
                continue;
            }

            if let Some(existingSubpackage) = toolPkgSubpackageByPackageNameTarget.get(&packageName)
            {
                let Some(ownerContainer) =
                    toolPkgContainersTarget.get(&existingSubpackage.containerPackageName)
                else {
                    return Err(format!(
                        "ToolPkg member '{}' has an unregistered owner '{}'",
                        packageName, existingSubpackage.containerPackageName
                    ));
                };
                if ownerContainer.sourceType != ToolPkgSourceType::ASSET {
                    return Err(format!(
                        "ToolPkg member '{}' is already registered by '{}' at '{}'",
                        packageName, ownerContainer.packageName, ownerContainer.sourcePath
                    ));
                }
                builtInContainersToRemove.insert(ownerContainer.packageName.clone());
                continue;
            }

            if let Some(existingPackage) = availablePackagesTarget.get(&packageName) {
                if !existingPackage.is_built_in {
                    return Err(format!("ToolPkg member '{}' is already registered as an external standalone package", packageName));
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
        Ok(())
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

    /// Validates source ownership and manifest restrictions before writing an installed update.
    fn bundledInstallationScope(
        &self,
        packageName: &str,
        sourceFileName: &str,
        loadResult: &Option<ToolPkgLoadResult>,
    ) -> Result<ToolPkgConfigScope, String> {
        let records = self.scopeState.lock().map_err(|error| error.to_string())?;
        let Some(record) = records.get(packageName) else {
            return Self::bundledCandidateScope(loadResult);
        };
        if record.settings["customized"] == true {
            return Err(format!(
                "Package has a custom implementation: {packageName}"
            ));
        }
        if record.settings["builtin"] == true {
            if !record.sourceName.is_empty() {
                return Err(format!(
                    "Built-in package has an unexpected source: {packageName}"
                ));
            }
        } else if record.sourceName != sourceFileName {
            return Err(format!(
                "Package source conflicts with installation: {packageName}"
            ));
        }
        let scope = serde_json::from_value::<ToolPkgConfigScope>(serde_json::Value::String(
            record.scope.clone(),
        ))
        .map_err(|error| error.to_string())?;
        let requiredScope = loadResult
            .as_ref()
            .map(|loaded| Self::requiredInstallationScope(&loaded.containerRuntime))
            .transpose()?
            .flatten();
        if requiredScope.is_some_and(|required| required != scope) {
            return Err(format!(
                "Package {packageName} cannot be updated in {} scope",
                scope.as_str()
            ));
        }
        Ok(scope)
    }

    /// Resolves the manifest restriction for a bundled asset before installation.
    fn bundledCandidateScope(
        loadResult: &Option<ToolPkgLoadResult>,
    ) -> Result<ToolPkgConfigScope, String> {
        match loadResult
            .as_ref()
            .map(|loaded| Self::requiredInstallationScope(&loaded.containerRuntime))
            .transpose()?
            .flatten()
        {
            Some(scope) => Ok(scope),
            None => Ok(ToolPkgConfigScope::Device),
        }
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
            let bytes = self
                .fileSystemHost
                .readFileBytes(&hostPath(&file))
                .map_err(|error| error.to_string())?;
            let scope = self.importedToolPkgScope(&bytes)?;
            let (loadResult, loadIssues) = self
                .loadToolPkgFromExternalFileWithIssues(&file, scope)
                .map_err(|error| format!("Error importing package: {error}"))?;
            appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &loadIssues);
            let packageName = loadResult.containerPackage.name.clone();
            let marketOrigin = loadResult.marketOrigin.clone();
            if !self.canImportToolPkg(&loadResult)? {
                return Err(format!(
                    "A package with name '{}' already exists in available packages",
                    packageName
                ));
            }
            let packageRoot = self
                .storePaths
                .runtime_storage_path(&ExtensionStore::root("package", scope.as_str())?);
            self.fileSystemHost
                .makeDirectory(&hostPath(&packageRoot), true)
                .map_err(|error| error.to_string())?;
            let fileName = file
                .file_name()
                .ok_or_else(|| "Error importing package: invalid file name".to_string())?;
            let destinationFile = packageRoot.join(fileName);
            if file != destinationFile {
                self.fileSystemHost
                    .copyFile(&hostPath(&file), &hostPath(&destinationFile), false)
                    .map_err(|error| format!("Error importing package: {error}"))?;
            }
            let (importedLoadResult, importedLoadIssues) = self
                .loadToolPkgFromExternalFileWithIssues(&destinationFile, scope)
                .map_err(|error| format!("Error importing package: {error}"))?;
            appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &importedLoadIssues);
            if !self.registerImportedToolPkg(importedLoadResult)? {
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

        let customization = self.customizablePackage(&packageMetadata.name)?;
        if customization.is_none()
            && (self.availablePackages().contains_key(&packageMetadata.name)
                || self.toolPkgManager().isToolPkgContainer(&packageMetadata.name)
                || self.toolPkgManager().hasSubpackage(&packageMetadata.name))
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
        let scope = match &customization {
            Some(record) => serde_json::from_value::<ToolPkgConfigScope>(serde_json::Value::String(record.scope.clone()))
                .map_err(|error| error.to_string())?,
            None => ToolPkgConfigScope::Device,
        };
        let packageRoot = self.storePaths.runtime_storage_path(&ExtensionStore::root("package", scope.as_str())?);
        self.fileSystemHost.makeDirectory(&hostPath(&packageRoot), true).map_err(|error| error.to_string())?;
        let destinationFile = packageRoot.join(fileName);
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
        if customization.is_some() {
            let enabledNames = self.getEnabledPackageNames();
            self.saveEnabledPackageNames(&enabledNames).map_err(|error| error.to_string())?;
            ExtensionStore::default().customizePackage(
                &packageMetadata.name, &fileName.to_string_lossy(), vec![packageMetadata.name.clone()],
            )?;
            self.externalPackageScanCache.remove(&hostPath(&destinationFile));
            self.loadAvailablePackages();
        } else {
            self.registerScope(
                &packageMetadata.name, &fileName.to_string_lossy(), false,
                vec![packageMetadata.name.clone()], scope, None,
            )?;
            self.pluginPackageManager.registerPackage(ToolPackage {
                is_built_in: false,
                ..packageMetadata.clone()
            });
        }
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

    /// Lists built-in identities currently supplied by user-installed custom packages.
    #[allow(non_snake_case)]
    pub fn getCustomizedPackageNames(&self) -> Result<Vec<String>, String> {
        Ok(ExtensionStore::default()
            .records("package")?
            .into_iter()
            .filter(|record| record.settings["customized"] == true)
            .map(|record| record.id)
            .collect())
    }

    /// Resolves an installed built-in identity that accepts a custom implementation.
    fn customizablePackage(&self, packageName: &str) -> Result<Option<ExtensionRecord>, String> {
        Ok(ExtensionStore::default()
            .records("package")?
            .into_iter()
            .find(|record| {
                record.id == packageName
                    && (record.settings["builtin"] == true || record.settings["customized"] == true)
            }))
    }

    /// Resolves an archive's installation scope before executing registration code.
    fn importedToolPkgScope(&self, bytes: &[u8]) -> Result<ToolPkgConfigScope, String> {
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|error| error.to_string())?;
        let index = ToolPkgArchiveParser::buildZipEntryIndex(&mut archive);
        let preview = ToolPkgArchiveParser::readToolPkgManifestPreview(&mut archive, &index)
            .ok_or("ToolPkg manifest is missing or invalid")?;
        if let Some(record) = self.customizablePackage(&preview.manifest.toolpkgId)? {
            return serde_json::from_value(serde_json::Value::String(record.scope))
                .map_err(|error| error.to_string());
        }
        match preview.manifest.installationScope {
            Some(scope) => Ok(scope),
            None => Ok(ToolPkgConfigScope::Device),
        }
    }

    /// Checks package names with the replaced built-in container removed from the candidate registry.
    fn canImportToolPkg(&self, loadResult: &ToolPkgLoadResult) -> Result<bool, String> {
        if self
            .customizablePackage(&loadResult.containerPackage.name)?
            .is_none()
        {
            return Ok(self
                .toolPkgManager()
                .canRegisterToolPkg(loadResult, self.availablePackages()));
        }
        let mut packages = self.availablePackages().clone();
        let mut containers = self.toolPkgManager().getToolPkgContainerRuntimeMap();
        let mut subpackages = self.toolPkgManager().getToolPkgSubpackageRuntimeMap();
        Self::removeToolPkgContainerFromTargets(
            &loadResult.containerPackage.name,
            &mut packages,
            &mut containers,
            &mut subpackages,
        );
        packages.remove(&loadResult.containerPackage.name);
        Ok(Self::registerToolPkgInto(
            loadResult.clone(),
            &mut packages,
            &mut containers,
            &mut subpackages,
        ))
    }

    /// Registers an imported archive or switches its built-in owner to the custom source.
    fn registerImportedToolPkg(&mut self, loadResult: ToolPkgLoadResult) -> Result<bool, String> {
        let name = loadResult.containerPackage.name.clone();
        if self.customizablePackage(&name)?.is_none() {
            return Ok(self.registerToolPkg(loadResult));
        }
        let path = loadResult.containerRuntime.sourcePath.clone();
        let source = Path::new(&path)
            .file_name()
            .ok_or("Package source filename is missing")?
            .to_string_lossy();
        let mut members = vec![name.clone()];
        members.extend(
            loadResult
                .subpackagePackages
                .iter()
                .map(|package| package.name.clone()),
        );
        let enabledNames = self.getEnabledPackageNames();
        self.saveEnabledPackageNames(&enabledNames)
            .map_err(|error| error.to_string())?;
        ExtensionStore::default().customizePackage(&name, &source, members)?;
        self.externalPackageScanCache.remove(&path);
        self.loadAvailablePackages();
        Ok(self
            .getToolPkgContainerRuntime(&name)
            .is_some_and(|runtime| runtime.sourcePath == path))
    }

    /// Reads an archive from its registered ToolPkg source through the existing host abstractions.
    #[allow(non_snake_case)]
    pub fn readToolPkgArchiveBytes(&self, packageName: &str) -> Result<Vec<u8>, String> {
        let runtime = self
            .getToolPkgContainerRuntime(packageName)
            .ok_or("ToolPkg is not registered")?;
        match runtime.sourceType {
            ToolPkgSourceType::ASSET => RuntimeToolPkgAssetSource {
                runtimeSupport: self
                    .toolHandler
                    .runtimeDependencies()
                    .shared_runtime_support(),
            }
            .toolPkgAssetBytes(&runtime.sourcePath)
            .ok_or_else(|| "Registered ToolPkg asset is missing".to_string()),
            ToolPkgSourceType::EXTERNAL | ToolPkgSourceType::MARKET => self
                .fileSystemHost
                .readFileBytes(&runtime.sourcePath)
                .map_err(|error| error.to_string()),
        }
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
        if let Err(error) = ToolPkgProtection::validateMarketToolPkgArchive(&rawArchive) {
            return self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                &error,
                "market_toolpkg",
            );
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
        let scope = match self.importedToolPkgScope(&rawArchive) {
            Ok(scope) => scope,
            Err(error) => {
                return self.recordAndReturnPackageError(
                    filePath,
                    "market_import_failed",
                    &error,
                    "market_toolpkg",
                )
            }
        };
        let packagesDir = self.storePaths.runtime_storage_path(
            &ExtensionStore::root("package", scope.as_str()).expect("Validated package scope"),
        );
        if let Err(error) = self
            .fileSystemHost
            .makeDirectory(&hostPath(&packagesDir), true)
        {
            return self.recordAndReturnPackageError(
                filePath,
                "market_import_failed",
                &error.to_string(),
                "market_toolpkg",
            );
        }
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
                self.loadToolPkgFromMarketFileWithIssues(&stagingFile, scope)?;
            appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &previewIssues);
            let packageName = preview.containerPackage.name.clone();
            if !self.canImportToolPkg(&preview)? {
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
                self.loadToolPkgFromMarketFileWithIssues(&destinationFile, scope)?;
            appendUniqueToolPkgLoadIssues(&mut self.manualToolPkgLoadIssues, &loadedIssues);
            if !self.registerImportedToolPkg(loaded)? {
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

    /// Reads the registered main script directly for synchronous package consumers.
    #[allow(non_snake_case)]
    pub fn getRegisteredToolPkgMainScript(&self, containerPackageName: &str) -> Option<String> {
        let normalizedContainerPackageName = self.normalizePackageName(containerPackageName);
        self.toolPkgManager()
            .getRegisteredToolPkgMainScript(&normalizedContainerPackageName)
    }

    /// Captures a main-script read that shares the revision cache and performs no I/O until polled.
    pub(crate) fn mainScript(
        &self,
        runtime: ToolPkgContainerRuntime,
    ) -> impl Future<Output = Result<String, String>> + 'static {
        let reader = self.resourceReader();
        async move { reader.mainScript(&runtime) }
    }

    /// Reads a resource for synchronous SDK host operations.
    pub(crate) fn readToolPkgResourceBytes(
        &self,
        runtime: &ToolPkgContainerRuntime,
        resourcePath: &str,
    ) -> Option<Vec<u8>> {
        self.resourceReader()
            .readToolPkgResourceBytes(runtime, resourcePath)
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
    ) -> impl Future<Output = Option<String>> + 'static {
        let source = ToolPkgPackageService::new(self)
            .prepareToolPkgComposeDslScript(containerPackageName, uiModuleId);
        let reader = self.resourceReader();
        async move { source?.read(|runtime, screen| reader.readToolPkgResourceBytes(runtime, screen)) }
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
    /// Executes one before hook with an explicit caller-owned completion deadline.
    #[allow(non_snake_case)]
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
        self.runToolPkgMainHookWithDeadline(
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
            Some(timeoutMillis),
        )
    }

    /// Reports the exact timed-out before hook through the active frontend and returned error.
    #[allow(non_snake_case)]
    pub fn reportToolPkgHookTimeout(&self, packageName: &str, hookId: &str, event: &str) -> String {
        notifyToolPkgHookTimeout(&self.context, packageName, hookId, event)
    }

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
        self.runToolPkgMainHookWithDeadline(
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
            None,
        )
    }

    #[allow(non_snake_case)]
    /// Runs a main hook exported by a ToolPkg container with one explicit millisecond timeout.
    fn runToolPkgMainHookWithDeadline(
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
        timeoutMillis: Option<u64>,
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<Result<Option<String>, String>> {
        let startedAt = currentTimeMillis();
        let packageName = self.normalizePackageName(containerPackageName);
        let timedFunction = functionName.to_string();
        let timedEvent = eventName.unwrap_or(event).to_string();
        let pending = self.toolPkgManager().dispatchToolPkgHook(
            &self.getEnabledPackageNames(),
            ToolPkgHookInvocation {
                containerPackageName: packageName.clone(),
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
        );
        let preparedAt = currentTimeMillis();
        let manager = self.clone();
        let hookId = pluginId.map(str::to_owned);
        Box::pin(async move {
            let result = pending.await;
            let completedAt = currentTimeMillis();
            AppLogger::v_with_level(
                "ToolPkgTiming",
                &format!(
                    "kind=hook package={} function={} stage={} prepareMs={} executeMs={} totalMs={} success={}",
                    packageName, timedFunction, timedEvent,
                    preparedAt.saturating_sub(startedAt), completedAt.saturating_sub(preparedAt),
                    completedAt.saturating_sub(startedAt), result.is_ok(),
                ),
                operit_util::AppLogger::VERBOSE_LEVEL_3,
            );
            match result {
                Ok(value) => Ok(value),
                Err(error)
                    if error.kind
                        == operit_plugin_sdk::execution_result::JsExecutionErrorKind::Timeout =>
                {
                    Err(manager.reportToolPkgHookTimeout(
                        &packageName,
                        &hookId.ok_or_else(|| {
                            format!(
                                "Timed before hook {packageName}:{timedFunction} has no Hook ID"
                            )
                        })?,
                        &timedEvent,
                    ))
                }
                Err(error) => Err(error.to_string()),
            }
        })
    }

    /// Performs MCP discovery before synchronous package metadata/activation, without holding its lock.
    pub async fn prepareMcpPackage(
        manager: &Arc<Mutex<Self>>,
        packageName: &str,
    ) -> Result<(), String> {
        let mcpManager = {
            let guard = manager.lock().expect("package manager mutex poisoned");
            if !guard.isRegisteredMCPServer(packageName) {
                return Ok(());
            }
            guard.mcpManager.clone()
        };
        let client = mcpManager
            .getOrCreateClient(packageName)
            .await
            .ok_or_else(|| {
                mcpManager
                    .getLastConnectionFailureReason(packageName)
                    .unwrap_or_else(|| format!("Cannot connect to MCP server: {packageName}"))
            })?;
        if !client.ping() {
            return Err(format!("MCP server is not ready: {packageName}"));
        }
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
            self.context.clone(),
            serverConfig.name.clone(),
        )
        .getTools();
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
                record.scope != "space",
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
        let subpackages = self.toolPkgSubpackageByPackageNameInternal();
        let names: Vec<_> = self.normalizeEnabledPackageNames(names).into_iter()
            .filter(|name| !subpackages.contains_key(name)).collect();
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

    /// Retains explicit disablement for built-in identities and their custom implementations.
    #[allow(non_snake_case)]
    pub(crate) fn addToDisabledIfDefaultEnabled(
        &self,
        packageName: &str,
    ) -> Result<(), PreferencesDataStoreError> {
        let normalizedPackageName = self.normalizePackageName(packageName);
        let Some(toolPackage) = self.availablePackages().get(&normalizedPackageName) else {
            return Ok(());
        };
        if !(toolPackage.is_built_in && toolPackage.enabled_by_default)
            && self.customizablePackage(&normalizedPackageName)
                .map_err(PreferencesDataStoreError::Message)?.is_none()
        {
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
        Err(format!(
            "Package source is outside installed scope directories: {}",
            hostPath(path)
        ))
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
        operation: impl FnOnce(&dyn JsExecutionEngine) -> Result<T, String>,
    ) -> Result<T, String> {
        // Registration runs package-owned code; per-load ownership prevents one timed-out
        // package from retaining state or blocking registration of the next package.
        let registrationEngine = ToolPkgRegistrationEngineGuard {
            engine: self.toolPkgExecutionEngineFactory.createExecutionEngine()?,
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

    /// Resolves one container without copying the complete catalog.
    #[allow(non_snake_case)]
    fn getToolPkgContainerRuntime(&self, packageName: &str) -> Option<ToolPkgContainerRuntime> {
        RuntimePackageManager::getToolPkgContainerRuntime(self, packageName)
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
        .chain(runtimeSupport.builtinPluginAssets().iter())
        .find(|asset| asset.name == assetName)
}

#[allow(non_snake_case)]
fn buildBundledPluginAssetSignature(asset: &RuntimePluginAsset) -> String {
    format!("{}|{}|{}", asset.name, asset.bytes.len(), asset.sha256)
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
            "platform.ohos".to_string(),
            ConditionValue::Bool(platformName == "ohos"),
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

/// Classifies a load failure as a plugin when the source is a ToolPkg container.
fn packageLoadIssueKind(sourcePath: &str, ownsToolPkg: bool, market: bool) -> &'static str {
    if market {
        "market_toolpkg"
    } else if ownsToolPkg || hasToolPkgExtension(sourcePath) {
        "toolpkg"
    } else {
        "package"
    }
}

/// Classifies a startup row. ToolPkg archives stay plugins even when parsing fails.
fn packageLoadingKind(sourcePath: &str, parsedToolPkg: bool) -> &'static str {
    if parsedToolPkg || hasToolPkgExtension(sourcePath) {
        PLUGIN_LOAD_KIND_PLUGIN
    } else {
        PLUGIN_LOAD_KIND_PACKAGE
    }
}

fn hasToolPkgExtension(sourcePath: &str) -> bool {
    Path::new(sourcePath)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("toolpkg"))
}

#[cfg(test)]
mod condition_platform_tests {
    use super::*;

    /// Exposes each supported platform flag through the shared condition capability snapshot.
    #[test]
    fn platform_flags_include_ohos_without_matching_other_platforms() {
        let platforms = [
            (HostPlatform::Windows, "windows"),
            (HostPlatform::Linux, "linux"),
            (HostPlatform::Android, "android"),
            (HostPlatform::Macos, "macos"),
            (HostPlatform::Ios, "ios"),
            (HostPlatform::Web, "web"),
            (HostPlatform::Ohos, "ohos"),
        ];
        for (platform, name) in &platforms {
            let mut environment = HostEnvironmentDescriptor::android();
            environment.platform = platform.clone();
            let capabilities = buildConditionCapabilitiesSnapshot(&environment);
            assert_eq!(capabilities.get("platform.name"), Some(&ConditionValue::Str((*name).into())));
            for (_, flag) in &platforms {
                assert_eq!(capabilities.get(&format!("platform.{flag}")), Some(&ConditionValue::Bool(name == flag)));
            }
        }
    }
}

#[cfg(test)]
mod package_load_classification_tests {
    use super::{
        packageLoadIssueKind, packageLoadingKind, PLUGIN_LOAD_KIND_PACKAGE, PLUGIN_LOAD_KIND_PLUGIN,
    };

    /// Keeps ToolPkg archives and their containers out of the package category.
    #[test]
    fn toolpkg_failures_stay_plugins() {
        assert_eq!(
            packageLoadIssueKind("broken.toolpkg", false, false),
            "toolpkg"
        );
        assert_eq!(
            packageLoadIssueKind(r"C:\pkg\Broken.TOOLPKG", false, false),
            "toolpkg"
        );
        assert_eq!(packageLoadIssueKind("owner", true, false), "toolpkg");
        assert_eq!(packageLoadIssueKind("owner", true, true), "market_toolpkg");
        assert_eq!(packageLoadIssueKind("time.js", false, false), "package");
        assert_eq!(
            packageLoadIssueKind("metadata.hjson", false, false),
            "package"
        );
        assert_eq!(
            packageLoadingKind("broken.toolpkg", false),
            PLUGIN_LOAD_KIND_PLUGIN
        );
        assert_eq!(
            packageLoadingKind("time.js", false),
            PLUGIN_LOAD_KIND_PACKAGE
        );
        assert_eq!(
            packageLoadingKind("renamed.bin", true),
            PLUGIN_LOAD_KIND_PLUGIN
        );
    }
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
        packageLoadingKind(sourcePath, false),
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
    setPluginLoadingItemKind(
        &result.sourcePath,
        packageLoadingKind(&result.sourcePath, result.toolPkgLoadResult.is_some()),
    );
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
    let declarations = runtime
        .publicApis
        .iter()
        .filter(|declaration| declaration.id == methodName)
        .collect::<Vec<_>>();
    match declarations.as_slice() {
        [] => Err(format!(
            "Public API not found: {}/{methodName}",
            runtime.packageName
        )),
        [declaration] => {
            if declaration.function.trim().is_empty() {
                return Err(format!(
                    "Public API handler is empty: {}/{methodName}",
                    runtime.packageName
                ));
            }
            Ok(*declaration)
        }
        _ => Err(format!(
            "Duplicate public API: {}/{methodName}",
            runtime.packageName
        )),
    }
}

#[cfg(test)]
mod public_api_selection_tests {
    use super::*;
    use operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgRegisteredFunctionHook;

    /// Creates a public-method declaration with an explicit exported handler.
    fn declaration(name: &str, function: &str) -> ToolPkgRegisteredFunctionHook {
        ToolPkgRegisteredFunctionHook {
            id: name.to_string(),
            function: function.to_string(),
            functionSource: None,
        }
    }

    /// Selects arbitrary published names without applying a domain or command-root whitelist.
    #[test]
    fn selects_exact_registered_export() {
        let runtime = ToolPkgContainerRuntime {
            packageName: "demo".to_string(),
            publicApis: vec![declaration("configuration.read", "readConfiguration")],
            ..Default::default()
        };
        assert_eq!(
            selectToolPkgPublicApi(&runtime, "configuration.read")
                .unwrap()
                .function,
            "readConfiguration"
        );
        assert!(selectToolPkgPublicApi(&runtime, "Configuration.read").is_err());
        assert!(selectToolPkgPublicApi(&runtime, "missing").is_err());
        assert!(selectToolPkgPublicApi(&runtime, " ").is_err());
    }

    /// Rejects duplicate publications and empty handlers before entering a JavaScript runtime.
    #[test]
    fn rejects_duplicate_and_empty_exports() {
        let mut runtime = ToolPkgContainerRuntime {
            packageName: "demo".to_string(),
            publicApis: vec![declaration("read", "first"), declaration("read", "second")],
            ..Default::default()
        };
        assert_eq!(
            selectToolPkgPublicApi(&runtime, "read").unwrap_err(),
            "Duplicate public API: demo/read"
        );
        runtime.publicApis = vec![declaration("read", " ")];
        assert_eq!(
            selectToolPkgPublicApi(&runtime, "read").unwrap_err(),
            "Public API handler is empty: demo/read"
        );
    }
}

#[cfg(test)]
mod package_registration_tests {
    use super::*;

    /// Verifies duplicate diagnostics identify the actual conflicting member and preserve the original registry.
    #[test]
    fn external_subpackage_conflict_reports_its_real_name_without_mutating_registry() {
        let mut packages = BTreeMap::from([(
            "daily_life".into(),
            ToolPackage {
                name: "daily_life".into(),
                ..Default::default()
            },
        )]);
        let mut containers = BTreeMap::new();
        let mut subpackages = BTreeMap::new();
        let load = ToolPkgLoadResult {
            containerPackage: ToolPackage {
                name: "com.operit.daily_life".into(),
                ..Default::default()
            },
            subpackagePackages: vec![ToolPackage {
                name: "daily_life".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert_eq!(
            RuntimePackageManager::prepareExternalToolPkgOverride(
                &load,
                &mut packages,
                &mut containers,
                &mut subpackages
            )
            .unwrap_err(),
            "ToolPkg member 'daily_life' is already registered as an external standalone package"
        );
        assert_eq!(packages.len(), 1);
        assert!(packages.contains_key("daily_life"));
        assert!(containers.is_empty());
        assert!(subpackages.is_empty());
    }
}

/// Applies the package manager's context cache policy independently of transport attachment.
#[allow(non_snake_case)]
pub(crate) fn openComposeDslSessionForContext(
    sessions: &mut BTreeMap<(String, String), ToolPkgComposeDslSession>,
    key: (String, String),
    create: impl FnOnce() -> Result<ToolPkgComposeDslSession, String>,
) -> Result<String, String> {
    if let Some(session) = sessions.get(&key) {
        if !session.isClosed() {
            return Ok(session.id());
        }
    }
    // Closing the command producer removes the capability registry entry, but
    // the package manager can survive a Dart isolate and still hold this cache.
    sessions.remove(&key);
    let session = create()?;
    let id = session.id();
    sessions.insert(key, session);
    Ok(id)
}

/// Publishes a before-hook deadline failure and preserves presentation errors in the returned failure.
#[allow(non_snake_case)]
fn notifyToolPkgHookTimeout(
    context: &HostManager,
    packageName: &str,
    hookId: &str,
    event: &str,
) -> String {
    let message =
        format!("前置插件「{packageName}:{hookId}」响应超时（{event}），本次 Hook 已终止。");
    AppLogger::e("ToolPkgHookTimeout", &message);
    let notification = context
        .toastHost
        .as_ref()
        .ok_or_else(|| "Frontend Toast host is unavailable".to_string())
        .and_then(|host| host.toast(&message).map_err(|error| error.to_string()));
    match notification {
        Ok(()) => format!("TOOLPKG_BEFORE_HOOK_TIMEOUT: {message}"),
        Err(error) => format!("TOOLPKG_BEFORE_HOOK_TIMEOUT: {message} 超时提示显示失败：{error}"),
    }
}

#[cfg(test)]
mod hook_timeout_notification_tests {
    use super::*;

    /// Presents the exact owner and stage while returning the same terminal timeout to the caller.
    #[test]
    fn before_hook_timeout_is_presented_and_propagated() {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let received = messages.clone();
        let context = HostManager::new().withToastHost(Arc::new(move |message: &str| {
            received.lock().unwrap().push(message.to_string());
            Ok(())
        }));
        let error =
            notifyToolPkgHookTimeout(&context, "bridge.wechat", "before-send", "submit_requested");
        let messages = messages.lock().unwrap();
        assert_eq!(messages.as_slice(), ["前置插件「bridge.wechat:before-send」响应超时（submit_requested），本次 Hook 已终止。"]);
        assert_eq!(
            error,
            format!("TOOLPKG_BEFORE_HOOK_TIMEOUT: {}", messages[0])
        );
    }

    /// Propagates a failed presenter instead of claiming that a timeout notice was displayed.
    #[test]
    fn before_hook_timeout_preserves_presentation_failure() {
        let context = HostManager::new().withToastHost(Arc::new(|_message: &str| {
            Err(operit_host_api::HostError::new("toast channel closed"))
        }));
        let error =
            notifyToolPkgHookTimeout(&context, "bridge.wechat", "before-send", "submit_requested");
        assert!(error.starts_with("TOOLPKG_BEFORE_HOOK_TIMEOUT: "));
        assert!(error.ends_with("超时提示显示失败：toast channel closed"));
    }
}
