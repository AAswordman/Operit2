use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use crate::api::chat::enhance::ToolExecutionManager::AITool;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    ALLOW,
    ASK,
    FORBID,
}

impl PermissionLevel {
    pub fn fromString(value: Option<&str>) -> Self {
        match value {
            Some("ALLOW") => Self::ALLOW,
            Some("CAUTION") | Some("ASK") => Self::ASK,
            Some("FORBID") => Self::FORBID,
            _ => Self::ASK,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::ALLOW => "ALLOW",
            Self::ASK => "ASK",
            Self::FORBID => "FORBID",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionRequestResult {
    ALLOW,
    DENY,
    ALWAYS_ALLOW,
}

type PermissionRequester =
    Arc<dyn Fn(&AITool, &str) -> PermissionRequestResult + Send + Sync + 'static>;
type OperationDescriptionGenerator = Arc<dyn Fn(&AITool) -> String + Send + Sync + 'static>;

#[derive(Clone)]
pub struct ToolPermissionSystem {
    dataStore: PreferencesDataStore,
    operationDescriptionRegistry: Arc<Mutex<BTreeMap<String, OperationDescriptionGenerator>>>,
    permissionRequester: Arc<Mutex<Option<PermissionRequester>>>,
}

impl ToolPermissionSystem {
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(
                paths.root_dir().join("tool_permissions.preferences.json"),
            ),
            operationDescriptionRegistry: Arc::new(Mutex::new(BTreeMap::new())),
            permissionRequester: Arc::new(Mutex::new(None)),
        }
    }

    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    #[allow(non_snake_case)]
    fn MASTER_SWITCH() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("master_switch")
    }

    #[allow(non_snake_case)]
    fn toolPermissionKey(toolName: &str) -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey(&format!("tool_permission_{toolName}"))
    }

    #[allow(non_snake_case)]
    pub fn registerOperationDescription<F>(&self, toolName: &str, descriptionGenerator: F)
    where
        F: Fn(&AITool) -> String + Send + Sync + 'static,
    {
        self.operationDescriptionRegistry
            .lock()
            .expect("tool permission registry mutex poisoned")
            .insert(toolName.to_string(), Arc::new(descriptionGenerator));
    }

    #[allow(non_snake_case)]
    pub fn setPermissionRequester<F>(&self, requester: F)
    where
        F: Fn(&AITool, &str) -> PermissionRequestResult + Send + Sync + 'static,
    {
        *self
            .permissionRequester
            .lock()
            .expect("tool permission requester mutex poisoned") = Some(Arc::new(requester));
    }

    #[allow(non_snake_case)]
    pub fn clearPermissionRequester(&self) {
        *self
            .permissionRequester
            .lock()
            .expect("tool permission requester mutex poisoned") = None;
    }

    #[allow(non_snake_case)]
    pub fn saveMasterSwitch(
        &self,
        level: PermissionLevel,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.set(&Self::MASTER_SWITCH(), level.name().to_string());
        })
    }

    #[allow(non_snake_case)]
    pub fn saveToolPermission(
        &self,
        toolName: &str,
        level: PermissionLevel,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.set(&Self::toolPermissionKey(toolName), level.name().to_string());
        })
    }

    #[allow(non_snake_case)]
    pub fn clearToolPermission(&self, toolName: &str) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.remove(&Self::toolPermissionKey(toolName));
        })
    }

    #[allow(non_snake_case)]
    pub fn getMasterSwitch(&self) -> Result<PermissionLevel, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        Ok(PermissionLevel::fromString(
            preferences.get(&Self::MASTER_SWITCH()).map(String::as_str),
        ))
    }

    #[allow(non_snake_case)]
    pub fn getToolPermissionOverrides(
        &self,
    ) -> Result<BTreeMap<String, PermissionLevel>, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        let mut out = BTreeMap::new();
        for (key, value) in preferences.entries() {
            if let Some(toolName) = key.strip_prefix("tool_permission_") {
                out.insert(
                    toolName.to_string(),
                    PermissionLevel::fromString(Some(value.as_str())),
                );
            }
        }
        Ok(out)
    }

    #[allow(non_snake_case)]
    pub fn getToolPermission(
        &self,
        toolName: &str,
    ) -> Result<PermissionLevel, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        Ok(PermissionLevel::fromString(
            preferences
                .get(&Self::toolPermissionKey(toolName))
                .map(String::as_str),
        ))
    }

    #[allow(non_snake_case)]
    pub fn getToolPermissionOverride(
        &self,
        toolName: &str,
    ) -> Result<Option<PermissionLevel>, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        Ok(preferences
            .get(&Self::toolPermissionKey(toolName))
            .map(|value| PermissionLevel::fromString(Some(value.as_str()))))
    }

    #[allow(non_snake_case)]
    pub fn getOperationDescription(&self, tool: &AITool) -> String {
        self.operationDescriptionRegistry
            .lock()
            .expect("tool permission registry mutex poisoned")
            .get(&tool.name)
            .map(|generator| generator(tool))
            .unwrap_or_else(|| format!("Tool operation: {}", tool.name))
    }

    #[allow(non_snake_case)]
    pub fn checkToolPermission(&self, tool: &AITool) -> Result<bool, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        let masterSwitch = PermissionLevel::fromString(
            preferences.get(&Self::MASTER_SWITCH()).map(String::as_str),
        );
        let overrideLevel = preferences
            .get(&Self::toolPermissionKey(&tool.name))
            .map(|value| PermissionLevel::fromString(Some(value.as_str())));
        let permissionLevel = overrideLevel.unwrap_or(masterSwitch);

        match permissionLevel {
            PermissionLevel::ALLOW => Ok(true),
            PermissionLevel::FORBID => Ok(false),
            PermissionLevel::ASK => self.requestPermission(tool),
        }
    }

    #[allow(non_snake_case)]
    pub fn refreshPermissionRequestState(&self) -> bool {
        false
    }

    #[allow(non_snake_case)]
    fn requestPermission(&self, tool: &AITool) -> Result<bool, PreferencesDataStoreError> {
        let description = self.getOperationDescription(tool);
        let requester = self
            .permissionRequester
            .lock()
            .expect("tool permission requester mutex poisoned")
            .clone();

        let result = requester
            .map(|callback| callback(tool, &description))
            .unwrap_or(PermissionRequestResult::DENY);

        match result {
            PermissionRequestResult::ALLOW => Ok(true),
            PermissionRequestResult::DENY => Ok(false),
            PermissionRequestResult::ALWAYS_ALLOW => {
                self.saveToolPermission(&tool.name, PermissionLevel::ALLOW)?;
                Ok(true)
            }
        }
    }
}
