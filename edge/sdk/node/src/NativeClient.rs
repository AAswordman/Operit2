//! Native-only service composition for products using the shared Edge router.
use crate::EdgeCallHandler;
use async_trait::async_trait;
use operit_link::{CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CoreLinkSharedClient, CoreLinkPushSession, CorePushRequest, CoreWatchRequest};
use std::sync::Arc;

pub struct NativeClient { call: Arc<EdgeCallHandler> }
impl NativeClient {
    pub fn new(call: Arc<EdgeCallHandler>) -> Self { Self { call } }
}
#[async_trait(?Send)]
impl CoreLinkSharedClient for NativeClient {
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let id = request.requestId.clone();
        let key = format!("{}.{}", request.target, request.methodName);
        (self.call)(request).await.unwrap_or_else(|| CoreCallResponse::err(id, CoreLinkError::methodNotFound(&key)))
    }
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        Err(CoreLinkError::watchNotFound(&request.registryKey()))
    }
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        Err(CoreLinkError::watchNotFound(&request.registryKey()))
    }
    async fn openPush(&self, request: CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Err(CoreLinkError::methodNotFound(&format!("{}.{}", request.target, request.methodName)))
    }
}
