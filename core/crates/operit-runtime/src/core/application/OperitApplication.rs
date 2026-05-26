use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::data::preferences::UserPreferencesManager::UserPreferencesManager;
use crate::data::model::Memory::{Memory, MemoryLink};
use crate::data::mcp::plugins::MCPStarter::MCPStarter;
use crate::data::sync::SqlChatSyncStore::{SqlChatSyncStore, CHAT_SYNC_DOMAIN};
use operit_store::ObjectBoxStore::{ObjectBox, OBJECTBOX_SYNC_DOMAIN};
use operit_store::PreferencesDataStore::PreferencesDataStore;
use operit_store::RuntimeStorageHost::{setDefaultRuntimeSqliteHost, setDefaultRuntimeStorageHost};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;
use operit_store::SyncOperationStore::{
    compactSyncOperations, SyncClock, SyncOperation, SyncOperationStore,
};

pub struct OperitApplication {
    pub appStartupTimeMs: i64,
    pub applicationContext: OperitApplicationContext,
    pub chatRuntimeHolder: ChatRuntimeHolder,
    pub initialized: bool,
}

impl OperitApplication {
    pub fn new() -> Self {
        Self::newWithContext(OperitApplicationContext::new())
    }

    #[allow(non_snake_case)]
    pub fn newWithContext(applicationContext: OperitApplicationContext) -> Self {
        if let Some(runtimeStorageHost) = applicationContext.runtimeStorageHost.clone() {
            setDefaultRuntimeStorageHost(runtimeStorageHost);
        }
        if let Some(runtimeSqliteHost) = applicationContext.runtimeSqliteHost.clone() {
            setDefaultRuntimeSqliteHost(runtimeSqliteHost);
        }
        Self {
            appStartupTimeMs: 0,
            applicationContext,
            chatRuntimeHolder: ChatRuntimeHolder::new(),
            initialized: false,
        }
    }

    #[allow(non_snake_case)]
    pub fn onCreate(&mut self) -> Result<(), String> {
        self.appStartupTimeMs = currentTimeMillis();
        self.configureOpenMpEnvironment();
        self.ensureWorkManagerInitialized();
        AIMessageManager::initialize();
        self.initializeJsonSerializer();
        self.initializeAppLanguage();
        self.initUserPreferencesManager()?;
        self.initAndroidPermissionPreferences();
        self.initializeFunctionalPromptManager()?;
        self.preloadDatabase();
        self.chatRuntimeHolder = ChatRuntimeHolder::new();
        let mut toolHandler = AIToolHandler::getInstance(self.applicationContext.clone());
        toolHandler.registerDefaultTools();
        self.initMcpPlugins();
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
        let _ = starter.startAllDeployedPlugins();
    }

    #[allow(non_snake_case)]
    pub fn coreVersion(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
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
        serde_json::to_value(clock)
            .map_err(|error| error.to_string())
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
    pub fn syncApplyOperations(&self, operations: serde_json::Value) -> Result<serde_json::Value, String> {
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

impl Default for OperitApplication {
    fn default() -> Self {
        Self::new()
    }
}

fn currentTimeMillis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after UNIX_EPOCH")
        .as_millis() as i64
}

fn mergeSyncClock(target: &mut SyncClock, source: SyncClock) {
    for (deviceId, sequence) in source.sequences {
        if sequence > target.sequenceFor(&deviceId) {
            target.setSequence(deviceId, sequence);
        }
    }
}
