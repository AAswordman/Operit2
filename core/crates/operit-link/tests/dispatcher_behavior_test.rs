use async_trait::async_trait;
use axum::body::{to_bytes, Bytes};
use operit_link::{
    decodeCbor, decodeMessagePack, encodeCbor, encodeMessagePack, CoreCallRequest,
    CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient, CoreLinkError,
    CoreLinkHttpDispatcher, CoreWatchRequest, LinkCallEnvelope, LinkWatchEnvelope,
};
use serde_json::json;
use tokio::sync::mpsc;

struct TestCoreClient;

#[async_trait]
impl CoreLinkClient for TestCoreClient {
    /// Executes a deterministic test call response.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreCallResponse::ok(
            request.requestId,
            json!({
                "echoMethod": request.methodName
            }),
        )
    }

    /// Returns a deterministic test watch snapshot.
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
                "snapshot": true
            }),
        })
    }

    /// Opens a deterministic completed test watch stream.
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

/// Serializes a link call envelope for dispatcher tests.
fn call_body() -> Bytes {
    let envelope = LinkCallEnvelope {
        request: CoreCallRequest::new("test-call", "runtime.chat", "send", json!({})),
    };
    Bytes::from(serde_json::to_vec(&envelope).unwrap())
}

/// Serializes a link call envelope as CBOR for dispatcher tests.
fn call_body_cbor() -> Bytes {
    let envelope = LinkCallEnvelope {
        request: CoreCallRequest::new("test-call-cbor", "runtime.chat", "send", json!({})),
    };
    Bytes::from(encodeCbor(&envelope).unwrap())
}

/// Serializes a link call envelope as MessagePack for dispatcher tests.
fn call_body_message_pack() -> Bytes {
    let envelope = LinkCallEnvelope {
        request: CoreCallRequest::new("test-call-msgpack", "runtime.chat", "send", json!({})),
    };
    Bytes::from(encodeMessagePack(&envelope).unwrap())
}

/// Serializes a link watch envelope for dispatcher tests.
fn watch_body() -> Bytes {
    let envelope = LinkWatchEnvelope {
        request: CoreWatchRequest::new("test-watch", "runtime.chat", "stream", json!({})),
    };
    Bytes::from(serde_json::to_vec(&envelope).unwrap())
}

/// Serializes a link watch envelope as CBOR for dispatcher tests.
fn watch_body_cbor() -> Bytes {
    let envelope = LinkWatchEnvelope {
        request: CoreWatchRequest::new("test-watch-cbor", "runtime.chat", "stream", json!({})),
    };
    Bytes::from(encodeCbor(&envelope).unwrap())
}

/// Serializes a link watch envelope as MessagePack for dispatcher tests.
fn watch_body_message_pack() -> Bytes {
    let envelope = LinkWatchEnvelope {
        request: CoreWatchRequest::new("test-watch-msgpack", "runtime.chat", "stream", json!({})),
    };
    Bytes::from(encodeMessagePack(&envelope).unwrap())
}

/// Verifies dispatcher call responses pass through the core client.
#[tokio::test]
async fn dispatcher_call_returns_core_response() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let response = dispatcher.call(call_body()).await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let decoded = serde_json::from_slice::<CoreCallResponse>(&body).unwrap();

    assert_eq!(decoded.result.unwrap()["echoMethod"], "send");
}

/// Verifies dispatcher CBOR call responses pass through the core client.
#[tokio::test]
async fn dispatcher_cbor_call_returns_core_response() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let response = dispatcher.callCbor(call_body_cbor()).await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let decoded = decodeCbor::<CoreCallResponse>(&body).unwrap();

    assert_eq!(decoded.result.unwrap()["echoMethod"], "send");
}

/// Verifies dispatcher MessagePack call responses pass through the core client.
#[tokio::test]
async fn dispatcher_message_pack_call_returns_core_response() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let response = dispatcher.callMessagePack(call_body_message_pack()).await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let decoded = decodeMessagePack::<CoreCallResponse>(&body).unwrap();

    assert_eq!(decoded.result.unwrap()["echoMethod"], "send");
}

/// Verifies dispatcher watch snapshots pass through the core client.
#[tokio::test]
async fn dispatcher_watch_snapshot_returns_core_event() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let response = dispatcher.watchSnapshot(watch_body()).await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let decoded = serde_json::from_slice::<CoreEvent>(&body).unwrap();

    assert_eq!(decoded.kind, CoreEventKind::Snapshot);
    assert_eq!(decoded.value["snapshot"], true);
}

/// Verifies dispatcher CBOR watch snapshots pass through the core client.
#[tokio::test]
async fn dispatcher_cbor_watch_snapshot_returns_core_event() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let response = dispatcher.watchSnapshotCbor(watch_body_cbor()).await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let decoded = decodeCbor::<CoreEvent>(&body).unwrap();

    assert_eq!(decoded.kind, CoreEventKind::Snapshot);
    assert_eq!(decoded.value["snapshot"], true);
}

/// Verifies dispatcher MessagePack watch snapshots pass through the core client.
#[tokio::test]
async fn dispatcher_message_pack_watch_snapshot_returns_core_event() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let response = dispatcher
        .watchSnapshotMessagePack(watch_body_message_pack())
        .await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let decoded = decodeMessagePack::<CoreEvent>(&body).unwrap();

    assert_eq!(decoded.kind, CoreEventKind::Snapshot);
    assert_eq!(decoded.value["snapshot"], true);
}
