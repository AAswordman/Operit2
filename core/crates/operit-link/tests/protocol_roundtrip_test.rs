use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreObjectPath, CoreRequestId,
};
use serde_json::json;

/// Verifies call requests preserve their dispatch fields across JSON.
#[test]
fn call_request_roundtrip_preserves_registry_key() {
    let request = CoreCallRequest::new(
        "roundtrip-call",
        "runtime.chat",
        "send",
        json!({
            "message": "hello"
        }),
    );

    let bytes = serde_json::to_vec(&request).unwrap();
    let decoded = serde_json::from_slice::<CoreCallRequest>(&bytes).unwrap();

    assert_eq!(decoded.registryKey(), "runtime.chat::send");
    assert_eq!(decoded, request);
}

/// Verifies call responses preserve successful JSON values across JSON.
#[test]
fn call_response_roundtrip_preserves_result() {
    let response = CoreCallResponse::ok(
        CoreRequestId::new("roundtrip-response"),
        json!({
            "ok": true
        }),
    );

    let bytes = serde_json::to_vec(&response).unwrap();
    let decoded = serde_json::from_slice::<CoreCallResponse>(&bytes).unwrap();

    assert_eq!(decoded, response);
}

/// Verifies watch events preserve stream identity across JSON.
#[test]
fn watch_event_roundtrip_preserves_stream_identity() {
    let event = CoreEvent {
        requestId: Some(CoreRequestId::new("roundtrip-watch")),
        targetPath: CoreObjectPath::from("runtime.chat"),
        propertyName: "stream".to_string(),
        kind: CoreEventKind::Changed,
        value: json!({
            "delta": "hello"
        }),
    };

    let bytes = serde_json::to_vec(&event).unwrap();
    let decoded = serde_json::from_slice::<CoreEvent>(&bytes).unwrap();

    assert_eq!(decoded, event);
}
