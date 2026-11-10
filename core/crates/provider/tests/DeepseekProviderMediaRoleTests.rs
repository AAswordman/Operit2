//! Ports the DeepSeek cases from DeepseekProviderMediaRoleTest.kt.

use super::DeepseekProvider;
use crate::chat::llmprovider::ProviderMediaTestSupport::{
    send_request, test_runtime_context, tool_image_request, TestImage,
};
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};

fn provider(responses: bool, vision: bool) -> DeepseekProvider {
    DeepseekProvider::new(
        format!(
            "https://example.test/v1/{}",
            if responses {
                "responses"
            } else {
                "chat/completions"
            }
        ),
        "test-key".to_string(),
        "deepseek-flash".to_string(),
        "DEEPSEEK".to_string(),
        Vec::new(),
        vision,
        false,
        false,
        Vec::new(),
        true,
        test_runtime_context(),
    )
}

#[test]
fn chat_completions_history_images_are_readable_user_inputs() {
    let image = TestImage::new();
    let body = provider(false, true)
        .create_request_body(&send_request(vec![
            PromptTurn::new(PromptTurnKind::USER, format!("user text{}", image.link())),
            PromptTurn::new(
                PromptTurnKind::ASSISTANT,
                format!("<think>reasoning</think>assistant text{}", image.link()),
            ),
            PromptTurn::new(
                PromptTurnKind::TOOL_RESULT,
                format!("tool text{}", image.link()),
            ),
        ]))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0]["content"][0]["type"], "image_url");
    assert_eq!(
        messages[0]["content"][0]["image_url"]["url"],
        image.data_url()
    );
    assert_eq!(messages[0]["content"][1]["text"], "user text");
    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"], "assistant text");
    assert_eq!(messages[1]["reasoning_content"], "reasoning");
    assert_eq!(messages[2]["role"], "user");
    assert!(messages[2]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("assistant"));
    assert_eq!(
        messages[2]["content"][1]["image_url"]["url"],
        image.data_url()
    );
    assert_eq!(messages[3]["role"], "user");
    assert_eq!(
        messages[3]["content"][0]["image_url"]["url"],
        image.data_url()
    );
    assert_eq!(messages[3]["content"][1]["text"], "tool text");
    assert!(!body.to_string().contains("<link"));
}

#[test]
fn chat_completions_structured_tool_images_are_readable_user_inputs() {
    let image = TestImage::new();
    let body = provider(false, true)
        .create_request_body(&tool_image_request(&image))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0]["role"], "assistant");
    assert_eq!(messages[0]["reasoning_content"], "");
    assert!(messages[0]["content"].is_null());
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
        image.data_url()
    );
    assert!(!body.to_string().contains("<link"));
}

#[test]
fn deepseek_user_image_encoding_is_identical_for_streaming_and_non_streaming() {
    let image = TestImage::new();
    for responses in [false, true] {
        let mut request = send_request(vec![PromptTurn::new(
            PromptTurnKind::USER,
            format!("look{}", image.link()),
        )]);
        let non_streaming = provider(responses, true)
            .create_request_body(&request)
            .unwrap();
        request.stream = true;
        let streaming = provider(responses, true)
            .create_request_body(&request)
            .unwrap();
        let content_key = if responses { "input" } else { "messages" };
        assert_eq!(streaming[content_key], non_streaming[content_key]);
        let content = &streaming[content_key][0]["content"];
        if responses {
            assert_eq!(content[0]["type"], "input_image");
            assert_eq!(content[0]["image_url"], image.data_url());
        } else {
            assert_eq!(content[0]["type"], "image_url");
            assert_eq!(content[0]["image_url"]["url"], image.data_url());
        }
        assert!(!streaming.to_string().contains("<link"));
    }
}

#[test]
fn deepseek_responses_tool_images_stay_in_function_output() {
    let image = TestImage::new();
    let body = provider(true, true)
        .create_request_body(&tool_image_request(&image))
        .unwrap();
    let input = body["input"].as_array().unwrap();
    assert_eq!(input.len(), 2);
    assert_eq!(input[0]["type"], "function_call");
    assert_eq!(input[1]["type"], "function_call_output");
    assert_eq!(input[1]["call_id"], input[0]["call_id"]);
    assert_eq!(input[1]["output"][0]["type"], "input_image");
    assert_eq!(input[1]["output"][0]["image_url"], image.data_url());
    assert_eq!(input[1]["output"][1]["type"], "input_text");
    assert_eq!(input[1]["output"][1]["text"], "tool text");
    assert!(!body.to_string().contains("<link"));
}

#[test]
fn responses_assistant_images_are_forwarded_as_user_input_images() {
    let image = TestImage::new();
    let body = provider(true, true)
        .create_request_body(&send_request(vec![PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            format!("assistant text{}", image.link()),
        )]))
        .unwrap();
    let input = body["input"].as_array().unwrap();
    assert_eq!(input.len(), 2);
    assert_eq!(input[0]["role"], "assistant");
    assert_eq!(input[0]["content"], "assistant text");
    assert_eq!(input[1]["role"], "user");
    assert_eq!(input[1]["content"][1]["type"], "input_image");
    assert_eq!(input[1]["content"][1]["image_url"], image.data_url());
    assert!(!body.to_string().contains("<link"));
}

#[test]
fn deepseek_vision_is_opt_in_and_never_sends_system_images() {
    let image = TestImage::new();
    for responses in [false, true] {
        let history = vec![
            PromptTurn::new(
                PromptTurnKind::SYSTEM,
                format!("system text{}", image.link()),
            ),
            PromptTurn::new(PromptTurnKind::USER, format!("user text{}", image.link())),
        ];
        let disabled = provider(responses, false)
            .create_request_body(&send_request(history.clone()))
            .unwrap();
        let key = if responses { "input" } else { "messages" };
        assert_eq!(disabled[key].as_array().unwrap().len(), 2);
        assert_eq!(disabled[key][0]["content"], "system text");
        assert_eq!(disabled[key][1]["content"], "user text");
        assert!(!disabled.to_string().contains("image_url"));
        assert!(!disabled.to_string().contains("<link"));
        let enabled = provider(responses, true)
            .create_request_body(&send_request(history))
            .unwrap();
        assert_eq!(enabled[key][0]["content"], "system text");
    }
}
