//! Regression tests for ClaudeProvider.kt's explicit prompt-cache policy.

use super::{stable_json_value, ClaudeProvider};
use crate::chat::llmprovider::AIService::{AIService, TokenCounts};
use crate::chat::llmprovider::AIServiceFactory::{
    AIServiceFactory, ProviderCreateParams, ProviderCreateRequest,
};
use crate::chat::llmprovider::ProviderMediaTestSupport::{
    send_request, tool_image_request, TestImage,
};
use operit_model::ModelConfigData::{ApiProviderType, ModelRequestSpec, ResolvedModelConfig};
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_model::ToolPrompt::ToolPrompt;
use serde_json::{json, Value};

fn provider(one_hour: bool) -> ClaudeProvider {
    ClaudeProvider::new(
        "https://example.test/v1/messages".into(),
        "test-key".into(),
        "claude-test".into(),
        "ANTHROPIC".into(),
        Vec::new(),
        true,
    )
    .with_claude_1h_prompt_cache(one_hour)
}

fn request() -> crate::chat::llmprovider::AIService::SendMessageRequest {
    let mut request = send_request(vec![
        PromptTurn::new(PromptTurnKind::SYSTEM, "first system"),
        PromptTurn::new(PromptTurnKind::USER, "first user"),
        PromptTurn::new(PromptTurnKind::SYSTEM, "second system"),
        PromptTurn::new(PromptTurnKind::ASSISTANT, "answer"),
        PromptTurn::new(PromptTurnKind::USER, "last user"),
    ]);
    request.available_tools = vec![
        ToolPrompt::new("first".into(), "First tool".into()),
        ToolPrompt::new("last".into(), "Last tool".into()),
    ];
    request
}

fn cache_count(value: &Value) -> usize {
    match value {
        Value::Object(map) => {
            usize::from(map.contains_key("cache_control"))
                + map.values().map(cache_count).sum::<usize>()
        }
        Value::Array(values) => values.iter().map(cache_count).sum(),
        _ => 0,
    }
}

#[test]
fn breakpoints_match_kotlin_tools_system_and_last_message_block() {
    let body = provider(false).create_request_body(&request()).unwrap();
    assert_eq!(cache_count(&body), 3);
    assert!(body["tools"][0].get("cache_control").is_none());
    assert_eq!(
        body["tools"][1]["cache_control"],
        json!({"type":"ephemeral"})
    );
    assert_eq!(body["system"].as_array().unwrap().len(), 1);
    assert_eq!(body["system"][0]["text"], "first system\n\nsecond system");
    assert_eq!(
        body["system"][0]["cache_control"],
        json!({"type":"ephemeral"})
    );
    let messages = body["messages"].as_array().unwrap();
    assert!(messages[0]["content"][0].get("cache_control").is_none());
    assert!(messages[1]["content"][0].get("cache_control").is_none());
    assert_eq!(
        messages[2]["content"][0]["cache_control"],
        json!({"type":"ephemeral"})
    );
}

#[test]
fn one_hour_ttl_is_applied_at_all_three_breakpoints_for_both_stream_modes() {
    for stream in [false, true] {
        let mut request = request();
        request.stream = stream;
        let body = provider(true).create_request_body(&request).unwrap();
        let expected = json!({"type":"ephemeral", "ttl":"1h"});
        assert_eq!(body["tools"][1]["cache_control"], expected);
        assert_eq!(body["system"][0]["cache_control"], expected);
        assert_eq!(body["messages"][2]["content"][0]["cache_control"], expected);
    }
}

#[test]
fn existing_cache_control_is_preserved_and_application_is_idempotent() {
    let mut object = json!({
        "tools":[{"name":"first"},{"name":"last","cache_control":{"type":"ephemeral","ttl":"5m"}}],
        "system":[{"type":"text","text":"system","cache_control":null}],
        "messages":[{"role":"assistant","content":[{"type":"text","text":"answer","cache_control":{"type":"ephemeral","ttl":"5m"}}]}]
    }).as_object().unwrap().clone();
    let expected = object.clone();
    provider(true).apply_stable_cache_breakpoints(&mut object);
    provider(true).apply_stable_cache_breakpoints(&mut object);
    assert_eq!(object, expected);
}

#[test]
fn final_block_search_skips_empty_or_non_object_content_and_is_not_user_specific() {
    let mut object = json!({"messages":[
        {"role":"user","content":[{"type":"text","text":"user"}]},
        {"role":"assistant","content":[{"type":"text","text":"answer"},null,3]},
        {"role":"user","content":[]},
        {"role":"user","content":"not an array"}
    ]})
    .as_object()
    .unwrap()
    .clone();
    provider(false).apply_stable_cache_breakpoints(&mut object);
    assert_eq!(
        object["messages"][1]["content"][0]["cache_control"],
        json!({"type":"ephemeral"})
    );
    assert!(object["messages"][0]["content"][0]
        .get("cache_control")
        .is_none());
}

#[test]
fn tool_result_breakpoint_is_on_the_outer_block_not_nested_image_content() {
    let image = TestImage::new();
    let mut request = tool_image_request(&image);
    request.available_tools[0].description = "Read a file".into();
    let body = provider(false).create_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    let block = &messages.last().unwrap()["content"][0];
    assert_eq!(block["type"], "tool_result");
    assert_eq!(block["cache_control"], json!({"type":"ephemeral"}));
    assert!(block["content"][0].get("cache_control").is_none());
}

#[test]
fn empty_history_adds_no_breakpoints_and_blank_system_is_omitted() {
    let empty = provider(false)
        .create_request_body(&send_request(Vec::new()))
        .unwrap();
    assert_eq!(cache_count(&empty), 0);
    let blank = provider(false)
        .create_request_body(&send_request(vec![PromptTurn::new(
            PromptTurnKind::SYSTEM,
            "   ",
        )]))
        .unwrap();
    assert!(blank.get("system").is_none());
}

#[test]
fn cache_application_does_not_mark_each_system_or_tool_block() {
    let mut object = json!({
        "tools":[{"name":"a"},{"name":"b"}],
        "system":[{"type":"text","text":"a"},{"type":"text","text":"b"}],
        "messages":[]
    })
    .as_object()
    .unwrap()
    .clone();
    provider(false).apply_stable_cache_breakpoints(&mut object);
    assert!(object["tools"][0].get("cache_control").is_none());
    assert!(object["system"][0].get("cache_control").is_none());
    assert_eq!(cache_count(&Value::Object(object)), 2);
}

#[tokio::test]
async fn token_preflight_uses_serialized_schema_and_does_not_mutate_counters() {
    let mut provider = provider(true);
    let mut request = request();
    request.available_tools[0].parameters = json!({"type":"object", "properties":{"long_name":{"type":"string", "description":"full tool schema"}}}).to_string();
    let estimated = provider
        .calculate_input_tokens(&request.chat_history, &request.available_tools)
        .await
        .unwrap();
    assert_eq!(provider.input_token_count(), 0);
    assert_eq!(provider.cached_input_token_count(), 0);
    let body = provider.create_request_body(&request).unwrap();
    assert_eq!(provider.input_token_count(), estimated);
    assert_eq!(
        provider.calculate_and_store_input_tokens(body.as_object().unwrap(), false),
        estimated
    );
    provider.set_token_counts(TokenCounts {
        input: 20,
        cached_input: 100,
        output: 7,
    });
    provider
        .calculate_input_tokens(&request.chat_history, &request.available_tools)
        .await
        .unwrap();
    assert_eq!(provider.input_token_count(), 120);
    assert_eq!(provider.cached_input_token_count(), 100);
    assert_eq!(provider.output_token_count(), 7);
    provider.reset_token_counts();
    assert_eq!(provider.input_token_count(), 0);
    assert_eq!(provider.output_token_count(), 0);
}

#[test]
fn stable_json_sorts_nested_object_keys_but_preserves_array_order() {
    assert_eq!(
        stable_json_value(&json!({"z":[{"b":2,"a":1}],"a":"first"})),
        "{\"a\":\"first\",\"z\":[{\"a\":1,\"b\":2}]}"
    );
}

#[test]
fn old_request_settings_default_to_standard_cache_ttl_and_new_setting_round_trips() {
    let old: ModelRequestSpec =
        serde_json::from_value(json!({"supportsStructuredTools":true})).unwrap();
    assert!(!old.enableClaude1hPromptCache);
    let mut one_hour = old;
    one_hour.enableClaude1hPromptCache = true;
    let decoded: ModelRequestSpec =
        serde_json::from_value(serde_json::to_value(&one_hour).unwrap()).unwrap();
    assert!(decoded.enableClaude1hPromptCache);
}

#[test]
fn factory_forwards_one_hour_setting_for_official_and_generic_claude() {
    for provider_type in [
        ApiProviderType::ANTHROPIC,
        ApiProviderType::ANTHROPIC_GENERIC,
    ] {
        let config = ResolvedModelConfig {
            providerId: "test".into(),
            providerName: "Test".into(),
            modelId: "claude-test".into(),
            apiKey: "test-key".into(),
            apiEndpoint: "https://example.test/v1/messages".into(),
            apiProviderType: provider_type.clone(),
            apiProviderTypeId: provider_type.name().into(),
            useMultipleApiKeys: false,
            apiKeyPool: Vec::new(),
            currentKeyIndex: 0,
            keyRotationMode: "ROUND_ROBIN".into(),
            customHeaders: "{}".into(),
            requestLimitPerMinute: 0,
            maxConcurrentRequests: 0,
            pricing: None,
            context: Default::default(),
            capabilities: Default::default(),
            builtinTools: Vec::new(),
            request: ModelRequestSpec {
                supportsStructuredTools: true,
                enableClaude1hPromptCache: true,
            },
            parameters: Vec::new(),
            thinking: Default::default(),
            thinkingLevel: None,
            summary: Default::default(),
            localRuntime: Default::default(),
        };
        let spec = AIServiceFactory::create_service(ProviderCreateRequest {
            config,
            provider_type: provider_type.clone(),
            provider_type_id: provider_type.name().into(),
            tool_pkg_provider_registered: false,
        })
        .unwrap();
        assert!(matches!(
            spec.params,
            ProviderCreateParams::ClaudeProvider {
                enable_claude_1h_prompt_cache: true,
                ..
            }
        ));
    }
}

#[test]
fn anthropic_usage_includes_cache_creation_and_reads_in_total_input() {
    let mut provider = provider(false);
    let usage = provider.apply_usage(Some(&json!({
        "input_tokens": 20,
        "cache_read_input_tokens": 100,
        "cache_creation": {"ephemeral_5m_input_tokens": 30, "ephemeral_1h_input_tokens": 40},
        "output_tokens": 7
    })));
    assert_eq!(
        usage,
        TokenCounts {
            input: 90,
            cached_input: 100,
            output: 7
        }
    );
    assert_eq!(provider.input_token_count(), 190);
    assert_eq!(provider.cached_input_token_count(), 100);
    assert_eq!(provider.output_token_count(), 7);
}

#[test]
fn streaming_output_only_usage_preserves_input_and_cache_counts() {
    let mut provider = provider(false);
    let mut accumulated_usage = serde_json::Map::new();
    let mut chunks = Vec::new();
    let mut parser = None;
    let mut tag = None;
    let mut in_tool = false;
    let mut in_thinking = false;
    let mut completed = false;
    for event in [
        json!({"type":"message_start", "message":{"usage":{
            "input_tokens":20, "cache_read_input_tokens":100,
            "cache_creation_input_tokens":30, "output_tokens":0
        }}}),
        json!({"type":"message_delta", "usage":{"output_tokens":7}}),
        json!({"type":"message_stop"}),
    ] {
        provider
            .process_streaming_event(
                &event.to_string(),
                &mut chunks,
                &mut accumulated_usage,
                &mut parser,
                &mut tag,
                &mut in_tool,
                &mut in_thinking,
                &mut completed,
            )
            .unwrap();
    }
    assert_eq!(provider.input_token_count(), 150);
    assert_eq!(provider.cached_input_token_count(), 100);
    assert_eq!(provider.output_token_count(), 7);
    provider.apply_streaming_usage(&json!({"output_tokens":0}), &mut accumulated_usage);
    assert_eq!(provider.output_token_count(), 0);
    assert_eq!(provider.input_token_count(), 150);
}

#[test]
fn openai_compatible_usage_does_not_double_count_cached_prompt_tokens() {
    let mut provider = provider(false);
    let usage = provider.apply_usage(Some(&json!({
        "prompt_tokens":120, "prompt_tokens_details":{"cached_tokens":100},
        "completion_tokens":7
    })));
    assert_eq!(
        usage,
        TokenCounts {
            input: 20,
            cached_input: 100,
            output: 7
        }
    );
    assert_eq!(provider.input_token_count(), 120);
    assert_eq!(provider.cached_input_token_count(), 100);
}

#[test]
fn absent_or_unrecognized_usage_does_not_erase_estimates_or_measured_usage() {
    let mut provider = provider(false);
    provider.create_request_body(&request()).unwrap();
    let estimate = provider.input_token_count();
    assert!(estimate > 0);
    provider.apply_usage(None);
    provider.apply_usage(Some(&json!({})));
    assert_eq!(provider.input_token_count(), estimate);
    provider.apply_usage(Some(
        &json!({"input_tokens":20, "cache_read_input_tokens":100, "output_tokens":7}),
    ));
    provider.apply_usage(Some(&json!({"unrelated":42})));
    assert_eq!(provider.input_token_count(), 120);
    assert_eq!(provider.cached_input_token_count(), 100);
    assert_eq!(provider.output_token_count(), 7);
}

#[test]
fn compatible_streaming_usage_input_alias_is_total_not_native_anthropic_input() {
    let mut provider = provider(false);
    let mut accumulated_usage = serde_json::Map::new();
    let mut chunks = Vec::new();
    let mut parser = None;
    let mut tag = None;
    let mut in_tool = false;
    let mut in_thinking = false;
    let mut completed = false;
    for event in [
        json!({"choices":[], "usage":{
            "input_tokens":120, "input_tokens_details":{"cached_tokens":100}, "output_tokens":0
        }}),
        json!({"choices":[], "usage":{"output_tokens":7}}),
    ] {
        provider
            .process_streaming_event(
                &event.to_string(),
                &mut chunks,
                &mut accumulated_usage,
                &mut parser,
                &mut tag,
                &mut in_tool,
                &mut in_thinking,
                &mut completed,
            )
            .unwrap();
    }
    assert_eq!(provider.input_token_count(), 120);
    assert_eq!(provider.cached_input_token_count(), 100);
    assert_eq!(provider.output_token_count(), 7);
}
