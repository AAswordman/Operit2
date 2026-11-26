use operit_node_edge::{NativeClient::NativeClient, PeerRouter::EdgePeerRouter, EdgeCallHandler};
use operit_link::{CoreCallRequest, CoreCallResponse, CoreLinkError, CoreLinkSharedClient, CorePushRequest, CoreValue, CoreWatchRequest};
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn native_services_receive_the_original_request_and_preserve_responses() {
    let received = Arc::new(Mutex::new(None));
    let capture = received.clone();
    let handler: Arc<EdgeCallHandler> = Arc::new(move |request| {
        *capture.lock().unwrap() = Some(request.clone());
        Box::pin(async move { Some(CoreCallResponse::err(request.requestId,
            CoreLinkError::new("DEVICE_BUSY", "Scene is rendering"))) })
    });
    let client = NativeClient::new(handler);
    let request = CoreCallRequest::new("frame-7", "edge.scene", "frame", CoreValue::Bytes(vec![0, 255, 128].into()));
    let response = client.call(request.clone()).await;
    assert_eq!(*received.lock().unwrap(), Some(request.clone()));
    assert_eq!(response.requestId, request.requestId);
    assert_eq!(response.result.unwrap_err().code, "DEVICE_BUSY");
}

#[tokio::test]
async fn unsupported_services_fail_explicitly_and_installation_is_single_use() {
    let client = Arc::new(NativeClient::new(Arc::new(|_| Box::pin(async { None }))));
    let request = CoreCallRequest::new("unknown-1", "unknown", "method", CoreValue::Null);
    let response = client.call(request.clone()).await;
    assert_eq!(response.requestId, request.requestId);
    assert_eq!(response.result.unwrap_err().code, "METHOD_NOT_FOUND");
    let watch = CoreWatchRequest { requestId: request.requestId, target: "unknown".into(), propertyName: "state".into(), args: CoreValue::Null };
    assert_eq!(client.watchSnapshot(watch.clone()).await.unwrap_err().code, "WATCH_NOT_FOUND");
    assert!(matches!(client.watch(watch).await, Err(error) if error.code == "WATCH_NOT_FOUND"));
    assert!(matches!(client.openPush(CorePushRequest::new("push-1", "unknown", "stream")).await,
        Err(error) if error.code == "METHOD_NOT_FOUND"));
    let router = EdgePeerRouter::new("edge".into());
    router.installClient(client.clone()).unwrap();
    assert!(router.installClient(client).is_err());
}
