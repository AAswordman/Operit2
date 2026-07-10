use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

use operit_model::FunctionType::FunctionType;
use operit_model::MemorySearchConfig::MemorySearchConfig;
use operit_model::ModelConfigData::{ProviderProfile, ResolvedModelConfig};
use operit_model::PromptFunctionType::PromptFunctionType;
use operit_plugin_sdk::toolpkg::ToolPkgHooks::{
    decodeToolPkgHookResult, ToolPkgAiProviderRegistration,
};
use operit_providers::runtime_support::{
    ProviderCharacterPromptContext, ProviderFunctionModelBinding, ProviderMessageTiming,
    ProviderPackageInfo, ProviderRuntimeContext, ProviderRuntimeSupport,
    ProviderToolPkgAiProviderRegistration,
};
use operit_tools::tools::skill_runtime::SkillRepository::SkillRepository;
use operit_tools::tools::AIToolHandler::AIToolHandler;

use crate::data::preferences::ApiPreferences::ApiPreferences;
use crate::data::preferences::CharacterCardManager::CharacterCardManager;
use crate::data::preferences::FunctionalConfigManager::FunctionalConfigManager;
use crate::data::preferences::MemorySearchSettingsPreferences::MemorySearchSettingsPreferences;
use crate::data::preferences::ModelConfigManager::ModelConfigManager;
use crate::plugins::toolpkg::ToolPkgAiProviderRegistry::ToolPkgAiProviderRegistry;

/// Creates runtime-backed services required by provider crates.
pub struct ProviderRuntimeSupportService;

impl ProviderRuntimeSupportService {
    /// Creates provider runtime support bound to one tool handler instance.
    pub fn create(tool_handler: AIToolHandler) -> ProviderRuntimeContext {
        ProviderRuntimeContext::new(Arc::new(RuntimeProviderSupport { tool_handler }))
    }
}

/// Bridges provider-owned interfaces to runtime-owned managers and registries.
struct RuntimeProviderSupport {
    tool_handler: AIToolHandler,
}

impl ProviderRuntimeSupport for RuntimeProviderSupport {
    /// Returns the root directory used by runtime data.
    fn dataDir(&self) -> Result<PathBuf, String> {
        Ok(ApiPreferences::data_dir())
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
        inputTokens: i32,
        outputTokens: i32,
        cachedInputTokens: i32,
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

    /// Loads memory search settings for an owner key.
    fn memorySearchConfig(&self, ownerKey: &str) -> Result<MemorySearchConfig, String> {
        MemorySearchSettingsPreferences::new(ownerKey)
            .load()
            .map_err(|error| error.to_string())
    }

    /// Resolves character prompt data for a selected role card.
    fn characterPromptContext(
        &self,
        roleCardId: &str,
        promptFunctionType: PromptFunctionType,
    ) -> Result<ProviderCharacterPromptContext, String> {
        let manager = CharacterCardManager::getInstance();
        let activeCard = manager
            .getCharacterCard(roleCardId)
            .map_err(|error| error.to_string())?;
        let introPrompt = manager
            .combinePrompts(&activeCard.id, Vec::new(), promptFunctionType)
            .map_err(|error| error.to_string())?;
        let aiName = if activeCard.name.trim().is_empty() {
            "Operit".to_string()
        } else {
            activeCard.name.clone()
        };
        Ok(ProviderCharacterPromptContext {
            activeCard,
            introPrompt,
            aiName,
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

    /// Initializes model and function binding stores.
    fn initializeFunctionModelBindings(&self, rootDir: PathBuf) -> Result<(), String> {
        ModelConfigManager::new(rootDir.clone())
            .initializeIfNeeded()
            .map_err(|error| error.to_string())?;
        FunctionalConfigManager::new(rootDir)
            .initializeIfNeeded()
            .map_err(|error| error.to_string())
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
    ) -> Result<Option<String>, String> {
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
