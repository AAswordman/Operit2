use criterion::{black_box, criterion_group, criterion_main, Criterion};
use operit_link::{
    decodeCbor, decodeMessagePack, encodeCbor, encodeMessagePack, CoreCallRequest,
    CoreCallResponse, CoreEvent, CoreEventKind, CoreObjectPath, CoreRequestId,
};
use serde_json::json;

/// Builds a representative small link call request.
fn small_call_request() -> CoreCallRequest {
    CoreCallRequest::new(
        "bench-small-call",
        "runtime.chat",
        "send",
        json!({
            "message": "ping",
            "stream": false
        }),
    )
}

/// Builds a representative large link call request.
fn large_call_request() -> CoreCallRequest {
    CoreCallRequest::new(
        "bench-large-call",
        "runtime.chat",
        "send",
        json!({
            "message": "x".repeat(64 * 1024),
            "metadata": {
                "source": "criterion",
                "sequence": (0..256).collect::<Vec<_>>()
            }
        }),
    )
}

/// Builds a representative link call response.
fn call_response() -> CoreCallResponse {
    CoreCallResponse::ok(
        CoreRequestId::new("bench-response"),
        json!({
            "messageId": "response-1",
            "accepted": true
        }),
    )
}

/// Builds a representative watch event.
fn watch_event() -> CoreEvent {
    CoreEvent {
        requestId: Some(CoreRequestId::new("bench-watch")),
        targetPath: CoreObjectPath::from("runtime.chat"),
        propertyName: "stream".to_string(),
        kind: CoreEventKind::Changed,
        value: json!({
            "delta": "token",
            "index": 1
        }),
    }
}

/// Registers protocol codec benchmarks.
fn bench_protocol_codec(criterion: &mut Criterion) {
    criterion.bench_function("call request encode small", |bencher| {
        let request = small_call_request();
        bencher.iter(|| serde_json::to_vec(black_box(&request)).unwrap());
    });

    criterion.bench_function("call request encode cbor small", |bencher| {
        let request = small_call_request();
        bencher.iter(|| encodeCbor(black_box(&request)).unwrap());
    });

    criterion.bench_function("call request encode msgpack small", |bencher| {
        let request = small_call_request();
        bencher.iter(|| encodeMessagePack(black_box(&request)).unwrap());
    });

    criterion.bench_function("call request decode small", |bencher| {
        let bytes = serde_json::to_vec(&small_call_request()).unwrap();
        bencher.iter(|| serde_json::from_slice::<CoreCallRequest>(black_box(&bytes)).unwrap());
    });

    criterion.bench_function("call request decode cbor small", |bencher| {
        let bytes = encodeCbor(&small_call_request()).unwrap();
        bencher.iter(|| decodeCbor::<CoreCallRequest>(black_box(&bytes)).unwrap());
    });

    criterion.bench_function("call request decode msgpack small", |bencher| {
        let bytes = encodeMessagePack(&small_call_request()).unwrap();
        bencher.iter(|| decodeMessagePack::<CoreCallRequest>(black_box(&bytes)).unwrap());
    });

    criterion.bench_function("call request encode large", |bencher| {
        let request = large_call_request();
        bencher.iter(|| serde_json::to_vec(black_box(&request)).unwrap());
    });

    criterion.bench_function("call request encode cbor large", |bencher| {
        let request = large_call_request();
        bencher.iter(|| encodeCbor(black_box(&request)).unwrap());
    });

    criterion.bench_function("call request encode msgpack large", |bencher| {
        let request = large_call_request();
        bencher.iter(|| encodeMessagePack(black_box(&request)).unwrap());
    });

    criterion.bench_function("call response encode", |bencher| {
        let response = call_response();
        bencher.iter(|| serde_json::to_vec(black_box(&response)).unwrap());
    });

    criterion.bench_function("call response encode cbor", |bencher| {
        let response = call_response();
        bencher.iter(|| encodeCbor(black_box(&response)).unwrap());
    });

    criterion.bench_function("call response encode msgpack", |bencher| {
        let response = call_response();
        bencher.iter(|| encodeMessagePack(black_box(&response)).unwrap());
    });

    criterion.bench_function("watch event encode", |bencher| {
        let event = watch_event();
        bencher.iter(|| serde_json::to_vec(black_box(&event)).unwrap());
    });

    criterion.bench_function("watch event encode cbor", |bencher| {
        let event = watch_event();
        bencher.iter(|| encodeCbor(black_box(&event)).unwrap());
    });

    criterion.bench_function("watch event encode msgpack", |bencher| {
        let event = watch_event();
        bencher.iter(|| encodeMessagePack(black_box(&event)).unwrap());
    });
}

criterion_group!(benches, bench_protocol_codec);
criterion_main!(benches);
