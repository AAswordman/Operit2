use async_trait::async_trait;
use operit_access_runtime::CoreNodePeerLink::{
    activePeerNodeIds, peerLink, subscribePeerLinkChanges, CoreNodeBindingApplyRequest,
    CoreNodeLinkClient, PeerLinkClient, RoutedCoreRequest, RoutedCoreRequestKind,
};
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostRuntimeTaskSchedulerHost, RuntimeStorageHost};
use operit_link::route_runtime::CoreRouteRuntime;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream,
    CoreHandoffRequest, CoreHandoffResponse, CoreLinkClient, CoreLinkError, CoreLinkPushSession,
    CoreLinkSharedClient, CorePushItem, CorePushRequest, CoreValue, CoreWatchRequest,
    CORE_INTERNAL_ROUTE_OBJECT_ID, CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT,
    CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT, CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT,
    CORE_STREAM_POOL_OBJECT_ID,
};
use operit_store::CoreNodeBindingStore::{
    CoreNodeBindingChange, CoreNodeBindingChangeObserver, CoreNodeBindingCommit,
    CoreNodeBindingRecord, CoreNodeBindingStore,
};
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceStore};
use operit_store::SyncOperationStore::SyncOperation;
use operit_tools::runtime_support::{
    CoreNodeToolRuntime, RuntimeCoreNodeRouteState, RuntimeCoreNodeStatus,
};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

use crate::SpaceRuntime::SpaceRuntime;
use crate::{GeneratedCoreRoute, CORE_ROUTE_CURSOR_ARGUMENT, CORE_ROUTE_CURSOR_PROPERTY};

static CORE_NODE_ROUTER_REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Stores one push session whose CoreNode route is fixed when the stream opens.
#[derive(Clone)]
pub struct CoreNodePushTarget {
    pushId: String,
    state: Arc<Mutex<CoreNodePushState>>,
}
/// Stores the mutable sequence and session state for one routed push.
struct CoreNodePushState {
    session: Option<Box<dyn CoreLinkPushSession>>,
    nextSequence: u64,
}

/// Routes incoming Core Link traffic through CoreNode Binding and Space routing state.
#[derive(Clone)]
pub struct CoreNodeRouter {
    localCore: Arc<CoreNodeLocalRuntime>,
    bindingStore: Arc<dyn CoreNodeBindingRuntime>,
    localNodeId: String,
    spaceStore: CoreSpaceStore,
    bindingChanges: broadcast::Sender<String>,
    _bindingChangeObserver: Arc<CoreNodeBindingChangeObserver>,
}

/// Carries local Core capabilities into the server-side Space router.
#[derive(Clone)]
pub struct CoreNodeLocalRuntime {
    sharedClient: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    applicationClient: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    objectIdForSchema: Arc<dyn Fn(&str) -> Option<u32> + Send + Sync>,
    bindCoreNodeToolRuntime:
        Arc<dyn Fn(Arc<dyn CoreNodeToolRuntime>) -> Result<(), CoreLinkError> + Send + Sync>,
    handoffAtBoundary: Arc<
        dyn Fn(
                CoreHandoffRequest,
            )
                -> Pin<Box<dyn Future<Output = Result<CoreHandoffResponse, CoreLinkError>> + Send>>
            + Send
            + Sync,
    >,
    openPush: Arc<
        dyn Fn(CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>
            + Send
            + Sync,
    >,
    spaceRuntime: Arc<SpaceRuntime>,
}

impl CoreNodeLocalRuntime {
    /// Creates the server-side capability container for one local Core.
    pub fn new(
        sharedClient: Arc<dyn CoreLinkSharedClient + Send + Sync>,
        applicationClient: Arc<dyn CoreLinkSharedClient + Send + Sync>,
        runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
        objectIdForSchema: Arc<dyn Fn(&str) -> Option<u32> + Send + Sync>,
        bindCoreNodeToolRuntime: Arc<
            dyn Fn(Arc<dyn CoreNodeToolRuntime>) -> Result<(), CoreLinkError> + Send + Sync,
        >,
        handoffAtBoundary: Arc<
            dyn Fn(
                    CoreHandoffRequest,
                ) -> Pin<
                    Box<dyn Future<Output = Result<CoreHandoffResponse, CoreLinkError>> + Send>,
                > + Send
                + Sync,
        >,
        openPush: Arc<
            dyn Fn(CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>
                + Send
                + Sync,
        >,
        spaceRuntime: Arc<SpaceRuntime>,
    ) -> Self {
        Self {
            sharedClient,
            applicationClient,
            runtimeStorageHost,
            objectIdForSchema,
            bindCoreNodeToolRuntime,
            handoffAtBoundary,
            openPush,
            spaceRuntime,
        }
    }

    /// Returns the storage host owned by the local Core.
    #[allow(non_snake_case)]
    pub fn runtimeStorageHost(&self) -> Arc<dyn RuntimeStorageHost> {
        self.runtimeStorageHost.clone()
    }

    /// Resolves one generated Core schema to the local numeric object ID.
    #[allow(non_snake_case)]
    pub fn objectIdForSchema(&self, schema: &str) -> Option<u32> {
        (self.objectIdForSchema)(schema)
    }

    /// Installs the routing capability used by built-in tools.
    #[allow(non_snake_case)]
    fn bindCoreNodeToolRuntime(
        &self,
        runtime: Arc<dyn CoreNodeToolRuntime>,
    ) -> Result<(), CoreLinkError> {
        (self.bindCoreNodeToolRuntime)(runtime)
    }

    /// Starts one continuation on the Core selected by the route server.
    #[allow(non_snake_case)]
    async fn handoffAtBoundary(
        &self,
        request: CoreHandoffRequest,
    ) -> Result<CoreHandoffResponse, CoreLinkError> {
        (self.handoffAtBoundary)(request).await
    }

    /// Executes one local Core call through the local shared client.
    pub async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.sharedClient.call(request).await
    }

    /// Executes one local application call without entering the Core service dispatcher.
    pub async fn callApplication(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.applicationClient.call(request).await
    }

    /// Reads one local Core watch snapshot through the local shared client.
    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.sharedClient.watchSnapshot(request).await
    }

    /// Opens one local Core watch through the local shared client.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.sharedClient.watch(request).await
    }

    /// Opens one local Core push through the supplied capability.
    #[allow(non_snake_case)]
    pub fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        (self.openPush)(request)
    }

    /// Executes one Space call in the server-owned Space route namespace.
    async fn callSpace(&self, request: CoreCallRequest) -> CoreCallResponse {
        self.spaceRuntime.call(request).await
    }

    /// Reads one Space watch snapshot in the server-owned Space route namespace.
    async fn watchSpaceSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.spaceRuntime.watchSnapshot(request).await
    }

    /// Opens one Space watch in the server-owned Space route namespace.
    async fn watchSpace(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.spaceRuntime.watch(request).await
    }

    /// Opens one Space push in the server-owned Space route namespace.
    fn openSpacePush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        Err(CoreLinkError::new(
            "SPACE_PUSH_NOT_REGISTERED",
            format!("Space push route is not registered: {}", request.methodName),
        ))
    }
}

/// Exposes opaque persistent Binding operations required by CoreNode routing.
trait CoreNodeBindingRuntime: Send + Sync {
    /// Returns the complete Binding currently selected for one key.
    #[allow(non_snake_case)]
    fn binding(&self, key: &str) -> Result<CoreNodeBindingRecord, CoreLinkError>;

    /// Returns the CoreNode currently selected by one Binding key.
    #[allow(non_snake_case)]
    fn bindingNodeId(&self, key: &str) -> Result<String, CoreLinkError>;

    /// Atomically changes one Binding and returns its committed operation.
    #[allow(non_snake_case)]
    fn compareAndSetBinding(
        &self,
        key: &str,
        expectedNodeId: &str,
        expectedGeneration: i64,
        targetNodeId: &str,
    ) -> Result<CoreNodeBindingCommit, CoreLinkError>;

    /// Registers one observer for committed Binding keys.
    #[allow(non_snake_case)]
    fn addBindingChangeObserver(&self, observer: Arc<CoreNodeBindingChangeObserver>);

    /// Applies one directly transported Binding operation and returns its materialized record.
    #[allow(non_snake_case)]
    fn applyImmediateBindingOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<CoreNodeBindingRecord, CoreLinkError>;
}

impl CoreNodeBindingRuntime for CoreNodeBindingStore {
    /// Returns the complete persisted Binding record.
    #[allow(non_snake_case)]
    fn binding(&self, key: &str) -> Result<CoreNodeBindingRecord, CoreLinkError> {
        match CoreNodeBindingStore::bindingOptional(self, key) {
            Ok(Some(binding)) => Ok(binding),
            Ok(None) => Err(CoreLinkError::new(
                "CORE_BINDING_NOT_FOUND",
                format!("Binding does not exist: {key}"),
            )),
            Err(error) => Err(CoreLinkError::new("CORE_BINDING_READ_FAILED", error)),
        }
    }

    /// Returns the persisted CoreNode for one Binding key.
    #[allow(non_snake_case)]
    fn bindingNodeId(&self, key: &str) -> Result<String, CoreLinkError> {
        CoreNodeBindingRuntime::binding(self, key).map(|binding| binding.nodeId)
    }

    /// Commits or joins one exact Binding source transition.
    #[allow(non_snake_case)]
    fn compareAndSetBinding(
        &self,
        key: &str,
        expectedNodeId: &str,
        expectedGeneration: i64,
        targetNodeId: &str,
    ) -> Result<CoreNodeBindingCommit, CoreLinkError> {
        self.transitionGeneration(key, expectedNodeId, expectedGeneration, targetNodeId)
            .map_err(|error| CoreLinkError::new("CORE_BINDING_TRANSITION_FAILED", error))
    }

    /// Registers one observer on the persistent Binding store.
    #[allow(non_snake_case)]
    fn addBindingChangeObserver(&self, observer: Arc<CoreNodeBindingChangeObserver>) {
        self.addChangeObserver(observer);
    }

    /// Installs one directly transported Binding operation before business continuation.
    #[allow(non_snake_case)]
    fn applyImmediateBindingOperation(
        &self,
        operation: &SyncOperation,
    ) -> Result<CoreNodeBindingRecord, CoreLinkError> {
        self.applyImmediateOperation(operation)
            .map_err(|error| CoreLinkError::new("CORE_BINDING_APPLY_FAILED", error))?;
        self.binding(&operation.entityId)
            .map_err(|error| CoreLinkError::new("CORE_BINDING_READ_FAILED", error))
    }
}

impl CoreNodeRouter {
    /// Creates a router over the local Core and its runtime-owned Link Access records.
    pub fn new(localCore: CoreNodeLocalRuntime) -> Self {
        let bindingStore = Arc::new(
            CoreNodeBindingStore::new(localCore.runtimeStorageHost())
                .expect("CoreNodeRouter requires persistent Binding storage"),
        );
        Self::newWithRuntime(Arc::new(localCore), bindingStore)
    }

    /// Creates a router over one concrete local Core runtime implementation.
    #[allow(non_snake_case)]
    fn newWithRuntime(
        localCore: Arc<CoreNodeLocalRuntime>,
        bindingStore: Arc<dyn CoreNodeBindingRuntime>,
    ) -> Self {
        let localNodeId = CoreNodeIdentityStore::new(localCore.runtimeStorageHost())
            .initialize()
            .expect("CoreNodeRouter requires a CoreNode identity")
            .nodeId;
        let spaceStore = CoreSpaceStore::new(localCore.runtimeStorageHost());
        spaceStore
            .initialize()
            .expect("CoreNodeRouter requires an initialized Space");
        let (bindingChanges, _) = broadcast::channel(256);
        let bindingChangeSender = bindingChanges.clone();
        let bindingChangeObserver: Arc<CoreNodeBindingChangeObserver> = Arc::new(move |change| {
            let key = match change {
                CoreNodeBindingChange::Upsert(binding) => binding.key,
                CoreNodeBindingChange::Delete(key) => key,
            };
            let _ = bindingChangeSender.send(key);
        });
        bindingStore.addBindingChangeObserver(bindingChangeObserver.clone());
        localCore
            .bindCoreNodeToolRuntime(Arc::new(CoreNodeToolRouteRuntime {
                localNodeId: localNodeId.clone(),
                spaceStore: spaceStore.clone(),
                bindingStore: bindingStore.clone(),
            }))
            .expect("CoreNodeRouter requires tool routing state registration");
        let router = Self {
            localCore,
            bindingStore,
            localNodeId,
            spaceStore,
            bindingChanges,
            _bindingChangeObserver: bindingChangeObserver,
        };
        operit_link::installCoreRouteRuntime(Arc::new(router.clone()));
        router
    }

    /// Returns the stable identity of the CoreNode that owns this router.
    #[allow(non_snake_case)]
    pub fn localNodeId(&self) -> String {
        self.localNodeId.clone()
    }

    /// Executes one route-owned handoff at an EnhanceAI continuation boundary.
    #[allow(non_snake_case)]
    pub async fn handoffCoreAtBoundary(
        &self,
        request: CoreHandoffRequest,
    ) -> Result<(), CoreLinkError> {
        performCoreHandoff(
            &self.bindingStore,
            &self.localNodeId,
            &self.spaceStore,
            request,
        )
        .await
    }

    /// Resolves one generated Core schema through the owning local runtime.
    #[allow(non_snake_case)]
    pub fn objectIdForSchema(&self, schema: &str) -> Option<u32> {
        self.localCore.objectIdForSchema(schema)
    }

    /// Opens a push stream whose CoreNode route remains fixed for its lifetime.
    #[allow(non_snake_case)]
    pub async fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<CoreNodePushTarget, CoreLinkError> {
        let pushId = request.requestId.0.clone();
        let session = self.openPushSession(request).await?;
        Ok(CoreNodePushTarget {
            pushId,
            state: Arc::new(Mutex::new(CoreNodePushState {
                session: Some(session),
                nextSequence: 0,
            })),
        })
    }

    /// Sends one ordered item through a previously opened CoreNode push route.
    #[allow(non_snake_case)]
    pub async fn pushItem(
        &self,
        target: &CoreNodePushTarget,
        item: CorePushItem,
    ) -> Result<(), CoreLinkError> {
        if item.pushId != target.pushId {
            return Err(CoreLinkError::new(
                "PUSH_ID_MISMATCH",
                "push item does not belong to the opened CoreNode route",
            ));
        }
        let mut state = target.state.lock().await;
        if item.sequence != state.nextSequence {
            return Err(CoreLinkError::new(
                "PUSH_SEQUENCE_MISMATCH",
                format!(
                    "CoreNode push sequence is {}, expected {}",
                    item.sequence, state.nextSequence
                ),
            ));
        }
        state
            .session
            .as_mut()
            .ok_or_else(|| CoreLinkError::new("PUSH_CLOSED", "CoreNode push is already closed"))?
            .send(item.args)
            .await?;
        state.nextSequence += 1;
        Ok(())
    }

    /// Closes one CoreNode push route and waits for its target method to finish.
    #[allow(non_snake_case)]
    pub async fn closePush(&self, target: CoreNodePushTarget) -> Result<(), CoreLinkError> {
        let session =
            target.state.lock().await.session.take().ok_or_else(|| {
                CoreLinkError::new("PUSH_CLOSED", "CoreNode push is already closed")
            })?;
        session.close().await
    }

    /// Resolves generated request metadata to one concrete CoreNode id.
    async fn routeNodeId(&self, route: GeneratedCoreRoute) -> Result<String, CoreLinkError> {
        match route {
            GeneratedCoreRoute::Local => Ok(self.localNodeId.clone()),
            GeneratedCoreRoute::Binding { key, .. } => self.bindingRouteNodeId(&key),
        }
    }

    /// Resolves one Binding and requires its selected device to be reachable.
    #[allow(non_snake_case)]
    fn bindingRouteNodeId(&self, key: &str) -> Result<String, CoreLinkError> {
        let targetNodeId = self.bindingStore.bindingNodeId(key)?;
        if !self.nodeIsReachable(&targetNodeId)? {
            operit_util::AppLogger::AppLogger::w(
                "CoreNodeRouter",
                &format!(
                    "binding target unreachable; executing locally key={} target={} local={}",
                    key, targetNodeId, self.localNodeId
                ),
            );
            return Ok(self.localNodeId.clone());
        }
        Ok(targetNodeId)
    }

    /// Reports whether the active Peer Link graph currently proves one device reachable.
    #[allow(non_snake_case)]
    fn nodeIsReachable(&self, targetNodeId: &str) -> Result<bool, CoreLinkError> {
        coreNodeIsReachable(&self.localNodeId, &self.spaceStore, targetNodeId)
            .map_err(CoreLinkError::internal)
    }

    /// Opens the generic push session selected by generated CoreNode routing metadata.
    #[allow(non_snake_case)]
    async fn openPushSession(
        &self,
        mut request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let route = crate::generated_core_push_route(&request)?;
        let targetNodeId = self.routeNodeId(route).await?;
        if targetNodeId == self.localNodeId {
            return self.localCore.openPush(request);
        }
        self.openPushNode(targetNodeId, request).await
    }

    /// Builds the initial routed envelope for one explicit remote CoreNode.
    fn initialRoute<T>(
        &self,
        targetNodeId: String,
        payload: T,
    ) -> Result<RoutedCoreRequest<T>, CoreLinkError> {
        let space = self
            .spaceStore
            .initialize()
            .map_err(CoreLinkError::internal)?;
        if targetNodeId == self.localNodeId {
            return Err(CoreLinkError::new(
                "CORE_NODE_TARGET_LOCAL",
                "Explicit routed target is the local CoreNode",
            ));
        }
        if !space.members.iter().any(|member| member == &targetNodeId) {
            return Err(CoreLinkError::new(
                "CORE_NODE_NOT_IN_SPACE",
                format!("CoreNode is not a Space member: {targetNodeId}"),
            ));
        }
        let ttl = routeTtl(&space)?;
        Ok(RoutedCoreRequest {
            spaceId: space.spaceId.clone(),
            targetNodeId,
            ttl,
            routeKind: RoutedCoreRequestKind::ObjectId,
            payload,
        })
    }

    /// Builds the initial envelope for one annotation-addressed Space request.
    fn initialSpaceRoute<T>(
        &self,
        targetNodeId: String,
        payload: T,
    ) -> Result<RoutedCoreRequest<T>, CoreLinkError> {
        let mut route = self.initialRoute(targetNodeId, payload)?;
        route.routeKind = RoutedCoreRequestKind::SpaceRoute;
        Ok(route)
    }

    /// Resolves one routed hop while excluding peers already proven unable to reach the target.
    #[allow(non_snake_case)]
    fn forwardRouteAvoiding<T>(
        &self,
        previousNodeId: Option<&str>,
        excludedPeerNodeIds: &BTreeSet<String>,
        mut request: RoutedCoreRequest<T>,
    ) -> Result<(PeerLinkClient, RoutedCoreRequest<T>), CoreLinkError> {
        let space = self
            .spaceStore
            .initialize()
            .map_err(CoreLinkError::internal)?;
        if request.spaceId != space.spaceId {
            return Err(CoreLinkError::new(
                "SPACE_ID_MISMATCH",
                format!(
                    "Routed request targets Space {}, local Space is {}",
                    request.spaceId, space.spaceId
                ),
            ));
        }
        if !space
            .members
            .iter()
            .any(|member| member == &request.targetNodeId)
        {
            return Err(CoreLinkError::new(
                "CORE_NODE_NOT_IN_SPACE",
                format!("CoreNode is not a Space member: {}", request.targetNodeId),
            ));
        }
        if request.ttl == 0 {
            return Err(CoreLinkError::new(
                "CORE_NODE_ROUTE_TTL_EXHAUSTED",
                "Routed Core request TTL is exhausted",
            ));
        }
        let nextNodeId = self.nextActiveHopAvoiding(
            request.targetNodeId.clone(),
            previousNodeId,
            excludedPeerNodeIds,
        )?;
        if previousNodeId == Some(nextNodeId.as_str()) {
            return Err(CoreLinkError::new(
                "CORE_NODE_ROUTE_LOOP",
                format!("Routed Core request would return to {nextNodeId}"),
            ));
        }
        request.ttl -= 1;
        let peer = peerLink(&self.localNodeId, &nextNodeId)
            .map_err(|error| CoreLinkError::new("PEER_LINK_CLOSED", error))?;
        Ok((peer, request))
    }

    /// Resolves one route through the CoreNode's currently active direct Peer Links.
    #[allow(non_snake_case)]
    fn nextActiveHop(
        &self,
        targetNodeId: String,
        previousNodeId: Option<&str>,
    ) -> Result<String, CoreLinkError> {
        self.nextActiveHopAvoiding(targetNodeId, previousNodeId, &BTreeSet::new())
    }

    /// Resolves one active first hop after removing previous and failed adjacent devices.
    #[allow(non_snake_case)]
    fn nextActiveHopAvoiding(
        &self,
        targetNodeId: String,
        previousNodeId: Option<&str>,
        excludedPeerNodeIds: &BTreeSet<String>,
    ) -> Result<String, CoreLinkError> {
        let mut peers = activePeerNodeIds(&self.localNodeId).map_err(CoreLinkError::internal)?;
        if let Some(previousNodeId) = previousNodeId {
            peers.remove(previousNodeId);
        }
        for peerNodeId in excludedPeerNodeIds {
            peers.remove(peerNodeId);
        }
        let activePeers = peers.iter().cloned().collect::<Vec<_>>();
        let nextHop = self
            .spaceStore
            .reachableNextHopThroughPeers(targetNodeId.clone(), peers)
            .map_err(CoreLinkError::internal)?;
        if nextHop.is_none() {
            operit_util::AppLogger::AppLogger::w(
                "CoreNodeRouter",
                &format!(
                    "no active route source={} target={} previous={:?} peers={:?}",
                    self.localNodeId, targetNodeId, previousNodeId, activePeers
                ),
            );
        }
        nextHop.ok_or_else(|| {
            CoreLinkError::new(
                "CORE_NODE_UNREACHABLE",
                format!("Device is not reachable in the current device space: {targetNodeId}"),
            )
        })
    }

    /// Validates an incoming routed envelope and reports whether this CoreNode is the target.
    fn validateIncomingRoute<T>(
        &self,
        previousNodeId: &str,
        request: &RoutedCoreRequest<T>,
    ) -> Result<bool, CoreLinkError> {
        let space = self
            .spaceStore
            .initialize()
            .map_err(CoreLinkError::internal)?;
        if request.spaceId != space.spaceId {
            return Err(CoreLinkError::new(
                "SPACE_ID_MISMATCH",
                format!(
                    "Routed request targets Space {}, local Space is {}",
                    request.spaceId, space.spaceId
                ),
            ));
        }
        if !space.members.iter().any(|member| member == previousNodeId) {
            return Err(CoreLinkError::new(
                "PREVIOUS_CORE_NODE_NOT_IN_SPACE",
                format!("Previous CoreNode is not a Space member: {previousNodeId}"),
            ));
        }
        if !space
            .members
            .iter()
            .any(|member| member == &request.targetNodeId)
        {
            return Err(CoreLinkError::new(
                "CORE_NODE_NOT_IN_SPACE",
                format!("CoreNode is not a Space member: {}", request.targetNodeId),
            ));
        }
        Ok(request.targetNodeId == self.localNodeId)
    }

    /// Executes one call on an explicit target CoreNode.
    #[allow(non_snake_case)]
    pub async fn callNode(
        &self,
        targetNodeId: String,
        request: CoreCallRequest,
    ) -> CoreCallResponse {
        self.callNodeWithKind(targetNodeId, request, RoutedCoreRequestKind::ObjectId)
            .await
    }

    /// Executes one call on an explicit CoreNode with a selected request namespace.
    async fn callNodeWithKind(
        &self,
        targetNodeId: String,
        request: CoreCallRequest,
        routeKind: RoutedCoreRequestKind,
    ) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let route = match if routeKind == RoutedCoreRequestKind::SpaceRoute {
            self.initialSpaceRoute(targetNodeId, request)
        } else {
            self.initialRoute(targetNodeId, request)
        } {
            Ok(value) => value,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                match self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone()) {
                    Ok(value) => value,
                    Err(error) => return CoreCallResponse::err(requestId, error),
                };
            let peerNodeId = peer.peerNodeId();
            let response = peer.routedCall(forwardedRoute).await;
            if response
                .result
                .as_ref()
                .err()
                .map(isRouteUnavailableError)
                .unwrap_or(false)
            {
                excludedPeerNodeIds.insert(peerNodeId);
                continue;
            }
            return response;
        }
    }

    /// Executes one annotation-addressed Space call through Binding-selected routing.
    pub async fn callSpace(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let route = match crate::generated_space_call_route(&request) {
            Some(route) => route,
            None => {
                return CoreCallResponse::err(
                    requestId,
                    CoreLinkError::new(
                        "SPACE_ROUTE_NOT_FOUND",
                        "Space call route is not registered",
                    ),
                )
            }
        };
        let bindingKey = match route.bindingKey(&request.args) {
            Ok(value) => value,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        let targetNodeId = match self
            .routeNodeId(GeneratedCoreRoute::Binding {
                scope: 0,
                key: bindingKey,
            })
            .await
        {
            Ok(value) => value,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        if targetNodeId == self.localNodeId {
            return self.localCore.callSpace(request).await;
        }
        self.callNodeWithKind(targetNodeId, request, RoutedCoreRequestKind::SpaceRoute)
            .await
    }

    /// Reads one watch snapshot on an explicit target CoreNode.
    #[allow(non_snake_case)]
    pub async fn watchNodeSnapshot(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let route = self.initialRoute(targetNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatchSnapshot(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Opens one watch on an explicit target CoreNode.
    #[allow(non_snake_case)]
    pub async fn watchNode(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let route = self.initialRoute(targetNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatch(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Reads one annotation-addressed Space watch snapshot through Binding routing.
    pub async fn watchSpaceSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let route = crate::generated_space_watch_route(&request).ok_or_else(|| {
            CoreLinkError::new(
                "SPACE_ROUTE_NOT_FOUND",
                "Space watch route is not registered",
            )
        })?;
        let bindingKey = route.bindingKey(&request.args)?;
        let targetNodeId = self
            .routeNodeId(GeneratedCoreRoute::Binding {
                scope: 0,
                key: bindingKey,
            })
            .await?;
        if targetNodeId == self.localNodeId {
            return self.localCore.watchSpaceSnapshot(request).await;
        }
        self.watchNodeSnapshotSpace(targetNodeId, request).await
    }

    /// Opens one annotation-addressed Space watch through Binding routing.
    pub async fn watchSpace(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if request.targetObjectId == CORE_STREAM_POOL_OBJECT_ID {
            return self.watchSpaceEmbeddedStream(request).await;
        }
        let route = crate::generated_space_watch_route(&request).ok_or_else(|| {
            CoreLinkError::new(
                "SPACE_ROUTE_NOT_FOUND",
                "Space watch route is not registered",
            )
        })?;
        let bindingKey = route.bindingKey(&request.args)?;
        let targetNodeId = self
            .routeNodeId(GeneratedCoreRoute::Binding {
                scope: 0,
                key: bindingKey,
            })
            .await?;
        if targetNodeId == self.localNodeId {
            return self.localCore.watchSpace(request).await;
        }
        self.watchNodeSpace(targetNodeId, request).await
    }

    /// Opens one embedded stream on the CoreNode selected by its producing route.
    #[allow(non_snake_case)]
    async fn watchSpaceEmbeddedStream(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let route = embeddedStreamSourceRoute(&request)?;
        let bindingKey = route.bindingKey(embeddedStreamSourceArgs(&request)?)?;
        let targetNodeId = self
            .routeNodeId(GeneratedCoreRoute::Binding {
                scope: 0,
                key: bindingKey,
            })
            .await?;
        if targetNodeId == self.localNodeId {
            return self.localCore.watchSpace(request).await;
        }
        self.watchNodeSpace(targetNodeId, request).await
    }

    /// Opens one push on an explicit target CoreNode.
    #[allow(non_snake_case)]
    pub async fn openPushNode(
        &self,
        targetNodeId: String,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let route = self.initialRoute(targetNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedOpenPush(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Reads one Space watch snapshot on an explicit CoreNode.
    async fn watchNodeSnapshotSpace(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let route = self.initialSpaceRoute(targetNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatchSnapshot(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Opens one Space watch on an explicit CoreNode.
    async fn watchNodeSpace(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let route = self.initialSpaceRoute(targetNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatch(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Opens one Space push on an explicit CoreNode.
    async fn openPushNodeSpace(
        &self,
        targetNodeId: String,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let route = self.initialSpaceRoute(targetNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedOpenPush(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Opens one annotation-addressed Space push through Binding routing.
    pub async fn openSpacePush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let route = crate::generated_space_push_route(&request).ok_or_else(|| {
            CoreLinkError::new(
                "SPACE_ROUTE_NOT_FOUND",
                "Space push route is not registered",
            )
        })?;
        let bindingKey = route.bindingKey(&request.args)?;
        let targetNodeId = self
            .routeNodeId(GeneratedCoreRoute::Binding {
                scope: 0,
                key: bindingKey,
            })
            .await?;
        if targetNodeId == self.localNodeId {
            return self.localCore.openSpacePush(request);
        }
        self.openPushNodeSpace(targetNodeId, request).await
    }

    /// Executes one generated call directly on this CoreNode after Binding validation.
    #[allow(non_snake_case)]
    async fn executeLocalCall(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let response = operit_link::withCoreForceLocal(self.localCore.call(request)).await;
        if response.requestId != requestId {
            return CoreCallResponse::err(
                requestId,
                CoreLinkError::new(
                    "CORE_REQUEST_ID_MISMATCH",
                    "Local Core response request id mismatch",
                ),
            );
        }
        response
    }

    /// Verifies every locally executed Binding call against its current target.
    #[allow(non_snake_case)]
    async fn validateLocalCall(&self, request: &CoreCallRequest) -> Result<(), CoreLinkError> {
        let bindingKey = match crate::generated_core_call_route(request)? {
            GeneratedCoreRoute::Binding { key, .. } => Some(key),
            GeneratedCoreRoute::Local => None,
        };
        let Some(key) = bindingKey else {
            return Ok(());
        };
        let binding = self.bindingStore.bindingNodeId(&key)?;
        if binding != self.localNodeId {
            return Err(CoreLinkError::new(
                "CORE_BINDING_TARGET_MISMATCH",
                format!(
                    "CoreNode {} cannot execute Binding {key}; selected node is {binding}",
                    self.localNodeId
                ),
            ));
        }
        Ok(())
    }

    /// Opens one Binding watch segment on an explicit CoreNode.
    #[allow(non_snake_case)]
    pub(crate) async fn watchBindingNode(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if crate::generated_space_watch_route(&request).is_some() {
            if targetNodeId == self.localNodeId {
                operit_util::AppLogger::AppLogger::i(
                    "CoreNodeRouteTrace",
                    &format!(
                        "binding_watch_open_local_space requestId={} property={} local={}",
                        request.requestId.0, request.propertyName, self.localNodeId
                    ),
                );
                return self.localCore.watchSpace(request).await;
            }
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_watch_open_remote_space requestId={} property={} local={} target={}",
                    request.requestId.0, request.propertyName, self.localNodeId, targetNodeId
                ),
            );
            return self.watchNodeSpace(targetNodeId, request).await;
        }
        if targetNodeId == self.localNodeId {
            return operit_link::withCoreForceLocal(self.localCore.watch(request)).await;
        }
        self.watchNode(targetNodeId, request).await
    }

    /// Returns one logical Binding watch while recovery runs outside the caller's open operation.
    #[allow(non_snake_case)]
    async fn openRecoveringBindingWatch(
        &self,
        key: String,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let mut bindingChanges = self.bindingChanges.subscribe();
        let mut peerChanges = subscribePeerLinkChanges();
        let (cancelSender, mut cancelReceiver) = tokio::sync::oneshot::channel();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let router = self.clone();
        let taskKey = key.clone();
        HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(
            defaultHostRuntimeTaskSchedulerHost().as_ref(),
            "core-node-binding-watch",
            Box::new(move || {
                Box::pin(async move {
                    match openBindingWatchSourceUntilReachable(
                        &router,
                        &taskKey,
                        &request,
                        None,
                        &mut bindingChanges,
                        &mut peerChanges,
                        &mut cancelReceiver,
                    )
                    .await
                    {
                        Ok(Some((sourceBinding, stream))) => {
                            runRecoveringBindingWatch(
                                router,
                                taskKey,
                                request,
                                sourceBinding,
                                stream,
                                bindingChanges,
                                peerChanges,
                                sender,
                                cancelReceiver,
                            )
                            .await;
                        }
                        Ok(None) => {}
                        Err(error) => {
                            operit_util::AppLogger::AppLogger::e(
                                "CoreNodeRouteTrace",
                                &format!(
                                    "binding_watch_initial_source_failed requestId={} property={} key={} local={} code={} error={}",
                                    request.requestId.0,
                                    request.propertyName,
                                    taskKey,
                                    router.localNodeId,
                                    error.code,
                                    error
                                ),
                            );
                        }
                    }
                })
            }),
        )
        .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(CoreEventStream::new(receiver).withOnClose(move || {
            let _ = cancelSender.send(());
        }))
    }

    /// Returns one logical Binding watch beginning with a wrapper-opened local source stream.
    #[allow(non_snake_case)]
    async fn openRecoveringBindingWatchWithLocalSource(
        &self,
        key: String,
        request: CoreWatchRequest,
        stream: CoreEventStream,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let currentBinding = self.bindingStore.binding(&key)?;
        let sourceBinding = CoreNodeBindingRecord {
            key: key.clone(),
            nodeId: self.localNodeId.clone(),
            generation: currentBinding.generation,
        };
        let bindingChanges = self.bindingChanges.subscribe();
        let peerChanges = subscribePeerLinkChanges();
        let (cancelSender, cancelReceiver) = tokio::sync::oneshot::channel();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let router = self.clone();
        HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(
            defaultHostRuntimeTaskSchedulerHost().as_ref(),
            "core-node-binding-watch-local-source",
            Box::new(move || {
                Box::pin(async move {
                    runRecoveringBindingWatch(
                        router,
                        key,
                        request,
                        sourceBinding,
                        stream,
                        bindingChanges,
                        peerChanges,
                        sender,
                        cancelReceiver,
                    )
                    .await;
                })
            }),
        )
        .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(CoreEventStream::new(receiver).withOnClose(move || {
            let _ = cancelSender.send(());
        }))
    }

    /// Applies one committed Binding operation on its exact selected CoreNode.
    #[allow(non_snake_case)]
    async fn applyBindingCommitAtNode(
        &self,
        targetNodeId: String,
        request: CoreNodeBindingApplyRequest,
    ) -> Result<(), CoreLinkError> {
        if targetNodeId == self.localNodeId {
            return self.executeBindingApply(request);
        }
        let route = self.initialRoute(targetNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedBindingApply(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Materializes one directly transported Binding operation on the local CoreNode.
    #[allow(non_snake_case)]
    fn executeBindingApply(
        &self,
        request: CoreNodeBindingApplyRequest,
    ) -> Result<(), CoreLinkError> {
        let binding = self
            .bindingStore
            .applyImmediateBindingOperation(&request.operation)?;
        if binding.key != request.bindingKey
            || binding.nodeId != request.nodeId
            || binding.generation != request.generation
            || binding.nodeId != self.localNodeId
        {
            return Err(CoreLinkError::new(
                "CORE_BINDING_APPLY_MISMATCH",
                format!(
                    "Binding apply expected key={} node={} generation={}, applied key={} node={} generation={}",
                    request.bindingKey,
                    request.nodeId,
                    request.generation,
                    binding.key,
                    binding.nodeId,
                    binding.generation
                ),
            ));
        }
        Ok(())
    }
}

/// Executes one route-owned continuation transition and commits the target owner.
async fn performCoreHandoff(
    bindingStore: &Arc<dyn CoreNodeBindingRuntime>,
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    request: CoreHandoffRequest,
) -> Result<(), CoreLinkError> {
    let binding = bindingStore.binding(&request.bindingKey)?;
    if binding.nodeId != localNodeId {
        return Err(CoreLinkError::new(
            "CORE_HANDOFF_SOURCE_MISMATCH",
            format!(
                "Binding {} is owned by {}, not {}",
                request.bindingKey, binding.nodeId, localNodeId
            ),
        ));
    }
    if request.targetNodeId.trim().is_empty() {
        return Err(CoreLinkError::new(
            "CORE_HANDOFF_TARGET_REQUIRED",
            "Core handoff target must not be empty",
        ));
    }
    if request.targetNodeId == localNodeId {
        return Err(CoreLinkError::new(
            "CORE_HANDOFF_TARGET_LOCAL",
            "Core handoff target must differ from the current owner",
        ));
    }
    if !spaceStore
        .contains(request.targetNodeId.clone())
        .map_err(CoreLinkError::internal)?
    {
        return Err(CoreLinkError::new(
            "CORE_HANDOFF_TARGET_NOT_IN_SPACE",
            format!(
                "Core handoff target is not a Space member: {}",
                request.targetNodeId
            ),
        ));
    }
    if !coreNodeIsReachable(localNodeId, spaceStore, &request.targetNodeId)
        .map_err(CoreLinkError::internal)?
    {
        return Err(CoreLinkError::new(
            "CORE_HANDOFF_TARGET_UNREACHABLE",
            format!(
                "Core handoff target is not reachable: {}",
                request.targetNodeId
            ),
        ));
    }
    let space = spaceStore.initialize().map_err(CoreLinkError::internal)?;
    let ttl = routeTtl(&space)?;
    let routedRequest = RoutedCoreRequest {
        spaceId: space.spaceId.clone(),
        targetNodeId: request.targetNodeId.clone(),
        ttl,
        routeKind: RoutedCoreRequestKind::ObjectId,
        payload: request.clone(),
    };
    routeHandoffNode(localNodeId, spaceStore, routedRequest).await?;
    let commit = bindingStore.compareAndSetBinding(
        &request.bindingKey,
        &binding.nodeId,
        binding.generation,
        &request.targetNodeId,
    )?;
    routeBindingCommitNode(
        localNodeId,
        spaceStore,
        request.targetNodeId.clone(),
        CoreNodeBindingApplyRequest {
            bindingKey: commit.binding.key.clone(),
            nodeId: commit.binding.nodeId.clone(),
            generation: commit.binding.generation,
            operation: commit.operation.clone(),
        },
    )
    .await?;
    Ok(())
}

/// Routes one handoff start request through the active Space Peer Links.
async fn routeHandoffNode(
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    mut request: RoutedCoreRequest<CoreHandoffRequest>,
) -> Result<CoreHandoffResponse, CoreLinkError> {
    let mut excludedPeerNodeIds = BTreeSet::new();
    loop {
        let peers = activePeerNodeIds(localNodeId).map_err(CoreLinkError::internal)?;
        let nextNodeId = spaceStore
            .reachableNextHopThroughPeers(
                request.targetNodeId.clone(),
                peers
                    .into_iter()
                    .filter(|nodeId| !excludedPeerNodeIds.contains(nodeId))
                    .collect(),
            )
            .map_err(CoreLinkError::internal)?
            .ok_or_else(|| {
                CoreLinkError::new("CORE_NODE_UNREACHABLE", "Core handoff route is unavailable")
            })?;
        let peer = peerLink(localNodeId, &nextNodeId)
            .map_err(|error| CoreLinkError::new("PEER_LINK_CLOSED", error))?;
        request.ttl = request.ttl.checked_sub(1).ok_or_else(|| {
            CoreLinkError::new(
                "CORE_NODE_ROUTE_TTL_EXHAUSTED",
                "Core handoff route TTL is exhausted",
            )
        })?;
        match peer.routedHandoff(request.clone()).await {
            Ok(response) => return Ok(response),
            Err(error) if isRouteUnavailableError(&error) => {
                excludedPeerNodeIds.insert(nextNodeId);
            }
            Err(error) => return Err(error),
        }
    }
}

/// Materializes one committed Binding on the selected target before continuation starts.
async fn routeBindingCommitNode(
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    targetNodeId: String,
    request: CoreNodeBindingApplyRequest,
) -> Result<(), CoreLinkError> {
    let space = spaceStore.initialize().map_err(CoreLinkError::internal)?;
    let mut routedRequest = RoutedCoreRequest {
        spaceId: space.spaceId.clone(),
        targetNodeId,
        ttl: routeTtl(&space)?,
        routeKind: RoutedCoreRequestKind::ObjectId,
        payload: request,
    };
    let mut excludedPeerNodeIds = BTreeSet::new();
    loop {
        let peers = activePeerNodeIds(localNodeId).map_err(CoreLinkError::internal)?;
        let nextNodeId = spaceStore
            .reachableNextHopThroughPeers(
                routedRequest.targetNodeId.clone(),
                peers
                    .into_iter()
                    .filter(|nodeId| !excludedPeerNodeIds.contains(nodeId))
                    .collect(),
            )
            .map_err(CoreLinkError::internal)?
            .ok_or_else(|| {
                CoreLinkError::new(
                    "CORE_NODE_UNREACHABLE",
                    "Binding commit route is unavailable",
                )
            })?;
        let peer = peerLink(localNodeId, &nextNodeId)
            .map_err(|error| CoreLinkError::new("PEER_LINK_CLOSED", error))?;
        routedRequest.ttl = routedRequest.ttl.checked_sub(1).ok_or_else(|| {
            CoreLinkError::new(
                "CORE_NODE_ROUTE_TTL_EXHAUSTED",
                "Binding commit route TTL is exhausted",
            )
        })?;
        match peer.routedBindingApply(routedRequest.clone()).await {
            Ok(()) => return Ok(()),
            Err(error) if isRouteUnavailableError(&error) => {
                excludedPeerNodeIds.insert(nextNodeId);
            }
            Err(error) => return Err(error),
        }
    }
}

/// Opens the target continuation stream through the active Space Peer Links.
async fn routeWatchNode(
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    targetNodeId: String,
    request: CoreWatchRequest,
) -> Result<CoreEventStream, CoreLinkError> {
    let space = spaceStore.initialize().map_err(CoreLinkError::internal)?;
    let mut routedRequest = RoutedCoreRequest {
        spaceId: space.spaceId.clone(),
        targetNodeId,
        ttl: routeTtl(&space)?,
        routeKind: RoutedCoreRequestKind::ObjectId,
        payload: request,
    };
    let mut excludedPeerNodeIds = BTreeSet::new();
    loop {
        let peers = activePeerNodeIds(localNodeId).map_err(CoreLinkError::internal)?;
        let nextNodeId = spaceStore
            .reachableNextHopThroughPeers(
                routedRequest.targetNodeId.clone(),
                peers
                    .into_iter()
                    .filter(|nodeId| !excludedPeerNodeIds.contains(nodeId))
                    .collect(),
            )
            .map_err(CoreLinkError::internal)?
            .ok_or_else(|| {
                CoreLinkError::new("CORE_NODE_UNREACHABLE", "Core watch route is unavailable")
            })?;
        let peer = peerLink(localNodeId, &nextNodeId)
            .map_err(|error| CoreLinkError::new("PEER_LINK_CLOSED", error))?;
        routedRequest.ttl = routedRequest.ttl.checked_sub(1).ok_or_else(|| {
            CoreLinkError::new(
                "CORE_NODE_ROUTE_TTL_EXHAUSTED",
                "Core watch route TTL is exhausted",
            )
        })?;
        match peer.routedWatch(routedRequest.clone()).await {
            Ok(stream) => return Ok(stream),
            Err(error) if isRouteUnavailableError(&error) => {
                excludedPeerNodeIds.insert(nextNodeId);
            }
            Err(error) => return Err(error),
        }
    }
}

/// Supplies built-in tools with the same live reachability view used by the router.
struct CoreNodeToolRouteRuntime {
    localNodeId: String,
    spaceStore: CoreSpaceStore,
    bindingStore: Arc<dyn CoreNodeBindingRuntime>,
}

impl CoreNodeToolRuntime for CoreNodeToolRouteRuntime {
    /// Returns every device-space member with current route reachability.
    #[allow(non_snake_case)]
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String> {
        let space = self.spaceStore.initialize()?;
        let profiles = self.spaceStore.deviceProfiles()?;
        let peers = activePeerNodeIds(&self.localNodeId)?;
        let mut nodes = Vec::with_capacity(space.members.len());
        for nodeId in space.members {
            let profile = profiles.get(&nodeId).ok_or_else(|| {
                format!("Device profile is missing in the current device space: {nodeId}")
            })?;
            nodes.push(RuntimeCoreNodeStatus {
                displayName: profile.displayName.clone(),
                userName: profile.userName.clone(),
                platform: profile.platform.clone(),
                model: profile.model.clone(),
                reachable: coreNodeIsReachableThroughPeers(
                    &self.localNodeId,
                    &self.spaceStore,
                    &nodeId,
                    &peers,
                )?,
                nodeId,
            });
        }
        Ok(RuntimeCoreNodeRouteState {
            currentNodeId: self.localNodeId.clone(),
            nodes,
        })
    }

    /// Executes one route-owned handoff after EnhanceAI reaches a continuation boundary.
    #[allow(non_snake_case)]
    fn handoffCoreAtBoundary<'a>(
        &'a self,
        request: CoreHandoffRequest,
    ) -> operit_tools::runtime_support::ToolRuntimeSupportFuture<'a, Result<(), String>> {
        Box::pin(async move {
            performCoreHandoff(
                &self.bindingStore,
                &self.localNodeId,
                &self.spaceStore,
                request,
            )
            .await
            .map_err(|error| error.to_string())
        })
    }
}

/// Reports whether the active Peer Link graph currently proves one device reachable.
#[allow(non_snake_case)]
fn coreNodeIsReachable(
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    targetNodeId: &str,
) -> Result<bool, String> {
    if targetNodeId == localNodeId {
        return Ok(true);
    }
    let peers = activePeerNodeIds(localNodeId)?;
    coreNodeIsReachableThroughPeers(localNodeId, spaceStore, targetNodeId, &peers)
}

/// Reports device reachability through one fixed active Peer Link snapshot.
#[allow(non_snake_case)]
fn coreNodeIsReachableThroughPeers(
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    targetNodeId: &str,
    peers: &BTreeSet<String>,
) -> Result<bool, String> {
    if targetNodeId == localNodeId {
        return Ok(true);
    }
    if !spaceStore.contains(targetNodeId.to_string())? {
        return Ok(false);
    }
    spaceStore
        .reachableNextHopThroughPeers(targetNodeId.to_string(), peers.clone())
        .map(|nextHop| nextHop.is_some())
}

/// Opens the current Binding source while waiting for a temporary route outage to clear.
#[allow(non_snake_case)]
async fn openBindingWatchSourceUntilReachable(
    router: &CoreNodeRouter,
    key: &str,
    request: &CoreWatchRequest,
    cursor: Option<CoreValue>,
    bindingChanges: &mut broadcast::Receiver<String>,
    peerChanges: &mut broadcast::Receiver<()>,
    cancelReceiver: &mut tokio::sync::oneshot::Receiver<()>,
) -> Result<Option<(CoreNodeBindingRecord, CoreEventStream)>, CoreLinkError> {
    loop {
        let binding = router.bindingStore.binding(key)?;
        if !router.nodeIsReachable(&binding.nodeId)? {
            operit_util::AppLogger::AppLogger::w(
                "CoreNodeRouter",
                &format!(
                    "binding watch target unreachable; opening local source key={} target={} local={}",
                    key, binding.nodeId, router.localNodeId
                ),
            );
            let nextRequest = watchRequestWithRouteState(request, cursor.clone())?;
            let stream = router
                .watchBindingNode(router.localNodeId.clone(), nextRequest)
                .await?;
            let localBinding = CoreNodeBindingRecord {
                key: key.to_string(),
                nodeId: router.localNodeId.clone(),
                generation: binding.generation,
            };
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_watch_source_opened_local requestId={} property={} key={} local={} selected={} generation={}",
                    request.requestId.0,
                    request.propertyName,
                    key,
                    router.localNodeId,
                    binding.nodeId,
                    binding.generation
                ),
            );
            return Ok(Some((localBinding, stream)));
        }
        let targetNodeId = binding.nodeId.clone();
        let nextRequest = watchRequestWithRouteState(request, cursor.clone())?;
        match router.watchBindingNode(targetNodeId, nextRequest).await {
            Ok(stream) => {
                operit_util::AppLogger::AppLogger::i(
                    "CoreNodeRouteTrace",
                    &format!(
                        "binding_watch_source_opened requestId={} property={} key={} local={} source={} generation={}",
                        request.requestId.0,
                        request.propertyName,
                        key,
                        router.localNodeId,
                        binding.nodeId,
                        binding.generation
                    ),
                );
                return Ok(Some((binding, stream)));
            }
            Err(error) if isRouteUnavailableError(&error) => {
                tokio::select! {
                    biased;
                    _ = &mut *cancelReceiver => return Ok(None),
                    change = bindingChanges.recv() => match change {
                        Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => return Err(error),
                    },
                    change = peerChanges.recv() => match change {
                        Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => return Err(error),
                    },
                }
            }
            Err(error) => return Err(error),
        }
    }
}

/// Pumps one stable outer Binding stream while replacing its inner source by generation.
#[allow(non_snake_case)]
async fn runRecoveringBindingWatch(
    router: CoreNodeRouter,
    key: String,
    request: CoreWatchRequest,
    sourceBinding: CoreNodeBindingRecord,
    mut stream: CoreEventStream,
    mut bindingChanges: broadcast::Receiver<String>,
    mut peerChanges: broadcast::Receiver<()>,
    sender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    mut cancelReceiver: tokio::sync::oneshot::Receiver<()>,
) {
    let mut routeCursor = None;
    let mut sourceNodeId = sourceBinding.nodeId;
    let mut sourceGeneration = sourceBinding.generation;
    operit_util::AppLogger::AppLogger::i(
        "CoreNodeRouteTrace",
        &format!(
            "binding_watch_recovery_started requestId={} property={} key={} local={} source={}",
            request.requestId.0, request.propertyName, key, router.localNodeId, sourceNodeId
        ),
    );
    loop {
        tokio::select! {
            biased;
            _ = &mut cancelReceiver => return,
            event = stream.recv() => {
                let Some(event) = event else {
                    match openBindingWatchSourceUntilReachable(
                        &router,
                        &key,
                        &request,
                        routeCursor.clone(),
                        &mut bindingChanges,
                        &mut peerChanges,
                        &mut cancelReceiver,
                    )
                    .await
                    {
                        Ok(Some((binding, nextStream))) => {
                            sourceNodeId = binding.nodeId;
                            sourceGeneration = binding.generation;
                            stream = nextStream;
                            continue;
                        }
                        Ok(None) => return,
                        Err(error) => {
                            operit_util::AppLogger::AppLogger::e(
                                "CoreNodeRouteTrace",
                                &format!(
                                    "binding_watch_source_reopen_failed requestId={} property={} key={} local={} source={} generation={} code={} error={}",
                                    request.requestId.0,
                                    request.propertyName,
                                    key,
                                    router.localNodeId,
                                    sourceNodeId,
                                    sourceGeneration,
                                    error.code,
                                    error
                                ),
                            );
                            return;
                        }
                    }
                };
                if event.propertyName == CORE_ROUTE_CURSOR_PROPERTY {
                    routeCursor = Some(event.value);
                    continue;
                }
                if event.kind == CoreEventKind::Completed {
                    if let Ok(binding) = router.bindingStore.binding(&key) {
                        if binding.nodeId != sourceNodeId
                            || binding.generation != sourceGeneration
                        {
                            match openBindingWatchSourceUntilReachable(
                                &router,
                                &key,
                                &request,
                                routeCursor.clone(),
                                &mut bindingChanges,
                                &mut peerChanges,
                                &mut cancelReceiver,
                            )
                            .await
                            {
                                Ok(Some((nextBinding, nextStream))) => {
                                    sourceNodeId = nextBinding.nodeId;
                                    sourceGeneration = nextBinding.generation;
                                    stream = nextStream;
                                    continue;
                                }
                                Ok(None) => return,
                                Err(error) => {
                                    operit_util::AppLogger::AppLogger::e(
                                        "CoreNodeRouteTrace",
                                        &format!(
                                            "binding_watch_completion_reopen_failed requestId={} key={} code={} error={}",
                                            request.requestId.0, key, error.code, error
                                        ),
                                    );
                                    return;
                                }
                            }
                        }
                    }
                    let _ = sender.send(event);
                    return;
                }
                if sender.send(event).is_err() {
                    return;
                }
            }
            change = bindingChanges.recv() => {
                let changedKey = match change {
                    Ok(changedKey) => changedKey,
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        operit_util::AppLogger::AppLogger::e(
                            "CoreNodeRouteTrace",
                            &format!("Binding watch missed {skipped} Binding changes"),
                        );
                        key.clone()
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                };
                if changedKey != key {
                    continue;
                }
                operit_util::AppLogger::AppLogger::i(
                    "CoreNodeRouteTrace",
                    &format!(
                        "binding_watch_change_received requestId={} property={} key={} local={} source={} generation={}",
                        request.requestId.0,
                        request.propertyName,
                        key,
                        router.localNodeId,
                        sourceNodeId,
                        sourceGeneration
                    ),
                );
                let binding = match router.bindingStore.binding(&key) {
                    Ok(binding) => binding,
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::e(
                            "CoreNodeRouteTrace",
                            &format!(
                                "binding_watch_change_read_failed requestId={} property={} key={} local={} error={}",
                                request.requestId.0,
                                request.propertyName,
                                key,
                                router.localNodeId,
                                error
                            ),
                        );
                        return;
                    }
                };
                if binding.nodeId == sourceNodeId && binding.generation == sourceGeneration {
                    continue;
                }
                operit_util::AppLogger::AppLogger::i(
                    "CoreNodeRouteTrace",
                    &format!(
                        "binding_watch_rebind_open requestId={} property={} key={} local={} nextSource={} nextGeneration={}",
                        request.requestId.0,
                        request.propertyName,
                        key,
                        router.localNodeId,
                        binding.nodeId,
                        binding.generation
                    ),
                );
                match openBindingWatchSourceUntilReachable(
                    &router,
                    &key,
                    &request,
                    routeCursor.clone(),
                    &mut bindingChanges,
                    &mut peerChanges,
                    &mut cancelReceiver,
                )
                .await
                {
                    Ok(Some((nextBinding, nextStream))) => {
                        sourceNodeId = nextBinding.nodeId;
                        sourceGeneration = nextBinding.generation;
                        stream = nextStream;
                    }
                    Ok(None) => return,
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::e(
                            "CoreNodeRouteTrace",
                            &format!(
                                "binding_watch_rebind_failed requestId={} key={} code={} error={}",
                                request.requestId.0, key, error.code, error
                            ),
                        );
                        return;
                    }
                }
            }
            change = peerChanges.recv() => {
                match change {
                    Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                }
                let binding = match router.bindingStore.binding(&key) {
                    Ok(binding) => binding,
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::e(
                            "CoreNodeRouteTrace",
                            &format!(
                                "binding_watch_peer_change_read_failed requestId={} property={} key={} local={} error={}",
                                request.requestId.0,
                                request.propertyName,
                                key,
                                router.localNodeId,
                                error
                            ),
                        );
                        return;
                    }
                };
                if binding.nodeId == sourceNodeId && binding.generation == sourceGeneration {
                    continue;
                }
                if !matches!(router.nodeIsReachable(&binding.nodeId), Ok(true)) {
                    continue;
                }
                operit_util::AppLogger::AppLogger::i(
                    "CoreNodeRouteTrace",
                    &format!(
                        "binding_watch_peer_rebind_open requestId={} property={} key={} local={} nextSource={} nextGeneration={}",
                        request.requestId.0,
                        request.propertyName,
                        key,
                        router.localNodeId,
                        binding.nodeId,
                        binding.generation
                    ),
                );
                match openBindingWatchSourceUntilReachable(
                    &router,
                    &key,
                    &request,
                    routeCursor.clone(),
                    &mut bindingChanges,
                    &mut peerChanges,
                    &mut cancelReceiver,
                )
                .await
                {
                    Ok(Some((nextBinding, nextStream))) => {
                        sourceNodeId = nextBinding.nodeId;
                        sourceGeneration = nextBinding.generation;
                        stream = nextStream;
                    }
                    Ok(None) => return,
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::e(
                            "CoreNodeRouteTrace",
                            &format!(
                                "binding_watch_peer_rebind_failed requestId={} key={} code={} error={}",
                                request.requestId.0, key, error.code, error
                            ),
                        );
                        return;
                    }
                }
            }
        }
    }
}

/// Resolves the route that produced one embedded stream descriptor.
#[allow(non_snake_case)]
fn embeddedStreamSourceRoute(
    request: &CoreWatchRequest,
) -> Result<crate::GeneratedSpaceRoute, CoreLinkError> {
    let arguments = embeddedStreamArguments(request)?;
    let sourceMethod =
        embeddedStreamStringArgument(arguments, CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT)?;
    let sourceMode =
        embeddedStreamStringArgument(arguments, CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT)?;
    let sourceArgs = embeddedStreamSourceArgs(request)?;
    match sourceMode.as_str() {
        "call" => crate::generated_space_call_route(&CoreCallRequest {
            requestId: request.requestId.clone(),
            targetObjectId: CORE_INTERNAL_ROUTE_OBJECT_ID,
            methodName: sourceMethod,
            args: sourceArgs.clone(),
        }),
        "watch" => crate::generated_space_watch_route(&CoreWatchRequest {
            requestId: request.requestId.clone(),
            targetObjectId: CORE_INTERNAL_ROUTE_OBJECT_ID,
            propertyName: sourceMethod,
            args: sourceArgs.clone(),
        }),
        _ => {
            return Err(CoreLinkError::new(
                "INVALID_ARGS",
                "embedded stream source mode is not supported",
            ))
        }
    }
    .ok_or_else(|| {
        CoreLinkError::new(
            "SPACE_ROUTE_NOT_FOUND",
            "Embedded stream source route is not registered",
        )
    })
}

/// Reads the route arguments that produced one embedded stream descriptor.
#[allow(non_snake_case)]
fn embeddedStreamSourceArgs(request: &CoreWatchRequest) -> Result<&CoreValue, CoreLinkError> {
    embeddedStreamArguments(request)?
        .get(CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT)
        .ok_or_else(|| {
            CoreLinkError::new(
                "CORE_BINDING_KEY_REQUIRED",
                "Embedded stream source arguments are missing",
            )
        })
}

/// Reads the argument map carried by one embedded stream open request.
#[allow(non_snake_case)]
fn embeddedStreamArguments(
    request: &CoreWatchRequest,
) -> Result<&BTreeMap<String, CoreValue>, CoreLinkError> {
    let CoreValue::Map(arguments) = &request.args else {
        return Err(CoreLinkError::new(
            "INVALID_ARGS",
            "Embedded stream route arguments must be a map",
        ));
    };
    Ok(arguments)
}

/// Reads one string argument from an embedded stream open request.
#[allow(non_snake_case)]
fn embeddedStreamStringArgument(
    arguments: &BTreeMap<String, CoreValue>,
    name: &str,
) -> Result<String, CoreLinkError> {
    match arguments.get(name) {
        Some(CoreValue::String(value)) => Ok(value.clone()),
        Some(_) => Err(CoreLinkError::new(
            "INVALID_ARGS",
            format!("{name} must be a string"),
        )),
        None => Err(CoreLinkError::new(
            "CORE_BINDING_KEY_REQUIRED",
            format!("{name} is missing"),
        )),
    }
}

/// Adds adapter-owned render and source state to one reopened watch request.
#[allow(non_snake_case)]
fn watchRequestWithRouteState(
    request: &CoreWatchRequest,
    cursor: Option<CoreValue>,
) -> Result<CoreWatchRequest, CoreLinkError> {
    if cursor.is_none() {
        return Ok(request.clone());
    }
    let mut reopened = request.clone();
    let arguments = match &mut reopened.args {
        CoreValue::Map(arguments) => arguments,
        CoreValue::Null => {
            reopened.args = CoreValue::Map(BTreeMap::new());
            let CoreValue::Map(arguments) = &mut reopened.args else {
                unreachable!("Core watch arguments were initialized as a map")
            };
            arguments
        }
        _ => {
            return Err(CoreLinkError::new(
                "INVALID_ARGS",
                "Core watch arguments must be a map before route state injection",
            ))
        }
    };
    if let Some(cursor) = cursor {
        arguments.insert(CORE_ROUTE_CURSOR_ARGUMENT.to_string(), cursor);
    }
    Ok(reopened)
}

/// Computes a route TTL that permits one traversal of every Space member.
#[allow(non_snake_case)]
fn routeTtl(space: &CoreSpace) -> Result<u32, CoreLinkError> {
    u32::try_from(space.members.len())
        .map_err(|_| CoreLinkError::new("SPACE_TOO_LARGE", "Space member count exceeds u32"))
}

/// Returns whether a routed operation failed before reaching its selected device.
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

/// Creates a process-unique request id for one internal CoreNode control call.
#[allow(non_snake_case)]
pub(crate) fn nextCoreNodeRouterRequestId(methodName: &str) -> String {
    let sequence = CORE_NODE_ROUTER_REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("core-node-router-{methodName}-{sequence}")
}

#[async_trait(?Send)]
impl CoreLinkClient for CoreNodeRouter {
    /// Executes a one-shot request through the route selected by runtime state.
    async fn call(&mut self, request: CoreCallRequest) -> CoreCallResponse {
        CoreLinkSharedClient::call(self, request).await
    }

    /// Reads a watch snapshot through the route selected by runtime state.
    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &mut self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        CoreLinkSharedClient::watchSnapshot(self, request).await
    }

    /// Opens a watch stream through the route selected by runtime state.
    async fn watch(&mut self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        CoreLinkSharedClient::watch(self, request).await
    }

    #[allow(non_snake_case)]
    async fn openPush(
        &mut self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        self.openPushSession(request).await
    }
}

#[async_trait(?Send)]
impl CoreNodeLinkClient for CoreNodeRouter {
    /// Clones this router before a transport task performs network waits.
    #[allow(non_snake_case)]
    fn cloneCoreNodeLinkClient(&self) -> Box<dyn CoreNodeLinkClient + Send> {
        Box::new(self.clone())
    }

    /// Executes or forwards one routed EnhanceAI handoff.
    #[allow(non_snake_case)]
    async fn routedHandoff(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreHandoffRequest>,
    ) -> Result<CoreHandoffResponse, CoreLinkError> {
        if self.validateIncomingRoute(&previousNodeId, &request)? {
            return self.localCore.handoffAtBoundary(request.payload).await;
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedHandoff(forwardedRequest).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Executes or forwards one routed call.
    #[allow(non_snake_case)]
    async fn routedCall(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let requestId = request.payload.requestId.clone();
        match self.validateIncomingRoute(&previousNodeId, &request) {
            Ok(true) => {
                if request.routeKind == RoutedCoreRequestKind::SpaceRoute {
                    self.localCore.callSpace(request.payload).await
                } else {
                    self.executeLocalCall(request.payload).await
                }
            }
            Ok(false) => {
                let mut excludedPeerNodeIds = BTreeSet::new();
                loop {
                    let (peer, forwardedRequest) = match self.forwardRouteAvoiding(
                        Some(&previousNodeId),
                        &excludedPeerNodeIds,
                        request.clone(),
                    ) {
                        Ok(value) => value,
                        Err(error) => return CoreCallResponse::err(requestId, error),
                    };
                    let peerNodeId = peer.peerNodeId();
                    let response = peer.routedCall(forwardedRequest).await;
                    if response
                        .result
                        .as_ref()
                        .err()
                        .map(isRouteUnavailableError)
                        .unwrap_or(false)
                    {
                        excludedPeerNodeIds.insert(peerNodeId);
                        continue;
                    }
                    return response;
                }
            }
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    /// Executes or forwards one routed watch snapshot.
    #[allow(non_snake_case)]
    async fn routedWatchSnapshot(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        if self.validateIncomingRoute(&previousNodeId, &request)? {
            return if request.routeKind == RoutedCoreRequestKind::SpaceRoute {
                self.localCore.watchSpaceSnapshot(request.payload).await
            } else {
                operit_link::withCoreForceLocal(self.localCore.watchSnapshot(request.payload)).await
            };
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatchSnapshot(forwardedRequest).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Executes or forwards one routed watch.
    #[allow(non_snake_case)]
    async fn routedWatch(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if self.validateIncomingRoute(&previousNodeId, &request)? {
            return if request.routeKind == RoutedCoreRequestKind::SpaceRoute {
                self.localCore.watchSpace(request.payload).await
            } else if request.payload.targetObjectId == CORE_STREAM_POOL_OBJECT_ID {
                self.localCore.watchSpace(request.payload).await
            } else {
                operit_link::withCoreForceLocal(self.localCore.watch(request.payload)).await
            };
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatch(forwardedRequest).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Applies or forwards one committed Binding operation.
    #[allow(non_snake_case)]
    async fn routedBindingApply(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreNodeBindingApplyRequest>,
    ) -> Result<(), CoreLinkError> {
        if self.validateIncomingRoute(&previousNodeId, &request)? {
            return self.executeBindingApply(request.payload);
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedBindingApply(forwardedRequest).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Executes or forwards one routed push open.
    #[allow(non_snake_case)]
    async fn routedOpenPush(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        if self.validateIncomingRoute(&previousNodeId, &request)? {
            return if request.routeKind == RoutedCoreRequestKind::SpaceRoute {
                self.localCore.openSpacePush(request.payload)
            } else {
                self.localCore.openPush(request.payload)
            };
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedOpenPush(forwardedRequest).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for CoreNodeRouter {
    /// Executes a one-shot request on the CoreNode selected by generated metadata.
    async fn call(&self, mut request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let generatedRoute = match crate::generated_core_call_route(&request) {
            Ok(route) => route,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        let route = generatedRoute;
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_call_resolve requestId={} path={} method={} key={} local={}",
                    request.requestId.0,
                    request.targetObjectId.to_string(),
                    request.methodName,
                    key,
                    self.localNodeId
                ),
            );
        }
        let targetNodeId = match self.routeNodeId(route.clone()).await {
            Ok(targetNodeId) => targetNodeId,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouter",
                &format!(
                    "binding call route requestId={} path={} method={} key={} source={} target={}",
                    request.requestId.0,
                    request.targetObjectId.to_string(),
                    request.methodName,
                    key,
                    self.localNodeId,
                    targetNodeId
                ),
            );
        }
        if targetNodeId == self.localNodeId {
            return self.executeLocalCall(request).await;
        }
        let methodName = request.methodName.clone();
        let requestId = request.requestId.0.clone();
        let response = self.callNode(targetNodeId.clone(), request).await;
        operit_util::AppLogger::AppLogger::i(
            "CoreNodeRouter",
            &format!(
                "remote call returned requestId={} method={} target={} success={}",
                requestId,
                methodName,
                targetNodeId,
                response.result.is_ok()
            ),
        );
        response
    }

    /// Reads a watch snapshot on the CoreNode selected by generated metadata.
    #[allow(non_snake_case)]
    async fn watchSnapshot(
        &self,
        mut request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let route = crate::generated_core_watch_route(&request)?;
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_snapshot_resolve requestId={} path={} property={} key={} local={}",
                    request.requestId.0,
                    request.targetObjectId.to_string(),
                    request.propertyName,
                    key,
                    self.localNodeId
                ),
            );
        }
        let targetNodeId = self.routeNodeId(route).await?;
        if targetNodeId == self.localNodeId {
            return operit_link::withCoreForceLocal(self.localCore.watchSnapshot(request)).await;
        }
        self.watchNodeSnapshot(targetNodeId, request).await
    }

    /// Opens a watch stream on the CoreNode selected by generated metadata.
    async fn watch(&self, mut request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.targetObjectId == CORE_STREAM_POOL_OBJECT_ID {
            return self.watchSpaceEmbeddedStream(request).await;
        }
        let route = crate::generated_core_watch_route(&request)?;
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_watch_resolve requestId={} path={} property={} key={} local={}",
                    request.requestId.0,
                    request.targetObjectId.to_string(),
                    request.propertyName,
                    key,
                    self.localNodeId
                ),
            );
        }
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouter",
                &format!(
                    "binding watch open requestId={} path={} property={} key={} source={}",
                    request.requestId.0,
                    request.targetObjectId.to_string(),
                    request.propertyName,
                    key,
                    self.localNodeId
                ),
            );
            return self.openRecoveringBindingWatch(key.clone(), request).await;
        }
        let targetNodeId = self.routeNodeId(route).await?;
        if targetNodeId == self.localNodeId {
            return operit_link::withCoreForceLocal(self.localCore.watch(request)).await;
        }
        self.watchNode(targetNodeId, request).await
    }
}

impl CoreRouteRuntime for CoreNodeRouter {
    /// Resolves one annotation binding before the wrapper enters a remote route.
    fn shouldRoute(&self, methodName: &str, args: &CoreValue) -> Result<bool, CoreLinkError> {
        if operit_link::coreForceLocal() {
            return Ok(false);
        }
        let request = CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId(methodName),
            operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID,
            methodName,
            args.clone(),
        );
        let route = match crate::generated_core_call_route(&request) {
            Err(error) if error.code == "CORE_BINDING_KEY_REQUIRED" => return Ok(false),
            result => result?,
        };
        match route {
            GeneratedCoreRoute::Local => Ok(false),
            GeneratedCoreRoute::Binding { key, .. } => {
                let targetNodeId = match self.bindingStore.bindingNodeId(&key) {
                    Ok(targetNodeId) => targetNodeId,
                    Err(error) if error.code == "CORE_BINDING_NOT_FOUND" => return Ok(false),
                    Err(error) => return Err(error),
                };
                Ok(targetNodeId != self.localNodeId && self.nodeIsReachable(&targetNodeId)?)
            }
        }
    }

    /// Resolves one annotation watch binding before the wrapper opens a managed route stream.
    #[allow(non_snake_case)]
    fn shouldRouteWatch(&self, methodName: &str, args: &CoreValue) -> Result<bool, CoreLinkError> {
        if operit_link::coreForceLocal() {
            return Ok(false);
        }
        let request = CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId(methodName),
            operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID,
            methodName,
            args.clone(),
        );
        let route = match crate::generated_core_call_route(&request) {
            Err(error) if error.code == "CORE_BINDING_KEY_REQUIRED" => return Ok(false),
            result => result?,
        };
        match route {
            GeneratedCoreRoute::Local => Ok(false),
            GeneratedCoreRoute::Binding { key, .. } => {
                let targetNodeId = match self.bindingStore.bindingNodeId(&key) {
                    Ok(targetNodeId) => targetNodeId,
                    Err(error) if error.code == "CORE_BINDING_NOT_FOUND" => return Ok(false),
                    Err(error) => return Err(error),
                };
                Ok(targetNodeId != self.localNodeId && self.nodeIsReachable(&targetNodeId)?)
            }
        }
    }

    /// Resolves whether an annotation watch can start from the current local Core source.
    #[allow(non_snake_case)]
    fn shouldUseLocalWatchSource(
        &self,
        methodName: &str,
        args: &CoreValue,
    ) -> Result<bool, CoreLinkError> {
        if operit_link::coreForceLocal() {
            return Ok(false);
        }
        let request = CoreCallRequest::new(
            operit_link::nextCoreRouteRequestId(methodName),
            operit_link::CORE_INTERNAL_ROUTE_OBJECT_ID,
            methodName,
            args.clone(),
        );
        let route = match crate::generated_core_call_route(&request) {
            Err(error) if error.code == "CORE_BINDING_KEY_REQUIRED" => return Ok(false),
            result => result?,
        };
        match route {
            GeneratedCoreRoute::Local => Ok(false),
            GeneratedCoreRoute::Binding { key, .. } => {
                let targetNodeId = match self.bindingStore.bindingNodeId(&key) {
                    Ok(targetNodeId) => targetNodeId,
                    Err(error) if error.code == "CORE_BINDING_NOT_FOUND" => return Ok(false),
                    Err(error) => return Err(error),
                };
                Ok(targetNodeId == self.localNodeId || !self.nodeIsReachable(&targetNodeId)?)
            }
        }
    }

    /// Routes one annotation wrapper call through Binding-selected Space routing.
    fn call(&self, request: CoreCallRequest) -> Pin<Box<dyn Future<Output = CoreCallResponse>>> {
        let router = self.clone();
        Box::pin(async move { router.callSpace(request).await })
    }

    /// Routes one annotation StateFlow watch through Binding-selected Space routing.
    fn watch(
        &self,
        request: CoreWatchRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CoreEventStream, CoreLinkError>>>> {
        let router = self.clone();
        Box::pin(
            async move { <CoreNodeRouter as CoreLinkSharedClient>::watch(&router, request).await },
        )
    }

    /// Routes one annotation StateFlow watch while preserving the wrapper-opened local source.
    #[allow(non_snake_case)]
    fn watchWithLocalSource(
        &self,
        request: CoreWatchRequest,
        localStream: CoreEventStream,
    ) -> Pin<Box<dyn Future<Output = Result<CoreEventStream, CoreLinkError>>>> {
        let router = self.clone();
        Box::pin(async move {
            let route = crate::generated_core_watch_route(&request)?;
            match route {
                GeneratedCoreRoute::Binding { key, .. } => {
                    router
                        .openRecoveringBindingWatchWithLocalSource(key, request, localStream)
                        .await
                }
                GeneratedCoreRoute::Local => Ok(localStream),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_access_runtime::CoreNodePeerLink::{
        connectInMemoryPeerLinks, peerLink, CoreNodeTransportClient, RoutedCoreRequest,
        RoutedCoreRequestKind,
    };
    use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
    use operit_host_api::{
        FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
        GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult, HostRuntimeAsyncTask,
        HostRuntimeTask, HostRuntimeTaskSchedulerHost, HostSecretStore, RuntimeSqliteConnection,
        RuntimeSqliteHost, RuntimeSqliteTransaction, RuntimeStorageEntry, RuntimeStorageHost,
        SqliteRow, SqliteValue,
    };
    use operit_link::{
        CorePushRequest, CoreStream, CoreStreamSource, CORE_INTERNAL_ROUTE_OBJECT_ID,
    };
    use operit_model::ChatMessage::ChatMessage;
    use operit_model::InputProcessingState::InputProcessingState;
    use operit_rslink_runtime::CoreStreamPool;
    use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
    use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
    use operit_runtime::services::core::MessageProcessingDelegate::coreResponseStreamSource;
    use operit_runtime::services::ChatServiceCore::ChatState;
    use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
    use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
    use operit_store::SyncOperationStore::SyncOperationSemantics;
    use operit_util::stream::HotStream::mutable_shared_stream;
    use operit_util::stream::RevisableTextStream::DelegatingRevisableSharedTextStream;
    use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex as StdMutex, Once, OnceLock};
    use std::time::Duration;

    /// Stores runtime records in memory for route and Space integration tests.
    #[derive(Clone)]
    struct TestRuntimeStorageHost {
        files: Arc<StdMutex<BTreeMap<String, Vec<u8>>>>,
        runtimeRoot: std::path::PathBuf,
        workspaceRoot: std::path::PathBuf,
    }

    impl Default for TestRuntimeStorageHost {
        /// Creates an in-memory storage host with real root paths for path-only callers.
        fn default() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock must be after UNIX_EPOCH")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "operit-node-runtime-route-test-{}-{nanos}",
                std::process::id(),
            ));
            let runtimeRoot = root.join("runtime");
            let workspaceRoot = root.join("workspace");
            std::fs::create_dir_all(&runtimeRoot).expect("test runtime root must be creatable");
            std::fs::create_dir_all(&workspaceRoot).expect("test workspace root must be creatable");
            Self {
                files: Arc::new(StdMutex::new(BTreeMap::new())),
                runtimeRoot,
                workspaceRoot,
            }
        }
    }

    impl RuntimeStorageHost for TestRuntimeStorageHost {
        /// Returns the physical runtime root used by path-only test callers.
        #[allow(non_snake_case)]
        fn runtimeRootDir(&self) -> Option<std::path::PathBuf> {
            Some(self.runtimeRoot.clone())
        }

        /// Returns the physical workspace root used by path-only test callers.
        #[allow(non_snake_case)]
        fn workspaceRootDir(&self) -> Option<std::path::PathBuf> {
            Some(self.workspaceRoot.clone())
        }

        /// Reads one in-memory runtime storage file.
        #[allow(non_snake_case)]
        fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .get(path)
                .cloned()
                .ok_or_else(|| HostError::new(format!("missing runtime storage entry: {path}")))
        }

        /// Writes one in-memory runtime storage file.
        #[allow(non_snake_case)]
        fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .insert(path.to_string(), content.to_vec());
            Ok(())
        }

        /// Appends bytes to one in-memory runtime storage file.
        #[allow(non_snake_case)]
        fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .entry(path.to_string())
                .or_default()
                .extend_from_slice(content);
            Ok(())
        }

        /// Removes one in-memory runtime storage entry.
        fn delete(&self, path: &str, _recursive: bool) -> HostResult<()> {
            self.files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .remove(path);
            Ok(())
        }

        /// Reports whether one in-memory runtime storage entry exists.
        fn exists(&self, path: &str) -> HostResult<bool> {
            Ok(self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .contains_key(path))
        }

        /// Lists in-memory runtime storage entries under a prefix.
        fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
            Ok(self
                .files
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .iter()
                .filter(|(path, _)| path.starts_with(prefix))
                .map(|(path, content)| RuntimeStorageEntry {
                    path: path.clone(),
                    isDirectory: false,
                    size: content.len() as i64,
                })
                .collect())
        }
    }

    /// Provides a file host shell for constructing unused local chat runtime state in tests.
    #[derive(Clone, Debug, Default)]
    struct TestFileSystemHost;

    impl TestFileSystemHost {
        /// Creates the standard file host error used by unsupported test operations.
        fn unsupported(operation: &str) -> HostError {
            HostError::new(format!(
                "test file system host does not support {operation}"
            ))
        }
    }

    impl FileSystemHost for TestFileSystemHost {
        /// Returns the test environment label.
        #[allow(non_snake_case)]
        fn envLabel(&self) -> &str {
            "test"
        }

        /// Returns a host descriptor for the synthetic test environment.
        #[allow(non_snake_case)]
        fn environmentDescriptor(&self) -> HostEnvironmentDescriptor {
            HostEnvironmentDescriptor::android()
        }

        /// Accepts paths without touching a platform file system.
        #[allow(non_snake_case)]
        fn validatePath(&self, _path: &str, _paramName: &str) -> HostResult<()> {
            Ok(())
        }

        /// Rejects file listing because the route test does not access files.
        #[allow(non_snake_case)]
        fn listFiles(&self, _path: &str) -> HostResult<Vec<FileEntry>> {
            Err(Self::unsupported("listFiles"))
        }

        /// Rejects text reads because the route test does not access files.
        #[allow(non_snake_case)]
        fn readFile(&self, _path: &str) -> HostResult<String> {
            Err(Self::unsupported("readFile"))
        }

        /// Rejects bounded text reads because the route test does not access files.
        #[allow(non_snake_case)]
        fn readFileWithLimit(&self, _path: &str, _maxBytes: usize) -> HostResult<String> {
            Err(Self::unsupported("readFileWithLimit"))
        }

        /// Rejects byte reads because the route test does not access files.
        #[allow(non_snake_case)]
        fn readFileBytes(&self, _path: &str) -> HostResult<Vec<u8>> {
            Err(Self::unsupported("readFileBytes"))
        }

        /// Rejects text writes because the route test does not access files.
        #[allow(non_snake_case)]
        fn writeFile(&self, _path: &str, _content: &str, _append: bool) -> HostResult<()> {
            Err(Self::unsupported("writeFile"))
        }

        /// Rejects byte writes because the route test does not access files.
        #[allow(non_snake_case)]
        fn writeFileBytes(&self, _path: &str, _content: &[u8]) -> HostResult<()> {
            Err(Self::unsupported("writeFileBytes"))
        }

        /// Rejects deletion because the route test does not access files.
        #[allow(non_snake_case)]
        fn deleteFile(&self, _path: &str, _recursive: bool) -> HostResult<()> {
            Err(Self::unsupported("deleteFile"))
        }

        /// Reports missing files without touching a platform file system.
        #[allow(non_snake_case)]
        fn fileExists(&self, _path: &str) -> HostResult<FileExistence> {
            Ok(FileExistence {
                exists: false,
                isDirectory: false,
                size: 0,
            })
        }

        /// Rejects moves because the route test does not access files.
        #[allow(non_snake_case)]
        fn moveFile(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Err(Self::unsupported("moveFile"))
        }

        /// Rejects copies because the route test does not access files.
        #[allow(non_snake_case)]
        fn copyFile(&self, _source: &str, _destination: &str, _recursive: bool) -> HostResult<()> {
            Err(Self::unsupported("copyFile"))
        }

        /// Rejects directory creation because the route test does not access files.
        #[allow(non_snake_case)]
        fn makeDirectory(&self, _path: &str, _createParents: bool) -> HostResult<()> {
            Err(Self::unsupported("makeDirectory"))
        }

        /// Rejects file search because the route test does not access files.
        #[allow(non_snake_case)]
        fn findFiles(&self, _request: FindFilesRequest) -> HostResult<Vec<String>> {
            Err(Self::unsupported("findFiles"))
        }

        /// Rejects file metadata reads because the route test does not access files.
        #[allow(non_snake_case)]
        fn fileInfo(&self, _path: &str) -> HostResult<FileInfo> {
            Err(Self::unsupported("fileInfo"))
        }

        /// Rejects code grep because the route test does not access files.
        #[allow(non_snake_case)]
        fn grepCode(&self, _request: GrepCodeRequest) -> HostResult<GrepCodeResult> {
            Err(Self::unsupported("grepCode"))
        }

        /// Rejects archive creation because the route test does not access files.
        #[allow(non_snake_case)]
        fn zipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Err(Self::unsupported("zipFiles"))
        }

        /// Rejects archive extraction because the route test does not access files.
        #[allow(non_snake_case)]
        fn unzipFiles(&self, _source: &str, _destination: &str) -> HostResult<()> {
            Err(Self::unsupported("unzipFiles"))
        }

        /// Rejects OS open because the route test does not access files.
        #[allow(non_snake_case)]
        fn openFile(&self, _path: &str) -> HostResult<()> {
            Err(Self::unsupported("openFile"))
        }

        /// Rejects OS share because the route test does not access files.
        #[allow(non_snake_case)]
        fn shareFile(&self, _path: &str, _title: &str) -> HostResult<()> {
            Err(Self::unsupported("shareFile"))
        }
    }

    /// Stores encrypted preference secrets in process memory for runtime initialization tests.
    #[derive(Default)]
    struct TestSecretStore {
        values: StdMutex<BTreeMap<String, Vec<u8>>>,
    }

    impl HostSecretStore for TestSecretStore {
        /// Reads one test secret from memory.
        #[allow(non_snake_case)]
        fn readSecret(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
            Ok(self
                .values
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .get(key)
                .cloned())
        }

        /// Writes one test secret into memory.
        #[allow(non_snake_case)]
        fn writeSecret(&self, key: &str, content: &[u8]) -> HostResult<()> {
            self.values
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .insert(key.to_string(), content.to_vec());
            Ok(())
        }

        /// Deletes one test secret from memory.
        #[allow(non_snake_case)]
        fn deleteSecret(&self, key: &str) -> HostResult<()> {
            self.values
                .lock()
                .map_err(|error| HostError::new(error.to_string()))?
                .remove(key);
            Ok(())
        }
    }

    /// Opens file-backed SQLite databases under the test runtime root.
    struct TestSqliteHost {
        root: std::path::PathBuf,
    }

    impl TestSqliteHost {
        /// Creates a SQLite host rooted at one runtime directory.
        fn new(root: std::path::PathBuf) -> Self {
            Self { root }
        }

        /// Resolves one runtime-relative database path under the test root.
        fn resolve(&self, path: &str) -> HostResult<std::path::PathBuf> {
            let path = std::path::Path::new(path);
            if path.is_absolute() {
                return Err(HostError::new(format!(
                    "test SQLite path must be relative: {}",
                    path.display()
                )));
            }
            let mut resolved = self.root.clone();
            for component in path.components() {
                match component {
                    std::path::Component::Normal(segment) => resolved.push(segment),
                    std::path::Component::CurDir => {}
                    _ => {
                        return Err(HostError::new(format!(
                            "test SQLite path is invalid: {}",
                            path.display()
                        )))
                    }
                }
            }
            Ok(resolved)
        }
    }

    impl RuntimeSqliteHost for TestSqliteHost {
        /// Opens one SQLite database in the test runtime directory.
        #[allow(non_snake_case)]
        fn openSqliteDatabase(&self, path: &str) -> HostResult<Box<dyn RuntimeSqliteConnection>> {
            let path = self.resolve(path)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let connection = rusqlite::Connection::open(path)
                .map_err(|error| HostError::new(error.to_string()))?;
            connection
                .execute_batch(
                    r#"
                    PRAGMA journal_mode = MEMORY;
                    PRAGMA synchronous = OFF;
                    PRAGMA temp_store = MEMORY;
                    "#,
                )
                .map_err(|error| HostError::new(error.to_string()))?;
            Ok(Box::new(TestSqliteConnection { connection }))
        }
    }

    /// Wraps one rusqlite connection in the host SQLite trait.
    struct TestSqliteConnection {
        connection: rusqlite::Connection,
    }

    impl RuntimeSqliteConnection for TestSqliteConnection {
        /// Executes one SQL batch on the wrapped connection.
        #[allow(non_snake_case)]
        fn executeBatch(&mut self, sql: &str) -> HostResult<()> {
            self.connection
                .execute_batch(sql)
                .map_err(|error| HostError::new(error.to_string()))
        }

        /// Executes one parameterized SQL statement on the wrapped connection.
        fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
            let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
            self.connection
                .execute(sql, rusqlite::params_from_iter(params))
                .map_err(|error| HostError::new(error.to_string()))
        }

        /// Queries rows from the wrapped connection.
        fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
            querySqliteRows(&self.connection, sql, params)
        }

        /// Returns the last inserted row id for the wrapped connection.
        #[allow(non_snake_case)]
        fn lastInsertRowId(&self) -> HostResult<i64> {
            Ok(self.connection.last_insert_rowid())
        }

        /// Starts one SQLite transaction on the wrapped connection.
        #[allow(non_snake_case)]
        fn beginTransaction(&mut self) -> HostResult<Box<dyn RuntimeSqliteTransaction + '_>> {
            let transaction = self
                .connection
                .transaction()
                .map_err(|error| HostError::new(error.to_string()))?;
            Ok(Box::new(TestSqliteTransaction { transaction }))
        }
    }

    /// Wraps one rusqlite transaction in the host SQLite transaction trait.
    struct TestSqliteTransaction<'a> {
        transaction: rusqlite::Transaction<'a>,
    }

    impl RuntimeSqliteTransaction for TestSqliteTransaction<'_> {
        /// Executes one parameterized SQL statement in the transaction.
        fn execute(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<usize> {
            let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
            self.transaction
                .execute(sql, rusqlite::params_from_iter(params))
                .map_err(|error| HostError::new(error.to_string()))
        }

        /// Queries rows from the transaction.
        fn query(&mut self, sql: &str, params: Vec<SqliteValue>) -> HostResult<Vec<SqliteRow>> {
            querySqliteRows(&self.transaction, sql, params)
        }

        /// Returns the last inserted row id for the transaction.
        #[allow(non_snake_case)]
        fn lastInsertRowId(&self) -> HostResult<i64> {
            Ok(self.transaction.last_insert_rowid())
        }

        /// Commits the transaction.
        fn commit(self: Box<Self>) -> HostResult<()> {
            self.transaction
                .commit()
                .map_err(|error| HostError::new(error.to_string()))
        }
    }

    /// Provides common statement preparation for rusqlite connections and transactions.
    trait TestRusqliteAccess {
        /// Prepares one SQL statement against the underlying SQLite object.
        #[allow(non_snake_case)]
        fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>>;
    }

    impl TestRusqliteAccess for rusqlite::Connection {
        /// Prepares one SQL statement against a connection.
        #[allow(non_snake_case)]
        fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>> {
            self.prepare(sql)
        }
    }

    impl TestRusqliteAccess for rusqlite::Transaction<'_> {
        /// Prepares one SQL statement against a transaction.
        #[allow(non_snake_case)]
        fn prepareStatement<'a>(&'a self, sql: &str) -> rusqlite::Result<rusqlite::Statement<'a>> {
            self.prepare(sql)
        }
    }

    /// Queries rows from one rusqlite access object.
    #[allow(non_snake_case)]
    fn querySqliteRows(
        connection: &impl TestRusqliteAccess,
        sql: &str,
        params: Vec<SqliteValue>,
    ) -> HostResult<Vec<SqliteRow>> {
        let params = params.into_iter().map(toRusqliteValue).collect::<Vec<_>>();
        let mut statement = connection
            .prepareStatement(sql)
            .map_err(|error| HostError::new(error.to_string()))?;
        let columns = statement
            .column_names()
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let mut rows = statement
            .query(rusqlite::params_from_iter(params))
            .map_err(|error| HostError::new(error.to_string()))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|error| HostError::new(error.to_string()))?
        {
            let mut values = Vec::new();
            for index in 0..columns.len() {
                let value = row
                    .get::<_, rusqlite::types::Value>(index)
                    .map_err(|error| HostError::new(error.to_string()))?;
                values.push(fromRusqliteValue(value));
            }
            out.push(SqliteRow {
                columns: columns.clone(),
                values,
            });
        }
        Ok(out)
    }

    /// Converts one host SQLite value to rusqlite's value type.
    #[allow(non_snake_case)]
    fn toRusqliteValue(value: SqliteValue) -> rusqlite::types::Value {
        match value {
            SqliteValue::Null => rusqlite::types::Value::Null,
            SqliteValue::Integer(value) => rusqlite::types::Value::Integer(value),
            SqliteValue::Real(value) => rusqlite::types::Value::Real(value),
            SqliteValue::Text(value) => rusqlite::types::Value::Text(value),
            SqliteValue::Blob(value) => rusqlite::types::Value::Blob(value),
        }
    }

    /// Converts one rusqlite value to the host SQLite value type.
    #[allow(non_snake_case)]
    fn fromRusqliteValue(value: rusqlite::types::Value) -> SqliteValue {
        match value {
            rusqlite::types::Value::Null => SqliteValue::Null,
            rusqlite::types::Value::Integer(value) => SqliteValue::Integer(value),
            rusqlite::types::Value::Real(value) => SqliteValue::Real(value),
            rusqlite::types::Value::Text(value) => SqliteValue::Text(value),
            rusqlite::types::Value::Blob(value) => SqliteValue::Blob(value),
        }
    }

    /// Dispatches local Core requests through a real test SpaceRuntime.
    struct TestSpaceRuntimeSharedClient {
        spaceRuntime: Arc<SpaceRuntime>,
    }

    #[async_trait(?Send)]
    impl CoreLinkSharedClient for TestSpaceRuntimeSharedClient {
        /// Executes one local annotation-addressed call through SpaceRuntime.
        async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
            self.spaceRuntime.call(request).await
        }

        /// Reads one local annotation-addressed watch snapshot through SpaceRuntime.
        #[allow(non_snake_case)]
        async fn watchSnapshot(
            &self,
            request: CoreWatchRequest,
        ) -> Result<CoreEvent, CoreLinkError> {
            self.spaceRuntime.watchSnapshot(request).await
        }

        /// Opens one local annotation-addressed watch through SpaceRuntime.
        async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
            self.spaceRuntime.watch(request).await
        }
    }

    /// Stores one binding table entry for route integration tests.
    struct TestBindingRuntime {
        binding: StdMutex<CoreNodeBindingRecord>,
    }

    impl TestBindingRuntime {
        /// Creates a binding runtime with one chat binding.
        fn new(key: &str, nodeId: String) -> Self {
            Self {
                binding: StdMutex::new(CoreNodeBindingRecord {
                    key: key.to_string(),
                    nodeId,
                    generation: 1,
                }),
            }
        }

        /// Changes the selected test node and increments the binding generation.
        #[allow(non_snake_case)]
        fn setNodeId(&self, targetNodeId: String) -> CoreNodeBindingRecord {
            let mut binding = self
                .binding
                .lock()
                .expect("test binding mutex must not be poisoned");
            binding.nodeId = targetNodeId;
            binding.generation += 1;
            binding.clone()
        }

        /// Builds a synthetic operation for unexercised binding transition tests.
        #[allow(non_snake_case)]
        fn operationFor(binding: &CoreNodeBindingRecord) -> SyncOperation {
            SyncOperation {
                opId: "test-binding-operation".to_string(),
                originDeviceId: binding.nodeId.clone(),
                sequence: binding.generation,
                domain: "core_node".to_string(),
                entityType: "binding".to_string(),
                entityId: binding.key.clone(),
                operation: "set".to_string(),
                semantics: SyncOperationSemantics::EntityState,
                payload: serde_json::json!({ "nodeId": binding.nodeId }),
                createdAt: 1,
                schemaVersion: 1,
            }
        }
    }

    impl CoreNodeBindingRuntime for TestBindingRuntime {
        /// Returns the single binding record owned by this test runtime.
        fn binding(&self, key: &str) -> Result<CoreNodeBindingRecord, CoreLinkError> {
            let binding = self
                .binding
                .lock()
                .expect("test binding mutex must not be poisoned");
            if key == binding.key {
                Ok(binding.clone())
            } else {
                Err(CoreLinkError::new(
                    "CORE_BINDING_READ_FAILED",
                    format!("unknown test binding: {key}"),
                ))
            }
        }

        /// Returns the selected node id for the single test binding.
        #[allow(non_snake_case)]
        fn bindingNodeId(&self, key: &str) -> Result<String, CoreLinkError> {
            self.binding(key).map(|binding| binding.nodeId)
        }

        /// Commits a synthetic binding transition in tests that exercise handoff plumbing.
        #[allow(non_snake_case)]
        fn compareAndSetBinding(
            &self,
            key: &str,
            expectedNodeId: &str,
            expectedGeneration: i64,
            targetNodeId: &str,
        ) -> Result<CoreNodeBindingCommit, CoreLinkError> {
            let mut current = self
                .binding
                .lock()
                .expect("test binding mutex must not be poisoned");
            if key != current.key
                || expectedNodeId != current.nodeId
                || expectedGeneration != current.generation
            {
                return Err(CoreLinkError::new(
                    "CORE_BINDING_TRANSITION_FAILED",
                    "test binding transition did not match the current record",
                ));
            }
            let binding = CoreNodeBindingRecord {
                key: key.to_string(),
                nodeId: targetNodeId.to_string(),
                generation: expectedGeneration + 1,
            };
            *current = binding.clone();
            Ok(CoreNodeBindingCommit {
                operation: Self::operationFor(&binding),
                binding,
            })
        }

        /// Ignores observers because the test opens a fixed route only.
        #[allow(non_snake_case)]
        fn addBindingChangeObserver(&self, _observer: Arc<CoreNodeBindingChangeObserver>) {}

        /// Applies one directly transported binding operation to the synthetic record shape.
        #[allow(non_snake_case)]
        fn applyImmediateBindingOperation(
            &self,
            _operation: &SyncOperation,
        ) -> Result<CoreNodeBindingRecord, CoreLinkError> {
            Ok(self
                .binding
                .lock()
                .expect("test binding mutex must not be poisoned")
                .clone())
        }
    }

    /// Creates a local runtime shell for tests that route every exercised request remotely.
    #[allow(non_snake_case)]
    fn testLocalRuntime(storage: Arc<dyn RuntimeStorageHost>) -> CoreNodeLocalRuntime {
        testLocalRuntimeWithHolder(storage).0
    }

    /// Creates a local runtime shell and returns its real chat holder for end-to-end tests.
    #[allow(non_snake_case)]
    fn testLocalRuntimeWithHolder(
        storage: Arc<dyn RuntimeStorageHost>,
    ) -> (
        CoreNodeLocalRuntime,
        Arc<tokio::sync::Mutex<ChatRuntimeHolder>>,
    ) {
        let runtimeRoot = storage
            .runtimeRootDir()
            .expect("test runtime storage host must expose a runtime root");
        let workspaceRoot = storage
            .workspaceRootDir()
            .expect("test runtime storage host must expose a workspace root");
        operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost(storage.clone());
        operit_store::RuntimeStorageHost::setDefaultRuntimeSqliteHost(Arc::new(
            TestSqliteHost::new(runtimeRoot.clone()),
        ));
        operit_store::RuntimeStorageHost::setDefaultHostSecretStore(Arc::new(
            TestSecretStore::default(),
        ));
        operit_util::RuntimeStoreRoot::setDefaultRuntimeStoreRootConfig(
            operit_util::RuntimeStoreRoot::RuntimeStoreRootConfig::new(runtimeRoot, workspaceRoot),
        );
        let chatRuntimeHolder = Arc::new(tokio::sync::Mutex::new(ChatRuntimeHolder::new(
            Arc::new(TestFileSystemHost),
        )));
        let spaceRuntime = Arc::new(SpaceRuntime::new(chatRuntimeHolder.clone()));
        let sharedClient: Arc<dyn CoreLinkSharedClient + Send + Sync> =
            Arc::new(TestSpaceRuntimeSharedClient {
                spaceRuntime: spaceRuntime.clone(),
            });
        (
            CoreNodeLocalRuntime::new(
                sharedClient.clone(),
                sharedClient,
                storage,
                Arc::new(|_schema| None),
                Arc::new(|_runtime| Ok(())),
                Arc::new(|_request| {
                    Box::pin(async {
                        Err(CoreLinkError::new(
                            "UNEXPECTED_LOCAL_HANDOFF",
                            "local shell handoff was invoked",
                        ))
                    })
                }),
                Arc::new(|request| {
                    Err(CoreLinkError::new(
                        "UNEXPECTED_LOCAL_PUSH",
                        format!("local shell push was invoked: {}", request.methodName),
                    ))
                }),
                spaceRuntime,
            ),
            chatRuntimeHolder,
        )
    }

    /// Creates one real router with synthetic local runtime and in-memory route state.
    #[allow(non_snake_case)]
    fn testCoreNodeRouter(
        localNodeId: &str,
        targetNodeId: &str,
        bindingKey: &str,
    ) -> CoreNodeRouter {
        let storage: Arc<dyn RuntimeStorageHost> = Arc::new(TestRuntimeStorageHost::default());
        CoreNodeIdentityStore::new(storage.clone())
            .writeNodeId(localNodeId.to_string())
            .expect("test node identity must be writable");
        let spaceStore = CoreSpaceStore::new(storage.clone());
        let localSpace = spaceStore
            .initializeNamed("route-test-space".to_string())
            .expect("test space must initialize");
        let joinedSpace = CoreSpace {
            spaceId: localSpace.spaceId.clone(),
            spaceName: localSpace.spaceName.clone(),
            spaceRevision: localSpace.spaceRevision + 1,
            members: vec![localNodeId.to_string(), targetNodeId.to_string()],
        };
        spaceStore
            .adopt(joinedSpace)
            .expect("test space must contain the route target");
        spaceStore
            .setDirectPeers(vec![targetNodeId.to_string()])
            .expect("test space topology must contain the direct peer");
        let (bindingChanges, _) = broadcast::channel(16);
        let bindingStore: Arc<dyn CoreNodeBindingRuntime> = Arc::new(TestBindingRuntime::new(
            bindingKey,
            targetNodeId.to_string(),
        ));
        CoreNodeRouter {
            localCore: Arc::new(testLocalRuntime(storage)),
            bindingStore,
            localNodeId: localNodeId.to_string(),
            spaceStore,
            bindingChanges,
            _bindingChangeObserver: Arc::new(|_change| {}),
        }
    }

    /// Creates one router inside an explicit two-node Space projection.
    #[allow(non_snake_case)]
    fn testCoreNodeRouterInJoinedSpace(
        localNodeId: &str,
        peerNodeId: &str,
        bindingKey: &str,
        bindingNodeId: &str,
        joinedSpace: CoreSpace,
    ) -> (CoreNodeRouter, Arc<tokio::sync::Mutex<ChatRuntimeHolder>>) {
        let (router, holder, _) = testCoreNodeRouterInJoinedSpaceWithBindingRuntime(
            localNodeId,
            peerNodeId,
            bindingKey,
            bindingNodeId,
            joinedSpace,
        );
        (router, holder)
    }

    /// Creates one router in an explicit Space and returns its mutable test Binding runtime.
    #[allow(non_snake_case)]
    fn testCoreNodeRouterInJoinedSpaceWithBindingRuntime(
        localNodeId: &str,
        peerNodeId: &str,
        bindingKey: &str,
        bindingNodeId: &str,
        joinedSpace: CoreSpace,
    ) -> (
        CoreNodeRouter,
        Arc<tokio::sync::Mutex<ChatRuntimeHolder>>,
        Arc<TestBindingRuntime>,
    ) {
        let storage: Arc<dyn RuntimeStorageHost> = Arc::new(TestRuntimeStorageHost::default());
        CoreNodeIdentityStore::new(storage.clone())
            .writeNodeId(localNodeId.to_string())
            .expect("test node identity must be writable");
        let spaceStore = CoreSpaceStore::new(storage.clone());
        spaceStore
            .adopt(joinedSpace)
            .expect("test joined Space must be adopted");
        spaceStore
            .setDirectPeers(vec![peerNodeId.to_string()])
            .expect("test joined Space topology must contain the direct peer");
        let (localRuntime, holder) = testLocalRuntimeWithHolder(storage);
        let (bindingChanges, _) = broadcast::channel(16);
        let bindingStore = Arc::new(TestBindingRuntime::new(
            bindingKey,
            bindingNodeId.to_string(),
        ));
        let bindingRuntime: Arc<dyn CoreNodeBindingRuntime> = bindingStore.clone();
        (
            CoreNodeRouter {
                localCore: Arc::new(localRuntime),
                bindingStore: bindingRuntime,
                localNodeId: localNodeId.to_string(),
                spaceStore,
                bindingChanges,
                _bindingChangeObserver: Arc::new(|_change| {}),
            },
            holder,
            bindingStore,
        )
    }

    /// Creates one real router whose Binding store has no record for the exercised chat.
    #[allow(non_snake_case)]
    fn testCoreNodeRouterWithoutBinding(localNodeId: &str) -> CoreNodeRouter {
        let storage: Arc<dyn RuntimeStorageHost> = Arc::new(TestRuntimeStorageHost::default());
        CoreNodeIdentityStore::new(storage.clone())
            .writeNodeId(localNodeId.to_string())
            .expect("test node identity must be writable");
        CoreNodeRouter::new(testLocalRuntime(storage))
    }

    /// Returns the sender feeding the long-lived local Tokio executor used by host tasks.
    #[allow(non_snake_case)]
    fn testHostAsyncTaskSender() -> &'static tokio::sync::mpsc::UnboundedSender<HostRuntimeAsyncTask>
    {
        static SENDER: OnceLock<tokio::sync::mpsc::UnboundedSender<HostRuntimeAsyncTask>> =
            OnceLock::new();
        SENDER.get_or_init(|| {
            let (sender, mut receiver) =
                tokio::sync::mpsc::unbounded_channel::<HostRuntimeAsyncTask>();
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("create CoreNode route test runtime failed");
                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, async move {
                    while let Some(task) = receiver.recv().await {
                        tokio::task::spawn_local(task());
                    }
                });
            });
            sender
        })
    }

    /// Schedules test host tasks on native threads and one long-lived local Tokio executor.
    #[derive(Clone, Copy, Debug, Default)]
    struct TestHostRuntimeTaskScheduler;

    impl HostRuntimeTaskSchedulerHost for TestHostRuntimeTaskScheduler {
        /// Starts one synchronous test task on a native thread.
        fn scheduleHostRuntimeTask(
            &self,
            _taskName: &str,
            task: HostRuntimeTask,
        ) -> HostResult<()> {
            std::thread::spawn(task);
            Ok(())
        }

        /// Starts one asynchronous test task on the shared local Tokio executor.
        fn scheduleHostRuntimeAsyncTask(
            &self,
            _taskName: &str,
            task: HostRuntimeAsyncTask,
        ) -> HostResult<()> {
            testHostAsyncTaskSender()
                .send(task)
                .map_err(|error| operit_host_api::HostError::new(error.to_string()))
        }

        /// Starts one delayed synchronous test task on a native thread.
        fn scheduleDelayedHostRuntimeTask(
            &self,
            _taskName: &str,
            delayMs: u64,
            task: HostRuntimeTask,
        ) -> HostResult<()> {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(delayMs));
                task();
            });
            Ok(())
        }

        /// Completes one host turn wait immediately in tests.
        fn waitForHostRuntimeTaskTurn(&self) -> operit_host_api::HostRuntimeTurnFuture {
            Box::pin(async { Ok(()) })
        }

        /// Waits for one host delay on the active Tokio timer.
        fn waitForHostRuntimeDelay(&self, delayMs: u64) -> operit_host_api::HostRuntimeTurnFuture {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(delayMs)).await;
                Ok(())
            })
        }
    }

    /// Installs the process-wide host scheduler used by PeerLink and route stream tasks.
    #[allow(non_snake_case)]
    fn installTestRuntimeScheduler() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            setDefaultHostRuntimeTaskSchedulerHost(Arc::new(TestHostRuntimeTaskScheduler));
        });
    }

    /// Returns the process-wide guard for tests that replace global host runtime state.
    #[allow(non_snake_case)]
    fn routeTestGlobalLock() -> &'static tokio::sync::Mutex<()> {
        static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    /// Clears the installed route runtime when an annotation-wrapper test exits.
    struct InstalledCoreRouteRuntimeGuard;

    impl Drop for InstalledCoreRouteRuntimeGuard {
        /// Removes the process-wide route runtime installed by one test.
        fn drop(&mut self) {
            operit_link::clearCoreRouteRuntime();
        }
    }

    /// Installs one route runtime and returns a guard that clears it after the test.
    #[allow(non_snake_case)]
    fn installTestCoreRouteRuntime(
        runtime: Arc<dyn CoreRouteRuntime>,
    ) -> InstalledCoreRouteRuntimeGuard {
        operit_link::installCoreRouteRuntime(runtime);
        InstalledCoreRouteRuntimeGuard
    }

    /// Carries the chat message shape needed by routed message Flow tests.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct RoutedChatMessage {
        text: String,
        contentStream: Option<CoreStream<String>>,
    }

    /// Exposes a SpaceRoute endpoint with a real Flow encoder and stream pool.
    struct TestSpaceEndpoint {
        messages: MutableStateFlow<Vec<RoutedChatMessage>>,
        streamPool: Arc<CoreStreamPool>,
        embeddedStreamOpened: AtomicBool,
    }

    impl TestSpaceEndpoint {
        /// Creates an empty test Space endpoint.
        fn new() -> Arc<Self> {
            Arc::new(Self {
                messages: mutableStateFlow(Vec::new()),
                streamPool: Arc::new(CoreStreamPool::new()),
                embeddedStreamOpened: AtomicBool::new(false),
            })
        }

        /// Publishes one chat message containing a live embedded response stream.
        #[allow(non_snake_case)]
        fn publishMessageWithStream(&self, streamId: &str, text: &str) {
            let source = testTextStreamSource(text.to_string());
            self.messages.set_value(vec![RoutedChatMessage {
                text: text.to_string(),
                contentStream: Some(CoreStream::fromSourceWithId(streamId.to_string(), source)),
            }]);
        }

        /// Creates the attachment adopter used by the Space Flow encoder.
        #[allow(non_snake_case)]
        fn streamAttachmentAdopter(
            &self,
        ) -> Arc<dyn Fn(Vec<operit_link::CoreStreamAttachment>) + Send + Sync> {
            let streamPool = self.streamPool.clone();
            Arc::new(move |attachments| {
                streamPool.adoptAll(attachments);
            })
        }

        /// Opens the routed chat message Flow through the same encoder used by SpaceRuntime.
        #[allow(non_snake_case)]
        fn openChatMessagesFlow(
            &self,
            request: CoreWatchRequest,
        ) -> Result<CoreEventStream, CoreLinkError> {
            operit_rslink_runtime::core_state_flow_event_stream(
                self.messages.asStateFlow(),
                request,
                self.streamAttachmentAdopter(),
            )
        }

        /// Opens an embedded stream from the endpoint-owned stream pool.
        #[allow(non_snake_case)]
        fn openEmbeddedStream(
            &self,
            request: CoreWatchRequest,
        ) -> Result<CoreEventStream, CoreLinkError> {
            self.embeddedStreamOpened.store(true, Ordering::SeqCst);
            self.streamPool.openCoreStreamWatch(request)
        }
    }

    /// Exposes a real Space runtime backed by ChatRuntimeHolder and ChatServiceCore.
    struct TestRealChatSpaceEndpoint {
        holder: Arc<tokio::sync::Mutex<ChatRuntimeHolder>>,
        spaceRuntime: Arc<SpaceRuntime>,
        embeddedStreamOpened: AtomicBool,
    }

    impl TestRealChatSpaceEndpoint {
        /// Creates a Space endpoint that dispatches through generated ChatServiceCore routes.
        fn new() -> Arc<Self> {
            let holder = Arc::new(tokio::sync::Mutex::new(ChatRuntimeHolder::new(Arc::new(
                TestFileSystemHost,
            ))));
            let spaceRuntime = Arc::new(SpaceRuntime::new(holder.clone()));
            Arc::new(Self {
                holder,
                spaceRuntime,
                embeddedStreamOpened: AtomicBool::new(false),
            })
        }

        /// Publishes one real chat message containing a live Markdown content stream.
        #[allow(non_snake_case)]
        async fn publishChatMessageWithStream(&self, chatId: &str, streamId: &str, text: &str) {
            let source = testMarkdownStreamSource(chatId.to_string(), text.to_string());
            let mut message = ChatMessage::new("ai".to_string());
            message.contentStream =
                Some(CoreStream::fromSourceWithId(streamId.to_string(), source));
            let mut holder = self.holder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatHistoryDelegate
                .publishChatMessage(chatId, message);
        }

        /// Publishes one real chat message whose stream receives chunks after clients subscribe.
        #[allow(non_snake_case)]
        async fn publishLiveChatMessageStream(
            &self,
            chatId: &str,
            streamId: &str,
        ) -> DelegatingRevisableSharedTextStream {
            let responseStream = DelegatingRevisableSharedTextStream::new_ordered(
                mutable_shared_stream(usize::MAX),
                mutable_shared_stream(usize::MAX),
            );
            let source = coreResponseStreamSource(responseStream.clone(), streamId.to_string());
            let mut message = ChatMessage::new("ai".to_string());
            message.contentStream =
                Some(CoreStream::fromSourceWithId(streamId.to_string(), source));
            let mut holder = self.holder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatHistoryDelegate
                .publishChatMessage(chatId, message);
            responseStream
        }
    }

    #[async_trait]
    impl CoreNodeTransportClient for TestRealChatSpaceEndpoint {
        /// Rejects direct calls because this endpoint is reached through SpaceRoute.
        async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
            CoreCallResponse::err(
                request.requestId,
                CoreLinkError::new(
                    "UNEXPECTED_TEST_CALL",
                    "direct call is not part of this test",
                ),
            )
        }

        /// Rejects direct snapshots because this endpoint is reached through SpaceRoute.
        #[allow(non_snake_case)]
        async fn watchSnapshot(
            &self,
            request: CoreWatchRequest,
        ) -> Result<CoreEvent, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.registryKey()))
        }

        /// Rejects direct watches because this endpoint is reached through SpaceRoute.
        async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.registryKey()))
        }

        /// Rejects direct push streams because this endpoint is reached through SpaceRoute.
        #[allow(non_snake_case)]
        async fn openPush(
            &self,
            request: CorePushRequest,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_PUSH",
                format!(
                    "direct push is not part of this test: {}",
                    request.methodName
                ),
            ))
        }

        /// Rejects routed calls because this test exercises routed Flow watches.
        #[allow(non_snake_case)]
        async fn routedCall(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreCallRequest>,
        ) -> CoreCallResponse {
            CoreCallResponse::err(
                request.payload.requestId,
                CoreLinkError::new(
                    "UNEXPECTED_TEST_CALL",
                    "routed call is not part of this test",
                ),
            )
        }

        /// Rejects routed handoff because this test exercises routed Flow watches.
        #[allow(non_snake_case)]
        async fn routedHandoff(
            &self,
            _previousNodeId: String,
            _request: RoutedCoreRequest<CoreHandoffRequest>,
        ) -> Result<CoreHandoffResponse, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_HANDOFF",
                "routed handoff is not part of this test",
            ))
        }

        /// Rejects routed snapshots because this test opens streaming watches.
        #[allow(non_snake_case)]
        async fn routedWatchSnapshot(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEvent, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.payload.registryKey()))
        }

        /// Dispatches one routed watch through the real SpaceRuntime.
        #[allow(non_snake_case)]
        async fn routedWatch(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEventStream, CoreLinkError> {
            assert_eq!(request.routeKind, RoutedCoreRequestKind::SpaceRoute);
            let payload = request.payload;
            if payload.targetObjectId == CORE_STREAM_POOL_OBJECT_ID {
                self.embeddedStreamOpened.store(true, Ordering::SeqCst);
            } else {
                assert_eq!(payload.targetObjectId, CORE_INTERNAL_ROUTE_OBJECT_ID);
                assert_eq!(payload.propertyName, "chatMessagesFlow");
            }
            self.spaceRuntime.watch(payload).await
        }

        /// Rejects routed binding application because this test opens Flow watches.
        #[allow(non_snake_case)]
        async fn routedBindingApply(
            &self,
            _previousNodeId: String,
            _request: RoutedCoreRequest<CoreNodeBindingApplyRequest>,
        ) -> Result<(), CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_BINDING",
                "binding apply is not part of this test",
            ))
        }

        /// Rejects routed push streams because this test opens Flow watches.
        #[allow(non_snake_case)]
        async fn routedOpenPush(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CorePushRequest>,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_PUSH",
                format!(
                    "routed push is not part of this test: {}",
                    request.payload.methodName
                ),
            ))
        }
    }

    #[async_trait]
    impl CoreNodeTransportClient for TestSpaceEndpoint {
        /// Rejects direct calls because the test exercises routed watches only.
        async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
            CoreCallResponse::err(
                request.requestId,
                CoreLinkError::new(
                    "UNEXPECTED_TEST_CALL",
                    "direct call is not part of this test",
                ),
            )
        }

        /// Rejects direct watch snapshots because the test exercises routed watches only.
        #[allow(non_snake_case)]
        async fn watchSnapshot(
            &self,
            request: CoreWatchRequest,
        ) -> Result<CoreEvent, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.registryKey()))
        }

        /// Rejects direct watches because the test exercises routed watches only.
        async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.registryKey()))
        }

        /// Rejects direct push streams because the test exercises routed watches only.
        #[allow(non_snake_case)]
        async fn openPush(
            &self,
            request: CorePushRequest,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_PUSH",
                format!(
                    "direct push is not part of this test: {}",
                    request.methodName
                ),
            ))
        }

        /// Rejects routed calls because the test exercises routed watches only.
        #[allow(non_snake_case)]
        async fn routedCall(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreCallRequest>,
        ) -> CoreCallResponse {
            CoreCallResponse::err(
                request.payload.requestId,
                CoreLinkError::new(
                    "UNEXPECTED_TEST_CALL",
                    "routed call is not part of this test",
                ),
            )
        }

        /// Rejects routed handoff because the test exercises routed watches only.
        #[allow(non_snake_case)]
        async fn routedHandoff(
            &self,
            _previousNodeId: String,
            _request: RoutedCoreRequest<CoreHandoffRequest>,
        ) -> Result<CoreHandoffResponse, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_HANDOFF",
                "routed handoff is not part of this test",
            ))
        }

        /// Rejects routed snapshots because the test exercises streaming watches only.
        #[allow(non_snake_case)]
        async fn routedWatchSnapshot(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEvent, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.payload.registryKey()))
        }

        /// Opens one routed Space watch exactly as a target CoreNode would receive it.
        #[allow(non_snake_case)]
        async fn routedWatch(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEventStream, CoreLinkError> {
            assert_eq!(request.routeKind, RoutedCoreRequestKind::SpaceRoute);
            let payload = request.payload;
            if payload.targetObjectId == CORE_STREAM_POOL_OBJECT_ID {
                return self.openEmbeddedStream(payload);
            }
            assert_eq!(payload.targetObjectId, CORE_INTERNAL_ROUTE_OBJECT_ID);
            assert_eq!(payload.propertyName, "chatMessagesFlow");
            self.openChatMessagesFlow(payload)
        }

        /// Rejects routed binding application because the test exercises routed watches only.
        #[allow(non_snake_case)]
        async fn routedBindingApply(
            &self,
            _previousNodeId: String,
            _request: RoutedCoreRequest<CoreNodeBindingApplyRequest>,
        ) -> Result<(), CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_BINDING",
                "binding apply is not part of this test",
            ))
        }

        /// Rejects routed push streams because the test exercises routed watches only.
        #[allow(non_snake_case)]
        async fn routedOpenPush(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CorePushRequest>,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_PUSH",
                format!(
                    "routed push is not part of this test: {}",
                    request.payload.methodName
                ),
            ))
        }
    }

    /// Exposes a real CoreNodeRouter as one PeerLink transport endpoint.
    struct TestCoreNodeRouterEndpoint {
        router: CoreNodeRouter,
    }

    impl TestCoreNodeRouterEndpoint {
        /// Creates a transport endpoint backed by one router clone.
        fn new(router: CoreNodeRouter) -> Arc<Self> {
            Arc::new(Self { router })
        }

        /// Runs one router operation on the host-owned local async executor.
        async fn run_on_router_executor<T>(
            &self,
            taskName: &'static str,
            operation: impl FnOnce(CoreNodeRouter) -> Pin<Box<dyn Future<Output = T> + 'static>>
                + Send
                + 'static,
        ) -> T
        where
            T: Send + 'static,
        {
            let router = self.router.clone();
            let (sender, receiver) = tokio::sync::oneshot::channel();
            HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(
                defaultHostRuntimeTaskSchedulerHost().as_ref(),
                taskName,
                Box::new(move || {
                    Box::pin(async move {
                        let result = operation(router).await;
                        let _ = sender.send(result);
                    })
                }),
            )
            .expect("test router endpoint task must schedule");
            receiver
                .await
                .expect("test router endpoint task must return a result")
        }
    }

    #[async_trait]
    impl CoreNodeTransportClient for TestCoreNodeRouterEndpoint {
        /// Executes a direct Core call through the wrapped router.
        async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
            self.run_on_router_executor("test-router-direct-call", move |router| {
                Box::pin(async move { CoreLinkSharedClient::call(&router, request).await })
            })
            .await
        }

        /// Reads a direct Core watch snapshot through the wrapped router.
        #[allow(non_snake_case)]
        async fn watchSnapshot(
            &self,
            request: CoreWatchRequest,
        ) -> Result<CoreEvent, CoreLinkError> {
            self.run_on_router_executor("test-router-direct-watch-snapshot", move |router| {
                Box::pin(async move { CoreLinkSharedClient::watchSnapshot(&router, request).await })
            })
            .await
        }

        /// Opens a direct Core watch through the wrapped router.
        async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
            self.run_on_router_executor("test-router-direct-watch", move |router| {
                Box::pin(async move { CoreLinkSharedClient::watch(&router, request).await })
            })
            .await
        }

        /// Opens a direct Core push through the wrapped router.
        #[allow(non_snake_case)]
        async fn openPush(
            &self,
            request: CorePushRequest,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            self.run_on_router_executor("test-router-direct-push", move |mut router| {
                Box::pin(async move { CoreLinkClient::openPush(&mut router, request).await })
            })
            .await
        }

        /// Executes a routed Core call through the wrapped router.
        #[allow(non_snake_case)]
        async fn routedCall(
            &self,
            previousNodeId: String,
            request: RoutedCoreRequest<CoreCallRequest>,
        ) -> CoreCallResponse {
            self.run_on_router_executor("test-router-routed-call", move |mut router| {
                Box::pin(async move { router.routedCall(previousNodeId, request).await })
            })
            .await
        }

        /// Executes a routed handoff through the wrapped router.
        #[allow(non_snake_case)]
        async fn routedHandoff(
            &self,
            previousNodeId: String,
            request: RoutedCoreRequest<CoreHandoffRequest>,
        ) -> Result<CoreHandoffResponse, CoreLinkError> {
            self.run_on_router_executor("test-router-routed-handoff", move |mut router| {
                Box::pin(async move { router.routedHandoff(previousNodeId, request).await })
            })
            .await
        }

        /// Reads a routed watch snapshot through the wrapped router.
        #[allow(non_snake_case)]
        async fn routedWatchSnapshot(
            &self,
            previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEvent, CoreLinkError> {
            self.run_on_router_executor("test-router-routed-watch-snapshot", move |mut router| {
                Box::pin(async move { router.routedWatchSnapshot(previousNodeId, request).await })
            })
            .await
        }

        /// Opens a routed watch through the wrapped router.
        #[allow(non_snake_case)]
        async fn routedWatch(
            &self,
            previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEventStream, CoreLinkError> {
            self.run_on_router_executor("test-router-routed-watch", move |mut router| {
                Box::pin(async move { router.routedWatch(previousNodeId, request).await })
            })
            .await
        }

        /// Applies a routed Binding operation through the wrapped router.
        #[allow(non_snake_case)]
        async fn routedBindingApply(
            &self,
            previousNodeId: String,
            request: RoutedCoreRequest<CoreNodeBindingApplyRequest>,
        ) -> Result<(), CoreLinkError> {
            self.run_on_router_executor("test-router-routed-binding-apply", move |mut router| {
                Box::pin(async move { router.routedBindingApply(previousNodeId, request).await })
            })
            .await
        }

        /// Opens a routed push through the wrapped router.
        #[allow(non_snake_case)]
        async fn routedOpenPush(
            &self,
            previousNodeId: String,
            request: RoutedCoreRequest<CorePushRequest>,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            self.run_on_router_executor("test-router-routed-push", move |mut router| {
                Box::pin(async move { router.routedOpenPush(previousNodeId, request).await })
            })
            .await
        }
    }

    /// Provides an inert local endpoint for the requesting side of the PeerLink.
    struct TestClientEndpoint;

    #[async_trait]
    impl CoreNodeTransportClient for TestClientEndpoint {
        /// Rejects direct calls because the client endpoint only receives peer events.
        async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
            CoreCallResponse::err(
                request.requestId,
                CoreLinkError::new(
                    "UNEXPECTED_TEST_CALL",
                    "client call is not part of this test",
                ),
            )
        }

        /// Rejects direct watch snapshots because the client endpoint only receives peer events.
        #[allow(non_snake_case)]
        async fn watchSnapshot(
            &self,
            request: CoreWatchRequest,
        ) -> Result<CoreEvent, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.registryKey()))
        }

        /// Rejects direct watches because the client endpoint only receives peer events.
        async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.registryKey()))
        }

        /// Rejects direct pushes because the client endpoint only receives peer events.
        #[allow(non_snake_case)]
        async fn openPush(
            &self,
            request: CorePushRequest,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_PUSH",
                format!(
                    "client push is not part of this test: {}",
                    request.methodName
                ),
            ))
        }

        /// Rejects routed calls because the client endpoint only receives peer events.
        #[allow(non_snake_case)]
        async fn routedCall(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreCallRequest>,
        ) -> CoreCallResponse {
            CoreCallResponse::err(
                request.payload.requestId,
                CoreLinkError::new(
                    "UNEXPECTED_TEST_CALL",
                    "client routed call is not part of this test",
                ),
            )
        }

        /// Rejects routed handoff because the client endpoint only receives peer events.
        #[allow(non_snake_case)]
        async fn routedHandoff(
            &self,
            _previousNodeId: String,
            _request: RoutedCoreRequest<CoreHandoffRequest>,
        ) -> Result<CoreHandoffResponse, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_HANDOFF",
                "client routed handoff is not part of this test",
            ))
        }

        /// Rejects routed snapshots because the client endpoint only receives peer events.
        #[allow(non_snake_case)]
        async fn routedWatchSnapshot(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEvent, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.payload.registryKey()))
        }

        /// Rejects routed watches because the client endpoint only receives peer events.
        #[allow(non_snake_case)]
        async fn routedWatch(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEventStream, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.payload.registryKey()))
        }

        /// Rejects routed binding application because the client endpoint only receives peer events.
        #[allow(non_snake_case)]
        async fn routedBindingApply(
            &self,
            _previousNodeId: String,
            _request: RoutedCoreRequest<CoreNodeBindingApplyRequest>,
        ) -> Result<(), CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_BINDING",
                "client binding apply is not part of this test",
            ))
        }

        /// Rejects routed push streams because the client endpoint only receives peer events.
        #[allow(non_snake_case)]
        async fn routedOpenPush(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CorePushRequest>,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_PUSH",
                format!(
                    "client routed push is not part of this test: {}",
                    request.payload.methodName
                ),
            ))
        }
    }

    /// Routes annotation watches through a real in-memory PeerLink.
    struct TestPeerRouteRuntime {
        localNodeId: String,
        targetNodeId: String,
        spaceId: String,
    }

    impl CoreRouteRuntime for TestPeerRouteRuntime {
        /// Reports that every test route should cross the PeerLink.
        fn shouldRoute(&self, _methodName: &str, _args: &CoreValue) -> Result<bool, CoreLinkError> {
            Ok(true)
        }

        /// Rejects calls because the test exercises routed StateFlow watches only.
        fn call(
            &self,
            request: CoreCallRequest,
        ) -> Pin<Box<dyn Future<Output = CoreCallResponse>>> {
            Box::pin(async move {
                CoreCallResponse::err(
                    request.requestId,
                    CoreLinkError::new(
                        "UNEXPECTED_TEST_CALL",
                        "route call is not part of this test",
                    ),
                )
            })
        }

        /// Opens one routed Space watch through the registered in-memory PeerLink.
        fn watch(
            &self,
            request: CoreWatchRequest,
        ) -> Pin<Box<dyn Future<Output = Result<CoreEventStream, CoreLinkError>>>> {
            let localNodeId = self.localNodeId.clone();
            let targetNodeId = self.targetNodeId.clone();
            let spaceId = self.spaceId.clone();
            Box::pin(async move {
                let peer = peerLink(&localNodeId, &targetNodeId)
                    .map_err(|error| CoreLinkError::new("PEER_LINK_CLOSED", error))?;
                peer.routedWatch(RoutedCoreRequest {
                    spaceId,
                    targetNodeId,
                    ttl: 2,
                    routeKind: RoutedCoreRequestKind::SpaceRoute,
                    payload: request,
                })
                .await
            })
        }
    }

    /// Creates one Core stream source that emits a single text chunk and a completion event.
    #[allow(non_snake_case)]
    fn testTextStreamSource(text: String) -> Arc<CoreStreamSource> {
        Arc::new(CoreStreamSource::new(move |request| {
            let (sender, receiver) = CoreEventStream::channel();
            let text = text.clone();
            defaultHostRuntimeTaskSchedulerHost()
                .scheduleHostRuntimeAsyncTask(
                    "core-node-route-test-text-stream",
                    Box::new(move || {
                        Box::pin(async move {
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId.clone()),
                                targetObjectId: request.targetObjectId,
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value: CoreValue::String(text),
                            });
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId),
                                targetObjectId: request.targetObjectId,
                                propertyName: request.propertyName,
                                kind: CoreEventKind::Completed,
                                value: CoreValue::Null,
                            });
                        })
                    }),
                )
                .map_err(|error| CoreLinkError::internal(error.to_string()))?;
            Ok(receiver)
        }))
    }

    /// Creates one Core stream source that emits a single Markdown chunk and completion event.
    #[allow(non_snake_case)]
    fn testMarkdownStreamSource(chatId: String, text: String) -> Arc<CoreStreamSource> {
        Arc::new(CoreStreamSource::new(move |request| {
            let (sender, receiver) = CoreEventStream::channel();
            let chatId = chatId.clone();
            let text = text.clone();
            defaultHostRuntimeTaskSchedulerHost()
                .scheduleHostRuntimeAsyncTask(
                    "core-node-route-test-markdown-stream",
                    Box::new(move || {
                        Box::pin(async move {
                            let event = MarkdownStreamEvent {
                                chatId,
                                eventType: "chunk".to_string(),
                                value: Some(text),
                                id: None,
                                blockId: None,
                                inlineId: None,
                                parentBlockId: None,
                                nodeType: None,
                                headerLevel: None,
                            };
                            let value = operit_link::toCoreValue(event)
                                .expect("Markdown stream event must encode");
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId.clone()),
                                targetObjectId: request.targetObjectId,
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value,
                            });
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId),
                                targetObjectId: request.targetObjectId,
                                propertyName: request.propertyName,
                                kind: CoreEventKind::Completed,
                                value: CoreValue::Null,
                            });
                        })
                    }),
                )
                .map_err(|error| CoreLinkError::internal(error.to_string()))?;
            Ok(receiver)
        }))
    }

    /// Creates one Core stream source that emits a complete structured Markdown event sequence.
    #[allow(non_snake_case)]
    fn testStructuredMarkdownStreamSource(
        chatId: String,
        events: Vec<MarkdownStreamEvent>,
    ) -> Arc<CoreStreamSource> {
        Arc::new(CoreStreamSource::new(move |request| {
            let (sender, receiver) = CoreEventStream::channel();
            let events = events.clone();
            let requestId = request.requestId.clone();
            let targetObjectId = request.targetObjectId;
            let propertyName = request.propertyName.clone();
            defaultHostRuntimeTaskSchedulerHost()
                .scheduleHostRuntimeAsyncTask(
                    "core-node-route-test-structured-markdown-stream",
                    Box::new(move || {
                        Box::pin(async move {
                            for event in events {
                                let value = operit_link::toCoreValue(event)
                                    .expect("structured Markdown stream event must encode");
                                let _ = sender.send(CoreEvent {
                                    requestId: Some(requestId.clone()),
                                    targetObjectId,
                                    propertyName: propertyName.clone(),
                                    kind: CoreEventKind::Changed,
                                    value,
                                });
                            }
                            let _ = sender.send(CoreEvent {
                                requestId: Some(requestId),
                                targetObjectId,
                                propertyName,
                                kind: CoreEventKind::Completed,
                                value: CoreValue::Null,
                            });
                        })
                    }),
                )
                .map_err(|error| CoreLinkError::internal(error.to_string()))?;
            Ok(receiver)
        }))
    }

    /// Waits until the routed StateFlow contains the published message.
    #[allow(non_snake_case)]
    fn waitForRoutedMessages(flow: &StateFlow<Vec<RoutedChatMessage>>) -> Vec<RoutedChatMessage> {
        for _ in 0..100 {
            let messages = flow.value();
            if !messages.is_empty() {
                return messages;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("routed chat message Flow did not receive the published message");
    }

    /// Waits until the real routed chat message Flow contains a live content stream.
    #[allow(non_snake_case)]
    async fn waitForRoutedChatMessages(flow: &StateFlow<Vec<ChatMessage>>) -> Vec<ChatMessage> {
        for _ in 0..100 {
            let messages = flow.value();
            if messages
                .iter()
                .any(|message| message.contentStream.is_some())
            {
                return messages;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("real routed chat message Flow did not receive the live content stream");
    }

    /// Waits until the real routed chat message Flow contains the selected live content stream.
    #[allow(non_snake_case)]
    async fn waitForRoutedChatMessageStream(
        flow: &StateFlow<Vec<ChatMessage>>,
        streamId: &str,
    ) -> CoreStream<MarkdownStreamEvent> {
        for _ in 0..100 {
            let messages = flow.value();
            if let Some(stream) = messages.iter().find_map(|message| {
                message
                    .contentStream
                    .as_ref()
                    .filter(|stream| stream.descriptor.streamId == streamId)
                    .cloned()
            }) {
                return stream;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("real routed chat message Flow did not receive stream {streamId}");
    }

    /// Publishes one live Markdown stream message through the selected test holder.
    #[allow(non_snake_case)]
    async fn publishHolderChatMessageWithStream(
        holder: &Arc<tokio::sync::Mutex<ChatRuntimeHolder>>,
        chatId: &str,
        streamId: &str,
        text: &str,
    ) {
        let source = testMarkdownStreamSource(chatId.to_string(), text.to_string());
        let mut message = ChatMessage::new("ai".to_string());
        message.contentStream = Some(CoreStream::fromSourceWithId(streamId.to_string(), source));
        let mut holder = holder.lock().await;
        let core = holder.getCore(ChatRuntimeSlot::MAIN);
        let _flow = core
            .chatHistoryDelegate
            .chatMessageFlowForChat(chatId.to_string());
        core.chatHistoryDelegate.publishChatMessage(chatId, message);
    }

    /// Opens one routed Markdown stream and verifies its first chunk and completion.
    #[allow(non_snake_case)]
    async fn assertRoutedMarkdownStreamChunk(
        stream: CoreStream<MarkdownStreamEvent>,
        requestId: &str,
        expectedChatId: &str,
        expectedText: &str,
    ) {
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                requestId,
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("routed embedded stream must reopen through CoreNodeRouter");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        let event: MarkdownStreamEvent =
            operit_link::fromCoreValue(chunk.value).expect("Markdown stream event must decode");
        assert_eq!(event.chatId, expectedChatId);
        assert_eq!(event.eventType, "chunk");
        assert_eq!(event.value, Some(expectedText.to_string()));
        let completed = receiveEvent(&mut openedStream).await;
        assert_eq!(completed.kind, CoreEventKind::Completed);
    }

    /// Waits until the routed chat state Flow reports the selected input-processing state.
    #[allow(non_snake_case)]
    async fn waitForRoutedChatState(
        flow: &StateFlow<ChatState>,
        expectedState: &InputProcessingState,
    ) -> ChatState {
        for _ in 0..100 {
            let state = flow.value();
            if &state.inputProcessingState == expectedState {
                return state;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("routed chat state Flow did not receive the selected input-processing state");
    }

    /// Receives one event from a Core event stream.
    #[allow(non_snake_case)]
    async fn receiveEvent(stream: &mut CoreEventStream) -> CoreEvent {
        stream
            .recv()
            .await
            .expect("Core event stream ended before the expected event")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn routed_space_flow_embedded_stream_opens_through_peer_link_pool() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-client".to_string();
        let targetNodeId = "core-node-source".to_string();
        let spaceId = "space-route-test".to_string();
        let source = TestSpaceEndpoint::new();
        let client = Arc::new(TestClientEndpoint);
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            client,
            targetNodeId.clone(),
            source.clone(),
        )
        .expect("in-memory PeerLink must connect test endpoints");
        let routeRuntime = Arc::new(TestPeerRouteRuntime {
            localNodeId,
            targetNodeId,
            spaceId,
        });
        let args = CoreValue::Map(BTreeMap::from([(
            "chatId".to_string(),
            CoreValue::String("chat-a".to_string()),
        )]));
        let routedFlow = operit_rslink_runtime::core_route_state_flow::<Vec<RoutedChatMessage>>(
            routeRuntime,
            "chatMessagesFlow",
            args,
        )
        .await
        .expect("routed chat message Flow must open through PeerLink");
        source.publishMessageWithStream("chat-message-stream:route-test", "hello from source");
        let messages = waitForRoutedMessages(&routedFlow);
        let stream = messages[0]
            .contentStream
            .clone()
            .expect("routed message must carry an embedded content stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        let attachments = attachments;
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("embedded stream must open through the remote Space stream pool");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        assert_eq!(
            chunk.value,
            CoreValue::String("hello from source".to_string())
        );
        loop {
            let event = receiveEvent(&mut openedStream).await;
            if event.kind == CoreEventKind::Completed {
                break;
            }
            assert_eq!(event.kind, CoreEventKind::Changed);
        }
        assert!(source.embeddedStreamOpened.load(Ordering::SeqCst));
        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn routed_space_flow_embedded_stream_reopens_through_real_core_node_router() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-router-client".to_string();
        let targetNodeId = "core-node-router-source".to_string();
        let chatId = "chat-router-a".to_string();
        let router = testCoreNodeRouter(&localNodeId, &targetNodeId, &chatId);
        let source = TestSpaceEndpoint::new();
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            Arc::new(TestClientEndpoint),
            targetNodeId.clone(),
            source.clone(),
        )
        .expect("in-memory PeerLink must connect router and Space endpoint");
        let routeRuntime: Arc<dyn CoreRouteRuntime> = Arc::new(router);
        let args = CoreValue::Map(BTreeMap::from([(
            "chatId".to_string(),
            CoreValue::String(chatId),
        )]));
        let routedFlow = operit_rslink_runtime::core_route_state_flow::<Vec<RoutedChatMessage>>(
            routeRuntime,
            "chatMessagesFlow",
            args,
        )
        .await
        .expect("routed chat message Flow must open through the real CoreNodeRouter");
        source.publishMessageWithStream(
            "chat-message-stream:real-router-route-test",
            "hello through real router",
        );
        let messages = waitForRoutedMessages(&routedFlow);
        let stream = messages[0]
            .contentStream
            .clone()
            .expect("routed message must carry an embedded content stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-real-router-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("embedded stream must reopen through the real CoreNodeRouter");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        assert_eq!(
            chunk.value,
            CoreValue::String("hello through real router".to_string())
        );
        let completed = receiveEvent(&mut openedStream).await;
        assert_eq!(completed.kind, CoreEventKind::Completed);
        assert!(source.embeddedStreamOpened.load(Ordering::SeqCst));
        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn routed_real_chat_messages_flow_embedded_stream_reopens_through_real_core_node_router()
    {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-real-chat-client".to_string();
        let targetNodeId = "core-node-real-chat-source".to_string();
        let chatId = "chat-real-flow-a".to_string();
        let router = testCoreNodeRouter(&localNodeId, &targetNodeId, &chatId);
        let source = TestRealChatSpaceEndpoint::new();
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            Arc::new(TestClientEndpoint),
            targetNodeId.clone(),
            source.clone(),
        )
        .expect("in-memory PeerLink must connect router and real Space endpoint");
        let routeRuntime: Arc<dyn CoreRouteRuntime> = Arc::new(router);
        let args = CoreValue::Map(BTreeMap::from([(
            "chatId".to_string(),
            CoreValue::String(chatId.clone()),
        )]));
        let routedFlow = operit_rslink_runtime::core_route_state_flow::<Vec<ChatMessage>>(
            routeRuntime,
            "chatMessagesFlow",
            args,
        )
        .await
        .expect("real routed chat message Flow must open through the real CoreNodeRouter");
        source
            .publishChatMessageWithStream(
                &chatId,
                "chat-message-stream:real-chat-route-test",
                "hello from real ChatMessage",
            )
            .await;
        let messages = waitForRoutedChatMessages(&routedFlow).await;
        let stream = messages
            .iter()
            .find_map(|message| message.contentStream.clone())
            .expect("real routed message must carry an embedded content stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-real-chat-router-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("real embedded ChatMessage stream must reopen through the real CoreNodeRouter");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        let event: MarkdownStreamEvent =
            operit_link::fromCoreValue(chunk.value).expect("Markdown stream event must decode");
        assert_eq!(event.chatId, chatId);
        assert_eq!(event.eventType, "chunk");
        assert_eq!(event.value, Some("hello from real ChatMessage".to_string()));
        let completed = receiveEvent(&mut openedStream).await;
        assert_eq!(completed.kind, CoreEventKind::Completed);
        assert!(source.embeddedStreamOpened.load(Ordering::SeqCst));
        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn routed_real_chat_messages_flow_embedded_stream_receives_live_chunks_after_open() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-real-chat-live-client".to_string();
        let targetNodeId = "core-node-real-chat-live-source".to_string();
        let chatId = "chat-real-flow-live".to_string();
        let router = testCoreNodeRouter(&localNodeId, &targetNodeId, &chatId);
        let source = TestRealChatSpaceEndpoint::new();
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            Arc::new(TestClientEndpoint),
            targetNodeId.clone(),
            source.clone(),
        )
        .expect("in-memory PeerLink must connect router and live Space endpoint");
        let routeRuntime: Arc<dyn CoreRouteRuntime> = Arc::new(router);
        let args = CoreValue::Map(BTreeMap::from([(
            "chatId".to_string(),
            CoreValue::String(chatId.clone()),
        )]));
        let routedFlow = operit_rslink_runtime::core_route_state_flow::<Vec<ChatMessage>>(
            routeRuntime,
            "chatMessagesFlow",
            args,
        )
        .await
        .expect("real routed chat message Flow must open through the real CoreNodeRouter");
        let liveStream = source
            .publishLiveChatMessageStream(&chatId, "chat-message-stream:real-chat-live-route-test")
            .await;
        let messages = waitForRoutedChatMessages(&routedFlow).await;
        let stream = messages
            .iter()
            .find_map(|message| message.contentStream.clone())
            .expect("real routed live message must carry an embedded content stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-real-chat-live-router-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("real live embedded ChatMessage stream must reopen through CoreNodeRouter");
        liveStream.emit_chunk("hello after live open".to_string());
        liveStream.close();
        let reset = receiveEvent(&mut openedStream).await;
        assert_eq!(reset.kind, CoreEventKind::Changed);
        let resetEvent: MarkdownStreamEvent =
            operit_link::fromCoreValue(reset.value).expect("Markdown reset event must decode");
        assert_eq!(resetEvent.eventType, "reset");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        let event: MarkdownStreamEvent =
            operit_link::fromCoreValue(chunk.value).expect("Markdown chunk event must decode");
        assert_eq!(
            event.chatId,
            "chat-message-stream:real-chat-live-route-test"
        );
        assert_eq!(event.eventType, "chunk");
        assert_eq!(event.value, Some("hello after live open".to_string()));
        loop {
            let event = receiveEvent(&mut openedStream).await;
            if event.kind == CoreEventKind::Completed {
                break;
            }
            assert_eq!(event.kind, CoreEventKind::Changed);
        }
        assert!(source.embeddedStreamOpened.load(Ordering::SeqCst));
        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_chat_messages_flow_routes_between_real_core_node_routers() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-two-router-client".to_string();
        let targetNodeId = "core-node-two-router-source".to_string();
        let chatId = "chat-two-router-flow-a".to_string();
        let joinedSpace = CoreSpace {
            spaceId: "space-two-router-route-test".to_string(),
            spaceName: "Route Test Space".to_string(),
            spaceRevision: 2,
            members: vec![localNodeId.clone(), targetNodeId.clone()],
        };
        let (localRouter, localHolder) = testCoreNodeRouterInJoinedSpace(
            &localNodeId,
            &targetNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace.clone(),
        );
        let (targetRouter, targetHolder) = testCoreNodeRouterInJoinedSpace(
            &targetNodeId,
            &localNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace,
        );
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(localRouter.clone()),
            targetNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(targetRouter),
        )
        .expect("in-memory PeerLink must connect both real CoreNodeRouters");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(localRouter));
        let routedFlow = {
            let mut holder = localHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatMessagesFlow(chatId.clone())
                .await
        };
        {
            let source = testMarkdownStreamSource(
                chatId.clone(),
                "hello between real CoreNodeRouters".to_string(),
            );
            let mut message = ChatMessage::new("ai".to_string());
            message.contentStream = Some(CoreStream::fromSourceWithId(
                "chat-message-stream:two-router-route-test".to_string(),
                source,
            ));
            let mut holder = targetHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatHistoryDelegate
                .publishChatMessage(&chatId, message);
        }
        let messages = waitForRoutedChatMessages(&routedFlow).await;
        let stream = messages
            .iter()
            .find_map(|message| message.contentStream.clone())
            .expect("two-router routed message must carry an embedded content stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-two-router-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("two-router embedded stream must reopen through CoreNodeRouter");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        let event: MarkdownStreamEvent =
            operit_link::fromCoreValue(chunk.value).expect("Markdown stream event must decode");
        assert_eq!(event.chatId, chatId);
        assert_eq!(event.eventType, "chunk");
        assert_eq!(
            event.value,
            Some("hello between real CoreNodeRouters".to_string())
        );
        let completed = receiveEvent(&mut openedStream).await;
        assert_eq!(completed.kind, CoreEventKind::Completed);
        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_route_preserves_structured_embedded_stream_events() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-structured-client".to_string();
        let targetNodeId = "core-node-structured-source".to_string();
        let chatId = "chat-structured-route-a".to_string();
        let joinedSpace = CoreSpace {
            spaceId: "space-structured-route-test".to_string(),
            spaceName: "Structured Route Test Space".to_string(),
            spaceRevision: 2,
            members: vec![localNodeId.clone(), targetNodeId.clone()],
        };
        let (localRouter, localHolder) = testCoreNodeRouterInJoinedSpace(
            &localNodeId,
            &targetNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace.clone(),
        );
        let (targetRouter, targetHolder) = testCoreNodeRouterInJoinedSpace(
            &targetNodeId,
            &localNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace,
        );
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(localRouter.clone()),
            targetNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(targetRouter),
        )
        .expect("in-memory PeerLink must connect both structured CoreNodes");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(localRouter));
        let expectedEvents = vec![
            MarkdownStreamEvent {
                chatId: chatId.clone(),
                eventType: "reset".to_string(),
                value: None,
                id: None,
                blockId: None,
                inlineId: None,
                parentBlockId: None,
                nodeType: None,
                headerLevel: None,
            },
            MarkdownStreamEvent {
                chatId: chatId.clone(),
                eventType: "markdownBlockStart".to_string(),
                value: Some("heading".to_string()),
                id: Some("block-7".to_string()),
                blockId: Some(7),
                inlineId: None,
                parentBlockId: None,
                nodeType: Some("Heading".to_string()),
                headerLevel: Some(2),
            },
            MarkdownStreamEvent {
                chatId: chatId.clone(),
                eventType: "markdownInlineStart".to_string(),
                value: None,
                id: Some("inline-3".to_string()),
                blockId: Some(7),
                inlineId: Some(3),
                parentBlockId: Some(7),
                nodeType: Some("Strong".to_string()),
                headerLevel: None,
            },
            MarkdownStreamEvent {
                chatId: chatId.clone(),
                eventType: "markdownInlineChunk".to_string(),
                value: Some("lossless remote chunk".to_string()),
                id: None,
                blockId: Some(7),
                inlineId: Some(3),
                parentBlockId: Some(7),
                nodeType: Some("Strong".to_string()),
                headerLevel: None,
            },
            MarkdownStreamEvent {
                chatId: chatId.clone(),
                eventType: "savepoint".to_string(),
                value: None,
                id: Some("revision-1".to_string()),
                blockId: None,
                inlineId: None,
                parentBlockId: None,
                nodeType: None,
                headerLevel: None,
            },
            MarkdownStreamEvent {
                chatId,
                eventType: "completed".to_string(),
                value: None,
                id: None,
                blockId: Some(7),
                inlineId: None,
                parentBlockId: None,
                nodeType: None,
                headerLevel: None,
            },
        ];
        let routedFlow = {
            let mut holder = localHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatMessagesFlow("chat-structured-route-a".to_string())
                .await
        };
        {
            let source = testStructuredMarkdownStreamSource(
                "chat-structured-route-a".to_string(),
                expectedEvents.clone(),
            );
            let mut message = ChatMessage::new("ai".to_string());
            message.contentStream = Some(CoreStream::fromSourceWithId(
                "chat-message-stream:structured-route-test".to_string(),
                source,
            ));
            let mut holder = targetHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatHistoryDelegate
                .publishChatMessage(&"chat-structured-route-a".to_string(), message);
        }
        let messages = waitForRoutedChatMessages(&routedFlow).await;
        let stream = messages
            .iter()
            .find_map(|message| message.contentStream.clone())
            .expect("structured routed message must carry an embedded stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-structured-route-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("structured embedded stream must reopen through CoreNodeRouter");
        let mut receivedEvents = Vec::new();
        loop {
            let event = receiveEvent(&mut openedStream).await;
            if event.kind == CoreEventKind::Completed {
                break;
            }
            assert_eq!(event.kind, CoreEventKind::Changed);
            receivedEvents.push(
                operit_link::fromCoreValue::<MarkdownStreamEvent>(event.value)
                    .expect("structured Markdown event must decode"),
            );
        }
        let receivedValues = receivedEvents
            .into_iter()
            .map(|event| operit_link::toCoreValue(event).expect("received event must encode"))
            .collect::<Vec<_>>();
        let expectedValues = expectedEvents
            .into_iter()
            .map(|event| operit_link::toCoreValue(event).expect("expected event must encode"))
            .collect::<Vec<_>>();
        assert_eq!(receivedValues, expectedValues);
        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_chat_messages_flow_rebinds_after_binding_owner_changes() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-rebind-client".to_string();
        let targetNodeId = "core-node-rebind-source".to_string();
        let chatId = "chat-rebind-flow-a".to_string();
        let joinedSpace = CoreSpace {
            spaceId: "space-rebind-route-test".to_string(),
            spaceName: "Route Rebind Test Space".to_string(),
            spaceRevision: 2,
            members: vec![localNodeId.clone(), targetNodeId.clone()],
        };
        let (localRouter, localHolder, localBinding) =
            testCoreNodeRouterInJoinedSpaceWithBindingRuntime(
                &localNodeId,
                &targetNodeId,
                &chatId,
                &targetNodeId,
                joinedSpace.clone(),
            );
        let (targetRouter, targetHolder) = testCoreNodeRouterInJoinedSpace(
            &targetNodeId,
            &localNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace,
        );
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(localRouter.clone()),
            targetNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(targetRouter),
        )
        .expect("in-memory PeerLink must connect both real CoreNodeRouters");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(localRouter.clone()));
        let routedFlow = {
            let mut holder = localHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatMessagesFlow(chatId.clone())
                .await
        };
        assert!(routedFlow.value().is_empty());
        {
            let source = testMarkdownStreamSource(
                chatId.clone(),
                "hello after binding owner moved".to_string(),
            );
            let mut message = ChatMessage::new("ai".to_string());
            message.contentStream = Some(CoreStream::fromSourceWithId(
                "chat-message-stream:binding-rebind-route-test".to_string(),
                source,
            ));
            let mut holder = localHolder.lock().await;
            let core = holder.getCore(ChatRuntimeSlot::MAIN);
            let _localFlow = core
                .chatHistoryDelegate
                .chatMessageFlowForChat(chatId.clone());
            core.chatHistoryDelegate
                .publishChatMessage(&chatId, message);
        }
        let binding = localBinding.setNodeId(localNodeId.clone());
        assert_eq!(binding.nodeId, localNodeId);
        let _ = localRouter.bindingChanges.send(chatId.clone());
        let messages = waitForRoutedChatMessages(&routedFlow).await;
        let stream = messages
            .iter()
            .find_map(|message| message.contentStream.clone())
            .expect("rebound routed message must carry an embedded content stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-binding-rebind-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("rebound embedded stream must reopen through CoreNodeRouter");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        let event: MarkdownStreamEvent =
            operit_link::fromCoreValue(chunk.value).expect("Markdown stream event must decode");
        assert_eq!(event.chatId, chatId);
        assert_eq!(event.eventType, "chunk");
        assert_eq!(
            event.value,
            Some("hello after binding owner moved".to_string())
        );
        let completed = receiveEvent(&mut openedStream).await;
        assert_eq!(completed.kind, CoreEventKind::Completed);
        peerHandle.close();
    }

    /// Verifies an annotated chat message Flow follows a Binding owner moving from local to remote.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_chat_messages_flow_rebinds_after_local_owner_moves_remote() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-local-remote-rebind-client".to_string();
        let targetNodeId = "core-node-local-remote-rebind-source".to_string();
        let chatId = "chat-local-remote-rebind-flow-a".to_string();
        let joinedSpace = CoreSpace {
            spaceId: "space-local-remote-rebind-route-test".to_string(),
            spaceName: "Route Local Remote Rebind Test Space".to_string(),
            spaceRevision: 2,
            members: vec![localNodeId.clone(), targetNodeId.clone()],
        };
        let (localRouter, localHolder, localBinding) =
            testCoreNodeRouterInJoinedSpaceWithBindingRuntime(
                &localNodeId,
                &targetNodeId,
                &chatId,
                &localNodeId,
                joinedSpace.clone(),
            );
        let (targetRouter, targetHolder) = testCoreNodeRouterInJoinedSpace(
            &targetNodeId,
            &localNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace,
        );
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(localRouter.clone()),
            targetNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(targetRouter),
        )
        .expect("in-memory PeerLink must connect both real CoreNodeRouters");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(localRouter.clone()));
        let routedFlow = {
            let mut holder = localHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatMessagesFlow(chatId.clone())
                .await
        };
        assert!(routedFlow.value().is_empty());
        let binding = localBinding.setNodeId(targetNodeId.clone());
        assert_eq!(binding.nodeId, targetNodeId);
        let _ = localRouter.bindingChanges.send(chatId.clone());
        {
            let source = testMarkdownStreamSource(
                chatId.clone(),
                "hello after local owner moved remote".to_string(),
            );
            let mut message = ChatMessage::new("ai".to_string());
            message.contentStream = Some(CoreStream::fromSourceWithId(
                "chat-message-stream:local-remote-rebind-route-test".to_string(),
                source,
            ));
            let mut holder = targetHolder.lock().await;
            let core = holder.getCore(ChatRuntimeSlot::MAIN);
            let _targetFlow = core
                .chatHistoryDelegate
                .chatMessageFlowForChat(chatId.clone());
            core.chatHistoryDelegate
                .publishChatMessage(&chatId, message);
        }
        let messages = waitForRoutedChatMessages(&routedFlow).await;
        let stream = messages
            .iter()
            .find_map(|message| message.contentStream.clone())
            .expect("local-remote rebound message must carry an embedded content stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-local-remote-binding-rebind-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("local-remote rebound embedded stream must reopen through CoreNodeRouter");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        let event: MarkdownStreamEvent =
            operit_link::fromCoreValue(chunk.value).expect("Markdown stream event must decode");
        assert_eq!(event.chatId, chatId);
        assert_eq!(event.eventType, "chunk");
        assert_eq!(
            event.value,
            Some("hello after local owner moved remote".to_string())
        );
        let completed = receiveEvent(&mut openedStream).await;
        assert_eq!(completed.kind, CoreEventKind::Completed);
        peerHandle.close();
    }

    /// Verifies an annotated chat message Flow follows repeated Binding owner changes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_chat_messages_flow_rebinds_across_multiple_owner_changes() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-multi-rebind-client".to_string();
        let targetNodeId = "core-node-multi-rebind-source".to_string();
        let chatId = "chat-multi-rebind-flow-a".to_string();
        let joinedSpace = CoreSpace {
            spaceId: "space-multi-rebind-route-test".to_string(),
            spaceName: "Route Multi Rebind Test Space".to_string(),
            spaceRevision: 2,
            members: vec![localNodeId.clone(), targetNodeId.clone()],
        };
        let (localRouter, localHolder, localBinding) =
            testCoreNodeRouterInJoinedSpaceWithBindingRuntime(
                &localNodeId,
                &targetNodeId,
                &chatId,
                &localNodeId,
                joinedSpace.clone(),
            );
        let (targetRouter, targetHolder) = testCoreNodeRouterInJoinedSpace(
            &targetNodeId,
            &localNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace,
        );
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(localRouter.clone()),
            targetNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(targetRouter),
        )
        .expect("in-memory PeerLink must connect both real CoreNodeRouters");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(localRouter.clone()));
        let routedFlow = {
            let mut holder = localHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatMessagesFlow(chatId.clone())
                .await
        };
        assert!(routedFlow.value().is_empty());

        let firstStreamId = "chat-message-stream:multi-rebind-local-1";
        publishHolderChatMessageWithStream(
            &localHolder,
            &chatId,
            firstStreamId,
            "multi rebind local first",
        )
        .await;
        let firstStream = waitForRoutedChatMessageStream(&routedFlow, firstStreamId).await;
        assertRoutedMarkdownStreamChunk(
            firstStream,
            "embedded-open-multi-rebind-local-1",
            &chatId,
            "multi rebind local first",
        )
        .await;

        let binding = localBinding.setNodeId(targetNodeId.clone());
        assert_eq!(binding.nodeId, targetNodeId);
        let _ = localRouter.bindingChanges.send(chatId.clone());
        let secondStreamId = "chat-message-stream:multi-rebind-remote-1";
        publishHolderChatMessageWithStream(
            &targetHolder,
            &chatId,
            secondStreamId,
            "multi rebind remote first",
        )
        .await;
        let secondStream = waitForRoutedChatMessageStream(&routedFlow, secondStreamId).await;
        assertRoutedMarkdownStreamChunk(
            secondStream,
            "embedded-open-multi-rebind-remote-1",
            &chatId,
            "multi rebind remote first",
        )
        .await;

        let binding = localBinding.setNodeId(localNodeId.clone());
        assert_eq!(binding.nodeId, localNodeId);
        let _ = localRouter.bindingChanges.send(chatId.clone());
        let thirdStreamId = "chat-message-stream:multi-rebind-local-2";
        publishHolderChatMessageWithStream(
            &localHolder,
            &chatId,
            thirdStreamId,
            "multi rebind local second",
        )
        .await;
        let thirdStream = waitForRoutedChatMessageStream(&routedFlow, thirdStreamId).await;
        assertRoutedMarkdownStreamChunk(
            thirdStream,
            "embedded-open-multi-rebind-local-2",
            &chatId,
            "multi rebind local second",
        )
        .await;

        let binding = localBinding.setNodeId(targetNodeId.clone());
        assert_eq!(binding.nodeId, targetNodeId);
        let _ = localRouter.bindingChanges.send(chatId.clone());
        let fourthStreamId = "chat-message-stream:multi-rebind-remote-2";
        publishHolderChatMessageWithStream(
            &targetHolder,
            &chatId,
            fourthStreamId,
            "multi rebind remote second",
        )
        .await;
        let fourthStream = waitForRoutedChatMessageStream(&routedFlow, fourthStreamId).await;
        assertRoutedMarkdownStreamChunk(
            fourthStream,
            "embedded-open-multi-rebind-remote-2",
            &chatId,
            "multi rebind remote second",
        )
        .await;

        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_chat_state_flow_routes_between_real_core_node_routers() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-state-router-client".to_string();
        let targetNodeId = "core-node-state-router-source".to_string();
        let chatId = "chat-two-router-state-a".to_string();
        let joinedSpace = CoreSpace {
            spaceId: "space-two-router-state-test".to_string(),
            spaceName: "Route State Test Space".to_string(),
            spaceRevision: 2,
            members: vec![localNodeId.clone(), targetNodeId.clone()],
        };
        let (localRouter, localHolder) = testCoreNodeRouterInJoinedSpace(
            &localNodeId,
            &targetNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace.clone(),
        );
        let (targetRouter, targetHolder) = testCoreNodeRouterInJoinedSpace(
            &targetNodeId,
            &localNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace,
        );
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(localRouter.clone()),
            targetNodeId.clone(),
            TestCoreNodeRouterEndpoint::new(targetRouter),
        )
        .expect("in-memory PeerLink must connect both real CoreNodeRouters");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(localRouter));
        let routedFlow = {
            let mut holder = localHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .chatStateFlow(chatId.clone())
                .await
        };
        let selectedState = InputProcessingState::Receiving {
            message: "receiving_from_target_core".to_string(),
        };
        {
            let mut holder = targetHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .messageProcessingDelegate
                .setInputProcessingStateForChat(chatId.clone(), selectedState.clone());
        }
        let state = waitForRoutedChatState(&routedFlow, &selectedState).await;
        assert_eq!(state.currentChatId, chatId);
        assert!(!state.isLoading);
        peerHandle.close();
    }

    /// Verifies a routed chat StateFlow opens from the local Core while its selected owner has no live channel.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_chat_state_flow_opens_local_source_when_binding_owner_unreachable() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-state-local-source-client".to_string();
        let targetNodeId = "core-node-state-local-source-owner".to_string();
        let chatId = "chat-state-local-source-a".to_string();
        let joinedSpace = CoreSpace {
            spaceId: "space-state-local-source-test".to_string(),
            spaceName: "Route State Local Source Test Space".to_string(),
            spaceRevision: 2,
            members: vec![localNodeId.clone(), targetNodeId.clone()],
        };
        let (localRouter, _localHolder) = testCoreNodeRouterInJoinedSpace(
            &localNodeId,
            &targetNodeId,
            &chatId,
            &targetNodeId,
            joinedSpace,
        );
        let mut stream = tokio::time::timeout(
            Duration::from_millis(500),
            CoreLinkSharedClient::watch(
                &localRouter,
                CoreWatchRequest::new(
                    "core-route-chat-state-local-source",
                    CORE_INTERNAL_ROUTE_OBJECT_ID,
                    "chatStateFlow",
                    operit_link::toCoreValue(serde_json::json!({ "chatId": chatId.clone() }))
                        .expect("chat StateFlow route args must encode"),
                ),
            ),
        )
        .await
        .expect("routed chat StateFlow watch must open without a connected selected owner")
        .expect("routed chat StateFlow watch must return a stream");
        let event = receiveEvent(&mut stream).await;
        assert_eq!(event.kind, CoreEventKind::Snapshot);
        let state: ChatState =
            operit_link::fromCoreValue(event.value).expect("chat StateFlow event must decode");
        assert_eq!(state.currentChatId, chatId);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_annotated_chat_messages_flow_routes_and_reopens_embedded_stream() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-rust-wrapper-client".to_string();
        let targetNodeId = "core-node-rust-wrapper-source".to_string();
        let chatId = "chat-rust-wrapper-flow-a".to_string();
        let router = testCoreNodeRouter(&localNodeId, &targetNodeId, &chatId);
        let source = TestRealChatSpaceEndpoint::new();
        let peerHandle = connectInMemoryPeerLinks(
            localNodeId.clone(),
            Arc::new(TestClientEndpoint),
            targetNodeId.clone(),
            source.clone(),
        )
        .expect("in-memory PeerLink must connect wrapper caller and real Space endpoint");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(router));
        let mut clientHolder = ChatRuntimeHolder::new(Arc::new(TestFileSystemHost));
        let routedFlow = clientHolder
            .getCore(ChatRuntimeSlot::MAIN)
            .chatMessagesFlow(chatId.clone())
            .await;
        source
            .publishChatMessageWithStream(
                &chatId,
                "chat-message-stream:rust-wrapper-route-test",
                "hello through annotated Rust wrapper",
            )
            .await;
        let messages = waitForRoutedChatMessages(&routedFlow).await;
        let stream = messages
            .iter()
            .find_map(|message| message.contentStream.clone())
            .expect("annotated routed message must carry an embedded content stream");
        let localPool = Arc::new(CoreStreamPool::new());
        let (_encoded, attachments) =
            operit_link::withCoreStreamCaptureSync(|| operit_link::toCoreValue(stream.clone()));
        assert_eq!(attachments.len(), 1);
        localPool.adoptAll(attachments);
        let mut openedStream = localPool
            .openCoreStreamWatch(CoreWatchRequest::new(
                "embedded-open-rust-wrapper-router-test",
                CORE_STREAM_POOL_OBJECT_ID,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("annotated routed ChatMessage stream must reopen through CoreNodeRouter");
        let chunk = receiveEvent(&mut openedStream).await;
        assert_eq!(chunk.kind, CoreEventKind::Changed);
        let event: MarkdownStreamEvent = operit_link::fromCoreValue(chunk.value)
            .expect("annotated routed Markdown stream event must decode");
        assert_eq!(event.chatId, chatId);
        assert_eq!(event.eventType, "chunk");
        assert_eq!(
            event.value,
            Some("hello through annotated Rust wrapper".to_string())
        );
        let completed = receiveEvent(&mut openedStream).await;
        assert_eq!(completed.kind, CoreEventKind::Completed);
        assert!(source.embeddedStreamOpened.load(Ordering::SeqCst));
        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_annotated_chat_messages_flow_without_binding_opens_local_flow() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let router = testCoreNodeRouterWithoutBinding("core-node-local-without-binding");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(router));
        let mut clientHolder = ChatRuntimeHolder::new(Arc::new(TestFileSystemHost));
        let flow = clientHolder
            .getCore(ChatRuntimeSlot::MAIN)
            .chatMessagesFlow("chat-without-binding-record".to_string())
            .await;
        assert!(flow.value().is_empty());
    }
}
