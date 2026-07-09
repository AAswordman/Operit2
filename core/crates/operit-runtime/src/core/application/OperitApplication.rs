use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
#[cfg(not(target_arch = "wasm32"))]
use crate::data::backup::Operit1SnapshotImportManager::{
    observeOperit1SnapshotImportProgress, publishOperit1SnapshotImportProgress,
    Operit1ModelConfigImportResult, Operit1ModelConfigSnapshotPreview,
    Operit1SnapshotImportManager, Operit1SnapshotImportProgress, Operit1SnapshotImportResult,
    Operit1SnapshotPreview,
};
use crate::data::backup::RawSnapshotBackupManager::{
    RawSnapshotBackupManager, RawSnapshotManifest,
};
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::data::preferences::UserPreferencesManager::UserPreferencesManager;
use crate::plugins::PluginRegistry::PluginRegistry;
use crate::services::ProviderRuntimeSupportService::ProviderRuntimeSupportService;
use crate::services::ToolRuntimeSupportService::ToolRuntimeSupportService;
use operit_host_api::HostManager::{setDefaultHttpHost, HostManager};
use operit_host_api::HostRuntimeEventRegistration;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_js_bridge::javascript::JsBridgeSupportService::JsBridgeSupportService;
use operit_model::Memory::{Memory, MemoryLink};
use operit_store::db::AppDatabase::AppDatabase;
use operit_store::sync::SqlChatSyncStore::{SqlChatSyncStore, CHAT_SYNC_DOMAIN};
use operit_store::ObjectBoxStore::{ObjectBox, OBJECTBOX_SYNC_DOMAIN};
use operit_store::PreferencesDataStore::PreferencesDataStore;
use operit_store::PreferencesDataStore::StateFlow;
use operit_store::RuntimeStorageHost::{
    defaultRuntimeStorageHost, setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::SyncOperationStore::{
    compactSyncOperations, SyncClock, SyncOperation, SyncOperationStore,
};
use operit_tools::tools::mcp_runtime::plugins::MCPStarter::MCPStarter;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_util::RuntimeStoreRoot::setDefaultRuntimeStoreRoot;
use std::fs;
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
    pub initialized: bool,
    hostRuntimeEventRegistration: Option<Box<dyn HostRuntimeEventRegistration>>,
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
            if let Some(rootDir) = runtimeStorageHost.rootDir() {
                AppLogger::configure_log_files(&rootDir);
                setDefaultRuntimeStoreRoot(rootDir);
            }
            setDefaultRuntimeStorageHost(runtimeStorageHost);
        }
        if let Some(runtimeSqliteHost) = hostManager.runtimeSqliteHost.clone() {
            setDefaultRuntimeSqliteHost(runtimeSqliteHost);
        }
        if let Some(httpHost) = hostManager.httpHost.clone() {
            setDefaultHttpHost(httpHost);
        }
        Self {
            appStartupTimeMs: 0,
            hostManager,
            chatRuntimeHolder: Arc::new(AsyncMutex::new(ChatRuntimeHolder::new())),
            initialized: false,
            hostRuntimeEventRegistration: None,
        }
    }

    /// Initializes persistent stores, prompt managers, tool handlers, plugins, and runtime events.
    #[allow(non_snake_case)]
    pub fn onCreate(&mut self) -> Result<(), String> {
        self.appStartupTimeMs = currentTimeMillis();
        setHostManager(self.hostManager.clone());
        JsBridgeSupportService::install()?;
        ProviderRuntimeSupportService::install()?;
        ToolRuntimeSupportService::install(self.chatRuntimeHolder.clone())?;
        self.configureOpenMpEnvironment();
        OperitPaths::cleanOnExitCleanup()?;
        self.ensureWorkManagerInitialized();
        AIMessageManager::initialize();
        self.initializeJsonSerializer();
        self.initializeAppLanguage();
        self.initUserPreferencesManager()?;
        self.initAndroidPermissionPreferences();
        self.initializeFunctionalPromptManager()?;
        self.preloadDatabase();
        let mut toolHandler = AIToolHandler::getInstance(self.hostManager.clone());
        toolHandler.registerDefaultTools();
        PluginRegistry::initializeBuiltins();
        self.initMcpPlugins();
        *self
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy during application startup".to_string())? =
            ChatRuntimeHolder::newWithHostManager(self.hostManager.clone());
        self.hostRuntimeEventRegistration = crate::services::RuntimeEventIngressService::RuntimeEventIngressService::startHostRuntimeEventSupport(
            self.hostManager.clone(),
        )?;
        self.initialized = true;
        Ok(())
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

    /// Prepares model, functional, and user preference stores for runtime access.
    #[allow(non_snake_case)]
    pub fn initUserPreferencesManager(&self) -> Result<(), String> {
        ModelConfigManager::default()
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        FunctionalConfigManager::default()
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        UserPreferencesManager::getInstance()
            .initializeIfNeeded("Default")
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Initializes platform permission preference state used by Android-facing tools.
    #[allow(non_snake_case)]
    pub fn initAndroidPermissionPreferences(&self) {}

    /// Loads character and functional prompt data required by chat sessions.
    #[allow(non_snake_case)]
    pub fn initializeFunctionalPromptManager(&self) -> Result<(), String> {
        CharacterCardManager::getInstance()
            .initializeIfNeeded()
            .map_err(|error| error.to_string())
    }

    /// Touches database-backed services early so schema setup happens during startup.
    #[allow(non_snake_case)]
    pub fn preloadDatabase(&self) {}

    /// Starts deployed MCP plugins according to the configured startup timeout.
    #[allow(non_snake_case)]
    pub fn initMcpPlugins(&self) {
        let starter = MCPStarter::new(self.hostManager.clone());
        let timeoutSeconds = ApiPreferences::getInstance()
            .getMcpStartupTimeoutSeconds()
            .expect("api preferences must provide mcp startup timeout seconds");
        let _ = starter.startAllDeployedPluginsWithTimeout(timeoutSeconds);
    }

    /// Returns the Cargo package version compiled into the runtime crate.
    #[allow(non_snake_case)]
    pub fn coreVersion(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
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
        OperitPaths::operitRootPathSdcard()
    }

    /// Returns the directory used for exported user artifacts.
    #[allow(non_snake_case)]
    pub fn exportsPath(&self) -> Result<String, String> {
        OperitPaths::exportsPathSdcard()
    }

    /// Returns the directory used for files removed during clean-on-exit maintenance.
    #[allow(non_snake_case)]
    pub fn cleanOnExitPath(&self) -> Result<String, String> {
        OperitPaths::cleanOnExitPathSdcard()
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
        let store = SyncOperationStore::native(RuntimeStorePaths::default());
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
        let store = SyncOperationStore::native(RuntimeStorePaths::default());
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
        let mut operations: Vec<SyncOperation> =
            serde_json::from_value(operations).map_err(|error| error.to_string())?;
        operations.sort_by(|left, right| {
            left.originDeviceId
                .cmp(&right.originDeviceId)
                .then(left.sequence.cmp(&right.sequence))
        });
        let store = SyncOperationStore::native(RuntimeStorePaths::default());
        let sqlStore = SqlChatSyncStore::default().map_err(|error| error.to_string())?;
        let mut applied = 0usize;
        for operation in operations {
            if operation.domain == CHAT_SYNC_DOMAIN {
                sqlStore
                    .applyOperation(&operation)
                    .map_err(|error| error.to_string())?;
            } else {
                let clock = store.localClock().map_err(|error| error.to_string())?;
                if operation.sequence <= clock.sequenceFor(&operation.originDeviceId) {
                    continue;
                }
                self.applySyncOperation(&operation)?;
                store
                    .appendOperation(&operation)
                    .map_err(|error| error.to_string())?;
            }
            applied += 1;
        }
        Ok(serde_json::json!({ "applied": applied }))
    }

    /// Exports all raw runtime storage into a portable snapshot archive.
    #[allow(non_snake_case)]
    pub fn exportRawSnapshot(&self) -> Result<Vec<u8>, String> {
        RawSnapshotBackupManager::new(defaultRuntimeStorageHost()).exportSnapshot()
    }

    /// Restores a raw runtime storage snapshot after closing the active database handle.
    #[allow(non_snake_case)]
    pub fn importRawSnapshot(&self, bytes: Vec<u8>) -> Result<(), String> {
        AppDatabase::closeDatabase();
        RawSnapshotBackupManager::new(defaultRuntimeStorageHost()).restoreSnapshot(bytes)
    }

    /// Reads snapshot metadata without mutating the current runtime store.
    #[allow(non_snake_case)]
    pub fn inspectRawSnapshot(&self, bytes: Vec<u8>) -> Result<RawSnapshotManifest, String> {
        RawSnapshotBackupManager::new(defaultRuntimeStorageHost()).inspectSnapshot(bytes)
    }

    /// Previews an Operit 1 model-configuration snapshot before import.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn inspectOperit1ModelConfigSnapshot(
        &self,
        bytes: Vec<u8>,
    ) -> Result<Operit1ModelConfigSnapshotPreview, String> {
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .inspectModelConfigSnapshot(bytes)
    }

    /// Previews an Operit 1 full snapshot before import.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn inspectOperit1Snapshot(&self, bytes: Vec<u8>) -> Result<Operit1SnapshotPreview, String> {
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .inspectSnapshot(bytes)
    }

    /// Reads and previews an Operit 1 snapshot file from disk.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn inspectOperit1SnapshotFile(
        &self,
        path: String,
    ) -> Result<Operit1SnapshotPreview, String> {
        let bytes = fs::read(path).map_err(|_| "无法读取 Operit1 快照文件".to_string())?;
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .inspectSnapshot(bytes)
    }

    /// Imports a model configuration from an Operit 1 snapshot into the selected profile.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn importOperit1ModelConfigSnapshot(
        &self,
        bytes: Vec<u8>,
        configId: String,
        modelId: String,
    ) -> Result<Operit1ModelConfigImportResult, String> {
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .importModelConfigSnapshot(bytes, configId, modelId)
    }

    /// Imports an Operit 1 full snapshot from bytes and publishes progress events.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn importOperit1Snapshot(
        &self,
        bytes: Vec<u8>,
    ) -> Result<Operit1SnapshotImportResult, String> {
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress {
            stage: "parse".to_string(),
            title: "解析快照".to_string(),
            detail: "正在读取 Operit1 快照内容。".to_string(),
            progress: 0.04,
            active: true,
        });
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .importSnapshot(bytes)
    }

    /// Reads and imports an Operit 1 full snapshot file from disk.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn importOperit1SnapshotFile(
        &self,
        path: String,
    ) -> Result<Operit1SnapshotImportResult, String> {
        publishOperit1SnapshotImportProgress(Operit1SnapshotImportProgress {
            stage: "read_file".to_string(),
            title: "读取快照文件".to_string(),
            detail: "正在从所选路径读取快照。".to_string(),
            progress: 0.02,
            active: true,
        });
        let bytes = fs::read(path).map_err(|_| "无法读取 Operit1 快照文件".to_string())?;
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .importSnapshot(bytes)
    }

    /// Observes the latest Operit 1 snapshot import progress state.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn operit1SnapshotImportProgressFlow(&self) -> StateFlow<Operit1SnapshotImportProgress> {
        observeOperit1SnapshotImportProgress()
    }

    /// Applies a single non-chat sync operation to the correct persistent domain.
    #[allow(non_snake_case)]
    fn applySyncOperation(&self, operation: &SyncOperation) -> Result<(), String> {
        match (
            operation.domain.as_str(),
            operation.entityType.as_str(),
            operation.operation.as_str(),
        ) {
            ("preferences", _, "upsert") => PreferencesDataStore::applySyncedPreferences(
                &operation.entityId,
                operation.payload.clone(),
            )
            .map_err(|error| error.to_string()),
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
