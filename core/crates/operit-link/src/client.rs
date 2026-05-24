use async_trait::async_trait;

use crate::protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CoreWatchRequest,
};

#[async_trait]
pub trait CoreLinkClient {
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse;

    #[allow(non_snake_case)]
    async fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError>;

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError>;
}

#[async_trait]
impl<T> CoreLinkClient for Box<T>
where
    T: CoreLinkClient + Send + ?Sized,
{
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        self.as_mut().call(request).await
    }

    #[allow(non_snake_case)]
    async fn watchSnapshot(&mut self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.as_mut().watchSnapshot(request).await
    }

    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.as_mut().watch(request).await
    }
}
