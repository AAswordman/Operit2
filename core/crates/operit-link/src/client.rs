use async_trait::async_trait;

use crate::protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CoreWatchRequest,
};

#[async_trait(?Send)]
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

#[async_trait(?Send)]
pub trait CoreLinkSharedClient {
    /// Executes a one-shot core method call through a shared client.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse;

    /// Reads the current value for a watched core path through a shared client.
    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError>;

    /// Opens a stream of events for a watched core path through a shared client.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError>;
}

/// Provides Send-safe call and watch operations to a Link transport carrier.
#[async_trait]
pub trait CoreLinkTransportClient: Send {
    /// Executes one call through the transport boundary.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse;

    /// Reads one watch snapshot through the transport boundary.
    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError>;

    /// Opens one watch stream through the transport boundary.
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError>;
}

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
