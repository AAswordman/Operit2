use crate::core::chat::hooks::ActivePromptHookMetadata::build_active_prompt_hook_metadata;
use crate::core::chat::hooks::PromptHookRegistry::{PromptHookContext, PromptHookRegistry};
use serde_json::Value;
use std::collections::HashMap;

pub struct InputProcessor;

#[derive(Clone, Debug)]
pub struct ProcessUserInputRequest {
    pub input: String,
    pub chat_id: Option<String>,
    pub role_card_id: Option<String>,
}

impl InputProcessor {
    pub fn process_user_input(request: ProcessUserInputRequest) -> String {
        let active_prompt_metadata =
            build_active_prompt_hook_metadata(request.chat_id.as_deref(), request.role_card_id.as_deref());
        let mut metadata = HashMap::<String, Value>::new();
        metadata.insert(
            "activePrompt".to_string(),
            Value::Object(active_prompt_metadata.to_value_map()),
        );

        let before_context = PromptHookRegistry::dispatchPromptInputHooks(PromptHookContext {
            stage: "before_process".to_string(),
            chat_id: request.chat_id.clone(),
            raw_input: Some(request.input.clone()),
            processed_input: Some(request.input.clone()),
            metadata,
            ..PromptHookContext::default()
        });

        let processed_input = before_context
            .processed_input
            .clone()
            .or(before_context.raw_input.clone())
            .unwrap_or(request.input);

        let after_context = PromptHookRegistry::dispatchPromptInputHooks(PromptHookContext {
            stage: "after_process".to_string(),
            chat_id: before_context.chat_id.clone(),
            raw_input: before_context.raw_input.clone(),
            processed_input: Some(processed_input.clone()),
            metadata: before_context.metadata.clone(),
            ..PromptHookContext::default()
        });

        after_context.processed_input.unwrap_or(processed_input)
    }
}
