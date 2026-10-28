use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{ProviderProfile, ResolvedModelConfig};
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{ToolPkgAiProviderRegistration, decodeToolPkgHookResult};
use operit_providers::runtime_support::{
    ChatConfigurationDisplayResult, ChatConfigurationPurpose, ChatConfigurationRequest, ChatConfigurationResult,
    ChatDisplayIdentity, ChatDisplayParticipant, ChatParticipantProfile, ChatToolAccess, ProviderFunctionModelBinding,
    ProviderMessageTiming, ProviderPackageInfo, ProviderRuntimeContext, ProviderRuntimeSupport,
    ProviderRuntimeSupportFuture, ProviderToolPkgAiProviderRegistration,
};
use operit_tools::tools::AIToolHandler::AIToolHandler;
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;

use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::data::preferences::TtsConfigManager::TtsConfigManager;
use crate::plugins::toolpkg::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry;
use operit_store::RuntimeStorePaths::RuntimeStorePaths;

/// Creates runtime-backed services required by provider crates.
pub struct ProviderRuntimeSupportService;

impl ProviderRuntimeSupportService {
    /// Creates provider runtime support bound to one tool handler instance.
    pub fn create(tool_handler: AIToolHandler) -> ProviderRuntimeContext {
        ProviderRuntimeContext::new(Arc::new(RuntimeProviderSupport { tool_handler }))
    }
}

/// Identifies the built-in model conversation owner independently of optional plugins.
pub const MODEL_CONFIGURATION_OWNER: &str = "host.chat.model";

/// Optional plugin backend. Removing it switches every subsequent operation to native defaults.
#[derive(Clone)]
struct ChatConfigurationPlugin {
    owner: String,
    manager: RuntimePackageManager,
}

/// Resolves optional plugin configuration; native defaults remain independent of plugin availability.
#[derive(Clone)]
pub struct ChatConfigurationApi {
    toolHandler: AIToolHandler,
    plugin: Option<ChatConfigurationPlugin>,
}

impl ChatConfigurationApi {
    /// Loads the optional backend without making plugin readiness a prerequisite for chatting.
    pub async fn ready(toolHandler: &AIToolHandler) -> Self {
        let plugin = async {
            let manager = RuntimePackageManager::readySnapshot(toolHandler.getOrCreatePackageManager()).await?;
            let candidates = manager
                .getEnabledToolPkgContainerRuntimes()
                .into_iter()
                .flat_map(|runtime| {
                    runtime
                        .publicApis
                        .into_iter()
                        .map(move |api| (runtime.packageName.clone(), api.id))
                })
                .filter(|(_, id)| id == "chat.configuration.resolve")
                .map(|(owner, _)| owner)
                .collect();
            let owner = selectChatConfigurationOwner(candidates)?;
            Ok::<_, String>((owner != MODEL_CONFIGURATION_OWNER).then_some(ChatConfigurationPlugin { owner, manager }))
        }
        .await;
        let plugin = match plugin {
            Ok(plugin) => plugin,
            Err(error) => {
                reportConfigurationFallback(&toolHandler.getContext(), "插件目录", &error);
                None
            }
        };
        Self {
            toolHandler: toolHandler.clone(),
            plugin,
        }
    }

    pub fn extensionOwner(&self) -> &str {
        self.plugin
            .as_ref()
            .map_or(MODEL_CONFIGURATION_OWNER, |plugin| &plugin.owner)
    }

    /// The only plugin resolution boundary: deadline, strict decoding, owner-attributed notification, then removal.
    async fn pluginConfiguration<T>(
        &mut self,
        method: &str,
        request: &ChatConfigurationRequest,
        decode: fn(Value, &str) -> Result<T, String>,
    ) -> Option<T> {
        let plugin = self.plugin.as_ref()?;
        let result = async {
            let raw = plugin
                .manager
                .invokeToolPkgPublicApiWithTimeoutMillis(
                    &plugin.owner,
                    method,
                    serde_json::to_value(request).map_err(|error| error.to_string())?,
                    CONFIGURATION_TIMEOUT_MILLIS,
                )
                .await?;
            decode(raw, &plugin.owner)
        }
        .await;
        match result {
            Ok(configuration) => Some(configuration),
            Err(error) => {
                self.disablePlugin(&error);
                None
            }
        }
    }

    fn disablePlugin(&mut self, error: &str) {
        if let Some(plugin) = self.plugin.take() {
            reportConfigurationFallback(&self.toolHandler.getContext(), &plugin.owner, error);
        }
    }

    /// Resolves presentation without allowing plugin failure to prevent opening a conversation.
    pub async fn resolveDisplay(
        &mut self,
        request: ChatConfigurationRequest,
    ) -> Result<ChatConfigurationDisplayResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Display)?;
        if let Some(configuration) = self
            .pluginConfiguration("chat.configuration.resolve", &request, decodeDisplayConfigurationResult)
            .await
        {
            return Ok(configuration);
        }
        self.modelDisplay(&request)
    }

    /// Resolves a send with the plugin, or uses the exact native path without plugin-owned selection data.
    pub async fn resolve(&mut self, request: ChatConfigurationRequest) -> Result<ChatConfigurationResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Execution)?;
        if request.participantId.as_deref() != Some("model:chat") {
            if let Some(configuration) = self
                .pluginConfiguration("chat.configuration.resolve", &request, decodeConfigurationResult)
                .await
            {
                return Ok(configuration);
            }
        }
        nativeModelExecution(self.modelProfile(&request)?)
    }

    /// A failed participant invalidates the whole plugin estimate; never mix native and plugin contexts.
    pub async fn resolveContextParticipants(
        &mut self,
        mut request: ChatConfigurationRequest,
    ) -> Result<Vec<ChatConfigurationResult>, String> {
        request.requirePurpose(ChatConfigurationPurpose::Execution)?;
        if request.participantId.is_some() || self.plugin.is_none() {
            return Ok(vec![self.resolve(request).await?]);
        }
        let original = request.clone();
        request.purpose = ChatConfigurationPurpose::Display;
        let display = self.resolveDisplay(request.clone()).await?;
        request.purpose = ChatConfigurationPurpose::Execution;
        let mut configurations = Vec::new();
        for participant in display.participants {
            request.participantId = Some(participant.id.clone());
            let configuration = self.resolve(request.clone()).await?;
            if self.plugin.is_none() {
                return Ok(vec![configuration]);
            }
            if display.contextKey.as_ref() != Some(&configuration.contextKey)
                || configuration.profile.id != participant.id
            {
                self.disablePlugin("Conversation binding changed during context estimation");
                return Ok(vec![self.resolve(original).await?]);
            }
            configurations.push(configuration);
        }
        if configurations.is_empty() {
            self.disablePlugin("Conversation has no participants");
            return Ok(vec![self.resolve(original).await?]);
        }
        Ok(configurations)
    }

    /// Resolves creation drafts through the same isolated boundary as existing conversations.
    pub async fn resolveDraft(
        &mut self,
        mut request: ChatConfigurationRequest,
        chatExtension: Option<operit_providers::runtime_support::JsonObject>,
    ) -> Result<ChatConfigurationDisplayResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Display)?;
        if let Some(chatId) = &request.chatId {
            requireChatId(chatId)?;
        }
        request.chatExtension = chatExtension;
        if let Some(configuration) = self
            .pluginConfiguration(
                "chat.configuration.draft.resolve",
                &request,
                decodeDisplayConfigurationResult,
            )
            .await
        {
            return Ok(configuration);
        }
        self.modelDisplay(&request)
    }

    pub async fn commitDraft(&self, chatId: &str) -> Result<(), String> {
        self.updateBinding(chatId, "chat.configuration.draft.commit", "committed")
            .await
    }

    pub async fn abortDraft(&self, chatId: &str) -> Result<(), String> {
        self.updateBinding(chatId, "chat.configuration.draft.abort", "aborted")
            .await
    }

    pub async fn deleteConfiguration(&self, chatId: &str) -> Result<(), String> {
        self.updateBinding(chatId, "chat.configuration.delete", "deleted").await
    }

    /// Mutations retain their transaction semantics; only resolution is allowed to degrade.
    async fn updateBinding(&self, chatId: &str, method: &str, acknowledgement: &str) -> Result<(), String> {
        requireChatId(chatId)?;
        let Some(plugin) = &self.plugin else {
            return Ok(());
        };
        let value = plugin
            .manager
            .invokeToolPkgPublicApi(&plugin.owner, method, serde_json::json!({"chatId":chatId}))
            .await?;
        if value != serde_json::json!({(acknowledgement):true}) {
            return Err(format!("Plugin {} did not acknowledge {method}", plugin.owner));
        }
        Ok(())
    }

    /// Reads the exact configured model through the runtime's platform-neutral host-backed manager.
    fn modelProfile(&self, request: &ChatConfigurationRequest) -> Result<ChatParticipantProfile, String> {
        let support = RuntimeProviderSupport {
            tool_handler: self.toolHandler.clone(),
        };
        let binding = &request.defaultModelBinding;
        let model = support.resolvedModelConfig(support.dataDir()?, &binding.providerId, &binding.modelId)?;
        if request.defaultTtsConfigId.trim().is_empty() {
            return Err("Model conversation speech configuration is empty".into());
        }
        Ok(ChatParticipantProfile {
            id: "model:chat".into(),
            name: model.modelId,
            avatarUri: Some(format!("provider-logo:{}", model.apiProviderTypeId)),
            introPrompt: String::new(),
            userPreferencesText: String::new(),
            openingStatement: String::new(),
            modelBinding: binding.clone(),
            ttsConfigId: request.defaultTtsConfigId.clone(),
            toolAccess: ChatToolAccess {
                enabled: false,
                allowedBuiltinTools: vec![],
                allowedPackages: vec![],
                allowedSkills: vec![],
                allowedMcpServers: vec![],
            },
            resources: vec![],
        })
    }

    /// Presents the ordinary model conversation when the ready catalog has no configuration plugin.
    fn modelDisplay(&self, request: &ChatConfigurationRequest) -> Result<ChatConfigurationDisplayResult, String> {
        let profile = self.modelProfile(request)?;
        let result = ChatConfigurationDisplayResult {
            contextKey: Some("model:chat".into()),
            identity: Some(ChatDisplayIdentity {
                title: profile.name.clone(),
                avatarUri: profile.avatarUri.clone(),
            }),
            participants: vec![ChatDisplayParticipant {
                id: profile.id,
                name: profile.name,
                avatarUri: profile.avatarUri,
            }],
            initialMessages: vec![],
            extensionOwner: MODEL_CONFIGURATION_OWNER.into(),
        };
        result.validate()?;
        Ok(result)
    }
}

/// Builds the standalone native descriptor without any plugin identity, policy, prompt or resource data.
fn nativeModelExecution(profile: ChatParticipantProfile) -> Result<ChatConfigurationResult, String> {
    let executionData = serde_json::json!({ "schemaVersion": 1, "contextKey": "model:chat", "profile": profile, "participants": [profile] })
            .as_object().cloned().ok_or("Model execution data is not an object")?;
    let result = ChatConfigurationResult {
        historyPolicy: operit_providers::runtime_support::ChatHistoryPolicy {
            isolateParticipants: false,
            labelUserMessages: false,
        },
        contextKey: "model:chat".into(),
        profile: profile.clone(),
        participants: vec![profile],
        executionData,
        extensionOwner: MODEL_CONFIGURATION_OWNER.into(),
    };
    result.validate()?;
    Ok(result)
}

/// Bounds configuration scripts using the engine deadline, which also interrupts synchronous JavaScript.
const CONFIGURATION_TIMEOUT_MILLIS: u64 = 10_000;

/// Reports plugin failure independently of the send result; a broken notification host cannot abort fallback.
fn reportConfigurationFallback(context: &operit_host_api::HostManager::HostManager, owner: &str, error: &str) {
    let message = format!("插件「{owner}」的聊天配置解析失败，已切换为默认聊天配置。错误：{error}");
    operit_util::AppLogger::AppLogger::e("ChatConfigurationFallback", &message);
    if let Some(host) = &context.toastHost {
        if let Err(error) = host.toast(&message) {
            operit_util::AppLogger::AppLogger::e(
                "ChatConfigurationFallback",
                &format!("插件错误提示显示失败：{error}"),
            );
        }
    }
}

/// Rejects plugin-supplied runtime identity and validates display JSON without any execution-profile interpretation.
fn decodeDisplayConfigurationResult(
    raw: Value,
    owner: &str,
) -> Result<ChatConfigurationDisplayResult, String> {
    if owner.trim().is_empty() {
        return Err("Display configuration owner is not authenticated".to_string());
    }
    if raw
        .as_object()
        .is_some_and(|object| object.contains_key("extensionOwner"))
    {
        return Err(
            "Display configuration extensionOwner is assigned only by the runtime".to_string(),
        );
    }
    let mut result: ChatConfigurationDisplayResult =
        serde_json::from_value(raw).map_err(|error| error.to_string())?;
    result.extensionOwner = owner.to_string();
    result.validate()?;
    Ok(result)
}

/// Rejects plugin-supplied runtime identity and validates the exact selected owner's current execution data.
fn decodeConfigurationResult(raw: Value, owner: &str) -> Result<ChatConfigurationResult, String> {
    if owner.trim().is_empty() {
        return Err("Chat configuration owner is not authenticated".to_string());
    }
    if raw
        .as_object()
        .is_some_and(|object| object.contains_key("extensionOwner"))
    {
        return Err(
            "Chat configuration extensionOwner is assigned only by the runtime".to_string(),
        );
    }
    let mut result: ChatConfigurationResult =
        serde_json::from_value(raw).map_err(|error| error.to_string())?;
    result.extensionOwner = owner.to_string();
    result.validate()?;
    Ok(result)
}

/// Resolves a unique registered configuration owner without naming any plugin domain.
fn selectChatConfigurationOwner(mut candidates: Vec<String>) -> Result<String, String> {
    if candidates.iter().any(|owner| owner == MODEL_CONFIGURATION_OWNER) {
        return Err("Plugin registration cannot claim the native model configuration owner".into());
    }
    match candidates.len() {
        0 => Ok(MODEL_CONFIGURATION_OWNER.to_string()),
        1 => Ok(candidates.remove(0)),
        _ => {
            candidates.sort();
            Err(format!(
                "Duplicate chat.configuration.resolve registrations: {}",
                candidates.join(", ")
            ))
        }
    }
}

/// Defines the current native execution data for plugin-independent model conversations.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(non_snake_case)]
struct ModelExecutionData {
    schemaVersion: u32,
    contextKey: String,
    profile: ChatParticipantProfile,
    participants: Vec<ChatParticipantProfile>,
}

/// Validates native execution data without reading the current provider catalog or optional plugin storage.
fn validateModelExecutionData(snapshot: &operit_providers::runtime_support::JsonObject) -> Result<ModelExecutionData, String> {
    let saved: ModelExecutionData = serde_json::from_value(Value::Object(snapshot.clone())).map_err(|error| error.to_string())?;
    if saved.schemaVersion != 1 || saved.contextKey != "model:chat" || saved.profile.id != "model:chat"
        || saved.profile.name != saved.profile.modelBinding.modelId || saved.participants != vec![saved.profile.clone()]
        || !saved.profile.resources.is_empty() || !saved.profile.introPrompt.is_empty()
        || !saved.profile.userPreferencesText.is_empty() || !saved.profile.openingStatement.is_empty() {
        return Err("Invalid native model execution data".into());
    }
    Ok(saved)
}

/// Authenticates the complete native model execution data supplied to generic tool policy hooks.
pub(crate) fn validateNativeModelExecutionData(snapshot: &operit_providers::runtime_support::JsonObject, participantId: Option<&str>) -> Result<(), String> {
    let saved = validateModelExecutionData(snapshot)?;
    if participantId != Some(saved.profile.id.as_str()) {
        return Err("Native model tool participant differs from its saved profile".into());
    }
    let result = ChatConfigurationResult { historyPolicy: operit_providers::runtime_support::ChatHistoryPolicy { isolateParticipants: false, labelUserMessages: false }, contextKey: saved.contextKey, profile: saved.profile, participants: saved.participants,
        executionData: snapshot.clone(), extensionOwner: MODEL_CONFIGURATION_OWNER.into() };
    result.validate()
}

/// Rejects empty conversation identities before canonical record operations.
fn requireChatId(chatId: &str) -> Result<(), String> {
    if chatId.trim().is_empty() {
        return Err("Chat configuration chatId is empty".to_string());
    }
    Ok(())
}

/// Bridges provider-owned interfaces to runtime-owned managers and registries.
struct RuntimeProviderSupport {
    tool_handler: AIToolHandler,
}

impl ProviderRuntimeSupport for RuntimeProviderSupport {
    fn generateLocalLlm(
        &self,
        bindingKey: String,
        modelBinding: String,
        request: Value,
        settings: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send + '_>>
    {
        let context = self.tool_handler.getContext();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let scheduled = operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "local-llm-inference",
                Box::new(move || {
                    Box::pin(async move {
                        let result = async {
                            let storage = context
                                .runtimeStorageHost
                                .clone()
                                .ok_or("Local inference requires runtime storage")?;
                            let key = if bindingKey.is_empty() {
                                super::LocalLlmService::ensureLocalLlmBinding(
                                    storage.clone(),
                                    &modelBinding,
                                )?
                            } else {
                                bindingKey
                            };
                            let response = super::LocalLlmService::generateRoutedLocalLlm(
                                storage,
                                key,
                                modelBinding,
                                request.to_string(),
                                settings.to_string(),
                            )
                            .await?;
                            serde_json::to_value(response).map_err(|error| error.to_string())
                        }
                        .await;
                        let _ = sender.send(result);
                    })
                }),
            );
        Box::pin(async move {
            scheduled.map_err(|error| error.to_string())?;
            receiver
                .await
                .map_err(|error| format!("Local inference task ended: {error}"))?
        })
    }

    /// Returns the root directory used by runtime data.
    fn dataDir(&self) -> Result<PathBuf, String> {
        let context = self.tool_handler.getContext();
        let storage = context
            .runtimeStorageHost
            .as_ref()
            .ok_or("Provider runtime has no storage host")?;
        storage
            .runtimeRootDir()
            .ok_or_else(|| "Provider runtime storage host has no data root".to_string())
    }

    /// Returns the current thinking quality level.
    fn thinkingQualityLevel(&self) -> Result<i32, String> {
        ApiPreferences::getInstance()
            .thinkingQualityLevelFlow()
            .first()
            .map_err(|error| error.to_string())
    }

    /// Records provider/model token usage.
    fn updateTokensForProviderModel(
        &self,
        providerModel: &str,
        inputTokens: i64,
        outputTokens: i64,
        cachedInputTokens: i64,
    ) -> Result<(), String> {
        ApiPreferences::getInstance()
            .updateTokensForProviderModel(
                providerModel,
                inputTokens,
                outputTokens,
                cachedInputTokens,
            )
            .map_err(|error| error.to_string())
    }

    /// Returns the application preference controlling profile-document prompt visibility.
    fn disableUserPreferenceDescription(&self) -> Result<bool, String> {
        Ok(
            crate::data::preferences::ApiPreferences::ApiPreferences::getInstance()
                .disableUserPreferenceDescriptionFlow()
                .first()
                .map_err(|e| e.to_string())?,
        )
    }

    /// Returns the deliberate application-default speech configuration.
    fn defaultTtsConfigId(&self) -> Result<String, String> {
        TtsConfigManager::new(RuntimeStorePaths::default()).getCurrentTtsConfigId()
    }

    /// Resolves a persisted chat context using authenticated calls on one ready registration snapshot.
    fn resolveChatConfiguration(
        &self,
        request: ChatConfigurationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ChatConfigurationResult, String>> + Send + '_>,
    > {
        let tool_handler = self.tool_handler.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let scheduled = operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "resolve-chat-configuration",
                Box::new(move || {
                    Box::pin(async move {
                        let result = async {
                            let mut api = ChatConfigurationApi::ready(&tool_handler).await;
                            api.resolve(request).await
                        }
                        .await;
                        let _ = sender.send(result);
                    })
                }),
            );
        Box::pin(async move {
            scheduled.map_err(|error| format!("Chat configuration scheduling failed: {error}"))?;
            receiver.await.map_err(|error| {
                format!("Chat configuration execution ended without a result: {error}")
            })?
        })
    }

    /// Returns skill package descriptions visible to AI prompt composition.
    fn aiVisibleSkillPackages(&self) -> Result<Vec<ProviderPackageInfo>, String> {
        let hostManager = self.tool_handler.getContext();
        let packages =
            SkillRepository::getInstance(&hostManager, self.tool_handler.runtimeSupport())
                .getAiVisibleSkillPackages()
                .into_iter()
                .map(|(name, skill)| ProviderPackageInfo {
                    name,
                    description: skill.description,
                })
                .collect();
        Ok(packages)
    }

    /// Returns the model binding for a function.
    fn modelBindingForFunction(
        &self,
        rootDir: PathBuf,
        functionType: FunctionType,
    ) -> Result<ProviderFunctionModelBinding, String> {
        let binding = FunctionalConfigManager::new(rootDir)
            .getModelBindingForFunction(functionType)
            .map_err(|error| error.to_string())?;
        Ok(ProviderFunctionModelBinding {
            providerId: binding.providerId,
            modelId: binding.modelId,
        })
    }

    /// Returns the resolved model config for a provider/model pair.
    fn resolvedModelConfig(
        &self,
        rootDir: PathBuf,
        providerId: &str,
        modelId: &str,
    ) -> Result<ResolvedModelConfig, String> {
        ModelConfigManager::new(rootDir)
            .getResolvedModelConfig(providerId, modelId)
            .map_err(|error| error.to_string())
    }

    /// Returns the saved ChatGPT Codex session.
    fn loadCodexTokens(
        &self,
    ) -> Result<operit_providers::chat::llmprovider::CodexOAuth::CodexOAuthTokens, String> {
        crate::data::preferences::CodexAuthPreferences::CodexAuthPreferences::getInstance()
            .load()?
            .ok_or_else(|| "Codex authorization is required".to_string())
    }

    /// Persists a refreshed ChatGPT Codex session.
    fn saveCodexTokens(
        &self,
        tokens: operit_providers::chat::llmprovider::CodexOAuth::CodexOAuthTokens,
    ) -> Result<(), String> {
        crate::data::preferences::CodexAuthPreferences::CodexAuthPreferences::getInstance()
            .save(&tokens)
    }

    /// Returns a provider profile by id.
    fn providerProfile(
        &self,
        rootDir: PathBuf,
        providerId: &str,
    ) -> Result<ProviderProfile, String> {
        ModelConfigManager::new(rootDir)
            .getProviderProfile(providerId)
            .map_err(|error| error.to_string())
    }

    /// Returns whether a ToolPkg AI provider is registered.
    fn hasToolPkgAiProvider(&self, providerId: &str) -> bool {
        ToolPkgAiProviderRegistry::get(providerId).is_some()
    }

    /// Returns a ToolPkg AI provider registration.
    fn toolPkgAiProvider(&self, providerId: &str) -> Option<ProviderToolPkgAiProviderRegistration> {
        ToolPkgAiProviderRegistry::get(providerId).map(providerRegistrationToProvider)
    }

    /// Invokes a ToolPkg AI provider hook.
    fn runToolPkgAiProviderHook(
        &self,
        containerPackageName: &str,
        functionName: &str,
        functionSource: Option<&str>,
        event: &str,
        tag: Option<String>,
        sourceKey: Option<String>,
        eventPayload: Value,
        runtimeContextKey: Option<String>,
        executionKind: Option<String>,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send>>
    {
        let package_manager = self.tool_handler.getOrCreatePackageManager();
        let manager = package_manager
            .lock()
            .expect("package manager mutex poisoned")
            .clone();
        manager.runToolPkgMainHook(
            containerPackageName,
            functionName,
            event,
            tag.as_deref(),
            sourceKey.as_deref(),
            functionSource,
            eventPayload,
            runtimeContextKey.as_deref(),
            executionKind.as_deref(),
            onIntermediateResult,
        )
    }

    /// Decodes a ToolPkg hook result.
    fn decodeToolPkgHookResult(&self, raw: Option<String>) -> Option<Value> {
        decodeToolPkgHookResult(raw)
    }

    /// Returns a provider timing snapshot.
    fn messageTimingNow(&self) -> ProviderMessageTiming {
        let timing = crate::core::chat::AIMessageManager::messageTimingNow();
        ProviderMessageTiming {
            startedAtMs: timing.startedAtMs,
        }
    }

    /// Writes a provider timing log entry.
    fn logMessageTiming(
        &self,
        stage: &str,
        startTimeMs: ProviderMessageTiming,
        details: Option<String>,
    ) {
        crate::core::chat::AIMessageManager::logMessageTiming(
            stage,
            crate::core::chat::AIMessageManager::MessageTiming {
                startedAtMs: startTimeMs.startedAtMs,
            },
            details,
        );
    }
}

/// Converts a runtime ToolPkg provider registration to the provider crate shape.
fn providerRegistrationToProvider(
    registration: ToolPkgAiProviderRegistration,
) -> ProviderToolPkgAiProviderRegistration {
    ProviderToolPkgAiProviderRegistration {
        containerPackageName: registration.containerPackageName,
        providerId: registration.providerId,
        displayName: registration.displayName,
        description: registration.description,
        listModelsFunctionName: registration.listModelsFunctionName,
        listModelsFunctionSource: registration.listModelsFunctionSource,
        sendMessageFunctionName: registration.sendMessageFunctionName,
        sendMessageFunctionSource: registration.sendMessageFunctionSource,
        testConnectionFunctionName: registration.testConnectionFunctionName,
        testConnectionFunctionSource: registration.testConnectionFunctionSource,
        calculateInputTokensFunctionName: registration.calculateInputTokensFunctionName,
        calculateInputTokensFunctionSource: registration.calculateInputTokensFunctionSource,
    }
}

#[cfg(test)]
mod chat_configuration_contract_tests {
    use super::{
        decodeConfigurationResult, decodeDisplayConfigurationResult, requireChatId,
        selectChatConfigurationOwner, validateNativeModelExecutionData, MODEL_CONFIGURATION_OWNER,
    };

    use super::{nativeModelExecution, reportConfigurationFallback};
    use operit_providers::runtime_support::ChatParticipantProfile;
    use std::sync::{Arc, Mutex};

    fn defaultProfile() -> ChatParticipantProfile {
        serde_json::from_value(serde_json::json!({
            "id":"model:chat", "name":"default-model", "avatarUri":null,
            "introPrompt":"", "userPreferencesText":"", "openingStatement":"",
            "modelBinding":{"providerId":"default-provider", "modelId":"default-model"}, "ttsConfigId":"default-voice",
            "toolAccess":{"enabled":false, "allowedBuiltinTools":[], "allowedPackages":[], "allowedSkills":[], "allowedMcpServers":[]},
            "resources":[]
        })).unwrap()
    }

    /// Error notices identify both the responsible plugin and original failure.
    #[test]
    fn fallback_notification_identifies_plugin_and_original_error() {
        let notices = Arc::new(Mutex::new(Vec::new()));
        let sink = notices.clone();
        let host = operit_host_api::HostManager::HostManager::new().withToastHost(Arc::new(move |message: &str| {
            sink.lock().unwrap().push(message.to_string());
            Ok(())
        }));
        reportConfigurationFallback(&host, "org.example.broken", "missing provider");
        let notices = notices.lock().unwrap();
        assert_eq!(notices.len(), 1);
        for expected in ["org.example.broken", "missing provider", "已切换为默认聊天配置"] {
            assert!(notices[0].contains(expected));
        }
    }

    /// Failure to present a notice is logged, not rethrown into the send pipeline.
    #[test]
    fn notification_failure_does_not_block_native_execution() {
        let host = operit_host_api::HostManager::HostManager::new().withToastHost(Arc::new(|_: &str| {
            Err(operit_host_api::HostError::new("notification channel closed"))
        }));
        reportConfigurationFallback(&host, "broken", "missing provider");
        let native = nativeModelExecution(defaultProfile()).unwrap();
        validateNativeModelExecutionData(&native.executionData, Some("model:chat")).unwrap();
    }

    /// The default descriptor has the exact standalone identity, model, voice, history policy and empty plugin resources.
    #[test]
    fn native_fallback_has_no_plugin_configuration() {
        let native = nativeModelExecution(defaultProfile()).unwrap();
        assert_eq!(native.extensionOwner, MODEL_CONFIGURATION_OWNER);
        assert_eq!(native.profile, defaultProfile());
        assert_eq!(native.participants, vec![defaultProfile()]);
        assert_eq!(native.contextKey, "model:chat");
        assert!(!native.historyPolicy.isolateParticipants);
        assert!(!native.historyPolicy.labelUserMessages);
        validateNativeModelExecutionData(&native.executionData, Some("model:chat")).unwrap();
    }

    /// Selects arbitrary uniquely registered owners without a built-in identity whitelist.
    #[test]
    fn accepts_arbitrary_unique_owner() {
        assert_eq!(
            selectChatConfigurationOwner(vec!["org.example.dynamic.profile".into()]).unwrap(),
            "org.example.dynamic.profile"
        );
    }

    /// Allows standalone model operation and preserves duplicate registration errors.
    #[test]
    fn supports_no_plugin_and_rejects_duplicate_owners() {
        assert_eq!(
            selectChatConfigurationOwner(Vec::new()).unwrap(),
            MODEL_CONFIGURATION_OWNER
        );
        assert_eq!(
            selectChatConfigurationOwner(vec!["owner-a".into(), "owner-b".into()]).unwrap_err(),
            "Duplicate chat.configuration.resolve registrations: owner-a, owner-b"
        );
    }

    /// Verifies that an ordinary model execution context is valid without any plugin declaration or datastore.
    #[test]
    fn native_model_execution_data_does_not_require_a_plugin_owner() {
        let profile = serde_json::json!({
            "id":"model:chat", "name":"actual-model", "avatarUri":"provider-logo:OPENAI",
            "introPrompt":"", "userPreferencesText":"", "openingStatement":"",
            "modelBinding":{"providerId":"actual-provider", "modelId":"actual-model"}, "ttsConfigId":"actual-voice",
            "toolAccess":{"enabled":false, "allowedBuiltinTools":[], "allowedPackages":[], "allowedSkills":[], "allowedMcpServers":[]},
            "resources":[]
        });
        let snapshot = serde_json::json!({"schemaVersion":1, "contextKey":"model:chat", "profile":profile, "participants":[profile]}).as_object().unwrap().clone();
        validateNativeModelExecutionData(&snapshot, Some("model:chat")).unwrap();
        assert!(validateNativeModelExecutionData(&snapshot, Some("different-participant")).is_err());
        assert!(validateNativeModelExecutionData(&snapshot, None).is_err());
        let mut changed = snapshot;
        changed.get_mut("profile").unwrap()["introPrompt"] = serde_json::json!("unexpected role prompt");
        assert!(validateNativeModelExecutionData(&changed, Some("model:chat")).is_err());
    }

    /// Keeps registered packages from impersonating the native configuration mode.
    #[test]
    fn rejects_plugin_registration_using_the_native_model_owner() {
        assert!(selectChatConfigurationOwner(vec![MODEL_CONFIGURATION_OWNER.into()]).is_err());
    }

    /// Injects only the exact selected registration owner while preserving the plugin's complete opaque execution data.
    #[test]
    fn injects_owner_without_exposing_it_to_javascript_results() {
        let profile = serde_json::json!({
            "id":"participant", "name":"Participant", "avatarUri":null,
            "introPrompt":"intro", "userPreferencesText":"preferences", "openingStatement":"opening",
            "modelBinding":{"providerId":"provider", "modelId":"model"}, "ttsConfigId":"voice",
            "toolAccess":{"enabled":true, "allowedBuiltinTools":["tool"], "allowedPackages":[], "allowedSkills":[], "allowedMcpServers":[]},
            "resources":[{"key":"opaque/resource", "readable":true, "writable":false}]
        });
        let raw = serde_json::json!({
            "contextKey":"opaque/context", "historyPolicy":{"isolateParticipants":false,"labelUserMessages":false},"profile":profile, "participants":[profile],
            "executionData":{"actor":{"id":"participant"}, "voice":"voice", "revision":"7"}
        });
        for owner in ["org.example.owner-a", "org.example.owner-b"] {
            let configuration = decodeConfigurationResult(raw.clone(), owner).unwrap();
            assert_eq!(configuration.extensionOwner, owner);
            assert_eq!(
                configuration.executionData,
                raw["executionData"].as_object().unwrap().clone()
            );
            assert_eq!(serde_json::to_value(configuration).unwrap(), raw);
        }
    }

    /// Rejects forged runtime identities before validating any plugin-authored descriptor.
    #[test]
    fn rejects_forged_owner_and_unauthenticated_dispatch() {
        assert_eq!(
            decodeConfigurationResult(serde_json::json!({"extensionOwner":"other"}), "real-owner")
                .unwrap_err(),
            "Chat configuration extensionOwner is assigned only by the runtime"
        );
        assert!(decodeConfigurationResult(serde_json::json!({}), " ").is_err());
        assert!(requireChatId(" ").is_err());
    }
    /// Authenticates a presentation descriptor independently of every execution binding.
    #[test]
    fn display_decode_authenticates_owner_and_never_accepts_execution_profile() {
        let raw = serde_json::json!({"contextKey":"opaque","identity":{"title":"Display","avatarUri":null},"participants":[],"initialMessages":[]});
        let display = decodeDisplayConfigurationResult(raw.clone(), "arbitrary.owner").unwrap();
        assert_eq!(display.extensionOwner, "arbitrary.owner");
        assert_eq!(serde_json::to_value(display).unwrap(), raw);
        assert!(decodeConfigurationResult(raw.clone(), "arbitrary.owner").is_err());
        let mut forged = raw.clone();
        forged["extensionOwner"] = serde_json::json!("forged.owner");
        assert!(decodeDisplayConfigurationResult(forged, "arbitrary.owner").is_err());
        assert!(decodeDisplayConfigurationResult(raw, " ").is_err());
    }
}
