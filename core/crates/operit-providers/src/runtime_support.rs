use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;

use operit_model::CharacterCard::CharacterCard;
use operit_model::FunctionType::FunctionType;
use operit_model::MemorySearchConfig::MemorySearchConfig;
use operit_model::ModelConfigData::{ProviderProfile, ResolvedModelConfig};
use operit_model::PromptFunctionType::PromptFunctionType;

/// Describes the model binding selected for one runtime function.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct ProviderFunctionModelBinding {
    pub providerId: String,
    pub modelId: String,
}

/// Describes a ToolPkg-backed AI provider registration.
pub type ProviderToolPkgAiProviderRegistration =
    operit_plugin_sdk::toolpkg::ToolPkgHooks::ToolPkgAiProviderRegistration;

/// Captures a timestamp for provider request timing logs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderMessageTiming {
    pub startedAtMs: u64,
}

/// Describes a package surfaced in provider prompt composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderPackageInfo {
    pub name: String,
    pub description: String,
}

/// Describes character prompt data resolved by the runtime.
#[derive(Clone, Debug, PartialEq)]
#[allow(non_snake_case)]
pub struct ProviderCharacterPromptContext {
    pub activeCard: CharacterCard,
    pub introPrompt: String,
    pub aiName: String,
}

/// Supplies runtime-owned data and plugin operations to provider code.
pub trait ProviderRuntimeSupport: Send + Sync {
    /// Returns the root directory used by provider runtime data.
    fn dataDir(&self) -> Result<PathBuf, String>;

    /// Returns the current thinking quality level.
    fn thinkingQualityLevel(&self) -> Result<i32, String>;

    /// Records provider/model token usage.
    fn updateTokensForProviderModel(
        &self,
        providerModel: &str,
        inputTokens: i32,
        outputTokens: i32,
        cachedInputTokens: i32,
    ) -> Result<(), String>;

    /// Loads memory search settings for an owner key.
    fn memorySearchConfig(&self, ownerKey: &str) -> Result<MemorySearchConfig, String>;

    /// Resolves character prompt data for a selected role card.
    fn characterPromptContext(
        &self,
        roleCardId: &str,
        promptFunctionType: PromptFunctionType,
    ) -> Result<ProviderCharacterPromptContext, String>;

    /// Returns deployed skill package descriptions for provider prompt composition.
    fn aiVisibleSkillPackages(&self) -> Result<Vec<ProviderPackageInfo>, String>;

    /// Initializes function/model binding state.
    fn initializeFunctionModelBindings(&self, rootDir: PathBuf) -> Result<(), String>;

    /// Returns the model binding for a function.
    fn modelBindingForFunction(
        &self,
        rootDir: PathBuf,
        functionType: FunctionType,
    ) -> Result<ProviderFunctionModelBinding, String>;

    /// Returns the resolved model config for a provider/model pair.
    fn resolvedModelConfig(
        &self,
        rootDir: PathBuf,
        providerId: &str,
        modelId: &str,
    ) -> Result<ResolvedModelConfig, String>;

    /// Returns the provider profile for a provider id.
    fn providerProfile(
        &self,
        rootDir: PathBuf,
        providerId: &str,
    ) -> Result<ProviderProfile, String>;

    /// Returns whether a tool package AI provider is registered.
    fn hasToolPkgAiProvider(&self, providerId: &str) -> bool;

    /// Returns a tool package AI provider registration.
    fn toolPkgAiProvider(&self, providerId: &str) -> Option<ProviderToolPkgAiProviderRegistration>;

    /// Invokes a tool package AI provider function.
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
    ) -> Result<Option<String>, String>;

    /// Decodes a tool package hook result.
    fn decodeToolPkgHookResult(&self, raw: Option<String>) -> Option<Value>;

    /// Returns a provider timing snapshot.
    fn messageTimingNow(&self) -> ProviderMessageTiming;

    /// Writes a provider timing log entry.
    fn logMessageTiming(
        &self,
        stage: &str,
        startTimeMs: ProviderMessageTiming,
        details: Option<String>,
    );
}

/// Carries one runtime-specific provider support implementation.
#[derive(Clone)]
pub struct ProviderRuntimeContext {
    support: Arc<dyn ProviderRuntimeSupport>,
}

impl ProviderRuntimeContext {
    /// Creates a provider context from a caller-owned support implementation.
    pub fn new(support: Arc<dyn ProviderRuntimeSupport>) -> Self {
        Self { support }
    }

    /// Returns the runtime support implementation for this context.
    pub fn support(&self) -> &dyn ProviderRuntimeSupport {
        self.support.as_ref()
    }

    /// Clones the shared runtime support implementation.
    pub fn shared_support(&self) -> Arc<dyn ProviderRuntimeSupport> {
        self.support.clone()
    }
}
