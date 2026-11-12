use super::ModelConfigConnectionTester;
use crate::chat::llmprovider::AIService::{response_stream_from_chunks, AiServiceError, AIService, SendMessageRequest};
use operit_util::stream::RevisableTextStream::{
    empty_revisable_event_channel, with_event_channel_and_failure, TerminalFailureState,
};
use operit_util::stream::Stream::FnStream;
use operit_model::ModelConfigData::{ApiProviderType, ModelCapabilities};
use operit_model::ModelParameter::{ModelParameter, ParameterValueType};
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Records actual probe requests and rejects a response mode the gateway does not support.
struct ModeCheckingService {
    expected_stream: bool,
    requests: Arc<Mutex<Vec<bool>>>,
}

#[async_trait::async_trait]
impl AIService for ModeCheckingService {
    /// Models a gateway that accepts only the selected streaming or buffered response mode.
    async fn send_message(
        &mut self,
        request: SendMessageRequest,
    ) -> Result<Box<dyn operit_util::stream::RevisableTextStream::RevisableTextStreamLike>, AiServiceError> {
        self.requests.lock().unwrap().push(request.stream);
        if request.stream != self.expected_stream {
            return Err(AiServiceError::RequestFailed("HTTP 503: unsupported response mode".into()));
        }
        Ok(response_stream_from_chunks(vec!["OK".into()]))
    }
}

/// Every capability probe must use the selected mode, including gateways that require streaming.
#[tokio::test]
async fn connection_probe_honors_selected_mode_for_all_capabilities() {
    let capabilities = ModelCapabilities {
        directImage: true,
        directAudio: true,
        directVideo: true,
        toolCall: true,
    };
    for stream in [true, false] {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let service = Box::new(ModeCheckingService {
            expected_stream: stream,
            requests: requests.clone(),
        });
        let items = ModelConfigConnectionTester::testService(
            service,
            &capabilities,
            ModelConfigConnectionTester::testParameters(&ApiProviderType::OPENAI, Vec::new()),
            stream,
        ).await;
        assert_eq!(items.len(), 5);
        assert!(items.iter().all(|item| item.success), "probe failures: {items:?}");
        assert_eq!(*requests.lock().unwrap(), vec![stream; 5]);
    }
}

#[test]
fn connection_probe_sets_one_output_limit_for_each_protocol() {
    for (provider, field) in [
        (ApiProviderType::OPENAI, "max_completion_tokens"),
        (ApiProviderType::OPENAI_RESPONSES, "max_output_tokens"),
        (ApiProviderType::OPENAI_RESPONSES_GENERIC, "max_output_tokens"),
        (ApiProviderType::DEEPSEEK, "max_tokens"),
        (ApiProviderType::GOOGLE, "max_tokens"),
        (ApiProviderType::ANTHROPIC, "max_tokens"),
    ] {
        let parameters = ModelConfigConnectionTester::testParameters(&provider, Vec::new());
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].apiName, field);
        assert_eq!(parameters[0].currentValue, json!(1024));
        assert!(parameters[0].isEnabled);
    }
}

#[test]
fn connection_probe_replaces_large_limits_without_changing_model_parameters() {
    let mut configured = Vec::new();
    for field in ["max_tokens", "max_output_tokens", "max_completion_tokens", "maxOutputTokens"] {
        configured.push(ModelParameter::new(
            field.to_string(),
            field.to_string(),
            field.to_string(),
            json!(943716),
            json!(943716),
            true,
            ParameterValueType::INT,
        ));
    }
    let temperature = ModelParameter::new(
        "temperature".to_string(),
        "Temperature".to_string(),
        "temperature".to_string(),
        json!(1.0),
        json!(0.2),
        true,
        ParameterValueType::FLOAT,
    );
    configured.push(temperature.clone());
    let parameters = ModelConfigConnectionTester::testParameters(
        &ApiProviderType::OPENAI_RESPONSES,
        configured.clone(),
    );
    assert_eq!(parameters.len(), 2);
    assert_eq!(parameters[0], temperature);
    assert_eq!(parameters[1].apiName, "max_output_tokens");
    assert_eq!(parameters[1].currentValue, json!(1024));
    assert_eq!(configured.len(), 5);
    assert_eq!(configured[0].currentValue, json!(943716));
}

#[test]
fn responses_connection_probe_serializes_its_output_budget() {
    use crate::chat::llmprovider::AIService::SendMessageRequest;
    use crate::chat::llmprovider::OpenAIProvider::OpenAIProvider;
    use crate::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesPayloadAdapter;
    use crate::chat::llmprovider::ThinkingDialect::RequestThinking;
    use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};

    let provider = OpenAIProvider::new(
        "https://example.test/v1".to_string(),
        String::new(),
        "test-model".to_string(),
        "OPENAI".to_string(),
        Vec::new(),
        false,
    );
    let request = SendMessageRequest {
        chat_history: vec![PromptTurn::new(PromptTurnKind::USER, "Hi")],
        model_parameters: ModelConfigConnectionTester::testParameters(
            &ApiProviderType::OPENAI_RESPONSES,
            Vec::new(),
        ),
        thinking: RequestThinking::default(),
        stream: false,
        available_tools: Vec::new(),
        preserve_think_in_history: false,
        enable_retry: false,
        on_non_fatal_error: None,
        on_tool_invocation: None,
    };
    let body = OpenAIResponsesPayloadAdapter::to_responses_request(
        provider.create_request_body(&request).unwrap(),
    );
    assert_eq!(body["max_output_tokens"], json!(1024));
    assert!(body.get("max_tokens").is_none());
    assert!(body.get("max_completion_tokens").is_none());
}

/// Rejects deferred producer failures even when partial text arrived without an error callback.
#[tokio::test]
async fn connection_probe_rejects_failure_reported_during_collection() {
    for chunks in [Vec::new(), vec!["unfinished answer".to_string()]] {
        let failure = TerminalFailureState::new();
        let producer_failure = failure.clone();
        let event_channel = empty_revisable_event_channel();
        let producer_events = event_channel.clone();
        let error = AiServiceError::ConnectionFailed("transport closed".to_string());
        let producer_error = error.to_string();
        let stream = FnStream::new(move |emit| {
            let chunks = chunks.clone();
            let failure = producer_failure.clone();
            let events = producer_events.clone();
            let error = producer_error.clone();
            Box::pin(async move {
                for chunk in chunks {
                    emit(chunk);
                }
                failure.set(error);
                events.close();
            })
        });
        let response = Box::new(with_event_channel_and_failure(
            stream,
            event_channel,
            failure,
        ));

        assert_eq!(
            ModelConfigConnectionTester::verifyTestResponse(response).await,
            Err(AiServiceError::RequestFailed(error.to_string()).to_string()),
        );
    }
}

/// Accepts successful completion for both empty responses and responses with text.
#[tokio::test]
async fn connection_probe_accepts_successful_completion() {
    for chunks in [Vec::new(), vec!["answer".to_string()]] {
        assert_eq!(
            ModelConfigConnectionTester::verifyTestResponse(response_stream_from_chunks(chunks))
                .await,
            Ok(()),
        );
    }
}
