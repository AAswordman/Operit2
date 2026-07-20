use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use operit_store::PreferencesDataStore::{
    stringPreferencesKey, PreferencesDataStore, PreferencesDataStoreError,
};
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

use operit_tools::ToolExecutionManager::{AITool, ToolEffect};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiPermissionMode {
    ReadOnly,
    WorkspaceWrite,
    Full,
}

impl AiPermissionMode {
    pub fn fromString(value: Option<&str>) -> Result<Self, PreferencesDataStoreError> {
        match value {
            None => Ok(Self::WorkspaceWrite),
            Some("ReadOnly") | Some("READ_ONLY") => Ok(Self::ReadOnly),
            Some("WorkspaceWrite") | Some("WORKSPACE_WRITE") => Ok(Self::WorkspaceWrite),
            Some("Full") | Some("FULL") => Ok(Self::Full),
            Some(value) => Err(PreferencesDataStoreError::Message(format!(
                "unknown AI permission mode: {value}"
            ))),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::ReadOnly => "ReadOnly",
            Self::WorkspaceWrite => "WorkspaceWrite",
            Self::Full => "Full",
        }
    }

    pub fn allowsEffect(&self, effect: ToolEffect) -> bool {
        match (self, effect) {
            (Self::ReadOnly, ToolEffect::READ) => true,
            (Self::ReadOnly, ToolEffect::WRITE) => false,
            (Self::WorkspaceWrite | Self::Full, _) => true,
        }
    }
}

/// Result returned by an interactive permission requester.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionRequestResult {
    /// Allow only the current tool request.
    ALLOW,
    /// Deny the current tool request.
    DENY,
    /// Allow matching tool requests until this runtime session ends.
    ALLOW_SESSION,
}

type AsyncPermissionRequestFuture =
    Pin<Box<dyn Future<Output = PermissionRequestResult> + Send + 'static>>;
type AsyncPermissionRequester =
    Arc<dyn Fn(AITool, String) -> AsyncPermissionRequestFuture + Send + Sync + 'static>;
type OperationDescriptionGenerator = Arc<dyn Fn(&AITool) -> String + Send + Sync + 'static>;

/// Persists tool permission policy and mediates interactive permission checks.
#[derive(Clone)]
pub struct ToolPermissionSystem {
    dataStore: PreferencesDataStore,
    operationDescriptionRegistry: Arc<Mutex<BTreeMap<String, OperationDescriptionGenerator>>>,
    asyncPermissionRequester: Arc<Mutex<Option<AsyncPermissionRequester>>>,
    sessionApprovedTools: Arc<Mutex<BTreeSet<String>>>,
}

impl ToolPermissionSystem {
    /// Creates a permission system backed by the runtime preferences path.
    pub fn new(paths: RuntimeStorePaths) -> Self {
        Self {
            dataStore: PreferencesDataStore::new(paths.tool_permissions_preferences_path()),
            operationDescriptionRegistry: Arc::new(Mutex::new(BTreeMap::new())),
            asyncPermissionRequester: Arc::new(Mutex::new(None)),
            sessionApprovedTools: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    /// Creates a permission system using the default runtime store paths.
    #[allow(non_snake_case)]
    pub fn getInstance() -> Self {
        Self::new(RuntimeStorePaths::default())
    }

    /// Returns the preferences key used for the global tool permission setting.
    #[allow(non_snake_case)]
    fn AI_PERMISSION_MODE() -> operit_store::PreferencesDataStore::PreferencesKey {
        stringPreferencesKey("ai_permission_mode")
    }

    /// Registers a human-readable operation description generator for a tool.
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

    /// Installs the asynchronous callback used when a tool requires user approval.
    #[allow(non_snake_case)]
    pub fn setAsyncPermissionRequester<F, TFuture>(&self, requester: F)
    where
        F: Fn(AITool, String) -> TFuture + Send + Sync + 'static,
        TFuture: Future<Output = PermissionRequestResult> + Send + 'static,
    {
        *self
            .asyncPermissionRequester
            .lock()
            .expect("tool async permission requester mutex poisoned") =
            Some(Arc::new(move |tool, description| {
                Box::pin(requester(tool, description))
            }));
    }

    /// Removes the active interactive permission requester.
    #[allow(non_snake_case)]
    pub fn clearAsyncPermissionRequester(&self) {
        *self
            .asyncPermissionRequester
            .lock()
            .expect("tool async permission requester mutex poisoned") = None;
    }

    /// Clears approvals that were granted for the current runtime session.
    #[allow(non_snake_case)]
    pub fn clearSessionApprovals(&self) {
        self.sessionApprovedTools
            .lock()
            .expect("tool permission session approvals mutex poisoned")
            .clear();
    }

    /// Saves the global AI permission mode used to allow or block tool effects.
    #[allow(non_snake_case)]
    pub fn saveAiPermissionMode(
        &self,
        mode: AiPermissionMode,
    ) -> Result<(), PreferencesDataStoreError> {
        self.dataStore.edit(|preferences| {
            preferences.set(&Self::AI_PERMISSION_MODE(), mode.name().to_string());
        })
    }

    /// Reads the global AI permission mode used to allow or block tool effects.
    #[allow(non_snake_case)]
    pub fn getAiPermissionMode(&self) -> Result<AiPermissionMode, PreferencesDataStoreError> {
        let preferences = self.dataStore.data()?;
        AiPermissionMode::fromString(
            preferences
                .get(&Self::AI_PERMISSION_MODE())
                .map(String::as_str),
        )
    }

    /// Builds the description shown to the requester for a tool invocation.
    #[allow(non_snake_case)]
    pub fn getOperationDescription(&self, tool: &AITool) -> String {
        self.operationDescriptionRegistry
            .lock()
            .expect("tool permission registry mutex poisoned")
            .get(&tool.name)
            .map(|generator| generator(tool))
            .unwrap_or_else(|| format!("Tool operation: {}", tool.name))
    }

    /// Asynchronously requests approval for a package tool invocation.
    #[allow(non_snake_case)]
    pub async fn checkPackageToolApprovalAsync(
        &self,
        tool: &AITool,
    ) -> Result<bool, PreferencesDataStoreError> {
        self.requestPermissionAsync(tool).await
    }

    /// Asynchronously requests approval for a tool invocation that can escape the sandbox boundary.
    #[allow(non_snake_case)]
    pub async fn checkSandboxEscapeApprovalAsync(
        &self,
        tool: &AITool,
    ) -> Result<bool, PreferencesDataStoreError> {
        self.requestPermissionAsync(tool).await
    }

    /// Refreshes permission request state exposed to front-end observers.
    #[allow(non_snake_case)]
    pub fn refreshPermissionRequestState(&self) -> bool {
        false
    }

    /// Awaits one registered approval callback and applies its result to session state.
    #[allow(non_snake_case)]
    async fn requestPermissionAsync(
        &self,
        tool: &AITool,
    ) -> Result<bool, PreferencesDataStoreError> {
        if self
            .sessionApprovedTools
            .lock()
            .expect("tool permission session approvals mutex poisoned")
            .contains(&tool.name)
        {
            return Ok(true);
        }

        let description = self.getOperationDescription(tool);
        let requester = self
            .asyncPermissionRequester
            .lock()
            .expect("tool async permission requester mutex poisoned")
            .clone();
        let result = match requester {
            Some(callback) => callback(tool.clone(), description).await,
            None => PermissionRequestResult::DENY,
        };

        self.applyPermissionResult(tool, result)
    }

    /// Applies an approval decision to the current tool request and session state.
    #[allow(non_snake_case)]
    fn applyPermissionResult(
        &self,
        tool: &AITool,
        result: PermissionRequestResult,
    ) -> Result<bool, PreferencesDataStoreError> {
        match result {
            PermissionRequestResult::ALLOW => Ok(true),
            PermissionRequestResult::DENY => Ok(false),
            PermissionRequestResult::ALLOW_SESSION => {
                self.sessionApprovedTools
                    .lock()
                    .expect("tool permission session approvals mutex poisoned")
                    .insert(tool.name.clone());
                Ok(true)
            }
        }
    }
}
