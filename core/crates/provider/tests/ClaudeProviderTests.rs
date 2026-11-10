use super::ClaudeProvider;
use crate::chat::llmprovider::MediaLinkBuilder::MediaLinkBuilder;
use crate::chat::llmprovider::ProviderMediaTestSupport::{send_request, TestImage};
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_util::ImagePoolManager::ImagePoolManager;
use serde_json::{json, Value};

/// Creates a Claude provider for content-block conversion tests.
fn test_provider() -> ClaudeProvider {
    ClaudeProvider::new(
        "http://localhost".to_string(),
        String::new(),
        "claude-test".to_string(),
        "CLAUDE".to_string(),
        Vec::new(),
        true,
    )
}

/// Verifies image media links become Claude base64 image blocks.
#[test]
fn imageLinksBecomeClaudeImageBlocks() {
    let image_id = ImagePoolManager::add_image_bytes(
        include_bytes!("fixtures/pixel.png"),
        Some("image/png"),
        None,
    );
    let prompt = format!("look {}", MediaLinkBuilder::image(&image_id));
    let provider = test_provider();

    let content = provider.build_content_array(&prompt);
    let blocks = content.as_array().expect("Claude content must be an array");

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["type"], "image");
    assert_eq!(blocks[0]["source"]["media_type"], "image/png");
    assert!(!blocks[0]["source"]["data"]
        .as_str()
        .unwrap_or_default()
        .is_empty());
    assert_eq!(blocks[1]["text"], "look");
    ImagePoolManager::remove_image(&image_id);
}

fn history_messages(history: Vec<PromptTurn>) -> Vec<Value> {
    test_provider()
        .create_request_body(&send_request(history))
        .unwrap()["messages"]
        .as_array()
        .unwrap()
        .clone()
}

fn proxy_call(name: &str, target: &str) -> String {
    format!(
        r#"<tool name="{name}"><param name="tool_name">{target}</param><param name="params">{{}}</param></tool>"#
    )
}

fn tool_result(name: &str, text: &str) -> String {
    format!(r#"<tool_result name="{name}"><content>{text}</content></tool_result>"#)
}

#[test]
fn proxy_history_preserves_json_objects_and_string_values() {
    for proxy in ["package_proxy", "proxy"] {
        let script = "python3 - <<'PY'\nprint(\"<tag>&\\path\")\nPY\n\n";
        let params = json!({
            "script": script, "enabled": true, "count": 2, "empty": null,
            "nested": {"items": [1, false, null, "0012"]},
        });
        let calls = json!([{"function": {
            "name": proxy,
            "arguments": json!({"tool_name": "files:write", "params": params}).to_string(),
        }}]);
        let markup =
            super::StructuredToolCallBridge::convertToolCallPayloadToXml(&calls.to_string());
        let messages = history_messages(vec![
            PromptTurn::new(PromptTurnKind::ASSISTANT, format!("Opening file\n{markup}")),
            PromptTurn::new(
                PromptTurnKind::TOOL_RESULT,
                tool_result("files:write", "done"),
            ),
        ]);
        let blocks = messages[0]["content"].as_array().unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0]["text"], "Opening file");
        assert_eq!(blocks[1]["name"], proxy);
        assert_eq!(blocks[1]["input"]["tool_name"], "files:write");
        assert_eq!(blocks[1]["input"]["params"], params);
        assert_eq!(messages[1]["content"][0]["tool_use_id"], blocks[1]["id"]);
    }
}

#[test]
fn proxy_results_match_targets_instead_of_result_order() {
    let calls = [
        proxy_call("package_proxy", "character_memory:create_memory"),
        proxy_call("proxy", "character_memory:query_memory"),
        proxy_call("package_proxy", "character_memory:create_memory"),
    ]
    .join("\n");
    let results = [
        tool_result("character_memory:query_memory", "found"),
        tool_result("character_memory:create_memory", "created first"),
        tool_result("character_memory:create_memory", "created second"),
    ]
    .join("\n");
    let messages = history_messages(vec![
        PromptTurn::new(PromptTurnKind::ASSISTANT, calls),
        PromptTurn::new(PromptTurnKind::TOOL_RESULT, results),
    ]);
    let calls = messages[0]["content"].as_array().unwrap();
    let results = messages[1]["content"].as_array().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(results.len(), 3);
    for (result_index, call_index, text) in [
        (0, 1, "found"),
        (1, 0, "created first"),
        (2, 2, "created second"),
    ] {
        assert_eq!(
            results[result_index]["tool_use_id"],
            calls[call_index]["id"]
        );
        assert_eq!(results[result_index]["content"][0]["text"], text);
    }
}

#[test]
fn partial_results_close_missing_calls_without_stealing_later_results() {
    let messages = history_messages(vec![
        PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            [
                proxy_call("package_proxy", "files:write"),
                proxy_call("package_proxy", "files:read"),
            ]
            .join("\n"),
        ),
        PromptTurn::new(
            PromptTurnKind::TOOL_RESULT,
            tool_result("files:read", "read data"),
        ),
        PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            proxy_call("package_proxy", "files:write"),
        ),
        PromptTurn::new(
            PromptTurnKind::TOOL_RESULT,
            tool_result("files:write", "written"),
        ),
    ]);
    let first_calls = &messages[0]["content"];
    let results = messages[1]["content"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["tool_use_id"], first_calls[1]["id"]);
    assert_eq!(results[1]["tool_use_id"], first_calls[0]["id"]);
    assert_eq!(results[1]["is_error"], true);
    assert!(results[1]["content"]
        .as_str()
        .unwrap()
        .contains("files:write"));
    assert_eq!(
        messages[3]["content"][0]["tool_use_id"],
        messages[2]["content"][0]["id"]
    );
    assert_eq!(messages[3]["content"][0]["content"][0]["text"], "written");
}

#[test]
fn missing_results_are_closed_at_history_boundaries() {
    for boundary in [
        None,
        Some(PromptTurnKind::USER),
        Some(PromptTurnKind::SUMMARY),
        Some(PromptTurnKind::SYSTEM),
    ] {
        let mut history = vec![PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            proxy_call("package_proxy", "files:write"),
        )];
        if let Some(kind) = boundary {
            history.push(PromptTurn::new(kind, "next turn"));
        }
        let messages = history_messages(history);
        assert_eq!(messages[1]["role"], "user");
        let result = &messages[1]["content"][0];
        assert_eq!(result["type"], "tool_result");
        assert_eq!(result["tool_use_id"], messages[0]["content"][0]["id"]);
        assert_eq!(result["is_error"], true);
    }
}

#[test]
fn unrelated_results_are_text_and_do_not_receive_fabricated_ids() {
    let orphan = tool_result("files:read", "orphan data");
    let messages = history_messages(vec![
        PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            proxy_call("package_proxy", "files:write"),
        ),
        PromptTurn::new(PromptTurnKind::TOOL_RESULT, orphan.clone()),
    ]);
    let results = messages[1]["content"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["type"], "tool_result");
    assert_eq!(results[0]["is_error"], true);
    assert_eq!(results[1]["type"], "text");
    assert_eq!(results[1]["text"], orphan);
    let standalone = history_messages(vec![PromptTurn::new(
        PromptTurnKind::TOOL_RESULT,
        orphan.clone(),
    )]);
    assert_eq!(standalone[0]["content"][0]["type"], "text");
    assert_eq!(standalone[0]["content"][0]["text"], orphan);
}

#[test]
fn matched_tool_results_keep_images_and_surrounding_text() {
    let image = TestImage::new();
    let messages = history_messages(vec![
        PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            proxy_call("package_proxy", "files:read"),
        ),
        PromptTurn::new(
            PromptTurnKind::TOOL_RESULT,
            format!(
                "context\n{}",
                tool_result("files:read", &format!("image data{}", image.link()))
            ),
        ),
    ]);
    let blocks = messages[1]["content"].as_array().unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["type"], "tool_result");
    assert_eq!(blocks[0]["content"][0]["type"], "image");
    assert_eq!(blocks[0]["content"][1]["text"], "image data");
    assert_eq!(blocks[1]["text"], "context");
}
