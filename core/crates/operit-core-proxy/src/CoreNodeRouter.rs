use async_trait::async_trait;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::{HostRuntimeTaskSchedulerHost, RuntimeStorageHost};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient,
    CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CorePushItem, CorePushRequest,
    CoreValue, CoreWatchRequest,
};
use operit_link_access::CoreNodePeerLink::{
    activePeerNodeIds, peerLink, subscribePeerLinkChanges, CoreNodeBindingApplyRequest,
    CoreNodeBindingTransitionRequest, CoreNodeBindingTransitionResult, CoreNodeLinkClient,
    CoreNodeWatchSourceActivationRequest, PeerLinkClient, RoutedCoreRequest,
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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, Mutex};

use crate::{
    CoreRouteSourceTransition, GeneratedCoreRoute,
    CORE_ROUTE_CURSOR_ARGUMENT, CORE_ROUTE_CURSOR_PROPERTY, CORE_ROUTE_SOURCE_TRANSITION_PROPERTY,
};

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
    localCore: Arc<dyn CoreRouteLocalRuntime>,
    bindingStore: Arc<dyn CoreNodeBindingRuntime>,
    localNodeId: String,
    spaceStore: CoreSpaceStore,
    bindingChanges: broadcast::Sender<String>,
    bindingTransitionCoordinator: Arc<Mutex<()>>,
    _bindingChangeObserver: Arc<CoreNodeBindingChangeObserver>,
}

/// Exposes the Link operations and storage capability required by Core routing.
#[async_trait(?Send)]
pub trait CoreRouteLocalRuntime: Send + Sync {
    /// Installs the live device routing capability used by built-in tools.
    #[allow(non_snake_case)]
    fn bindCoreNodeToolRuntime(
        &self,
        runtime: Arc<dyn CoreNodeToolRuntime>,
    ) -> Result<(), CoreLinkError>;

    /// Returns the runtime storage capability owned by this Core.
    #[allow(non_snake_case)]
    fn runtimeStorageHost(&self) -> Arc<dyn RuntimeStorageHost>;

    /// Executes one local Core call.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse;

    /// Reads one local Core watch snapshot.
    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError>;

    /// Opens one local Core watch stream.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError>;

    /// Activates one generated local watch source from opaque route state.
    #[allow(non_snake_case)]
    async fn activateWatchSource(
        &self,
        request: CoreWatchRequest,
        generation: i64,
        payload: Vec<u8>,
    ) -> Result<(), CoreLinkError>;

    /// Opens one local Core push stream.
    #[allow(non_snake_case)]
    fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;
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
    pub fn new(localCore: Arc<dyn CoreRouteLocalRuntime>) -> Self {
        let bindingStore = Arc::new(
            CoreNodeBindingStore::new(localCore.runtimeStorageHost())
                .expect("CoreNodeRouter requires persistent Binding storage"),
        );
        Self::newWithRuntime(localCore, bindingStore)
    }

    /// Creates a router over one concrete local Core runtime implementation.
    #[allow(non_snake_case)]
    fn newWithRuntime(
        localCore: Arc<dyn CoreRouteLocalRuntime>,
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
        Self {
            localCore,
            bindingStore,
            localNodeId,
            spaceStore,
            bindingChanges,
            bindingTransitionCoordinator: Arc::new(Mutex::new(())),
            _bindingChangeObserver: bindingChangeObserver,
        }
    }

    /// Returns the stable identity of the CoreNode that owns this router.
    #[allow(non_snake_case)]
    pub fn localNodeId(&self) -> String {
        self.localNodeId.clone()
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
            GeneratedCoreRoute::CurrentBinding { .. } => Err(CoreLinkError::new(
                "CORE_BINDING_CURRENT_UNRESOLVED",
                "Current Binding must be resolved before selecting a CoreNode",
            )),
        }
    }

    /// Resolves one Binding and requires its selected device to be reachable.
    #[allow(non_snake_case)]
    fn bindingRouteNodeId(&self, key: &str) -> Result<String, CoreLinkError> {
        let targetNodeId = self.bindingStore.bindingNodeId(key)?;
        if !self.nodeIsReachable(&targetNodeId)? {
            return Err(CoreLinkError::new(
                "CORE_BINDING_TARGET_UNREACHABLE",
                format!("Binding {key} selects unreachable CoreNode {targetNodeId}"),
            ));
        }
        Ok(targetNodeId)
    }

    /// Reports whether the active Peer Link graph currently proves one device reachable.
    #[allow(non_snake_case)]
    fn nodeIsReachable(&self, targetNodeId: &str) -> Result<bool, CoreLinkError> {
        coreNodeIsReachable(&self.localNodeId, &self.spaceStore, targetNodeId)
            .map_err(CoreLinkError::internal)
    }

    /// Resolves an object-scoped current Binding through the local Core state.
    #[allow(non_snake_case)]
    async fn resolveGeneratedRoute(
        &self,
        route: GeneratedCoreRoute,
        targetPath: operit_link::CoreObjectPath,
    ) -> Result<(GeneratedCoreRoute, Option<(usize, String)>), CoreLinkError> {
        let GeneratedCoreRoute::CurrentBinding { scope, resolver } = route else {
            return Ok((route, None));
        };
        let event = self
            .localCore
            .watchSnapshot(CoreWatchRequest::new(
                nextCoreNodeRouterRequestId(resolver),
                targetPath,
                resolver,
                CoreValue::emptyMap(),
            ))
            .await?;
        let CoreValue::String(key) = event.value else {
            return Err(CoreLinkError::new(
                "CORE_BINDING_CURRENT_REQUIRED",
                "Binding resolver did not return a current key",
            ));
        };
        if key.trim().is_empty() {
            return Err(CoreLinkError::new(
                "CORE_BINDING_KEY_REQUIRED",
                "Binding resolver returned an empty key",
            ));
        }
        Ok((
            GeneratedCoreRoute::Binding {
                scope,
                key: key.clone(),
            },
            Some((scope, key)),
        ))
    }

    /// Opens the generic push session selected by generated CoreNode routing metadata.
    #[allow(non_snake_case)]
    async fn openPushSession(
        &self,
        mut request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let (route, currentBinding) = self
            .resolveGeneratedRoute(
                crate::generated_core_push_route(&request)?,
                request.targetPath.clone(),
            )
            .await?;
        if let Some((scope, key)) = currentBinding {
            crate::generated_inject_current_binding_push(scope, &mut request, key)?;
        }
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
            spaceId: space.spaceId,
            targetNodeId,
            ttl,
            payload,
        })
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
        let requestId = request.requestId.clone();
        let route = match self.initialRoute(targetNodeId, request) {
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

    /// Executes one generated call directly on this CoreNode after Binding validation.
    #[allow(non_snake_case)]
    async fn executeLocalCall(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        if let Err(error) = self.validateLocalCall(&request).await {
            return CoreCallResponse::err(requestId, error);
        }
        self.localCore.call(request).await
    }

    /// Verifies every locally executed Binding call against its current target.
    #[allow(non_snake_case)]
    async fn validateLocalCall(&self, request: &CoreCallRequest) -> Result<(), CoreLinkError> {
        let bindingKey = match crate::generated_core_call_route(request)? {
            GeneratedCoreRoute::Binding { key, .. } => Some(key),
            GeneratedCoreRoute::Local => None,
            GeneratedCoreRoute::CurrentBinding { .. } => {
                return Err(CoreLinkError::new(
                    "CORE_BINDING_CURRENT_UNRESOLVED",
                    "Current Binding call reached local execution before resolution",
                ))
            }
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

    /// Atomically changes one generated Binding and returns its committed operation.
    pub(crate) async fn compareAndSetBinding(
        &self,
        key: String,
        expectedNodeId: String,
        expectedGeneration: i64,
        targetNodeId: String,
    ) -> Result<CoreNodeBindingCommit, CoreLinkError> {
        self.bindingStore.compareAndSetBinding(
            &key,
            &expectedNodeId,
            expectedGeneration,
            &targetNodeId,
        )
    }

    /// Opens one Binding watch segment on an explicit CoreNode.
    #[allow(non_snake_case)]
    pub(crate) async fn watchBindingNode(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if targetNodeId == self.localNodeId {
            return self.localCore.watch(request).await;
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

    /// Opens one stable outer watch whose Binding key follows a local resolver flow.
    #[allow(non_snake_case)]
    async fn openCurrentBindingWatch(
        &self,
        scope: usize,
        resolver: &'static str,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let resolverRequest = CoreWatchRequest::new(
            nextCoreNodeRouterRequestId(resolver),
            request.targetPath.clone(),
            resolver,
            CoreValue::emptyMap(),
        );
        let initialEvent = self.localCore.watchSnapshot(resolverRequest.clone()).await?;
        let initialKey = currentBindingKeyFromEvent(initialEvent, resolver)?;
        let mut initialRequest = request.clone();
        crate::generated_inject_current_binding_watch(scope, &mut initialRequest, initialKey.clone())?;
        let source = self
            .openRecoveringBindingWatch(initialKey.clone(), initialRequest)
            .await?;
        let resolverStream = self.localCore.watch(resolverRequest).await?;
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let (cancelSender, cancelReceiver) = tokio::sync::oneshot::channel();
        let router = self.clone();
        HostRuntimeTaskSchedulerHost::scheduleHostRuntimeAsyncTask(
            defaultHostRuntimeTaskSchedulerHost().as_ref(),
            "core-node-current-binding-watch",
            Box::new(move || {
                Box::pin(async move {
                    runCurrentBindingWatch(
                        router,
                        scope,
                        resolver,
                        request,
                        initialKey,
                        source,
                        resolverStream,
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

    /// Commits one source transition and synchronizes its Binding operation to the target.
    #[allow(non_snake_case)]
    async fn transitionBindingSource(
        &self,
        bindingKey: String,
        sourceNodeId: String,
        sourceGeneration: i64,
        targetNodeId: String,
        watchRequest: CoreWatchRequest,
        payload: Vec<u8>,
    ) -> Result<CoreNodeBindingRecord, CoreLinkError> {
        let transition = self
            .requestBindingTransition(
                bindingKey.clone(),
                sourceNodeId,
                sourceGeneration,
                targetNodeId.clone(),
                watchRequest,
                payload,
            )
            .await?;
        self.applyBindingCommitAtNode(
            targetNodeId,
            CoreNodeBindingApplyRequest {
                bindingKey: bindingKey.clone(),
                nodeId: transition.nodeId.clone(),
                generation: transition.generation,
                operation: transition.operation.clone(),
            },
        )
        .await?;
        let observedBinding = self
            .bindingStore
            .applyImmediateBindingOperation(&transition.operation)?;
        if observedBinding.key != bindingKey
            || observedBinding.nodeId != transition.nodeId
            || observedBinding.generation != transition.generation
        {
            return Err(CoreLinkError::new(
                "CORE_BINDING_OBSERVATION_MISMATCH",
                format!(
                    "Binding observation expected key={} node={} generation={}, applied key={} node={} generation={}",
                    bindingKey,
                    transition.nodeId,
                    transition.generation,
                    observedBinding.key,
                    observedBinding.nodeId,
                    observedBinding.generation
                ),
            ));
        }
        Ok(CoreNodeBindingRecord {
            key: bindingKey,
            nodeId: transition.nodeId,
            generation: transition.generation,
        })
    }

    /// Routes one Binding generation transition to the Router that owns its source.
    #[allow(non_snake_case)]
    async fn requestBindingTransition(
        &self,
        bindingKey: String,
        sourceNodeId: String,
        sourceGeneration: i64,
        targetNodeId: String,
        watchRequest: CoreWatchRequest,
        payload: Vec<u8>,
    ) -> Result<CoreNodeBindingTransitionResult, CoreLinkError> {
        let request = CoreNodeBindingTransitionRequest {
            bindingKey,
            sourceGeneration,
            targetNodeId,
            watchRequest,
            payload,
        };
        if sourceNodeId == self.localNodeId {
            return self.executeBindingTransition(request).await;
        }
        let route = self.initialRoute(sourceNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedBindingTransition(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Commits one exact Binding transition on its source Router.
    #[allow(non_snake_case)]
    async fn executeBindingTransition(
        &self,
        request: CoreNodeBindingTransitionRequest,
    ) -> Result<CoreNodeBindingTransitionResult, CoreLinkError> {
        let _coordinator = self.bindingTransitionCoordinator.lock().await;
        validateCoreNodeSwitchTarget(&self.localNodeId, &self.spaceStore, &request.targetNodeId)
            .map_err(CoreLinkError::internal)?;
        match crate::generated_core_watch_route(&request.watchRequest)? {
            GeneratedCoreRoute::Binding { key, .. } if key == request.bindingKey => {}
            GeneratedCoreRoute::Binding { key, .. } => {
                return Err(CoreLinkError::new(
                    "STREAM_SOURCE_BINDING_MISMATCH",
                    format!(
                        "Watch Binding key {} does not match transition Binding {}",
                        key, request.bindingKey
                    ),
                ))
            }
            _ => {
                return Err(CoreLinkError::new(
                    "STREAM_SOURCE_BINDING_REQUIRED",
                    "Generated watch source activation requires Binding routing",
                ))
            }
        }
        let targetGeneration = request.sourceGeneration.checked_add(1).ok_or_else(|| {
            CoreLinkError::new(
                "CORE_BINDING_GENERATION_OVERFLOW",
                format!("Binding generation overflow for {}", request.bindingKey),
            )
        })?;
        let currentBinding = self.bindingStore.binding(&request.bindingKey)?;
        if currentBinding.nodeId == self.localNodeId
            && currentBinding.generation == request.sourceGeneration
        {
            let activationStartedAt = Instant::now();
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_transition_activation_start key={} source={} sourceGeneration={} target={} targetGeneration={} requestId={}",
                    request.bindingKey,
                    self.localNodeId,
                    request.sourceGeneration,
                    request.targetNodeId,
                    targetGeneration,
                    request.watchRequest.requestId.0
                ),
            );
            self.activateWatchSourceAtNode(
                request.targetNodeId.clone(),
                CoreNodeWatchSourceActivationRequest {
                    bindingKey: request.bindingKey.clone(),
                    sourceNodeId: self.localNodeId.clone(),
                    sourceGeneration: request.sourceGeneration,
                    request: request.watchRequest,
                    generation: targetGeneration,
                    payload: request.payload,
                },
            )
            .await?;
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_transition_activation_ready key={} source={} sourceGeneration={} target={} targetGeneration={} elapsedMs={}",
                    request.bindingKey,
                    self.localNodeId,
                    request.sourceGeneration,
                    request.targetNodeId,
                    targetGeneration,
                    activationStartedAt.elapsed().as_millis()
                ),
            );
        } else if currentBinding.nodeId != request.targetNodeId
            || currentBinding.generation != targetGeneration
        {
            return Err(CoreLinkError::new(
                "CORE_BINDING_TRANSITION_FAILED",
                format!(
                    "Binding transition conflict for {}: expected {}@{} or {}@{}, actual {}@{}",
                    request.bindingKey,
                    self.localNodeId,
                    request.sourceGeneration,
                    request.targetNodeId,
                    targetGeneration,
                    currentBinding.nodeId,
                    currentBinding.generation
                ),
            ));
        } else {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_transition_join key={} source={} sourceGeneration={} target={} targetGeneration={}",
                    request.bindingKey,
                    self.localNodeId,
                    request.sourceGeneration,
                    request.targetNodeId,
                    targetGeneration
                ),
            );
        }
        let commit = self
            .compareAndSetBinding(
                request.bindingKey,
                self.localNodeId.clone(),
                request.sourceGeneration,
                request.targetNodeId,
            )
            .await?;
        operit_util::AppLogger::AppLogger::i(
            "CoreNodeRouteTrace",
            &format!(
                "binding_transition_committed key={} source={} sourceGeneration={} target={} targetGeneration={}",
                commit.binding.key,
                self.localNodeId,
                request.sourceGeneration,
                commit.binding.nodeId,
                commit.binding.generation
            ),
        );
        Ok(CoreNodeBindingTransitionResult {
            nodeId: commit.binding.nodeId,
            generation: commit.binding.generation,
            operation: commit.operation,
        })
    }

    /// Routes one opaque generated watch-source activation to its selected CoreNode.
    #[allow(non_snake_case)]
    async fn activateWatchSourceAtNode(
        &self,
        targetNodeId: String,
        request: CoreNodeWatchSourceActivationRequest,
    ) -> Result<(), CoreLinkError> {
        if targetNodeId == self.localNodeId {
            return self.executeWatchSourceActivation(request).await;
        }
        let route = self.initialRoute(targetNodeId, request)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatchSourceActivate(forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Validates source ownership and activates one opaque generated watch source locally.
    #[allow(non_snake_case)]
    async fn executeWatchSourceActivation(
        &self,
        request: CoreNodeWatchSourceActivationRequest,
    ) -> Result<(), CoreLinkError> {
        let binding = self.bindingStore.binding(&request.bindingKey)?;
        if binding.nodeId != request.sourceNodeId || binding.generation != request.sourceGeneration
        {
            return Err(CoreLinkError::new(
                "STREAM_SOURCE_BINDING_CONFLICT",
                format!(
                    "Watch source activation for {} expected {}@{}, actual {}@{}",
                    request.bindingKey,
                    request.sourceNodeId,
                    request.sourceGeneration,
                    binding.nodeId,
                    binding.generation
                ),
            ));
        }
        self.localCore
            .activateWatchSource(request.request, request.generation, request.payload)
            .await
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

    /// Validates one exact target before the response stream publishes its source transition.
    #[allow(non_snake_case)]
    fn requestCoreSwitch(&self, bindingKey: &str, targetNodeId: &str) -> Result<(), String> {
        let binding = self
            .bindingStore
            .binding(bindingKey)
            .map_err(|error| error.to_string())?;
        if binding.nodeId != self.localNodeId {
            return Err(format!(
                "Binding {bindingKey} is owned by {}, not {}",
                binding.nodeId, self.localNodeId
            ));
        }
        validateCoreNodeSwitchTarget(&self.localNodeId, &self.spaceStore, targetNodeId)
    }
}

/// Validates that an opaque stream transition targets a distinct reachable Space member.
#[allow(non_snake_case)]
fn validateCoreNodeSwitchTarget(
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    targetNodeId: &str,
) -> Result<(), String> {
    if targetNodeId.trim().is_empty() {
        return Err("CoreNode stream transition target must not be empty".to_string());
    }
    if targetNodeId == localNodeId {
        return Err(
            "CoreNode stream transition target must differ from the current owner".to_string(),
        );
    }
    if !spaceStore.contains(targetNodeId.to_string())? {
        return Err(format!(
            "CoreNode stream transition target is not a Space member: {targetNodeId}"
        ));
    }
    if !coreNodeIsReachable(localNodeId, spaceStore, targetNodeId)? {
        return Err(format!("CoreNode is not reachable: {targetNodeId}"));
    }
    Ok(())
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

/// Extracts the required Binding key from one local resolver event.
#[allow(non_snake_case)]
fn currentBindingKeyFromEvent(
    event: CoreEvent,
    resolver: &str,
) -> Result<String, CoreLinkError> {
    let CoreValue::String(key) = event.value else {
        return Err(CoreLinkError::new(
            "CORE_BINDING_CURRENT_REQUIRED",
            format!("Binding resolver {resolver} did not return a key"),
        ));
    };
    if key.trim().is_empty() {
        return Err(CoreLinkError::new(
            "CORE_BINDING_KEY_REQUIRED",
            format!("Binding resolver {resolver} returned an empty key"),
        ));
    }
    Ok(key)
}

/// Emits the locally observable snapshot for one current Binding watch key.
#[allow(non_snake_case)]
async fn emitCurrentBindingLocalSnapshot(
    router: &CoreNodeRouter,
    request: &CoreWatchRequest,
    sender: &tokio::sync::mpsc::UnboundedSender<CoreEvent>,
) -> Result<(), CoreLinkError> {
    let event = router.localCore.watchSnapshot(request.clone()).await?;
    sender
        .send(event)
        .map_err(|_| CoreLinkError::new("WATCH_CLOSED", "current Binding watch is closed"))?;
    Ok(())
}

/// Pumps one stable outer watch while its Binding key follows a local resolver stream.
#[allow(non_snake_case)]
async fn runCurrentBindingWatch(
    router: CoreNodeRouter,
    scope: usize,
    resolver: &'static str,
    request: CoreWatchRequest,
    mut bindingKey: String,
    mut source: CoreEventStream,
    mut resolverStream: CoreEventStream,
    sender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
    mut cancelReceiver: tokio::sync::oneshot::Receiver<()>,
) {
    let mut currentRequest = request.clone();
    if let Err(error) =
        crate::generated_inject_current_binding_watch(scope, &mut currentRequest, bindingKey.clone())
    {
        operit_util::AppLogger::AppLogger::e(
            "CoreNodeRouteTrace",
            &format!(
                "current_binding_watch_initial_injection_failed requestId={} resolver={} key={} code={} error={}",
                request.requestId.0, resolver, bindingKey, error.code, error
            ),
        );
        return;
    }
    if let Err(error) = emitCurrentBindingLocalSnapshot(&router, &currentRequest, &sender).await {
        operit_util::AppLogger::AppLogger::e(
            "CoreNodeRouteTrace",
            &format!(
                "current_binding_watch_initial_snapshot_failed requestId={} resolver={} key={} code={} error={}",
                request.requestId.0, resolver, bindingKey, error.code, error
            ),
        );
        return;
    }
    loop {
        tokio::select! {
            biased;
            _ = &mut cancelReceiver => return,
            resolverEvent = resolverStream.recv() => {
                let Some(resolverEvent) = resolverEvent else {
                    return;
                };
                if resolverEvent.kind == CoreEventKind::Completed {
                    return;
                }
                let nextKey = match currentBindingKeyFromEvent(resolverEvent, resolver) {
                    Ok(key) => key,
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::e(
                            "CoreNodeRouteTrace",
                            &format!(
                                "current_binding_watch_resolver_failed requestId={} resolver={} code={} error={}",
                                request.requestId.0, resolver, error.code, error
                            ),
                        );
                        return;
                    }
                };
                if nextKey == bindingKey {
                    continue;
                }
                let mut nextRequest = request.clone();
                if let Err(error) = crate::generated_inject_current_binding_watch(
                    scope,
                    &mut nextRequest,
                    nextKey.clone(),
                ) {
                    operit_util::AppLogger::AppLogger::e(
                        "CoreNodeRouteTrace",
                        &format!(
                            "current_binding_watch_injection_failed requestId={} resolver={} key={} code={} error={}",
                            request.requestId.0, resolver, nextKey, error.code, error
                        ),
                    );
                    return;
                }
                match router.openRecoveringBindingWatch(nextKey.clone(), nextRequest).await {
                    Ok(nextSource) => {
                        bindingKey = nextKey;
                        source = nextSource;
                        currentRequest = request.clone();
                        if let Err(error) = crate::generated_inject_current_binding_watch(
                            scope,
                            &mut currentRequest,
                            bindingKey.clone(),
                        ) {
                            operit_util::AppLogger::AppLogger::e(
                                "CoreNodeRouteTrace",
                                &format!(
                                    "current_binding_watch_snapshot_injection_failed requestId={} resolver={} key={} code={} error={}",
                                    request.requestId.0, resolver, bindingKey, error.code, error
                                ),
                            );
                            return;
                        }
                        if let Err(error) =
                            emitCurrentBindingLocalSnapshot(&router, &currentRequest, &sender).await
                        {
                            operit_util::AppLogger::AppLogger::e(
                                "CoreNodeRouteTrace",
                                &format!(
                                    "current_binding_watch_snapshot_failed requestId={} resolver={} key={} code={} error={}",
                                    request.requestId.0, resolver, bindingKey, error.code, error
                                ),
                            );
                            return;
                        }
                    }
                    Err(error) => {
                        operit_util::AppLogger::AppLogger::e(
                            "CoreNodeRouteTrace",
                            &format!(
                                "current_binding_watch_switch_failed requestId={} resolver={} key={} code={} error={}",
                                request.requestId.0, resolver, nextKey, error.code, error
                            ),
                        );
                        return;
                    }
                }
            }
            event = source.recv() => {
                let Some(event) = event else {
                    return;
                };
                if sender.send(event).is_err() {
                    return;
                }
            }
        }
    }
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
        let nextRequest = watchRequestWithRouteState(request, cursor.clone())?;
        match router
            .watchBindingNode(binding.nodeId.clone(), nextRequest)
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
                if event.propertyName == CORE_ROUTE_SOURCE_TRANSITION_PROPERTY {
                    let transition: CoreRouteSourceTransition = match operit_link::fromCoreValue(event.value) {
                        Ok(transition) => transition,
                        Err(error) => {
                            operit_util::AppLogger::AppLogger::e(
                                "CoreNodeRouteTrace",
                                &format!(
                                    "binding_watch_source_transition_decode_failed requestId={} key={} source={} generation={} error={}",
                                    request.requestId.0,
                                    key,
                                    sourceNodeId,
                                    sourceGeneration,
                                    error
                                ),
                            );
                            return;
                        }
                    };
                    let transitionTargetNodeId = transition.targetNodeId.clone();
                    let transitionResult = router
                        .transitionBindingSource(
                            key.clone(),
                            sourceNodeId.clone(),
                            sourceGeneration,
                            transitionTargetNodeId.clone(),
                            request.clone(),
                            transition.payload,
                        )
                        .await;
                    if let Err(error) = transitionResult {
                        let currentBinding = router.bindingStore.binding(&key);
                        if let Ok(currentBinding) = currentBinding {
                            if currentBinding.nodeId != sourceNodeId
                                || currentBinding.generation != sourceGeneration
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
                                    Err(reopenError) => {
                                        operit_util::AppLogger::AppLogger::e(
                                            "CoreNodeRouteTrace",
                                            &format!(
                                                "binding_watch_source_transition_reopen_failed requestId={} key={} code={} error={}",
                                                request.requestId.0,
                                                key,
                                                reopenError.code,
                                                reopenError
                                            ),
                                        );
                                        return;
                                    }
                                }
                            }
                        }
                        operit_util::AppLogger::AppLogger::e(
                            "CoreNodeRouteTrace",
                            &format!(
                                "binding_watch_source_transition_failed requestId={} key={} source={} generation={} target={} code={} error={}",
                                request.requestId.0,
                                key,
                                sourceNodeId,
                                sourceGeneration,
                                transitionTargetNodeId,
                                error.code,
                                error
                            ),
                        );
                        return;
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
                                    "binding_watch_next_source_failed requestId={} key={} code={} error={}",
                                    request.requestId.0, key, error.code, error
                                ),
                            );
                            return;
                        }
                    }
                    continue;
                }
                if event.kind == CoreEventKind::Completed {
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

    /// Executes or forwards one routed call.
    #[allow(non_snake_case)]
    async fn routedCall(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let requestId = request.payload.requestId.clone();
        match self.validateIncomingRoute(&previousNodeId, &request) {
            Ok(true) => self.executeLocalCall(request.payload).await,
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
            return self.localCore.watchSnapshot(request.payload).await;
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
            return self.localCore.watch(request.payload).await;
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

    /// Commits or forwards one source-owned Binding generation transition.
    #[allow(non_snake_case)]
    async fn routedBindingTransition(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreNodeBindingTransitionRequest>,
    ) -> Result<CoreNodeBindingTransitionResult, CoreLinkError> {
        if self.validateIncomingRoute(&previousNodeId, &request)? {
            return self.executeBindingTransition(request.payload).await;
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedBindingTransition(forwardedRequest).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Activates or forwards one generated watch source before Binding publication.
    #[allow(non_snake_case)]
    async fn routedWatchSourceActivate(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreNodeWatchSourceActivationRequest>,
    ) -> Result<(), CoreLinkError> {
        if self.validateIncomingRoute(&previousNodeId, &request)? {
            return self.executeWatchSourceActivation(request.payload).await;
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            let peerNodeId = peer.peerNodeId();
            match peer.routedWatchSourceActivate(forwardedRequest).await {
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
            return self.localCore.openPush(request.payload);
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
        let (route, currentBinding) = match self
            .resolveGeneratedRoute(generatedRoute, request.targetPath.clone())
            .await
        {
            Ok(route) => route,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        if let Some((scope, key)) = currentBinding {
            if let Err(error) =
                crate::generated_inject_current_binding_call(scope, &mut request, key)
            {
                return CoreCallResponse::err(requestId, error);
            }
        }
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_call_resolve requestId={} path={} method={} key={} local={}",
                    request.requestId.0,
                    request.targetPath.key(),
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
                    request.targetPath.key(),
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
        let (route, currentBinding) = self
            .resolveGeneratedRoute(
                crate::generated_core_watch_route(&request)?,
                request.targetPath.clone(),
            )
            .await?;
        if let Some((scope, key)) = currentBinding {
            crate::generated_inject_current_binding_watch(scope, &mut request, key)?;
        }
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_snapshot_resolve requestId={} path={} property={} key={} local={}",
                    request.requestId.0,
                    request.targetPath.key(),
                    request.propertyName,
                    key,
                    self.localNodeId
                ),
            );
        }
        let targetNodeId = self.routeNodeId(route).await?;
        if targetNodeId == self.localNodeId {
            return self.localCore.watchSnapshot(request).await;
        }
        self.watchNodeSnapshot(targetNodeId, request).await
    }

    /// Opens a watch stream on the CoreNode selected by generated metadata.
    async fn watch(&self, mut request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let generatedRoute = crate::generated_core_watch_route(&request)?;
        if let GeneratedCoreRoute::CurrentBinding { scope, resolver } = &generatedRoute {
            return self
                .openCurrentBindingWatch(*scope, *resolver, request)
                .await;
        }
        let (route, currentBinding) = self
            .resolveGeneratedRoute(generatedRoute, request.targetPath.clone())
            .await?;
        if let Some((scope, key)) = currentBinding {
            crate::generated_inject_current_binding_watch(scope, &mut request, key)?;
        }
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_watch_resolve requestId={} path={} property={} key={} local={}",
                    request.requestId.0,
                    request.targetPath.key(),
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
                    request.targetPath.key(),
                    request.propertyName,
                    key,
                    self.localNodeId
                ),
            );
            return self.openRecoveringBindingWatch(key.clone(), request).await;
        }
        let targetNodeId = self.routeNodeId(route).await?;
        if targetNodeId == self.localNodeId {
            return self.localCore.watch(request).await;
        }
        self.watchNode(targetNodeId, request).await
    }
}
