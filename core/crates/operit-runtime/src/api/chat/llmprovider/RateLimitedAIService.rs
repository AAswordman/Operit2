use std::sync::Arc;

use async_trait::async_trait;

use super::AIService::{
    AIService, AiResponseStream, AiServiceError, SendMessageRequest,
};
use crate::api::chat::llmprovider::RequestConcurrencyRegistry::RequestSemaphore;
use crate::api::chat::llmprovider::SlidingWindowRateLimiter::SlidingWindowRateLimiter;
use crate::core::chat::hooks::PromptTurn::PromptTurn;
use crate::data::model::OpenAIModels::ModelOption;
use crate::data::model::ToolPrompt::ToolPrompt;

pub struct RateLimitedAIService {
    delegate: Box<dyn AIService>,
    rateLimiter: Option<Arc<SlidingWindowRateLimiter>>,
    concurrencySemaphore: Option<Arc<RequestSemaphore>>,
}

impl RateLimitedAIService {
    pub fn new(
        delegate: Box<dyn AIService>,
        rateLimiter: Option<Arc<SlidingWindowRateLimiter>>,
        concurrencySemaphore: Option<Arc<RequestSemaphore>>,
    ) -> Self {
        Self {
            delegate,
            rateLimiter,
            concurrencySemaphore,
        }
    }
}

#[async_trait]
impl AIService for RateLimitedAIService {
    fn input_token_count(&self) -> i32 {
        self.delegate.input_token_count()
    }

    fn cached_input_token_count(&self) -> i32 {
        self.delegate.cached_input_token_count()
    }

    fn output_token_count(&self) -> i32 {
        self.delegate.output_token_count()
    }

    fn provider_model(&self) -> String {
        self.delegate.provider_model()
    }

    fn reset_token_counts(&mut self) {
        self.delegate.reset_token_counts();
    }

    fn cancel_streaming(&mut self) {
        self.delegate.cancel_streaming();
    }

    async fn get_models_list(&self) -> Result<Vec<ModelOption>, AiServiceError> {
        self.delegate.get_models_list().await
    }

    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<AiResponseStream, AiServiceError> {
        if let Some(rateLimiter) = &self.rateLimiter {
            rateLimiter.acquire();
        }
        if let Some(semaphore) = &self.concurrencySemaphore {
            semaphore.acquire();
        }

        let result = self.delegate.send_message(request).await;

        if let Some(semaphore) = &self.concurrencySemaphore {
            semaphore.release();
        }

        result
    }

    async fn test_connection(&self) -> Result<String, AiServiceError> {
        self.delegate.test_connection().await
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        self.delegate
            .calculate_input_tokens(chat_history, available_tools)
            .await
    }

    fn release(&mut self) {
        self.delegate.release();
    }
}
