#![allow(non_snake_case)]

use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError,
    CoreLinkSharedClient, CoreWatchRequest,
};
use std::sync::Arc;

/// Dispatches application-owned Link requests without entering service-object routing.
#[async_trait(?Send)]
pub trait LocalApplicationBridgeTarget: Send + Sync {
    /// Dispatches one application-owned call request.
    async fn callLocalApplication(&self, request: CoreCallRequest) -> CoreCallResponse;

    /// Reads one application-owned watch snapshot.
    async fn watchLocalApplicationSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError>;

    /// Opens one application-owned watch stream.
    async fn watchLocalApplication(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError>;

    /// Opens one application-owned input stream.
    async fn openLocalApplicationPush(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, CoreLinkError>;
}

/// Adapts one local application dispatch target into a shared Link client.
pub struct LocalApplicationSharedClient<T: LocalApplicationBridgeTarget + ?Sized> {
    target: Arc<T>,
    application_object_id: &'static str,
}

impl<T: LocalApplicationBridgeTarget + ?Sized> LocalApplicationSharedClient<T> {
    /// Creates a shared client for one application object id.
    pub fn new(target: Arc<T>, application_object_id: &'static str) -> Self {
        Self {
            target,
            application_object_id,
        }
    }

    /// Builds the target mismatch error used by application-only clients.
    fn targetError(&self, target_route: &str) -> CoreLinkError {
        CoreLinkError::new(
            "LOCAL_APPLICATION_TARGET_REQUIRED",
            format!(
                "local application client cannot dispatch object {}",
                target_route
            ),
        )
    }
}

#[async_trait(?Send)]
impl<T> CoreLinkSharedClient for LocalApplicationSharedClient<T>
where
    T: LocalApplicationBridgeTarget + ?Sized + Send + Sync,
{
    /// Dispatches one application-owned call request through the bridge target.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        if request.target != self.application_object_id {
            return CoreCallResponse::err(requestId, self.targetError(&request.target));
        }
        self.target.callLocalApplication(request).await
    }

    /// Reads one application-owned watch snapshot through the bridge target.
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        if request.target != self.application_object_id {
            return Err(self.targetError(&request.target));
        }
        self.target.watchLocalApplicationSnapshot(request).await
    }

    /// Opens one application-owned watch stream through the bridge target.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.target != self.application_object_id {
            return Err(self.targetError(&request.target));
        }
        self.target.watchLocalApplication(request).await
    }

    /// Opens an input stream after enforcing the application object boundary.
    async fn openPush(
        &self,
        request: operit_link::CorePushRequest,
    ) -> Result<Box<dyn operit_link::CoreLinkPushSession>, operit_link::CoreLinkError> {
        if request.target != self.application_object_id {
            return Err(self.targetError(&request.target));
        }
        self.target.openLocalApplicationPush(request).await
    }
}
