use async_trait::async_trait;
use axum::body::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures_util::future::join_all;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkHttpDispatcher, CoreLinkSharedClient, CoreWatchRequest,
    LinkCallEnvelope,
};
use serde_json::json;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;

const SEQUENTIAL_CALLS: usize = 1_000;
const CONCURRENT_CALLS: usize = 256;

#[derive(Clone)]
struct StressCoreClient;

#[derive(Clone)]
struct SharedStressCoreClient;

#[async_trait]
impl CoreLinkClient for StressCoreClient {
    /// Executes a deterministic stress call response.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreCallResponse::ok(
            request.requestId,
            json!({
                "ok": true,
                "method": request.methodName
            }),
        )
    }

    /// Returns a deterministic stress watch snapshot.
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
                "ready": true
            }),
        })
    }

    /// Opens a deterministic completed stress watch stream.
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

#[async_trait]
impl CoreLinkSharedClient for SharedStressCoreClient {
    /// Executes a deterministic shared stress call response.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        CoreCallResponse::ok(
            request.requestId,
            json!({
                "ok": true,
                "method": request.methodName
            }),
        )
    }

    /// Returns a deterministic shared stress watch snapshot.
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        Ok(CoreEvent {
            requestId: Some(request.requestId),
            targetPath: request.targetPath,
            propertyName: request.propertyName,
            kind: CoreEventKind::Snapshot,
            value: json!({
                "ready": true
            }),
        })
    }

    /// Opens a deterministic completed shared stress watch stream.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
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

/// Builds a serialized link call envelope for a stress request.
fn call_body(index: usize) -> Bytes {
    let envelope = LinkCallEnvelope {
        request: CoreCallRequest::new(
            format!("stress-call-{index}"),
            "runtime.chat",
            "send",
            json!({
                "message": "ping",
                "index": index
            }),
        ),
    };
    Bytes::from(serde_json::to_vec(&envelope).unwrap())
}

/// Builds a fixed set of serialized call bodies.
fn call_bodies(count: usize) -> Vec<Bytes> {
    (0..count).map(call_body).collect()
}

/// Builds a current-thread Tokio runtime for stress benchmarks.
fn build_runtime() -> Runtime {
    Builder::new_current_thread().build().unwrap()
}

/// Runs a sequential call burst through one dispatcher.
async fn run_sequential_call_burst(dispatcher: &CoreLinkHttpDispatcher, bodies: &[Bytes]) {
    for body in bodies {
        let response = dispatcher.call(body.clone()).await;
        black_box(response);
    }
}

/// Runs a concurrent call burst through one dispatcher.
async fn run_concurrent_call_burst(dispatcher: &CoreLinkHttpDispatcher, bodies: &[Bytes]) {
    let futures = bodies.iter().map(|body| {
        let dispatcher = dispatcher.clone();
        let body = body.clone();
        async move { dispatcher.call(body).await }
    });
    let responses = join_all(futures).await;
    black_box(responses);
}

/// Registers call pressure benchmarks for the link dispatcher.
fn bench_link_stress(criterion: &mut Criterion) {
    let runtime = build_runtime();
    let locked_dispatcher = CoreLinkHttpDispatcher::new(StressCoreClient);
    let shared_dispatcher = CoreLinkHttpDispatcher::newShared(SharedStressCoreClient);
    let sequential_bodies = call_bodies(SEQUENTIAL_CALLS);
    let concurrent_bodies = call_bodies(CONCURRENT_CALLS);

    criterion.bench_function(
        "stress sequential 1000 locked dispatcher calls",
        |bencher| {
            bencher.iter(|| {
                runtime.block_on(run_sequential_call_burst(
                    black_box(&locked_dispatcher),
                    black_box(&sequential_bodies),
                ));
            });
        },
    );

    criterion.bench_function("stress concurrent 256 locked dispatcher calls", |bencher| {
        bencher.iter(|| {
            runtime.block_on(run_concurrent_call_burst(
                black_box(&locked_dispatcher),
                black_box(&concurrent_bodies),
            ));
        });
    });

    criterion.bench_function(
        "stress sequential 1000 shared dispatcher calls",
        |bencher| {
            bencher.iter(|| {
                runtime.block_on(run_sequential_call_burst(
                    black_box(&shared_dispatcher),
                    black_box(&sequential_bodies),
                ));
            });
        },
    );

    criterion.bench_function("stress concurrent 256 shared dispatcher calls", |bencher| {
        bencher.iter(|| {
            runtime.block_on(run_concurrent_call_burst(
                black_box(&shared_dispatcher),
                black_box(&concurrent_bodies),
            ));
        });
    });
}

criterion_group!(benches, bench_link_stress);
criterion_main!(benches);
