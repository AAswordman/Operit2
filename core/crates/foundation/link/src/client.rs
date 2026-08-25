use async_trait::async_trait;

use crate::protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError, CorePushRequest,
    CoreValue, CoreWatchRequest,
};

#[async_trait]
pub trait CoreLinkPushSession: Send {
    /// Sends one typed input value through the opened Link stream.
    async fn send(&mut self, value: CoreValue) -> Result<(), CoreLinkError>;

    /// Closes the input stream and waits for its target method to finish.
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError>;
}

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

    /// Opens a caller-owned input stream for one schema-declared method.
    #[allow(non_snake_case)]
    async fn openPush(
        &mut self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;
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

    #[allow(non_snake_case)]
    async fn openPush(
        &mut self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.as_mut().openPush(request).await
    }
}
