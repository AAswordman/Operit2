#![allow(non_snake_case)]

use crate::EdgeNode;
use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError,
    CoreLinkPushSession, CoreLinkSharedClient, CoreWatchRequest, RoutedCoreRequest,
    RoutedCoreRequestKind,
};
use operit_node_runtime::NodeSpaceService::{
    acceptSpaceControlCall, NodeSpaceContext, NodeSpaceService, NODE_SPACE_APPROVAL_TARGET,
    NODE_SPACE_CONTROL_TARGET, NODE_SPACE_TARGET,
};
use operit_node_runtime::PeerRouter::PeerRouter;
use std::sync::{Arc, OnceLock};

/// The Edge adapter uses the shared authenticated session engine and only
/// supplies local device-service execution. It has no second pairing protocol.
pub struct EdgePeerRouter {
    nodeId: String,
    node: OnceLock<Arc<EdgeNode>>,
    space: OnceLock<Arc<NodeSpaceService>>,
}
impl EdgePeerRouter {
    pub fn new(nodeId: String) -> Arc<Self> {
        Arc::new(Self {
            nodeId,
            node: OnceLock::new(),
            space: OnceLock::new(),
        })
    }
    pub fn install(&self, node: Arc<EdgeNode>) -> Result<(), String> {
        self.node
            .set(node)
            .map_err(|_| "Edge node already installed".into())
    }
    pub fn installSpace(&self, space: Arc<NodeSpaceService>) -> Result<(), String> {
        self.space
            .set(space)
            .map_err(|_| "Edge Space services already installed".into())
    }
    fn node(&self) -> Result<Arc<EdgeNode>, CoreLinkError> {
        self.node
            .get()
            .cloned()
            .ok_or_else(|| CoreLinkError::internal("Edge services are not installed"))
    }
    fn check(&self, request: &RoutedCoreRequest<impl Clone>) -> Result<(), CoreLinkError> {
        if request.targetNodeId != self.nodeId {
            return Err(CoreLinkError::new(
                "EDGE_ROUTE_TARGET_MISMATCH",
                "Edge request targets a different node",
            ));
        }
        if request.routeKind != RoutedCoreRequestKind::Target {
            return Err(CoreLinkError::new(
                "EDGE_ROUTE_UNSUPPORTED",
                "Standalone Edge only accepts target routes",
            ));
        }
        Ok(())
    }
    fn checkSpaceOrigin(&self, previous: &str, request: &RoutedCoreRequest<CoreCallRequest>) -> Result<(), CoreLinkError> {
        if request.originNodeId != previous || (request.payload.target == NODE_SPACE_TARGET && request.ttl != 0) {
            return Err(CoreLinkError::new("PEER_DIRECT_CALL_REQUIRED", "Edge Space calls cannot be relayed"));
        }
        Ok(())
    }

}
#[async_trait(?Send)]
impl PeerRouter for EdgePeerRouter {
    fn localNodeId(&self) -> String {
        self.nodeId.clone()
    }
    async fn routedCall(
        &self,
        previous: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let id = request.payload.requestId.clone();
        if let Err(e) = self.check(&request) {
            return CoreCallResponse::err(id, e);
        }
        if request.payload.target == NODE_SPACE_TARGET
            || request.payload.target == NODE_SPACE_APPROVAL_TARGET
            || request.payload.target == NODE_SPACE_CONTROL_TARGET
        {
            // Authenticate the direct origin, not the remaining routing budget.
            // Core's same-Space control/reviewer calls carry a positive TTL even
            // on a direct connection. Pre-admission calls remain zero-hop only.
            if let Err(error) = self.checkSpaceOrigin(&previous, &request) {
                return CoreCallResponse::err(id, error);
            }
            let Some(space) = self.space.get() else {
                return CoreCallResponse::err(
                    id,
                    CoreLinkError::new(
                        "EDGE_SPACE_UNAVAILABLE",
                        "Edge Space services are not installed",
                    ),
                );
            };
            if request.payload.target == NODE_SPACE_APPROVAL_TARGET
                || request.payload.target == NODE_SPACE_CONTROL_TARGET
            {
                match space.spaceStore().space() {
                    Ok(current) if request.spaceId == current.spaceId => {}
                    Ok(_) => {
                        return CoreCallResponse::err(
                            id,
                            CoreLinkError::new(
                                "SPACE_ID_MISMATCH",
                                "Approval traffic must belong to the current Space",
                            ),
                        )
                    }
                    Err(error) => return CoreCallResponse::err(id, CoreLinkError::internal(error)),
                }
            }
            let result = if request.payload.target == NODE_SPACE_TARGET {
                space.acceptPeerSpaceCall(&previous, request.payload)
            } else if request.payload.target == NODE_SPACE_CONTROL_TARGET {
                acceptSpaceControlCall(space.as_ref(), &previous, request.payload)
            } else {
                space
                    .acceptSpaceApprovalCall(&previous, request.payload)
                    .await
            }
            .map_err(operit_node_runtime::NodeSpaceService::spaceCallError);
            return CoreCallResponse {
                requestId: id,
                result,
            };
        }
        match self.node() {
            Ok(node) => node.call(request.payload).await,
            Err(e) => CoreCallResponse::err(id, e),
        }
    }
    fn prepareRoutedCall(&self, previous: &str, request: &RoutedCoreRequest<CoreCallRequest>) -> Option<Result<Vec<u8>, CoreLinkError>> {
        if request.payload.target != NODE_SPACE_TARGET || request.payload.methodName != "snapshot" { return None; }
        let prepare = || {
            self.check(request)?;
            self.checkSpaceOrigin(previous, request)?;
            self.space.get().ok_or_else(|| CoreLinkError::new("EDGE_SPACE_UNAVAILABLE", "Edge Space services are not installed"))
        };
        let result = match prepare() {
            Ok(space) => space.encodePeerSpaceSnapshotReply(previous, &request.payload.requestId).map_err(CoreLinkError::internal),
            Err(error) => operit_link::encodeLink(operit_link::CoreLinkResponse::Call(CoreCallResponse::err(request.payload.requestId.clone(), error)))
                .map_err(|error| CoreLinkError::internal(error.to_string())),
        };
        Some(result)
    }
    /// Dispatches authenticated Space payloads directly to their typed command decoder.
    fn prepareEncodedCall(&self, previous: &str, bytes: &mut Vec<u8>) -> Option<Result<Vec<u8>, CoreLinkError>> {
        let (id, request) = operit_node_runtime::NodeSpaceService::encodedSpaceCallHeader(bytes)?;
        let control = request.payload.target == NODE_SPACE_CONTROL_TARGET
            && matches!(request.payload.methodName.as_str(), "exchange" | "exchangeBounded");
        let bounded = request.payload.methodName == "exchangeBounded";
        let applicant = request.payload.target == NODE_SPACE_TARGET
            && matches!(request.payload.methodName.as_str(), "requestJoin" | "cancelJoin" | "joinStatus");
        if !control && !applicant && (request.payload.target != NODE_SPACE_TARGET || request.payload.methodName != "observeSpaceSnapshot") { return None; }
        let prepare = || {
            if request.payload.requestId != id { return Err(CoreLinkError::new("PEER_SECURITY", "Routed correlation mismatch")); }
            self.check(&request)?;
            self.checkSpaceOrigin(previous, &request)?;
            let space = self.space.get().ok_or_else(|| CoreLinkError::new("EDGE_SPACE_UNAVAILABLE", "Edge Space services are not installed"))?;
            if control && request.spaceId != space.spaceStore().space().map_err(CoreLinkError::internal)?.spaceId {
                return Err(CoreLinkError::new("SPACE_ID_MISMATCH", "Approval traffic must belong to the current Space"));
            }
            Ok(space)
        };
        Some(match prepare() {
            Ok(space) => if control {
                if bounded { space.encodePeerSpaceBoundedControlReply(previous, bytes, id) }
                else { space.encodePeerSpaceControlReply(previous, bytes, id) }
            } else if applicant { space.encodePeerSpaceJoinReply(previous, bytes, id, &request.payload.methodName) }
            else { space.encodePeerSpaceObservationReply(previous, bytes, id) }.map_err(CoreLinkError::internal),
            Err(error) => operit_link::encodeLink(operit_link::CoreLinkResponse::Call(CoreCallResponse::err(id, error)))
                .map_err(|error| CoreLinkError::internal(error.to_string())),
        })
    }
    async fn routedWatchSnapshot(
        &self,
        _previous: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.check(&request)?;
        self.node()?.watchSnapshot(request.payload).await
    }
    async fn routedWatch(
        &self,
        _previous: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.check(&request)?;
        self.node()?.watch(request.payload).await
    }
    async fn routedOpenPush(
        &self,
        _previous: String,
        request: RoutedCoreRequest<operit_link::CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.check(&request)?;
        self.node()?.openPush(request.payload).await
    }
    fn spaceChannelScope(&self, peerNodeId: &str) -> Result<Option<String>, CoreLinkError> {
        match self.space.get() {
            Some(space) => space.spaceChannelScope(peerNodeId),
            None => Ok(None),
        }
    }
}
