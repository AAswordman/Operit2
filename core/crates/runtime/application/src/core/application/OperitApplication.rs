use crate::data::backup::CharacterPluginMigration::{
    isRetiredCharacterOperation, isRetiredCharacterPreferenceKey,
};
use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use crate::core::events::RuntimeEvent::RuntimeEvent;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::data::preferences::PreferenceSpaceMerge;
use crate::data::preferences::ThemeConfigManager::ThemeConfigManager;
use crate::plugins::toolpkg::ToolPkgAppLifecycleHookBridge::ToolPkgAppLifecycleHookBridge;
use crate::plugins::toolpkg::ToolPkgChatComposerSlotBridge::ToolPkgChatComposerSlotBridge;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge;
use crate::plugins::PluginRegistry::PluginRegistry;
use crate::services::ProviderRuntimeSupportService::ProviderRuntimeSupportService;
use crate::services::ToolRuntimeSupportService::ToolRuntimeSupportService;
#[cfg(feature = "javascript")]
use operit_host_api::HostManager::setDefaultHostJavaScriptRuntimeHost;
use operit_host_api::HostManager::{
    setDefaultHostRuntimeTaskSchedulerHost, setDefaultHttpHost, setDefaultSerialPortHost,
    setDefaultWebSocketHost, HostManager,
};
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{HostRuntimeEventRegistration, HostRuntimeTaskSchedulerHost};
#[cfg(feature = "javascript")]
use operit_js_bridge::javascript::JsExecutionProvider::HostJsExecutionProvider;
use operit_providers::chat::llmprovider::ModelConfigConnectionTester::ModelConnectionTestReport;
use operit_providers::runtime_support::ProviderRuntimeContext;
use operit_store::sync::SqlChatSyncStore::{SqlChatSyncStore, CHAT_SYNC_DOMAIN};
use operit_store::CoreNodeBindingStore::{CoreNodeBindingStore, BINDING_SYNC_DOMAIN};
use operit_store::NetworkControlStore::{NetworkControlStore, NETWORK_CONTROL_SYNC_DOMAIN};
use operit_store::PluginStorage::{self, PLUGIN_STORAGE_SYNC_DOMAIN};
use operit_store::PreferencesDataStore::{Flow, StateFlow};
use operit_store::PreferenceMerge::applyIndependentOperations;
use operit_store::RuntimeFileSyncStore::{
    RuntimeFileSyncReference, RuntimeFileSyncStore, RUNTIME_FILE_SYNC_DOMAIN,
};
use operit_plugin_theme::ThemeValidationReceipt;
use operit_store::RuntimeStorageHost::{
    defaultRuntimeStorageHost, setDefaultHostSecretStore, setDefaultRuntimeSqliteHost,
    setDefaultRuntimeStorageHost,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::SyncOperationStore::{
    boundSyncOperationPage, compactSyncOperations, SyncOperation, SyncOperationKnowledge,
    SyncOperationSemantics, SyncOperationStore, SyncReceipt, DEFAULT_SYNC_PAGE_BYTES,
};
use sha2::{Digest, Sha256};
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::files::VisualFileSystem::VisualFileSystem;
use operit_tools::runtime_support::ToolRuntimeDependencies;
use operit_tools::tools::mcp_runtime::plugins::MCPStarter::MCPStarter;
use operit_tools::tools::PackageLoadingProgress::{
    completePluginLoadingSession, observePluginLoadingProgress, showPluginLoading,
};

use operit_tools::tools::mcp_runtime::MCPRepository::MCPRepository;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;
pub use operit_tools::tools::PackageLoadingProgress::{PluginLoadingItem, PluginLoadingProgress};
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as AsyncMutex;

/// One in-memory asset handed to the preview surface. It is not a library
/// record and is never written by the preview API.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemePreviewAsset {
    pub path: String,
    /// Theme media crosses the CoreLink boundary as MessagePack bin data.
    /// Without this annotation serde treats `Vec<u8>` as an integer array,
    /// while the generated Dart proxy correctly expects `Uint8List`/bin.
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

/// Receipt for the preview phase. It deliberately carries no selection or
/// persistence result.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemePreviewReceipt {
    pub validation: ThemeValidationReceipt,
    pub theme_id: String,
    pub version: String,
    pub theme_json: String,
    pub assets: Vec<ThemePreviewAsset>,
}

use operit_util::AppLogger::AppLogger;
use operit_util::OperitPaths;

static HOST_MANAGER: OnceLock<Mutex<Option<HostManager>>> = OnceLock::new();

/// Owns process-wide runtime initialization and exposes host-facing application operations.
pub struct OperitApplication {
    pub appStartupTimeMs: i64,
    pub hostManager: HostManager,
    pub chatRuntimeHolder: Arc<AsyncMutex<ChatRuntimeHolder>>,
    pub toolRuntimeDependencies: ToolRuntimeDependencies,
    pub toolHandler: AIToolHandler,
    pub toolPkgBridgeRuntime: ToolPkgBridgeRuntime,
    pub providerRuntimeContext: ProviderRuntimeContext,
    pub initialized: bool,
    extensionChanges: Option<operit_store::SyncAppliedChanges::SyncAppliedSubscription>,
    hostRuntimeEventRegistration: Option<Arc<Mutex<Box<dyn HostRuntimeEventRegistration>>>>,
}

impl OperitApplication {
    /// Creates an application using the default Android-style host context.
    pub fn new() -> Self {
        Self::newWithContext(HostManager::new())
    }

    /// Creates an application around a supplied host manager and installs shared host defaults.
    #[allow(non_snake_case)]
    pub fn newWithContext(hostManager: HostManager) -> Self {
        if let Some(runtimeStorageHost) = hostManager.runtimeStorageHost.clone() {
            let runtimeRoot = runtimeStorageHost
                .runtimeRootDir()
                .expect("runtime storage host must provide a runtime root directory");
            let workspaceRoot = runtimeStorageHost
                .workspaceRootDir()
                .expect("runtime storage host must provide a workspace root directory");
            let fileSystemHost = hostManager
                .fileSystemHost
                .clone()
                .expect("runtime storage host requires a file-system host for logging");
            let pathMapper = PathMapper::new(
                runtimeRoot.clone(),
                workspaceRoot.clone(),
                runtimeStorageHost.clone(),
            );
            let logFile = pathMapper
                .resolve("/app/data/logs/operit.log")
                .expect("runtime log path must resolve through the file-system host")
                .physicalPath;
            let packageLogFile = pathMapper
                .resolve("/app/data/logs/toolpkg.log")
                .expect("ToolPkg log path must resolve through the file-system host")
                .physicalPath;
            if let Err(error) =
                AppLogger::configure_log_files(fileSystemHost, logFile, packageLogFile)
            {
                AppLogger::w(
                    "OperitApplication",
                    &format!(
                        "File logging unavailable; continuing with console and in-memory logs: {error}"
                    ),
                );
            }
            setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(
                runtimeRoot,
                workspaceRoot,
            ));
            setDefaultRuntimeStorageHost(runtimeStorageHost);
        }
        if let Some(runtimeSqliteHost) = hostManager.runtimeSqliteHost.clone() {
            setDefaultRuntimeSqliteHost(runtimeSqliteHost);
        }
        if let Some(hostSecretStore) = hostManager.hostSecretStore.clone() {
            setDefaultHostSecretStore(hostSecretStore);
        }
        if let Some(httpHost) = hostManager.httpHost.clone() {
            setDefaultHttpHost(httpHost);
        }
        if let Some(webSocketHost) = hostManager.webSocketHost.clone() {
            setDefaultWebSocketHost(webSocketHost);
        }
        if let Some(discoveryHost) = hostManager.serviceDiscoveryHost.clone() {
            operit_host_api::HostManager::setDefaultServiceDiscoveryHost(discoveryHost);
        }
        if let Some(host) = hostManager.httpServerHost.clone() {
            operit_host_api::HostManager::setDefaultHttpServerHost(host);
        }
        if let Some(host) = hostManager.tcpHost.clone() {
            operit_host_api::HostManager::setDefaultTcpHost(host);
        }
        if let Some(serialPortHost) = hostManager.serialPortHost.clone() {
            setDefaultSerialPortHost(serialPortHost);
        }
        if let Some(taskSchedulerHost) = hostManager.hostRuntimeTaskSchedulerHost.clone() {
            setDefaultHostRuntimeTaskSchedulerHost(taskSchedulerHost);
        }
        #[cfg(feature = "javascript")]
        setDefaultHostJavaScriptRuntimeHost(
            hostManager
                .hostJavaScriptRuntimeHost
                .clone()
                .expect("JavaScript runtime requires a HostJavaScriptRuntimeHost"),
        );
        let chatFileSystemHost = hostManager
            .fileSystemHost
            .clone()
            .expect("Chat runtime requires a FileSystemHost");
        let chatRuntimeHolder = Arc::new(AsyncMutex::new(ChatRuntimeHolder::new(
            chatFileSystemHost.clone(),
        )));
        let runtimeToolSupport =
            ToolRuntimeSupportService::create(hostManager.clone(), chatRuntimeHolder.clone());
        #[cfg(feature = "javascript")]
        let toolRuntimeDependencies = ToolRuntimeDependencies::new(
            runtimeToolSupport.clone(),
            Arc::new(HostJsExecutionProvider::new()),
        );
        let toolHandler = AIToolHandler::new(hostManager.clone(), toolRuntimeDependencies.clone());
        let toolPkgBridgeRuntime =
            ToolPkgBridgeRuntime::new(toolHandler.clone(), hostManager.clone());
        let providerRuntimeContext = ProviderRuntimeSupportService::create(toolHandler.clone());
        runtimeToolSupport
            .bindRuntimeServices(toolHandler.clone(), providerRuntimeContext.clone())
            .expect("tool runtime services must bind exactly once");
        let initializedChatRuntimeHolder = ChatRuntimeHolder::newWithRuntimeDependencies(
            chatFileSystemHost,
            toolHandler.clone(),
            providerRuntimeContext.clone(),
        );
        runtimeToolSupport
            .bindChatHistoryManager(
                initializedChatRuntimeHolder
                    .chatHistoryManager()
                    .expect("initialized chat runtime must expose its actual main record manager")
                    .clone(),
            )
            .expect("tool runtime must bind the canonical chat record manager exactly once");
        *chatRuntimeHolder
            .try_lock()
            .expect("new chat runtime holder must be unlocked") = initializedChatRuntimeHolder;
        Self {
            appStartupTimeMs: 0,
            hostManager,
            chatRuntimeHolder,
            toolRuntimeDependencies,
            toolHandler,
            toolPkgBridgeRuntime,
            providerRuntimeContext,
            initialized: false,
            hostRuntimeEventRegistration: None,
            extensionChanges: None,
        }
    }

    /// Shares live services for a command without creating a second runtime or lifecycle.
    /// Each invocation owns its dispatch view so nested tool calls need no application lock.
    #[allow(non_snake_case)]
    pub fn sharedCoreCommandRuntime(&self) -> Self {
        Self {
            appStartupTimeMs: self.appStartupTimeMs,
            hostManager: self.hostManager.clone(),
            chatRuntimeHolder: self.chatRuntimeHolder.clone(),
            toolRuntimeDependencies: self.toolRuntimeDependencies.clone(),
            toolHandler: self.toolHandler.clone(),
            toolPkgBridgeRuntime: self.toolPkgBridgeRuntime.clone(),
            providerRuntimeContext: self.providerRuntimeContext.clone(),
            initialized: self.initialized,
            // The original application remains the sole lifecycle/subscription owner.
            extensionChanges: None,
            hostRuntimeEventRegistration: None,
        }
    }

    /// Removes files queued for cleanup through the configured file-system host.
    #[allow(non_snake_case)]
    fn cleanOnExitFiles(&self) -> Result<(), String> {
        let fileSystem = self.runtimeFileSystem()?;
        const CLEAN_ON_EXIT_PATH: &str = "/app/data/temp/clean_on_exit";
        fileSystem.makeDirectory(CLEAN_ON_EXIT_PATH, true)?;
        for entry in fileSystem.listFiles(CLEAN_ON_EXIT_PATH)? {
            let path = PathMapper::joinVfsPath(CLEAN_ON_EXIT_PATH, &entry.name)?;
            fileSystem.deleteFile(&path, entry.isDirectory)?;
        }
        Ok(())
    }

    /// Builds the virtual file system backed by the configured runtime hosts.
    fn runtimeFileSystem(&self) -> Result<VisualFileSystem, String> {
        let runtimeStorageHost = self
            .hostManager
            .runtimeStorageHost
            .as_ref()
            .ok_or_else(|| {
                "RuntimeStorageHost is not registered for runtime file-system access".to_string()
            })?;
        let runtimeRoot = runtimeStorageHost.runtimeRootDir().ok_or_else(|| {
            "RuntimeStorageHost runtime root is not configured for runtime file-system access"
                .to_string()
        })?;
        let workspaceRoot = runtimeStorageHost.workspaceRootDir().ok_or_else(|| {
            "RuntimeStorageHost workspace root is not configured for runtime file-system access"
                .to_string()
        })?;
        let fileSystemHost = self.hostManager.fileSystemHost.clone().ok_or_else(|| {
            "FileSystemHost is not registered for runtime file-system access".to_string()
        })?;
        Ok(VisualFileSystem::new(
            fileSystemHost,
            PathMapper::new(runtimeRoot, workspaceRoot, runtimeStorageHost.clone()),
        ))
    }

    /// Ensures a mapped directory exists through the configured file-system host.
    fn ensureHostDirectory(&self, path: &std::path::Path) -> Result<(), String> {
        let fileSystemHost = self.hostManager.fileSystemHost.as_ref().ok_or_else(|| {
            "FileSystemHost is not registered for runtime directory creation".to_string()
        })?;
        fileSystemHost
            .makeDirectory(&path.to_string_lossy(), true)
            .map_err(|error| error.message)
    }

    /// Initializes persistent stores, prompt managers, tool handlers, plugins, and runtime events.
    #[allow(non_snake_case)]
    pub fn onCreate(&mut self) -> Result<(), String> {
        self.appStartupTimeMs = currentTimeMillis();
        AppLogger::v("OperitApplication", "runtime initialization start");
        setHostManager(self.hostManager.clone());
        if let Some(storage) = self.hostManager.runtimeStorageHost.clone() {
            if let Err(error) = PreferenceSpaceMerge::prepareLayout(storage) {
                AppLogger::e(
                    "PreferenceLayout",
                    &format!("Preference layout migration incomplete: {error}"),
                );
            }
        }
        let cleanupStartedAt = currentTimeMillis();
        self.configureOpenMpEnvironment();
        self.cleanOnExitFiles()?;
        AppLogger::v(
            "OperitApplication",
            &format!(
                "clean on-exit files done elapsedMs={}",
                currentTimeMillis() - cleanupStartedAt
            ),
        );
        let databaseStartedAt = currentTimeMillis();
        self.ensureWorkManagerInitialized();
        AIMessageManager::initialize();
        self.initializeJsonSerializer();
        self.initializeAppLanguage();
        self.initAndroidPermissionPreferences();
        operit_store::db::AppDatabase::AppDatabase::getDatabase(RuntimeStorePaths::default())
            .map_err(|error| error.to_string())?;
        crate::data::backup::CharacterPluginMigration::migrateLegacyCharacters(
            RuntimeStorePaths::default(),
        )?;
        self.preloadDatabase();
        AppLogger::v(
            "OperitApplication",
            &format!(
                "database preload done elapsedMs={}",
                currentTimeMillis() - databaseStartedAt
            ),
        );
        let mut toolHandler = self.toolHandler.clone();
        let toolRegistrationStartedAt = currentTimeMillis();
        AppLogger::v("OperitApplication", "default tool registration start");
        toolHandler.registerDefaultTools();
        AppLogger::v(
            "OperitApplication",
            &format!(
                "default tool registration done elapsedMs={}",
                currentTimeMillis() - toolRegistrationStartedAt
            ),
        );
        let pluginInitializationStartedAt = currentTimeMillis();
        AppLogger::v("OperitApplication", "built-in plugin initialization start");
        PluginRegistry::initializeBuiltins(self.toolPkgBridgeRuntime.clone());
        crate::plugins::toolpkg::ToolPkgChatLifecycleHookBridge::ToolPkgChatLifecycleHookBridge::register(self.toolPkgBridgeRuntime.clone())?;
        AppLogger::v(
            "OperitApplication",
            &format!(
                "built-in plugin initialization done elapsedMs={}",
                currentTimeMillis() - pluginInitializationStartedAt
            ),
        );
        ToolPkgAppLifecycleHookBridge::dispatchEvent(
            &self.toolPkgBridgeRuntime,
            operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::TOOLPKG_EVENT_APPLICATION_ON_CREATE,
            serde_json::json!({
                "extras": {
                    "startupTimeMs": self.appStartupTimeMs,
                    "elapsedMs": currentTimeMillis() - self.appStartupTimeMs,
                }
            }),
        );
        let runtimeEventRegistrationStartedAt = currentTimeMillis();
        AppLogger::v("OperitApplication", "host runtime event registration start");
        self.hostRuntimeEventRegistration =
            crate::services::RuntimeEventIngressService::RuntimeEventIngressService::startHostRuntimeEventSupport(
                self.hostManager.clone(),
                self.toolPkgBridgeRuntime.clone(),
            )?
            .map(|registration| Arc::new(Mutex::new(registration)));
        AppLogger::v(
            "OperitApplication",
            &format!(
                "host runtime event registration done elapsedMs={}",
                currentTimeMillis() - runtimeEventRegistrationStartedAt
            ),
        );
        self.extensionChanges = crate::services::ExtensionRuntimeService::start(
            self.hostManager.clone(),
            self.toolHandler.clone(),
        );
        self.dispatchPluginLoading()?;
        self.initialized = true;
        AppLogger::i(
            "OperitApplication",
            &format!(
                "runtime initialization done elapsedMs={}",
                currentTimeMillis() - self.appStartupTimeMs
            ),
        );
        Ok(())
    }

    /// Delivers one normalized host event to registered ToolPkg host-event hooks.
    #[allow(non_snake_case)]
    pub fn ingestRuntimeEvent(&self, event: RuntimeEvent) -> serde_json::Value {
        crate::services::RuntimeEventIngressService::RuntimeEventIngressService::getInstance(
            &self.hostManager,
            self.toolPkgBridgeRuntime.clone(),
        )
        .ingestEvent(event)
    }

    /// Applies host-specific OpenMP environment setup before runtime services start.
    #[allow(non_snake_case)]
    pub fn configureOpenMpEnvironment(&self) {}

    /// Ensures background work infrastructure is available for runtime tasks.
    #[allow(non_snake_case)]
    pub fn ensureWorkManagerInitialized(&self) {}

    /// Registers JSON serialization rules used by generated bridge payloads.
    #[allow(non_snake_case)]
    pub fn initializeJsonSerializer(&self) {}

    /// Initializes application language resources before user-facing services are created.
    #[allow(non_snake_case)]
    pub fn initializeAppLanguage(&self) {}

    /// Initializes platform permission preference state used by Android-facing tools.
    #[allow(non_snake_case)]
    pub fn initAndroidPermissionPreferences(&self) {}

    /// Touches database-backed services early so schema setup happens during startup.
    #[allow(non_snake_case)]
    pub fn preloadDatabase(&self) {}

    /// Schedules the single initial package and MCP load and returns its submission error.
    #[allow(non_snake_case)]
    pub fn initMcpPlugins(&self) -> Result<(), String> {
        self.dispatchPluginLoading()
    }

    /// Claims and schedules the owning initial load task that publishes canonical registry readiness.
    fn dispatchPluginLoading(&self) -> Result<(), String> {
        let packageManager = self.toolHandler.getOrCreatePackageManager();
        let readiness = {
            let manager = packageManager.lock().map_err(|error| error.to_string())?;
            if !manager.claimInitialPackageRegistryLoad()? {
                return Ok(());
            }
            manager.packageRegistryReadiness()
        };
        let taskScheduler = self
            .hostManager
            .hostRuntimeTaskSchedulerHost
            .clone()
            .ok_or_else(|| {
                let error =
                    "runtime task scheduler host must be configured for plugin startup".to_string();
                readiness.fail(error.clone());
                error
            })?;
        let hostManager = self.hostManager.clone();
        let taskReadiness = readiness.clone();
        let startup = move || {
            Box::pin(async move {
                let loadingGeneration = showPluginLoading();
                let loadOutcome = packageManager
                    .lock()
                    .map_err(|error| error.to_string())
                    .and_then(|mut manager| {
                        manager.loadAvailablePackages();
                        manager.packageRegistryReadiness().require_ready()
                    });
                if let Err(error) = loadOutcome {
                    taskReadiness.fail(error.clone());
                    completePluginLoadingSession(loadingGeneration);
                    AppLogger::e("OperitApplication", &error);
                    return;
                }
                let starter = MCPStarter::new(hostManager);
                let timeoutSeconds = ApiPreferences::getInstance()
                    .getMcpStartupTimeoutSeconds()
                    .expect("api preferences must provide mcp startup timeout seconds");
                let _ = starter
                    .startAllDeployedPluginsWithTimeout(timeoutSeconds)
                    .await;
                completePluginLoadingSession(loadingGeneration);
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>
        };
        taskScheduler
            .scheduleHostRuntimeAsyncTask("operit-plugin-startup", Box::new(startup))
            .map_err(|error| {
                let error = error.to_string();
                readiness.fail(error.clone());
                error
            })
    }

    /// Observes package and plugin loading overlay progress.
    #[allow(non_snake_case)]
    #[operit_route_macros::operit_plugin_sdk_expose]
    pub fn pluginLoadingProgressFlow(&self) -> StateFlow<PluginLoadingProgress> {
        observePluginLoadingProgress()
    }

    /// Hides the package and plugin loading overlay without stopping the load session.
    #[allow(non_snake_case)]
    pub fn skipPluginLoading(&self) {
        operit_tools::tools::PackageLoadingProgress::skipPluginLoading();
    }

    /// Returns the initialized tool handler owned by this runtime.
    #[allow(non_snake_case)]
    pub fn aiToolHandler(&self) -> AIToolHandler {
        self.toolHandler.clone()
    }

    /// Creates an MCP repository with this runtime's host and tool support.
    #[allow(non_snake_case)]
    pub fn mcpRepository(&self) -> MCPRepository {
        MCPRepository::getInstance(&self.hostManager, self.toolHandler.runtimeSupport())
    }

    /// Creates a skill repository with this runtime's host and tool support.
    #[allow(non_snake_case)]
    pub fn skillRepository(&self) -> SkillRepository {
        SkillRepository::getInstance(&self.hostManager, self.toolHandler.runtimeSupport())
    }

    /// Observes extension catalog changes from both local operations and device-space synchronization.
    #[allow(non_snake_case)]
    pub fn extensionCatalogRevisionFlow(&self) -> StateFlow<i64> {
        operit_store::ExtensionStore::catalogRevisionFlow()
    }

    /// Lists explicit installation scopes, including subpackages that inherit their container scope.
    #[allow(non_snake_case)]
    pub fn getExtensionScopes(&self, kind: String) -> Result<BTreeMap<String, String>, String> {
        let packageOwners = match kind.as_str() {
            "package" => {
                let manager = self.packageManager();
                let manager = manager.lock().map_err(|error| error.to_string())?;
                Some(
                    manager
                        .getTopLevelAvailablePackages()
                        .into_keys()
                        .collect::<std::collections::BTreeSet<_>>(),
                )
            }
            "skill" => {
                self.skillRepository().getAvailableSkillPackages();
                None
            }
            "mcp" => None,
            _ => return Err(format!("Unknown extension kind: {kind}")),
        };
        let mut result = BTreeMap::new();
        for record in operit_store::ExtensionStore::ExtensionStore::default().records(&kind)? {
            // Stale owners and rejected packages must not invalidate the loaded catalog's scopes.
            if packageOwners
                .as_ref()
                .is_some_and(|owners| !owners.contains(&record.id))
            {
                continue;
            }
            let scope = record.scope.clone();
            if result.insert(record.id.clone(), scope.clone()).is_some() {
                return Err(format!(
                    "Extension ownership conflict: {} exists in both locations",
                    record.id
                ));
            }
            if kind == "package" {
                let members: Vec<String> =
                    serde_json::from_value(record.settings["members"].clone())
                        .map_err(|e| e.to_string())?;
                for member in members {
                    if member != record.id {
                        result.insert(member, scope.clone());
                    }
                }
            }
        }
        Ok(result)
    }

    /// Moves an existing extension; local MCP is rejected by the owning store before content changes.
    #[allow(non_snake_case)]
    pub fn setExtensionScope(&self, kind: String, id: String, scope: String) -> Result<(), String> {
        match kind.as_str() {
            "package" => self
                .packageManager()
                .lock()
                .map_err(|e| e.to_string())?
                .setPackageScope(&id, &scope),
            "skill" => {
                self.skillRepository().getAvailableSkillPackages();
                operit_store::ExtensionStore::ExtensionStore::default()
                    .moveScope(&kind, &id, &scope)
            }
            "mcp" => operit_store::ExtensionStore::ExtensionStore::default()
                .moveScope(&kind, &id, &scope),
            _ => Err(format!("Unknown extension kind: {kind}")),
        }
    }

    /// Creates an input menu bridge backed by this application's tool package runtime.
    #[allow(non_snake_case)]
    pub fn inputMenuToggleBridge(&self) -> ToolPkgInputMenuToggleBridge {
        ToolPkgInputMenuToggleBridge::new(self.toolPkgBridgeRuntime.clone())
    }

    /// Creates a host-composed chat composer slot bridge.
    #[allow(non_snake_case)]
    pub fn chatComposerSlotBridge(&self) -> ToolPkgChatComposerSlotBridge {
        ToolPkgChatComposerSlotBridge::new(self.toolPkgBridgeRuntime.clone())
    }

    /// Creates an independent theme manager over this application's actual ordinary preference host.
    #[allow(non_snake_case)]
    pub fn themeConfigManager(&self) -> Result<ThemeConfigManager, String> {
        let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for theme configuration".to_string()
        })?;
        Ok(ThemeConfigManager::new(storageHost))
    }

    /// Returns the shared package manager owned by the initialized tool handler.
    #[allow(non_snake_case)]
    pub fn packageManager(&self) -> Arc<Mutex<RuntimePackageManager>> {
        self.toolHandler.getOrCreatePackageManager()
    }

    /// Downloads and imports a market artifact entirely inside Core, not through UI byte buffers.
    #[allow(non_snake_case)]
    pub fn installMarketArtifact(
        &self,
        assetId: String,
        fileName: String,
        expectedSha256: String,
    ) -> Result<String, String> {
        let fileName = fileName.trim().to_string();
        if fileName.is_empty()
            || fileName.contains(['/', '\\'])
            || std::path::Path::new(&fileName)
                .file_name()
                .and_then(|name| name.to_str())
                != Some(fileName.as_str())
        {
            return Err("Market artifact file name is invalid".to_string());
        }
        let normalizedSha256 = expectedSha256.trim().to_ascii_lowercase();
        if normalizedSha256.len() != 64
            || !normalizedSha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("Market artifact SHA-256 is invalid".to_string());
        }

        let packageManager = self.packageManager();
        let targetPath = packageManager
            .lock()
            .map_err(|error| error.to_string())?
            .prepareMarketArtifactDownload(&fileName)?;
        let directory = std::path::Path::new(&targetPath)
            .parent()
            .map(|path| path.to_string_lossy().to_string());
        let result = (|| {
            operit_providers::market::MarketStatsApiService::MarketStatsApiService::new()
                .download_asset_to_file(&assetId, &targetPath)?;
            Ok(packageManager
                .lock()
                .map_err(|error| error.to_string())?
                .addMarketArtifactFile(targetPath.clone(), normalizedSha256))
        })();
        if let Some(directory) = directory {
            if let Some(fileSystem) = self.hostManager.fileSystemHost.as_ref() {
                let _ = fileSystem.deleteFile(&directory, true);
            }
        }
        result
    }

    /// Returns marketplace entry/version markers for plugins that are still installed.
    #[allow(non_snake_case)]
    pub fn getInstalledMarketVersions(&self) -> Result<BTreeMap<String, String>, String> {
        super::MarketInstallSupport::installed_versions(self)
    }

    /// Installs a market entry and persists its successfully installed version in Core.
    #[allow(non_snake_case)]
    pub fn installMarketEntry(
        &self,
        entryId: String,
        versionId: Option<String>,
    ) -> Result<String, String> {
        super::MarketInstallSupport::install_entry(self, &entryId, versionId.as_deref())
    }

    /// Returns package names enabled in this application runtime.
    pub fn active_package_names(&self) -> Vec<String> {
        self.toolHandler
            .getOrCreatePackageManager()
            .lock()
            .expect("package manager mutex poisoned")
            .getActivePackageNames()
    }

    /// Tests one model connection using this application's provider runtime.
    pub fn test_model_connection(
        &self,
        provider_id: String,
        model_id: String,
    ) -> impl std::future::Future<Output = Result<ModelConnectionTestReport, String>> + 'static
    {
        let runtime = self.providerRuntimeContext.clone();
        async move {
            ModelConfigManager::default()
                .testModelConnection(&provider_id, &model_id, runtime)
                .await
                .map_err(|error| error.to_string())
        }
    }

    /// Returns the Cargo package version compiled into the runtime crate.
    #[allow(non_snake_case)]
    pub fn coreVersion(&self) -> String {
        crate::CORE_VERSION.to_string()
    }

    /// Returns structured in-memory application log entries.
    #[allow(non_snake_case)]
    pub fn logEntries(&self) -> serde_json::Value {
        AppLogger::entries_json()
    }

    /// Reads the application log file as text.
    #[allow(non_snake_case)]
    pub fn logText(&self) -> Result<String, String> {
        AppLogger::text()
    }

    /// Reads the package-manager log file as text.
    #[allow(non_snake_case)]
    pub fn packageLogText(&self) -> Result<String, String> {
        AppLogger::package_text()
    }

    /// Returns the active application log file path.
    #[allow(non_snake_case)]
    pub fn logFilePath(&self) -> Result<String, String> {
        AppLogger::get_log_file_path()
    }

    /// Returns the active package-manager log file path.
    #[allow(non_snake_case)]
    pub fn packageLogFilePath(&self) -> Result<String, String> {
        AppLogger::get_package_log_file_path()
    }

    /// Returns the user-visible Operit root directory path.
    #[allow(non_snake_case)]
    pub fn operitRootPath(&self) -> Result<String, String> {
        let path = OperitPaths::operitRootDir()?;
        self.ensureHostDirectory(&path)?;
        Ok(path.to_string_lossy().into_owned())
    }

    /// Returns the directory used for exported user artifacts.
    #[allow(non_snake_case)]
    pub fn exportsPath(&self) -> Result<String, String> {
        let path = OperitPaths::exportsDir()?;
        self.ensureHostDirectory(&path)?;
        Ok(path.to_string_lossy().into_owned())
    }

    /// Returns the directory used for files removed during clean-on-exit maintenance.
    #[allow(non_snake_case)]
    pub fn cleanOnExitPath(&self) -> Result<String, String> {
        let path = OperitPaths::cleanOnExitDir()?;
        self.ensureHostDirectory(&path)?;
        Ok(path.to_string_lossy().into_owned())
    }

    /// Returns the directory holding persisted chat attachment files.
    #[allow(non_snake_case)]
    pub fn chatAttachmentsPath(&self) -> Result<String, String> {
        let path = OperitPaths::chatAttachmentsDir()?;
        self.ensureHostDirectory(&path)?;
        Ok(path.to_string_lossy().into_owned())
    }

    /// Clears the current runtime log files.
    #[allow(non_snake_case)]
    pub fn resetLogs(&self) {
        AppLogger::reset_log_file();
    }

    /// Returns the globally registered host manager after startup has completed.
    #[allow(non_snake_case)]
    pub fn hostManager() -> HostManager {
        HOST_MANAGER
            .get_or_init(|| Mutex::new(None))
            .lock()
            .expect("HostManager context mutex poisoned")
            .clone()
            .expect("HostManager context must be initialized")
    }

    /// Advertises exact durable receipts and separately reports unapplied data.
    #[allow(non_snake_case)]
    pub fn syncClock(&self) -> Result<serde_json::Value, String> {
        let storage = self
            .hostManager
            .runtimeStorageHost
            .clone()
            .ok_or("RuntimeStorageHost is not registered for persistent sync")?;
        let mut errors = PreferenceSpaceMerge::prepare(storage);
        if let Err(error) = self
            .runtimeWorkspaceFileSyncStore()
            .and_then(|store| store.scan())
        {
            errors.push(format!("workspace scan: {error}"));
        }
        let store = self.runtimeSyncOperationStore()?;
        errors.extend(store.publishPreferenceOutboxes().map_err(|error| error.to_string())?);
        let schema = PreferenceSpaceMerge::schema();
        let pending_owners = store.pendingOperations().map_err(|e|e.to_string())?;
        let registry = self.transactionRegistry(&schema,&pending_owners)?;
        let (receipt,failures) = registry.receipt()?;
        errors.extend(failures);
        let pending = store.pendingOperationIds().map_err(|error| error.to_string())?;
        let mut blobs = BTreeMap::new();
        store.visitPendingOperations(&[RUNTIME_FILE_SYNC_DOMAIN], |operation| {
            match RuntimeFileSyncStore::requiredBlob(operation) {
                Ok(Some(reference)) => {
                    blobs.insert(reference.contentHash.clone(), reference);
                }
                Ok(None) => {}
                Err(error) => errors.push(format!("pending blob op={}: {error}", operation.opId)),
            }
        }).map_err(|error| error.to_string())?;
        let mut result = serde_json::to_value(receipt).map_err(|error| error.to_string())?;
        result["protocolVersion"] = serde_json::json!(1);
        result["consistencyVersion"] = serde_json::json!(5);
        result["pending"] = serde_json::json!(pending.len());
        result["pendingOperationIds"] = serde_json::json!(pending);
        result["pendingBlobs"] = serde_json::to_value(blobs.into_values().collect::<Vec<_>>())
            .map_err(|error| error.to_string())?;
        result["errors"] = serde_json::json!(errors);
        Ok(result)
    }

    /// Compares the materialized datasets after their ordinary transaction exchange.
    pub fn syncConsistency(&self) -> Result<serde_json::Value, String> {
        self.consistencyManifest()
    }

    pub fn syncReadSnapshot(&self, owner: serde_json::Value, token: String) -> Result<serde_json::Value, String> {
        self.exportConsistencySnapshot(serde_json::from_value(owner).map_err(|e| e.to_string())?, &token)
    }

    pub fn syncReadCoverage(&self, owner: serde_json::Value, token: String) -> Result<serde_json::Value, String> {
        self.exportConsistencyCoverage(serde_json::from_value(owner).map_err(|e| e.to_string())?, &token)
    }

    pub fn syncRepairSnapshot(&self, owner: serde_json::Value, expected: String, reference: serde_json::Value) -> Result<bool, String> {
        self.repairConsistencySnapshot(serde_json::from_value(owner).map_err(|e| e.to_string())?, &expected,
            serde_json::from_value(reference).map_err(|e| e.to_string())?)
    }

    /// Returns a compact change fingerprint of the state a full synchronization
    /// pass would exchange. Peers poll this instead of the full receipt: the
    /// receipt carries one exact range per synced entity, which grows with the
    /// Space and made every idle probe transfer the complete clock.
    ///
    /// Only durable operation clocks are fingerprinted. A storage-handle close
    /// publishes a process mutation without writing any operation, so the
    /// mutation revision cannot distinguish real changes from idle opens.
    /// Only origins this device authored count as a change: applying a peer's
    /// operations advances their ranges locally, but that is not new work to
    /// offer the other members. Device preferences and projected control
    /// operations change no clock, but the device that made them detects them
    /// in its own local-state summary and starts the pass that pushes them to
    /// its peers.
    #[allow(non_snake_case)]
    pub fn syncClockDigest(&self) -> Result<serde_json::Value, String> {
        let store = self.runtimeSyncOperationStore()?;
        let localDeviceId = store
            .localDeviceId()
            .map_err(|error| error.to_string())?;
        let mut clock = store
            .currentLocalClock()
            .map_err(|error| error.to_string())?;
        // Chat operations live in their own SQL journal; merge its clock the
        // same way the local-state summary does so chat-only writes are seen.
        if operit_util::RuntimeStoreRoot::default_runtime_store_root_config().is_some() {
            match SqlChatSyncStore::default() {
                Ok(sqlStore) => match sqlStore.localClock() {
                    Ok(sqlClock) => {
                        for (deviceId, sequence) in sqlClock.sequences {
                            if sequence > clock.sequenceFor(&deviceId) {
                                clock.setSequence(deviceId, sequence);
                            }
                        }
                    }
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::v_with_level(
                            "CoreSyncTrace",
                            &format!("sync_clock_digest.chat_clock_unavailable error={error}"),
                            operit_util::AppLogger::VERBOSE_LEVEL_6,
                        );
                    }
                },
                Err(error) => {
                    operit_util::AppLogger::AppLogger::v_with_level(
                        "CoreSyncTrace",
                        &format!("sync_clock_digest.chat_storage_unavailable error={error}"),
                        operit_util::AppLogger::VERBOSE_LEVEL_6,
                    );
                }
            }
        }
        // Operations applied from peers advance their origins in this clock.
        // Fingerprinting every origin would make each applied batch look like
        // new work to every neighbor, reopening full passes that have nothing
        // to exchange. Advertise only what this device authored: members
        // synchronize with each other over routed links, and projected control
        // operations are advertised through the local-state summary.
        clock.retainAuthoredBy(&localDeviceId);
        let fingerprint = serde_json::to_vec(&clock).map_err(|error| error.to_string())?;
        let digest = Sha256::digest(&fingerprint);
        Ok(serde_json::json!({
            "protocolVersion": 1,
            "digest": format!("{digest:x}"),
        }))
    }

    /// Exports missing operations while reporting independently unavailable storage engines.
    #[allow(non_snake_case)]
    pub fn syncOperationsSince(
        &self, clock: serde_json::Value, domains: Vec<String>, limit: usize,
    ) -> Result<serde_json::Value,String> {
        let include_progress = clock.get("includeProgress").and_then(serde_json::Value::as_bool) == Some(true);
        let receipt: SyncReceipt = serde_json::from_value(clock).map_err(|e|e.to_string())?;
        receipt.validate()?;
        let storage = self.hostManager.runtimeStorageHost.clone().ok_or("Runtime storage is unavailable")?;
        let mut errors = PreferenceSpaceMerge::prepare(storage);
        let journal = self.runtimeSyncOperationStore()?;
        let schema = PreferenceSpaceMerge::schema();
        let pending = journal.pendingOperations().map_err(|e|e.to_string())?;
        let registry = self.transactionRegistry(&schema,&pending)?;
        let (operations,failures) = registry.export(&receipt,&domains,limit)?;
        errors.extend(failures);
        let (remaining,progress_errors) = if include_progress {
            match registry.inventory(&receipt,&domains) {
                Ok((ids,failures)) => ((failures.is_empty() && errors.is_empty()).then_some(ids.len()),failures),
                Err(error) => (None,vec![error]),
            }
        } else { (None,vec![]) };
        let failed_domains = if errors.is_empty() { vec![] } else if domains.is_empty() {
            vec!["preferences".to_owned(),CHAT_SYNC_DOMAIN.into(),PLUGIN_STORAGE_SYNC_DOMAIN.into(),RUNTIME_FILE_SYNC_DOMAIN.into()]
        } else { domains };
        let files = self.runtimeFileSyncStore()?;
        let operations = operations.into_iter().map(|op|files.prepareTransactionForTransport(op)).collect::<Result<Vec<_>,_>>()?;
        Ok(serde_json::json!({"operations":boundSyncOperationPage(operations,DEFAULT_SYNC_PAGE_BYTES),"remaining":remaining,
            "progressErrors":progress_errors,"errors":errors,"failedDomains":failed_domains}))
    }

    /// Durably receives originals and materializes independent owners without acknowledging failed application.
    #[allow(non_snake_case)]
    pub fn syncApplyOperations(&self, operations: serde_json::Value) -> Result<serde_json::Value,String> {
        let operations: Vec<SyncOperation> = serde_json::from_value(operations).map_err(|e|e.to_string())?;
        let files = self.runtimeFileSyncStore()?;
        let operations = operations.into_iter().map(|op|files.hydrateTransaction(op)).collect::<Result<Vec<_>,_>>()?;
        let journal = self.runtimeSyncOperationStore()?;
        let application = journal.lockApplication().map_err(|e|e.to_string())?;
        journal.receiveOperations(&operations).map_err(|e|e.to_string())?;
        let pending = journal.pendingOperations().map_err(|e|e.to_string())?;
        let storage = self.hostManager.runtimeStorageHost.clone().ok_or("Runtime storage is unavailable")?;
        let mut errors = Vec::new();
        if let Err(error) = PreferenceSpaceMerge::prepareLayout(storage.clone()) { errors.push(format!("layout: {error}")); }
        let schema = PreferenceSpaceMerge::schema();
        let registry = self.transactionRegistry(&schema,&pending)?;
        let mut report = registry.apply_pending()?;
        errors.append(&mut report.errors);
        drop(application);
        operit_store::SyncAppliedChanges::publish(&storage,&report.applied);
        let chats = report.applied.iter().filter_map(synchronizedChatId).collect();
        if let Err(error) = self.refreshSynchronizedChatFlows(&chats) { errors.push(format!("chat refresh: {error}")); }
        Ok(serde_json::json!({"received":operations.len(),"applied":report.applied.len(),"skipped":report.skipped,"pending":report.pending.len(),
            "pendingOperationIds":report.pending,"errors":errors}))
    }

    /// Applies one route-transported Binding operation without moving persistent sync clocks.
    #[allow(non_snake_case)]
    pub fn syncApplyImmediateBindingOperation(
        &self,
        operation: SyncOperation,
    ) -> Result<serde_json::Value, String> {
        let bindingStore = self.runtimeBindingStore()?;
        bindingStore.applyImmediateOperation(&operation)?;
        AppLogger::trace(
            "CoreRouteTrace",
            &format!(
                "route_change.binding_installed chatId={} origin={} sequence={}",
                operation.entityId, operation.originDeviceId, operation.sequence
            ),
        );
        Ok(serde_json::json!({ "applied": true }))
    }

    /// Refreshes quiescent opened chat flows after synchronized SQL state is committed.
    #[allow(non_snake_case)]
    fn refreshSynchronizedChatFlows(
        &self,
        chatIds: &std::collections::BTreeSet<String>,
    ) -> Result<(), String> {
        if chatIds.is_empty() {
            return Ok(());
        }
        let store = SqlChatSyncStore::default().map_err(|error| error.to_string())?;
        let mut deletedChatIds = Vec::new();
        let mut liveChatIds = Vec::new();
        for chatId in chatIds {
            if store.chatIsDeleted(chatId).map_err(|error| error.to_string())? {
                deletedChatIds.push(chatId.clone());
            } else {
                liveChatIds.push(chatId.clone());
            }
        }
        if !deletedChatIds.is_empty() {
            self.dispatchSynchronizedChatDeletions(deletedChatIds)?;
        }
        if liveChatIds.is_empty() {
            return Ok(());
        }
        let mut holder = self.chatRuntimeHolder.try_lock().map_err(|_| {
            "chat runtime holder is busy during synchronized chat refresh".to_string()
        })?;
        let mut refreshed = 0usize;
        for (slot, core) in holder.cores.iter_mut() {
            let activeChatIds = core.activeStreamingChatIds();
            AppLogger::trace(
                "CoreSyncTrace",
                &format!(
                    "sync_apply.chat_flow_refresh_core slot={:?} activeChatIds={}",
                    slot,
                    activeChatIds.join(",")
                ),
            );
            for chatId in &liveChatIds {
                let flowSnapshot = core
                    .chatHistoryDelegate
                    .chatMessageFlowsByChatId
                    .lock()
                    .expect("chat message flow registry mutex must not be poisoned")
                    .get(chatId)
                    .cloned()
                    .map(|flow| flow.value());
                let flowSnapshotMessages = flowSnapshot.as_deref().unwrap_or(&[]);
                let streamMessages = flowSnapshotMessages
                    .iter()
                    .filter(|message| message.contentStream.is_some())
                    .count();
                let lastMessage = flowSnapshotMessages.last().map(|message| {
                    format!(
                        "ts={} sender={} chars={} stream={}",
                        message.timestamp,
                        message.sender,
                        message.displayText().chars().count(),
                        message
                            .contentStream
                            .as_ref()
                            .map(|stream| stream.descriptor.streamId.as_str())
                            .unwrap_or("none")
                    )
                });
                if activeChatIds
                    .iter()
                    .any(|activeChatId| activeChatId == chatId)
                {
                    AppLogger::trace(
                        "CoreSyncTrace",
                        &format!(
                            "sync_apply.chat_flow_skip slot={:?} chatId={} reason=active_local_stream flowOpened={} snapshotMessages={} streamMessages={} last={}",
                            slot,
                            chatId,
                            flowSnapshot.is_some(),
                            flowSnapshotMessages.len(),
                            streamMessages,
                            lastMessage.as_deref().unwrap_or("none")
                        ),
                    );
                    continue;
                }
                AppLogger::trace(
                    "CoreSyncTrace",
                    &format!(
                        "sync_apply.chat_flow_refresh slot={:?} chatId={} activeLocal=false flowOpened={} snapshotMessages={} streamMessages={} last={}",
                        slot,
                        chatId,
                        flowSnapshot.is_some(),
                        flowSnapshotMessages.len(),
                        streamMessages,
                        lastMessage.as_deref().unwrap_or("none")
                    ),
                );
                core.chatHistoryDelegate
                    .refreshChatMessagesFromPersistence(chatId.clone());
                refreshed += 1;
            }
        }
        AppLogger::trace(
            "CoreSyncTrace",
            &format!(
                "sync_apply.chat_flow_refresh chatCount={} coreFlowRefreshCount={}",
                chatIds.len(),
                refreshed
            ),
        );
        Ok(())
    }

    /// Applies committed chat deletions once the shared runtime is available, including active streams.
    fn dispatchSynchronizedChatDeletions(&self, chatIds: Vec<String>) -> Result<(), String> {
        let scheduler = self.hostManager.hostRuntimeTaskSchedulerHost.clone().ok_or_else(|| {
            "runtime task scheduler host must be configured for synchronized chat deletion".to_string()
        })?;
        let holder = self.chatRuntimeHolder.clone();
        scheduler.scheduleHostRuntimeAsyncTask("operit-chat-sync-deletion", Box::new(move || {
            Box::pin(async move {
                let mut holder = holder.lock().await;
                for core in holder.cores.values_mut() {
                    for chatId in &chatIds {
                        if let Err(error) = core.applySynchronizedChatDeletion(chatId.clone()).await {
                            AppLogger::e("CoreSyncTrace", &format!(
                                "sync_apply.chat_deletion_cleanup chatId={chatId}: {error}"
                            ));
                        }
                    }
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>
        })).map_err(|error| error.to_string())
    }

    /// Reports whether one complete verified synchronization blob exists locally.
    #[allow(non_snake_case)]
    pub fn syncBlobExists(&self, contentHash: String, size: i64) -> Result<bool, String> {
        self.runtimeFileSyncStore()?
            .hasBlob(&RuntimeFileSyncReference { contentHash, size })
    }

    /// Reads one bounded synchronization blob chunk for transfer to another CoreNode.
    #[allow(non_snake_case)]
    pub fn syncReadBlobChunk(
        &self,
        contentHash: String,
        offset: i64,
        length: i64,
    ) -> Result<Vec<u8>, String> {
        self.runtimeFileSyncStore()?
            .readBlobChunk(&contentHash, offset, length)
    }

    /// Builds a validated creation payload without changing installed packages or appearance.
    #[allow(non_snake_case)]
    fn previewThemeArchive(
        &self,
        archiveBytes: Vec<u8>,
        themeId: String,
    ) -> Result<ThemePreviewReceipt, String> {
        let package = self.validateThemeArchive(&archiveBytes, &themeId)?;
        let themeJson = String::from_utf8(package.theme().canonical_bytes())
            .map_err(|error| error.to_string())?;
        let assets = package
            .receipt()
            .files
            .iter()
            .filter(|file| file.path == "preview.png" || file.path.starts_with("assets/"))
            .map(|file| {
                let bytes = package
                    .file_bytes(&file.path)
                    .ok_or_else(|| format!("validated preview asset disappeared: {}", file.path))?;
                Ok(ThemePreviewAsset {
                    path: file.path.clone(),
                    bytes: bytes.to_vec(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(ThemePreviewReceipt {
            validation: package.receipt().clone(),
            theme_id: package.metadata().theme_id.clone(),
            version: package.metadata().version.clone(),
            theme_json: themeJson,
            assets,
        })
    }

    /// Prepares a declared template from the current ToolPkg registry without persisting a selection.
    #[allow(non_snake_case)]
    pub fn previewToolPkgTheme(
        &self,
        packageName: String,
        templateId: String,
    ) -> Result<ThemePreviewReceipt, String> {
        let bytes = {
            let manager = self.packageManager();
            let manager = manager.lock().map_err(|error| error.to_string())?;
            let runtime = manager.getToolPkgContainerRuntime(&packageName)
                .ok_or("ToolPkg is not registered")?;
            if !runtime.themes.iter().any(|template| template.id == templateId) {
                return Err("Theme template is not declared by this ToolPkg".to_string());
            }
            manager.readToolPkgArchiveBytes(&packageName)?
        };
        self.previewThemeArchive(bytes, format!("{packageName}:{templateId}"))
    }

    /// Validates an explicitly declared template using the ordinary ToolPkg parser.
    #[allow(non_snake_case)]
    fn validateThemeArchive(
        &self,
        archiveBytes: &[u8],
        themeId: &str,
    ) -> Result<operit_plugin_theme::ValidatedThemeTemplate, String> {
        operit_plugin_sdk::toolpkg::ToolPkgParser::ToolPkgArchiveParser::readThemeTemplates(archiveBytes)?
            .into_iter().find(|template| template.metadata().theme_id == themeId)
            .ok_or_else(|| "Selected theme template is not declared by this ToolPkg".to_string())
    }

    /// Creates the persistent sync operation store owned by this application instance.
    #[allow(non_snake_case)]
    pub(super) fn runtimeSyncOperationStore(&self) -> Result<SyncOperationStore, String> {
        let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for persistent sync".to_string()
        })?;
        Ok(SyncOperationStore::new(
            storageHost,
            RUNTIME_SYNC_DIR_PATH.to_string(),
        ))
    }

    /// Creates the opaque Binding store owned by this application instance.
    #[allow(non_snake_case)]
    pub(super) fn runtimeBindingStore(&self) -> Result<CoreNodeBindingStore, String> {
        let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for persistent Binding storage".to_string()
        })?;
        CoreNodeBindingStore::new(storageHost)
    }

    /// Creates the synchronized Space authorization and control store owned by this application.
    #[allow(non_snake_case)]
    pub(super) fn runtimeNetworkControlStore(&self) -> Result<NetworkControlStore, String> {
        let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for network control storage".to_string()
        })?;
        NetworkControlStore::new(storageHost)
    }

    pub(super) fn runtimeWorkspaceFileSyncStore(
        &self,
    ) -> Result<operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore, String> {
        let host = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for workspace synchronization".to_string()
        })?;
        Ok(
            operit_store::WorkspaceFileSyncStore::WorkspaceFileSyncStore::new(
                host,
                RUNTIME_SYNC_DIR_PATH,
            ),
        )
    }

    /// Creates the content-addressed runtime file store owned by this application instance.
    #[allow(non_snake_case)]
    pub(super) fn runtimeFileSyncStore(&self) -> Result<RuntimeFileSyncStore, String> {
        let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for runtime file synchronization".to_string()
        })?;
        Ok(RuntimeFileSyncStore::new(
            storageHost,
            RUNTIME_SYNC_DIR_PATH.to_string(),
        ))
    }

    /// Requires the existing runtime SQLite Host for plugin storage synchronization.
    #[allow(non_snake_case)]
    pub(super) fn pluginSqliteHost(
        &self,
    ) -> Result<std::sync::Arc<dyn operit_host_api::RuntimeSqliteHost>, String> {
        self.hostManager
            .runtimeSqliteHost
            .clone()
            .ok_or_else(|| "RuntimeSqliteHost is not registered for plugin storage".to_string())
    }

    /// Applies one non-preferences persistent operation to its owning domain.
    #[allow(non_snake_case)]
    pub(super) fn applyNonPreferenceSyncOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        match (
            operation.domain.as_str(),
            operation.entityType.as_str(),
            operation.operation.as_str(),
        ) {
            (RUNTIME_FILE_SYNC_DOMAIN, "file", "upsert" | "delete") => {
                let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
                    "RuntimeStorageHost is not registered for persistent sync".to_string()
                })?;
                RuntimeFileSyncStore::applySyncedOperation(
                    storageHost,
                    RUNTIME_SYNC_DIR_PATH.to_string(),
                    &operation.entityId,
                    &operation.operation,
                    operation.payload.clone(),
                )
            }
            (domain, entityType, operationName) => Err(format!(
                "unsupported sync operation: {domain}/{entityType}/{operationName}"
            )),
        }
    }
}

/// Stores the host manager for code paths that need process-wide access.
#[allow(non_snake_case)]
fn setHostManager(hostManager: HostManager) {
    *HOST_MANAGER
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("HostManager context mutex poisoned") = Some(hostManager);
}

/// Resolves each owner's declared durable commit boundary for ordered failure isolation.
pub(super) fn synchronizationDataset(operation: &SyncOperation) -> Result<String, String> {
    match operation.domain.as_str() {
        CHAT_SYNC_DOMAIN => synchronizedChatId(operation).ok_or_else(|| "Chat transaction dataset is missing".into()),
        PLUGIN_STORAGE_SYNC_DOMAIN => PluginStorage::syncDatasetPath(operation).map_err(|error| error.to_string()),
        NETWORK_CONTROL_SYNC_DOMAIN => operation.payload.get("spaceId").and_then(serde_json::Value::as_str)
            .map(str::to_owned).ok_or_else(|| "Network control Space identity is missing".into()),
        _ => serde_json::to_string(&(&operation.entityType, &operation.entityId)).map_err(|error| error.to_string()),
    }
}

/// Extracts the chat id addressed by one SQL synchronization operation.
fn synchronizedChatId(operation: &SyncOperation) -> Option<String> {
    if operation.domain != CHAT_SYNC_DOMAIN {
        return None;
    }
    match operation.entityType.as_str() {
        "chat" | "messages" => Some(operation.entityId.clone()),
        "message" => operation
            .entityId
            .split_once(':')
            .map(|(chatId, _)| chatId.to_string()),
        _ => None,
    }
}

impl Default for OperitApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod theme_preview_wire_tests {
    use super::ThemePreviewAsset;
    use operit_link::{toCoreValue, CoreValue};

    #[test]
    fn preview_asset_bytes_use_messagepack_binary() {
        let value = toCoreValue(&ThemePreviewAsset {
            path: "assets/images/example.png".to_string(),
            bytes: vec![0, 127, 255],
        })
        .expect("theme preview asset must convert to a CoreValue map");
        let CoreValue::Map(fields) = value else {
            panic!("theme preview asset must convert to a CoreValue map");
        };
        assert_eq!(fields.get("bytes"), Some(&CoreValue::Bytes(vec![0, 127, 255].into())));
    }
}
