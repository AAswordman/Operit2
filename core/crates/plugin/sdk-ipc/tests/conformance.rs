use std::sync::Arc;

use async_trait::async_trait;
use operit_host_api::PluginSdkIpcEndpoint;
use operit_host_native_plugin_sdk_ipc::MemoryPluginSdkIpcHost;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream,
    CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CorePushRequest, CoreValue, CoreWatchRequest,
};
use operit_plugin_sdk_ipc::{PluginSdkClient, PluginSdkIpcServer};

/// Provides deterministic shared Link behavior without mutable ownership or client cloning.
struct TestCoreLinkTarget;

#[async_trait(?Send)]
impl CoreLinkSharedClient for TestCoreLinkTarget {
    /// Returns the input argument through the canonical call response.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        CoreCallResponse::ok(request.requestId, request.args)
    }

    /// Returns a snapshot event for the requested Core watch path.
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        Ok(CoreEvent {
            requestId: Some(request.requestId.clone()),
            target: request.target.clone(),
            propertyName: request.propertyName,
            kind: CoreEventKind::Snapshot,
            value: request.args,
        })
    }

    /// Produces an ordered snapshot, changed, and completed event stream.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let (sender, stream) = CoreEventStream::channel();
        let target = request.target.clone();
        let propertyName = request.propertyName;
        let requestId = request.requestId;
        sender
            .send(CoreEvent {
                requestId: Some(requestId.clone()),
                target: target.clone(),
                propertyName: propertyName.clone(),
                kind: CoreEventKind::Snapshot,
                value: CoreValue::String("snapshot".to_string()),
            })
            .expect("snapshot receiver must be alive");
        sender
            .send(CoreEvent {
                requestId: Some(requestId.clone()),
                target: target.clone(),
                propertyName: propertyName.clone(),
                kind: CoreEventKind::Changed,
                value: CoreValue::String("changed".to_string()),
            })
            .expect("changed receiver must be alive");
        sender
            .send(CoreEvent {
                requestId: Some(requestId),
                target: target.clone(),
                propertyName,
                kind: CoreEventKind::Completed,
                value: CoreValue::Null,
            })
            .expect("completed receiver must be alive");
        Ok(stream)
    }

    /// Opens a push target that records no state and accepts every value.
    async fn openPush(
        &self,
        _request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Ok(Box::new(TestPushSession))
    }
}

struct TestPushSession;

#[async_trait]
impl CoreLinkPushSession for TestPushSession {
    /// Accepts one pushed value.
    async fn send(&mut self, _value: CoreValue) -> Result<(), CoreLinkError> {
        Ok(())
    }

    /// Closes the test push target.
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        Ok(())
    }
}

/// Creates a memory-host server/client pair for one test.
fn pair() -> (
    Arc<MemoryPluginSdkIpcHost>,
    PluginSdkClient,
    PluginSdkIpcServer,
) {
    let host = Arc::new(MemoryPluginSdkIpcHost::new());
    let endpoint = PluginSdkIpcEndpoint::standard();
    let server = PluginSdkIpcServer::new(
        host.clone(),
        endpoint.clone(),
        Arc::new(TestCoreLinkTarget),
        {
            let mut surface = operit_plugin_sdk_ipc::PluginSdkSurface::new();
            surface.expose("core/test", "echo");
            surface.expose("core/test", "state");
            surface.expose("core/test", "input");
            surface
        },
    );
    server.start().expect("server starts");
    let client = PluginSdkClient::connect(host.clone(), endpoint).expect("client connects");
    (host, client, server)
}

/// Exercises all Link families through a shared client and a shared-only IPC target.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forwards_call_watch_snapshot_watch_and_push() {
    let (_host, client, server) = pair();

    let call = client
        .call(CoreCallRequest::new(
            "call-1",
            "core/test",
            "echo",
            CoreValue::String("value".to_string()),
        ))
        .await;
    assert_eq!(call.result.expect("call succeeds"), CoreValue::String("value".to_string()));

    let snapshot = client
        .watchSnapshot(CoreWatchRequest::new(
            "snapshot-1",
            "core/test",
            "state",
            CoreValue::String("initial".to_string()),
        ))
        .await
        .expect("snapshot succeeds");
    assert_eq!(snapshot.kind, CoreEventKind::Snapshot);

    let mut stream = client
        .watch(CoreWatchRequest::new(
            "watch-1",
            "core/test",
            "state",
            CoreValue::emptyMap(),
        ))
        .await
        .expect("watch opens");
    assert_eq!(stream.recv().await.expect("snapshot event").kind, CoreEventKind::Snapshot);
    assert_eq!(stream.recv().await.expect("changed event").kind, CoreEventKind::Changed);
    assert_eq!(stream.recv().await.expect("completed event").kind, CoreEventKind::Completed);
    drop(stream);

    let shared: &dyn CoreLinkSharedClient = &client;
    let mut push = shared
        .openPush(CorePushRequest::new("push-1", "core/test", "input"))
        .await
        .expect("push opens");
    push.send(CoreValue::String("item".to_string()))
        .await
        .expect("push item succeeds");
    push.close().await.expect("push closes");

    server.stop().expect("server stops");
}

/// Rejects a Core route that is absent from the generated SDK surface.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_unexposed_route() {
    let (_host, client, server) = pair();
    let response = client
        .call(CoreCallRequest::new("blocked-1", "core/test", "privateMethod", CoreValue::Null))
        .await;
    assert_eq!(response.result.expect_err("route must be rejected").code, "PLUGIN_SDK_ROUTE_NOT_EXPOSED");
    server.stop().expect("server stops");
}
