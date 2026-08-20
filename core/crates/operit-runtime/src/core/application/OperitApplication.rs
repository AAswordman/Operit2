use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use crate::core::events::RuntimeEvent::RuntimeEvent;
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::plugins::toolpkg::ToolPkgAppLifecycleHookBridge::ToolPkgAppLifecycleHookBridge;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::ToolPkgBridgeRuntime;
use crate::plugins::toolpkg::ToolPkgInputMenuToggleBridge::ToolPkgInputMenuToggleBridge;
use crate::plugins::PluginRegistry::PluginRegistry;
use crate::services::ProviderRuntimeSupportService::ProviderRuntimeSupportService;
use crate::services::ToolRuntimeSupportService::ToolRuntimeSupportService;
#[cfg(feature = "javascript")]
use operit_host_api::HostManager::setDefaultHostJavaScriptRuntimeHost;
use operit_host_api::HostManager::{
    setDefaultHostRuntimeTaskSchedulerHost, setDefaultHttpHost, setDefaultWebSocketHost,
    HostManager,
};
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{HostRuntimeEventRegistration, HostRuntimeTaskSchedulerHost};
#[cfg(feature = "javascript")]
use operit_js_bridge::javascript::JsExecutionProvider::QuickJsExecutionProvider;
use operit_model::Memory::{Memory, MemoryLink};
use operit_providers::chat::llmprovider::ModelConfigConnectionTester::ModelConnectionTestReport;
use operit_providers::runtime_support::ProviderRuntimeContext;
use operit_store::repository::UserMarkdownRepository::UserMarkdownRepository;
use operit_store::sync::SqlChatSyncStore::{SqlChatSyncStore, CHAT_SYNC_DOMAIN};
use operit_store::CoreNodeBindingStore::{CoreNodeBindingStore, BINDING_SYNC_DOMAIN};
use operit_store::ObjectBoxStore::{ObjectBox, OBJECTBOX_SYNC_DOMAIN};
use operit_store::PreferencesDataStore::StateFlow;
use operit_store::PreferencesDataStore::{PreferencesDataStore, PreferencesSyncedEntry};
use operit_store::RuntimeFileSyncStore::{
    RuntimeFileSyncReference, RuntimeFileSyncStore, RUNTIME_FILE_SYNC_DOMAIN,
};
use operit_store::RuntimeStorageHost::{
    defaultRuntimeStorageHost, setDefaultHostSecretStore, setDefaultRuntimeSqliteHost,
    setDefaultRuntimeStorageHost,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::SyncOperationStore::{
    compactSyncOperations, SyncClock, SyncOperation, SyncOperationStore,
};
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::files::VisualFileSystem::VisualFileSystem;
use operit_tools::runtime_support::ToolRuntimeDependencies;
use operit_tools::tools::mcp_runtime::plugins::MCPStarter::MCPStarter;
use operit_tools::tools::mcp_runtime::MCPRepository::MCPRepository;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_util::RuntimeStorageLayout::RUNTIME_SYNC_DIR_PATH;
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as AsyncMutex;

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
            let pathMapper = PathMapper::new(runtimeRoot.clone(), workspaceRoot.clone());
            let logFile = pathMapper
                .resolve("/app/data/logs/operit.log")
                .expect("runtime log path must resolve through the file-system host")
                .physicalPath;
            let packageLogFile = pathMapper
                .resolve("/app/data/logs/toolpkg.log")
                .expect("ToolPkg log path must resolve through the file-system host")
                .physicalPath;
            AppLogger::configure_log_files(fileSystemHost, logFile, packageLogFile)
                .expect("runtime log files must be configured through the file-system host");
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
            Arc::new(QuickJsExecutionProvider::new()),
        );
        let toolHandler = AIToolHandler::new(hostManager.clone(), toolRuntimeDependencies.clone());
        let toolPkgBridgeRuntime =
            ToolPkgBridgeRuntime::new(toolHandler.clone(), hostManager.clone());
        let providerRuntimeContext = ProviderRuntimeSupportService::create(toolHandler.clone());
        runtimeToolSupport
            .bindRuntimeServices(toolHandler.clone(), providerRuntimeContext.clone())
            .expect("tool runtime services must bind exactly once");
        *chatRuntimeHolder
            .try_lock()
            .expect("new chat runtime holder must be unlocked") =
            ChatRuntimeHolder::newWithRuntimeDependencies(
                chatFileSystemHost,
                toolHandler.clone(),
                providerRuntimeContext.clone(),
            );
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
            PathMapper::new(runtimeRoot, workspaceRoot),
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
        AppLogger::i("OperitApplication", "runtime initialization start");
        setHostManager(self.hostManager.clone());
        self.configureOpenMpEnvironment();
        self.cleanOnExitFiles()?;
        self.ensureWorkManagerInitialized();
        AIMessageManager::initialize();
        self.initializeJsonSerializer();
        self.initializeAppLanguage();
        self.initAndroidPermissionPreferences();
        self.preloadDatabase();
        let mut toolHandler = self.toolHandler.clone();
        let toolRegistrationStartedAt = currentTimeMillis();
        AppLogger::i("OperitApplication", "default tool registration start");
        toolHandler.registerDefaultTools();
        AppLogger::i(
            "OperitApplication",
            &format!(
                "default tool registration done elapsedMs={}",
                currentTimeMillis() - toolRegistrationStartedAt
            ),
        );
        let pluginInitializationStartedAt = currentTimeMillis();
        AppLogger::i("OperitApplication", "built-in plugin initialization start");
        PluginRegistry::initializeBuiltins(self.toolPkgBridgeRuntime.clone());
        AppLogger::i(
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
        AppLogger::i("OperitApplication", "host runtime event registration start");
        self.hostRuntimeEventRegistration =
            crate::services::RuntimeEventIngressService::RuntimeEventIngressService::startHostRuntimeEventSupport(
                self.hostManager.clone(),
                self.toolPkgBridgeRuntime.clone(),
            )?
            .map(|registration| Arc::new(Mutex::new(registration)));
        AppLogger::i(
            "OperitApplication",
            &format!(
                "host runtime event registration done elapsedMs={}",
                currentTimeMillis() - runtimeEventRegistrationStartedAt
            ),
        );
        self.initialized = true;
        self.initMcpPlugins();
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

    /// Starts deployed MCP plugins according to the configured startup timeout.
    #[allow(non_snake_case)]
    pub fn initMcpPlugins(&self) {
        let hostManager = self.hostManager.clone();
        let runtimeSupport = self.toolHandler.runtimeSupport();
        let taskScheduler = self
            .hostManager
            .hostRuntimeTaskSchedulerHost
            .clone()
            .expect("runtime task scheduler host must be configured for MCP startup");
        let startup = move || {
            let starter = MCPStarter::new(hostManager, runtimeSupport);
            let timeoutSeconds = ApiPreferences::getInstance()
                .getMcpStartupTimeoutSeconds()
                .expect("api preferences must provide mcp startup timeout seconds");
            let _ = starter.startAllDeployedPluginsWithTimeout(timeoutSeconds);
        };
        taskScheduler
            .scheduleHostRuntimeTask("operit-mcp-startup", Box::new(startup))
            .expect("MCP startup task must be scheduled");
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

    /// Creates a user-markdown repository using this runtime's configured storage host.
    #[allow(non_snake_case)]
    pub fn userMarkdownRepository(&self, ownerKey: String) -> UserMarkdownRepository {
        UserMarkdownRepository::new(ownerKey, defaultRuntimeStorageHost())
    }

    /// Creates an input menu bridge backed by this application's tool package runtime.
    #[allow(non_snake_case)]
    pub fn inputMenuToggleBridge(&self) -> ToolPkgInputMenuToggleBridge {
        ToolPkgInputMenuToggleBridge::new(self.toolPkgBridgeRuntime.clone())
    }

    /// Returns the shared package manager owned by the initialized tool handler.
    #[allow(non_snake_case)]
    pub fn packageManager(&self) -> Arc<Mutex<RuntimePackageManager>> {
        self.toolHandler.getOrCreatePackageManager()
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
    pub async fn test_model_connection(
        &self,
        provider_id: String,
        model_id: String,
    ) -> Result<ModelConnectionTestReport, String> {
        ModelConfigManager::default()
            .testModelConnection(&provider_id, &model_id, self.providerRuntimeContext.clone())
            .await
            .map_err(|error| error.to_string())
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

    /// Combines sync clocks from key-value/object stores and SQL chat storage.
    #[allow(non_snake_case)]
    pub fn syncClock(&self) -> Result<serde_json::Value, String> {
        let store = self.runtimeSyncOperationStore()?;
        let mut clock = store.localClock().map_err(|error| error.to_string())?;
        let sqlStore = SqlChatSyncStore::default().map_err(|error| error.to_string())?;
        mergeSyncClock(
            &mut clock,
            sqlStore.localClock().map_err(|error| error.to_string())?,
        );
        serde_json::to_value(clock).map_err(|error| error.to_string())
    }

    /// Lists compacted sync operations newer than the provided device clock.
    #[allow(non_snake_case)]
    pub fn syncOperationsSince(
        &self,
        clock: serde_json::Value,
        domains: Vec<String>,
        limit: usize,
    ) -> Result<serde_json::Value, String> {
        let clock: SyncClock = serde_json::from_value(clock).map_err(|error| error.to_string())?;
        let store = self.runtimeSyncOperationStore()?;
        let mut operations = store
            .operationsSince(&clock, &domains, limit)
            .map_err(|error| error.to_string())?;
        let sqlStore = SqlChatSyncStore::default().map_err(|error| error.to_string())?;
        operations.extend(
            sqlStore
                .operationsSince(&clock, &domains, limit)
                .map_err(|error| error.to_string())?,
        );
        operations.sort_by(|left, right| {
            left.createdAt
                .cmp(&right.createdAt)
                .then(left.originDeviceId.cmp(&right.originDeviceId))
                .then(left.sequence.cmp(&right.sequence))
        });
        operations = compactSyncOperations(operations);
        operations.truncate(limit);
        serde_json::to_value(operations).map_err(|error| error.to_string())
    }

    /// Applies incoming sync operations to their owning persistent stores.
    #[allow(non_snake_case)]
    pub fn syncApplyOperations(
        &self,
        operations: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let forceApply = operations
            .get("forceApply")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let operationsValue = if forceApply {
            operations
                .get("operations")
                .cloned()
                .ok_or_else(|| "bootstrap sync request is missing operations".to_string())?
        } else {
            operations
        };
        let mut operations: Vec<SyncOperation> =
            serde_json::from_value(operationsValue).map_err(|error| error.to_string())?;
        let store = self.runtimeSyncOperationStore()?;
        let sqlStore = SqlChatSyncStore::default().map_err(|error| error.to_string())?;
        if forceApply && operations.is_empty() {
            store
                .markLocalOperationsUnexportable()
                .map_err(|error| error.to_string())?;
            sqlStore
                .markLocalOperationsUnexportable()
                .map_err(|error| error.to_string())?;
            return Ok(serde_json::json!({ "applied": 0 }));
        }
        if forceApply {
            operations.sort_by(|left, right| {
                left.createdAt
                    .cmp(&right.createdAt)
                    .then(left.originDeviceId.cmp(&right.originDeviceId))
                    .then(left.sequence.cmp(&right.sequence))
            });
        } else {
            operations.sort_by(|left, right| {
                left.originDeviceId
                    .cmp(&right.originDeviceId)
                    .then(left.sequence.cmp(&right.sequence))
            });
        }
        let bindingStore = self.runtimeBindingStore()?;
        let mut applied = 0usize;
        let mut syncClock = store.localClock().map_err(|error| error.to_string())?;
        let mut persistentOperations = Vec::new();
        for operation in operations {
            if operation.domain == CHAT_SYNC_DOMAIN {
                if forceApply {
                    sqlStore
                        .applyBootstrapOperation(&operation)
                        .map_err(|error| error.to_string())?;
                } else {
                    sqlStore
                        .applyOperation(&operation)
                        .map_err(|error| error.to_string())?;
                }
            } else if operation.domain == BINDING_SYNC_DOMAIN {
                if forceApply {
                    bindingStore.applyBootstrapOperation(&operation)?;
                } else {
                    bindingStore.applySyncedOperation(&operation)?;
                }
            } else {
                if !forceApply
                    && operation.sequence <= syncClock.sequenceFor(&operation.originDeviceId)
                {
                    continue;
                }
                syncClock.setSequence(operation.originDeviceId.clone(), operation.sequence);
                persistentOperations.push(operation);
            }
            applied += 1;
        }

        let applyDecisions = if forceApply {
            vec![true; persistentOperations.len()]
        } else {
            store
                .shouldApplyOperations(&persistentOperations)
                .map_err(|error| error.to_string())?
        };
        let mut appliedPersistentOperations = Vec::new();
        let mut preferenceEntriesByPath = BTreeMap::<String, Vec<PreferencesSyncedEntry>>::new();
        let mut nonPreferenceOperations = Vec::new();
        for (operation, shouldApply) in persistentOperations.iter().zip(applyDecisions) {
            if !shouldApply {
                continue;
            }
            if operation.domain == "preferences" {
                let entry = PreferencesSyncedEntry::fromOperation(operation)
                    .map_err(|error| error.to_string())?;
                preferenceEntriesByPath
                    .entry(entry.storagePath().to_string())
                    .or_default()
                    .push(entry);
            } else {
                nonPreferenceOperations.push(operation);
            }
            appliedPersistentOperations.push(operation.clone());
        }
        for operation in nonPreferenceOperations {
            self.applyNonPreferenceSyncOperation(operation)?;
        }
        if !preferenceEntriesByPath.is_empty() {
            let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
                "RuntimeStorageHost is not registered for persistent sync".to_string()
            })?;
            for entries in preferenceEntriesByPath.values() {
                PreferencesDataStore::applySyncedEntriesWithStorage(storageHost.clone(), entries)
                    .map_err(|error| error.to_string())?;
            }
        }
        if forceApply {
            store
                .recordBootstrapAppliedOperations(&appliedPersistentOperations)
                .map_err(|error| error.to_string())?;
        } else {
            store
                .recordAppliedOperations(&appliedPersistentOperations)
                .map_err(|error| error.to_string())?;
        }
        store
            .appendOperations(&persistentOperations)
            .map_err(|error| error.to_string())?;
        Ok(serde_json::json!({ "applied": applied }))
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

    /// Creates the persistent sync operation store owned by this application instance.
    #[allow(non_snake_case)]
    fn runtimeSyncOperationStore(&self) -> Result<SyncOperationStore, String> {
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
    fn runtimeBindingStore(&self) -> Result<CoreNodeBindingStore, String> {
        let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for persistent Binding storage".to_string()
        })?;
        CoreNodeBindingStore::new(storageHost)
    }

    /// Creates the content-addressed runtime file store owned by this application instance.
    #[allow(non_snake_case)]
    fn runtimeFileSyncStore(&self) -> Result<RuntimeFileSyncStore, String> {
        let storageHost = self.hostManager.runtimeStorageHost.clone().ok_or_else(|| {
            "RuntimeStorageHost is not registered for runtime file synchronization".to_string()
        })?;
        Ok(RuntimeFileSyncStore::new(
            storageHost,
            RUNTIME_SYNC_DIR_PATH.to_string(),
        ))
    }

    /// Applies one non-preferences persistent operation to its owning domain.
    #[allow(non_snake_case)]
    fn applyNonPreferenceSyncOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        match (
            operation.domain.as_str(),
            operation.entityType.as_str(),
            operation.operation.as_str(),
        ) {
            (OBJECTBOX_SYNC_DOMAIN, "Memory", "upsert" | "delete") => {
                ObjectBox::<Memory>::applySyncedEntity(
                    &operation.entityId,
                    &operation.operation,
                    operation.payload.clone(),
                )
                .map_err(|error| error.to_string())
            }
            (OBJECTBOX_SYNC_DOMAIN, "MemoryLink", "upsert" | "delete") => {
                ObjectBox::<MemoryLink>::applySyncedEntity(
                    &operation.entityId,
                    &operation.operation,
                    operation.payload.clone(),
                )
                .map_err(|error| error.to_string())
            }
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

impl Default for OperitApplication {
    fn default() -> Self {
        Self::new()
    }
}

/// Merges source device sequence positions into the target clock.
fn mergeSyncClock(target: &mut SyncClock, source: SyncClock) {
    for (deviceId, sequence) in source.sequences {
        if sequence > target.sequenceFor(&deviceId) {
            target.setSequence(deviceId, sequence);
        }
    }
}
