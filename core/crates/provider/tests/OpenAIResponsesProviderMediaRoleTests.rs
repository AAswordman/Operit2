//! Ports the shared Responses image-output case from DeepseekProviderMediaRoleTest.kt.

use super::OpenAIResponsesProvider;
use crate::chat::llmprovider::ThinkingDialect::RequestThinking;
use operit_model::ModelThinking::ModelThinkingSpec;
use crate::chat::llmprovider::ProviderMediaTestSupport::{
    test_runtime_context, tool_image_request, TestImage, IMAGE_DATA_URL,
};

#[test]
fn shared_responses_wrappers_keep_tool_images_in_function_output() {
    let image = TestImage::new();
    for provider_type in [
        "OPENAI_RESPONSES",
        "OPENAI_RESPONSES_GENERIC",
        "OPENAI_CODEX",
    ] {
        let responses_provider = OpenAIResponsesProvider::new(
            "https://example.test/v1/responses".to_string(),
            "test-key".to_string(),
            "test-model".to_string(),
            provider_type.to_string(),
            Vec::new(),
            true,
            false,
            false,
            Vec::new(),
            true,
            test_runtime_context(),
        );
        let body = responses_provider
            .create_request_body(&tool_image_request(&image))
            .unwrap();
        let input = body["input"].as_array().unwrap();
        let output = input
            .iter()
            .find(|item| item["type"] == "function_call_output")
            .unwrap();
        assert_eq!(
            output["output"][0]["type"], "input_image",
            "{provider_type}"
        );
        assert_eq!(
            output["output"][0]["image_url"], IMAGE_DATA_URL,
            "{provider_type}"
        );
        assert_eq!(output["output"][1]["text"], "tool text", "{provider_type}");
        assert!(
            !input.iter().any(|item| item["role"] == "user"),
            "tool images must not become extra user turns in {provider_type}"
        );
        assert!(!body.to_string().contains("<link"));
    }
}

#[test]
fn shared_responses_history_images_and_encrypted_reasoning_are_preserved() {
    use crate::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesPayloadAdapter;
    use crate::chat::llmprovider::ProviderMediaTestSupport::send_request;
    use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
    use serde_json::json;

    let image = TestImage::new();
    let metadata = OpenAIResponsesPayloadAdapter::create_reasoning_metadata_tag(&json!({
        "type": "reasoning",
        "id": "rs_image_history",
        "encrypted_content": "encrypted-reasoning",
        "summary": []
    }))
    .unwrap();
    let mut request = send_request(vec![
        PromptTurn::new(PromptTurnKind::USER, format!("user text{}", image.link())),
        PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            format!("assistant text{}\n{metadata}", image.link()),
        ),
    ]);
    request.thinking = RequestThinking {
        spec: ModelThinkingSpec {
            supported: true,
            ..ModelThinkingSpec::default()
        },
        level: None,
    };
    let provider = OpenAIResponsesProvider::new(
        "https://example.test/v1/responses".to_string(),
        "test-key".to_string(),
        "test-model".to_string(),
        "OPENAI_RESPONSES_GENERIC".to_string(),
        Vec::new(),
        true,
        false,
        false,
        Vec::new(),
        true,
        test_runtime_context(),
    );
    let body = provider.create_request_body(&request).unwrap();
    let input = body["input"].as_array().unwrap();
    let reasoning = input
        .iter()
        .find(|item| item["type"] == "reasoning")
        .unwrap();
    assert_eq!(reasoning["id"], "rs_image_history");
    assert_eq!(reasoning["encrypted_content"], "encrypted-reasoning");
    let users: Vec<_> = input.iter().filter(|item| item["role"] == "user").collect();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0]["content"][0]["image_url"], IMAGE_DATA_URL);
    assert_eq!(users[1]["content"][1]["image_url"], IMAGE_DATA_URL);
    let assistant = input
        .iter()
        .find(|item| item["role"] == "assistant")
        .unwrap();
    assert_eq!(assistant["content"], "assistant text");
    assert!(!body.to_string().contains("<link"));
    assert!(!body.to_string().contains("<meta"));
}
