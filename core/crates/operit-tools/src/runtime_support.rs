use std::collections::{BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use operit_host_api::HostEnvironmentDescriptor;
use operit_model::ChatTurnOptions::ChatTurnOptions;
use operit_model::ToolPrompt::SystemToolPromptCategory;

use crate::tools::packTool::PackageManager::PackageManager;

/// Future returned by runtime support async boundaries.
pub type ToolRuntimeSupportFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Runtime-owned character card tool-access result consumed by tools.
#[derive(Clone, Debug, Default)]
pub struct ResolvedCharacterCardToolAccess {
    pub customEnabled: bool,
    pub effectiveBuiltinToolVisibility: HashMap<String, bool>,
    pub allowedPackageNames: HashSet<String>,
    pub allowedSkillNames: HashSet<String>,
    pub allowedMcpServerNames: HashSet<String>,
    pub canUsePackageSystem: bool,
    pub hasAnyAllowedExternalSource: bool,
}

impl ResolvedCharacterCardToolAccess {
    /// Returns whether a built-in tool is visible under the resolved access rules.
    #[allow(non_snake_case)]
    pub fn isBuiltinToolAllowed(&self, toolName: &str) -> bool {
        if !self.customEnabled {
            return self
                .effectiveBuiltinToolVisibility
                .get(toolName)
                .copied()
                .unwrap_or(true);
        }
        match toolName {
            "package_proxy" => self.hasAnyAllowedExternalSource,
            _ => self
                .effectiveBuiltinToolVisibility
                .get(toolName)
                .copied()
                .unwrap_or(false),
        }
    }

    /// Returns whether an external package, skill, or MCP source is visible.
    #[allow(non_snake_case)]
    pub fn isExternalSourceAllowed(&self, sourceName: &str) -> bool {
        if !self.customEnabled {
            return true;
        }
        if !self.canUsePackageSystem {
            return false;
        }
        self.allowedPackageNames.contains(sourceName)
            || self.allowedSkillNames.contains(sourceName)
            || self.allowedMcpServerNames.contains(sourceName)
    }
}

/// Bundled package asset exposed by the runtime crate.
#[derive(Clone, Copy)]
pub struct RuntimePluginAsset {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

/// Bundled external skill asset exposed by the runtime crate.
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
pub struct RuntimeBundledExternalSkillAsset {
    pub skillName: &'static str,
    pub path: &'static str,
    pub bytes: &'static [u8],
}

/// Cached MCP tool metadata exposed by the runtime crate.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CachedMcpToolInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "inputSchema", default)]
    pub inputSchema: String,
    #[serde(rename = "cachedAt", default)]
    pub cachedAt: i64,
}

/// Skill metadata needed by hidden tool catalog search.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeSkillCatalogEntry {
    pub name: String,
    pub description: String,
}

/// Character card metadata exposed to tools.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct RuntimeCharacterCardInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub isDefault: bool,
    pub createdAt: i64,
    pub updatedAt: i64,
}

/// Character memory binding data exposed to memory tools.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct RuntimeCharacterMemoryBinding {
    pub id: String,
    pub memoryBindingMode: String,
    pub sharedMemoryId: Option<String>,
}

/// Operation supported by the structured file edit interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeStructuredEditAction {
    REPLACE,
    DELETE,
}

/// One structured file edit operation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct RuntimeStructuredEditOperation {
    pub action: RuntimeStructuredEditAction,
    pub oldContent: String,
    pub newContent: String,
}

/// Chat runtime slot selected by chat management tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeChatSlot {
    MAIN,
    FLOATING,
}

/// Parameters for sending a chat message through the parent runtime.
#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct RuntimeChatSendRequest {
    pub slot: RuntimeChatSlot,
    pub roleCardId: Option<String>,
    pub chatId: Option<String>,
    pub message: String,
    pub proxySenderName: Option<String>,
    pub turnOptions: ChatTurnOptions,
}

/// Provides runtime-owned services that the tools crate must not own.
pub trait ToolRuntimeSupport: Send + Sync {
    /// Resolves role-card tool access for the active invocation context.
    #[allow(non_snake_case)]
    fn resolveCharacterCardToolAccess(
        &self,
        roleCardId: Option<&str>,
        packageManager: &PackageManager,
        globalToolVisibility: Option<HashMap<String, bool>>,
    ) -> ResolvedCharacterCardToolAccess;

    /// Reads one stored environment variable.
    #[allow(non_snake_case)]
    fn readEnvironmentVariable(&self, key: &str) -> Result<Option<String>, String>;

    /// Writes one stored environment variable.
    #[allow(non_snake_case)]
    fn writeEnvironmentVariable(&self, key: &str, value: &str) -> Result<(), String>;

    /// Removes one stored environment variable.
    #[allow(non_snake_case)]
    fn removeEnvironmentVariable(&self, key: &str) -> Result<(), String>;

    /// Returns built-in and internal tool prompt categories for hidden catalog search.
    #[allow(non_snake_case)]
    fn buildBuiltinAndInternalCategories(
        &self,
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> Vec<SystemToolPromptCategory>;

    /// Returns AI-visible built-in tool names for hidden catalog source labeling.
    #[allow(non_snake_case)]
    fn buildBuiltinToolNameSet(
        &self,
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> BTreeSet<String>;

    /// Returns AI-visible skill package metadata.
    #[allow(non_snake_case)]
    fn aiVisibleSkillPackages(&self) -> Vec<RuntimeSkillCatalogEntry>;

    /// Returns cached MCP tool descriptions for a server.
    #[allow(non_snake_case)]
    fn cachedMcpTools(&self, serverName: &str) -> Vec<CachedMcpToolInfo>;

    /// Returns built-in package assets owned by the runtime.
    #[allow(non_snake_case)]
    fn builtinPluginAssets(&self) -> &'static [RuntimePluginAsset];

    /// Returns bundled external package assets owned by the runtime.
    #[allow(non_snake_case)]
    fn bundledExternalPluginAssets(&self) -> &'static [RuntimePluginAsset];

    /// Returns bundled external skill assets owned by the runtime.
    #[allow(non_snake_case)]
    fn bundledExternalSkillAssets(&self) -> &'static [RuntimeBundledExternalSkillAsset];

    /// Returns whether a skill is visible to AI package activation.
    #[allow(non_snake_case)]
    fn isSkillVisibleToAi(&self, skillName: &str) -> bool;

    /// Updates AI visibility for a skill package.
    #[allow(non_snake_case)]
    fn setSkillVisibleToAi(&self, skillName: &str, visible: bool) -> Result<(), String>;

    /// Generates an MCP plugin description using parent-owned model services.
    #[allow(non_snake_case)]
    fn generateMcpPluginDescription<'a>(
        &'a self,
        pluginName: &'a str,
        toolDescriptions: &'a [String],
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>>;

    /// Starts parent-owned chat services.
    #[allow(non_snake_case)]
    fn startChatServices(&self) -> Result<(), String>;

    /// Stops parent-owned chat services.
    #[allow(non_snake_case)]
    fn stopChatServices(&self) -> Result<(), String>;

    /// Returns whether the requested chat is currently processing.
    #[allow(non_snake_case)]
    fn isChatProcessing(&self, chatId: &str) -> Result<bool, String>;

    /// Switches the parent-owned main chat runtime to a chat id.
    #[allow(non_snake_case)]
    fn switchMainChat(&self, chatId: &str) -> Result<(), String>;

    /// Creates a chat through the parent-owned chat runtime.
    #[allow(non_snake_case)]
    fn createChatRuntime(
        &self,
        characterCardName: Option<String>,
        group: Option<String>,
        setAsCurrentChat: bool,
    ) -> Result<(), String>;

    /// Sends a message through the parent-owned chat runtime.
    #[allow(non_snake_case)]
    fn sendChatMessage<'a>(
        &'a self,
        request: RuntimeChatSendRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>>;

    /// Lists character cards through parent-owned preferences.
    #[allow(non_snake_case)]
    fn listCharacterCards(&self) -> Result<Vec<RuntimeCharacterCardInfo>, String>;

    /// Resolves a character card name by id.
    #[allow(non_snake_case)]
    fn characterCardName(&self, cardId: &str) -> Result<String, String>;

    /// Resolves memory binding metadata by character card id.
    #[allow(non_snake_case)]
    fn characterMemoryBinding(&self, cardId: &str) -> Result<RuntimeCharacterMemoryBinding, String>;

    /// Loads memory search settings for an owner scope.
    #[allow(non_snake_case)]
    fn loadMemorySearchSettings(&self, ownerScope: &str) -> Result<(), String>;

    /// Applies structured edits to file content.
    #[allow(non_snake_case)]
    fn processFileBindingOperations(
        &self,
        originalContent: &str,
        operations: &[RuntimeStructuredEditOperation],
    ) -> (String, String);

    /// Generates a unified diff for file apply results.
    #[allow(non_snake_case)]
    fn generateUnifiedDiff(&self, oldContent: &str, newContent: &str) -> String;
}

static TOOL_RUNTIME_SUPPORT: OnceLock<Arc<dyn ToolRuntimeSupport>> = OnceLock::new();

/// Installs the runtime support implementation supplied by the parent crate.
#[allow(non_snake_case)]
pub fn setToolRuntimeSupport(support: Arc<dyn ToolRuntimeSupport>) -> Result<(), String> {
    TOOL_RUNTIME_SUPPORT
        .set(support)
        .map_err(|_| "Tool runtime support is already installed".to_string())
}

/// Returns the active runtime support implementation.
#[allow(non_snake_case)]
pub fn toolRuntimeSupport() -> &'static dyn ToolRuntimeSupport {
    TOOL_RUNTIME_SUPPORT
        .get()
        .map(|support| support.as_ref())
        .expect("Tool runtime support must be installed before tools are used")
}
