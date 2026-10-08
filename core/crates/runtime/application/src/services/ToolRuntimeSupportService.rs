use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, OnceLock, RwLock};

use operit_host_api::HostEnvironmentDescriptor;
use operit_model::FunctionType::FunctionType;
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::{SystemToolPromptCategory, ToolParameterSchema, ToolPrompt};
use operit_providers::chat::config::FunctionalPrompts::FunctionalPrompts;
use operit_providers::chat::config::SystemToolPrompts as ProviderToolPrompts;
use operit_providers::chat::enhance::FileBindingService::{
    FileBindingService, StructuredEditAction, StructuredEditOperation,
};
use operit_providers::chat::llmprovider::AIService::SendMessageRequest;
use operit_providers::chat::EnhancedAIService::EnhancedAIService;
use operit_providers::runtime_support::ProviderRuntimeContext;
use operit_tools::runtime_support::{
    CachedMcpToolInfo, EdgeToolRuntime, CoreNodeToolRuntime, CoreRouteChangeHandler, CoreRouteResumeContext,
    RuntimeBundledExternalSkillAsset, RuntimeChatCallRequest, RuntimeChatSendRequest,
    RuntimeChatSlot, RuntimeCoreNodeRouteState, RuntimePluginAsset, RuntimeSkillCatalogEntry,
    RuntimeStructuredEditAction, RuntimeStructuredEditOperation, ToolRuntimeSupport,
    ToolRuntimeSupportFuture,
};
use operit_tools::tools::mcp_runtime::MCPLocalServer::MCPLocalServer;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::skill::SkillManager::SkillManager;
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_util::stream::Stream::Stream;
use tokio::sync::Mutex as AsyncMutex;

use crate::services::core::ChatTurnTransport::{
    AdmittedChatTurn, AdmittedChatTurnWork, ChatTurnTransport, SharedChatTurnTransport,
};
use crate::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
use crate::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
use crate::data::preferences::EnvPreferences::EnvPreferences;
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
            chatTurnTransport: Arc::new(std::sync::Mutex::new(ChatTurnTransport::default())),
            runtimeBindings: OnceLock::new(),
            chatHistoryManager: OnceLock::new(),
            coreNodeToolRuntime: RwLock::new(None),
            edgeToolRuntime: OnceLock::new(),
            coreRouteChangeHandler: RwLock::new(None),
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
    chatTurnTransport: SharedChatTurnTransport,
    runtimeBindings: OnceLock<RuntimeToolBindings>,
    chatHistoryManager: OnceLock<operit_store::repository::ChatHistoryManager::ChatHistoryManager>,
    coreNodeToolRuntime: RwLock<Option<Arc<dyn CoreNodeToolRuntime>>>,
    edgeToolRuntime: OnceLock<Arc<dyn EdgeToolRuntime>>,
    coreRouteChangeHandler: RwLock<Option<CoreRouteChangeHandler>>,
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

    /// Captures the canonical manager shared with this runtime's chat delegates and execution leases.
    pub fn bindChatHistoryManager(
        &self,
        manager: operit_store::repository::ChatHistoryManager::ChatHistoryManager,
    ) -> Result<(), String> {
        self.chatHistoryManager
            .set(manager)
            .map_err(|_| "Tool runtime chat record store is already bound".to_string())
    }

    /// Returns the captured record manager without reacquiring a chat holder or resolving global store roots.
    fn runtimeChatHistoryManager(
        &self,
    ) -> Result<&operit_store::repository::ChatHistoryManager::ChatHistoryManager, String> {
        self.chatHistoryManager
            .get()
            .ok_or_else(|| "Tool runtime chat record store is not bound".to_string())
    }

    /// Opens the canonical ordinary appearance manager with this runtime's actual preference-storage host.
    fn runtimeThemeConfigManager(&self) -> Result<crate::data::preferences::ThemeConfigManager::ThemeConfigManager, String> {
        let storageHost = self.hostManager.runtimeStorageHost.clone()
            .ok_or_else(|| "RuntimeStorageHost is not registered for theme configuration".to_string())?;
        Ok(crate::data::preferences::ThemeConfigManager::ThemeConfigManager::new(storageHost))
    }

    /// Returns services bound to this runtime support instance.
    fn runtimeBindings(&self) -> Result<&RuntimeToolBindings, String> {
        self.runtimeBindings
            .get()
            .ok_or_else(|| "Tool runtime services are not bound".to_string())
    }
}

impl ToolRuntimeSupport for RuntimeToolSupport {
    fn bindEdgeToolRuntime(&self, runtime: Arc<dyn EdgeToolRuntime>) -> Result<(), String> {
        self.edgeToolRuntime.set(runtime).map_err(|_| "Edge tool routing is already initialized".into())
    }
    fn executeEdgeTool(&self, nodeId: String, request: operit_link::CoreCallRequest)
        -> operit_plugin_sdk::javascript::JsExecutionCompletion<Result<serde_json::Value, String>> {
        match self.edgeToolRuntime.get() {
            Some(runtime) => runtime.execute(nodeId, request),
            None => Box::pin(async {Err("Edge tool routing is not initialized".into())}),
        }
    }

    /// Installs the live routing capability provided by the outer Core proxy.
    #[allow(non_snake_case)]
    fn bindCoreNodeToolRuntime(&self, runtime: Arc<dyn CoreNodeToolRuntime>) -> Result<(), String> {
        *self
            .coreNodeToolRuntime
            .write()
            .map_err(|error| format!("CoreNode tool runtime lock poisoned: {error}"))? =
            Some(runtime);
        Ok(())
    }

    /// Reads the current routing state from the installed outer Core proxy.
    #[allow(non_snake_case)]
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String> {
        let runtime = self
            .coreNodeToolRuntime
            .read()
            .map_err(|error| format!("CoreNode tool runtime lock poisoned: {error}"))?
            .clone()
            .ok_or_else(|| "CoreNode routing is not initialized".to_string())?;
        runtime.coreNodeRouteState()
    }

    /// Installs the route change controller supplied by the owning Core application.
    #[allow(non_snake_case)]
    fn bindCoreRouteChangeHandler(&self, handler: CoreRouteChangeHandler) -> Result<(), String> {
        let mut current = self
            .coreRouteChangeHandler
            .write()
            .map_err(|error| format!("Core route change handler lock poisoned: {error}"))?;
        if current.is_some() {
            return Err("Core route change handler is already bound".to_string());
        }
        *current = Some(handler);
        Ok(())
    }

    /// Requests a route change through the installed Core application controller.
    #[allow(non_snake_case)]
    fn requestCoreRouteChange<'a>(
        &'a self,
        chatId: String,
        targetNodeId: String,
        resumeContext: CoreRouteResumeContext,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>> {
        let handler = self
            .coreRouteChangeHandler
            .read()
            .expect("Core route change handler lock must not be poisoned")
            .clone();
        Box::pin(async move {
            let handler =
                handler.ok_or_else(|| "Core route change handler is not bound".to_string())?;
            handler(chatId, targetNodeId, resumeContext).await
        })
    }

    /// Reads complete real configured model records and validates the public SDK shape.
    #[allow(non_snake_case)]
    fn listModelSummaries(
        &self,
    ) -> Result<Vec<operit_plugin_sdk::js_sdk::software_settings::SoftwareModelSummary>, String>
    {
        let summaries = crate::data::preferences::ModelConfigManager::ModelConfigManager::default()
            .getAllModelSummaries()
            .map_err(|error| error.to_string())?;
        serde_json::from_value(serde_json::to_value(summaries).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    /// Reads complete real configured speech records without consulting character preferences.
    #[allow(non_snake_case)]
    fn listTtsConfigs(
        &self,
    ) -> Result<Vec<operit_plugin_sdk::js_sdk::software_settings::SoftwareTtsConfig>, String> {
        let configs = crate::data::preferences::TtsConfigManager::TtsConfigManager::getInstance()
            .getAllTtsConfigs()?;
        serde_json::from_value(serde_json::to_value(configs).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    /// Reads full named appearance records from the canonical ordinary preference ledger.
    #[allow(non_snake_case)]
    fn listThemeConfigs(&self) -> Result<Vec<operit_plugin_sdk::js_sdk::software_settings::SoftwareThemeConfig>, String> {
        let configs = self.runtimeThemeConfigManager()?.list().map_err(|error| error.to_string())?;
        serde_json::from_value(serde_json::to_value(configs).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    /// Applies only an exact existing named theme using the canonical ordinary appearance transaction.
    #[allow(non_snake_case)]
    fn applyThemeConfig(&self, id: String) -> Result<operit_plugin_sdk::js_sdk::software_settings::SoftwareThemeConfig, String> {
        if id.trim().is_empty() || id.trim() != id {
            return Err("Theme configuration ID must be exact nonblank text".to_string());
        }
        let config = self.runtimeThemeConfigManager()?.apply(id).map_err(|error| error.to_string())?;
        serde_json::from_value(serde_json::to_value(config).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())
    }

    /// Reads the current canonical speech ID without interpreting role references or plugin ownership.
    #[allow(non_snake_case)]
    fn getCurrentTtsConfigId(&self) -> Result<String, String> {
        crate::data::preferences::TtsConfigManager::TtsConfigManager::getInstance().getCurrentTtsConfigId()
    }

    /// Validates the exact speech identity before calling the existing canonical persistence setter.
    #[allow(non_snake_case)]
    fn setCurrentTtsConfigId(&self, id: String) -> Result<String, String> {
        if id.trim().is_empty() || id.trim() != id {
            return Err("TTS configuration ID must be exact nonblank text".to_string());
        }
        crate::data::preferences::TtsConfigManager::TtsConfigManager::getInstance().setCurrentTtsConfigId(&id)
    }

    /// Applies the same actual tool-prompt registrations to the CLI model-facing catalog.
    #[allow(non_snake_case)]
    fn filterToolCatalog<'a>(&'a self, entries: Vec<operit_tools::tools::climode::CliToolModeSupport::HiddenToolCatalogEntry>, context: operit_tools::ToolExecutionManager::ToolRuntimeContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<operit_tools::tools::climode::CliToolModeSupport::HiddenToolCatalogEntry>, String>> + Send + 'a>>{
        let manager = self
            .runtimeBindings()
            .map(|bindings| bindings.toolHandler.getOrCreatePackageManager());
        Box::pin(async move {
            let manager = RuntimePackageManager::readySnapshot(manager?).await?;
            crate::plugins::toolpkg::ToolPkgToolLifecycleBridge::ToolPkgToolLifecycleBridge::filterToolCatalog(&manager, entries, &context).await
        })
    }

    /// Reads each actual canonical tool-source registry after readiness, without heuristic name classification.
    #[allow(non_snake_case)]
    fn readToolSourceCatalog<'a>(
        &'a self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        operit_plugin_sdk::js_sdk::software_settings::SoftwareToolSourceCatalog,
                        String,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            use operit_plugin_sdk::js_sdk::software_settings::{
                SoftwareToolSource, SoftwareToolSourceCatalog,
            };
            let mut handler = self.runtimeBindings()?.toolHandler.clone();
            let manager =
                RuntimePackageManager::readySnapshot(handler.getOrCreatePackageManager()).await?;
            handler.registerDefaultTools();
            let registered = handler
                .getAllToolNames()
                .into_iter()
                .collect::<std::collections::HashSet<_>>();
            let categories =
                self.buildBuiltinAndInternalCategories(false, &self.hostManager.hostEnvironment);
            let mut builtinTools = BTreeMap::new();
            for category in categories {
                for tool in category.tools {
                    if !registered.contains(&tool.name) {
                        continue;
                    }
                    let entry = SoftwareToolSource {
                        name: tool.name.clone(),
                        displayName: tool.name.clone(),
                        description: tool.description,
                    };
                    if builtinTools.insert(tool.name.clone(), entry).is_some() {
                        return Err(format!(
                            "Duplicate builtin tool catalog entry: {}",
                            tool.name
                        ));
                    }
                }
            }
            let packages = manager
                .getAvailablePackages()
                .into_iter()
                .filter(|(name, _)| !manager.isToolPkgContainer(name))
                .map(|(name, package)| SoftwareToolSource {
                    name,
                    displayName: package.display_name.resolve(false),
                    description: package.description.resolve(false),
                })
                .collect();
            let (skillPackages, errors) = SkillManager::fromDefaultPaths(manager.fileSystemHost())
                .getAvailableSkillsSnapshot();
            if !errors.is_empty() {
                return Err(serde_json::to_string(&errors).map_err(|error| error.to_string())?);
            }
            let skills = skillPackages
                .into_iter()
                .map(|(name, skill)| SoftwareToolSource {
                    displayName: name.clone(),
                    name,
                    description: skill.description,
                })
                .collect();
            let mcpServers = manager
                .getAvailableServerPackages()
                .into_iter()
                .map(|(name, server)| SoftwareToolSource {
                    displayName: server.name,
                    name,
                    description: server.description,
                })
                .collect();
            Ok(SoftwareToolSourceCatalog {
                builtinTools: builtinTools.into_values().collect(),
                packages,
                skills,
                mcpServers,
            })
        })
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

    /// Generates an MCP description through the functional-model capability without a conversation profile.
    #[allow(non_snake_case)]
    fn generateMcpPluginDescription<'a>(
        &'a self,
        pluginName: &'a str,
        toolDescriptions: &'a [String],
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let output = self
                .callChatModel(RuntimeChatCallRequest {
                    functionType: FunctionType::CHAT,
                    turns: vec![
                        PromptTurn {
                            kind: PromptTurnKind::SYSTEM,
                            content: FunctionalPrompts::packageDescriptionSystemPrompt(true)
                                .to_string(),
                            tool_name: None,
                            metadata: HashMap::new(),
                        },
                        PromptTurn {
                            kind: PromptTurnKind::USER,
                            content: FunctionalPrompts::packageDescriptionUserPrompt(
                                pluginName,
                                &toolDescriptions.join("\n"),
                                true,
                            ),
                            tool_name: None,
                            metadata: HashMap::new(),
                        },
                    ],
                    recordTokenUsage: true,
                    enableThinking: false,
                })
                .await?;
            if output.trim().is_empty() {
                return Err("MCP description generation returned no content".to_string());
            }
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

    /// Switches the main chat only after its registered configuration owner has resolved the actual binding.
    #[allow(non_snake_case)]
    fn switchMainChat<'a>(
        &'a self,
        chatId: &'a str,
    ) -> ToolRuntimeSupportFuture<'a, Result<(), String>> {
        Box::pin(async move {
            let mut holder = self.chatRuntimeHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .switchChat(chatId.to_string())
                .await?;
            holder.observeStats();
            Ok(())
        })
    }

    /// Creates a workspace conversation through its explicit source and creation-hook input.
    #[allow(non_snake_case)]
    fn createChatRuntime<'a>(
        &'a self,
        setAsCurrentChat: bool,
        sourceChatId: Option<String>,
        input: Option<serde_json::Value>,
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let mut holder = self.chatRuntimeHolder.lock().await;
            let chatId = holder
                .getCore(ChatRuntimeSlot::MAIN)
                .createNewChat(setAsCurrentChat, sourceChatId, input)
                .await?;
            holder.observeStats();
            Ok(chatId)
        })
    }

    /// Reads one owner's extension directly from the canonical record without acquiring a chat holder lock.
    fn readChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
    ) -> Result<Option<operit_plugin_sdk::js_sdk::core::JsonObject>, String> {
        let target = canonicalExtensionTarget(target)?;
        let value = self
            .runtimeChatHistoryManager()?
            .readPluginExtension(owner, &target)
            .map_err(|error| error.to_string())?;
        value
            .map(|value| match value {
                serde_json::Value::Object(fields) => Ok(fields.into_iter().collect()),
                _ => Err("Stored chat extension must be a JSON object".to_string()),
            })
            .transpose()
    }

    /// Replaces only one owner namespace through the record transaction and its execution guard.
    fn writeChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
        value: operit_plugin_sdk::js_sdk::core::JsonObject,
    ) -> Result<operit_plugin_sdk::js_sdk::core::JsonObject, String> {
        let target = canonicalExtensionTarget(target)?;
        let persistedValue = serde_json::Value::Object(
            value
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        );
        self.runtimeChatHistoryManager()?
            .writePluginExtension(owner, &target, persistedValue)
            .map_err(|error| error.to_string())?;
        Ok(value)
    }

    /// Deletes only one owner namespace through the record transaction and its execution guard.
    fn deleteChatExtension(
        &self,
        owner: &str,
        target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
    ) -> Result<bool, String> {
        let target = canonicalExtensionTarget(target)?;
        self.runtimeChatHistoryManager()?
            .deletePluginExtension(owner, &target)
            .map_err(|error| error.to_string())
    }

    /// Admits exactly one authenticated send and owns its generation, finalized receipt and cleanup.
    fn openPluginChatMessage(
        &self, owner: &str, request: operit_plugin_sdk::js_sdk::chat::ChatSendRequest, observeParts: bool,
    ) -> Result<operit_plugin_sdk::js_sdk::JsAsyncIterable<operit_plugin_sdk::js_sdk::chat::ChatSendEvent>, String> {
        use operit_plugin_sdk::js_sdk::chat::ChatSendRequest;
        use crate::services::core::ChatSendStream::ChatSendObservation;
        let chatId = match &request { ChatSendRequest::Submit { chatId, .. } | ChatSendRequest::Continue { chatId, .. } => chatId.clone() };
        let (observation, iterator) = ChatSendObservation::create(chatId);
        let admitted = self.chatTurnTransport.lock().map_err(|_| "Native chat transport mutex poisoned".to_string())?
            .admit(owner, request, self.runtimeChatHistoryManager()?, if observeParts { Some(observation.clone()) } else { None })?;
        let executionId = admitted.handle.executionId.clone();
        let owner = owner.to_string();
        let taskOwner = owner.clone();
        let holder = self.chatRuntimeHolder.clone();
        let transport = self.chatTurnTransport.clone();
        let schedule = operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask("native-chat-send", Box::new(move || Box::pin(async move {
                // Keep native lease cleanup armed even when finalization returns an actual error.
                let _executionGuard = admitted.lease.requestGuard();
                let result = runAdmittedChatTurn(holder.clone(), &admitted).await;
                let captured = {
                    let mut runtime = holder.lock().await;
                    runtime.getCore(admitted.slot.clone()).messageProcessingDelegate
                        .nativeCommittedTurnReceipt(&admitted.handle.chatId, &admitted.handle.executionId)
                };
                let (result, captured) = match captured {
                    Ok(captured) => (result, captured),
                    Err(capture) => (Err(match result { Ok(_) => capture,
                        Err(error) => format!("{error}; native commit receipt capture failed: {capture}") }), None),
                };
                let finalized = finalizeAdmittedChatSend(&taskOwner, holder, transport, &admitted, result, captured).await;
                if let Err(error) = &finalized { operit_util::AppLogger::AppLogger::e("NativeChatSend", error); }
                observation.finish(finalized);
            })));
        if let Err(error) = schedule {
            let message = format!("Native chat send scheduling failed: {error}");
            self.chatTurnTransport.lock().map_err(|_| format!("{message}; native transport mutex poisoned"))?
                .rejectScheduling(&owner, &executionId, message.clone())?;
            return Err(message);
        }
        Ok(iterator)
    }

    /// Captures exact ownership synchronously so queued cancellation cannot target a later execution.
    fn requestPluginChatCancellation(
        &self, owner: &str, chatId: &str,
    ) -> Result<operit_plugin_sdk::js_sdk::chat::ChatCancelResult, String> {
        use operit_plugin_sdk::js_sdk::chat::ChatCancelResult;
        let captured = {
            let mut transport = self.chatTurnTransport.lock().map_err(|_| "Native chat transport mutex poisoned".to_string())?;
            match transport.ownedActiveExecution(owner, chatId)? {
                Some(executionId) => transport.requestCancellation(owner, &executionId)?.map(|(chatId, slot)| (executionId, chatId, slot)),
                None => None,
            }
        };
        let Some((executionId, capturedChatId, slot)) = captured else {
            return Ok(ChatCancelResult { chatId: chatId.to_string(), cancelRequested: false });
        };
        let holder = self.chatRuntimeHolder.clone();
        operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask("native-chat-cancel", Box::new(move || Box::pin(async move {
                let mut runtime = holder.lock().await;
                if let Err(error) = runtime.getCore(slot).cancelNativeChatExecution(&capturedChatId, &executionId).await {
                    operit_util::AppLogger::AppLogger::e("NativeChatCancel", &error);
                }
                runtime.observeStats();
            }))).map_err(|error| format!("Native chat cancellation scheduling failed: {error}"))?;
        Ok(ChatCancelResult { chatId: chatId.to_string(), cancelRequested: true })
    }

    /// Invokes authoritative submit hooks outside Holder and awaits only the originating turn's actual commit receipt.
    #[allow(non_snake_case)]
    fn sendChatMessage<'a>(
        &'a self, mut request: RuntimeChatSendRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<operit_plugin_sdk::js_sdk::results::MessageSendResultData, String>> {
        Box::pin(async move {
            use crate::plugins::toolpkg::ToolPkgChatInputHookBridge::{ToolPkgChatInputHookBridge, CHAT_INPUT_EVENT_SUBMITTED,
                CHAT_INPUT_SUBMIT_ACTION_BLOCK, CHAT_INPUT_SUBMIT_ACTION_CONSUME, CHAT_INPUT_SUBMIT_ACTION_ALLOW, CHAT_INPUT_SUBMIT_ACTION_REPLACE};
            use operit_plugin_sdk::js_sdk::results::{JsOptional, MessageSendOutcome, MessageSendResultData};
            let slot = runtimeChatSlotToRuntimeSlot(request.slot);
            let mut context = {
                let mut holder = self.chatRuntimeHolder.lock().await;
                let core = holder.getCore(slot);
                let chatId = match request.chatId.clone() {
                    Some(chatId) => chatId,
                    None => core.chatHistoryDelegate.currentChatIdFlow.value()
                        .ok_or_else(|| "Chat send has no selected conversation".to_string())?,
                };
                core.prepareChatInputSubmit(chatId, request.message.clone(), request.attachments.clone(), request.replyToMessageTimestamp)?
            };
            context.source = "Tool".to_string();
            context.notifyReply = request.turnOptions.notifyReply != Some(false);
            request.chatId = Some(context.chatId.clone());
            if request.turnOptions.continuation.is_none() {
                if let Some(decision) = ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(context.clone()).await? {
                    match decision.action.as_str() {
                        CHAT_INPUT_SUBMIT_ACTION_BLOCK | CHAT_INPUT_SUBMIT_ACTION_CONSUME => {
                            let outcome = if decision.action == CHAT_INPUT_SUBMIT_ACTION_CONSUME {
                                MessageSendOutcome::Consumed { metadata: decision.metadata.into_iter().collect() }
                            } else { MessageSendOutcome::Blocked { message: decision.message } };
                            return Ok(MessageSendResultData { chatId: context.chatId, message: request.message,
                                aiResponse: JsOptional::Null, receivedAt: JsOptional::Null,
                                sentAt: operit_host_api::TimeUtils::currentTimeMillis(), outcome });
                        }
                        CHAT_INPUT_SUBMIT_ACTION_ALLOW | CHAT_INPUT_SUBMIT_ACTION_REPLACE => {
                            if let Some(text) = decision.text { request.message = text; }
                        }
                        _ => return Err("Chat input returned an unvalidated submit action".to_string()),
                    }
                }
                context.eventName = CHAT_INPUT_EVENT_SUBMITTED.to_string();
                context.text = request.message.clone();
                context.selectionStart = context.text.chars().count() as i32;
                context.selectionEnd = context.selectionStart;
                ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(context.clone()).await?;
            }
            // Only this native, successfully awaited dispatch may mark the submit as handled.
            request.turnOptions.chatInputSubmitRequestedHandled = true;
            let submission = {
                let mut holder = self.chatRuntimeHolder.lock().await;
                let core = holder.getCore(slot);
                let reply = request.replyToMessageTimestamp.map(|timestamp| core.chatHistoryDelegate.chatHistoryManager
                    .loadChatMessageVariant(&context.chatId, timestamp, 0).map_err(|error| error.to_string())).transpose()?;
                let submission = core.startUserMessage(PromptFunctionType::CHAT, request.participantId,
                    Some(context.chatId), request.message, request.proxySenderName, None, None,
                    request.attachments, reply, request.turnOptions, None).await?;
                holder.observeStats();
                submission
            };
            submission.wait().await
        })
    }

    /// Calls the configured functional model directly and keeps the request outside chat history.
    #[allow(non_snake_case)]
    fn callChatModel<'a>(
        &'a self,
        request: RuntimeChatCallRequest,
    ) -> ToolRuntimeSupportFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let bindings = self.runtimeBindings()?.clone();
            let mut service =
                EnhancedAIService::new(bindings.toolHandler, bindings.providerRuntimeContext);
            let (modelConfig, modelParameters, aiService) = service
                .multi_service_manager
                .getServiceBundleForFunction(request.functionType.clone())
                .map_err(|error| error.to_string())?;
            let thinkingQualityLevel = service
                .provider_runtime_context
                .support()
                .thinkingQualityLevel()
                .map_err(|error| error.to_string())?;
            let providerModel = {
                let provider = aiService.lock().await;
                provider.provider_model()
            };
            let mut response = {
                let mut provider = aiService.lock().await;
                provider
                    .send_message(SendMessageRequest {
                        chat_history: request.turns,
                        model_parameters: modelParameters,
                        enable_thinking: request.enableThinking,
                        thinking_quality_level: thinkingQualityLevel,
                        thinking_configurations: modelConfig.thinkingConfigurations,
                        thinking_option_id: modelConfig.thinkingOptionId,
                        stream: false,
                        available_tools: Vec::new(),
                        preserve_think_in_history: true,
                        enable_retry: false,
                        on_non_fatal_error: None,
                        on_tool_invocation: None,
                    })
                    .await
                    .map_err(|error| error.to_string())?
            };
            let mut output = String::new();
            response.collect(&mut |chunk| output.push_str(&chunk)).await;
            if request.recordTokenUsage {
                let (inputTokens, cachedInputTokens, outputTokens) = {
                    let provider = aiService.lock().await;
                    (
                        provider.input_token_count(),
                        provider.cached_input_token_count(),
                        provider.output_token_count(),
                    )
                };
                service
                    .provider_runtime_context
                    .support()
                    .updateTokensForProviderModel(
                        &providerModel,
                        inputTokens,
                        outputTokens,
                        cachedInputTokens,
                    )
                    .map_err(|error| error.to_string())?;
                operit_store::repository::UsageStatisticsStore::UsageStatisticsStore::new()
                    .recordProviderModelRequest(
                        providerModel,
                        request.functionType,
                        operit_store::repository::UsageStatisticsStore::UsageRequestSource::CHAT_RESPONSE,
                        None,
                        inputTokens,
                        outputTokens,
                        cachedInputTokens,
                    )
                    .map_err(|error| error.to_string())?;
            }
            Ok(output)
        })
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

/// Finalizes only the originating native records and always attempts the canonical lease release.
async fn finalizeAdmittedChatSend(
    owner: &str, holder: Arc<AsyncMutex<ChatRuntimeHolder>>, transport: SharedChatTurnTransport,
    admitted: &AdmittedChatTurn, result: Result<operit_plugin_sdk::js_sdk::results::MessageSendResultData, String>,
    captured: Option<operit_plugin_sdk::js_sdk::results::MessageSendResultData>,
) -> Result<operit_plugin_sdk::js_sdk::results::MessageSendResultData, String> {
    use operit_plugin_sdk::js_sdk::results::MessageSendOutcome;
    let prepared = {
        let mut registry = transport.lock().map_err(|_| "Native chat transport mutex poisoned".to_string())?;
        registry.settle(&admitted.handle.executionId, result.clone(), captured)?;
        registry.prepareFinish(owner, &admitted.handle.sequenceId)?
    };
    let (slot, notifyReply, receipt) = prepared;
    let publication = if matches!(&result, Ok(value) if matches!(value.outcome, MessageSendOutcome::Blocked { .. } | MessageSendOutcome::Consumed { .. })) {
        Ok(())
    } else {
        let mut runtime = holder.lock().await;
        let publication = runtime.getCore(slot).finishNativeChatSequence(&receipt, notifyReply, matches!(admitted.work, AdmittedChatTurnWork::Submit { .. }));
        runtime.observeStats();
        publication
    };
    let release = transport.lock().map_err(|_| "Native chat transport mutex poisoned".to_string())?
        .completeFinish(owner, &admitted.handle.sequenceId);
    let cleanup = match (publication, release) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(publication), Err(release)) => Err(format!("Chat finalization failed: {publication}; chat lease release failed: {release}")),
    };
    match (result, cleanup) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(format!("{error}; chat cleanup failed: {cleanup}")),
    }
}

/// Executes an admitted native turn through the same real pipeline while keeping hooks and terminal waits outside Holder.
async fn runAdmittedChatTurn(
    holder: Arc<AsyncMutex<ChatRuntimeHolder>>, admitted: &AdmittedChatTurn,
) -> Result<operit_plugin_sdk::js_sdk::results::MessageSendResultData, String> {
    use std::sync::atomic::Ordering;
    use operit_plugin_sdk::js_sdk::chat::ChatInitialTurn;
    use operit_plugin_sdk::js_sdk::results::{JsOptional, MessageSendOutcome, MessageSendResultData};
    use operit_model::ChatTurnOptions::{ChatTurnContinuation, ChatTurnOptions};
    use crate::plugins::toolpkg::ToolPkgChatInputHookBridge::{
        ToolPkgChatInputHookBridge, CHAT_INPUT_EVENT_SUBMITTED, CHAT_INPUT_SUBMIT_ACTION_ALLOW,
        CHAT_INPUT_SUBMIT_ACTION_REPLACE, CHAT_INPUT_SUBMIT_ACTION_BLOCK, CHAT_INPUT_SUBMIT_ACTION_CONSUME,
    };
    if admitted.cancelled.load(Ordering::Acquire) { return Ok(cancelledBeforeNativeTurn(admitted)); }
    let chatId = &admitted.handle.chatId;
    let (participantId, message, attachments, replyTimestamp, continuation) = match &admitted.work {
        AdmittedChatTurnWork::Submit { input, turn } => {
            let attachments = input.attachments.iter().map(|attachment| operit_model::AttachmentInfo::AttachmentInfo {
                filePath: attachment.filePath.clone(), nodeId: attachment.nodeId.as_value().cloned(), fileName: attachment.fileName.clone(),
                mimeType: attachment.mimeType.clone(), fileSize: attachment.fileSize, content: attachment.content.clone(),
            }).collect::<Vec<_>>();
            let mut context = {
                let mut runtime = holder.lock().await;
                runtime.getCore(admitted.slot.clone()).prepareChatInputSubmit(
                    chatId.clone(), input.text.clone(), attachments.clone(), input.replyToMessageTimestamp.as_value().copied())?
            };
            // The actual API origin is explicit; registered plugins decide their own recursive input policy.
            context.source = "Sequence".to_string();
            context.notifyReply = admitted.notifyReply;
            let mut text = input.text.clone();
            if let Some(decision) = ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(context.clone()).await? {
                match decision.action.as_str() {
                    CHAT_INPUT_SUBMIT_ACTION_BLOCK | CHAT_INPUT_SUBMIT_ACTION_CONSUME => {
                        let outcome = if decision.action == CHAT_INPUT_SUBMIT_ACTION_CONSUME {
                            MessageSendOutcome::Consumed { metadata: decision.metadata.into_iter().collect() }
                        } else { MessageSendOutcome::Blocked { message: decision.message } };
                        return Ok(MessageSendResultData { chatId: chatId.clone(), message: text,
                            aiResponse: JsOptional::Null, receivedAt: JsOptional::Null,
                            sentAt: operit_host_api::TimeUtils::currentTimeMillis(), outcome });
                    }
                    CHAT_INPUT_SUBMIT_ACTION_ALLOW | CHAT_INPUT_SUBMIT_ACTION_REPLACE => {
                        if let Some(replacement) = decision.text { text = replacement; }
                    }
                    _ => return Err("Native turn input returned an unvalidated submit action".to_string()),
                }
            }
            context.eventName = CHAT_INPUT_EVENT_SUBMITTED.to_string();
            context.text = text.clone();
            context.selectionStart = context.text.chars().count() as i32;
            context.selectionEnd = context.selectionStart;
            ToolPkgChatInputHookBridge::dispatchRegisteredChatInputHooks(context).await?;
            if admitted.cancelled.load(Ordering::Acquire) { return Ok(cancelledBeforeNativeTurn(admitted)); }
            match turn {
                ChatInitialTurn::RecordOnly => {
                    let mut runtime = holder.lock().await;
                    if admitted.cancelled.load(Ordering::Acquire) { return Ok(cancelledBeforeNativeTurn(admitted)); }
                    let result = runtime.getCore(admitted.slot.clone()).recordOnlySequenceInput(chatId, &text,
                        &attachments, input.replyToMessageTimestamp.as_value().copied(), &admitted.lease);
                    runtime.observeStats();
                    return result;
                }
                ChatInitialTurn::Execute { participantId } => (participantId.clone(), text, attachments, input.replyToMessageTimestamp.as_value().copied(), None),
            }
        }
        AdmittedChatTurnWork::Continue { userMessageTimestamp, participantId } =>
            (Some(participantId.clone()), String::new(), Vec::new(), None, Some(ChatTurnContinuation { userMessageTimestamp: *userMessageTimestamp })),
    };
    let submission = {
        let mut runtime = holder.lock().await;
        if admitted.cancelled.load(Ordering::Acquire) { return Ok(cancelledBeforeNativeTurn(admitted)); }
        let core = runtime.getCore(admitted.slot.clone());
        let reply = replyTimestamp.map(|timestamp| core.chatHistoryDelegate.chatHistoryManager
            .loadChatMessageVariant(chatId, timestamp, 0).map_err(|error| error.to_string())).transpose()?;
        let options = ChatTurnOptions {
            continuation, nativeExecutionId: Some(admitted.handle.executionId.clone()), outputObserver: admitted.outputObserver.clone(), deferSequenceCompletion: true,
            chatInputSubmitRequestedHandled: true, notifyReply: Some(false), ..ChatTurnOptions::default()
        };
        let submission = core.startUserMessage(PromptFunctionType::CHAT, participantId, Some(chatId.clone()),
            message, None, None, None, attachments, reply, options, Some(admitted.lease.clone())).await?;
        if admitted.cancelled.load(Ordering::Acquire) {
            core.cancelNativeChatExecution(chatId, &admitted.handle.executionId).await?;
        }
        runtime.observeStats();
        submission
    };
    submission.wait().await
}

/// Reports cancellation before execution with only the existing continuation identity, never a manufactured user row.
fn cancelledBeforeNativeTurn(admitted: &AdmittedChatTurn) -> operit_plugin_sdk::js_sdk::results::MessageSendResultData {
    use operit_plugin_sdk::js_sdk::results::{JsOptional, MessageSendOutcome, MessageSendResultData, MessageSendStatus};
    let (message, outcome) = match &admitted.work {
        AdmittedChatTurnWork::Submit { input, .. } => (input.text.clone(), MessageSendOutcome::NotPersisted { status: MessageSendStatus::Cancelled }),
        AdmittedChatTurnWork::Continue { userMessageTimestamp, .. } => (String::new(), MessageSendOutcome::Committed {
            status: MessageSendStatus::Cancelled, userMessageTimestamp: Some(*userMessageTimestamp), assistant: None,
        }),
    };
    MessageSendResultData { chatId: admitted.handle.chatId.clone(), message, aiResponse: JsOptional::Null,
        receivedAt: JsOptional::Null, sentAt: operit_host_api::TimeUtils::currentTimeMillis(), outcome }
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

/// Converts the validated SDK target explicitly without adding or interpreting an owner namespace.
fn canonicalExtensionTarget(
    target: &operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget,
) -> Result<operit_model::PluginExtensionTarget::PluginExtensionTarget, String> {
    use operit_model::PluginExtensionTarget::PluginExtensionTarget;
    use operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget;
    target.validate()?;
    Ok(match target {
        ChatExtensionTarget::Chat { chatId } => PluginExtensionTarget::Chat {
            chatId: chatId.clone(),
        },
        ChatExtensionTarget::Message {
            chatId,
            messageTimestamp,
            variantIndex,
        } => PluginExtensionTarget::Message {
            chatId: chatId.clone(),
            messageTimestamp: *messageTimestamp,
            variantIndex: *variantIndex,
        },
    })
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
