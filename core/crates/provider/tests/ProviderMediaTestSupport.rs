//! Shared request/image fixtures for the Kotlin multimodal parity tests.

use crate::chat::llmprovider::AIService::SendMessageRequest;
use crate::chat::llmprovider::MediaLinkBuilder::MediaLinkBuilder;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::ToolPrompt;
use operit_util::ImagePoolManager::ImagePoolManager;

use crate::runtime_support::{
    ChatConfigurationRequest, ChatConfigurationResult, ProviderFunctionModelBinding,
    ProviderMessageTiming, ProviderPackageInfo, ProviderRuntimeContext, ProviderRuntimeSupport,
    ProviderToolPkgAiProviderRegistration,
};
use operit_model::FunctionType::FunctionType;
use operit_model::ModelConfigData::{ProviderProfile, ResolvedModelConfig};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) const IMAGE_DATA_URL: &str = "data:image/png;base64,aW1hZ2U=";

/// Owns one unique pool entry without clearing other tests' images.
pub(crate) struct TestImage(String);

impl TestImage {
    pub(crate) fn new() -> Self {
        let id = ImagePoolManager::add_image_from_base64("aW1hZ2U=", "image/png", None);
        assert_ne!(id, "error");
        Self(id)
    }

    pub(crate) fn link(&self) -> String {
        MediaLinkBuilder::image(&self.0)
    }
}

impl Drop for TestImage {
    fn drop(&mut self) {
        ImagePoolManager::remove_image(&self.0);
    }
}

pub(crate) fn send_request(chat_history: Vec<PromptTurn>) -> SendMessageRequest {
    SendMessageRequest {
        chat_history,
        model_parameters: Vec::new(),
        enable_thinking: false,
        thinking_quality_level: 1,
        thinking_configurations: "[]".to_string(),
        thinking_option_id: String::new(),
        stream: false,
        available_tools: Vec::new(),
        preserve_think_in_history: false,
        enable_retry: false,
        on_non_fatal_error: None,
        on_tool_invocation: None,
    }
}

pub(crate) fn tool_image_request(image: &TestImage) -> SendMessageRequest {
    let mut request = send_request(vec![
        PromptTurn::new(
            PromptTurnKind::TOOL_CALL,
            "<tool name=\"read_file\"><param name=\"path\">/tmp/image.png</param></tool>",
        ),
        PromptTurn::new(
            PromptTurnKind::TOOL_RESULT,
            format!(
                "<tool_result name=\"read_file\"><content>tool text{}</content></tool_result>",
                image.link()
            ),
        ),
    ]);
    request.available_tools = vec![ToolPrompt::new(
        "read_file".to_string(),
        "Read a file".to_string(),
    )];
    request
}

/// Supplies deterministic runtime capabilities for request-shape tests.
struct TestRuntimeSupport;

impl ProviderRuntimeSupport for TestRuntimeSupport {
    /// Returns no filesystem root in the isolated test runtime.
    fn dataDir(&self) -> Result<PathBuf, String> {
        Err("test runtime does not expose a data directory".to_string())
    }

    /// Returns a stable thinking quality level for the test runtime.
    fn thinkingQualityLevel(&self) -> Result<i32, String> {
        Ok(2)
    }

    /// Accepts token updates without persisting them.
    fn updateTokensForProviderModel(
        &self,
        _providerModel: &str,
        _inputTokens: i64,
        _outputTokens: i64,
        _cachedInputTokens: i64,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Does not disable profile text in the isolated media formatting fixture.
    fn disableUserPreferenceDescription(&self) -> Result<bool, String> {
        Ok(false)
    }

    /// Rejects speech configuration lookup in this isolated nonproduction media fixture.
    fn defaultTtsConfigId(&self) -> Result<String, String> {
        Err("Media formatting tests do not resolve speech configuration".to_string())
    }

    /// Rejects configuration execution because this fixture intentionally tests no plugin runtime.
    fn resolveChatConfiguration(
        &self,
        _request: ChatConfigurationRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ChatConfigurationResult, String>> + Send + '_>,
    > {
        Box::pin(async { Err("Media formatting tests do not invoke chat plugins".to_string()) })
    }

    /// Returns no visible skill packages in the isolated test runtime.
    fn aiVisibleSkillPackages(&self) -> Result<Vec<ProviderPackageInfo>, String> {
        Ok(Vec::new())
    }

    /// Returns no function binding in the isolated test runtime.
    fn modelBindingForFunction(
        &self,
        _rootDir: PathBuf,
        _functionType: FunctionType,
    ) -> Result<ProviderFunctionModelBinding, String> {
        Err("test runtime does not expose model bindings".to_string())
    }

    /// Returns no model configuration in the isolated test runtime.
    fn resolvedModelConfig(
        &self,
        _rootDir: PathBuf,
        _providerId: &str,
        _modelId: &str,
    ) -> Result<ResolvedModelConfig, String> {
        Err("test runtime does not expose model configuration".to_string())
    }

    /// Returns no provider profile in the isolated test runtime.
    fn providerProfile(
        &self,
        _rootDir: PathBuf,
        _providerId: &str,
    ) -> Result<ProviderProfile, String> {
        Err("test runtime does not expose provider profiles".to_string())
    }

    /// Reports that no package AI provider is registered in the test runtime.
    fn hasToolPkgAiProvider(&self, _providerId: &str) -> bool {
        false
    }

    /// Returns no package AI provider registration in the test runtime.
    fn toolPkgAiProvider(
        &self,
        _providerId: &str,
    ) -> Option<ProviderToolPkgAiProviderRegistration> {
        None
    }

    /// Rejects package hook execution in the isolated test runtime.
    fn runToolPkgAiProviderHook(
        &self,
        _containerPackageName: &str,
        _functionName: &str,
        _functionSource: Option<&str>,
        _event: &str,
        _tag: Option<String>,
        _sourceKey: Option<String>,
        _eventPayload: Value,
        _runtimeContextKey: Option<String>,
        _executionKind: Option<String>,
        _onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<Result<Option<String>, String>> {
        Box::pin(async move { Err("test runtime does not execute package hooks".to_string()) })
    }

    /// Does not decode package hook output in the test runtime.
    fn decodeToolPkgHookResult(&self, _raw: Option<String>) -> Option<Value> {
        None
    }

    /// Returns a zero timestamp for deterministic timing assertions.
    fn messageTimingNow(&self) -> ProviderMessageTiming {
        ProviderMessageTiming { startedAtMs: 0 }
    }

    /// Ignores timing records in the isolated test runtime.
    fn logMessageTiming(
        &self,
        _stage: &str,
        _startTimeMs: ProviderMessageTiming,
        _details: Option<String>,
    ) {
    }
}

/// Builds the isolated runtime context used by provider request tests.
pub(crate) fn test_runtime_context() -> ProviderRuntimeContext {
    ProviderRuntimeContext::new(Arc::new(TestRuntimeSupport))
}
