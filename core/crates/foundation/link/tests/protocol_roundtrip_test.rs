use std::collections::BTreeMap;

use operit_link::{
    decodeLink, encodeLink, encodeLinkInto, fromCoreValue, toCoreValue, CoreCallRequest,
    CoreCallResponse, CoreEvent, CoreEventKind, CorePushItem, CorePushRequest, CoreRequestId,
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

/// Verifies amortized writer growth keeps exact bytes and serializes only once.
#[test]
fn growing_writer_matches_standard_messagepack_and_reuses_output() {
    let value = core_map([
        ("text", CoreValue::String("中文😀".repeat(2000))),
        (
            "bytes",
            CoreValue::Bytes((0..65536).map(|i| i as u8).collect::<Vec<_>>().into()),
        ),
        (
            "nested",
            CoreValue::List(
                (0..100)
                    .map(|i| {
                        core_map([
                            ("id", CoreValue::Unsigned(i)),
                            ("value", CoreValue::String(format!("record-{i}"))),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]);
    let expected = rmp_serde::to_vec_named(&value).unwrap();
    let mut output = vec![0xff; 3];
    encodeLinkInto(&value, &mut output).unwrap();
    assert_eq!(output, expected);
    assert_eq!(decodeLink::<CoreValue>(&output).unwrap(), value);
    let capacity = output.capacity();
    encodeLinkInto(CoreValue::Bool(true), &mut output).unwrap();
    assert_eq!(output, [0xc3]);
    assert_eq!(output.capacity(), capacity);

    struct Counted<'a>(&'a std::cell::Cell<u32>);
    impl Serialize for Counted<'_> {
        /// Counts each serialization so sizing passes cannot consume stream handles.
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.set(self.0.get() + 1);
            serializer.serialize_str("once")
        }
    }
    let count = std::cell::Cell::new(0);
    encodeLinkInto(Counted(&count), &mut output).unwrap();
    assert_eq!(count.get(), 1);
    assert_eq!(output, rmp_serde::to_vec_named("once").unwrap());
}

/// Verifies call requests preserve dispatch fields through the Link codec.
#[test]
fn call_request_roundtrip_preserves_registry_key() {
    let request = CoreCallRequest::new(
        "roundtrip-call",
        "core/test",
        "send",
        core_map([("message", CoreValue::String("hello".to_string()))]),
    );

    let bytes = encodeLink(&request).unwrap();
    let decoded = decodeLink::<CoreCallRequest>(&bytes).unwrap();

    assert_eq!(decoded.registryKey(), "core/test::send");
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
        target: "core/test".to_string(),
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
fn push_item_roundtrip_preserves_stream_identity() {
    let item = CorePushItem {
        pushId: "push-input".to_string(),
        sequence: 7,
        args: core_map([("type", CoreValue::String("scroll".to_string()))]),
    };

    let decoded = decodeLink::<CorePushItem>(&encodeLink(&item).unwrap()).unwrap();
    assert_eq!(decoded, item);
    assert_eq!(decoded.pushId, "push-input");
    assert_eq!(decoded.sequence, 7);
}

/// Verifies native bytes use the MessagePack bin family instead of an integer array.
#[test]
fn core_value_bytes_use_message_pack_bin() {
    let bytes = encodeLink(CoreValue::Bytes(vec![1, 2, 3, 4].into())).unwrap();

    assert_eq!(bytes, vec![0xc4, 4, 1, 2, 3, 4]);
    assert_eq!(
        decodeLink::<CoreValue>(&bytes).unwrap(),
        CoreValue::Bytes(vec![1, 2, 3, 4].into())
    );
}

/// Verifies empty native bytes still use the MessagePack bin family.
#[test]
fn empty_core_value_bytes_use_message_pack_bin() {
    let bytes = encodeLink(CoreValue::Bytes(Vec::new().into())).unwrap();

    assert_eq!(bytes, vec![0xc4, 0]);
    assert_eq!(
        decodeLink::<CoreValue>(&bytes).unwrap(),
        CoreValue::Bytes(Vec::new().into())
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
        Some(&CoreValue::Bytes(vec![0, 127, 255].into()))
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

/// Rejects the removed numeric target protocol rather than guessing an object mapping.
#[test]
fn numeric_and_legacy_targets_are_rejected() {
    for field in ["target", "targetObjectId"] {
        let wire = core_map([
            ("requestId", CoreValue::String("old-sdk".into())),
            (field, CoreValue::Unsigned(5)),
            ("methodName", CoreValue::String("activatePackage".into())),
            ("args", CoreValue::emptyMap()),
        ]);
        assert!(decodeLink::<CoreCallRequest>(&encodeLink(wire).unwrap()).is_err());
    }
}

/// Makes the stable target visible as an ordinary MessagePack string field.
#[test]
fn target_is_a_named_messagepack_string() {
    let call = CoreCallRequest::new(
        "sdk",
        "core/application.packageManager",
        "activatePackage",
        CoreValue::emptyMap(),
    );
    let CoreValue::Map(wire) = decodeLink::<CoreValue>(&encodeLink(call).unwrap()).unwrap() else {
        panic!("request must be a map")
    };
    assert_eq!(
        wire.get("target"),
        Some(&CoreValue::String("core/application.packageManager".into()))
    );
    assert!(!wire.contains_key("targetObjectId"));
}
