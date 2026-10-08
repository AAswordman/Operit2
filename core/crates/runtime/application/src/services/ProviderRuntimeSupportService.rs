use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{ProviderProfile, ResolvedModelConfig};
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{
    decodeToolPkgHookResult, ToolPkgAiProviderRegistration,
};
use operit_providers::runtime_support::{
    ChatConfigurationDisplayResult, ChatConfigurationPurpose, ChatConfigurationRequest,
    ChatConfigurationResult, ProviderFunctionModelBinding,
    ProviderMessageTiming, ProviderPackageInfo, ProviderRuntimeContext, ProviderRuntimeSupport,
    ProviderRuntimeSupportFuture, ProviderToolPkgAiProviderRegistration,
};
use operit_tools::tools::packTool::RuntimePackageManager::RuntimePackageManager;
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;

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

/// Holds one uniquely registered configuration owner and its immutable ready catalog.
#[derive(Clone)]
pub struct ChatConfigurationApi {
    manager: RuntimePackageManager,
    ownerPackage: String,
    recordSupport: Arc<dyn operit_tools::runtime_support::ToolRuntimeSupport>,
}

impl ChatConfigurationApi {
    /// Selects only the unique enabled resolve registration without requiring association APIs.
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
            ownerPackage: selectChatConfigurationOwner(candidates)?,
            manager,
            recordSupport: toolHandler.runtimeSupport(),
        })
    }

    /// Returns the authenticated catalog owner for Core record operations and snapshot validation.
    pub fn extensionOwner(&self) -> &str {
        &self.ownerPackage
    }

    /// Resolves presentation from a real conversation without requiring an executable profile or snapshot.
    pub async fn resolveDisplay(
        &self,
        mut request: ChatConfigurationRequest,
    ) -> Result<ChatConfigurationDisplayResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Display)?;
        self.readConversationExtension(&mut request)?;
        decodeDisplayConfigurationResult(self.invokeRegisteredResolve(request).await?, &self.ownerPackage)
    }

    /// Resolves an existing execution after reading only this owner's canonical extension.
    pub async fn resolve(
        &self,
        mut request: ChatConfigurationRequest,
    ) -> Result<ChatConfigurationResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Execution)?;
        self.readConversationExtension(&mut request)?;
        self.invokeResolve(request).await
    }

    /// Reads only the exact authenticated owner's extension from the canonical persisted chat record.
    fn readConversationExtension(&self, request: &mut ChatConfigurationRequest) -> Result<(), String> {
        let chatId = request
            .chatId
            .as_deref()
            .ok_or("Persisted configuration resolve requires chatId")?;
        requireChatId(chatId)?;
        let target = operit_plugin_sdk::js_sdk::chat::ChatExtensionTarget::Chat {
            chatId: chatId.to_string(),
        };
        request.chatExtension = self
            .recordSupport
            .readChatExtension(&self.ownerPackage, &target)?
            .map(|value| value.into_iter().collect());
        Ok(())
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
        decodeDisplayConfigurationResult(self.invokeRegisteredResolve(request).await?, &self.ownerPackage)
    }

    /// Invokes the exact selected declaration and injects its authenticated owner after successful decoding.
    async fn invokeResolve(
        &self,
        request: ChatConfigurationRequest,
    ) -> Result<ChatConfigurationResult, String> {
        request.requirePurpose(ChatConfigurationPurpose::Execution)?;
        decodeConfigurationResult(self.invokeRegisteredResolve(request).await?, &self.ownerPackage)
    }

    /// Invokes one immutable selected registration without choosing a second owner or reinterpreting its result.
    async fn invokeRegisteredResolve(&self, request: ChatConfigurationRequest) -> Result<Value, String> {
        self
            .manager
            .invokeToolPkgPublicApi(
                &self.ownerPackage,
                "chat.configuration.resolve",
                serde_json::to_value(request).map_err(|error| error.to_string())?,
            )
            .await
    }
}

/// Rejects plugin-supplied runtime identity and validates display JSON without any execution-profile interpretation.
fn decodeDisplayConfigurationResult(raw: Value, owner: &str) -> Result<ChatConfigurationDisplayResult, String> {
    if owner.trim().is_empty() {
        return Err("Display configuration owner is not authenticated".to_string());
    }
    if raw.as_object().is_some_and(|object| object.contains_key("extensionOwner")) {
        return Err("Display configuration extensionOwner is assigned only by the runtime".to_string());
    }
    let mut result: ChatConfigurationDisplayResult = serde_json::from_value(raw).map_err(|error| error.to_string())?;
    result.extensionOwner=owner.to_string();
    result.validate()?;
    Ok(result)
}

/// Rejects plugin-supplied runtime identity and validates the exact selected owner's execution snapshot.
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
    match candidates.len() {
        0 => Err("No enabled plugin registers chat.configuration.resolve".to_string()),
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
        Box::pin(async move {
            let api = ChatConfigurationApi::ready(&self.tool_handler).await?;
            api.resolve(request).await
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
    use super::{decodeConfigurationResult, decodeDisplayConfigurationResult, requireChatId, selectChatConfigurationOwner};

    /// Selects arbitrary uniquely registered owners without a built-in identity whitelist.
    #[test]
    fn accepts_arbitrary_unique_owner() {
        assert_eq!(
            selectChatConfigurationOwner(vec!["org.example.dynamic.profile".into()]).unwrap(),
            "org.example.dynamic.profile"
        );
    }

    /// Preserves errors for missing and duplicate actual declarations.
    #[test]
    fn rejects_missing_and_duplicate_owners() {
        assert_eq!(
            selectChatConfigurationOwner(Vec::new()).unwrap_err(),
            "No enabled plugin registers chat.configuration.resolve"
        );
        assert_eq!(
            selectChatConfigurationOwner(vec!["owner-a".into(), "owner-b".into()]).unwrap_err(),
            "Duplicate chat.configuration.resolve registrations: owner-a, owner-b"
        );
    }

    /// Injects only the exact selected registration owner while preserving the plugin's complete opaque execution snapshot.
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
            "messageExtension":{"actor":{"id":"participant"}, "voice":"voice", "revision":"7"}
        });
        for owner in ["org.example.owner-a", "org.example.owner-b"] {
            let configuration = decodeConfigurationResult(raw.clone(), owner).unwrap();
            assert_eq!(configuration.extensionOwner, owner);
            assert_eq!(
                configuration.messageExtension,
                raw["messageExtension"].as_object().unwrap().clone()
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
        let raw=serde_json::json!({"contextKey":"opaque","identity":{"title":"Display","avatarUri":null},"participants":[],"initialMessages":[]});
        let display=decodeDisplayConfigurationResult(raw.clone(),"arbitrary.owner").unwrap();
        assert_eq!(display.extensionOwner,"arbitrary.owner");
        assert_eq!(serde_json::to_value(display).unwrap(),raw);
        assert!(decodeConfigurationResult(raw.clone(),"arbitrary.owner").is_err());
        let mut forged=raw.clone();forged["extensionOwner"]=serde_json::json!("forged.owner");
        assert!(decodeDisplayConfigurationResult(forged,"arbitrary.owner").is_err());
        assert!(decodeDisplayConfigurationResult(raw," ").is_err());
    }

}
