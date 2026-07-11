use async_trait::async_trait;
use axum::body::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkHttpDispatcher, CoreObjectPath, CoreRequestId, CoreWatchRequest,
    LinkCallEnvelope,
};
use serde_json::json;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;

struct BenchCoreClient;

#[async_trait]
impl CoreLinkClient for BenchCoreClient {
    /// Executes a deterministic benchmark call response.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreCallResponse::ok(
            request.requestId,
            json!({
                "ok": true,
                "value": 42
            }),
        )
    }

    /// Returns a deterministic benchmark watch snapshot.
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        Ok(CoreEvent {
            requestId: Some(request.requestId),
            targetPath: request.targetPath,
            propertyName: request.propertyName,
            kind: CoreEventKind::Snapshot,
            value: json!({
                "value": 42
            }),
        })
    }

    /// Opens a benchmark watch stream with one completed event.
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        sender
            .send(CoreEvent {
                requestId: Some(request.requestId),
                targetPath: request.targetPath,
                propertyName: request.propertyName,
                kind: CoreEventKind::Completed,
                value: json!({}),
            })
            .unwrap();
        Ok(CoreEventStream::new(receiver))
    }
}

/// Builds a serialized link call envelope.
fn call_body() -> Bytes {
    let envelope = LinkCallEnvelope {
        request: CoreCallRequest::new(
            "bench-dispatch",
            CoreObjectPath::from("runtime.chat"),
            "send",
            json!({
                "message": "ping"
            }),
        ),
    };
    Bytes::from(serde_json::to_vec(&envelope).unwrap())
}

/// Registers HTTP dispatcher call benchmarks.
fn bench_dispatcher_call(criterion: &mut Criterion) {
    let runtime = build_runtime();
    let dispatcher = CoreLinkHttpDispatcher::new(BenchCoreClient);
    let body = call_body();

    criterion.bench_function("dispatcher call", |bencher| {
        bencher.iter(|| {
            runtime.block_on(async {
                let response = dispatcher.call(black_box(body.clone())).await;
                black_box(response);
            });
        });
    });
}

/// Builds a current-thread Tokio runtime for dispatcher benchmarks.
fn build_runtime() -> Runtime {
    Builder::new_current_thread().build().unwrap()
}

criterion_group!(benches, bench_dispatcher_call);
criterion_main!(benches);
