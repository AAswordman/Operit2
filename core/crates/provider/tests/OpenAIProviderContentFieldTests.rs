//! Ports the image-role behavior of OpenAIProviderContentFieldTest.kt.

use super::OpenAIProvider;
use crate::chat::llmprovider::ProviderMediaTestSupport::{
    send_request, tool_image_request, TestImage, IMAGE_DATA_URL,
};
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::ToolPrompt;
use serde_json::{json, Value};

fn provider(vision: bool) -> OpenAIProvider {
    OpenAIProvider::new_with_capabilities(
        "https://example.test/v1/chat/completions".to_string(),
        "test-key".to_string(),
        "test-model".to_string(),
        "OPENAI_GENERIC".to_string(),
        Vec::new(),
        vision,
        false,
        false,
        true,
    )
}

#[test]
fn history_images_are_forwarded_as_readable_user_inputs() {
    let image = TestImage::new();
    let body = provider(true)
        .create_request_body(&send_request(vec![
            PromptTurn::new(PromptTurnKind::USER, format!("user text{}", image.link())),
            PromptTurn::new(
                PromptTurnKind::ASSISTANT,
                format!("assistant text{}", image.link()),
            ),
            PromptTurn::new(
                PromptTurnKind::TOOL_RESULT,
                format!("tool text{}", image.link()),
            ),
        ]))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(
        messages[0]["content"][0]["image_url"]["url"],
        IMAGE_DATA_URL
    );
    assert_eq!(messages[0]["content"][1]["text"], "user text");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"], "assistant text");
    assert_eq!(messages[2]["role"], "user");
    assert!(messages[2]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("assistant"));
    assert_eq!(
        messages[2]["content"][1]["image_url"]["url"],
        IMAGE_DATA_URL
    );
    assert_eq!(messages[3]["role"], "user");
    assert_eq!(
        messages[3]["content"][0]["image_url"]["url"],
        IMAGE_DATA_URL
    );
    assert!(!body.to_string().contains("<link"));
}

#[test]
fn structured_tool_images_are_forwarded_as_readable_user_inputs() {
    let image = TestImage::new();
    let body = provider(true)
        .create_request_body(&tool_image_request(&image))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0]["role"], "assistant");
    assert_eq!(messages[1]["role"], "tool");
    assert_eq!(
        messages[1]["tool_call_id"],
        messages[0]["tool_calls"][0]["id"]
    );
    assert_eq!(messages[1]["content"], "tool text");
    assert_eq!(messages[2]["role"], "user");
    assert!(messages[2]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("tool result"));
    assert_eq!(
        messages[2]["content"][1]["image_url"]["url"],
        IMAGE_DATA_URL
    );
    assert!(!body.to_string().contains("<link"));
}

#[test]
fn tool_batch_images_are_deduplicated_after_all_required_results() {
    let image = TestImage::new();
    let mut request = send_request(vec![
        PromptTurn::new(
            PromptTurnKind::TOOL_CALL,
            "<tool name=\"read_a\"></tool><tool name=\"read_b\"></tool>",
        ),
        PromptTurn::new(
            PromptTurnKind::TOOL_RESULT,
            format!(
                "<tool_result name=\"read_a\"><content>a{0}</content></tool_result><tool_result name=\"read_b\"><content>b{0}</content></tool_result>",
                image.link()
            ),
        ),
        PromptTurn::new(PromptTurnKind::USER, "Continue"),
    ]);
    request.available_tools = vec![
        ToolPrompt::new("read_a".to_string(), "Read A".to_string()),
        ToolPrompt::new("read_b".to_string(), "Read B".to_string()),
    ];
    let body = provider(true).create_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 5);
    assert_eq!(messages[0]["tool_calls"].as_array().unwrap().len(), 2);
    assert_eq!(messages[1]["role"], "tool");
    assert_eq!(messages[2]["role"], "tool");
    assert_eq!(messages[3]["role"], "user");
    assert_eq!(messages[3]["content"].as_array().unwrap().len(), 2);
    assert_eq!(
        messages[3]["content"][1]["image_url"]["url"],
        IMAGE_DATA_URL
    );
    assert_eq!(messages[4]["content"], "Continue");
    assert!(!body.to_string().contains("<link"));
}

#[test]
fn disabled_vision_and_system_images_remain_plain_text() {
    let image = TestImage::new();
    let history = vec![
        PromptTurn::new(
            PromptTurnKind::SYSTEM,
            format!("system text{}", image.link()),
        ),
        PromptTurn::new(PromptTurnKind::USER, format!("user text{}", image.link())),
        PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            format!("assistant text{}", image.link()),
        ),
    ];
    let disabled = provider(false)
        .create_request_body(&send_request(history.clone()))
        .unwrap();
    assert_eq!(disabled["messages"].as_array().unwrap().len(), 3);
    assert_eq!(disabled["messages"][0]["content"], "system text");
    assert_eq!(disabled["messages"][1]["content"], "user text");
    assert_eq!(disabled["messages"][2]["content"], "assistant text");
    assert!(!disabled.to_string().contains("image_url"));
    assert!(!disabled.to_string().contains("<link"));
    let enabled = provider(true)
        .create_request_body(&send_request(history))
        .unwrap();
    assert_eq!(enabled["messages"][0]["content"], "system text");
}

#[test]
fn missing_images_and_empty_tool_call_content_keep_valid_message_shapes() {
    let request = send_request(vec![PromptTurn::new(
        PromptTurnKind::USER,
        "before<link type=\"image\" id=\"missing-image\"></link>after",
    )]);
    let body = provider(true).create_request_body(&request).unwrap();
    assert_eq!(body["messages"][0]["content"], "beforeafter");
    assert_eq!(body["messages"].as_array().unwrap().len(), 1);

    let mut messages = json!([{"role": "assistant", "content": null, "tool_calls": []}]);
    provider(true).rewrite_message_media_content(&mut messages);
    assert!(messages[0]["content"].is_null());
}

#[test]
fn assistant_image_forwarding_preserves_reasoning_content() {
    let image = TestImage::new();
    let mut reasoning_provider = provider(true);
    reasoning_provider.preserve_reasoning_content = true;
    let body = reasoning_provider
        .create_request_body(&send_request(vec![PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            format!("<think>reasoning</think>visible{}", image.link()),
        )]))
        .unwrap();
    assert_eq!(body["messages"][0]["reasoning_content"], "reasoning");
    assert_eq!(body["messages"][0]["content"], "visible");
    assert_eq!(
        body["messages"][1]["content"][1]["image_url"]["url"],
        IMAGE_DATA_URL
    );
}

#[test]
fn assistant_image_forwarding_deduplicates_links_within_one_message() {
    let image = TestImage::new();
    let body = provider(true)
        .create_request_body(&send_request(vec![PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            format!("visible{0}{0}", image.link()),
        )]))
        .unwrap();
    let content = body["messages"][1]["content"].as_array().unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[1]["image_url"]["url"], IMAGE_DATA_URL);
    assert!(!body.to_string().contains("<link"));
}

#[test]
fn text_only_messages_are_unchanged() {
    let mut messages = json!([
        {"role": "system", "content": "system"},
        {"role": "assistant", "content": null, "tool_calls": []},
        {"role": "tool", "content": "tool", "tool_call_id": "call_1"},
        {"role": "user", "content": "user"},
    ]);
    let expected: Value = messages.clone();
    provider(true).rewrite_message_media_content(&mut messages);
    assert_eq!(messages, expected);
}

#[test]
fn chat_history_does_not_replay_responses_metadata_as_visible_text() {
    use crate::chat::llmprovider::OpenAIResponsesProvider::OpenAIResponsesPayloadAdapter;

    let image = TestImage::new();
    let metadata = OpenAIResponsesPayloadAdapter::create_reasoning_metadata_tag(&json!({
        "type": "reasoning",
        "id": "rs_old_protocol",
        "encrypted_content": "encrypted-reasoning",
        "summary": []
    }))
    .unwrap();
    let body = provider(true)
        .create_request_body(&send_request(vec![PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            format!("assistant text{}\n{metadata}", image.link()),
        )]))
        .unwrap();
    assert_eq!(body["messages"][0]["content"], "assistant text");
    assert_eq!(
        body["messages"][1]["content"][1]["image_url"]["url"],
        IMAGE_DATA_URL
    );
    assert!(!body.to_string().contains("<meta"));
    assert!(!body.to_string().contains("encrypted-reasoning"));
}

#[test]
fn audio_and_video_links_are_converted_to_openai_multimodal_parts() {
    use crate::chat::llmprovider::MediaLinkBuilder::MediaLinkBuilder;
    use operit_util::MediaPoolManager::MediaPoolManager;

    let audio_id = MediaPoolManager::add_media_bytes(b"audio", "audio/mpeg");
    let video_id = MediaPoolManager::add_media_bytes(b"video", "video/mp4");
    let prompt = format!(
        "{}\n{}\nAnalyze media",
        MediaLinkBuilder::audio(&audio_id),
        MediaLinkBuilder::video(&video_id)
    );
    let multimodal_provider = OpenAIProvider::new_with_capabilities(
        "https://example.test/v1/chat/completions".to_string(),
        "test-key".to_string(),
        "test-model".to_string(),
        "OPENAI_GENERIC".to_string(),
        Vec::new(),
        true,
        true,
        true,
        true,
    );
    let body = multimodal_provider
        .create_request_body(&send_request(vec![PromptTurn::new(
            PromptTurnKind::USER,
            prompt,
        )]))
        .unwrap();

    MediaPoolManager::remove_media(&audio_id);
    MediaPoolManager::remove_media(&video_id);

    let content = body["messages"][0]["content"].as_array().unwrap();
    assert_eq!(content.len(), 3);
    assert_eq!(content[0]["type"], "input_audio");
    assert_eq!(content[0]["input_audio"]["format"], "mp3");
    assert_eq!(content[0]["input_audio"]["data"], "YXVkaW8=");
    assert_eq!(content[1]["type"], "video_url");
    assert_eq!(content[1]["video_url"]["url"], "data:video/mp4;base64,dmlkZW8=");
    assert_eq!(content[2]["type"], "text");
    assert_eq!(content[2]["text"], "Analyze media");
    assert!(!body.to_string().contains("<link"));
}
