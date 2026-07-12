use std::collections::BTreeMap;
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use operit_link::{decodeLink, encodeLink, CoreCallRequest, CoreValue};

/// Builds a representative small Link call request.
fn small_call_request() -> CoreCallRequest {
    CoreCallRequest::new(
        "bench-small",
        "application",
        "coreVersion",
        CoreValue::emptyMap(),
    )
}

/// Builds a representative large Link call request with native bytes.
fn large_call_request() -> CoreCallRequest {
    CoreCallRequest::new(
        "bench-large",
        "services.runtimeBrowserService",
        "publishBrowserSessionEvent",
        CoreValue::Map(BTreeMap::from([(
            "frameData".to_string(),
            CoreValue::Bytes(vec![0x5a; 256 * 1024]),
        )])),
    )
}

/// Benchmarks the final MessagePack-only Link protocol codec.
fn bench_protocol_codec(criterion: &mut Criterion) {
    criterion.bench_function("link encode small call", |bencher| {
        let request = small_call_request();
        bencher.iter(|| encodeLink(black_box(&request)).unwrap());
    });
    criterion.bench_function("link decode small call", |bencher| {
        let bytes = encodeLink(small_call_request()).unwrap();
        bencher.iter(|| decodeLink::<CoreCallRequest>(black_box(&bytes)).unwrap());
    });
    criterion.bench_function("link encode native frame call", |bencher| {
        let request = large_call_request();
        bencher.iter(|| encodeLink(black_box(&request)).unwrap());
    });
    criterion.bench_function("link decode native frame call", |bencher| {
        let bytes = encodeLink(large_call_request()).unwrap();
        bencher.iter(|| decodeLink::<CoreCallRequest>(black_box(&bytes)).unwrap());
    });
}

criterion_group!(benches, bench_protocol_codec);
criterion_main!(benches);
