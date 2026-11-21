use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::watch;

use crate::chat::llmprovider::AIService::{
    AIService, AiServiceError, SendMessageRequest, SharedAiResponseStream,
};
use crate::runtime_support::{ProviderRuntimeContext, ProviderToolPkgAiProviderRegistration};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_model::ModelConfigData::ResolvedModelConfig;
use operit_model::OpenAIModels::ModelOption;
use operit_model::PromptTurn::PromptTurn;
use operit_model::ToolPrompt::ToolPrompt;
use operit_plugin_sdk::toolpkg::ToolPkgCommonPluginConstants::{
    TOOLPKG_EVENT_AI_PROVIDER_CALCULATE_INPUT_TOKENS, TOOLPKG_EVENT_AI_PROVIDER_LIST_MODELS,
    TOOLPKG_EVENT_AI_PROVIDER_SEND_MESSAGE, TOOLPKG_EVENT_AI_PROVIDER_TEST_CONNECTION,
};
use operit_util::stream::HotStream::mutable_shared_stream;
use operit_util::stream::RevisableTextStream::RevisableTextStreamLike;

#[derive(Clone)]
pub struct ToolPkgJsAiProviderService {
    config: ResolvedModelConfig,
    provider: ProviderToolPkgAiProviderRegistration,
    tokenCounts: Arc<Mutex<ToolPkgProviderTokenCounts>>,
    cancelState: Arc<Mutex<ToolPkgCancelState>>,
    executionChatId: String,
    providerRuntimeContextKey: String,
    runtimeContext: ProviderRuntimeContext,
    keyGetter: super::ApiKeyProvider::KeyGetter,
}

#[derive(Clone, Debug, Default)]
struct ToolPkgProviderTokenCounts {
    input: i64,
    cachedInput: i64,
    output: i64,
}

/// Shared cancellation state mirrored across every clone of one provider service.
struct ToolPkgCancelState {
    cancelled: bool,
    cancelGeneration: u64,
    cancelSignal: watch::Sender<u64>,
}

impl Default for ToolPkgCancelState {
    fn default() -> Self {
        let (cancelSignal, _) = watch::channel(0);
        Self {
            cancelled: false,
            cancelGeneration: 0,
            cancelSignal,
        }
    }
}

impl ToolPkgJsAiProviderService {
    /// Creates a ToolPkg-backed provider bound to one provider runtime context.
    pub fn new(
        config: ResolvedModelConfig,
        provider: ProviderToolPkgAiProviderRegistration,
        runtimeContext: ProviderRuntimeContext,
        keyGetter: super::ApiKeyProvider::KeyGetter,
    ) -> Self {
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
            cancelState: Arc::new(Mutex::new(ToolPkgCancelState::default())),
            runtimeContext,
            keyGetter,
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelState
            .lock()
            .map(|state| state.cancelled)
            .unwrap_or(false)
    }

    fn begin_request(&self) {
        if let Ok(mut state) = self.cancelState.lock() {
            state.cancelled = false;
        }
    }

    fn mark_cancelled(&self) {
        if let Ok(mut state) = self.cancelState.lock() {
            state.cancelled = true;
            state.cancelGeneration = state.cancelGeneration.wrapping_add(1);
            let _ = state.cancelSignal.send(state.cancelGeneration);
        }
    }

    fn cancel_receiver(&self) -> watch::Receiver<u64> {
        self.cancelState
            .lock()
            .expect("ToolPkg cancel mutex poisoned")
            .cancelSignal
            .subscribe()
    }

    async fn waitForCancellation(mut receiver: watch::Receiver<u64>, observedGeneration: u64) {
        while receiver.changed().await.is_ok() {
            if *receiver.borrow() != observedGeneration {
                break;
            }
        }
    }

    #[allow(non_snake_case)]
    async fn invokeProviderFunction(
        &self,
        functionName: &str,
        functionSource: Option<&str>,
        event: &str,
        eventPayload: Value,
        onIntermediateResult: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Result<Value, AiServiceError> {
        let mut payload = eventPayload;
        payload["config"]["apiKey"] = Value::String(
            self.keyGetter.acquire().map_err(AiServiceError::RequestFailed)?,
        );
        if let Value::Object(object) = &mut payload {
            object.insert(
                "chatId".to_string(),
                Value::String(self.executionChatId.clone()),
            );
        }
        let sourceKey = format!("{}:{}", self.provider.providerId, event);
        let raw = self
            .runtimeContext
            .support()
            .runToolPkgAiProviderHook(
                &self.provider.containerPackageName,
                functionName,
                functionSource,
                event,
                None,
                Some(sourceKey),
                payload,
                Some(self.providerRuntimeContextKey.clone()),
                Some("provider".to_string()),
                onIntermediateResult,
            )
            .await
            .map_err(AiServiceError::RequestFailed)?;
        self.runtimeContext
            .support()
            .decodeToolPkgHookResult(raw)
            .ok_or_else(|| {
                AiServiceError::RequestFailed("ToolPkg AI provider call returned null".to_string())
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
            "providerId": self.config.providerId,
            "providerName": self.config.providerName,
            "modelId": self.config.modelId,
            "apiProviderType": self.config.apiProviderTypeId,
            "apiProviderTypeId": self.config.apiProviderTypeId,
            "apiEndpoint": self.config.apiEndpoint,
            "modelId": self.config.modelId,
            "customHeaders": decodeJsonObjectString(&self.config.customHeaders),
            "modelParameters": self.config.parameters,
            "enableDirectImageProcessing": self.config.capabilities.directImage,
            "enableDirectAudioProcessing": self.config.capabilities.directAudio,
            "enableDirectVideoProcessing": self.config.capabilities.directVideo,
            "builtinTools": self.config.builtinTools,
            "enableToolCall": self.config.capabilities.toolCall,
            "requestLimitPerMinute": self.config.requestLimitPerMinute,
            "maxConcurrentRequests": self.config.maxConcurrentRequests,
            "locale": Value::Null,
        })
    }

    #[allow(non_snake_case)]
    /// Applies provider usage while holding the counters once.
    fn applyUsage(&self, decoded: &Value) {
        applyTokenUsage(&self.tokenCounts, decoded);
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

    #[allow(non_snake_case)]
    async fn runSendMessageHook(
        &self,
        payload: Value,
        request: &SendMessageRequest,
        producer: &SharedAiResponseStream,
    ) -> Result<(), AiServiceError> {
        let has_intermediate_text_chunk = Arc::new(Mutex::new(false));
        let has_intermediate_text_chunk_for_callback = has_intermediate_text_chunk.clone();
        let token_counts = self.tokenCounts.clone();
        let on_non_fatal_error = request.on_non_fatal_error.clone();
        let runtime_support = self.runtimeContext.shared_support();
        let stream_for_intermediate = producer.clone();
        let decoded = self
            .invokeProviderFunction(
                &self.provider.sendMessageFunctionName,
                self.provider.sendMessageFunctionSource.as_deref(),
                TOOLPKG_EVENT_AI_PROVIDER_SEND_MESSAGE,
                payload,
                Some(Arc::new(move |raw| {
                    let Some(decoded) = runtime_support.decodeToolPkgHookResult(Some(raw)) else {
                        return;
                    };
                    applyTokenUsage(&token_counts, &decoded);
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
                            stream_for_intermediate.emit_chunk(chunk);
                        }
                    }
                })),
            )
            .await?;
        self.ensureNoFatalError(&decoded)?;
        self.applyUsage(&decoded);
        if let Some(error) = extractNonFatalError(&decoded) {
            if let Some(callback) = &request.on_non_fatal_error {
                callback(error);
            }
        }
        if !*has_intermediate_text_chunk
            .lock()
            .expect("intermediate chunk mutex poisoned")
        {
            for chunk in extractMessageChunks(&decoded) {
                if !chunk.is_empty() {
                    producer.emit_chunk(chunk);
                }
            }
        }
        Ok(())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AIService for ToolPkgJsAiProviderService {
    fn input_token_count(&self) -> i64 {
        self.tokenCounts.lock().expect("token mutex poisoned").input
    }

    fn cached_input_token_count(&self) -> i64 {
        self.tokenCounts
            .lock()
            .expect("token mutex poisoned")
            .cachedInput
    }

    fn output_token_count(&self) -> i64 {
        self.tokenCounts
            .lock()
            .expect("token mutex poisoned")
            .output
    }

    fn provider_model(&self) -> String {
        format!("{}:{}", self.provider.displayName, self.config.modelId)
    }

    fn reset_token_counts(&mut self) {
        *self.tokenCounts.lock().expect("token mutex poisoned") =
            ToolPkgProviderTokenCounts::default();
    }

    fn cancel_streaming(&mut self) {
        self.mark_cancelled();
    }

    async fn get_models_list(&self) -> Result<Vec<ModelOption>, AiServiceError> {
        let decoded = self
            .invokeProviderFunction(
                &self.provider.listModelsFunctionName,
                self.provider.listModelsFunctionSource.as_deref(),
                TOOLPKG_EVENT_AI_PROVIDER_LIST_MODELS,
                self.buildBasePayload(),
                None,
            )
            .await?;
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
                Value::Bool(request.thinking.is_on()),
            );
            object.insert("stream".to_string(), Value::Bool(request.stream));
            object.insert(
                "preserveThinkInHistory".to_string(),
                Value::Bool(request.preserve_think_in_history),
            );
            object.insert("enableRetry".to_string(), Value::Bool(request.enable_retry));
        }
        // Background the hook so the service lock frees immediately and cancel_streaming can interrupt it.
        let output = SharedAiResponseStream::new_ordered(
            mutable_shared_stream(usize::MAX),
            mutable_shared_stream(usize::MAX),
        );
        let producer = output.clone();
        let provider = self.clone();
        let receiver = provider.cancel_receiver();
        let observedGeneration = *receiver.borrow();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "toolpkg-ai-provider-response-stream",
                Box::new(move || {
                    Box::pin(async move {
                        let result = if provider.is_cancelled() {
                            Err(AiServiceError::RequestCancelled)
                        } else {
                            tokio::select! {
                                result = provider.runSendMessageHook(payload, &request, &producer) => result,
                                _ = ToolPkgJsAiProviderService::waitForCancellation(receiver, observedGeneration) => {
                                    Err(AiServiceError::RequestCancelled)
                                }
                            }
                        };
                        match result {
                            Ok(()) | Err(AiServiceError::RequestCancelled) => {}
                            Err(error) => producer.set_terminal_failure(error.to_string()),
                        }
                        producer.close();
                    })
                }),
            )
            .map_err(|error| AiServiceError::RequestFailed(error.to_string()))?;
        Ok(Box::new(output))
    }

    async fn test_connection(&self) -> Result<String, AiServiceError> {
        let decoded = self
            .invokeProviderFunction(
                &self.provider.testConnectionFunctionName,
                self.provider.testConnectionFunctionSource.as_deref(),
                TOOLPKG_EVENT_AI_PROVIDER_TEST_CONNECTION,
                self.buildBasePayload(),
                None,
            )
            .await?;
        self.ensureNoFatalError(&decoded)?;
        parseConnectionMessage(&decoded)
    }

    async fn calculate_input_tokens(
        &self,
        chat_history: &[PromptTurn],
        available_tools: &[ToolPrompt],
    ) -> Result<i64, AiServiceError> {
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
        let decoded = self
            .invokeProviderFunction(
                &self.provider.calculateInputTokensFunctionName,
                self.provider.calculateInputTokensFunctionSource.as_deref(),
                TOOLPKG_EVENT_AI_PROVIDER_CALCULATE_INPUT_TOKENS,
                payload,
                None,
            )
            .await?;
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
fn parseTokenCount(decoded: &Value) -> Result<i64, AiServiceError> {
    match decoded {
        Value::Number(value) => value.as_i64().ok_or_else(|| {
            AiServiceError::TokenCalculationFailed("Invalid token count result".to_string())
        }),
        Value::String(value) => value.trim().parse::<i64>().map_err(|_| {
            AiServiceError::TokenCalculationFailed(format!("Invalid token count result: {value}"))
        }),
        Value::Object(object) => ["tokens", "inputTokens", "count"]
            .iter()
            .find_map(|key| object.get(*key).and_then(Value::as_i64))
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
    input: i64,
    cachedInput: i64,
    output: i64,
}

/// Updates partial usage atomically without reacquiring the same non-reentrant mutex.
#[allow(non_snake_case)]
fn applyTokenUsage(counts: &Mutex<ToolPkgProviderTokenCounts>, decoded: &Value) {
    let mut counts = counts.lock().expect("token mutex poisoned");
    if let Some(usage) = extractUsage(decoded, &counts) {
        counts.input = usage.input;
        counts.cachedInput = usage.cachedInput;
        counts.output = usage.output;
    }
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
    let input = read_i64(source, "input").or_else(|| read_i64(source, "inputTokens"));
    let cachedInput =
        read_i64(source, "cachedInput").or_else(|| read_i64(source, "cachedInputTokens"));
    let output = read_i64(source, "output").or_else(|| read_i64(source, "outputTokens"));
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

fn read_i64(object: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
    object.get(key).and_then(Value::as_i64)
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
fn nonBlankOr(value: &str, text: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        text.to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod usage_tests {
    use super::*;

    /// Streaming usage updates preserve omitted counters and release the mutex before text delivery.
    #[test]
    fn partial_usage_updates_release_counters_for_the_next_callback() {
        let counts = Mutex::new(ToolPkgProviderTokenCounts::default());
        applyTokenUsage(&counts, &serde_json::json!({"usage": {"input": 12, "cachedInput": 4}}));
        assert_eq!(counts.try_lock().unwrap().input, 12);
        applyTokenUsage(&counts, &serde_json::json!({"usage": {"output": 3}, "chunk": "hello"}));
        applyTokenUsage(&counts, &serde_json::json!({"chunk": "world"}));
        let counts = counts.try_lock().unwrap();
        assert_eq!((counts.input, counts.cachedInput, counts.output), (12, 4, 3));
    }
}
