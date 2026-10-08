//! Shared authenticated peer-session routing boundary.
//! The pairing/session implementation calls this boundary; Core and Edge provide
//! different local execution backends without duplicating the access protocol.
use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError,
    CoreLinkPushSession, CorePushRequest, CoreWatchRequest, RoutedCoreRequest,
};

#[async_trait(?Send)]
pub trait PeerRouter: Send + Sync {
    fn localNodeId(&self) -> String;
    async fn routedCall(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse;
    async fn routedWatchSnapshot(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError>;
    async fn routedWatch(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError>;
    async fn routedOpenPush(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;
    /// Returns the current Space scope for a directly paired peer. `None` is
    /// the normal answer for a standalone Edge that has no Space membership.
    fn spaceChannelScope(&self, peerNodeId: &str) -> Result<Option<String>, CoreLinkError>;
}
