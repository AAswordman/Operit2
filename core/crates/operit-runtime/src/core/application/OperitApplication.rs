use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use crate::core::application::OperitApplicationContext::OperitApplicationContext;
use crate::core::chat::AIMessageManager::AIMessageManager;
use crate::core::tools::AIToolHandler::AIToolHandler;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::data::mcp::plugins::MCPStarter::MCPStarter;

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
