use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};

use operit_host_api::HostEnvironmentDescriptor;
use operit_model::FunctionType::FunctionType;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_model::ToolPrompt::{SystemToolPromptCategory, ToolParameterSchema, ToolPrompt};
use operit_providers::chat::config::FunctionalPrompts::FunctionalPrompts;
use operit_providers::chat::config::SystemToolPrompts as ProviderToolPrompts;
use operit_providers::chat::enhance::FileBindingService::{
    FileBindingService, StructuredEditAction, StructuredEditOperation,
};
use operit_providers::chat::EnhancedAIService::{EnhancedAIService, SendMessageOptions};
use operit_providers::runtime_support::ProviderRuntimeContext;
use operit_tools::runtime_support::{
    CachedMcpToolInfo, ResolvedCharacterCardToolAccess, RuntimeBundledExternalSkillAsset,
    RuntimeCharacterCardInfo, RuntimeCharacterMemoryBinding, RuntimeChatSendRequest,
    RuntimeChatSlot, RuntimePluginAsset, RuntimeSkillCatalogEntry, RuntimeStructuredEditAction,
    RuntimeStructuredEditOperation, ToolRuntimeSupport, ToolRuntimeSupportFuture,
};
use operit_tools::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::skill::SkillManager::SkillManager;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_util::stream::Stream::Stream;
use tokio::sync::Mutex as AsyncMutex;

use crate::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use crate::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::CharacterCardToolAccessResolver::CharacterCardToolAccessResolver;
use crate::data::preferences::EnvPreferences::EnvPreferences;
use crate::data::preferences::MemorySearchSettingsPreferences::MemorySearchSettingsPreferences;
use crate::data::preferences::SkillVisibilityPreferences::SkillVisibilityPreferences;
use crate::plugins::BuiltinPluginAssets::{BUILTIN_PLUGIN_ASSETS, BUNDLED_EXTERNAL_PLUGIN_ASSETS};
use crate::plugins::BundledExternalSkillAssets::BUNDLED_EXTERNAL_SKILL_ASSETS;

/// Creates runtime-backed services required by the tools crate.
pub struct ToolRuntimeSupportService;

impl ToolRuntimeSupportService {
    /// Creates tool runtime support for one application instance.
    pub fn create(
        hostManager: operit_host_api::HostManager::HostManager,
        chatRuntimeHolder: Arc<AsyncMutex<ChatRuntimeHolder>>,
    ) -> Arc<RuntimeToolSupport> {
        Arc::new(RuntimeToolSupport {
            hostManager,
            chatRuntimeHolder,
            runtimeBindings: OnceLock::new(),
        })
    }
}

#[derive(Clone)]
struct RuntimeToolBindings {
    toolHandler: AIToolHandler,
    providerRuntimeContext: ProviderRuntimeContext,
}

/// Bridges tool-owned interfaces to runtime-owned managers and registries.
pub struct RuntimeToolSupport {
    hostManager: operit_host_api::HostManager::HostManager,
    chatRuntimeHolder: Arc<AsyncMutex<ChatRuntimeHolder>>,
    runtimeBindings: OnceLock<RuntimeToolBindings>,
}

impl RuntimeToolSupport {
    /// Binds services that depend on the tool runtime support instance itself.
    pub fn bindRuntimeServices(
        &self,
        toolHandler: AIToolHandler,
        providerRuntimeContext: ProviderRuntimeContext,
    ) -> Result<(), String> {
        self.runtimeBindings
            .set(RuntimeToolBindings {
                toolHandler,
                providerRuntimeContext,
            })
            .map_err(|_| "Tool runtime services are already bound".to_string())
    }

    /// Returns services bound to this runtime support instance.
    fn runtimeBindings(&self) -> Result<&RuntimeToolBindings, String> {
        self.runtimeBindings
            .get()
            .ok_or_else(|| "Tool runtime services are not bound".to_string())
    }
}

impl ToolRuntimeSupport for RuntimeToolSupport {
    /// Resolves role-card tool access through runtime preferences.
    #[allow(non_snake_case)]
    fn resolveCharacterCardToolAccess(
        &self,
        roleCardId: Option<&str>,
        packageManager: &RuntimePackageManager,
        globalToolVisibility: Option<HashMap<String, bool>>,
    ) -> ResolvedCharacterCardToolAccess {
        CharacterCardToolAccessResolver::getInstance().resolve(
            roleCardId,
            packageManager,
            globalToolVisibility,
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
        categories
            .into_iter()
            .map(providerCategoryToModel)
            .collect()
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
        SkillManager::fromDefaultPaths(
            self.hostManager
                .fileSystemHost
                .clone()
                .expect("ToolRuntimeSupportService requires a FileSystemHost"),
        )
        .getAvailableSkills()
        .into_iter()
        .filter(|(name, _)| SkillVisibilityPreferences::getInstance().isSkillVisibleToAi(name))
        .map(|(name, skill)| RuntimeSkillCatalogEntry {
            name,
            description: skill.description,
        })
        .collect()
    }

    /// Returns cached MCP tool descriptions for a server.
    #[allow(non_snake_case)]
    fn cachedMcpTools(&self, serverName: &str) -> Vec<CachedMcpToolInfo> {
        MCPLocalServer::getInstance(&self.hostManager)
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
            let bindings = self.runtimeBindings()?.clone();
            let mut service =
                EnhancedAIService::new(bindings.toolHandler, bindings.providerRuntimeContext);
            let mut options = SendMessageOptions::new();
            options.message = FunctionalPrompts::packageDescriptionUserPrompt(
                pluginName,
                &toolDescriptions.join("\n"),
                true,
            );
            options.customSystemPromptTemplate =
                Some(FunctionalPrompts::packageDescriptionSystemPrompt(true).to_string());
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
            stream.collect(&mut |chunk| output.push_str(&chunk)).await;
            Ok(output.trim().to_string())
        })
    }

    /// Starts parent-owned chat services.
    #[allow(non_snake_case)]
    fn startChatServices(&self) -> Result<(), String> {
        let mut holder = self
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy".to_string())?;
        holder.getCore(ChatRuntimeSlot::MAIN);
        holder.getCore(ChatRuntimeSlot::FLOATING);
        holder.observeStats();
        Ok(())
    }

    /// Stops parent-owned chat services.
    #[allow(non_snake_case)]
    fn stopChatServices(&self) -> Result<(), String> {
        let mut holder = self
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy".to_string())?;
        holder.cores.clear();
        holder.activeConversationCount = 0;
        holder.currentSessionToolCount = 0;
        Ok(())
    }

    /// Returns whether the requested chat is currently processing.
    #[allow(non_snake_case)]
    fn isChatProcessing(&self, chatId: &str) -> Result<bool, String> {
        let holder = self
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy".to_string())?;
        Ok(holder
            .cores
            .values()
            .any(|core| core.activeStreamingChatIds().iter().any(|id| id == chatId)))
    }

    /// Switches the parent-owned main chat runtime to a chat id.
    #[allow(non_snake_case)]
    fn switchMainChat(&self, chatId: &str) -> Result<(), String> {
        let mut holder = self
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy".to_string())?;
        holder
            .getCore(ChatRuntimeSlot::MAIN)
            .switchChat(chatId.to_string());
        holder.syncMainChatSelectionToFloating(chatId.to_string());
        holder.observeStats();
        Ok(())
    }

    /// Creates a chat through the parent-owned chat runtime.
    #[allow(non_snake_case)]
    fn createChatRuntime(
        &self,
        characterCardName: Option<String>,
        group: Option<String>,
        setAsCurrentChat: bool,
    ) -> Result<(), String> {
        let mut holder = self
            .chatRuntimeHolder
            .try_lock()
            .map_err(|_| "Chat runtime holder is busy".to_string())?;
        holder.getCore(ChatRuntimeSlot::MAIN).createNewChat(
            characterCardName,
            group,
            false,
            setAsCurrentChat,
            None,
        );
        holder.observeStats();
        Ok(())
    }

    /// Sends a message through the parent-owned chat runtime.
    #[allow(non_snake_case)]
    fn sendChatMessage<'a>(
        &'a self,
        request: RuntimeChatSendRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let mut holder = self.chatRuntimeHolder.lock().await;
            let slot = runtimeChatSlotToRuntimeSlot(request.slot);
            holder
                .getCore(slot)
                .sendUserMessage(
                    PromptFunctionType::CHAT,
                    request.roleCardId,
                    request.chatId,
                    request.message,
                    request.proxySenderName,
                    None,
                    None,
                    Vec::new(),
                    None,
                    request.turnOptions,
                )
                .await;
            holder.observeStats();
            Ok(())
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
    fn characterMemoryBinding(
        &self,
        cardId: &str,
    ) -> Result<RuntimeCharacterMemoryBinding, String> {
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

/// Converts provider prompt categories into the shared model shape.
fn providerCategoryToModel(
    category: ProviderToolPrompts::SystemToolPromptCategory,
) -> SystemToolPromptCategory {
    SystemToolPromptCategory {
        categoryName: category.category_name,
        categoryHeader: category.category_header,
        tools: category
            .tools
            .into_iter()
            .map(providerToolToModel)
            .collect(),
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

/// Converts a tool runtime slot into the runtime holder slot.
#[allow(non_snake_case)]
fn runtimeChatSlotToRuntimeSlot(slot: RuntimeChatSlot) -> ChatRuntimeSlot {
    match slot {
        RuntimeChatSlot::MAIN => ChatRuntimeSlot::MAIN,
        RuntimeChatSlot::FLOATING => ChatRuntimeSlot::FLOATING,
    }
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
