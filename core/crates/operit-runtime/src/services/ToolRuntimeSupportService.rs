use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

use operit_host_api::HostEnvironmentDescriptor;
use operit_model::FunctionType::FunctionType;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_model::ToolPrompt::{
    SystemToolPromptCategory, ToolParameterSchema, ToolPrompt,
};
use operit_providers::chat::config::FunctionalPrompts::FunctionalPrompts;
use operit_providers::chat::config::SystemToolPrompts as ProviderToolPrompts;
use operit_providers::chat::enhance::ConversationService::ConversationService;
use operit_providers::chat::enhance::FileBindingService::{
    FileBindingService, StructuredEditAction, StructuredEditOperation,
};
use operit_providers::chat::EnhancedAIService::{
    EnhancedAIService, SendMessageOptions,
};
use operit_tools::runtime_support::{
    setToolRuntimeSupport, CachedMcpToolInfo, ResolvedCharacterCardToolAccess,
    RuntimeBundledExternalSkillAsset, RuntimeCharacterCardInfo,
    RuntimeCharacterMemoryBinding, RuntimeChatSendRequest, RuntimeChatSlot, RuntimePluginAsset,
    RuntimeSkillCatalogEntry, RuntimeStructuredEditAction, RuntimeStructuredEditOperation,
    ToolRuntimeSupport, ToolRuntimeSupportFuture,
};
use operit_tools::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use operit_tools::tools::packTool::PackageManager::PackageManager;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_util::stream::Stream::Stream;

use crate::core::application::OperitApplication::OperitApplication;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::CharacterCardToolAccessResolver::{
    CharacterCardToolAccessResolver,
    ResolvedCharacterCardToolAccess as RuntimeResolvedCharacterCardToolAccess,
};
use crate::data::preferences::EnvPreferences::EnvPreferences;
use crate::data::preferences::MemorySearchSettingsPreferences::MemorySearchSettingsPreferences;
use crate::data::preferences::SkillVisibilityPreferences::SkillVisibilityPreferences;
use crate::plugins::BuiltinPluginAssets::{
    BUILTIN_PLUGIN_ASSETS, BUNDLED_EXTERNAL_PLUGIN_ASSETS,
};
use crate::plugins::BundledExternalSkillAssets::BUNDLED_EXTERNAL_SKILL_ASSETS;

/// Installs runtime-backed services required by the tools crate.
pub struct ToolRuntimeSupportService;

impl ToolRuntimeSupportService {
    /// Installs tool runtime support for this process.
    pub fn install() -> Result<(), String> {
        setToolRuntimeSupport(Arc::new(RuntimeToolSupport))
    }
}

/// Bridges tool-owned interfaces to runtime-owned managers and registries.
struct RuntimeToolSupport;

impl ToolRuntimeSupport for RuntimeToolSupport {
    /// Resolves role-card tool access through runtime preferences.
    #[allow(non_snake_case)]
    fn resolveCharacterCardToolAccess(
        &self,
        roleCardId: Option<&str>,
        packageManager: &PackageManager,
        globalToolVisibility: Option<HashMap<String, bool>>,
    ) -> ResolvedCharacterCardToolAccess {
        runtimeToolAccessToToolAccess(
            CharacterCardToolAccessResolver::getInstance().resolve(
                roleCardId,
                packageManager,
                globalToolVisibility,
            ),
        )
    }

    /// Reads one stored environment variable.
    #[allow(non_snake_case)]
    fn readEnvironmentVariable(&self, key: &str) -> Result<Option<String>, String> {
        EnvPreferences::getInstance()
            .getEnv(key)
            .map_err(|error| error.to_string())
    }

    /// Writes one stored environment variable.
    #[allow(non_snake_case)]
    fn writeEnvironmentVariable(&self, key: &str, value: &str) -> Result<(), String> {
        EnvPreferences::getInstance()
            .setEnv(key, value)
            .map_err(|error| error.to_string())
    }

    /// Removes one stored environment variable.
    #[allow(non_snake_case)]
    fn removeEnvironmentVariable(&self, key: &str) -> Result<(), String> {
        EnvPreferences::getInstance()
            .removeEnv(key)
            .map_err(|error| error.to_string())
    }

    /// Returns built-in and internal tool prompt categories.
    #[allow(non_snake_case)]
    fn buildBuiltinAndInternalCategories(
        &self,
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> Vec<SystemToolPromptCategory> {
        let categories = if useEnglish {
            ProviderToolPrompts::SystemToolPrompts::getAllCategoriesEnForHost(
                false,
                false,
                false,
                false,
                false,
                false,
                &[],
                hostEnvironment,
            )
        } else {
            ProviderToolPrompts::SystemToolPrompts::getAllCategoriesCnForHost(
                false,
                false,
                false,
                false,
                false,
                false,
                &[],
                hostEnvironment,
            )
        };
        categories.into_iter().map(providerCategoryToModel).collect()
    }

    /// Returns AI-visible built-in tool names.
    #[allow(non_snake_case)]
    fn buildBuiltinToolNameSet(
        &self,
        useEnglish: bool,
        hostEnvironment: &HostEnvironmentDescriptor,
    ) -> BTreeSet<String> {
        let categories = if useEnglish {
            ProviderToolPrompts::SystemToolPrompts::getAIAllCategoriesEnForHost(
                false,
                false,
                false,
                false,
                false,
                false,
                &[],
                hostEnvironment,
            )
        } else {
            ProviderToolPrompts::SystemToolPrompts::getAIAllCategoriesCnForHost(
                false,
                false,
                false,
                false,
                false,
                false,
                &[],
                hostEnvironment,
            )
        };
        categories
            .into_iter()
            .flat_map(|category| category.tools.into_iter())
            .map(|tool| tool.name)
            .collect()
    }

    /// Returns AI-visible skill package metadata.
    #[allow(non_snake_case)]
    fn aiVisibleSkillPackages(&self) -> Vec<RuntimeSkillCatalogEntry> {
        let hostManager = OperitApplication::hostManager();
        SkillRepository::getInstance(&hostManager)
            .getAiVisibleSkillPackages()
            .into_iter()
            .map(|(name, skill)| RuntimeSkillCatalogEntry {
                name,
                description: skill.description,
            })
            .collect()
    }

    /// Returns cached MCP tool descriptions for a server.
    #[allow(non_snake_case)]
    fn cachedMcpTools(&self, serverName: &str) -> Vec<CachedMcpToolInfo> {
        let hostManager = OperitApplication::hostManager();
        MCPLocalServer::getInstance(&hostManager)
            .getCachedTools(serverName)
            .unwrap_or_default()
            .into_iter()
            .map(|tool| CachedMcpToolInfo {
                name: tool.name,
                description: tool.description,
                inputSchema: tool.inputSchema,
                cachedAt: tool.cachedAt,
            })
            .collect()
    }

    /// Returns built-in package assets owned by the runtime.
    #[allow(non_snake_case)]
    fn builtinPluginAssets(&self) -> &'static [RuntimePluginAsset] {
        runtimeBuiltinPluginAssets()
    }

    /// Returns bundled external package assets owned by the runtime.
    #[allow(non_snake_case)]
    fn bundledExternalPluginAssets(&self) -> &'static [RuntimePluginAsset] {
        runtimeBundledExternalPluginAssets()
    }

    /// Returns bundled external skill assets owned by the runtime.
    #[allow(non_snake_case)]
    fn bundledExternalSkillAssets(&self) -> &'static [RuntimeBundledExternalSkillAsset] {
        runtimeBundledExternalSkillAssets()
    }

    /// Returns whether a skill is visible to AI package activation.
    #[allow(non_snake_case)]
    fn isSkillVisibleToAi(&self, skillName: &str) -> bool {
        SkillVisibilityPreferences::getInstance().isSkillVisibleToAi(skillName)
    }

    /// Updates AI visibility for a skill package.
    #[allow(non_snake_case)]
    fn setSkillVisibleToAi(&self, skillName: &str, visible: bool) -> Result<(), String> {
        SkillVisibilityPreferences::getInstance()
            .setSkillVisibleToAi(skillName, visible)
            .map_err(|error| error.to_string())
    }

    /// Generates an MCP plugin description using provider services.
    #[allow(non_snake_case)]
    fn generateMcpPluginDescription<'a>(
        &'a self,
        pluginName: &'a str,
        toolDescriptions: &'a [String],
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let mut service = EnhancedAIService::new(ConversationService);
            let mut options = SendMessageOptions::new();
            options.message = FunctionalPrompts::packageDescriptionUserPrompt(
                pluginName,
                &toolDescriptions.join("\n"),
                true,
            );
            options.customSystemPromptTemplate = Some(
                FunctionalPrompts::packageDescriptionSystemPrompt(true).to_string(),
            );
            options.functionType = FunctionType::CHAT;
            options.promptFunctionType = PromptFunctionType::CHAT;
            options.roleCardId = Some(CharacterCardManager::DEFAULT_CHARACTER_CARD_ID.to_string());
            options.stream = false;
            options.disableWarning = true;
            let mut stream = service
                .sendMessage(options)
                .await
                .map_err(|error| error.to_string())?;
            let mut output = String::new();
            stream.collect(&mut |chunk| output.push_str(&chunk));
            Ok(output.trim().to_string())
        })
    }

    /// Starts parent-owned chat services.
    #[allow(non_snake_case)]
    fn startChatServices(&self) -> Result<(), String> {
        Err("Chat runtime support is not connected to the application holder".to_string())
    }

    /// Stops parent-owned chat services.
    #[allow(non_snake_case)]
    fn stopChatServices(&self) -> Result<(), String> {
        Err("Chat runtime support is not connected to the application holder".to_string())
    }

    /// Returns whether the requested chat is currently processing.
    #[allow(non_snake_case)]
    fn isChatProcessing(&self, _chatId: &str) -> Result<bool, String> {
        Err("Chat runtime support is not connected to the application holder".to_string())
    }

    /// Switches the parent-owned main chat runtime to a chat id.
    #[allow(non_snake_case)]
    fn switchMainChat(&self, _chatId: &str) -> Result<(), String> {
        Err("Chat runtime support is not connected to the application holder".to_string())
    }

    /// Creates a chat through the parent-owned chat runtime.
    #[allow(non_snake_case)]
    fn createChatRuntime(
        &self,
        _characterCardName: Option<String>,
        _group: Option<String>,
        _setAsCurrentChat: bool,
    ) -> Result<(), String> {
        Err("Chat runtime support is not connected to the application holder".to_string())
    }

    /// Sends a message through the parent-owned chat runtime.
    #[allow(non_snake_case)]
    fn sendChatMessage<'a>(
        &'a self,
        _request: RuntimeChatSendRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>> {
        Box::pin(async {
            Err("Chat runtime support is not connected to the application holder".to_string())
        })
    }

    /// Lists character cards through runtime preferences.
    #[allow(non_snake_case)]
    fn listCharacterCards(&self) -> Result<Vec<RuntimeCharacterCardInfo>, String> {
        CharacterCardManager::getInstance()
            .getAllCharacterCards()
            .map(|cards| {
                cards
                    .into_iter()
                    .map(|card| RuntimeCharacterCardInfo {
                        id: card.id,
                        name: card.name,
                        description: card.description,
                        isDefault: card.isDefault,
                        createdAt: card.createdAt,
                        updatedAt: card.updatedAt,
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    }

    /// Resolves a character card name by id.
    #[allow(non_snake_case)]
    fn characterCardName(&self, cardId: &str) -> Result<String, String> {
        CharacterCardManager::getInstance()
            .getCharacterCard(cardId)
            .map(|card| card.name)
            .map_err(|error| error.to_string())
    }

    /// Resolves memory binding metadata by character card id.
    #[allow(non_snake_case)]
    fn characterMemoryBinding(&self, cardId: &str) -> Result<RuntimeCharacterMemoryBinding, String> {
        CharacterCardManager::getInstance()
            .getCharacterCard(cardId)
            .map(|card| RuntimeCharacterMemoryBinding {
                id: card.id,
                memoryBindingMode: card.memoryBindingMode,
                sharedMemoryId: card.sharedMemoryId,
            })
            .map_err(|error| error.to_string())
    }

    /// Loads memory search settings for an owner scope.
    #[allow(non_snake_case)]
    fn loadMemorySearchSettings(&self, ownerScope: &str) -> Result<(), String> {
        MemorySearchSettingsPreferences::new(ownerScope)
            .load()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Applies structured edits to file content.
    #[allow(non_snake_case)]
    fn processFileBindingOperations(
        &self,
        originalContent: &str,
        operations: &[RuntimeStructuredEditOperation],
    ) -> (String, String) {
        FileBindingService.processFileBindingOperations(
            originalContent,
            &operations
                .iter()
                .map(runtimeEditOperationToProvider)
                .collect::<Vec<_>>(),
        )
    }

    /// Generates a unified diff for file apply results.
    #[allow(non_snake_case)]
    fn generateUnifiedDiff(&self, oldContent: &str, newContent: &str) -> String {
        FileBindingService.generateUnifiedDiff(oldContent, newContent)
    }
}

/// Converts runtime preference access data into the tools crate shape.
fn runtimeToolAccessToToolAccess(
    access: RuntimeResolvedCharacterCardToolAccess,
) -> ResolvedCharacterCardToolAccess {
    ResolvedCharacterCardToolAccess {
        customEnabled: access.customEnabled,
        effectiveBuiltinToolVisibility: access.effectiveBuiltinToolVisibility,
        allowedPackageNames: access.allowedPackageNames,
        allowedSkillNames: access.allowedSkillNames,
        allowedMcpServerNames: access.allowedMcpServerNames,
        canUsePackageSystem: access.canUsePackageSystem,
        hasAnyAllowedExternalSource: access.hasAnyAllowedExternalSource,
    }
}

/// Converts provider prompt categories into the shared model shape.
fn providerCategoryToModel(
    category: ProviderToolPrompts::SystemToolPromptCategory,
) -> SystemToolPromptCategory {
    SystemToolPromptCategory {
        categoryName: category.category_name,
        categoryHeader: category.category_header,
        tools: category.tools.into_iter().map(providerToolToModel).collect(),
        categoryFooter: category.category_footer,
    }
}

/// Converts provider tool prompts into the shared model shape.
fn providerToolToModel(tool: ProviderToolPrompts::ToolPrompt) -> ToolPrompt {
    ToolPrompt {
        name: tool.name,
        description: tool.description,
        parameters: tool.parameters,
        parametersStructured: Some(
            tool.parameters_structured
                .into_iter()
                .map(providerParameterToModel)
                .collect(),
        ),
        details: tool.details,
        notes: tool.notes,
    }
}

/// Converts provider parameter schemas into the shared model shape.
fn providerParameterToModel(
    parameter: ProviderToolPrompts::ToolParameterSchema,
) -> ToolParameterSchema {
    ToolParameterSchema {
        name: parameter.name,
        r#type: parameter.value_type,
        description: parameter.description,
        required: parameter.required,
        default: parameter.default,
    }
}

/// Returns a static view of runtime built-in package assets.
fn runtimeBuiltinPluginAssets() -> &'static [RuntimePluginAsset] {
    static ASSETS: OnceLock<Vec<RuntimePluginAsset>> = OnceLock::new();
    ASSETS
        .get_or_init(|| {
            BUILTIN_PLUGIN_ASSETS
                .iter()
                .map(|asset| RuntimePluginAsset {
                    name: asset.name,
                    bytes: asset.bytes,
                })
                .collect()
        })
        .as_slice()
}

/// Returns a static view of bundled external package assets.
fn runtimeBundledExternalPluginAssets() -> &'static [RuntimePluginAsset] {
    static ASSETS: OnceLock<Vec<RuntimePluginAsset>> = OnceLock::new();
    ASSETS
        .get_or_init(|| {
            BUNDLED_EXTERNAL_PLUGIN_ASSETS
                .iter()
                .map(|asset| RuntimePluginAsset {
                    name: asset.name,
                    bytes: asset.bytes,
                })
                .collect()
        })
        .as_slice()
}

/// Returns a static view of bundled external skill assets.
fn runtimeBundledExternalSkillAssets() -> &'static [RuntimeBundledExternalSkillAsset] {
    static ASSETS: OnceLock<Vec<RuntimeBundledExternalSkillAsset>> = OnceLock::new();
    ASSETS
        .get_or_init(|| {
            BUNDLED_EXTERNAL_SKILL_ASSETS
                .iter()
                .map(|asset| RuntimeBundledExternalSkillAsset {
                    skillName: asset.skill_name,
                    path: asset.path,
                    bytes: asset.bytes,
                })
                .collect()
        })
        .as_slice()
}

/// Converts tool crate edit operations into provider edit operations.
fn runtimeEditOperationToProvider(
    operation: &RuntimeStructuredEditOperation,
) -> StructuredEditOperation {
    StructuredEditOperation {
        action: match operation.action {
            RuntimeStructuredEditAction::REPLACE => StructuredEditAction::REPLACE,
            RuntimeStructuredEditAction::DELETE => StructuredEditAction::DELETE,
        },
        oldContent: operation.oldContent.clone(),
        newContent: operation.newContent.clone(),
    }
}
