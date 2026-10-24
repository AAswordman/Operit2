use super::*;

/// Builds an authenticated provider fixture directly from its production catalog entry.
fn provider_config(provider: &str) -> TtsConfig {
    let catalog = TtsCatalog::provider(provider).unwrap();
    TtsConfig {
        id: "voice-list-fixture".to_string(),
        name: catalog.displayName,
        providerType: catalog.providerTypeId,
        endpoint: catalog.defaultEndpoint,
        apiKey: "fixture-key".to_string(),
        model: catalog.defaultModel,
        voice: "fixture-voice".to_string(),
        responseFormat: catalog.defaultResponseFormat,
        speed: 1.0,
        httpMethod: catalog.defaultHttpMethod,
        requestBody: catalog.defaultRequestBody,
        contentType: catalog.defaultContentType,
        headers: catalog.defaultHeaders,
        responsePipeline: catalog.defaultResponsePipeline,
        createdAt: 0,
        updatedAt: 0,
    }
}

/// Reads provider voice response schemas through the real catalog-driven JSON selectors.
#[test]
fn provider_voice_lists_preserve_ids_and_configured_models() {
    for (provider, response) in [
        (
            "CARTESIA_TTS",
            serde_json::json!({"data":[{"id":"voice-id","name":"Voice name"}]}),
        ),
        (
            "SPEECHIFY_TTS",
            serde_json::json!({"voices":[{"id":"voice-id","display_name":"Voice name","locale":"en-US"}]}),
        ),
        (
            "INWORLD_TTS",
            serde_json::json!({"voices":[{"voiceId":"voice-id","displayName":"Voice name"}]}),
        ),
        (
            "HUME_TTS",
            serde_json::json!({"voices_page":[{"id":"voice-id","name":"Voice name","provider":"HUME_AI"}]}),
        ),
        (
            "ELEVENLABS_TTS",
            serde_json::json!({"voices":[{"voice_id":"voice-id","name":"Voice name","category":"premade"}]}),
        ),
    ] {
        let config = provider_config(provider);
        let catalog = TtsCatalog::provider(provider).unwrap();
        let operation = catalog
            .operations
            .iter()
            .find(|operation| operation.operationType == "list_voices")
            .unwrap();
        let voices = parseVoiceItems(&config, operation, &response).unwrap();
        assert_eq!(voices.len(), 1, "{provider}");
        assert_eq!(voices[0].voice, "voice-id", "{provider}");
        assert_eq!(voices[0].displayName, "Voice name", "{provider}");
        assert_eq!(voices[0].model, config.model, "{provider}");
        assert_eq!(
            voices[0].responseFormat, config.responseFormat,
            "{provider}"
        );
        assert!(
            parseVoiceItems(&config, operation, &serde_json::json!({})).is_err(),
            "{provider}"
        );
    }
}

/// Keeps discovery on the configured provider host with the exact documented route and authentication.
#[test]
fn provider_voice_operations_use_declared_routes_and_credentials() {
    for (provider, route, header, expected) in [
        (
            "CARTESIA_TTS",
            "/voices?limit=100",
            "Authorization",
            "Bearer fixture-key",
        ),
        (
            "SPEECHIFY_TTS",
            "/v1/voices?limit=200",
            "Authorization",
            "Bearer fixture-key",
        ),
        (
            "INWORLD_TTS",
            "/voices/v1/voices?pageSize=2000",
            "Authorization",
            "Basic fixture-key",
        ),
        (
            "HUME_TTS",
            "/v0/tts/voices?provider=HUME_AI&page_size=100",
            "X-Hume-Api-Key",
            "fixture-key",
        ),
        (
            "ELEVENLABS_TTS",
            "/v2/voices?page_size=100",
            "xi-api-key",
            "fixture-key",
        ),
    ] {
        let config = provider_config(provider);
        let catalog = TtsCatalog::provider(provider).unwrap();
        let operation = catalog
            .operations
            .iter()
            .find(|operation| operation.operationType == "list_voices")
            .unwrap();
        let url = operationUrl(&config.endpoint, &operation.path).unwrap();
        let endpoint = url::Url::parse(&config.endpoint).unwrap();
        let mut expected_url = endpoint.origin().ascii_serialization();
        expected_url.push_str(route);
        assert_eq!(url, expected_url, "{provider}");
        let values = operationPlaceholders(&config);
        let headers = requestHeaders(&config, &values).unwrap();
        assert_eq!(
            headers.iter().find(|(name, _)| name == header).unwrap().1,
            expected,
            "{provider}"
        );
        assert_eq!(operation.authHeaderName, header, "{provider}");
        assert_eq!(
            replaceOperationPlaceholders(&operation.authHeaderValue, &values).unwrap(),
            expected,
            "{provider}"
        );
    }
}
