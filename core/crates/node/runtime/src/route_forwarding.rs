//! One forwarding loop for source and relay requests, independent of local execution.
use super::CoreNodeRouter;
use crate::space_policy::RoutingPolicy;
use crate::RuntimePeerService::RuntimePeerService;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError,
    CoreLinkPushSession, CorePushRequest, CoreWatchRequest, RoutedCoreRequest,
};
use operit_store::CoreSpaceStore::CoreSpace;
use operit_tools::tools::DeviceTools::DEVICE_TOOL_TARGET;
use std::{collections::BTreeSet, future::Future, sync::Arc};

pub(super) enum RouteParticipant {
    Previous,
    Origin,
    Target,
}

pub(super) fn requireRouteMember(
    space: &CoreSpace,
    policy: &RoutingPolicy,
    node: &str,
    role: RouteParticipant,
) -> Result<(), CoreLinkError> {
    let (label, missing, revoked) = match role {
        RouteParticipant::Previous => (
            "Previous CoreNode",
            "PREVIOUS_CORE_NODE_NOT_IN_SPACE",
            "PREVIOUS_CORE_NODE_REVOKED",
        ),
        RouteParticipant::Origin => (
            "Origin CoreNode",
            "ORIGIN_CORE_NODE_NOT_IN_SPACE",
            "ORIGIN_CORE_NODE_REVOKED",
        ),
        RouteParticipant::Target => ("CoreNode", "CORE_NODE_NOT_IN_SPACE", "CORE_NODE_REVOKED"),
    };
    if !space.members.iter().any(|member| member == node) {
        return Err(CoreLinkError::new(
            missing,
            format!("{label} is not a Space member: {node}"),
        ));
    }
    if policy.blocked.contains(node) {
        return Err(CoreLinkError::new(
            revoked,
            format!("{label} is revoked from routing: {node}"),
        ));
    }
    Ok(())
}

pub(super) fn requireRoutableTarget(target: &str) -> Result<(), CoreLinkError> {
    if target.starts_with("core/server.") {
        return Err(CoreLinkError::new(
            "LOCAL_MANAGEMENT_ONLY",
            "Node management is only available to the local application",
        ));
    }
    Ok(())
}

trait RoutedPayload: Clone {
    fn target(&self) -> &str;
}
impl RoutedPayload for CoreCallRequest {
    fn target(&self) -> &str {
        &self.target
    }
}
impl RoutedPayload for CoreWatchRequest {
    fn target(&self) -> &str {
        &self.target
    }
}
impl RoutedPayload for CorePushRequest {
    fn target(&self) -> &str {
        &self.target
    }
}

/// A missing reply does not prove a tool or firmware action never ran.
/// Source and relay submissions use the same replay policy for every operation.
pub(super) fn canRetryRouteTarget(target: &str) -> bool {
    target != DEVICE_TOOL_TARGET && !target.starts_with("edge.")
}

/// Classifies route failures; the target replay policy separately decides whether resubmission is allowed.
#[allow(non_snake_case)]
fn isRouteUnavailableError(error: &CoreLinkError) -> bool {
    matches!(
        error.code.as_str(),
        "CORE_NODE_UNREACHABLE"
            | "PEER_LINK_CLOSED"
            | "PEER_RESPONSE_CLOSED"
            | "PEER_SEND_FAILED"
            | "PEER_WATCH_SOURCE_CLOSED"
    )
}

impl CoreNodeRouter {
    async fn forwardOperation<T, R, F, Fut>(
        &self,
        previous: Option<&str>,
        request: RoutedCoreRequest<T>,
        submit: F,
    ) -> Result<R, CoreLinkError>
    where
        T: RoutedPayload,
        F: Fn(Arc<dyn RuntimePeerService>, String, RoutedCoreRequest<T>) -> Fut,
        Fut: Future<Output = Result<R, CoreLinkError>>,
    {
        let retry = canRetryRouteTarget(request.payload.target());
        let mut excluded = BTreeSet::new();
        loop {
            let (peer, next, forwarded) =
                self.forwardRouteAvoiding(previous, &excluded, request.clone())?;
            match submit(peer, next.clone(), forwarded).await {
                Err(error) if retry && isRouteUnavailableError(&error) => {
                    excluded.insert(next);
                }
                result => return result,
            }
        }
    }

    pub(super) async fn forwardCall(
        &self,
        previous: Option<&str>,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let requestId = request.payload.requestId.clone();
        let result = self
            .forwardOperation(previous, request, |peer, next, request| async move {
                peer.call(&next, request).await.result
            })
            .await;
        CoreCallResponse { requestId, result }
    }

    pub(super) async fn forwardWatchSnapshot(
        &self,
        previous: Option<&str>,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.forwardOperation(previous, request, |peer, next, request| async move {
            peer.watchSnapshot(&next, request).await
        })
        .await
    }

    pub(super) async fn forwardWatch(
        &self,
        previous: Option<&str>,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.forwardOperation(previous, request, |peer, next, request| async move {
            peer.watch(&next, request).await
        })
        .await
    }

    pub(super) async fn forwardPush(
        &self,
        previous: Option<&str>,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.forwardOperation(previous, request, |peer, next, request| async move {
            peer.openPush(&next, request).await
        })
        .await
    }
}
