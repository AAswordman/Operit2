use super::AIService::{
    response_stream_from_chunks, AIService, AiServiceError, SendMessageRequest,
};
use super::OpenAIProvider::OpenAIProvider;
use crate::runtime_support::ProviderRuntimeContext;
use async_trait::async_trait;
use operit_model::ModelConfigData::LocalModelRuntimeSettings;
use operit_util::stream::RevisableTextStream::RevisableTextStreamLike;

/// Runs an installed LLM through runtime Binding routes, including same-device calls.
pub struct LocalChatAIProvider {
    model: String,
    settings: LocalModelRuntimeSettings,
    runtime: ProviderRuntimeContext,
    serializer: OpenAIProvider,
    inputTokens: i64,
    outputTokens: i64,
}
impl LocalChatAIProvider {
    pub fn new(
        model: String,
        settings: LocalModelRuntimeSettings,
        runtime: ProviderRuntimeContext,
    ) -> Self {
        let serializer = OpenAIProvider::new_with_capabilities(
            String::new(),
            String::new(),
            model.clone(),
            "LOCAL_MODEL".into(),
            Vec::new(),
            false,
            false,
            false,
            false,
        );
        Self {
            model,
            settings,
            runtime,
            serializer,
            inputTokens: 0,
            outputTokens: 0,
        }
    }
}
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for LocalChatAIProvider {
    fn input_token_count(&self) -> i64 {
        self.inputTokens
    }
    fn output_token_count(&self) -> i64 {
        self.outputTokens
    }
    fn provider_model(&self) -> String {
        format!("LOCAL_MODEL:{}", self.model)
    }
    fn reset_token_counts(&mut self) {
        self.inputTokens = 0;
        self.outputTokens = 0;
    }
    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let mut body = self
            .serializer
            .create_request_body_without_thinking(&request)?;
        body["stream"] = false.into();
        body["enable_thinking"] = request.enable_thinking.into();
        if let Some(maximum) = body.get("max_completion_tokens").cloned() {
            body["max_tokens"] = maximum;
        }
        let response = self
            .runtime
            .support()
            .generateLocalLlm(
                self.settings.inferenceBindingKey.clone(),
                self.model.clone(),
                body,
                serde_json::to_value(&self.settings)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
            )
            .await
            .map_err(AiServiceError::RequestFailed)?;
        self.inputTokens += response["metadata"]["inputTokens"].as_i64().unwrap_or(0);
        self.outputTokens += response["metadata"]["outputTokens"].as_i64().unwrap_or(0);
        let text = response["text"].as_str().ok_or_else(|| {
            AiServiceError::RequestFailed("Local inference returned no text".into())
        })?;
        Ok(response_stream_from_chunks(vec![text.to_string()]))
    }
    async fn calculate_input_tokens(
        &self,
        history: &[operit_model::PromptTurn::PromptTurn],
        tools: &[operit_model::ToolPrompt::ToolPrompt],
    ) -> Result<i64, AiServiceError> {
        self.serializer.calculate_input_tokens(history, tools).await
    }
    async fn test_connection(&self) -> Result<String, AiServiceError> {
        let response = self
            .runtime
            .support()
            .generateLocalLlm(
                self.settings.inferenceBindingKey.clone(),
                self.model.clone(),
                serde_json::json!({"messages":[{"role":"user","content":"Hi"}],"max_tokens":4}),
                serde_json::to_value(&self.settings)
                    .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?,
            )
            .await
            .map_err(AiServiceError::RequestFailed)?;
        Ok(response["text"].as_str().unwrap_or_default().to_string())
    }
}
