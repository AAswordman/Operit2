//! Node-scoped Link client usable by any typed proxy, regardless of its size or transport.
use async_trait::async_trait;
use operit_link::{CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CoreLinkSharedClient, CoreWatchRequest};
use crate::CoreNodeRouter::CoreNodeRouter;

#[derive(Clone)]
pub struct NodeClient {
    router: CoreNodeRouter,
    nodeId: String,
}
impl NodeClient {
    pub fn new(router: CoreNodeRouter, nodeId: String) -> Self { Self { router, nodeId } }
}
#[async_trait(?Send)]
impl CoreLinkSharedClient for NodeClient {
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.router.callNode(self.nodeId.clone(), request).await
    }
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.router.watchNodeSnapshot(self.nodeId.clone(), request).await
    }
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.router.watchNode(self.nodeId.clone(), request).await
    }

    /// Opens one input stream on the explicitly addressed CoreNode.
    async fn openPush(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, operit_link::CoreLinkError> {
        self.router.openPushNode(self.nodeId.clone(), request).await
    }
}
