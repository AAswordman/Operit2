use std::collections::BTreeMap;

use async_trait::async_trait;
use axum::body::{to_bytes, Bytes};
use operit_link::{
    decodeLink, encodeLink, CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind,
    CoreEventStream, CoreLinkClient, CoreLinkError, CoreLinkHttpDispatcher, CorePushItem,
    CorePushRequest, CoreValue, CoreWatchRequest, LinkCallEnvelope, LinkPushItemResponse,
    LinkPushOpenEnvelope, LinkPushOpenResponse, LinkWatchEnvelope,
};
use tokio::sync::mpsc;

struct TestCoreClient;

/// Builds a string-keyed CoreValue map for dispatcher tests.
fn core_map(entries: impl IntoIterator<Item = (&'static str, CoreValue)>) -> CoreValue {
    CoreValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<BTreeMap<_, _>>(),
    )
}

#[async_trait]
impl CoreLinkClient for TestCoreClient {
    /// Executes a deterministic test call response.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreCallResponse::ok(
            request.requestId,
            core_map([("echoMethod", CoreValue::String(request.methodName))]),
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
            value: core_map([("snapshot", CoreValue::Bool(true))]),
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
                value: CoreValue::emptyMap(),
            })
            .unwrap();
        Ok(CoreEventStream::new(receiver))
    }
}

/// Serializes a Link call envelope for dispatcher tests.
fn call_body() -> Bytes {
    let envelope = LinkCallEnvelope {
        request: CoreCallRequest::new("test-call", "runtime.chat", "send", CoreValue::emptyMap()),
    };
    Bytes::from(encodeLink(&envelope).unwrap())
}

/// Serializes a Link watch envelope for dispatcher tests.
fn watch_body() -> Bytes {
    let envelope = LinkWatchEnvelope {
        request: CoreWatchRequest::new(
            "test-watch",
            "runtime.chat",
            "stream",
            CoreValue::emptyMap(),
        ),
    };
    Bytes::from(encodeLink(&envelope).unwrap())
}

/// Verifies dispatcher call responses pass through the core client as MessagePack.
#[tokio::test]
async fn dispatcher_call_returns_core_response() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let response = dispatcher.call(call_body()).await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let decoded = decodeLink::<CoreCallResponse>(&body).unwrap();

    assert_eq!(
        decoded.result.unwrap(),
        core_map([("echoMethod", CoreValue::String("send".to_string()))])
    );
}

/// Verifies dispatcher watch snapshots pass through the core client as MessagePack.
#[tokio::test]
async fn dispatcher_watch_snapshot_returns_core_event() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let response = dispatcher.watchSnapshot(watch_body()).await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let decoded = decodeLink::<CoreEvent>(&body).unwrap();

    assert_eq!(decoded.kind, CoreEventKind::Snapshot);
    assert_eq!(
        decoded.value,
        core_map([("snapshot", CoreValue::Bool(true))])
    );
}

/// Verifies dispatcher push streams preserve item identity and target dispatch.
#[tokio::test]
async fn dispatcher_push_dispatches_ordered_items() {
    let dispatcher = CoreLinkHttpDispatcher::new(TestCoreClient);
    let open = LinkPushOpenEnvelope {
        pushId: "input-1".to_string(),
        request: CorePushRequest::new("input-1", "runtime.browser", "interact"),
    };
    let open_response = dispatcher
        .pushOpen(Bytes::from(encodeLink(open).unwrap()))
        .await;
    let open_body = to_bytes(open_response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        decodeLink::<LinkPushOpenResponse>(&open_body)
            .unwrap()
            .pushId,
        "input-1"
    );

    let item_response = dispatcher
        .pushItem(Bytes::from(
            encodeLink(CorePushItem {
                pushId: "input-1".to_string(),
                sequence: 0,
                args: CoreValue::emptyMap(),
            })
            .unwrap(),
        ))
        .await;
    let item_body = to_bytes(item_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let accepted = decodeLink::<LinkPushItemResponse>(&item_body).unwrap();
    assert_eq!(accepted.pushId, "input-1");
    assert_eq!(accepted.sequence, 0);

    let duplicate_response = dispatcher
        .pushItem(Bytes::from(
            encodeLink(CorePushItem {
                pushId: "input-1".to_string(),
                sequence: 0,
                args: CoreValue::emptyMap(),
            })
            .unwrap(),
        ))
        .await;
    let duplicate_body = to_bytes(duplicate_response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        decodeLink::<CoreLinkError>(&duplicate_body).unwrap().code,
        "PUSH_SEQUENCE_MISMATCH"
    );
}
