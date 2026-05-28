use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;

use super::AIService::{
    response_stream_from_chunks, AIService, AiServiceError, SendMessageRequest,
};
use crate::core::chat::hooks::PromptTurn::PromptTurn;
use crate::core::tools::packTool::ToolPkgCommonPluginConstants::{
    TOOLPKG_EVENT_AI_PROVIDER_CALCULATE_INPUT_TOKENS, TOOLPKG_EVENT_AI_PROVIDER_LIST_MODELS,
    TOOLPKG_EVENT_AI_PROVIDER_SEND_MESSAGE, TOOLPKG_EVENT_AI_PROVIDER_TEST_CONNECTION,
};
use crate::data::model::ModelConfigData::ModelConfigData;
use crate::data::model::OpenAIModels::ModelOption;
use crate::data::model::ToolPrompt::ToolPrompt;
use crate::plugins::toolpkg::ToolPkgHookBridgeSupport::{
    decodeToolPkgHookResult, ToolPkgAiProviderRegistration,
};
use crate::util::stream::RevisableTextStream::RevisableTextStreamLike;
use crate::util::stream::RevisableTextStream::{
    empty_revisable_event_channel, with_event_channel_shared,
};

#[derive(Clone)]
pub struct ToolPkgJsAiProviderService {
    config: ModelConfigData,
    provider: ToolPkgAiProviderRegistration,
    tokenCounts: Arc<Mutex<ToolPkgProviderTokenCounts>>,
    executionChatId: String,
    providerRuntimeContextKey: String,
}

#[derive(Clone, Debug, Default)]
struct ToolPkgProviderTokenCounts {
    input: i32,
    cachedInput: i32,
    output: i32,
}

impl ToolPkgJsAiProviderService {
    pub fn new(config: ModelConfigData, provider: ToolPkgAiProviderRegistration) -> Self {
        let normalizedProviderId = provider.providerId.trim().to_ascii_lowercase();
        Self {
            executionChatId: format!(
                "toolpkg-ai-provider:{}:{}",
                provider.providerId,
                operit_host_api::TimeUtils::currentTimeMillis()
            ),
            providerRuntimeContextKey: format!(
                "toolpkg_provider:{}:{}",
                provider.containerPackageName, normalizedProviderId
            ),
            config,
            provider,
            tokenCounts: Arc::new(Mutex::new(ToolPkgProviderTokenCounts::default())),
        }
    }

    #[allow(non_snake_case)]
    fn invokeProviderFunction(
        &self,
        functionName: &str,
        functionSource: Option<&str>,
        event: &str,
        eventPayload: Value,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Result<Value, AiServiceError> {
        let mut payload = eventPayload;
        if let Value::Object(object) = &mut payload {
            object.insert(
                "chatId".to_string(),
                Value::String(self.executionChatId.clone()),
            );
        }
        crate::core::tools::AIToolHandler::AIToolHandler::getInstance(
            crate::core::application::OperitApplicationContext::OperitApplicationContext::new(),
        )
        .getOrCreatePackageManager()
        .lock()
        .expect("package manager mutex poisoned")
        .runToolPkgMainHook(
            &self.provider.containerPackageName,
            functionName,
            event,
            None,
            Some(&format!("{}:{}", self.provider.providerId, event)),
            functionSource,
            payload,
            Some(&self.providerRuntimeContextKey),
            Some("provider"),
            onIntermediateResult,
        )
        .map_err(AiServiceError::RequestFailed)
        .and_then(|raw| {
            decodeToolPkgHookResult(raw).ok_or_else(|| {
                AiServiceError::RequestFailed("ToolPkg AI provider call returned null".to_string())
            })
        })
    }

    #[allow(non_snake_case)]
    fn buildBasePayload(&self) -> Value {
        serde_json::json!({
            "providerId": self.provider.providerId,
            "providerDisplayName": self.provider.displayName,
            "providerDescription": self.provider.description,
            "config": self.serializeModelConfig(),
        })
    }

    #[allow(non_snake_case)]
    fn serializeModelConfig(&self) -> Value {
        serde_json::json!({
            "id": self.config.id,
            "name": self.config.name,
            "apiProviderType": self.config.apiProviderTypeId,
            "apiProviderTypeId": self.config.apiProviderTypeId,
            "apiKey": self.config.apiKey,
            "apiEndpoint": self.config.apiEndpoint,
            "modelName": self.config.modelName,
            "customHeaders": decodeJsonObjectString(&self.config.customHeaders),
            "customParameters": decodeJsonArrayString(&self.config.customParameters),
            "enableDirectImageProcessing": self.config.enableDirectImageProcessing,
            "enableDirectAudioProcessing": self.config.enableDirectAudioProcessing,
            "enableDirectVideoProcessing": self.config.enableDirectVideoProcessing,
            "enableGoogleSearch": self.config.enableGoogleSearch,
            "enableToolCall": self.config.enableToolCall,
            "requestLimitPerMinute": self.config.requestLimitPerMinute,
            "maxConcurrentRequests": self.config.maxConcurrentRequests,
            "locale": Value::Null,
        })
    }

    #[allow(non_snake_case)]
    fn applyUsage(&self, decoded: &Value) {
        let Some(usage) = extractUsage(
            decoded,
            &self.tokenCounts.lock().expect("token mutex poisoned"),
        ) else {
            return;
        };
        let mut counts = self.tokenCounts.lock().expect("token mutex poisoned");
        counts.input = usage.input;
        counts.cachedInput = usage.cachedInput;
        counts.output = usage.output;
    }

    #[allow(non_snake_case)]
    fn ensureNoFatalError(&self, decoded: &Value) -> Result<(), AiServiceError> {
        if let Value::Object(object) = decoded {
            let success = object
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            if !success {
                let message = object
                    .get("error")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("ToolPkg AI provider call failed");
                return Err(AiServiceError::RequestFailed(message.to_string()));
            }
        }
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for ToolPkgJsAiProviderService {
    fn input_token_count(&self) -> i32 {
        self.tokenCounts.lock().expect("token mutex poisoned").input
    }

    fn cached_input_token_count(&self) -> i32 {
        self.tokenCounts
            .lock()
            .expect("token mutex poisoned")
            .cachedInput
    }

    fn output_token_count(&self) -> i32 {
        self.tokenCounts
            .lock()
            .expect("token mutex poisoned")
            .output
    }

    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider.displayName, self.config.modelName)
    }

    fn reset_token_counts(&mut self) {
        *self.tokenCounts.lock().expect("token mutex poisoned") =
            ToolPkgProviderTokenCounts::default();
    }

    async fn get_models_list(&self) -> Result<Vec<ModelOption>, AiServiceError> {
        let decoded = self.invokeProviderFunction(
            &self.provider.listModelsFunctionName,
            self.provider.listModelsFunctionSource.as_deref(),
            TOOLPKG_EVENT_AI_PROVIDER_LIST_MODELS,
            self.buildBasePayload(),
            None,
        )?;
        self.ensureNoFatalError(&decoded)?;
        Ok(parseModelOptions(&decoded))
    }

    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn RevisableTextStreamLike>, AiServiceError> {
        let mut payload = self.buildBasePayload();
        if let Value::Object(object) = &mut payload {
            object.insert(
                "chatHistory".to_string(),
                Value::Array(
                    request
                        .chat_history
                        .iter()
                        .map(serializePromptTurn)
                        .collect(),
                ),
            );
            object.insert(
                "modelParameters".to_string(),
                Value::Array(
                    request
                        .model_parameters
                        .iter()
                        .map(|parameter| serde_json::to_value(parameter).unwrap_or(Value::Null))
                        .collect(),
                ),
            );
            object.insert(
                "availableTools".to_string(),
                Value::Array(
                    request
                        .available_tools
                        .iter()
                        .map(serializeToolPrompt)
                        .collect(),
                ),
            );
            object.insert(
                "enableThinking".to_string(),
                Value::Bool(request.enable_thinking),
            );
            object.insert("stream".to_string(), Value::Bool(request.stream));
            object.insert(
                "preserveThinkInHistory".to_string(),
                Value::Bool(request.preserve_think_in_history),
            );
            object.insert("enableRetry".to_string(), Value::Bool(request.enable_retry));
        }
        let stream = crate::util::stream::HotStream::MutableSharedStreamImpl::new(usize::MAX);
        let stream_for_intermediate = stream.clone();
        let has_intermediate_text_chunk = Arc::new(Mutex::new(false));
        let has_intermediate_text_chunk_for_callback = has_intermediate_text_chunk.clone();
        let token_counts = self.tokenCounts.clone();
        let on_non_fatal_error = request.on_non_fatal_error.clone();
        let decoded = self.invokeProviderFunction(
            &self.provider.sendMessageFunctionName,
            self.provider.sendMessageFunctionSource.as_deref(),
            TOOLPKG_EVENT_AI_PROVIDER_SEND_MESSAGE,
            payload,
            Some(Arc::new(move |raw| {
                let Some(decoded) = decodeToolPkgHookResult(Some(raw)) else {
                    return;
                };
                if let Some(usage) = extractUsage(
                    &decoded,
                    &token_counts.lock().expect("token mutex poisoned"),
                ) {
                    let mut counts = token_counts.lock().expect("token mutex poisoned");
                    counts.input = usage.input;
                    counts.cachedInput = usage.cachedInput;
                    counts.output = usage.output;
                }
                if let Some(error) = extractNonFatalError(&decoded) {
                    if let Some(callback) = &on_non_fatal_error {
                        callback(error);
                    }
                }
                for chunk in extractMessageChunks(&decoded) {
                    if !chunk.is_empty() {
                        *has_intermediate_text_chunk_for_callback
                            .lock()
                            .expect("intermediate chunk mutex poisoned") = true;
                        stream_for_intermediate.emit(chunk);
                    }
                }
            })),
        )?;
        self.ensureNoFatalError(&decoded)?;
        self.applyUsage(&decoded);
        if let Some(error) = extractNonFatalError(&decoded) {
            if let Some(callback) = request.on_non_fatal_error {
                callback(error);
            }
        }
        if !*has_intermediate_text_chunk
            .lock()
            .expect("intermediate chunk mutex poisoned")
        {
            for chunk in extractMessageChunks(&decoded) {
                if !chunk.is_empty() {
                    stream.emit(chunk);
                }
            }
        }
        stream.close();
        let event_channel = empty_revisable_event_channel();
        event_channel.close();
        Ok(Box::new(with_event_channel_shared(stream, event_channel)))
    }

    async fn test_connection(&self) -> Result<String, AiServiceError> {
        let decoded = self.invokeProviderFunction(
            &self.provider.testConnectionFunctionName,
            self.provider.testConnectionFunctionSource.as_deref(),
            TOOLPKG_EVENT_AI_PROVIDER_TEST_CONNECTION,
            self.buildBasePayload(),
            None,
        )?;
        self.ensureNoFatalError(&decoded)?;
        parseConnectionMessage(&decoded)
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i32, AiServiceError> {
        let mut payload = self.buildBasePayload();
        if let Value::Object(object) = &mut payload {
            object.insert(
                "chatHistory".to_string(),
                Value::Array(chat_history.iter().map(serializePromptTurn).collect()),
            );
            object.insert(
                "availableTools".to_string(),
                Value::Array(available_tools.iter().map(serializeToolPrompt).collect()),
            );
        }
        let decoded = self.invokeProviderFunction(
            &self.provider.calculateInputTokensFunctionName,
            self.provider.calculateInputTokensFunctionSource.as_deref(),
            TOOLPKG_EVENT_AI_PROVIDER_CALCULATE_INPUT_TOKENS,
            payload,
            None,
        )?;
        self.ensureNoFatalError(&decoded)?;
        parseTokenCount(&decoded)
    }

    fn release(&mut self) {}
}

#[allow(non_snake_case)]
fn serializePromptTurn(turn: &PromptTurn) -> Value {
    serde_json::json!({
        "kind": turn.kind,
        "content": turn.content,
        "toolName": turn.tool_name,
        "metadata": turn.metadata,
    })
}

#[allow(non_snake_case)]
fn serializeToolPrompt(tool: &ToolPrompt) -> Value {
    serde_json::json!({
        "name": tool.name,
        "description": tool.description,
        "parameters": tool.parameters,
        "parametersStructured": tool.parametersStructured,
        "details": tool.details,
        "notes": tool.notes,
    })
}

#[allow(non_snake_case)]
fn parseModelOptions(decoded: &Value) -> Vec<ModelOption> {
    let items = match decoded {
        Value::Object(object) => object.get("models").unwrap_or(decoded),
        _ => decoded,
    };
    match items {
        Value::Array(values) => values.iter().filter_map(parseModelOption).collect(),
        value => parseModelOption(value).into_iter().collect(),
    }
}

#[allow(non_snake_case)]
fn parseModelOption(raw: &Value) -> Option<ModelOption> {
    match raw {
        Value::String(value) => {
            let id = value.trim();
            if id.is_empty() {
                None
            } else {
                Some(ModelOption {
                    id: id.to_string(),
                    name: id.to_string(),
                })
            }
        }
        Value::Object(object) => {
            let id = object
                .get("id")
                .and_then(Value::as_str)
                .or_else(|| object.get("name").and_then(Value::as_str))
                .or_else(|| object.get("model").and_then(Value::as_str))
                .unwrap_or_default()
                .trim()
                .to_string();
            if id.is_empty() {
                None
            } else {
                let name = object
                    .get("name")
                    .and_then(Value::as_str)
                    .or_else(|| object.get("displayName").and_then(Value::as_str))
                    .or_else(|| object.get("title").and_then(Value::as_str))
                    .unwrap_or(&id)
                    .trim()
                    .to_string();
                Some(ModelOption { id, name })
            }
        }
        _ => None,
    }
}

#[allow(non_snake_case)]
fn parseConnectionMessage(decoded: &Value) -> Result<String, AiServiceError> {
    match decoded {
        Value::String(value) => Ok(nonBlankOr(value, "Connection successful")),
        Value::Bool(true) => Ok("Connection successful".to_string()),
        Value::Bool(false) => Err(AiServiceError::ConnectionFailed(
            "Connection failed".to_string(),
        )),
        Value::Object(object) => {
            let success = object
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            if !success {
                let message = object
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("Connection failed");
                return Err(AiServiceError::ConnectionFailed(message.to_string()));
            }
            Ok(object
                .get("message")
                .and_then(Value::as_str)
                .map(|value| nonBlankOr(value, "Connection successful"))
                .unwrap_or_else(|| "Connection successful".to_string()))
        }
        _ => Ok("Connection successful".to_string()),
    }
}

#[allow(non_snake_case)]
fn parseTokenCount(decoded: &Value) -> Result<i32, AiServiceError> {
    match decoded {
        Value::Number(value) => value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| {
                AiServiceError::TokenCalculationFailed("Invalid token count result".to_string())
            }),
        Value::String(value) => value.trim().parse::<i32>().map_err(|_| {
            AiServiceError::TokenCalculationFailed(format!("Invalid token count result: {value}"))
        }),
        Value::Object(object) => ["tokens", "inputTokens", "count"]
            .iter()
            .find_map(|key| object.get(*key).and_then(Value::as_i64))
            .and_then(|value| i32::try_from(value).ok())
            .ok_or_else(|| {
                AiServiceError::TokenCalculationFailed("Invalid token count result".to_string())
            }),
        _ => Err(AiServiceError::TokenCalculationFailed(
            "Invalid token count result".to_string(),
        )),
    }
}

#[derive(Clone, Debug)]
struct TokenUsage {
    input: i32,
    cachedInput: i32,
    output: i32,
}

#[allow(non_snake_case)]
fn extractUsage(decoded: &Value, current: &ToolPkgProviderTokenCounts) -> Option<TokenUsage> {
    let Value::Object(object) = decoded else {
        return None;
    };
    let source = object
        .get("usage")
        .and_then(Value::as_object)
        .unwrap_or(object);
    let input = read_i32(source, "input").or_else(|| read_i32(source, "inputTokens"));
    let cachedInput =
        read_i32(source, "cachedInput").or_else(|| read_i32(source, "cachedInputTokens"));
    let output = read_i32(source, "output").or_else(|| read_i32(source, "outputTokens"));
    if input.is_none() && cachedInput.is_none() && output.is_none() {
        return None;
    }
    Some(TokenUsage {
        input: input.unwrap_or(current.input),
        cachedInput: cachedInput.unwrap_or(current.cachedInput),
        output: output.unwrap_or(current.output),
    })
}

#[allow(non_snake_case)]
fn extractNonFatalError(decoded: &Value) -> Option<String> {
    decoded
        .as_object()
        .and_then(|object| object.get("nonFatalError"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

#[allow(non_snake_case)]
fn extractMessageChunks(decoded: &Value) -> Vec<String> {
    match decoded {
        Value::String(value) => {
            if value.is_empty() {
                Vec::new()
            } else {
                vec![value.clone()]
            }
        }
        Value::Object(object) => {
            let mut chunks = Vec::new();
            if let Some(value) = object.get("chunk").and_then(Value::as_str) {
                if !value.is_empty() {
                    chunks.push(value.to_string());
                }
            }
            if let Some(Value::Array(values)) = object.get("chunks") {
                for value in values {
                    if let Some(chunk) = value.as_str() {
                        if !chunk.is_empty() {
                            chunks.push(chunk.to_string());
                        }
                    }
                }
            }
            if let Some(value) = object.get("text").and_then(Value::as_str) {
                if !value.is_empty() {
                    chunks.push(value.to_string());
                }
            } else if let Some(value) = object.get("content").and_then(Value::as_str) {
                if !value.is_empty() {
                    chunks.push(value.to_string());
                }
            }
            chunks
        }
        _ => Vec::new(),
    }
}

fn read_i32(object: &serde_json::Map<String, Value>, key: &str) -> Option<i32> {
    object
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

#[allow(non_snake_case)]
fn decodeJsonObjectString(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return serde_json::json!({});
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::json!({}))
}

#[allow(non_snake_case)]
fn decodeJsonArrayString(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return serde_json::json!([]);
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| serde_json::json!([]))
}

#[allow(non_snake_case)]
fn nonBlankOr(value: &str, text: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        text.to_string()
    } else {
        value.to_string()
    }
}
