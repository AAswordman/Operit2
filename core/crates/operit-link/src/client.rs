use async_trait::async_trait;

use crate::protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CoreWatchRequest,
};

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait CoreLinkClient {
    /// Executes a one-shot core method call and returns its serialized response.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse;

    /// Reads the current value for a watched core path without opening a stream.
    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError>;

    /// Opens a stream of events for a watched core path.
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError>;
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl<T> CoreLinkClient for Box<T>
where
    T: CoreLinkClient + Send + ?Sized,
{
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        self.as_mut().call(request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.as_mut().watchSnapshot(request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.as_mut().watch(request).await
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl<T> CoreLinkClient for Box<T>
where
    T: CoreLinkClient + ?Sized,
{
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        self.as_mut().call(request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.as_mut().watchSnapshot(request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.as_mut().watch(request).await
    }
}
