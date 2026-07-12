use async_trait::async_trait;
use axum::body::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkHttpDispatcher, CoreWatchRequest, LinkWatchChannelEnvelope,
    LinkWatchChannelOpenEnvelope,
};
use serde_json::json;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::mpsc;

macro_rules! core_json {
    ($($json:tt)+) => {
        operit_link::toCoreValue(json!($($json)+)).expect("benchmark JSON must convert to CoreValue")
    };
}

struct BenchWatchClient;

#[async_trait]
impl CoreLinkClient for BenchWatchClient {
    /// Executes a deterministic benchmark call response.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreCallResponse::ok(request.requestId, core_json!({}))
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
            value: core_json!({
                "ready": true
            }),
        })
    }

    /// Opens a benchmark stream containing a fixed event burst.
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        for index in 0..64 {
            sender
                .send(CoreEvent {
                    requestId: Some(request.requestId.clone()),
                    targetPath: request.targetPath.clone(),
                    propertyName: request.propertyName.clone(),
                    kind: CoreEventKind::Changed,
                    value: core_json!({
                        "index": index
                    }),
                })
                .unwrap();
        }
        sender
            .send(CoreEvent {
                requestId: Some(request.requestId),
                targetPath: request.targetPath,
                propertyName: request.propertyName,
                kind: CoreEventKind::Completed,
                value: core_json!({}),
            })
            .unwrap();
        Ok(CoreEventStream::new(receiver))
    }
}

/// Builds a serialized watch channel open envelope.
fn watch_open_body(channel_id: &str, subscription_id: &str) -> Bytes {
    let envelope = LinkWatchChannelOpenEnvelope {
        channelId: channel_id.to_string(),
        subscriptionId: subscription_id.to_string(),
        request: CoreWatchRequest::new(
            "bench-watch",
            "runtime.chat",
            "stream",
            core_json!({
                "message": "ping"
            }),
        ),
    };
    Bytes::from(serde_json::to_vec(&envelope).unwrap())
}

/// Builds a serialized watch channel event envelope.
fn watch_events_body(channel_id: &str) -> Bytes {
    let envelope = LinkWatchChannelEnvelope {
        channelId: channel_id.to_string(),
    };
    Bytes::from(serde_json::to_vec(&envelope).unwrap())
}

/// Registers watch channel dispatcher benchmarks.
fn bench_watch_channel(criterion: &mut Criterion) {
    let runtime = build_runtime();

    criterion.bench_function("watch channel open", |bencher| {
        let mut sequence = 0u64;
        bencher.iter(|| {
            sequence += 1;
            let dispatcher = CoreLinkHttpDispatcher::new(BenchWatchClient);
            let channel_id = format!("channel-{sequence}");
            let subscription_id = format!("subscription-{sequence}");
            let events_body = watch_events_body(&channel_id);
            let open_body = watch_open_body(&channel_id, &subscription_id);

            runtime.block_on(async {
                let events_response = dispatcher.watchChannelEvents(events_body).await;
                let open_response = dispatcher.watchChannelOpen(black_box(open_body)).await;
                black_box((events_response, open_response));
            });
        });
    });
}

/// Builds a current-thread Tokio runtime for watch channel benchmarks.
fn build_runtime() -> Runtime {
    Builder::new_current_thread().build().unwrap()
}

criterion_group!(benches, bench_watch_channel);
criterion_main!(benches);
