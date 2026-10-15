use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError,
    CoreLinkPushSession, CoreLinkSharedClient, CorePushRequest, CoreValue, CoreWatchRequest,
};
use operit_proxy_bridge::{LocalApplicationBridgeTarget, LocalApplicationSharedClient};

#[derive(Default)]
struct RecordedInput {
    requests: Mutex<Vec<CorePushRequest>>,
    values: Mutex<Vec<CoreValue>>,
    closes: Mutex<usize>,
}

struct Application(Arc<RecordedInput>);
struct Input(Arc<RecordedInput>);

#[async_trait(?Send)]
impl LocalApplicationBridgeTarget for Application {
    /// Rejects calls outside the input-only fixture contract.
    async fn callLocalApplication(&self, _: CoreCallRequest) -> CoreCallResponse {
        panic!("unexpected call")
    }

    /// Rejects snapshots outside the input-only fixture contract.
    async fn watchLocalApplicationSnapshot(
        &self,
        _: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        panic!("unexpected snapshot")
    }

    /// Rejects watches outside the input-only fixture contract.
    async fn watchLocalApplication(
        &self,
        _: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        panic!("unexpected watch")
    }

    /// Records the exact request admitted by the application object boundary.
    async fn openLocalApplicationPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.0.requests.lock().unwrap().push(request);
        Ok(Box::new(Input(self.0.clone())))
    }
}

#[async_trait]
impl CoreLinkPushSession for Input {
    /// Records each typed value in stream order.
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError> {
        self.0.values.lock().unwrap().push(value);
        Ok(())
    }

    /// Records completion of the caller-owned input stream.
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        *self.0.closes.lock().unwrap() += 1;
        Ok(())
    }
}

/// Checks target isolation and preserves binary values through an erased shared client.
#[tokio::test]
async fn shared_application_push_enforces_target_and_preserves_input() {
    let recorded = Arc::new(RecordedInput::default());
    let adapter = LocalApplicationSharedClient::new(
        Arc::new(Application(recorded.clone())),
        "core/server.application",
    );
    let shared: &dyn CoreLinkSharedClient = &adapter;
    let denied = shared
        .openPush(CorePushRequest::new("denied", "core/services.other", "input"))
        .await;
    assert!(matches!(denied, Err(error) if error.code == "LOCAL_APPLICATION_TARGET_REQUIRED"));
    assert!(recorded.requests.lock().unwrap().is_empty());

    let request = CorePushRequest::new("input", "core/server.application", "input")
        .withArgs(CoreValue::Bytes(vec![0, 255].into()));
    let mut input = shared.openPush(request.clone()).await.unwrap();
    assert_eq!(*recorded.requests.lock().unwrap(), vec![request]);
    let values = vec![
        CoreValue::Bytes(vec![1, 0, 255, 17].into()),
        CoreValue::Bytes(Vec::new().into()),
    ];
    for value in &values {
        input.send(value.clone()).await.unwrap();
    }
    input.close().await.unwrap();
    assert_eq!(*recorded.values.lock().unwrap(), values);
    assert_eq!(*recorded.closes.lock().unwrap(), 1);
}
