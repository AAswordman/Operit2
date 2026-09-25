use async_trait::async_trait;
use reqwest::header::{HeaderName, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;
use std::ops::{Deref, DerefMut};

use crate::chat::llmprovider::AIService::{
    AIService, AiServiceError, SendMessageRequest,
};
use crate::chat::llmprovider::ClaudeProvider::ClaudeProvider;
use crate::chat::llmprovider::GeminiProvider::GeminiProvider;
use crate::chat::llmprovider::OpenAIProvider::OpenAIProvider;
use crate::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesProvider;
use crate::chat::llmprovider::ThinkingConfiguration::ThinkingConfigurationApplier;
use crate::runtime_support::ProviderRuntimeContext;
use operit_model::ModelConfigData::{ApiProviderType, ModelBuiltinTool};
use operit_model::OpenAIModels::ModelOption;
use operit_util::stream::RevisableTextStream::RevisableTextStreamLike;

/// Routes OpenCode Zen and Go models to the protocol-specific provider used by the runtime.
pub struct OpenCodeProvider {
    delegate: OpenCodeDelegate,
    base_endpoint: String,
    model_name: String,
    protocol: ApiProviderType,
    api_key: String,
    custom_headers: Vec<(String, String)>,
}

enum OpenCodeDelegate {
    Chat(OpenCodeChatProvider),
    Responses(OpenCodeResponsesProvider),
    Claude(OpenCodeClaudeProvider),
    Gemini(OpenCodeGeminiProvider),
}

/// OpenCode's OpenAI-compatible chat route.
struct OpenCodeChatProvider {
    inner: OpenAIProvider,
}

/// OpenCode's OpenAI Responses route.
struct OpenCodeResponsesProvider {
    inner: OpenAIResponsesProvider,
}

/// OpenCode's Anthropic-compatible route.
struct OpenCodeClaudeProvider {
    inner: ClaudeProvider,
}

/// OpenCode's Google-compatible route.
struct OpenCodeGeminiProvider {
    inner: GeminiProvider,
}

macro_rules! impl_deref {
    ($wrapper:ty, $inner:ty) => {
        impl Deref for $wrapper {
            type Target = $inner;

            /// Borrows the wrapped protocol provider.
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl DerefMut for $wrapper {
            /// Mutably borrows the wrapped protocol provider.
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }
    };
}

impl_deref!(OpenCodeChatProvider, OpenAIProvider);
impl_deref!(OpenCodeResponsesProvider, OpenAIResponsesProvider);
impl_deref!(OpenCodeClaudeProvider, ClaudeProvider);
impl_deref!(OpenCodeGeminiProvider, GeminiProvider);

/// Resolves OpenCode model ids to their upstream protocol and route.
pub struct OpenCodeRouting;

impl OpenCodeRouting {
    /// Selects the protocol used by one OpenCode model.
    pub fn protocol_for(
        base_endpoint: &str,
        model_name: &str,
    ) -> Result<ApiProviderType, String> {
        let model = model_name.trim().to_ascii_lowercase();
        let model_id = model
            .rsplit('/')
            .next()
            .expect("model id split always has one segment");
        let is_go_endpoint = Self::is_go(base_endpoint);
        if model_id.starts_with("gemini-") {
            return Ok(ApiProviderType::GEMINI_GENERIC);
        }
        if model_id.starts_with("claude-")
            || model_id.starts_with("qwen3.")
            || (is_go_endpoint && model_id.starts_with("minimax-"))
            || (!is_go_endpoint
                && model_id.starts_with("minimax-")
                && model_id.ends_with("-free"))
        {
            return Ok(ApiProviderType::ANTHROPIC_GENERIC);
        }
        if model_id.starts_with("gpt-")
            || model_id.starts_with("grok-")
            || model_id.starts_with("muse-spark-")
        {
            return Ok(ApiProviderType::OPENAI_RESPONSES_GENERIC);
        }
        if !is_go_endpoint && model_id.starts_with("minimax-") {
            return Ok(ApiProviderType::OPENAI_GENERIC);
        }
        let openai_prefixes = [
            "big-pickle",
            "deepseek-",
            "glm-",
            "hy3",
            "hy4-",
            "kimi-",
            "ling-",
            "longcat-",
            "mimo-",
            "nemotron-",
            "omen-",
            "qwen3-coder",
            "ring-",
            "north-",
            "laguna-",
            "trinity-",
            "x-preview-",
        ];
        if openai_prefixes
            .iter()
            .any(|prefix| model_id.starts_with(prefix))
        {
            return Ok(ApiProviderType::OPENAI_GENERIC);
        }
        Err(format!("Unsupported OpenCode model protocol: {model_name}"))
    }

    /// Builds the routed endpoint for one OpenCode model.
    pub fn endpoint_for(base_endpoint: &str, model_name: &str) -> Result<String, String> {
        let base = Self::normalized_base(base_endpoint);
        let protocol = Self::protocol_for(base_endpoint, model_name)?;
        let endpoint = match protocol {
            ApiProviderType::OPENAI_RESPONSES_GENERIC => format!("{base}/responses"),
            ApiProviderType::ANTHROPIC_GENERIC => format!("{base}/messages"),
            ApiProviderType::GEMINI_GENERIC => format!("{base}/models/{model_name}"),
            _ => format!("{base}/chat/completions"),
        };
        Ok(endpoint)
    }

    /// Builds the OpenCode model catalog endpoint.
    pub fn models_endpoint(base_endpoint: &str) -> String {
        format!("{}/models", Self::normalized_base(base_endpoint))
    }

    /// Returns whether an endpoint is the OpenCode Go gateway.
    pub fn is_go(endpoint: &str) -> bool {
        let trimmed = endpoint.trim().trim_end_matches('/').to_ascii_lowercase();
        trimmed.ends_with("/zen/go") || trimmed.ends_with("/zen/go/v1")
    }

    /// Returns the API base used by the OpenCode Gemini route.
    pub fn api_base(endpoint: &str) -> String {
        Self::normalized_base(
            endpoint
                .split("/models/")
                .next()
                .expect("endpoint split always has one segment"),
        )
    }

    /// Normalizes a Zen or Go endpoint to its v1 API base.
    fn normalized_base(endpoint: &str) -> String {
        let trimmed = endpoint.trim().trim_end_matches('/');
        if trimmed.ends_with("/v1") {
            trimmed.to_string()
        } else {
            format!("{trimmed}/v1")
        }
    }
}

impl OpenCodeProvider {
    /// Creates a routed OpenCode provider matching the Kotlin implementation.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_endpoint: String,
        api_key: String,
        model_name: String,
        custom_headers: Vec<(String, String)>,
        protocol: ApiProviderType,
        supports_vision: bool,
        supports_audio: bool,
        supports_video: bool,
        builtin_tools: Vec<ModelBuiltinTool>,
        enable_tool_call: bool,
        runtime_context: ProviderRuntimeContext,
    ) -> Result<Self, AiServiceError> {
        let model_name = model_name.trim();
        let model_name = if let Some(value) = model_name.strip_prefix("opencode/") {
            value
        } else if let Some(value) = model_name.strip_prefix("opencode-go/") {
            value
        } else {
            model_name
        }
        .to_string();
        let endpoint = OpenCodeRouting::endpoint_for(&base_endpoint, &model_name)
            .map_err(AiServiceError::RequestFailed)?;
        let mut headers = custom_headers;
        headers.push((
            "User-Agent".to_string(),
            format!("Operit/{}", env!("CARGO_PKG_VERSION")),
        ));
        if OpenCodeRouting::is_go(&base_endpoint) {
            headers.push((
                "x-opencode-session".to_string(),
                format!("operit-{}", model_name),
            ));
        }
        let delegate = match protocol {
            ApiProviderType::OPENAI_RESPONSES_GENERIC => OpenCodeDelegate::Responses(
                OpenCodeResponsesProvider {
                    inner: OpenAIResponsesProvider::new(
                    endpoint,
                    api_key.clone(),
                    model_name.clone(),
                    "OPENCODE".to_string(),
                    headers.clone(),
                    supports_vision,
                    supports_audio,
                    supports_video,
                    builtin_tools,
                    enable_tool_call,
                    runtime_context,
                    ),
                },
            ),
            ApiProviderType::ANTHROPIC_GENERIC => OpenCodeDelegate::Claude(OpenCodeClaudeProvider {
                inner: ClaudeProvider::new(
                    endpoint,
                    api_key.clone(),
                    model_name.clone(),
                    "OPENCODE".to_string(),
                    headers.clone(),
                    enable_tool_call,
                ),
            }),
            ApiProviderType::GEMINI_GENERIC => OpenCodeDelegate::Gemini(
                OpenCodeGeminiProvider {
                    inner: GeminiProvider::new_with_request_options(
                        endpoint,
                        api_key.clone(),
                        model_name.clone(),
                        "OPENCODE".to_string(),
                        headers.clone(),
                        builtin_tools,
                        enable_tool_call,
                        Some("x-goog-api-key".to_string()),
                    ),
                },
            ),
            _ => OpenCodeDelegate::Chat(OpenCodeChatProvider {
                inner: OpenAIProvider::new_with_capabilities_and_reasoning(
                    endpoint,
                    api_key.clone(),
                    model_name.clone(),
                    "OPENCODE".to_string(),
                    headers.clone(),
                    supports_vision,
                    supports_audio,
                    supports_video,
                    enable_tool_call,
                    true,
                ),
            }),
        };
        Ok(Self {
            delegate,
            base_endpoint,
            model_name,
            protocol,
            api_key,
            custom_headers: headers,
        })
    }

    /// Fetches the OpenCode model catalog using its OpenAI-compatible endpoint.
    async fn get_models_list_internal(&self) -> Result<Vec<ModelOption>, AiServiceError> {
        let mut request = reqwest::Client::new()
            .get(OpenCodeRouting::models_endpoint(&self.base_endpoint))
            .header(CONTENT_TYPE, "application/json");
        if !self.api_key.trim().is_empty() {
            request = request.header(AUTHORIZATION, format!("Bearer {}", self.api_key.trim()));
        }
        for (name, value) in &self.custom_headers {
            request = request.header(
                HeaderName::from_bytes(name.as_bytes())
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
                HeaderValue::from_str(value)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
            );
        }
        let response = request
            .send()
            .await
            .map_err(|error| AiServiceError::ConnectionFailed(error.to_string()))?;
        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        if !status.is_success() {
            return Err(AiServiceError::RequestFailed(format!(
                "{status}: {body}"
            )));
        }
        body.get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| AiServiceError::RequestFailed("OpenCode model catalog has no data".to_string()))?
            .iter()
            .map(|model| {
                let id = model
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AiServiceError::RequestFailed("OpenCode model has no id".to_string()))?;
                Ok(ModelOption {
                    id: id.to_string(),
                    name: id.to_string(),
                })
            })
            .collect()
    }
}

/// Delegates one provider operation to the selected protocol implementation.
macro_rules! delegate_provider_method {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match &mut $self.delegate {
            OpenCodeDelegate::Chat(provider) => provider.$method($($arg),*),
            OpenCodeDelegate::Responses(provider) => provider.$method($($arg),*),
            OpenCodeDelegate::Claude(provider) => provider.$method($($arg),*),
            OpenCodeDelegate::Gemini(provider) => provider.$method($($arg),*),
        }
    };
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for OpenCodeProvider {
    /// Returns the routed provider model identifier.
    fn provider_model(&self) -> String {
        format!("OPENCODE:{}", self.model_name)
    }

    /// Returns uncached input token usage.
    fn input_token_count(&self) -> i64 {
        match &self.delegate {
            OpenCodeDelegate::Chat(provider) => provider.input_token_count(),
            OpenCodeDelegate::Responses(provider) => provider.input_token_count(),
            OpenCodeDelegate::Claude(provider) => provider.input_token_count(),
            OpenCodeDelegate::Gemini(provider) => provider.input_token_count(),
        }
    }

    /// Returns cached input token usage.
    fn cached_input_token_count(&self) -> i64 {
        match &self.delegate {
            OpenCodeDelegate::Chat(provider) => provider.cached_input_token_count(),
            OpenCodeDelegate::Responses(provider) => provider.cached_input_token_count(),
            OpenCodeDelegate::Claude(provider) => provider.cached_input_token_count(),
            OpenCodeDelegate::Gemini(provider) => provider.cached_input_token_count(),
        }
    }

    /// Returns output token usage.
    fn output_token_count(&self) -> i64 {
        match &self.delegate {
            OpenCodeDelegate::Chat(provider) => provider.output_token_count(),
            OpenCodeDelegate::Responses(provider) => provider.output_token_count(),
            OpenCodeDelegate::Claude(provider) => provider.output_token_count(),
            OpenCodeDelegate::Gemini(provider) => provider.output_token_count(),
        }
    }

    /// Resets delegated token counters.
    fn reset_token_counts(&mut self) {
        delegate_provider_method!(self, reset_token_counts);
    }

    /// Cancels delegated streaming work.
    fn cancel_streaming(&mut self) {
        delegate_provider_method!(self, cancel_streaming);
    }

    /// Lists models exposed by OpenCode.
    async fn get_models_list(&self) -> Result<Vec<ModelOption>, AiServiceError> {
        self.get_models_list_internal().await
    }

    /// Sends a request through the selected OpenCode protocol.
    async fn send_message(
        &mut self,
        mut request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let settings = ThinkingConfigurationApplier::describe(
            "OPENCODE",
            &self.model_name,
            &self.base_endpoint,
            &request.thinking_configurations,
        )
        .map_err(AiServiceError::RequestFailed)?;
        request.enable_thinking = request.enable_thinking || settings.required;
        delegate_provider_method!(self, send_message, request).await
    }

    /// Tests the selected OpenCode endpoint.
    async fn test_connection(&self) -> Result<String, AiServiceError> {
        match &self.delegate {
            OpenCodeDelegate::Chat(provider) => provider.test_connection().await,
            OpenCodeDelegate::Responses(provider) => provider.test_connection().await,
            OpenCodeDelegate::Claude(provider) => provider.test_connection().await,
            OpenCodeDelegate::Gemini(provider) => provider.test_connection().await,
        }
    }

    /// Calculates input tokens with the selected protocol provider.
    async fn calculate_input_tokens(
        &self,
        chat_history: &[operit_model::PromptTurn::PromptTurn],
        available_tools: &[operit_model::ToolPrompt::ToolPrompt],
    ) -> Result<i64, AiServiceError> {
        match &self.delegate {
            OpenCodeDelegate::Chat(provider) => provider.calculate_input_tokens(chat_history, available_tools).await,
            OpenCodeDelegate::Responses(provider) => provider.calculate_input_tokens(chat_history, available_tools).await,
            OpenCodeDelegate::Claude(provider) => provider.calculate_input_tokens(chat_history, available_tools).await,
            OpenCodeDelegate::Gemini(provider) => provider.calculate_input_tokens(chat_history, available_tools).await,
        }
    }

    /// Releases delegated provider resources.
    fn release(&mut self) {
        delegate_provider_method!(self, release);
    }
}
