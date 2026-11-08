use serde_json::{json, Value};

use super::DeepseekProvider;
use super::DeepseekResponsesPayloadAdapter;
use crate::chat::llmprovider::AIService::SendMessageRequest;
use crate::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesPayloadAdapter;
use crate::chat::llmprovider::ThinkingDialect::RequestThinking;
use operit_model::ModelThinking::ModelThinkingSpec;
use crate::chat::llmprovider::ProviderMediaTestSupport::test_runtime_context;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::ToolPrompt;

/// Exposes DeepSeek's transport counters when its wrapper has not completed collection.
#[test]
fn paused_deepseek_reads_active_parent_usage() {
    use crate::chat::llmprovider::AIService::{AIService, TokenCounts};
    let mut provider = DeepseekProvider::new(
        "https://fixture.invalid/responses".into(), String::new(), "test".into(),
        "DEEPSEEK".into(), Vec::new(), false, false, false, Vec::new(), true,
        test_runtime_context(),
    );
    let parent = provider.open_ai_parent(true);
    parent.apply_token_counts(TokenCounts { input: 100, cached_input: 20, output: 17 });
    let generation = provider.setActiveParent(parent);
    provider.cancel_streaming();
    assert_eq!((provider.input_token_count(), provider.cached_input_token_count(), provider.output_token_count()), (120, 20, 17));
    provider.reset_token_counts();
    let newer = provider.open_ai_parent(true);
    newer.apply_token_counts(TokenCounts { input: 200, cached_input: 40, output: 25 });
    provider.setActiveParent(newer);
    provider.finishActiveParent(generation, TokenCounts { input: 100, cached_input: 20, output: 17 });
    assert_eq!((provider.input_token_count(), provider.cached_input_token_count(), provider.output_token_count()), (240, 40, 25));
}

/// Builds a request carrying one assistant tool call and its result.
fn tool_continuation_request(reasoning_metadata: &str) -> SendMessageRequest {
    let assistant_content = format!(
        "<think>Inspect the workspace first.</think>visible\n{reasoning_metadata}\n<tool name=\"list_files\" call_id=\"call_1\"><param name=\"path\">/workspace</param></tool>"
    );
    SendMessageRequest {
        chat_history: vec![
            PromptTurn::new(PromptTurnKind::USER, "List the workspace files."),
            PromptTurn::new(PromptTurnKind::ASSISTANT, assistant_content),
            PromptTurn::new(
                PromptTurnKind::TOOL_RESULT,
                "<tool_result name=\"list_files\"><content>workspace result</content></tool_result>",
            ),
            PromptTurn::new(PromptTurnKind::USER, "Continue."),
        ],
        model_parameters: Vec::new(),
        thinking: RequestThinking {
            spec: ModelThinkingSpec {
                supported: true,
                ..ModelThinkingSpec::default()
            },
            level: None,
        },
        stream: false,
        available_tools: vec![ToolPrompt::new(
            "list_files".to_string(),
            "Lists workspace files".to_string(),
        )],
        preserve_think_in_history: true,
        enable_retry: false,
        on_non_fatal_error: None,
        on_tool_invocation: None,
    }
}

/// Builds the assistant/tool continuation used by Responses replay tests.
fn continuation_request(assistant_content: String, call_id: &str) -> Value {
    json!({
        "messages": [
            {
                "role": "assistant",
                "content": assistant_content,
                "tool_calls": [{
                    "id": call_id,
                    "type": "function",
                    "function": {
                        "name": "list_files",
                        "arguments": "{\"path\":\"/workspace\"}"
                    }
                }]
            },
            {
                "role": "tool",
                "tool_call_id": call_id,
                "content": "workspace result"
            }
        ]
    })
}

/// Verifies plaintext reasoning is replayed before the function call and result.
#[test]
fn plaintext_reasoning_replays_before_function_call() {
    let reasoning_item = json!({
        "type": "reasoning",
        "id": "rs_plain_1",
        "content": [{
            "type": "reasoning_text",
            "text": "Inspect the workspace first."
        }]
    });
    let metadata = DeepseekResponsesPayloadAdapter::create_reasoning_metadata_tag(&reasoning_item)
        .expect("reasoning metadata");
    let request = continuation_request(
        format!("<think>Inspect the workspace first.</think>visible{metadata}"),
        "call_plain_1",
    );

    let input = DeepseekResponsesPayloadAdapter::to_responses_request(request)["input"]
        .as_array()
        .expect("Responses input array")
        .clone();
    assert_eq!(input[0]["type"], "reasoning");
    assert_eq!(input[0]["id"], "rs_plain_1");
    assert_eq!(input[0]["content"][0]["type"], "reasoning_text");
    assert_eq!(input[1]["type"], "message");
    assert_eq!(input[1]["content"], "visible");
    assert_eq!(input[2]["type"], "function_call");
    assert_eq!(input[2]["call_id"], "call_plain_1");
    assert_eq!(input[3]["type"], "function_call_output");
    assert_eq!(input[3]["call_id"], "call_plain_1");
}

/// Verifies encrypted reasoning is not sent through DeepSeek plaintext replay.
#[test]
fn encrypted_reasoning_is_not_replayed_as_plaintext() {
    let parsed = DeepseekResponsesPayloadAdapter::parse_non_streaming_response(&json!({
        "output": [{
            "type": "reasoning",
            "id": "rs_encrypted_1",
            "encrypted_content": "encrypted"
        }]
    }));
    assert!(parsed.reasoningMetadataTags.is_empty());
}

/// Verifies commentary continuation is encoded as DeepSeek reasoning text.
#[test]
fn commentary_replays_as_reasoning_text() {
    let commentary_item = json!({
        "type": "message",
        "id": "msg_commentary_1",
        "role": "assistant",
        "phase": "commentary",
        "content": [{
            "type": "output_text",
            "text": "Activate the package before calling its tool."
        }]
    });
    let parsed = DeepseekResponsesPayloadAdapter::parse_non_streaming_response(&json!({
        "output": [commentary_item]
    }));
    assert!(parsed.reasoningChunks.is_empty());
    let metadata = parsed
        .outputItemMetadataTags
        .first()
        .expect("commentary metadata")
        .clone();
    let input = DeepseekResponsesPayloadAdapter::to_responses_request(continuation_request(
        metadata,
        "call_commentary_1",
    ))["input"]
        .as_array()
        .expect("Responses input array")
        .clone();
    assert_eq!(input[0]["type"], "reasoning");
    assert_eq!(input[0]["content"][0]["type"], "reasoning_text");
    assert_eq!(
        input[0]["content"][0]["text"],
        "Activate the package before calling its tool."
    );
    assert_eq!(input[1]["type"], "function_call");
    assert_eq!(input[2]["type"], "function_call_output");
}

/// Verifies web-search metadata does not remove an unrelated thinking block.
#[test]
fn web_search_metadata_does_not_remove_thinking_content() {
    let search_metadata = OpenAIResponsesPayloadAdapter::create_output_item_metadata_tag(
        &json!({"type": "web_search_call", "id": "search_1"}),
    )
    .expect("search metadata");
    let request = continuation_request(
        format!("<think>raw thinking</think>visible{search_metadata}"),
        "call_search_1",
    );
    let input = DeepseekResponsesPayloadAdapter::to_responses_request(request)["input"]
        .as_array()
        .expect("Responses input array")
        .clone();
    let message = input
        .iter()
        .find(|item| item["type"] == "message")
        .expect("assistant message");
    assert_eq!(message["content"], "<think>raw thinking</think>visible");
    assert!(input.iter().any(|item| item["type"] == "function_call"));
    assert!(input
        .iter()
        .any(|item| item["type"] == "function_call_output"));
}

/// Verifies the full DeepSeek request builder preserves replay order after tool bridging.
#[test]
fn request_builder_preserves_reasoning_tool_and_result_order() {
    let reasoning_item = json!({
        "type": "reasoning",
        "id": "rs_request_1",
        "content": [{
            "type": "reasoning_text",
            "text": "Inspect the workspace first."
        }]
    });
    let reasoning_metadata =
        DeepseekResponsesPayloadAdapter::create_reasoning_metadata_tag(&reasoning_item)
            .expect("reasoning metadata");
    let provider = DeepseekProvider::new(
        "https://api.deepseek.com/v1/responses".to_string(),
        String::new(),
        "deepseek-reasoner".to_string(),
        "DEEPSEEK".to_string(),
        Vec::new(),
        false,
        false,
        false,
        Vec::new(),
        true,
        test_runtime_context(),
    );

    let request = tool_continuation_request(&reasoning_metadata);
    let body = provider
        .create_request_body(&request)
        .expect("DeepSeek Responses request must be buildable");
    let input = body["input"].as_array().expect("Responses input array");

    let reasoning_index = input
        .iter()
        .position(|item| item["type"] == "reasoning")
        .expect("reasoning item must be replayed");
    let function_call_index = input
        .iter()
        .position(|item| item["type"] == "function_call")
        .expect("function call must be bridged");
    let function_output_index = input
        .iter()
        .position(|item| item["type"] == "function_call_output")
        .expect("function result must be bridged");
    assert!(reasoning_index < function_call_index);
    assert!(function_call_index < function_output_index);
    assert_eq!(input[reasoning_index]["id"], "rs_request_1");
    assert_eq!(input[function_call_index]["name"], "list_files");
    assert_eq!(input[function_output_index]["output"], "workspace result");
}
