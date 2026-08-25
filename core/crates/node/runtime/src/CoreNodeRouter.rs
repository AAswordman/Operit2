use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostRuntimeTaskSchedulerHost, RuntimeStorageHost};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreHandoffRequest, CoreHandoffResponse, CoreHandoffSegment, CoreLinkError,
    CoreLinkPushSession, CoreLinkSharedClient, CorePushItem, CorePushRequest, CoreValue,
    CoreWatchRequest,
};
use operit_link::route_runtime::CoreRouteRuntime;
use operit_access_runtime::CoreNodePeerLink::{
    activePeerNodeIds, peerLink, subscribePeerLinkChanges, CoreNodeBindingApplyRequest,
    CoreNodeLinkClient, PeerLinkClient, RoutedCoreRequest,
    RoutedCoreRequestKind,
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

use crate::{GeneratedCoreRoute, CORE_ROUTE_CURSOR_ARGUMENT, CORE_ROUTE_CURSOR_PROPERTY};
use crate::SpaceRuntime::SpaceRuntime;

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
            ) -> Pin<Box<dyn Future<Output = Result<CoreHandoffResponse, CoreLinkError>> + Send>>
            + Send
            + Sync,
    >,
    openPush: Arc<dyn Fn(CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> + Send + Sync>,
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
                ) -> Pin<Box<dyn Future<Output = Result<CoreHandoffResponse, CoreLinkError>> + Send>>
                + Send
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
    async fn watchSpaceSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        self.spaceRuntime.watchSnapshot(request).await
    }

    /// Opens one Space watch in the server-owned Space route namespace.
    async fn watchSpace(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        self.spaceRuntime.watch(request).await
    }

    /// Opens one Space push in the server-owned Space route namespace.
    fn openSpacePush(&self, request: CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
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
        CoreNodeBindingStore::binding(self, key)
            .map_err(|error| CoreLinkError::new("CORE_BINDING_READ_FAILED", error))
    }

    /// Returns the persisted CoreNode for one Binding key.
    #[allow(non_snake_case)]
    fn bindingNodeId(&self, key: &str) -> Result<String, CoreLinkError> {
        self.binding(key)
            .map(|binding| binding.nodeId)
            .map_err(|error| CoreLinkError::new("CORE_BINDING_READ_FAILED", error.to_string()))
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
                    CoreLinkError::new("SPACE_ROUTE_NOT_FOUND", "Space call route is not registered"),
                )
            }
        };
        let bindingKey = match route.bindingKey(&request.args) {
            Ok(value) => value,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        let targetNodeId = match self
            .routeNodeId(GeneratedCoreRoute::Binding { scope: 0, key: bindingKey })
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
        let route = crate::generated_space_watch_route(&request)
            .ok_or_else(|| CoreLinkError::new("SPACE_ROUTE_NOT_FOUND", "Space watch route is not registered"))?;
        let bindingKey = route.bindingKey(&request.args)?;
        let targetNodeId = self
            .routeNodeId(GeneratedCoreRoute::Binding { scope: 0, key: bindingKey })
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
        let route = crate::generated_space_watch_route(&request)
            .ok_or_else(|| CoreLinkError::new("SPACE_ROUTE_NOT_FOUND", "Space watch route is not registered"))?;
        let bindingKey = route.bindingKey(&request.args)?;
        let targetNodeId = self
            .routeNodeId(GeneratedCoreRoute::Binding { scope: 0, key: bindingKey })
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
            let (peer, forwardedRoute) = self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatchSnapshot(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => { excludedPeerNodeIds.insert(peerNodeId); }
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
            let (peer, forwardedRoute) = self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatch(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => { excludedPeerNodeIds.insert(peerNodeId); }
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
            let (peer, forwardedRoute) = self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedOpenPush(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => { excludedPeerNodeIds.insert(peerNodeId); }
                result => return result,
            }
        }
    }

    /// Opens one annotation-addressed Space push through Binding routing.
    pub async fn openSpacePush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let route = crate::generated_space_push_route(&request)
            .ok_or_else(|| CoreLinkError::new("SPACE_ROUTE_NOT_FOUND", "Space push route is not registered"))?;
        let bindingKey = route.bindingKey(&request.args)?;
        let targetNodeId = self
            .routeNodeId(GeneratedCoreRoute::Binding { scope: 0, key: bindingKey })
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
                CoreLinkError::new("CORE_REQUEST_ID_MISMATCH", "Local Core response request id mismatch"),
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

/// Executes one route-owned continuation transition and opens its remote stream segment.
async fn performCoreHandoff(
    bindingStore: &Arc<dyn CoreNodeBindingRuntime>,
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    request: CoreHandoffRequest,
) -> Result<CoreHandoffSegment, CoreLinkError> {
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
            format!("Core handoff target is not a Space member: {}", request.targetNodeId),
        ));
    }
    if !coreNodeIsReachable(localNodeId, spaceStore, &request.targetNodeId)
        .map_err(CoreLinkError::internal)?
    {
        return Err(CoreLinkError::new(
            "CORE_HANDOFF_TARGET_UNREACHABLE",
            format!("Core handoff target is not reachable: {}", request.targetNodeId),
        ));
    }
    let space = spaceStore
        .initialize()
        .map_err(CoreLinkError::internal)?;
    let ttl = routeTtl(&space)?;
    let targetNodeId = request.targetNodeId.clone();
    let routedRequest = RoutedCoreRequest {
        spaceId: space.spaceId.clone(),
        targetNodeId: request.targetNodeId.clone(),
        ttl,
        routeKind: RoutedCoreRequestKind::ObjectId,
        payload: request.clone(),
    };
    let response = routeHandoffNode(localNodeId, spaceStore, routedRequest).await?;
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
    let watchRequest = CoreWatchRequest::new(
        format!("core-handoff-watch-{}", response.stream.streamId),
        response.stream.targetObjectId,
        response.stream.propertyName.clone(),
        response.stream.args,
    );
    let stream = routeWatchNode(localNodeId, spaceStore, targetNodeId, watchRequest).await?;
    Ok(CoreHandoffSegment { stream })
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
            .ok_or_else(|| CoreLinkError::new("CORE_NODE_UNREACHABLE", "Core handoff route is unavailable"))?;
        let peer = peerLink(localNodeId, &nextNodeId)
            .map_err(|error| CoreLinkError::new("PEER_LINK_CLOSED", error))?;
        request.ttl = request.ttl.checked_sub(1).ok_or_else(|| {
            CoreLinkError::new("CORE_NODE_ROUTE_TTL_EXHAUSTED", "Core handoff route TTL is exhausted")
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
    let space = spaceStore
        .initialize()
        .map_err(CoreLinkError::internal)?;
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
            .ok_or_else(|| CoreLinkError::new("CORE_NODE_UNREACHABLE", "Binding commit route is unavailable"))?;
        let peer = peerLink(localNodeId, &nextNodeId)
            .map_err(|error| CoreLinkError::new("PEER_LINK_CLOSED", error))?;
        routedRequest.ttl = routedRequest.ttl.checked_sub(1).ok_or_else(|| {
            CoreLinkError::new("CORE_NODE_ROUTE_TTL_EXHAUSTED", "Binding commit route TTL is exhausted")
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
    let space = spaceStore
        .initialize()
        .map_err(CoreLinkError::internal)?;
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
            .ok_or_else(|| CoreLinkError::new("CORE_NODE_UNREACHABLE", "Core handoff stream route is unavailable"))?;
        let peer = peerLink(localNodeId, &nextNodeId)
            .map_err(|error| CoreLinkError::new("PEER_LINK_CLOSED", error))?;
        routedRequest.ttl = routedRequest.ttl.checked_sub(1).ok_or_else(|| {
            CoreLinkError::new("CORE_NODE_ROUTE_TTL_EXHAUSTED", "Core handoff stream route TTL is exhausted")
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
    ) -> operit_tools::runtime_support::ToolRuntimeSupportFuture<'a, Result<CoreHandoffSegment, String>> {
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
        let targetNodeId = if router.nodeIsReachable(&binding.nodeId)? {
            binding.nodeId.clone()
        } else {
            operit_util::AppLogger::AppLogger::w(
                "CoreNodeRouter",
                &format!(
                    "binding watch target unreachable; executing locally key={} target={} local={}",
                    key, binding.nodeId, router.localNodeId
                ),
            );
            router.localNodeId.clone()
        };
        let nextRequest = watchRequestWithRouteState(request, cursor.clone())?;
        match router
            .watchBindingNode(targetNodeId, nextRequest)
            .await
        {
            Ok(stream) => return Ok(Some((binding, stream))),
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
        }
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
        "CORE_NODE_UNREACHABLE" | "PEER_LINK_CLOSED" | "PEER_RESPONSE_CLOSED" | "PEER_SEND_FAILED"
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
                let targetNodeId = self.bindingStore.bindingNodeId(&key)?;
                Ok(targetNodeId != self.localNodeId && self.nodeIsReachable(&targetNodeId)?)
            }
        }
    }

    /// Routes one annotation wrapper call through Binding-selected Space routing.
    fn call(
        &self,
        request: CoreCallRequest,
    ) -> Pin<Box<dyn Future<Output = CoreCallResponse>>> {
        let router = self.clone();
        Box::pin(async move { router.callSpace(request).await })
    }

    /// Routes one annotation StateFlow watch through Binding-selected Space routing.
    fn watch(
        &self,
        request: CoreWatchRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CoreEventStream, CoreLinkError>>>> {
        let router = self.clone();
        Box::pin(async move { router.watchSpace(request).await })
    }
}
