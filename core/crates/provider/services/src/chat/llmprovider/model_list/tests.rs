use super::*;
use operit_model::ModelConfigData::{
    ApiProviderType, AvailableProviderModelSource, ProviderOperationResultSpec,
};

/// Creates a provider profile for model-list request tests.
fn test_provider(api_key: &str, custom_headers: &str) -> ProviderProfile {
    let mut provider = ProviderProfile::new(
        "test-provider".to_string(),
        "Test Provider".to_string(),
        ApiProviderType::MINIMAX,
        "https://example.com/v1/chat/completions".to_string(),
    );
    provider.apiKey = api_key.to_string();
    provider.customHeaders = custom_headers.to_string();
    provider
}

/// Creates a model-list operation with configurable authentication requirements.
fn test_operation(requires_api_key: bool) -> ProviderOperationSpec {
    ProviderOperationSpec {
        operationType: "list_models".to_string(),
        handlerId: "http_json".to_string(),
        method: "GET".to_string(),
        path: "/v1/models".to_string(),
        requiresApiKey: requires_api_key,
        result: ProviderOperationResultSpec {
            itemsJsonPath: None,
            itemIdJsonPath: None,
            inputPricePerTokenJsonPath: None,
            cachedInputPricePerTokenJsonPath: None,
            outputPricePerTokenJsonPath: None,
            pricePerRequestJsonPath: None,
            currencyJsonPath: None,
            maxContextLengthJsonPath: None,
            directImageJsonPath: None,
            directAudioJsonPath: None,
            directVideoJsonPath: None,
            toolCallJsonPath: None,
            supportsStructuredToolsJsonPath: None,
            amountJsonPath: None,
            amountCurrencyJsonPath: None,
        },
    }
}

/// Uses the actual catalog operation for Gemini discovery tests.
fn google_operation(provider_type: &ApiProviderType) -> ProviderOperationSpec {
    operit_model::ModelCatalog::ModelCatalog::provider(provider_type.name())
        .expect("Google provider catalog")
        .operations
        .into_iter()
        .find(|operation| operation.operationType == "list_models")
        .expect("Google list_models operation")
}

/// Uses query-key authentication for both Gemini provider types.
#[test]
fn google_model_list_uses_query_key_without_generated_bearer() {
    for provider_type in [ApiProviderType::GOOGLE, ApiProviderType::GEMINI_GENERIC] {
        let mut provider = test_provider("google-test-key", "{}");
        provider.providerType = provider_type;
        provider.endpoint = "https://example.com/v1beta/models".to_string();
        let operation = google_operation(&provider.providerType);
        assert_eq!(
            DiscoveryProtocol::for_provider(&provider.providerType)
                .authenticated_url(&provider, &operation)
                .unwrap(),
            "https://example.com/v1beta/models?key=google-test-key"
        );
        assert_eq!(
            DiscoveryProtocol::for_provider(&provider.providerType)
                .authenticated_headers(&provider, &operation)
                .unwrap(),
            vec![("Content-Type".to_string(), "application/json".to_string())]
        );
    }
}

/// Preserves configured origins and versions without redirecting to another service.
#[test]
fn google_model_list_preserves_configured_origin_and_version() {
    let mut provider = test_provider("google-test-key", "{}");
    provider.providerType = ApiProviderType::GEMINI_GENERIC;
    let operation = google_operation(&provider.providerType);
    for (endpoint, expected) in [
        ("https://example.com", "https://example.com/v1beta/models"),
        ("https://example.com/v1", "https://example.com/v1/models"),
        ("https://example.com/v1/", "https://example.com/v1/models"),
        (
            "https://example.com/v1/models",
            "https://example.com/v1/models",
        ),
        (
            "https://example.com/v1/models/gemini-test:generateContent",
            "https://example.com/v1/models",
        ),
        (
            "https://example.com/v1beta/models?key=old&alt=sse#fragment",
            "https://example.com/v1beta/models",
        ),
    ] {
        provider.endpoint = endpoint.to_string();
        assert_eq!(
            DiscoveryProtocol::for_provider(&provider.providerType)
                .authenticated_url(&provider, &operation)
                .unwrap(),
            format!("{expected}?key=google-test-key")
        );
    }
    provider.endpoint = "https://example.com/unknown-path".to_string();
    assert!(DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_url(&provider, &operation)
        .is_err());
}

/// Encodes selected key-pool entries as one query parameter without leaking old keys.
#[test]
fn google_model_list_encodes_preferred_api_key() {
    use operit_model::ApiKeyInfo::ApiKeyInfo;
    let mut provider = test_provider("unused", "{}");
    provider.providerType = ApiProviderType::GOOGLE;
    provider.endpoint = "https://example.com/v1beta/models?key=old".to_string();
    provider.useMultipleApiKeys = true;
    let mut disabled = ApiKeyInfo::new("disabled".to_string(), "disabled-key".to_string());
    disabled.isEnabled = false;
    provider.apiKeyPool = vec![
        disabled,
        ApiKeyInfo::new("first".to_string(), "first-key".to_string()),
        ApiKeyInfo::new("second".to_string(), "key+&=? /".to_string()),
    ];
    provider.currentKeyIndex = 2;
    let operation = google_operation(&provider.providerType);
    let url = url::Url::parse(
        &DiscoveryProtocol::for_provider(&provider.providerType)
            .authenticated_url(&provider, &operation)
            .unwrap(),
    )
    .unwrap();
    let query: Vec<_> = url.query_pairs().collect();
    assert_eq!(query.len(), 1);
    assert_eq!(query[0].0, "key");
    assert_eq!(query[0].1, "key+&=? /");
    provider.apiKeyPool.clear();
    assert!(DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_url(&provider, &operation)
        .is_err());
}

/// Rejects missing query credentials even when custom OAuth headers are present.
#[test]
fn google_model_list_requires_configured_api_key() {
    let mut provider = test_provider(" ", r#"{"Authorization":"Bearer custom"}"#);
    provider.providerType = ApiProviderType::GOOGLE;
    provider.endpoint = "https://example.com/v1beta/models".to_string();
    assert_eq!(
        DiscoveryProtocol::for_provider(&provider.providerType)
            .authenticated_url(&provider, &google_operation(&provider.providerType))
            .unwrap_err(),
        "provider api key is required"
    );
}

/// Applies explicit custom headers without generating authentication headers.
#[test]
fn google_model_list_preserves_explicit_custom_headers() {
    let mut provider = test_provider(
        "google-test-key",
        r#"{"content-type":"application/custom","Authorization":"Token custom","X-Test":"test"}"#,
    );
    provider.providerType = ApiProviderType::GEMINI_GENERIC;
    provider.endpoint = "https://example.com/v1beta/models".into();
    let operation = google_operation(&provider.providerType);
    let request_headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &operation)
        .unwrap();
    for (name, expected) in [
        ("Content-Type", "application/custom"),
        ("Authorization", "Token custom"),
        ("X-Test", "test"),
    ] {
        let values: Vec<_> = request_headers
            .iter()
            .filter(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(values, vec![expected]);
    }
    provider.customHeaders = r#"{"X-Test":123}"#.to_string();
    assert!(DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &operation)
        .is_err());
}

/// Lists only declared generation models with Kotlin's base identities and ordering.
#[test]
fn google_model_list_filters_and_normalizes_generation_models() {
    let items = vec![
        serde_json::json!({
            "name": "models/gemini-z-001",
            "baseModelId": "gemini-z",
            "supportedGenerationMethods": ["generateContent", "countTokens"]
        }),
        serde_json::json!({
            "name": "models/embedding-test",
            "supportedGenerationMethods": ["embedContent"]
        }),
        serde_json::json!({
            "name": "models/gemini-a-001",
            "baseModelId": "gemini-a",
            "supportedGenerationMethods": ["generateContent"]
        }),
    ];
    for provider_type in [ApiProviderType::GOOGLE, ApiProviderType::GEMINI_GENERIC] {
        let models = DiscoveryProtocol::for_provider(&provider_type)
            .parse_items(&items, &google_operation(&provider_type))
            .unwrap();
        assert_eq!(
            models
                .iter()
                .map(|model| model.modelId.as_str())
                .collect::<Vec<_>>(),
            vec!["gemini-a", "gemini-z"]
        );
        assert!(models
            .iter()
            .all(|model| model.source == AvailableProviderModelSource::Remote));
    }
}

/// Reports malformed Gemini metadata instead of inventing model capabilities or IDs.
#[test]
fn google_model_list_rejects_invalid_generation_metadata() {
    let provider_type = ApiProviderType::GOOGLE;
    let operation = google_operation(&provider_type);
    for item in [
        serde_json::json!({"name": "models/test", "baseModelId": "test"}),
        serde_json::json!({"supportedGenerationMethods": "generateContent"}),
        serde_json::json!({"supportedGenerationMethods": [123]}),
        serde_json::json!({"name": "models/test", "supportedGenerationMethods": ["generateContent"]}),
        serde_json::json!({"name": "models/test", "baseModelId": " ", "supportedGenerationMethods": ["generateContent"]}),
        serde_json::json!({"name": "models/test", "baseModelId": 123, "supportedGenerationMethods": ["generateContent"]}),
    ] {
        assert!(DiscoveryProtocol::for_provider(&provider_type)
            .parse_items(&[item], &operation)
            .is_err());
    }
    assert!(DiscoveryProtocol::for_provider(&provider_type)
        .parse_items(&[], &operation)
        .unwrap()
        .is_empty());
}

/// Keeps official and generic Anthropic model requests on the same protocol.
#[test]
fn anthropic_model_list_uses_version_and_api_key_headers() {
    for provider_type in [
        ApiProviderType::ANTHROPIC,
        ApiProviderType::ANTHROPIC_GENERIC,
    ] {
        let mut provider = test_provider("sk-ant-test", "{}");
        provider.providerType = provider_type;
        let headers = DiscoveryProtocol::for_provider(&provider.providerType)
            .authenticated_headers(&provider, &test_operation(true))
            .unwrap();
        assert_eq!(
            headers,
            vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
                ("x-api-key".to_string(), "sk-ant-test".to_string()),
            ]
        );
    }
}

/// Applies explicit Anthropic header overrides without duplicate header names.
#[test]
fn anthropic_custom_headers_override_protocol_headers_case_insensitively() {
    let mut provider = test_provider(
        "sk-ant-test",
        r#"{"X-Api-Key":"custom-key","Anthropic-Version":"2023-06-01","X-Test":"keep-me"}"#,
    );
    provider.providerType = ApiProviderType::ANTHROPIC_GENERIC;
    let headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &test_operation(true))
        .unwrap();
    for (name, expected) in [
        ("x-api-key", "custom-key"),
        ("anthropic-version", "2023-06-01"),
        ("x-test", "keep-me"),
    ] {
        let values: Vec<_> = headers
            .iter()
            .filter(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(values, vec![expected]);
    }
    assert!(!headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("Authorization")));
}

/// Rejects missing or explicitly emptied Anthropic protocol headers.
#[test]
fn anthropic_invalid_authentication_and_version_are_reported() {
    for (api_key, custom_headers) in [
        ("", "{}"),
        ("", r#"{"Authorization":"Bearer custom"}"#),
        ("sk-ant-test", r#"{"X-Api-Key":" "}"#),
        ("sk-ant-test", r#"{"Anthropic-Version":" "}"#),
        ("sk-ant-test", r#"{"X-Api-Key":123}"#),
    ] {
        let mut provider = test_provider(api_key, custom_headers);
        provider.providerType = ApiProviderType::ANTHROPIC;
        assert!(DiscoveryProtocol::for_provider(&provider.providerType)
            .authenticated_headers(&provider, &test_operation(true))
            .is_err());
    }
}

/// Authenticates Anthropic discovery with the preferred enabled entry.
#[test]
fn anthropic_model_list_uses_the_selected_api_key_pool_entry() {
    use operit_model::ApiKeyInfo::ApiKeyInfo;
    let mut provider = test_provider("unused", "{}");
    provider.providerType = ApiProviderType::ANTHROPIC;
    provider.useMultipleApiKeys = true;
    let mut disabled = ApiKeyInfo::new("disabled".to_string(), "disabled-key".to_string());
    disabled.isEnabled = false;
    provider.apiKeyPool = vec![
        disabled,
        ApiKeyInfo::new("blank".to_string(), " ".to_string()),
        ApiKeyInfo::new("first".to_string(), "sk-ant-first".to_string()),
        ApiKeyInfo::new("second".to_string(), "sk-ant-second".to_string()),
    ];
    provider.currentKeyIndex = 3;
    let headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &test_operation(true))
        .unwrap();
    assert!(headers
        .iter()
        .any(|(name, value)| name == "x-api-key" && value == "sk-ant-second"));
    provider.apiKeyPool.clear();
    assert!(DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &test_operation(true))
        .is_err());
}

/// Verifies generated authorization survives empty custom header.
#[test]
fn generated_authorization_survives_empty_custom_header() {
    let provider = test_provider("sk-test", r#"{"Authorization":""}"#);
    let headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &test_operation(true))
        .expect("headers should build");
    let authorization = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.as_str());
    assert_eq!(authorization, Some("Bearer sk-test"));
    assert_eq!(
        headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .count(),
        1
    );
}

/// Verifies non empty custom authorization is preserved.
#[test]
fn non_empty_custom_authorization_is_preserved() {
    let provider = test_provider("sk-test", r#"{"authorization":"Token custom"}"#);
    let headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &test_operation(true))
        .expect("headers should build");
    let authorization = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.as_str());
    assert_eq!(authorization, Some("Token custom"));
}

/// Creates an OpenCode provider with a configured catalog endpoint.
fn opencode_provider(endpoint: &str, api_key: &str, custom_headers: &str) -> ProviderProfile {
    let mut provider = test_provider(api_key, custom_headers);
    provider.providerType = ApiProviderType::OPENCODE;
    provider.providerTypeId = "OPENCODE".to_string();
    provider.endpoint = endpoint.to_string();
    provider
}

/// Reads the OpenCode model-list operation from the actual provider catalog.
fn opencode_operation() -> ProviderOperationSpec {
    operit_model::ModelCatalog::ModelCatalog::provider("OPENCODE")
        .expect("OpenCode catalog should exist")
        .operations
        .into_iter()
        .find(|operation| operation.operationType == "list_models")
        .expect("OpenCode should support model listing")
}

/// Verifies opencode model urls preserve zen and go base paths.
#[test]
fn opencode_model_urls_preserve_zen_and_go_base_paths() {
    for (endpoint, expected) in [
        (
            "https://opencode.ai/zen",
            "https://opencode.ai/zen/v1/models",
        ),
        (
            "https://opencode.ai/zen/",
            "https://opencode.ai/zen/v1/models",
        ),
        (
            "https://opencode.ai/zen/v1",
            "https://opencode.ai/zen/v1/models",
        ),
        (
            "https://opencode.ai/zen/v1/",
            "https://opencode.ai/zen/v1/models",
        ),
        (
            "https://opencode.ai/zen/go",
            "https://opencode.ai/zen/go/v1/models",
        ),
        (
            "https://opencode.ai/zen/go/v1/",
            "https://opencode.ai/zen/go/v1/models",
        ),
        (
            "https://gateway.example/proxy/zen/go",
            "https://gateway.example/proxy/zen/go/v1/models",
        ),
    ] {
        let provider = opencode_provider(endpoint, "sk-test", "{}");
        assert_eq!(
            DiscoveryProtocol::for_provider(&provider.providerType)
                .authenticated_url(&provider, &opencode_operation())
                .unwrap(),
            expected
        );
    }
}

/// Verifies opencode model listing sets agent user agent.
#[test]
fn opencode_model_listing_sets_agent_user_agent() {
    let provider = opencode_provider(
        "https://opencode.ai/zen/go",
        "sk-test",
        r#"{"user-agent":"custom-client","X-Test":"keep-me"}"#,
    );
    let headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &opencode_operation())
        .unwrap();
    let user_agents: Vec<_> = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("User-Agent"))
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(
        user_agents,
        vec![concat!("Operit/", env!("CARGO_PKG_VERSION"))]
    );
    assert!(headers.contains(&("X-Test".to_string(), "keep-me".to_string())));
    assert!(headers.contains(&("Authorization".to_string(), "Bearer sk-test".to_string())));
}

/// Verifies opencode public model catalog does not require an api key.
#[test]
fn opencode_public_model_catalog_does_not_require_an_api_key() {
    let provider = opencode_provider("https://opencode.ai/zen", "", "{}");
    let headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &opencode_operation())
        .unwrap();
    assert!(!headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("Authorization")));
}

/// Verifies ordinary provider operations keep their catalog paths.
#[test]
fn ordinary_provider_operations_keep_their_catalog_paths() {
    let provider = test_provider("sk-test", "{}");
    assert_eq!(
        DiscoveryProtocol::for_provider(&provider.providerType)
            .authenticated_url(&provider, &test_operation(true))
            .unwrap(),
        "https://example.com/v1/models"
    );
    assert!(DiscoveryProtocol::Bearer
        .authenticated_headers(&test_provider("", "{}"), &test_operation(true))
        .is_err());
}

/// Preserves every gateway path segment for OpenAI-compatible discovery protocols.
#[test]
fn catalog_model_urls_preserve_nested_gateway_paths() {
    for provider_type in [
        ApiProviderType::OPENAI,
        ApiProviderType::XAI,
        ApiProviderType::OPENAI_GENERIC,
        ApiProviderType::OPENAI_LOCAL,
        ApiProviderType::OPENAI_RESPONSES,
        ApiProviderType::OPENAI_RESPONSES_GENERIC,
        ApiProviderType::ANTHROPIC,
        ApiProviderType::ANTHROPIC_GENERIC,
        ApiProviderType::OTHER,
    ] {
        let mut provider = test_provider("sk-test", "{}");
        provider.providerType = provider_type;
        let operation =
            operit_model::ModelCatalog::ModelCatalog::provider(provider.providerType.name())
                .unwrap()
                .operations
                .into_iter()
                .find(|operation| operation.operationType == "list_models")
                .unwrap();
        let protocol = DiscoveryProtocol::for_provider(&provider.providerType);
        for (path, expected) in [
            ("", "/v1/models"),
            ("/", "/v1/models"),
            ("/a/b/c", "/a/b/c/v1/models"),
            ("/a/b/c/", "/a/b/c/v1/models"),
            ("/a/b/c/v1", "/a/b/c/v1/models"),
            ("/a/b/c/v1/", "/a/b/c/v1/models"),
            ("/a/b/c/v1/chat/completions", "/a/b/c/v1/models"),
            ("/a/b/c/v1/chat/completions/", "/a/b/c/v1/models"),
            ("/a/b/c/v1/responses", "/a/b/c/v1/models"),
            ("/a/b/c/v1/messages", "/a/b/c/v1/models"),
            ("/a/b/c/v1/models", "/a/b/c/v1/models"),
            ("/a/b/c/chat/completions", "/a/b/c/models"),
            ("/a/b/c/responses", "/a/b/c/models"),
            ("/a/b/c/messages", "/a/b/c/models"),
            ("/a/b/c/models", "/a/b/c/models"),
            ("/a/b/c/V1", "/a/b/c/V1/models"),
            ("/a/b/c/v2", "/a/b/c/v2/models"),
            ("/a/b/c/v2/responses", "/a/b/c/v2/models"),
            ("/v1gateway/a/b/c", "/v1gateway/a/b/c/v1/models"),
            ("/a/v1/b/v2/c/v3/chat/completions", "/a/v1/b/v2/c/v3/models"),
            ("/a%20b/c%2Fd/v1/chat/completions", "/a%20b/c%2Fd/v1/models"),
            ("/a//b/c/v1/chat/completions", "/a//b/c/v1/models"),
            ("/a/b/c/v1/responses?key=old#fragment", "/a/b/c/v1/models"),
        ] {
            provider.endpoint = format!("https://example.com:8443{path}");
            assert_eq!(
                protocol.authenticated_url(&provider, &operation).unwrap(),
                format!("https://example.com:8443{expected}"),
                "provider={} endpoint={}",
                provider.providerType.name(),
                provider.endpoint,
            );
        }
    }
}

/// Preserves provider-specific namespaces instead of copying their catalog path twice.
#[test]
fn catalog_model_urls_preserve_provider_namespaces() {
    for (provider_type, endpoint_path, expected) in [
        (
            ApiProviderType::OPENROUTER,
            "/api/v1/chat/completions",
            "/api/v1/models",
        ),
        (
            ApiProviderType::OPENROUTER,
            "/a/b/c/api/v1/chat/completions",
            "/a/b/c/api/v1/models",
        ),
        (
            ApiProviderType::OPENROUTER,
            "/a/b/c/api",
            "/a/b/c/api/v1/models",
        ),
        (
            ApiProviderType::ALIYUN,
            "/compatible-mode/v1/chat/completions",
            "/compatible-mode/v1/models",
        ),
        (
            ApiProviderType::ALIYUN,
            "/a/b/c/compatible-mode",
            "/a/b/c/compatible-mode/v1/models",
        ),
        (
            ApiProviderType::ZHIPU,
            "/api/coding/paas/v4/chat/completions",
            "/api/coding/paas/v4/models",
        ),
        (
            ApiProviderType::ZHIPU,
            "/a/b/c/api/paas/v4",
            "/a/b/c/api/paas/v4/models",
        ),
        (
            ApiProviderType::DOUBAO,
            "/api/coding/v3/chat/completions",
            "/api/coding/v3/models",
        ),
        (
            ApiProviderType::INFINIAI,
            "/maas/v1/chat/completions",
            "/maas/v1/models",
        ),
        (
            ApiProviderType::PPINFRA,
            "/openai/v1/chat/completions",
            "/openai/v1/models",
        ),
        (
            ApiProviderType::NOVITA,
            "/anthropic/v1/messages",
            "/anthropic/v1/models",
        ),
    ] {
        let mut provider = test_provider("sk-test", "{}");
        provider.providerType = provider_type;
        provider.endpoint = format!("https://example.com{endpoint_path}");
        let operation =
            operit_model::ModelCatalog::ModelCatalog::provider(provider.providerType.name())
                .unwrap()
                .operations
                .into_iter()
                .find(|operation| operation.operationType == "list_models")
                .unwrap();
        assert_eq!(
            DiscoveryProtocol::for_provider(&provider.providerType)
                .authenticated_url(&provider, &operation)
                .unwrap(),
            format!("https://example.com{expected}"),
        );
    }
}

/// Rejects malformed operation paths before they can produce a discovery request.
#[test]
fn catalog_model_urls_reject_invalid_operation_paths() {
    let endpoint = url::Url::parse("https://example.com/a/b/c").unwrap();
    for path in [
        "v1/models",
        "/v1/models?key=invalid",
        "/v1/models#fragment",
        "/v1/chat/completions",
    ] {
        assert!(endpoint::models_path(&endpoint, path).is_err());
    }
    let mut provider = test_provider("sk-test", "{}");
    provider.endpoint = "not a URL".to_string();
    assert!(DiscoveryProtocol::Bearer
        .authenticated_url(&provider, &test_operation(true))
        .is_err());
}

/// Verifies opencode optional auth preserves custom authorization.
#[test]
fn opencode_optional_auth_preserves_custom_authorization() {
    let provider = opencode_provider(
        "https://opencode.ai/zen",
        "sk-test",
        r#"{"authorization":"Bearer custom"}"#,
    );
    let headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &opencode_operation())
        .unwrap();
    let authorization: Vec<_> = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("Authorization"))
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(authorization, vec!["Bearer custom"]);
}

/// Uses the physical preferred entry without advancing it during header construction.
#[test]
fn opencode_optional_auth_uses_preferred_pool_entry() {
    use operit_model::ApiKeyInfo::ApiKeyInfo;
    let mut provider = opencode_provider("https://opencode.ai/zen/go", "unused", "{}");
    provider.useMultipleApiKeys = true;
    let mut disabled = ApiKeyInfo::new("disabled".to_string(), "disabled-key".to_string());
    disabled.isEnabled = false;
    provider.apiKeyPool = vec![
        disabled,
        ApiKeyInfo::new("blank".to_string(), "  ".to_string()),
        ApiKeyInfo::new("first".to_string(), "sk-first".to_string()),
        ApiKeyInfo::new("second".to_string(), "Bearer sk-second".to_string()),
    ];
    provider.currentKeyIndex = 3;
    let headers = DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &opencode_operation())
        .unwrap();
    assert!(headers.contains(&("Authorization".to_string(), "Bearer sk-second".to_string())));
    provider.apiKeyPool.clear();
    assert!(DiscoveryProtocol::for_provider(&provider.providerType)
        .authenticated_headers(&provider, &opencode_operation())
        .is_err());
}

/// Exercises final request authentication through the production transport encoder.
impl DiscoveryProtocol {
    /// Resolves an isolated test profile through the actual key decision policy.
    fn authenticated_request(
        self,
        provider: &ProviderProfile,
        operation: &ProviderOperationSpec,
    ) -> Result<operit_host_api::HttpRequestData, String> {
        use crate::chat::llmprovider::ApiKeyProvider::{ApiKeyPoolState, KeyGetter};
        use crate::chat::llmprovider::ProviderHttp::apply_key;
        let mut request = super::super::ModelListFetcher::request(provider, operation, self)?;
        let provider = provider.clone();
        let getter = KeyGetter::from_callbacks(
            std::sync::Arc::new(move || {
                ApiKeyPoolState::default().reserve(&mut provider.clone(), 0)
            }),
            std::sync::Arc::new(|_, _, _| Ok(())),
        );
        let getter = if operation.requiresApiKey || self == Self::Gemini {
            getter.required()
        } else {
            getter
        };
        let getter = self.authentication().getter(&request, getter);
        apply_key(&mut request, &self.authentication(), &getter.acquire()?)
            .map_err(|error| error.to_string())?;
        Ok(request)
    }
    /// Returns the authenticated URL observed at the HTTP boundary.
    fn authenticated_url(
        self,
        provider: &ProviderProfile,
        operation: &ProviderOperationSpec,
    ) -> Result<String, String> {
        Ok(self.authenticated_request(provider, operation)?.url)
    }
    /// Returns the authenticated headers observed at the HTTP boundary.
    fn authenticated_headers(
        self,
        provider: &ProviderProfile,
        operation: &ProviderOperationSpec,
    ) -> Result<Vec<(String, String)>, String> {
        Ok(self.authenticated_request(provider, operation)?.headers)
    }
}
