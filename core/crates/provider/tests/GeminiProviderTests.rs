use super::GeminiProvider;
use crate::chat::llmprovider::MediaLinkBuilder::MediaLinkBuilder;
use operit_model::ModelConfigData::ModelBuiltinTool;
use operit_model::PromptTurn::{PromptTurn, PromptTurnKind};
use operit_util::ImagePoolManager::ImagePoolManager;

/// Creates a Gemini provider for content-part conversion tests.
fn test_provider() -> GeminiProvider {
    GeminiProvider::new(
        "http://localhost".to_string(),
        String::new(),
        "gemini-test".to_string(),
        "GEMINI".to_string(),
        Vec::new(),
        Vec::<ModelBuiltinTool>::new(),
        true,
    )
}

/// Verifies image media links become Gemini inline_data parts.
#[test]
fn imageLinksBecomeGeminiInlineDataParts() {
    let image_id = ImagePoolManager::add_image_bytes(
        b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01",
        Some("image/png"),
        None,
    );
    let prompt = format!("look {}", MediaLinkBuilder::image(&image_id));
    let provider = test_provider();

    let parts = provider.build_parts_array(&prompt);

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0]["inline_data"]["mime_type"], "image/png");
    assert!(!parts[0]["inline_data"]["data"]
        .as_str()
        .unwrap_or_default()
        .is_empty());
    assert_eq!(parts[1]["text"], "look");
    ImagePoolManager::remove_image(&image_id);
}

/// Verifies Gemini pairs package-proxy calls with the proxied tool result name.
#[test]
fn packageProxyResultMatchesGeminiFunctionCall() {
    let provider = test_provider();
    let history = vec![
        PromptTurn::new(
            PromptTurnKind::ASSISTANT,
            [
                r#"<tool name="package_proxy">"#,
                r#"<param name="tool_name">daily_life:get_current_date</param>"#,
                r#"<param name="params">{}</param>"#,
                r#"</tool>"#,
            ]
            .join("\n"),
        ),
        PromptTurn::new(
            PromptTurnKind::TOOL_RESULT,
            [
                r#"<tool_result name="daily_life:get_current_date">"#,
                r#"<content>2026-09-12</content>"#,
                r#"</tool_result>"#,
            ]
            .join("\n"),
        ),
    ];

    let (contents, _, _) = provider
        .build_contents_and_count_tokens(&history, None, true)
        .expect("Gemini contents must be buildable");
    assert_eq!(contents.len(), 2);
    assert_eq!(
        contents[0].pointer("/parts/0/functionCall/name"),
        Some(&serde_json::json!("package_proxy"))
    );
    assert_eq!(
        contents[1].pointer("/parts/0/functionResponse/name"),
        Some(&serde_json::json!("package_proxy"))
    );
    assert_eq!(
        contents[1].pointer("/parts/0/functionResponse/response/result"),
        Some(&serde_json::json!("2026-09-12"))
    );
}

/// Verifies audio and video media links become Gemini inline_data parts.
#[test]
fn audioAndVideoLinksBecomeGeminiInlineDataParts() {
    use operit_util::MediaPoolManager::MediaPoolManager;

    let audio_id = MediaPoolManager::add_media_bytes(b"audio", "audio/mpeg");
    let video_id = MediaPoolManager::add_media_bytes(b"video", "video/mp4");
    let prompt = format!(
        "listen {} watch {}",
        MediaLinkBuilder::audio(&audio_id),
        MediaLinkBuilder::video(&video_id)
    );
    let provider = test_provider();

    let parts = provider.build_parts_array(&prompt);

    MediaPoolManager::remove_media(&audio_id);
    MediaPoolManager::remove_media(&video_id);

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0]["inline_data"]["mime_type"], "audio/mpeg");
    assert_eq!(parts[0]["inline_data"]["data"], "YXVkaW8=");
    assert_eq!(parts[1]["inline_data"]["mime_type"], "video/mp4");
    assert_eq!(parts[1]["inline_data"]["data"], "dmlkZW8=");
    assert_eq!(parts[2]["text"], "listen  watch");
}
