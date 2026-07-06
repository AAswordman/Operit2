use crate::api::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use crate::core::application::OperitApplicationContext::{
    setDefaultHttpHost, OperitApplicationContext,
};
use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::data::backup::Operit1SnapshotImportManager::{
    observeOperit1SnapshotImportProgress, publishOperit1SnapshotImportProgress,
    Operit1ModelConfigImportResult, Operit1ModelConfigSnapshotPreview,
    Operit1SnapshotImportManager, Operit1SnapshotImportProgress, Operit1SnapshotImportResult,
    Operit1SnapshotPreview,
};
use crate::data::backup::RawSnapshotBackupManager::{
    RawSnapshotBackupManager, RawSnapshotManifest,
};
use crate::data::db::AppDatabase::AppDatabase;
use crate::data::mcp::plugins::MCPStarter::MCPStarter;
use crate::data::model::Memory::{Memory, MemoryLink};
use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::data::preferences::UserPreferencesManager::UserPreferencesManager;
use crate::data::sync::SqlChatSyncStore::{SqlChatSyncStore, CHAT_SYNC_DOMAIN};
use crate::plugins::PluginRegistry::PluginRegistry;
use operit_host_api::HostRuntimeEventRegistration;
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_store::ObjectBoxStore::{ObjectBox, OBJECTBOX_SYNC_DOMAIN};
use operit_store::PreferencesDataStore::PreferencesDataStore;
use operit_store::PreferencesDataStore::StateFlow;
use operit_store::RuntimeStorageHost::{
    defaultRuntimeStorageHost, setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost,
};
use operit_store::RuntimeStorePaths::{setDefaultRuntimeStoreRoot, RuntimeStorePaths};
use operit_store::SyncOperationStore::{
    compactSyncOperations, SyncClock, SyncOperation, SyncOperationStore,
};
use std::fs;
use std::sync::{Mutex, OnceLock};

use crate::util::AppLogger::AppLogger;
use crate::util::OperitPaths;

static APPLICATION_CONTEXT: OnceLock<Mutex<Option<OperitApplicationContext>>> = OnceLock::new();

pub struct OperitApplication {
    pub appStartupTimeMs: i64,
    pub applicationContext: OperitApplicationContext,
    pub chatRuntimeHolder: ChatRuntimeHolder,
    pub initialized: bool,
    hostRuntimeEventRegistration: Option<Box<dyn HostRuntimeEventRegistration>>,
}

impl OperitApplication {
    pub fn new() -> Self {
        Self::newWithContext(OperitApplicationContext::new())
    }

    #[allow(non_snake_case)]
    pub fn newWithContext(applicationContext: OperitApplicationContext) -> Self {
        if let Some(runtimeStorageHost) = applicationContext.runtimeStorageHost.clone() {
            if let Some(rootDir) = runtimeStorageHost.rootDir() {
                AppLogger::configure_log_files(&rootDir);
                setDefaultRuntimeStoreRoot(rootDir);
            }
            setDefaultRuntimeStorageHost(runtimeStorageHost);
        }
        if let Some(runtimeSqliteHost) = applicationContext.runtimeSqliteHost.clone() {
            setDefaultRuntimeSqliteHost(runtimeSqliteHost);
        }
        if let Some(httpHost) = applicationContext.httpHost.clone() {
            setDefaultHttpHost(httpHost);
        }
        Self {
            appStartupTimeMs: 0,
            applicationContext,
            chatRuntimeHolder: ChatRuntimeHolder::new(),
            initialized: false,
            hostRuntimeEventRegistration: None,
        }
    }

    #[allow(non_snake_case)]
    pub fn onCreate(&mut self) -> Result<(), String> {
        self.appStartupTimeMs = currentTimeMillis();
        setApplicationContext(self.applicationContext.clone());
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
        let mut toolHandler = AIToolHandler::getInstance(self.applicationContext.clone());
        toolHandler.registerDefaultTools();
        PluginRegistry::initializeBuiltins();
        self.initMcpPlugins();
        self.chatRuntimeHolder =
            ChatRuntimeHolder::newWithApplicationContext(self.applicationContext.clone());
        self.hostRuntimeEventRegistration = crate::services::RuntimeEventIngressService::RuntimeEventIngressService::startHostRuntimeEventSupport(
            self.applicationContext.clone(),
        )?;
        self.initialized = true;
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn configureOpenMpEnvironment(&self) {}

    #[allow(non_snake_case)]
    pub fn ensureWorkManagerInitialized(&self) {}

    #[allow(non_snake_case)]
    pub fn initializeJsonSerializer(&self) {}

    #[allow(non_snake_case)]
    pub fn initializeAppLanguage(&self) {}

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

    #[allow(non_snake_case)]
    pub fn initAndroidPermissionPreferences(&self) {}

    #[allow(non_snake_case)]
    pub fn initializeFunctionalPromptManager(&self) -> Result<(), String> {
        CharacterCardManager::getInstance()
            .initializeIfNeeded()
            .map_err(|error| error.to_string())
    }

    #[allow(non_snake_case)]
    pub fn preloadDatabase(&self) {}

    #[allow(non_snake_case)]
    pub fn initMcpPlugins(&self) {
        let starter = MCPStarter::new(self.applicationContext.clone());
        let timeoutSeconds = ApiPreferences::getInstance()
            .getMcpStartupTimeoutSeconds()
            .expect("api preferences must provide mcp startup timeout seconds");
        let _ = starter.startAllDeployedPluginsWithTimeout(timeoutSeconds);
    }

    #[allow(non_snake_case)]
    pub fn coreVersion(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    #[allow(non_snake_case)]
    pub fn logEntries(&self) -> serde_json::Value {
        AppLogger::entries_json()
    }

    #[allow(non_snake_case)]
    pub fn logText(&self) -> Result<String, String> {
        AppLogger::text()
    }

    #[allow(non_snake_case)]
    pub fn packageLogText(&self) -> Result<String, String> {
        AppLogger::package_text()
    }

    #[allow(non_snake_case)]
    pub fn logFilePath(&self) -> Result<String, String> {
        AppLogger::get_log_file_path()
    }

    #[allow(non_snake_case)]
    pub fn packageLogFilePath(&self) -> Result<String, String> {
        AppLogger::get_package_log_file_path()
    }

    #[allow(non_snake_case)]
    pub fn operitRootPath(&self) -> Result<String, String> {
        OperitPaths::operitRootPathSdcard()
    }

    #[allow(non_snake_case)]
    pub fn exportsPath(&self) -> Result<String, String> {
        OperitPaths::exportsPathSdcard()
    }

    #[allow(non_snake_case)]
    pub fn cleanOnExitPath(&self) -> Result<String, String> {
        OperitPaths::cleanOnExitPathSdcard()
    }

    #[allow(non_snake_case)]
    pub fn resetLogs(&self) {
        AppLogger::reset_log_file();
    }

    #[allow(non_snake_case)]
    pub fn applicationContext() -> OperitApplicationContext {
        APPLICATION_CONTEXT
            .get_or_init(|| Mutex::new(None))
            .lock()
            .expect("OperitApplication application context mutex poisoned")
            .clone()
            .expect("OperitApplication application context must be initialized")
    }

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

    #[allow(non_snake_case)]
    pub fn exportRawSnapshot(&self) -> Result<Vec<u8>, String> {
        RawSnapshotBackupManager::new(defaultRuntimeStorageHost()).exportSnapshot()
    }

    #[allow(non_snake_case)]
    pub fn importRawSnapshot(&self, bytes: Vec<u8>) -> Result<(), String> {
        AppDatabase::closeDatabase();
        RawSnapshotBackupManager::new(defaultRuntimeStorageHost()).restoreSnapshot(bytes)
    }

    #[allow(non_snake_case)]
    pub fn inspectRawSnapshot(&self, bytes: Vec<u8>) -> Result<RawSnapshotManifest, String> {
        RawSnapshotBackupManager::new(defaultRuntimeStorageHost()).inspectSnapshot(bytes)
    }

    #[allow(non_snake_case)]
    pub fn inspectOperit1ModelConfigSnapshot(
        &self,
        bytes: Vec<u8>,
    ) -> Result<Operit1ModelConfigSnapshotPreview, String> {
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .inspectModelConfigSnapshot(bytes)
    }

    #[allow(non_snake_case)]
    pub fn inspectOperit1Snapshot(
        &self,
        bytes: Vec<u8>,
    ) -> Result<Operit1SnapshotPreview, String> {
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .inspectSnapshot(bytes)
    }

    #[allow(non_snake_case)]
    pub fn inspectOperit1SnapshotFile(
        &self,
        path: String,
    ) -> Result<Operit1SnapshotPreview, String> {
        let bytes = fs::read(path).map_err(|_| "无法读取 Operit1 快照文件".to_string())?;
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .inspectSnapshot(bytes)
    }

    #[allow(non_snake_case)]
    pub fn importOperit1ModelConfigSnapshot(
        &self,
        bytes: Vec<u8>,
        configId: String,
        modelId: String,
    ) -> Result<Operit1ModelConfigImportResult, String> {
        Operit1SnapshotImportManager::new(RuntimeStorePaths::default().root_dir().to_path_buf())
            .importModelConfigSnapshot(bytes, configId, modelId)
    }

    #[allow(non_snake_case)]
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

    #[allow(non_snake_case)]
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

    #[allow(non_snake_case)]
    pub fn operit1SnapshotImportProgressFlow(
        &self,
    ) -> StateFlow<Operit1SnapshotImportProgress> {
        observeOperit1SnapshotImportProgress()
    }

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

#[allow(non_snake_case)]
fn setApplicationContext(applicationContext: OperitApplicationContext) {
    *APPLICATION_CONTEXT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .expect("OperitApplication application context mutex poisoned") = Some(applicationContext);
}

impl Default for OperitApplication {
    fn default() -> Self {
        Self::new()
    }
}

fn mergeSyncClock(target: &mut SyncClock, source: SyncClock) {
    for (deviceId, sequence) in source.sequences {
        if sequence > target.sequenceFor(&deviceId) {
            target.setSequence(deviceId, sequence);
        }
    }
}
