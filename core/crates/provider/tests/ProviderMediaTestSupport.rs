//! Shared request/image fixtures for the Kotlin multimodal parity tests.

use crate::chat::llmprovider::AIService::SendMessageRequest;
use crate::chat::llmprovider::MediaLinkBuilder::MediaLinkBuilder;
use crate::chat::llmprovider::ThinkingDialect::RequestThinking;
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

/// Owns one unique pool entry without clearing other tests' images.
pub(crate) struct TestImage(String);

impl TestImage {
    /// Registers a complete image through the production normalization boundary.
    pub(crate) fn new() -> Self {
        let id = ImagePoolManager::add_image_bytes(
            include_bytes!("fixtures/pixel.png"), Some("image/png"), None,
        );
        assert_ne!(id, "error");
        Self(id)
    }

    /// Returns the serialized image reference used in prompt fixtures.
    pub(crate) fn link(&self) -> String {
        MediaLinkBuilder::image(&self.0)
    }

    /// Returns the actual normalized payload expected in provider request bodies.
    pub(crate) fn data_url(&self) -> String {
        let image = ImagePoolManager::get_image(&self.0).unwrap();
        format!("data:{};base64,{}", image.mime_type, image.base64)
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
        thinking: RequestThinking::default(),
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
        _functionType: FunctionType,
    ) -> Result<ProviderFunctionModelBinding, String> {
        Err("test runtime does not expose model bindings".to_string())
    }

    /// Returns no model configuration in the isolated test runtime.
    fn resolvedModelConfig(
        &self,
        _providerId: &str,
        _modelId: &str,
    ) -> Result<ResolvedModelConfig, String> {
        Err("test runtime does not expose model configuration".to_string())
    }

    /// Returns no provider profile in the isolated test runtime.
    fn providerProfile(
        &self,
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
