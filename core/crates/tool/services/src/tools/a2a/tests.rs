use super::*;

fn modern_card() -> Value {
    json!({"name":"Research", "description":"Research tasks", "version":"1", "supportedInterfaces":[{"url":"https://agent.example/a2a", "protocolBinding":"JSONRPC", "protocolVersion":"1.0", "tenant":"team"}], "skills":[{"name":"Research","description":"Find references"}]})
}

#[test]
fn selects_supported_jsonrpc_interface_and_preserves_tenant() {
    let selected = interface(&modern_card()).unwrap();
    assert_eq!(selected.url, "https://agent.example/a2a");
    assert_eq!(selected.tenant, "team");
    assert!(interface(&json!({"supportedInterfaces":[{"url":"https://agent.example", "protocolBinding":"GRPC","protocolVersion":"1.0"}]})).is_err());
    assert!(interface(&json!({"supportedInterfaces":[{"url":"https://agent.example", "protocolBinding":"JSONRPC","protocolVersion":"2.0"}]})).is_err());
}

#[test]
fn legacy_and_modern_messages_use_their_actual_wire_schemas() {
    let args = json!({"message":"Continue", "task_id":"task-1", "context_id":"context-1"});
    let (method, modern) = build_operation("1.0", "send_message", &args).unwrap();
    assert_eq!(method, "SendMessage");
    assert_eq!(modern["message"]["role"], "ROLE_USER");
    assert_eq!(modern["message"]["taskId"], "task-1");
    assert_eq!(modern["configuration"]["returnImmediately"], true);
    assert!(modern["message"]["parts"][0].get("kind").is_none());
    let (method, legacy) = build_operation("0.3.0", "send_message", &args).unwrap();
    assert_eq!(method, "message/send");
    assert_eq!(legacy["message"]["kind"], "message");
    assert_eq!(legacy["message"]["parts"][0]["kind"], "text");
    assert_eq!(legacy["configuration"]["blocking"], false);
    assert!(build_operation("1.0", "cancel_task", &json!({})).is_err());
}

#[test]
fn rejects_unsupported_urls_and_header_injection() {
    for url in [
        "file:///tmp/agent",
        "https://user:password@example.com",
        "https://example.com/#fragment",
    ] {
        assert!(validate_config(&A2AConfig {
            url: url.into(),
            ..Default::default()
        })
        .is_err());
    }
    let config = A2AConfig {
        url: "https://example.com".into(),
        headers: BTreeMap::from([("Authorization".into(), "token\r\nX-Other: value".into())]),
        ..Default::default()
    };
    assert!(validate_config(&config).is_err());
    assert!(validate_id("../agent").is_err());
}

#[test]
fn agent_capabilities_are_discoverable_as_package_tools() {
    let package = tool_package(
        "research",
        &A2AConfig {
            url: "https://agent.example".into(),
            card: modern_card(),
            ..Default::default()
        },
    );
    assert_eq!(package.name, "a2a.research");
    assert_eq!(package.category, "A2A");
    assert!(package
        .description
        .resolve(false)
        .contains("Find references"));
    assert_eq!(
        package
            .tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        vec!["send_message", "get_task", "cancel_task"]
    );
}

#[test]
fn rejects_invalid_card_shapes_and_unsupported_protocol_versions() {
    for version in ["0.30", "0.3x", "1.0.bad", "1.1", "2.0", ""] {
        assert!(!supported_version(version), "{version}");
    }
    let mut config = A2AConfig {
        url: "https://agent.example".into(),
        card: modern_card(),
        ..Default::default()
    };
    config.card["skills"] = json!({"name":"Not a skill array"});
    assert!(validate_config(&config).is_err());
}

#[test]
fn ordinary_script_categories_do_not_become_connection_packages() {
    let mut script = ToolPackage {
        name: "ordinary_script".into(),
        category: "A2A".into(),
        ..Default::default()
    };
    assert!(!is_a2a_package(&script));
    script.name = "a2a.ordinary_script".into();
    assert!(!is_a2a_package(&script));
    assert!(is_a2a_package(&tool_package(
        "research",
        &A2AConfig::default()
    )));
}
