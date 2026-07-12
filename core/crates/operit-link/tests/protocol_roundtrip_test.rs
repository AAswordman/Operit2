use std::collections::BTreeMap;

use operit_link::{
    decodeLink, encodeLink, fromCoreValue, toCoreValue, CoreCallRequest, CoreCallResponse,
    CoreEvent, CoreEventKind, CoreObjectPath, CorePushItem, CorePushRequest, CoreRequestId,
    CoreValue,
};
use serde::{Deserialize, Serialize};

/// Builds a string-keyed CoreValue map for protocol tests.
fn core_map(entries: impl IntoIterator<Item = (&'static str, CoreValue)>) -> CoreValue {
    CoreValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

/// Verifies call requests preserve dispatch fields through the Link codec.
#[test]
fn call_request_roundtrip_preserves_registry_key() {
    let request = CoreCallRequest::new(
        "roundtrip-call",
        "runtime.chat",
        "send",
        core_map([("message", CoreValue::String("hello".to_string()))]),
    );

    let bytes = encodeLink(&request).unwrap();
    let decoded = decodeLink::<CoreCallRequest>(&bytes).unwrap();

    assert_eq!(decoded.registryKey(), "runtime.chat::send");
    assert_eq!(decoded, request);
}

/// Verifies call responses preserve successful structured values.
#[test]
fn call_response_roundtrip_preserves_result() {
    let response = CoreCallResponse::ok(
        CoreRequestId::new("roundtrip-response"),
        core_map([("ok", CoreValue::Bool(true))]),
    );

    let bytes = encodeLink(&response).unwrap();
    let decoded = decodeLink::<CoreCallResponse>(&bytes).unwrap();

    assert_eq!(decoded, response);
}

/// Verifies watch events preserve stream identity through the Link codec.
#[test]
fn watch_event_roundtrip_preserves_stream_identity() {
    let event = CoreEvent {
        requestId: Some(CoreRequestId::new("roundtrip-watch")),
        targetPath: CoreObjectPath::from("runtime.chat"),
        propertyName: "stream".to_string(),
        kind: CoreEventKind::Changed,
        value: core_map([("delta", CoreValue::String("hello".to_string()))]),
    };

    let bytes = encodeLink(&event).unwrap();
    let decoded = decodeLink::<CoreEvent>(&bytes).unwrap();

    assert_eq!(decoded, event);
}

/// Verifies push targets and ordered items preserve client-owned stream identity.
#[test]
fn push_item_roundtrip_builds_the_target_call() {
    let request = CorePushRequest::new("push-input", "runtime.browser", "interact");
    let item = CorePushItem {
        pushId: "push-input".to_string(),
        sequence: 7,
        args: core_map([("type", CoreValue::String("scroll".to_string()))]),
    };

    let decoded = decodeLink::<CorePushItem>(&encodeLink(&item).unwrap()).unwrap();
    let call = request.itemCall(decoded.sequence, decoded.args.clone());

    assert_eq!(decoded, item);
    assert_eq!(call.requestId.0, "push-input:7");
    assert_eq!(call.registryKey(), "runtime.browser::interact");
}

/// Verifies native bytes use the MessagePack bin family instead of an integer array.
#[test]
fn core_value_bytes_use_message_pack_bin() {
    let bytes = encodeLink(CoreValue::Bytes(vec![1, 2, 3, 4])).unwrap();

    assert_eq!(bytes, vec![0xc4, 4, 1, 2, 3, 4]);
    assert_eq!(
        decodeLink::<CoreValue>(&bytes).unwrap(),
        CoreValue::Bytes(vec![1, 2, 3, 4])
    );
}

/// Verifies empty native bytes still use the MessagePack bin family.
#[test]
fn empty_core_value_bytes_use_message_pack_bin() {
    let bytes = encodeLink(CoreValue::Bytes(Vec::new())).unwrap();

    assert_eq!(bytes, vec![0xc4, 0]);
    assert_eq!(
        decodeLink::<CoreValue>(&bytes).unwrap(),
        CoreValue::Bytes(Vec::new())
    );
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct TypedValue {
    title: String,
    #[serde(with = "serde_bytes")]
    payload: Vec<u8>,
    count: u64,
}

/// Verifies typed values convert through CoreValue without JSON normalization.
#[test]
fn typed_core_value_conversion_preserves_native_bytes() {
    let source = TypedValue {
        title: "frame".to_string(),
        payload: vec![0, 127, 255],
        count: u64::MAX,
    };

    let value = toCoreValue(&source).unwrap();
    let CoreValue::Map(fields) = &value else {
        panic!("typed struct must become a CoreValue map");
    };
    assert_eq!(
        fields.get("payload"),
        Some(&CoreValue::Bytes(vec![0, 127, 255]))
    );
    assert_eq!(fromCoreValue::<TypedValue>(value).unwrap(), source);
}

/// Verifies CoreValue map ordering is deterministic for encoded protocol values.
#[test]
fn core_value_map_encoding_is_deterministic() {
    let mut first = BTreeMap::new();
    first.insert("z".to_string(), CoreValue::Unsigned(1));
    first.insert("a".to_string(), CoreValue::Unsigned(2));

    assert_eq!(
        encodeLink(CoreValue::Map(first.clone())).unwrap(),
        encodeLink(CoreValue::Map(first)).unwrap()
    );
}

/// Verifies ordinary integer vectors remain MessagePack arrays.
#[test]
fn integer_vectors_remain_message_pack_arrays() {
    let bytes = encodeLink(vec![1_i32, 2_i32]).unwrap();

    assert_eq!(bytes, vec![0x92, 1, 2]);
}
