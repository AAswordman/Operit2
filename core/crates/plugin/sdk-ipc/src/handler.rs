use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream,
    CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CorePushRequest, CoreWatchRequest,
};
use std::sync::Arc;

/// Provides the existing Core Link implementation to the SDK IPC server.
///
/// This adapter does not interpret object ids, method names, arguments, or
/// return values. Those remain owned by generated Core route and proxy code.
#[async_trait(?Send)]
pub trait PluginSdkLinkTarget: Send + Sync {
    /// Forwards one Core call request without changing its Link payload.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse;

    /// Forwards one watch snapshot request without changing its Link payload.
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError>;

    /// Opens one Core watch stream without changing its Link payload.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError>;

    /// Opens one Core push stream without changing its Link payload.
    async fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;
}

/// Shared target handle used by one SDK IPC server.
pub type SharedPluginSdkLinkTarget = Arc<dyn PluginSdkLinkTarget>;

/// Adapts any existing shared Core Link client into the SDK target.
#[async_trait(?Send)]
impl<T> PluginSdkLinkTarget for T
where
    T: CoreLinkSharedClient + Send + Sync + 'static,
{
    /// Forwards a call through the existing shared Core Link implementation.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self, request).await
    }

    /// Forwards a watch snapshot through the existing shared Core Link implementation.
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self, request).await
    }

    /// Forwards a watch through the existing shared Core Link implementation.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self, request).await
    }

    /// Opens a push through the existing shared Core Link implementation.
    async fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        CoreLinkSharedClient::openPush(self, request).await
    }
}
