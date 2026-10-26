use super::*;
use operit_model::TtsCatalog::TtsCatalog;

/// Creates a speech configuration from the production catalog without accessing a provider.
fn catalog_config(provider: &str) -> TtsConfig {
    let catalog = TtsCatalog::provider(provider).unwrap();
    TtsConfig {
        id: "catalog-fixture".to_string(),
        name: catalog.displayName,
        providerType: catalog.providerTypeId,
        endpoint: catalog.defaultEndpoint,
        apiKey: "fixture-key".to_string(),
        model: catalog.defaultModel,
        voice: "fixture-voice".to_string(),
        responseFormat: catalog.defaultResponseFormat,
        speed: 1.1,
        httpMethod: catalog.defaultHttpMethod,
        requestBody: catalog.defaultRequestBody,
        contentType: catalog.defaultContentType,
        headers: catalog.defaultHeaders,
        responsePipeline: catalog.defaultResponsePipeline,
        createdAt: 0,
        updatedAt: 0,
    }
}

/// Renders real provider request templates with JSON escaping and numeric speed values.
#[test]
fn catalog_requests_preserve_text_and_provider_fields() {
    let text = "你好，\"Operit\"。\nC:\\voices & <speech>";
    for (provider, text_path, model_path, voice_path) in [
        ("QWEN_TTS", "/input/text", "/model", "/input/voice"),
        ("CARTESIA_TTS", "/transcript", "/model_id", "/voice/id"),
        ("SPEECHIFY_TTS", "/input", "/model", "/voice_id"),
        ("INWORLD_TTS", "/text", "/modelId", "/voiceId"),
        (
            "HUME_TTS",
            "/utterances/0/text",
            "/version",
            "/utterances/0/voice/id",
        ),
        ("RIME_TTS", "/text", "/modelId", "/speaker"),
        ("ELEVENLABS_TTS", "/text", "/model_id", ""),
    ] {
        let config = catalog_config(provider);
        let values = placeholders(&config, text);
        let rendered = replacePlaceholders(
            &config.requestBody,
            &values,
            placeholderModeFromContentType(&config.contentType),
        )
        .unwrap();
        let body: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(
            body.pointer(text_path).unwrap().as_str(),
            Some(text),
            "{provider}"
        );
        assert_eq!(
            body.pointer(model_path).unwrap().as_str(),
            Some(config.model.as_str()),
            "{provider}"
        );
        if !voice_path.is_empty() {
            assert_eq!(
                body.pointer(voice_path).unwrap().as_str(),
                Some("fixture-voice"),
                "{provider}"
            );
        }
        assert_eq!(normalizeMethod(&config.httpMethod).unwrap(), "POST");
        let endpoint =
            replacePlaceholders(&config.endpoint, &values, PlaceholderMode::Url).unwrap();
        validateHttpUrl(&endpoint).unwrap();
        assert_eq!(
            HttpVoiceProvider::new().outputExtension(&config).unwrap(),
            config.responseFormat.split('_').next().unwrap()
        );
    }
    for (provider, speed_path) in [
        ("CARTESIA_TTS", "/generation_config/speed"),
        ("INWORLD_TTS", "/audioConfig/speakingRate"),
        ("HUME_TTS", "/utterances/0/speed"),
        ("ELEVENLABS_TTS", "/voice_settings/speed"),
    ] {
        let config = catalog_config(provider);
        let rendered = replacePlaceholders(
            &config.requestBody,
            &placeholders(&config, text),
            PlaceholderMode::JsonBody,
        )
        .unwrap();
        let body: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(
            body.pointer(speed_path).unwrap().as_f64(),
            Some(1.1),
            "{provider}"
        );
    }
}

/// Renders each provider's declared authentication through the shared HTTP header path.
#[test]
fn catalog_requests_use_provider_authentication() {
    for (provider, name, expected) in [
        ("QWEN_TTS", "Authorization", "Bearer fixture-key"),
        ("CARTESIA_TTS", "Authorization", "Bearer fixture-key"),
        ("CARTESIA_TTS", "Cartesia-Version", "2026-08-14"),
        ("SPEECHIFY_TTS", "Authorization", "Bearer fixture-key"),
        ("INWORLD_TTS", "Authorization", "Basic fixture-key"),
        ("HUME_TTS", "X-Hume-Api-Key", "fixture-key"),
        ("RIME_TTS", "Authorization", "Bearer fixture-key"),
        ("RIME_TTS", "Accept", "audio/mpeg"),
        ("ELEVENLABS_TTS", "xi-api-key", "fixture-key"),
    ] {
        let config = catalog_config(provider);
        let headers = replaceHeaders(&config.headers, &placeholders(&config, "text")).unwrap();
        assert_eq!(
            headers.iter().find(|(key, _)| key == name).unwrap().1,
            expected,
            "{provider}"
        );
    }
}

/// Decodes provider JSON responses into exactly the audio bytes returned by the API.
#[test]
fn catalog_base64_audio_responses_use_shared_pipeline() {
    let bytes = b"ID3\x04\x00\x01audio-fixture";
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    for (provider, response, invalid) in [
        (
            "SPEECHIFY_TTS",
            serde_json::json!({"audio_data": encoded}),
            serde_json::json!({"audio_data": "invalid-base64!"}),
        ),
        (
            "INWORLD_TTS",
            serde_json::json!({"audioContent": encoded}),
            serde_json::json!({"audioContent": "invalid-base64!"}),
        ),
    ] {
        let config = catalog_config(provider);
        let payload = BinaryPayload {
            bytes: serde_json::to_vec(&response).unwrap(),
            contentType: Some("application/json".to_string()),
        };
        let audio = resolvePipelineAudio(payload, &config.responsePipeline).unwrap();
        assert_eq!(audio.bytes, bytes, "{provider}");
        for malformed in [serde_json::json!({}), invalid] {
            assert!(
                resolvePipelineAudio(
                    BinaryPayload {
                        bytes: serde_json::to_vec(&malformed).unwrap(),
                        contentType: Some("application/json".to_string()),
                    },
                    &config.responsePipeline
                )
                .is_err(),
                "{provider}"
            );
        }
    }
}

/// Preserves binary speech responses without requiring provider-specific processing.
#[test]
fn catalog_binary_audio_responses_preserve_bytes() {
    for provider in ["CARTESIA_TTS", "HUME_TTS", "RIME_TTS", "ELEVENLABS_TTS"] {
        let config = catalog_config(provider);
        let audio = resolvePipelineAudio(
            BinaryPayload {
                bytes: b"ID3binary-audio".to_vec(),
                contentType: Some("audio/mpeg".to_string()),
            },
            &config.responsePipeline,
        )
        .unwrap();
        assert_eq!(audio.bytes, b"ID3binary-audio", "{provider}");
        assert_eq!(
            audio.contentType.as_deref(),
            Some("audio/mpeg"),
            "{provider}"
        );
    }
}

/// Extracts the signed Qwen audio URL without attaching the synthesis credentials to its download.
#[test]
fn qwen_audio_response_extracts_download_url() {
    let config = catalog_config("QWEN_TTS");
    let signed_url = "https://audio.example.invalid/speech.wav?token=signed&expires=123";
    let payload = BinaryPayload {
        bytes: serde_json::to_vec(&serde_json::json!({"output":{"audio":{"url":signed_url}}}))
            .unwrap(),
        contentType: Some("application/json".to_string()),
    };
    let json = parseJsonPayload(PipelineValue::Binary(payload), 0).unwrap();
    let picked = pickJsonValue(
        PipelineValue::Json(json),
        &config.responsePipeline[1].path,
        1,
    )
    .unwrap();
    assert_eq!(picked.as_str(), Some(signed_url));
    assert_eq!(config.responsePipeline[2].stepType, "http_get");
    assert!(config.responsePipeline[2].headers.is_empty());
}
