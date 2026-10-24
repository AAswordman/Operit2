use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{ProviderProfile, ResolvedModelConfig};
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{
    ToolPkgAiProviderRegistration, decodeToolPkgHookResult,
};
use operit_providers::runtime_support::{
    ChatConfigurationDisplayResult, ChatConfigurationPurpose, ChatConfigurationRequest,
    ChatConfigurationResult, ChatDisplayIdentity, ChatDisplayParticipant, ChatParticipantProfile, ChatToolAccess, ProviderFunctionModelBinding, ProviderMessageTiming,
    ProviderPackageInfo, ProviderRuntimeContext, ProviderRuntimeSupport,
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

/// Holds the explicit model owner or one registered plugin owner on a ready immutable catalog.
#[derive(Clone)]
pub struct ChatConfigurationApi {
    toolHandler: AIToolHandler,
    manager: RuntimePackageManager,
    ownerPackage: String,
}

impl ChatConfigurationApi {
    /// Selects ordinary model mode or the unique configuration plugin from one ready catalog.
    pub async fn ready(toolHandler: &AIToolHandler) -> Result<Self, String> {
        let manager =
            RuntimePackageManager::readySnapshot(toolHandler.getOrCreatePackageManager()).await?;
        let mut candidates = Vec::new();
        for runtime in manager.getEnabledToolPkgContainerRuntimes() {
            for declaration in &runtime.publicApis {
                if declaration.id == "chat.configuration.resolve" {
                    candidates.push(runtime.packageName.clone());
                }
            }
        }
        Ok(Self {
            toolHandler: toolHandler.clone(),
            ownerPackage: selectChatConfigurationOwner(candidates)?,
            manager,
        })
    }

    /// Returns the authenticated catalog owner for Core record operations and execution validation.
    pub fn extensionOwner(&self) -> &str {
        &self.ownerPackage
    }

    /// Resolves generic presentation through the selected model or registered configuration mode.
    pub async fn resolveDisplay(
        &self,
        request: ChatConfigurationRequest,
    ) -> Result<ChatConfigurationDisplayResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Display)?;
        if self.ownerPackage == MODEL_CONFIGURATION_OWNER {
            return self.modelDisplay(&request);
        }
        decodeDisplayConfigurationResult(
            self.invokeRegisteredResolve(request).await?,
            &self.ownerPackage,
        )
    }

    /// Resolves an existing execution through its explicit model or plugin configuration mode.
    pub async fn resolve(
        &self,
        request: ChatConfigurationRequest,
    ) -> Result<ChatConfigurationResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Execution)?;
        if self.ownerPackage == MODEL_CONFIGURATION_OWNER {
            return self.modelExecution(request);
        }
        self.invokeResolve(request).await
    }

    /// Resolves every currently bound participant for a conversation-wide context estimate.
    pub async fn resolveContextParticipants(
        &self,
        mut request: ChatConfigurationRequest,
    ) -> Result<Vec<ChatConfigurationResult>, String> {
        request.requirePurpose(ChatConfigurationPurpose::Execution)?;
        if request.participantId.is_some() {
            return Ok(vec![self.resolve(request).await?]);
        }
        request.purpose = ChatConfigurationPurpose::Display;
        let display = self.resolveDisplay(request.clone()).await?;
        if display.participants.is_empty() {
            return Err("Conversation context requires a current bound participant".to_string());
        }
        request.purpose = ChatConfigurationPurpose::Execution;
        let mut configurations = Vec::with_capacity(display.participants.len());
        for participant in display.participants {
            request.participantId = Some(participant.id.clone());
            let configuration = self.resolve(request.clone()).await?;
            if display.contextKey.as_ref() != Some(&configuration.contextKey)
                || configuration.profile.id != participant.id
            {
                return Err("Conversation binding changed during context estimation".to_string());
            }
            configurations.push(configuration);
        }
        Ok(configurations)
    }

    /// Resolves display data for a creation draft without requiring a nonexistent conversation or execution participant.
    pub async fn resolveDraft(
        &self,
        mut request: ChatConfigurationRequest,
        chatExtension: Option<operit_providers::runtime_support::JsonObject>,
    ) -> Result<ChatConfigurationDisplayResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Display)?;
        if let Some(chatId) = request.chatId.as_deref() {
            requireChatId(chatId)?;
        }
        request.chatExtension = chatExtension;
        if self.ownerPackage == MODEL_CONFIGURATION_OWNER {
            return self.modelDisplay(&request);
        }
        let value = self
            .manager
            .invokeToolPkgPublicApi(
                &self.ownerPackage,
                "chat.configuration.draft.resolve",
                serde_json::to_value(request).map_err(|error| error.to_string())?,
            )
            .await?;
        decodeDisplayConfigurationResult(value, &self.ownerPackage)
    }

    /// Publishes a prepared plugin binding only after the Core chat transaction succeeds.
    pub async fn commitDraft(&self, chatId: &str) -> Result<(), String> {
        requireChatId(chatId)?;
        if self.ownerPackage == MODEL_CONFIGURATION_OWNER {
            return Ok(());
        }
        let value = self
            .manager
            .invokeToolPkgPublicApi(
                &self.ownerPackage,
                "chat.configuration.draft.commit",
                serde_json::json!({"chatId":chatId}),
            )
            .await?;
        if value != serde_json::json!({"committed":true}) {
            return Err("Plugin did not acknowledge draft commit".into());
        }
        Ok(())
    }

    /// Discards an explicit prepared draft when an existing empty chat is reused.
    pub async fn abortDraft(&self, chatId: &str) -> Result<(), String> {
        requireChatId(chatId)?;
        if self.ownerPackage == MODEL_CONFIGURATION_OWNER {
            return Ok(());
        }
        let value = self
            .manager
            .invokeToolPkgPublicApi(
                &self.ownerPackage,
                "chat.configuration.draft.abort",
                serde_json::json!({"chatId":chatId}),
            )
            .await?;
        if value != serde_json::json!({"aborted":true}) {
            return Err("Plugin did not acknowledge draft abort".into());
        }
        Ok(())
    }

    /// Asks the selected configuration owner to release its state after the conversation is deleted.
    pub async fn deleteConfiguration(&self, chatId: &str) -> Result<(), String> {
        requireChatId(chatId)?;
        if self.ownerPackage == MODEL_CONFIGURATION_OWNER {
            return Ok(());
        }
        let value = self.manager
            .invokeToolPkgPublicApi(
                &self.ownerPackage,
                "chat.configuration.delete",
                serde_json::json!({"chatId":chatId}),
            )
            .await?;
        if value != serde_json::json!({"deleted":true}) {
            return Err("Plugin did not acknowledge configuration deletion".into());
        }
        Ok(())
    }

    /// Reads the exact configured model through the runtime's platform-neutral host-backed manager.
    fn modelProfile(&self, request: &ChatConfigurationRequest) -> Result<ChatParticipantProfile, String> {
        let support = RuntimeProviderSupport { tool_handler: self.toolHandler.clone() };
        let binding = &request.defaultModelBinding;
        let model = support.resolvedModelConfig(support.dataDir()?, &binding.providerId, &binding.modelId)?;
        if request.defaultTtsConfigId.trim().is_empty() {
            return Err("Model conversation speech configuration is empty".into());
        }
        Ok(ChatParticipantProfile {
            id: "model:chat".into(), name: model.modelId,
            avatarUri: Some(format!("provider-logo:{}", model.apiProviderTypeId)),
            introPrompt: String::new(), userPreferencesText: String::new(), openingStatement: String::new(),
            modelBinding: binding.clone(), ttsConfigId: request.defaultTtsConfigId.clone(),
            toolAccess: ChatToolAccess { enabled: false, allowedBuiltinTools: vec![], allowedPackages: vec![], allowedSkills: vec![], allowedMcpServers: vec![] },
            resources: vec![],
        })
    }

    /// Presents the ordinary model conversation when the ready catalog has no configuration plugin.
    fn modelDisplay(&self, request: &ChatConfigurationRequest) -> Result<ChatConfigurationDisplayResult, String> {
        let profile = self.modelProfile(request)?;
        let result = ChatConfigurationDisplayResult {
            contextKey: Some("model:chat".into()),
            identity: Some(ChatDisplayIdentity { title: profile.name.clone(), avatarUri: profile.avatarUri.clone() }),
            participants: vec![ChatDisplayParticipant { id: profile.id, name: profile.name, avatarUri: profile.avatarUri }],
            initialMessages: vec![], extensionOwner: MODEL_CONFIGURATION_OWNER.into(),
        };
        result.validate()?;
        Ok(result)
    }

    /// Resolves model execution from the current application model and voice defaults.
    fn modelExecution(&self, request: ChatConfigurationRequest) -> Result<ChatConfigurationResult, String> {
        let profile = self.modelProfile(&request)?;
        if request.participantId.as_ref().is_some_and(|id| id != &profile.id) {
            return Err("Model execution participant does not match the current model configuration".into());
        }
        let executionData = serde_json::json!({ "schemaVersion": 1, "contextKey": "model:chat", "profile": profile, "participants": [profile] })
            .as_object().cloned().ok_or("Model execution data is not an object")?;
        let result = ChatConfigurationResult { contextKey: "model:chat".into(), profile: profile.clone(), participants: vec![profile],
            executionData, extensionOwner: MODEL_CONFIGURATION_OWNER.into() };
        result.validate()?;
        Ok(result)
    }

    /// Invokes the exact selected declaration and injects its authenticated owner after successful decoding.
    async fn invokeResolve(
        &self,
        request: ChatConfigurationRequest,
    ) -> Result<ChatConfigurationResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Execution)?;
        decodeConfigurationResult(
            self.invokeRegisteredResolve(request).await?,
            &self.ownerPackage,
        )
    }

    /// Invokes one immutable selected registration without choosing a second owner or reinterpreting its result.
    async fn invokeRegisteredResolve(
        &self,
        request: ChatConfigurationRequest,
    ) -> Result<Value, String> {
        self.manager
            .invokeToolPkgPublicApi(
                &self.ownerPackage,
                "chat.configuration.resolve",
                serde_json::to_value(request).map_err(|error| error.to_string())?,
            )
            .await
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
    let result = ChatConfigurationResult { contextKey: saved.contextKey, profile: saved.profile, participants: saved.participants,
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
                            let api = ChatConfigurationApi::ready(&tool_handler).await?;
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
            "contextKey":"opaque/context", "profile":profile, "participants":[profile],
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
