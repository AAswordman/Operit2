use crate::PeerSync::{PeerSyncMethod, NODE_SYNC_TARGET};
use async_trait::async_trait;
use operit_link::{RoutedCoreRequest, RoutedCoreRequestKind};
use crate::RuntimePeerService::RuntimePeerService;
use crate::NodeServices::NodeServices;
use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;
use operit_host_api::RuntimeStorageHost;
use operit_link::route_runtime::CoreRouteRuntime;
use operit_link::{CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream, CoreLinkClient, CoreLinkError, CoreLinkPushSession, CoreLinkSharedClient, CorePushItem, CorePushRequest, CoreValue, CoreWatchRequest, CORE_INTERNAL_TARGET, CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT, CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT, CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT, CORE_STREAM_TARGET};
use operit_store::CoreNodeBindingStore::{CoreNodeBindingRecord, CoreNodeBindingStore};
use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
use operit_store::CoreSpaceStore::{CoreSpace, CoreSpaceRoutePlan, CoreSpaceStore};
use operit_store::NetworkControlStore::NetworkControlStore;
use operit_store::PreferencesDataStore::StateFlow;
use serde::{Deserialize, Serialize};
use operit_tools::runtime_support::{CoreNodeToolRuntime, RuntimeCoreNodeRouteState, RuntimeCoreNodeStatus};
use operit_util::AppLogger::{AppLogger, VERBOSE_LEVEL_5, VERBOSE_LEVEL_6};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use crate::GeneratedCoreRoute;
use crate::RuntimeRemoteLinkService::{
    RuntimeRemoteLinkService, NODE_SPACE_APPROVAL_TARGET, NODE_SPACE_TARGET,
};

#[path = "peer/sync_dispatch.rs"]
mod sync_dispatch;

// An event sender may not impersonate another Edge or use Target to bypass the
// Binding permission path. Called only after normal incoming-route validation.
fn validateEdgeEventOrigin(
    request: &CoreCallRequest,
    origin: &str,
    kind: RoutedCoreRequestKind,
) -> Result<(), CoreLinkError> {
    if request.target != CORE_INTERNAL_TARGET || request.methodName != "chatEdgeEvent" {
        return Ok(());
    }
    let node = match &request.args {
        CoreValue::Map(args) => args.get("nodeId"),
        _ => None,
    };
    if !matches!(
        kind,
        RoutedCoreRequestKind::SpaceBinding | RoutedCoreRequestKind::SpaceRoute
    ) || !matches!(node, Some(CoreValue::String(id)) if id == origin)
    {
        return Err(CoreLinkError::new(
            "EDGE_EVENT_ORIGIN_DENIED",
            "Edge event requires its authenticated Space origin",
        ));
    }
    Ok(())
}

/// Reports both persisted ownership and the device currently selected by routing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingRouteStatus {
    pub ownerNodeId: String,
    pub ownerPlatform: String,
    pub ownerIsLocal: bool,
    pub ownerReachable: bool,
    pub activeNodeId: String,
    pub activeIsLocal: bool,
    pub generation: i64,
}

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
    peerServices: Arc<std::sync::OnceLock<NodeServices>>,
    localCore: Arc<CoreNodeLocalRuntime>,
    bindingStore: Arc<dyn CoreNodeBindingRuntime>,
    localNodeId: String,
    spaceStore: CoreSpaceStore,
    networkControlStore: NetworkControlStore,
}

/// Carries local Core capabilities into the server-side Space router.
#[derive(Clone)]
pub struct CoreNodeLocalRuntime {
    peerServices: Arc<std::sync::OnceLock<NodeServices>>,
    sharedClient: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    applicationClient: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
    targetForSchema: Arc<dyn Fn(&str) -> Option<&'static str> + Send + Sync>,
    bindCoreNodeToolRuntime:
        Arc<dyn Fn(Arc<dyn CoreNodeToolRuntime>) -> Result<(), CoreLinkError> + Send + Sync>,
    openPush: Arc<
        dyn Fn(CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>
            + Send
            + Sync,
    >,
    spaceRuntime: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    edgeAudioIngress: Option<Arc<dyn Fn(CorePushRequest, String) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> + Send + Sync>>,
}

impl CoreNodeLocalRuntime {
    /// Creates the server-side capability container for one local Core.
    pub fn new(
        sharedClient: Arc<dyn CoreLinkSharedClient + Send + Sync>,
        applicationClient: Arc<dyn CoreLinkSharedClient + Send + Sync>,
        runtimeStorageHost: Arc<dyn RuntimeStorageHost>,
        targetForSchema: Arc<dyn Fn(&str) -> Option<&'static str> + Send + Sync>,
        bindCoreNodeToolRuntime: Arc<
            dyn Fn(Arc<dyn CoreNodeToolRuntime>) -> Result<(), CoreLinkError> + Send + Sync,
        >,
        openPush: Arc<
            dyn Fn(CorePushRequest) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>
                + Send
                + Sync,
        >,
        spaceRuntime: Arc<dyn CoreLinkSharedClient + Send + Sync>,
    ) -> Self {
        Self {
            peerServices: Arc::new(std::sync::OnceLock::new()),
            sharedClient,
            applicationClient,
            runtimeStorageHost,
            targetForSchema,
            bindCoreNodeToolRuntime,
            openPush,
            spaceRuntime,
            edgeAudioIngress: None,
        }
    }

    /// 生成 Proxy 和应用 Router 共享已注入的通信实例；不新建会话仓库。
    pub fn withPeerServices(mut self, services: Arc<std::sync::OnceLock<NodeServices>>) -> Self {
        self.peerServices = services;
        self
    }

    /// Attaches this runtime's audio receivers. Only authenticated routed Space
    /// Push may enter here; ordinary local/Target dispatch cannot bypass origin validation.
    pub fn withEdgeAudioIngress(mut self, ingress: Arc<dyn Fn(CorePushRequest, String) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> + Send + Sync>) -> Self {
        self.edgeAudioIngress = Some(ingress);
        self
    }

    /// Returns the storage host owned by the local Core.
    #[allow(non_snake_case)]
    pub fn runtimeStorageHost(&self) -> Arc<dyn RuntimeStorageHost> {
        self.runtimeStorageHost.clone()
    }

    /// Resolves one generated Core schema to the local numeric object ID.
    #[allow(non_snake_case)]
    pub fn targetForSchema(&self, schema: &str) -> Option<&'static str> {
        (self.targetForSchema)(schema)
    }

    /// Installs the routing capability used by built-in tools.
    #[allow(non_snake_case)]
    fn bindCoreNodeToolRuntime(
        &self,
        runtime: Arc<dyn CoreNodeToolRuntime>,
    ) -> Result<(), CoreLinkError> {
        (self.bindCoreNodeToolRuntime)(runtime)
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
    pub(crate) async fn callSpace(&self, request: CoreCallRequest) -> CoreCallResponse {
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
}

impl CoreNodeRouter {
    /// 由应用装配一次；所有 Router 克隆与外围共享同一个节点服务。
    pub fn installNodeServices(&self, services: NodeServices) -> Result<(), String> {
        self.peerServices.set(services).map_err(|_| "Node services are already installed".into())
    }


    /// 同步和业务服务读取同一节点服务；未装配时明确报错，不退回旧全局连接表。
    pub fn nodeServices(&self) -> Result<&NodeServices, String> {
        self.peerServices.get().ok_or_else(|| "Node services are not installed".into())
    }

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
        let networkControlStore = NetworkControlStore::new(localCore.runtimeStorageHost())
            .expect("CoreNodeRouter requires network control storage");
        spaceStore
            .initialize()
            .expect("CoreNodeRouter requires an initialized Space");
        let peerServices = localCore.peerServices.clone();
        localCore
            .bindCoreNodeToolRuntime(Arc::new(CoreNodeToolRouteRuntime {
                peerServices: peerServices.clone(),
                localNodeId: localNodeId.clone(),
                spaceStore: spaceStore.clone(),
                networkControlStore: networkControlStore.clone(),
            }))
            .expect("CoreNodeRouter requires tool routing state registration");
        let router = Self {
            peerServices,
            localCore,
            bindingStore,
            localNodeId,
            spaceStore,
            networkControlStore,
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
    pub fn targetForSchema(&self, schema: &str) -> Option<&'static str> {
        self.localCore.targetForSchema(schema)
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

    /// Resolves one Binding to its selected device.
    #[allow(non_snake_case)]
    fn bindingRouteNodeId(&self, key: &str) -> Result<String, CoreLinkError> {
        let targetNodeId = self.effectiveBinding(key)?.nodeId;
        operit_util::AppLogger::AppLogger::trace(
            "CoreNodeRouteTrace",
            &format!(
                "binding_target_resolve key={} local={} selected={}",
                key, self.localNodeId, targetNodeId
            ),
        );
        Ok(targetNodeId)
    }

    /// Selects local execution while the bound owner is unavailable without mutating ownership.
    fn effectiveBinding(&self, key: &str) -> Result<CoreNodeBindingRecord, CoreLinkError> {
        let mut binding = self.bindingStore.binding(key)?;
        if !self.nodeIsReachable(&binding.nodeId).map_err(CoreLinkError::internal)? {
            binding.nodeId = self.localNodeId.clone();
        }
        Ok(binding)
    }

    /// Returns persisted ownership and the effective route for an opaque binding key.
    pub fn bindingRouteStatus(&self, key: String) -> Result<BindingRouteStatus, String> {
        let binding = self.bindingStore.binding(&key).map_err(|error| error.to_string())?;
        let ownerReachable = self.nodeIsReachable(&binding.nodeId)?;
        let profiles = self.spaceStore.deviceProfiles()?;
        let profile = profiles.get(&binding.nodeId).ok_or_else(|| {
            format!("Binding owner device profile is missing: {}", binding.nodeId)
        })?;
        let ownerIsLocal = binding.nodeId == self.localNodeId;
        Ok(BindingRouteStatus {
            activeNodeId: if ownerReachable { binding.nodeId.clone() } else { self.localNodeId.clone() },
            activeIsLocal: ownerIsLocal || !ownerReachable,
            ownerNodeId: binding.nodeId,
            ownerPlatform: profile.platform.clone(),
            ownerIsLocal,
            ownerReachable,
            generation: binding.generation,
        })
    }

    /// Observes generic binding and reachability changes; None marks invalid route metadata.
    pub fn bindingRouteStatusFlow(&self, key: String) -> Result<StateFlow<Option<BindingRouteStatus>>, String> {
        let (changes, mut changed) = tokio::sync::mpsc::channel(1);
        let subscription = operit_store::SyncOperationStore::subscribeSyncMutations(move || {
            let _ = changes.try_send(());
        });
        let mut peers = self.nodeServices()?.peers().subscribePeerChanges();
        let state = StateFlow::new(Some(self.bindingRouteStatus(key.clone())?));
        let (stop, mut stopped) = oneshot::channel::<()>();
        let observed = state.map(move |value| { let _keepAlive = &stop; value });
        let router = self.clone();
        defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
            "binding-route-status-watch", Box::new(move || Box::pin(async move {
                let _subscription = subscription;
                loop {
                    tokio::select! {
                        biased;
                        _ = &mut stopped => break,
                        change = changed.recv() => { if change.is_none() { break; } },
                        change = peers.recv() => {
                            if matches!(change, Err(tokio::sync::broadcast::error::RecvError::Closed)) { break; }
                        },
                    }
                    match router.bindingRouteStatus(key.clone()) {
                        Ok(status) => state.set_value(Some(status)),
                        Err(error) => {
                            state.set_value(None);
                            AppLogger::w("CoreNodeRouter", &format!("Binding status invalid: {error}"));
                        },
                    }
                }
            })),
        ).map_err(|error| error.to_string())?;
        Ok(observed)
    }

    /// Enforces the capability declared by one generated Space route.
    fn requireRoutePermission(
        &self,
        route: &crate::GeneratedSpaceRoute,
        callerNodeId: &str,
        targetNodeId: &str,
    ) -> Result<(), CoreLinkError> {
        let capability = route.permissionCapability;
        let (subject, subjectNodeId) = match route.permissionScope {
            crate::GeneratedRoutePermissionSubject::Caller => ("caller", callerNodeId),
            crate::GeneratedRoutePermissionSubject::Target => ("target", targetNodeId),
        };
        if self
            .networkControlStore
            .nodeHasCapability(subjectNodeId, capability, None)
            .map_err(CoreLinkError::internal)?
        {
            return Ok(());
        }
        AppLogger::w(
            "CoreNodeRouter",
            &format!(
                "route permission denied method={} subject={} capability={} caller={} target={}",
                route.methodName, subject, capability, callerNodeId, targetNodeId
            ),
        );
        Err(CoreLinkError::withDetails(
            "ROUTE_PERMISSION_DENIED",
            format!(
                "Space route {} requires capability {} on {} {}",
                route.methodName, capability, subject, subjectNodeId
            ),
            CoreValue::Map(BTreeMap::from([
                (
                    "method".to_string(),
                    CoreValue::String(route.methodName.to_string()),
                ),
                (
                    "subject".to_string(),
                    CoreValue::String(subject.to_string()),
                ),
                (
                    "requiredCapability".to_string(),
                    CoreValue::String(capability.to_string()),
                ),
                (
                    "callerNodeId".to_string(),
                    CoreValue::String(callerNodeId.to_string()),
                ),
                (
                    "targetNodeId".to_string(),
                    CoreValue::String(targetNodeId.to_string()),
                ),
            ])),
        ))
    }

    /// Enforces the declared permission for one routed call request.
    fn requireCallPermission(
        &self,
        request: &CoreCallRequest,
        callerNodeId: &str,
        targetNodeId: &str,
    ) -> Result<(), CoreLinkError> {
        let route = crate::generated_space_call_route(request).ok_or_else(|| {
            CoreLinkError::new("SPACE_ROUTE_NOT_FOUND", "Space call route is not registered")
        })?;
        self.requireRoutePermission(&route, callerNodeId, targetNodeId)
    }

    /// Enforces the declared permission for one routed watch request.
    fn requireWatchPermission(
        &self,
        request: &CoreWatchRequest,
        callerNodeId: &str,
        targetNodeId: &str,
    ) -> Result<(), CoreLinkError> {
        let route = if request.target == CORE_STREAM_TARGET {
            embeddedStreamSourceRoute(request)?
        } else {
            crate::generated_space_watch_route(request).ok_or_else(|| {
                CoreLinkError::new("SPACE_ROUTE_NOT_FOUND", "Space watch route is not registered")
            })?
        };
        self.requireRoutePermission(&route, callerNodeId, targetNodeId)
    }

    /// Enforces the declared permission for one routed push request.
    fn requirePushPermission(
        &self,
        request: &CorePushRequest,
        callerNodeId: &str,
        targetNodeId: &str,
    ) -> Result<(), CoreLinkError> {
        let Some(route) = crate::generated_space_push_route(request) else {
            return Ok(());
        };
        self.requireRoutePermission(&route, callerNodeId, targetNodeId)
    }
    /// A transport grant is usable only after both memberships have been
    /// admitted by the current Space control log; mere pairing is insufficient.
    pub(crate) fn spaceChannelScope(&self, peer: &str) -> Result<Option<String>, CoreLinkError> {
        let space = self.spaceStore.space().map_err(CoreLinkError::internal)?;
        let control = self.networkControlStore.currentState().map_err(CoreLinkError::internal)?;
        let members = [&self.localNodeId, &peer.to_string()];
        if peer == self.localNodeId || !control.initialized || control.spaceId != space.spaceId
            || members.iter().any(|node| !space.members.contains(node)
                || !control.memberNodeIds.contains(*node)
                || control.disconnectedNodeIds.contains(*node)) {
            return Ok(None);
        }
        Ok(Some(space.spaceId))
    }

    /// Reports whether the active Peer Link graph currently proves one device reachable.
    #[allow(non_snake_case)]
    pub fn nodeIsReachable(&self, targetNodeId: &str) -> Result<bool, String> {
        if targetNodeId == self.localNodeId { return Ok(true); }
        let peers = self.nodeServices()?.peers().activePeerNodeIds().map_err(|error| error.to_string())?;
        coreNodeIsReachableThroughPeers(
            &self.localNodeId,
            &self.spaceStore,
            &self.networkControlStore,
            targetNodeId,
            &peers,
        )
    }

    /// Resolves the multi-hop route plan this device would use for one Space member.
    ///
    /// `None` covers the local device itself, a policy-disconnected member, a node outside
    /// the current Space, and any member the active Peer Link graph cannot reach.
    #[allow(non_snake_case)]
    pub fn nodeRoutePlan(&self, targetNodeId: &str) -> Result<Option<CoreSpaceRoutePlan>, String> {
        if targetNodeId == self.localNodeId { return Ok(None); }
        let peers = self.nodeServices()?.peers().activePeerNodeIds().map_err(|error| error.to_string())?;
        coreNodeRoutePlanThroughPeers(
            &self.localNodeId,
            &self.spaceStore,
            &self.networkControlStore,
            targetNodeId,
            &peers,
        )
    }

    /// Opens the generic push session selected by generated CoreNode routing metadata.
    #[allow(non_snake_case)]
    async fn openPushSession(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let route = crate::generated_core_push_route(&request)?;
        let targetNodeId = self.routeNodeId(route).await?;
        self.requirePushPermission(&request, &self.localNodeId, &targetNodeId)?;
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
        self.initialRouteWithOrigin(targetNodeId, payload, self.localNodeId.clone())
    }

    /// Builds one routed envelope while preserving its original caller identity.
    fn initialRouteWithOrigin<T>(
        &self,
        targetNodeId: String,
        payload: T,
        originNodeId: String,
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
        if self
            .networkControlStore
            .nodeIsDisconnected(&targetNodeId)
            .map_err(CoreLinkError::internal)?
        {
            return Err(CoreLinkError::new(
                "CORE_NODE_REVOKED",
                format!("CoreNode is revoked from routing: {targetNodeId}"),
            ));
        }
        let ttl = routeTtl(&space)?;
        Ok(RoutedCoreRequest {
            spaceId: space.spaceId.clone(),
            originNodeId,
            targetNodeId,
            ttl,
            routeKind: RoutedCoreRequestKind::Target,
            payload,
        })
    }

    /// Builds one Space route envelope while preserving its original caller identity.
    fn initialSpaceRouteWithOrigin<T>(
        &self,
        targetNodeId: String,
        payload: T,
        originNodeId: String,
    ) -> Result<RoutedCoreRequest<T>, CoreLinkError> {
        let mut route = self.initialRouteWithOrigin(targetNodeId, payload, originNodeId)?;
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
    ) -> Result<(Arc<dyn RuntimePeerService>, String, RoutedCoreRequest<T>), CoreLinkError> {
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
        if self
            .networkControlStore
            .nodeIsDisconnected(&request.targetNodeId)
            .map_err(CoreLinkError::internal)?
        {
            return Err(CoreLinkError::new(
                "CORE_NODE_REVOKED",
                format!(
                    "Routed request targets a revoked CoreNode: {}",
                    request.targetNodeId
                ),
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
        let services = self.peerServices.get().ok_or_else(||
            CoreLinkError::new("PEER_SERVICE_UNAVAILABLE", "RuntimePeerService has not been installed"))?;
        Ok((services.peers(), nextNodeId, request))
    }

    /// Resolves one active first hop after removing previous and failed adjacent devices.
    #[allow(non_snake_case)]
    fn nextActiveHopAvoiding(
        &self,
        targetNodeId: String,
        previousNodeId: Option<&str>,
        excludedPeerNodeIds: &BTreeSet<String>,
    ) -> Result<String, CoreLinkError> {
        let mut peers = self.nodeServices().map_err(CoreLinkError::internal)?.peers().activePeerNodeIds()?;
        if let Some(previousNodeId) = previousNodeId {
            peers.remove(previousNodeId);
        }
        for peerNodeId in excludedPeerNodeIds {
            peers.remove(peerNodeId);
        }
        let (blockedNodeIds, transitNodeIds) = self
            .networkControlStore
            .routingPolicySnapshot()
            .map_err(CoreLinkError::internal)?;
        for peerNodeId in blockedNodeIds {
            peers.remove(&peerNodeId);
        }
        let activePeers = peers.iter().cloned().collect::<Vec<_>>();
        let nextHop = self
            .spaceStore
            .reachableNextHopThroughPeersWithTransitNodes(
                targetNodeId.clone(),
                peers,
                transitNodeIds,
            )
            .map_err(CoreLinkError::internal)?;
        if nextHop.is_none() {
            AppLogger::v_with_level(
                "CoreNodeRouter",
                &format!(
                    "no active route source={} target={} previous={:?} peers={:?}",
                    self.localNodeId, targetNodeId, previousNodeId, activePeers
                ),
                VERBOSE_LEVEL_6,
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
        if self
            .networkControlStore
            .nodeIsDisconnected(previousNodeId)
            .map_err(CoreLinkError::internal)?
        {
            return Err(CoreLinkError::new(
                "PREVIOUS_CORE_NODE_REVOKED",
                format!("Previous CoreNode is revoked: {previousNodeId}"),
            ));
        }
        if !space
            .members
            .iter()
            .any(|member| member == &request.originNodeId)
        {
            return Err(CoreLinkError::new(
                "ORIGIN_CORE_NODE_NOT_IN_SPACE",
                format!(
                    "Origin CoreNode is not a Space member: {}",
                    request.originNodeId
                ),
            ));
        }
        if self
            .networkControlStore
            .nodeIsDisconnected(&request.originNodeId)
            .map_err(CoreLinkError::internal)?
        {
            return Err(CoreLinkError::new(
                "ORIGIN_CORE_NODE_REVOKED",
                format!(
                    "Origin CoreNode is revoked: {}",
                    request.originNodeId
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
        if self
            .networkControlStore
            .nodeIsDisconnected(&request.targetNodeId)
            .map_err(CoreLinkError::internal)?
        {
            return Err(CoreLinkError::new(
                "CORE_NODE_REVOKED",
                format!(
                    "Routed request targets a revoked CoreNode: {}",
                    request.targetNodeId
                ),
            ));
        }
        let atTarget = request.targetNodeId == self.localNodeId;
        Ok(atTarget)
    }

    /// Executes one call on an explicit target CoreNode.
    #[allow(non_snake_case)]
    pub async fn callNode(
        &self,
        targetNodeId: String,
        request: CoreCallRequest,
    ) -> CoreCallResponse {
        if request.target == NODE_SPACE_TARGET {
            let requestId = request.requestId.clone();
            if let Err(error) = self.requireDirectPeer(&targetNodeId, false) {
                return CoreCallResponse::err(requestId, error);
            }
            let space = match self.spaceStore.initialize() {
                Ok(space) => space,
                Err(error) => return CoreCallResponse::err(requestId, CoreLinkError::internal(error)),
            };
            let peers = match self.nodeServices() {
                Ok(services) => services.peers(),
                Err(error) => return CoreCallResponse::err(requestId, CoreLinkError::internal(error)),
            };
            return peers.call(&targetNodeId, RoutedCoreRequest {
                spaceId: space.spaceId,
                originNodeId: self.localNodeId.clone(),
                targetNodeId: targetNodeId.clone(),
                ttl: 0,
                routeKind: RoutedCoreRequestKind::Target,
                payload: request,
            }).await;
        }
        self.callNodeWithKind(targetNodeId, request, RoutedCoreRequestKind::Target)
            .await
    }

    /// Executes one Space route call on an explicit target CoreNode.
    #[allow(non_snake_case)]
    pub async fn callNodeSpace(
        &self,
        targetNodeId: String,
        request: CoreCallRequest,
    ) -> CoreCallResponse {
        self.callNodeWithKind(targetNodeId, request, RoutedCoreRequestKind::SpaceRoute)
            .await
    }

    /// Executes one call on an explicit CoreNode with a selected request namespace.
    async fn callNodeWithKind(
        &self,
        targetNodeId: String,
        request: CoreCallRequest,
        routeKind: RoutedCoreRequestKind,
    ) -> CoreCallResponse {
        self.callNodeWithKindAndOrigin(
            targetNodeId,
            request,
            routeKind,
            self.localNodeId.clone(),
        )
        .await
    }

    /// Executes one call while preserving the original caller identity.
    async fn callNodeWithKindAndOrigin(
        &self,
        targetNodeId: String,
        request: CoreCallRequest,
        routeKind: RoutedCoreRequestKind,
        originNodeId: String,
    ) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let route = match if routeKind == RoutedCoreRequestKind::SpaceRoute {
            self.initialSpaceRouteWithOrigin(targetNodeId, request, originNodeId)
        } else {
            self.initialRouteWithOrigin(targetNodeId, request, originNodeId)
        } {
            Ok(value) => value,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRoute) =
                match self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone()) {
                    Ok(value) => value,
                    Err(error) => return CoreCallResponse::err(requestId, error),
                };
            let response = peer.call(&peerNodeId, forwardedRoute).await;
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
        self.callSpaceWithOrigin(request, self.localNodeId.clone()).await
    }

    /// Allocate bindings only for explicit creation commands, after caller authorization.
    async fn prepareCreationBinding(&self, route: &crate::GeneratedSpaceRoute, key: &str,
        origin: &str) -> Result<(), CoreLinkError> {
        if route.createBindingCapability.is_empty() { return Ok(()); }
        self.requireRoutePermission(route, origin, &self.localNodeId)?;
        let bindings = CoreNodeBindingStore::new(self.localCore.runtimeStorageHost()).map_err(CoreLinkError::internal)?;
        let commit = if let Some(binding) = bindings.bindingOptional(key).map_err(CoreLinkError::internal)? {
            if !self.networkControlStore.nodeHasCapability(&binding.nodeId, route.createBindingCapability, None).map_err(CoreLinkError::internal)? {
                return Err(CoreLinkError::new("ROUTE_PERMISSION_DENIED", "Binding target lacks execution capability"));
            }
            bindings.compareAndSet(key, &binding.nodeId, &binding.nodeId).map_err(CoreLinkError::internal)?
        } else {
            let mut members = self.spaceStore.space().map_err(CoreLinkError::internal)?.members;
            members.sort_by_key(|id| (id != &self.localNodeId, id.clone()));
            let mut selected = None;
            for member in members {
                if self.networkControlStore.nodeHasCapability(&member, route.createBindingCapability, None).map_err(CoreLinkError::internal)?
                    && (member == self.localNodeId || self.nodeIsReachable(&member).map_err(CoreLinkError::internal)?) {
                    selected = Some(member); break;
                }
            }
            let target = selected.ok_or_else(|| CoreLinkError::new("ROUTE_UNAVAILABLE", "No reachable node has the required execution capability"))?;
            bindings.create(key, &target).map_err(CoreLinkError::internal)?
        };
        if commit.binding.nodeId != self.localNodeId {
            let args = operit_link::toCoreValue(serde_json::json!({"operation": commit.operation}))
                .map_err(|error| CoreLinkError::internal(error.to_string()))?;
            let request = PeerSyncMethod::SyncApplyImmediateBindingOperation.request(
                operit_link::nextCoreRouteRequestId("binding-install"), args,
            );
            Box::pin(self.callNode(commit.binding.nodeId, request)).await.result?;
        }
        Ok(())
    }

    /// Executes one Space call while preserving its original caller identity.
    async fn callSpaceWithOrigin(
        &self,
        request: CoreCallRequest,
        originNodeId: String,
    ) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        if let Err(error) = validateEdgeEventOrigin(&request, &originNodeId, RoutedCoreRequestKind::SpaceBinding) {
            return CoreCallResponse::err(requestId, error);
        }
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
        if let Err(error) = self.prepareCreationBinding(&route, &bindingKey, &originNodeId).await {
            return CoreCallResponse::err(requestId, error);
        }
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
        if let Err(error) = self.requireRoutePermission(&route, &originNodeId, &targetNodeId) {
            return CoreCallResponse::err(requestId, error);
        }
        if targetNodeId == self.localNodeId {
            return self.localCore.callSpace(request).await;
        }
        self.callNodeWithKindAndOrigin(
            targetNodeId,
            request,
            RoutedCoreRequestKind::SpaceRoute,
            originNodeId,
        )
        .await
    }

    /// Reads one watch snapshot on an explicit target CoreNode.
    #[allow(non_snake_case)]
    pub async fn watchNodeSnapshot(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        self.watchNodeSnapshotWithOrigin(targetNodeId, request, self.localNodeId.clone())
            .await
    }

    /// Reads one watch snapshot while preserving its original caller identity.
    async fn watchNodeSnapshotWithOrigin(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
        originNodeId: String,
    ) -> Result<CoreEvent, CoreLinkError> {
        let route = self.initialRouteWithOrigin(targetNodeId, request, originNodeId)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            match peer.watchSnapshot(&peerNodeId, forwardedRoute).await {
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
        self.watchNodeWithOrigin(targetNodeId, request, self.localNodeId.clone())
            .await
    }

    /// Opens one watch while preserving its original caller identity.
    async fn watchNodeWithOrigin(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
        originNodeId: String,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let route = self.initialRouteWithOrigin(targetNodeId, request, originNodeId)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            match peer.watch(&peerNodeId, forwardedRoute).await {
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
        self.watchSpaceSnapshotWithOrigin(request, self.localNodeId.clone())
            .await
    }

    /// Reads one Space watch snapshot while preserving its original caller identity.
    async fn watchSpaceSnapshotWithOrigin(
        &self,
        request: CoreWatchRequest,
        originNodeId: String,
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
                key: bindingKey.clone(),
            })
            .await?;
        self.requireRoutePermission(&route, &originNodeId, &targetNodeId)?;
        if targetNodeId == self.localNodeId {
            return self.localCore.watchSpaceSnapshot(request).await;
        }
        self.watchNodeSnapshotSpaceWithOrigin(targetNodeId, request, originNodeId)
            .await
    }

    /// Opens one annotation-addressed Space watch through Binding routing.
    pub async fn watchSpace(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.watchSpaceWithOrigin(request, self.localNodeId.clone()).await
    }

    /// Opens one Space watch while preserving its original caller identity.
    async fn watchSpaceWithOrigin(
        &self,
        request: CoreWatchRequest,
        originNodeId: String,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if request.target == CORE_STREAM_TARGET {
            return self.watchSpaceEmbeddedStreamWithOrigin(request, originNodeId).await;
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
                key: bindingKey.clone(),
            })
            .await?;
        self.requireRoutePermission(&route, &originNodeId, &targetNodeId)?;
        self.watchBindingFlow(bindingKey, request, originNodeId, None)
    }

    /// Opens one embedded stream on the CoreNode selected by its producing route.
    #[allow(non_snake_case)]
    async fn watchSpaceEmbeddedStream(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEventStream, CoreLinkError> {
        self.watchSpaceEmbeddedStreamWithOrigin(request, self.localNodeId.clone()).await
    }

    /// Opens one embedded stream while preserving its original caller identity.
    async fn watchSpaceEmbeddedStreamWithOrigin(
        &self,
        request: CoreWatchRequest,
        originNodeId: String,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let arguments = embeddedStreamArguments(&request)?;
        let streamId = embeddedStreamStringArgument(arguments, "streamId")?;
        let sourceMethod =
            embeddedStreamStringArgument(arguments, CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT)?;
        let sourceMode =
            embeddedStreamStringArgument(arguments, CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT)?;
        let sourceArgs = embeddedStreamSourceArgs(&request)?;
        let route = embeddedStreamSourceRoute(&request)?;
        let bindingKey = route.bindingKey(sourceArgs)?;
        operit_util::AppLogger::AppLogger::trace(
            "CoreNodeRouteTrace",
            &format!(
                "embedded_route_resolve requestId={} streamId={} sourceMode={} sourceMethod={} routeId={} routeMethod={} key={} local={}",
                request.requestId.0,
                streamId,
                sourceMode,
                sourceMethod,
                route.routeId,
                route.methodName,
                bindingKey,
                self.localNodeId
            ),
        );
        let targetNodeId = self
            .routeNodeId(GeneratedCoreRoute::Binding {
                scope: 0,
                key: bindingKey.clone(),
            })
            .await?;
        self.requireRoutePermission(&route, &originNodeId, &targetNodeId)?;
        operit_util::AppLogger::AppLogger::trace(
            "CoreNodeRouteTrace",
            &format!(
                "embedded_route_target requestId={} streamId={} routeMethod={} local={} target={}",
                request.requestId.0, streamId, route.methodName, self.localNodeId, targetNodeId
            ),
        );
        // A response is a logical watch too: opening during peer admission or
        // losing its physical connection must not permanently end the UI stream.
        // The producer's explicit Completed event remains terminal (unlike a
        // StateFlow segment completing when it needs to be rebound).
        self.watchBindingFlow(bindingKey, request, originNodeId, None)
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
            let (peer, peerNodeId, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            match peer.openPush(&peerNodeId, forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Reads one Space watch snapshot while preserving its original caller identity.
    async fn watchNodeSnapshotSpaceWithOrigin(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
        originNodeId: String,
    ) -> Result<CoreEvent, CoreLinkError> {
        let route = self.initialSpaceRouteWithOrigin(targetNodeId, request, originNodeId)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            match peer.watchSnapshot(&peerNodeId, forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Opens one Space watch while preserving its original caller identity.
    async fn watchNodeSpaceWithOrigin(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
        originNodeId: String,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let route = self.initialSpaceRouteWithOrigin(targetNodeId, request, originNodeId)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            match peer.watch(&peerNodeId, forwardedRoute).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Opens one Space push on an explicit CoreNode while preserving its caller.
    async fn openPushNodeSpaceWithOrigin(
        &self,
        targetNodeId: String,
        request: CorePushRequest,
        originNodeId: String,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let route = self.initialSpaceRouteWithOrigin(targetNodeId, request, originNodeId)?;
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRoute) =
                self.forwardRouteAvoiding(None, &excludedPeerNodeIds, route.clone())?;
            match peer.openPush(&peerNodeId, forwardedRoute).await {
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
        self.openSpacePushWithOrigin(request, self.localNodeId.clone())
            .await
    }

    /// Opens one Space push while preserving its original caller identity.
    async fn openSpacePushWithOrigin(
        &self,
        request: CorePushRequest,
        originNodeId: String,
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
        self.requireRoutePermission(&route, &originNodeId, &targetNodeId)?;
        if targetNodeId == self.localNodeId {
            return self.localCore.openSpacePush(request);
        }
        self.openPushNodeSpaceWithOrigin(targetNodeId, request, originNodeId)
            .await
    }

    /// Executes one generated call directly on this CoreNode after Binding validation.
    #[allow(non_snake_case)]
    async fn executeLocalCall(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let applicationObjectId = self.localCore.targetForSchema("application");
        let isApplicationCall =
            applicationObjectId.is_some_and(|objectId| objectId == request.target);
        let response = if request.target == NODE_SPACE_APPROVAL_TARGET {
            let result = RuntimeRemoteLinkService::newWithRouter((*self.localCore).clone(), self.clone())
                .acceptSpaceApprovalCall(&self.localNodeId, request).await.map_err(CoreLinkError::internal);
            CoreCallResponse { requestId: requestId.clone(), result }
        } else if isApplicationCall {
            self.localCore.callApplication(request).await
        } else {
            operit_link::withCoreForceLocal(self.localCore.call(request)).await
        };
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

    /// Opens one Binding watch segment on an explicit CoreNode.
    #[allow(non_snake_case)]
    pub(crate) async fn watchBindingNode(
        &self,
        targetNodeId: String,
        request: CoreWatchRequest,
        originNodeId: String,
    ) -> Result<CoreEventStream, CoreLinkError> {
        if request.target == CORE_STREAM_TARGET {
            let route = embeddedStreamSourceRoute(&request)?;
            self.requireRoutePermission(&route, &originNodeId, &targetNodeId)?;
            if targetNodeId == self.localNodeId {
                operit_util::AppLogger::AppLogger::trace(
                    "CoreNodeRouteTrace",
                    &format!(
                        "binding_watch_open_local_embedded_stream requestId={} property={} local={}",
                        request.requestId.0, request.propertyName, self.localNodeId
                    ),
                );
                return self.localCore.watchSpace(request).await;
            }
            operit_util::AppLogger::AppLogger::trace(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_watch_open_remote_embedded_stream requestId={} property={} local={} target={}",
                    request.requestId.0, request.propertyName, self.localNodeId, targetNodeId
                ),
            );
            return self
                .watchNodeSpaceWithOrigin(targetNodeId, request, originNodeId)
                .await;
        }
        if let Some(route) = crate::generated_space_watch_route(&request) {
            self.requireRoutePermission(&route, &originNodeId, &targetNodeId)?;
            if targetNodeId == self.localNodeId {
                operit_util::AppLogger::AppLogger::trace(
                    "CoreNodeRouteTrace",
                    &format!(
                        "binding_watch_open_local_space requestId={} property={} local={}",
                        request.requestId.0, request.propertyName, self.localNodeId
                    ),
                );
                return self.localCore.watchSpace(request).await;
            }
            AppLogger::v_with_level(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_watch_open_remote_space requestId={} property={} local={} target={}",
                    request.requestId.0, request.propertyName, self.localNodeId, targetNodeId
                ),
                VERBOSE_LEVEL_5,
            );
            return self
                .watchNodeSpaceWithOrigin(targetNodeId, request, originNodeId)
                .await;
        }
        if targetNodeId == self.localNodeId {
            return operit_link::withCoreForceLocal(self.localCore.watch(request)).await;
        }
        self.watchNodeWithOrigin(targetNodeId, request, originNodeId)
            .await
    }

    /// Opens a long-lived logical Binding Flow over replaceable physical watch segments.
    #[allow(non_snake_case)]
    fn watchBindingFlow(
        &self,
        bindingKey: String,
        request: CoreWatchRequest,
        originNodeId: String,
        localStream: Option<CoreEventStream>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let peerChanges = self.nodeServices().map_err(CoreLinkError::internal)?.peers().subscribePeerChanges();
        let (sender, receiver) = CoreEventStream::channel();
        let (cancelSender, cancelReceiver) = oneshot::channel();
        let router = self.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "core-node-route-binding-flow",
                Box::new(move || {
                    Box::pin(async move {
                        router
                            .pumpBindingFlow(
                                bindingKey,
                                request,
                                sender,
                                cancelReceiver,
                                originNodeId,
                                localStream,
                                peerChanges,
                            )
                            .await;
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(receiver.withOnClose(move || {
            let _ = cancelSender.send(());
        }))
    }

    /// Continuously forwards the current Binding owner into one stable outer Flow stream.
    #[allow(non_snake_case)]
    async fn pumpBindingFlow(
        &self,
        bindingKey: String,
        request: CoreWatchRequest,
        sender: tokio::sync::mpsc::UnboundedSender<CoreEvent>,
        mut cancelReceiver: oneshot::Receiver<()>,
        originNodeId: String,
        mut initialLocalStream: Option<CoreEventStream>,
        mut peerChanges: tokio::sync::broadcast::Receiver<()>,
    ) {
        let requestId = request.requestId.0.clone();
        let propertyName = request.propertyName.clone();
        let isEmbeddedStream = request.target == CORE_STREAM_TARGET;
        let mut segmentCount = 0_u64;
        let mut initialSnapshotForwarded = false;
        let (changes, mut routeChanges) = tokio::sync::mpsc::channel(1);
        let _subscription = operit_store::SyncOperationStore::subscribeSyncMutations(move || {
            let _ = changes.try_send(());
        });
        'outer: loop {
            let binding = match self.effectiveBinding(&bindingKey) {
                Ok(binding) => binding,
                Err(error) => {
                    operit_util::AppLogger::AppLogger::e(
                        "CoreNodeRouteTrace",
                        &format!(
                            "binding_flow_binding_missing requestId={} property={} key={} error={}",
                            requestId, propertyName, bindingKey, error
                        ),
                    );
                    break;
                }
            };
            segmentCount += 1;
            let useInitialLocalStream =
                binding.nodeId == self.localNodeId && initialLocalStream.is_some();
            if binding.nodeId != self.localNodeId {
                initialLocalStream = None;
            }
            AppLogger::v_with_level(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_flow_segment_open requestId={} property={} key={} segment={} owner={} generation={} localSource={}",
                    requestId,
                    propertyName,
                    bindingKey,
                    segmentCount,
                    binding.nodeId,
                    binding.generation,
                    useInitialLocalStream
                ),
                VERBOSE_LEVEL_5,
            );
            AppLogger::i(
                "CoreNodeRouter",
                &format!(
                    "binding watch segment open requestId={} property={} owner={} generation={} localSource={}",
                    requestId,
                    propertyName,
                    binding.nodeId,
                    binding.generation,
                    useInitialLocalStream
                ),
            );
            let mut stream = if useInitialLocalStream {
                initialLocalStream
                    .take()
                    .expect("initial local stream must be present")
            } else {
                match self
                    .watchBindingNode(
                        binding.nodeId.clone(),
                        request.clone(),
                        originNodeId.clone(),
                    )
                    .await
                {
                    Ok(stream) => stream,
                    Err(error) => {
                        AppLogger::e(
                            "CoreNodeRouter",
                            &format!(
                                "binding watch segment failed requestId={} property={} owner={} generation={} error={}",
                                requestId, propertyName, binding.nodeId, binding.generation, error
                            ),
                        );
                        AppLogger::v_with_level(
                            "CoreNodeRouteTrace",
                            &format!(
                                "binding_flow_segment_open_failed requestId={} property={} key={} owner={} generation={} error={}",
                                requestId,
                                propertyName,
                                bindingKey,
                                binding.nodeId,
                                binding.generation,
                                error
                            ),
                            VERBOSE_LEVEL_6,
                        );
                        if self
                            .waitForBindingFlowRetryOrCancel(&mut cancelReceiver, &mut routeChanges, &mut peerChanges)
                            .await
                        {
                            break;
                        }
                        continue;
                    }
                }
            };
            AppLogger::i(
                "CoreNodeRouter",
                &format!(
                    "binding watch segment stream opened requestId={} property={} owner={} generation={}",
                    requestId, propertyName, binding.nodeId, binding.generation
                ),
            );
            let mut firstEventLogged = false;
            let mut peerChangedSinceOpen = false;

            loop {
                tokio::select! {
                    biased;
                    _ = &mut cancelReceiver => {
                        break 'outer;
                    }
                    // Persistent ownership/policy and authenticated peer changes
                    // already publish notifications. Polling every 50 ms here
                    // repeatedly scans topology/policy files even with no output.
                    routeChange = routeChanges.recv() => {
                        if routeChange.is_none() { break 'outer; }
                        if !self.bindingStillCurrent(&bindingKey, &binding) {
                            break;
                        }
                    }
                    peerChange = peerChanges.recv() => {
                        if matches!(peerChange, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                            break 'outer;
                        }
                        // Connection replacement can be announced before the
                        // old watch delivers EOF, without changing its owner.
                        // Remember that evidence rather than consuming it and
                        // then waiting forever for another notification at EOF.
                        peerChangedSinceOpen = true;
                        // Lagged notifications also require a fresh route check.
                        if !self.bindingStillCurrent(&bindingKey, &binding) {
                            break;
                        }
                    }
                    maybeEvent = stream.recv() => {
                        let Some(mut event) = maybeEvent else {
                            if !firstEventLogged {
                                AppLogger::e(
                                    "CoreNodeRouter",
                                    &format!(
                                        "binding watch segment closed before event requestId={} property={} owner={} generation={}",
                                        requestId, propertyName, binding.nodeId, binding.generation
                                    ),
                                );
                            }
                            AppLogger::v_with_level(
                                "CoreNodeRouteTrace",
                                &format!(
                                    "binding_flow_segment_closed requestId={} property={} key={} segment={} owner={} generation={}",
                                    requestId,
                                    propertyName,
                                    bindingKey,
                                    segmentCount,
                                    binding.nodeId,
                                    binding.generation
                                ),
                                VERBOSE_LEVEL_5,
                            );
                            break;
                        };
                        if !firstEventLogged {
                            firstEventLogged = true;
                            AppLogger::i(
                                "CoreNodeRouter",
                                &format!(
                                    "binding watch first event requestId={} property={} owner={} generation={} kind={:?}",
                                    requestId, propertyName, binding.nodeId, binding.generation, event.kind
                                ),
                            );
                        }
                        if event.kind == CoreEventKind::Completed {
                            if isEmbeddedStream {
                                if self.bindingStillCurrent(&bindingKey, &binding) {
                                    let _ = sender.send(event);
                                    break 'outer;
                                }
                                break;
                            }
                            AppLogger::e(
                                "CoreNodeRouter",
                                &format!(
                                    "binding watch completed requestId={} property={} owner={} generation={}",
                                    requestId, propertyName, binding.nodeId, binding.generation
                                ),
                            );
                            break;
                        }
                        if initialSnapshotForwarded && event.kind == CoreEventKind::Snapshot {
                            event.kind = CoreEventKind::Changed;
                        }
                        if !self.bindingStillCurrent(&bindingKey, &binding) {
                            AppLogger::v_with_level(
                                "CoreNodeRouteTrace",
                                &format!(
                                    "binding_flow_stale_event requestId={} property={} key={} segment={} owner={} generation={} kind={:?}",
                                    requestId,
                                    propertyName,
                                    bindingKey,
                                    segmentCount,
                                    binding.nodeId,
                                    binding.generation,
                                    event.kind
                                ),
                                VERBOSE_LEVEL_5,
                            );
                            break;
                        }
                        AppLogger::v_with_level(
                            "CoreNodeRouteTrace",
                            &format!(
                                "binding_flow.forwarded requestId={} property={} key={} segment={} owner={} generation={} kind={:?}",
                                requestId,
                                propertyName,
                                bindingKey,
                                segmentCount,
                                binding.nodeId,
                                binding.generation,
                                event.kind
                            ),
                            VERBOSE_LEVEL_6,
                        );
                        if event.kind == CoreEventKind::Snapshot {
                            AppLogger::i(
                                "CoreNodeRouter",
                                &format!(
                                    "binding watch snapshot forwarded requestId={} property={} owner={} generation={}",
                                    requestId, propertyName, binding.nodeId, binding.generation
                                ),
                            );
                        }
                        if event.kind == CoreEventKind::Snapshot {
                            initialSnapshotForwarded = true;
                        }
                        if sender.send(event).is_err() {
                            AppLogger::v_with_level(
                                "CoreNodeRouteTrace",
                                &format!(
                                    "binding_flow.receiver_closed requestId={} property={} key={} segment={}",
                                    requestId,
                                    propertyName,
                                    bindingKey,
                                    segmentCount
                                ),
                                VERBOSE_LEVEL_5,
                            );
                            break 'outer;
                        }
                    }

                }
            }
            if self.bindingStillCurrent(&bindingKey, &binding)
                && !(isEmbeddedStream && peerChangedSinceOpen)
                && self
                    .waitForBindingFlowRetryOrCancel(&mut cancelReceiver, &mut routeChanges, &mut peerChanges)
                    .await
            {
                break;
            }
        }
    }

    /// Waits for routing evidence before reopening a failed physical segment.
    #[allow(non_snake_case)]
    async fn waitForBindingFlowRetryOrCancel(
        &self,
        cancelReceiver: &mut oneshot::Receiver<()>,
        routeChanges: &mut tokio::sync::mpsc::Receiver<()>,
        peerChanges: &mut tokio::sync::broadcast::Receiver<()>,
    ) -> bool {
        tokio::select! {
            biased;
            _ = &mut *cancelReceiver => true,
            event = routeChanges.recv() => event.is_none(),
            event = peerChanges.recv() => matches!(event, Err(tokio::sync::broadcast::error::RecvError::Closed)),
        }
    }

    /// Reports whether a Binding record still selects the same physical watch segment.
    #[allow(non_snake_case)]
    fn bindingStillCurrent(&self, bindingKey: &str, previous: &CoreNodeBindingRecord) -> bool {
        self.effectiveBinding(bindingKey)
            .map(|current| {
                current.nodeId == previous.nodeId && current.generation == previous.generation
            })
            .unwrap_or(false)
    }
}

/// Supplies built-in tools with the same live reachability view used by the router.
struct CoreNodeToolRouteRuntime {
    peerServices: Arc<std::sync::OnceLock<NodeServices>>,
    localNodeId: String,
    spaceStore: CoreSpaceStore,
    networkControlStore: NetworkControlStore,
}

impl CoreNodeToolRuntime for CoreNodeToolRouteRuntime {
    /// Returns every device-space member with current route reachability.
    #[allow(non_snake_case)]
    fn coreNodeRouteState(&self) -> Result<RuntimeCoreNodeRouteState, String> {
        let space = self.spaceStore.initialize()?;
        let profiles = self.spaceStore.deviceProfiles()?;
        let peers = self.peerServices.get()
            .ok_or_else(|| "Node services are not installed".to_string())?
            .peers().activePeerNodeIds().map_err(|error| error.to_string())?;
        let mut nodes = Vec::with_capacity(space.members.len());
        for nodeId in space.members {
            let profile = profiles.get(&nodeId).ok_or_else(|| {
                format!("Device profile is missing in the current device space: {nodeId}")
            })?;
            let plan = coreNodeRoutePlanThroughPeers(
                &self.localNodeId,
                &self.spaceStore,
                &self.networkControlStore,
                &nodeId,
                &peers,
            )?;
            // Any resolved plan proves reachability, including the single-hop
            // plan of a directly paired peer; relay metadata stays multi-hop only.
            let reachable = nodeId == self.localNodeId || plan.is_some();
            let (relayHops, relayPath) = match plan {
                Some(plan) if plan.hops() > 1 => {
                    (Some(plan.hops()), Some(plan.path.clone()))
                }
                _ => (None, None),
            };
            nodes.push(RuntimeCoreNodeStatus {
                displayName: profile.displayName.clone(),
                userName: profile.userName.clone(),
                platform: profile.platform.clone(),
                model: profile.model.clone(),
                reachable,
                relayHops,
                relayPath,
                nodeId,
            });
        }
        Ok(RuntimeCoreNodeRouteState {
            currentNodeId: self.localNodeId.clone(),
            nodes,
        })
    }
}

/// Reports device reachability through one fixed active Peer Link snapshot.
#[allow(non_snake_case)]
fn coreNodeIsReachableThroughPeers(
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    networkControlStore: &NetworkControlStore,
    targetNodeId: &str,
    peers: &BTreeSet<String>,
) -> Result<bool, String> {
    if targetNodeId == localNodeId {
        return Ok(true);
    }
    Ok(coreNodeRoutePlanThroughPeers(
        localNodeId,
        spaceStore,
        networkControlStore,
        targetNodeId,
        peers,
    )?
    .is_some())
}

/// Resolves the multi-hop route plan reachable through one fixed active Peer Link snapshot.
#[allow(non_snake_case)]
fn coreNodeRoutePlanThroughPeers(
    localNodeId: &str,
    spaceStore: &CoreSpaceStore,
    networkControlStore: &NetworkControlStore,
    targetNodeId: &str,
    peers: &BTreeSet<String>,
) -> Result<Option<CoreSpaceRoutePlan>, String> {
    if targetNodeId == localNodeId {
        return Ok(None);
    }
    if networkControlStore.nodeIsDisconnected(targetNodeId)? {
        return Ok(None);
    }
    if !spaceStore.contains(targetNodeId.to_string())? {
        return Ok(None);
    }
    let permittedPeers = permittedActivePeers(networkControlStore, peers)?;
    let transitNodeIds = networkControlStore.relayNodeIds()?;
    spaceStore.routePlanThroughPeersWithTransitNodes(
        targetNodeId.to_string(),
        permittedPeers,
        transitNodeIds,
    )
}

/// Removes peers the current Space policy has disconnected from one active Peer Link snapshot.
#[allow(non_snake_case)]
fn permittedActivePeers(
    networkControlStore: &NetworkControlStore,
    peers: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    let mut permittedPeers = peers.clone();
    for nodeId in peers {
        if networkControlStore.nodeIsDisconnected(nodeId)? {
            permittedPeers.remove(nodeId);
        }
    }
    Ok(permittedPeers)
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
            target: CORE_INTERNAL_TARGET.to_string(),
            methodName: sourceMethod,
            args: sourceArgs.clone(),
        }),
        "watch" => crate::generated_space_watch_route(&CoreWatchRequest {
            requestId: request.requestId.clone(),
            target: CORE_INTERNAL_TARGET.to_string(),
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

// 接收入口只接受 runtime 已鉴权的相邻节点；传输层不能直接把自报身份传进来。
impl CoreNodeRouter {
    /// Space 加入前只允许直接配对的业务 Call；出入授权不互相推导。
    fn requireDirectPeer(&self, peerNodeId: &str, inbound: bool) -> Result<(), CoreLinkError> {
        let peers = self.nodeServices().map_err(CoreLinkError::internal)?.peers().pairedPeers()?;
        if peerNodeId == self.localNodeId || !peers.iter().any(|peer|
            peer.nodeId == peerNodeId && if inbound { peer.inbound } else { peer.outbound }
        ) {
            return Err(CoreLinkError::new("PEER_NOT_AUTHORIZED", "Direct directional pairing is required"));
        }
        if self.networkControlStore.nodeIsDisconnected(peerNodeId).map_err(CoreLinkError::internal)? {
            return Err(CoreLinkError::new("CORE_NODE_REVOKED", "Peer is revoked from this Space"));
        }
        Ok(())
    }

    /// Executes or forwards one routed call. previousNodeId must come from runtime authentication,
    /// never from peer endpoint metadata or a request argument.
    #[allow(non_snake_case)]
    pub async fn routedCall(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let requestId = request.payload.requestId.clone();
        if request.payload.target.starts_with("core/server.") {
            return CoreCallResponse::err(requestId, CoreLinkError::new(
                "LOCAL_MANAGEMENT_ONLY", "Node management is only available to the local application",
            ));
        }
        if request.payload.target == NODE_SPACE_TARGET {
            let result = (|| {
                if request.targetNodeId != self.localNodeId
                    || request.originNodeId != previousNodeId
                    || request.ttl != 0
                    || request.routeKind != RoutedCoreRequestKind::Target
                {
                    return Err(CoreLinkError::new("PEER_DIRECT_CALL_REQUIRED", "Node Space calls cannot be relayed"));
                }
                self.requireDirectPeer(&previousNodeId, true)?;
                RuntimeRemoteLinkService::newWithRouter((*self.localCore).clone(), self.clone())
                    .acceptPeerSpaceCall(&previousNodeId, request.payload)
                    .map_err(CoreLinkError::internal)
            })();
            return CoreCallResponse { requestId, result };
        }
        if request.payload.target == NODE_SYNC_TARGET {
            match self.validatePeerSyncRoute(&previousNodeId, &request)
                .and_then(|atTarget| {
                    let method = PeerSyncMethod::fromRequest(&request.payload)?;
                    if method.requiresStorageProvider() {
                        self.requirePeerStorageProviders(&request.originNodeId, &request.targetNodeId)?;
                    }
                    Ok((atTarget, method))
                })
            {
                Err(error) => return CoreCallResponse::err(requestId, error),
                Ok((true, method)) => return self.dispatchPeerSyncCall(method, request.payload).await,
                Ok((false, _)) => {} // Preserve the protocol target when relaying.
            }
        }
        match self.validateIncomingRoute(&previousNodeId, &request) {
            Ok(true) => {
                if let Err(error) = validateEdgeEventOrigin(&request.payload, &request.originNodeId, request.routeKind) {
                    return CoreCallResponse::err(requestId, error);
                }
                if request.payload.target == crate::NodeSpaceService::NODE_SPACE_CONTROL_TARGET {
                    if request.routeKind != RoutedCoreRequestKind::Target {
                        return CoreCallResponse::err(requestId, CoreLinkError::new("CONTROL_ROUTE_INVALID", "Control exchange requires an explicit same-Space target"));
                    }
                    let service = RuntimeRemoteLinkService::newWithRouter((*self.localCore).clone(), self.clone());
                    let result = crate::NodeSpaceService::acceptSpaceControlCall(&service, &request.originNodeId, request.payload)
                        .map_err(CoreLinkError::internal);
                    return CoreCallResponse { requestId, result };
                }
                if request.payload.target == NODE_SPACE_APPROVAL_TARGET {
                    if request.routeKind != RoutedCoreRequestKind::Target {
                        return CoreCallResponse::err(requestId, CoreLinkError::new("APPROVAL_ROUTE_INVALID", "Approval requires explicit same-Space routing"));
                    }
                    let result = RuntimeRemoteLinkService::newWithRouter((*self.localCore).clone(), self.clone())
                        .acceptSpaceApprovalCall(&request.originNodeId, request.payload).await.map_err(CoreLinkError::internal);
                    return CoreCallResponse { requestId, result };
                }
                if request.routeKind == RoutedCoreRequestKind::SpaceBinding {
                    return self
                    .callSpaceWithOrigin(request.payload, request.originNodeId)
                    .await;
                }
                if request.routeKind == RoutedCoreRequestKind::SpaceRoute {
                    if let Err(error) =
                        self.requireCallPermission(&request.payload, &request.originNodeId, &self.localNodeId)
                    {
                        return CoreCallResponse::err(requestId, error);
                    }
                    self.localCore.callSpace(request.payload).await
                } else {
                    self.executeLocalCall(request.payload).await
                }
            }
            Ok(false) => {
                let mut excludedPeerNodeIds = BTreeSet::new();
                loop {
                    let (peer, peerNodeId, forwardedRequest) = match self.forwardRouteAvoiding(
                        Some(&previousNodeId),
                        &excludedPeerNodeIds,
                        request.clone(),
                    ) {
                        Ok(value) => value,
                        Err(error) => return CoreCallResponse::err(requestId, error),
                    };
                            let response = peer.call(&peerNodeId, forwardedRequest).await;
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
    pub async fn routedWatchSnapshot(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        // 节点控制 Service 是本机 Proxy 表面；远端只能进入业务路由。
        if request.payload.target.starts_with("core/server.") {
            return Err(CoreLinkError::new("LOCAL_MANAGEMENT_ONLY", "Node management is only available to the local application"));
        }

        if self.validateIncomingRoute(&previousNodeId, &request)? {
            if request.routeKind == RoutedCoreRequestKind::SpaceBinding {
                return self
                    .watchSpaceSnapshotWithOrigin(request.payload, request.originNodeId)
                    .await;
            }
            return if request.routeKind == RoutedCoreRequestKind::SpaceRoute {
                self.requireWatchPermission(&request.payload, &request.originNodeId, &self.localNodeId)?;
                self.localCore.watchSpaceSnapshot(request.payload).await
            } else {
                operit_link::withCoreForceLocal(self.localCore.watchSnapshot(request.payload)).await
            };
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            match peer.watchSnapshot(&peerNodeId, forwardedRequest).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Executes or forwards one routed watch.
    #[allow(non_snake_case)]
    pub async fn routedWatch(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        // 节点控制 Service 是本机 Proxy 表面；远端只能进入业务路由。
        if request.payload.target.starts_with("core/server.") {
            return Err(CoreLinkError::new("LOCAL_MANAGEMENT_ONLY", "Node management is only available to the local application"));
        }

        let atTarget = self.validateIncomingRoute(&previousNodeId, &request)?;
        operit_util::AppLogger::AppLogger::trace(
            "CoreNodeRouteTrace",
            &format!(
                "route_watch_incoming requestId={} property={} object={} previous={} local={} target={} ttl={} routeKind={:?} atTarget={}",
                request.payload.requestId.0,
                request.payload.propertyName,
                request.payload.target,
                previousNodeId,
                self.localNodeId,
                request.targetNodeId,
                request.ttl,
                request.routeKind,
                atTarget
            ),
        );
        if atTarget {
            if request.routeKind == RoutedCoreRequestKind::SpaceBinding {
                return self
                    .watchSpaceWithOrigin(request.payload, request.originNodeId)
                    .await;
            }
            return if request.routeKind == RoutedCoreRequestKind::SpaceRoute {
                self.requireWatchPermission(&request.payload, &request.originNodeId, &self.localNodeId)?;
                self.localCore.watchSpace(request.payload).await
            } else if request.payload.target == CORE_STREAM_TARGET {
                self.localCore.watchSpace(request.payload).await
            } else {
                operit_link::withCoreForceLocal(self.localCore.watch(request.payload)).await
            };
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            match peer.watch(&peerNodeId, forwardedRequest).await {
                Err(error) if isRouteUnavailableError(&error) => {
                    excludedPeerNodeIds.insert(peerNodeId);
                }
                result => return result,
            }
        }
    }

    /// Executes or forwards one routed push open.
    #[allow(non_snake_case)]
    pub async fn routedOpenPush(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        // 节点控制 Service 是本机 Proxy 表面；远端只能进入业务路由。
        if request.payload.target.starts_with("core/server.") {
            return Err(CoreLinkError::new("LOCAL_MANAGEMENT_ONLY", "Node management is only available to the local application"));
        }

        if request.payload.target == NODE_SYNC_TARGET {
            let atTarget = self.validatePeerSyncRoute(&previousNodeId, &request)?;
            crate::PeerSync::PeerSyncPushMethod::fromRequest(&request.payload)?;
            self.requirePeerStorageProviders(&request.originNodeId, &request.targetNodeId)?;
            if atTarget {
                let inner = self.dispatchPeerSyncPush(request.payload)?;
                return Ok(self.guardPeerSyncPush(request.originNodeId, request.targetNodeId, request.spaceId, inner));
            }
        }
        if self.validateIncomingRoute(&previousNodeId, &request)? {
            if request.payload.target == "edge.audio.ingress" {
                if request.routeKind != RoutedCoreRequestKind::SpaceRoute || request.payload.methodName != "upload" {
                    return Err(CoreLinkError::new("AUDIO_ORIGIN_DENIED", "Audio ingress requires an authenticated SpaceRoute upload"));
                }
                let ingress = self.localCore.edgeAudioIngress.as_ref().ok_or_else(||
                    CoreLinkError::new("AUDIO_UNAVAILABLE", "Core audio receiving is not installed"))?;
                return ingress(request.payload, request.originNodeId);
            }
            if request.routeKind == RoutedCoreRequestKind::SpaceBinding {
                return self
                    .openSpacePushWithOrigin(request.payload, request.originNodeId)
                    .await;
            }
            return if request.routeKind == RoutedCoreRequestKind::SpaceRoute {
                self.requirePushPermission(&request.payload, &request.originNodeId, &self.localNodeId)?;
                self.localCore.openSpacePush(request.payload)
            } else {
                self.localCore.openPush(request.payload)
            };
        }
        let mut excludedPeerNodeIds = BTreeSet::new();
        loop {
            let (peer, peerNodeId, forwardedRequest) = self.forwardRouteAvoiding(
                Some(&previousNodeId),
                &excludedPeerNodeIds,
                request.clone(),
            )?;
            match peer.openPush(&peerNodeId, forwardedRequest).await {
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
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        let generatedRoute = match crate::generated_core_call_route(&request) {
            Ok(route) => route,
            Err(error) => return CoreCallResponse::err(requestId, error),
        };
        let route = generatedRoute;
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::trace(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_call_resolve requestId={} path={} method={} key={} local={}",
                    request.requestId.0,
                    request.target.to_string(),
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
        if targetNodeId != self.localNodeId {
            if let Err(error) = self.requireCallPermission(&request, &self.localNodeId, &targetNodeId) {
                return CoreCallResponse::err(requestId, error);
            }
        }
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::i(
                "CoreNodeRouter",
                &format!(
                    "binding call route requestId={} path={} method={} key={} source={} target={}",
                    request.requestId.0,
                    request.target.to_string(),
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
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        let route = crate::generated_core_watch_route(&request)?;
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::trace(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_snapshot_resolve requestId={} path={} property={} key={} local={}",
                    request.requestId.0,
                    request.target.to_string(),
                    request.propertyName,
                    key,
                    self.localNodeId
                ),
            );
        }
        let targetNodeId = self.routeNodeId(route).await?;
        if targetNodeId != self.localNodeId {
            self.requireWatchPermission(&request, &self.localNodeId, &targetNodeId)?;
        }
        if targetNodeId == self.localNodeId {
            return operit_link::withCoreForceLocal(self.localCore.watchSnapshot(request)).await;
        }
        self.watchNodeSnapshot(targetNodeId, request).await
    }

    /// Opens a watch stream on the CoreNode selected by generated metadata.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        if request.target == CORE_STREAM_TARGET {
            operit_util::AppLogger::AppLogger::trace(
                "CoreNodeRouteTrace",
                &format!(
                    "watch_embedded_open requestId={} property={} object={} local={}",
                    request.requestId.0,
                    request.propertyName,
                    request.target,
                    self.localNodeId
                ),
            );
            return self.watchSpaceEmbeddedStream(request).await;
        }
        let route = crate::generated_core_watch_route(&request)?;
        if let GeneratedCoreRoute::Binding { key, .. } = &route {
            operit_util::AppLogger::AppLogger::trace(
                "CoreNodeRouteTrace",
                &format!(
                    "binding_watch_resolve requestId={} path={} property={} key={} local={}",
                    request.requestId.0,
                    request.target.to_string(),
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
                    request.target.to_string(),
                    request.propertyName,
                    key,
                    self.localNodeId
                ),
            );
            let routeMetadata = crate::generated_space_watch_route(&request).ok_or_else(|| {
                CoreLinkError::new(
                    "SPACE_ROUTE_NOT_FOUND",
                    "Space watch route is not registered",
                )
            })?;
            let targetNodeId = self.bindingStore.bindingNodeId(key)?;
            self.requireRoutePermission(&routeMetadata, &self.localNodeId, &targetNodeId)?;
            return self.watchBindingFlow(
                key.clone(),
                request,
                self.localNodeId.clone(),
                None,
            );
        }
        let targetNodeId = self.routeNodeId(route).await?;
        operit_util::AppLogger::AppLogger::trace(
            "CoreNodeRouteTrace",
            &format!(
                "watch_target_resolve requestId={} property={} object={} local={} target={}",
                request.requestId.0,
                request.propertyName,
                request.target,
                self.localNodeId,
                targetNodeId
            ),
        );
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
            operit_link::CORE_INTERNAL_TARGET,
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
                Ok(targetNodeId != self.localNodeId
                    && self
                        .nodeIsReachable(&targetNodeId)
                        .map_err(CoreLinkError::internal)?)
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
            operit_link::CORE_INTERNAL_TARGET,
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
                Ok(targetNodeId != self.localNodeId
                    && self.nodeIsReachable(&targetNodeId).map_err(CoreLinkError::internal)?)
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
            operit_link::CORE_INTERNAL_TARGET,
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
                Ok(targetNodeId == self.localNodeId
                    || !self.nodeIsReachable(&targetNodeId).map_err(CoreLinkError::internal)?)
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
            if request.target == CORE_STREAM_TARGET {
                let route = embeddedStreamSourceRoute(&request)?;
                let bindingKey = route.bindingKey(embeddedStreamSourceArgs(&request)?)?;
                let targetNodeId = router
                    .routeNodeId(GeneratedCoreRoute::Binding {
                        scope: 0,
                        key: bindingKey,
                    })
                    .await?;
                if targetNodeId == router.localNodeId {
                    return Ok(localStream);
                }
                return router
                    .watchNodeSpaceWithOrigin(
                        targetNodeId,
                        request,
                        router.localNodeId.clone(),
                    )
                    .await;
            }
            let route = crate::generated_core_watch_route(&request)?;
            match route {
                GeneratedCoreRoute::Binding { key, .. } => {
                    let routeMetadata = crate::generated_space_watch_route(&request).ok_or_else(|| {
                        CoreLinkError::new(
                            "SPACE_ROUTE_NOT_FOUND",
                            "Space watch route is not registered",
                        )
                    })?;
                    let targetNodeId = router.bindingStore.bindingNodeId(&key)?;
                    router.requireRoutePermission(&routeMetadata, &router.localNodeId, &targetNodeId)?;
                    router.watchBindingFlow(
                        key,
                        request,
                        router.localNodeId.clone(),
                        Some(localStream),
                    )
                }
                GeneratedCoreRoute::Local => Ok(localStream),
            }
        })
    }
}

#[async_trait(?Send)]
impl crate::PeerRouter::PeerRouter for CoreNodeRouter {
    fn localNodeId(&self) -> String {
        CoreNodeRouter::localNodeId(self)
    }
    async fn routedCall(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let mut router = self.clone();
        CoreNodeRouter::routedCall(&mut router, previousNodeId, request).await
    }
    async fn routedWatchSnapshot(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        let mut router = self.clone();
        CoreNodeRouter::routedWatchSnapshot(&mut router, previousNodeId, request).await
    }
    async fn routedWatch(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let mut router = self.clone();
        CoreNodeRouter::routedWatch(&mut router, previousNodeId, request).await
    }
    async fn routedOpenPush(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let mut router = self.clone();
        CoreNodeRouter::routedOpenPush(&mut router, previousNodeId, request).await
    }
    fn spaceChannelScope(&self, peerNodeId: &str) -> Result<Option<String>, CoreLinkError> {
        CoreNodeRouter::spaceChannelScope(self, peerNodeId)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeServices::{
        DiscoveredPeer, PairedPeer, PairingPrompt, PeerEndpoint, PeerTransport, PendingPairing,
    };
    use crate::SpaceRuntime::SpaceRuntime;
    use operit_host_api::HostManager::{
        defaultHostRuntimeTaskSchedulerHost, setDefaultHostRuntimeTaskSchedulerHost,
    };
    use operit_host_api::{
        FileEntry, FileExistence, FileInfo, FileSystemHost, FindFilesRequest, GrepCodeRequest,
        GrepCodeResult, HostEnvironmentDescriptor, HostError, HostResult, HostRuntimeAsyncTask,
        HostRuntimeTask, HostRuntimeTaskSchedulerHost, HostSecretStore, RuntimeSqliteConnection,
        RuntimeSqliteHost, RuntimeSqliteTransaction, RuntimeStorageEntry, RuntimeStorageHost,
        SqliteRow, SqliteValue,
    };
    use operit_link::{
        CoreEventKind, CorePushRequest, CoreStream, CoreStreamSource, CORE_INTERNAL_TARGET,
    };
    use operit_model::ChatMessage::ChatMessage;
    use operit_model::ChatTurnOptions::ChatTurnOptions;
    use operit_model::InputProcessingState::InputProcessingState;
    use operit_model::PromptFunctionType::PromptFunctionType;
    use operit_rslink_runtime::CoreStreamPool;
    use operit_runtime::core::chat::ChatRuntimeHolder::ChatRuntimeHolder;
    use operit_runtime::core::chat::ChatRuntimeSlot::ChatRuntimeSlot;
    use operit_runtime::services::core::MessageProcessingDelegate::coreResponseStreamSource;
    use operit_runtime::services::ChatServiceCore::ChatState;
    use operit_store::CoreNodeIdentityStore::CoreNodeIdentityStore;
    use operit_store::PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow};
    use operit_util::stream::HotStream::mutable_shared_stream;
    use operit_util::stream::RevisableTextStream::DelegatingRevisableSharedTextStream;
    use operit_util::MarkdownRenderStream::MarkdownStreamEvent;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Mutex as StdMutex, Once, OnceLock};
    use std::time::Duration;

    mod peer_fixture { include!("router_peer_fixture.rs"); }
    use peer_fixture::{TestRouteTarget, TestPeerService, installTestPeer};

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

        /// Reads one bounded range from an in-memory runtime storage file.
        #[allow(non_snake_case)]
        fn readBytesRange(&self, path: &str, offset: u64, length: usize) -> HostResult<Vec<u8>> {
            let content = self.readBytes(path)?;
            let start = usize::try_from(offset)
                .map_err(|_| HostError::new("runtime storage offset does not fit usize"))?;
            if start >= content.len() {
                return Ok(Vec::new());
            }
            let end = start
                .checked_add(length)
                .ok_or_else(|| HostError::new("runtime storage byte range overflows usize"))?
                .min(content.len());
            Ok(content[start..end].to_vec())
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
        spaceRuntime: Arc<dyn CoreLinkSharedClient + Send + Sync>,
        bindingStore: Option<CoreNodeBindingStore>,
    }

    #[async_trait(?Send)]
    impl CoreLinkSharedClient for TestSpaceRuntimeSharedClient {
        /// Executes one local annotation-addressed call through SpaceRuntime.
        async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
            if request.target == "core/server.application"
                && request.methodName == "syncApplyImmediateBindingOperation"
            {
                let result = (|| {
                    let CoreValue::Map(args) = &request.args else {
                        return Err(CoreLinkError::internal("Missing binding arguments"));
                    };
                    let operation = operit_link::fromCoreValue(
                        args.get("operation").cloned().ok_or_else(|| CoreLinkError::internal("Missing binding operation"))?,
                    ).map_err(|error| CoreLinkError::internal(error.to_string()))?;
                    self.bindingStore.as_ref().ok_or_else(|| CoreLinkError::internal("Missing binding store"))?
                        .applyImmediateOperation(&operation).map_err(CoreLinkError::internal)?;
                    operit_link::toCoreValue(serde_json::json!({"applied": true}))
                        .map_err(|error| CoreLinkError::internal(error.to_string()))
                })();
                return CoreCallResponse { requestId: request.requestId, result };
            }
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
        reads: std::sync::atomic::AtomicUsize,
    }

    impl TestBindingRuntime {
        /// Creates a binding runtime with one chat binding.
        fn new(key: &str, nodeId: String) -> Self {
            Self {
                reads: std::sync::atomic::AtomicUsize::new(0),
                binding: StdMutex::new(CoreNodeBindingRecord {
                    key: key.to_string(),
                    nodeId,
                    generation: 1,
                }),
            }
        }

        /// Selects a new owner for the test Binding and advances its generation.
        #[allow(non_snake_case)]
        fn setNodeId(&self, nodeId: String) {
            let mut binding = self
                .binding
                .lock()
                .expect("test binding mutex must not be poisoned");
            if binding.nodeId != nodeId {
                binding.nodeId = nodeId;
                binding.generation += 1;
            }
        }
    }

    impl CoreNodeBindingRuntime for TestBindingRuntime {
        /// Returns the single binding record owned by this test runtime.
        fn binding(&self, key: &str) -> Result<CoreNodeBindingRecord, CoreLinkError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
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
    }

    /// Creates a local runtime shell for tests that route every exercised request remotely.
    #[allow(non_snake_case)]
    fn testLocalRuntime(storage: Arc<dyn RuntimeStorageHost>) -> CoreNodeLocalRuntime {
        testLocalRuntimeWithHolder(storage).0
    }

    #[tokio::test]
    async fn device_space_watch_tracks_remote_membership_without_reopening() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        use crate::RuntimeRemoteLinkService::RuntimeRemoteLinkService;
        use operit_store::CoreSpaceStore::CoreSpaceDeviceProfile;
        installTestRuntimeScheduler();
        let storage: Arc<dyn RuntimeStorageHost> = Arc::new(TestRuntimeStorageHost::default());
        let store = CoreSpaceStore::new(storage.clone());
        let initial = store.initialize().unwrap();
        let localId = initial.members[0].clone();
        let peerId = "overview-watch-peer".to_string();
        store
            .importDeviceProfiles(
                [localId.clone(), peerId.clone()]
                    .into_iter()
                    .map(|nodeId| CoreSpaceDeviceProfile {
                        displayName: nodeId.clone(),
                        nodeId,
                        userName: String::new(),
                        platform: "test".to_string(),
                        model: "test".to_string(),
                        coreVersion: None,
                        updatedAt: 1,
                    })
                    .collect(),
            )
            .unwrap();
        let runtime = testLocalRuntime(storage.clone());
        let router = CoreNodeRouter::new(runtime.clone());
        let peers = TestPeerService::new(localId.clone(), peerId.clone(), Arc::new(TestClientEndpoint));
        peers.close();
        router.installNodeServices(NodeServices::new(peers)).unwrap();
        let service = RuntimeRemoteLinkService::newWithRouter(runtime, router);
        let watch = service.deviceSpaceSnapshotFlow().unwrap();
        assert_eq!(watch.value().space.members.len(), 1);
        // Simulate the inbound Access handler using a separate store handle.
        CoreSpaceStore::new(storage)
            .adopt(CoreSpace {
                members: vec![localId, peerId],
                spaceRevision: initial.spaceRevision + 1,
                ..initial
            })
            .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while watch.value().space.members.len() != 2 {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(watch.value().topology.devices.len(), 2);
        store.rename("renamed-from-peer".to_string()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while watch.value().space.spaceName != "renamed-from-peer" {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
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
                bindingStore: None,
            });
        (
            CoreNodeLocalRuntime::new(
                sharedClient.clone(),
                sharedClient,
                storage,
                Arc::new(|_schema| None),
                Arc::new(|_runtime| Ok(())),
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

    /// Node-local dispatch accepts a Link service without constructing chat/application state.
    #[tokio::test]
    async fn local_space_dispatch_uses_injected_link_service() {
        struct DeviceService;
        #[async_trait(?Send)]
        impl CoreLinkSharedClient for DeviceService {
            async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
                CoreCallResponse::ok(request.requestId, request.args)
            }
            async fn watchSnapshot(
                &self,
                request: CoreWatchRequest,
            ) -> Result<CoreEvent, CoreLinkError> {
                Ok(CoreEvent {
                    requestId: Some(request.requestId),
                    target: request.target,
                    propertyName: request.propertyName,
                    kind: CoreEventKind::Snapshot,
                    value: request.args,
                })
            }
            async fn watch(
                &self,
                request: CoreWatchRequest,
            ) -> Result<CoreEventStream, CoreLinkError> {
                let (sender, receiver) = CoreEventStream::channel();
                sender.send(self.watchSnapshot(request).await?).unwrap();
                Ok(receiver)
            }
        }
        let client = Arc::new(DeviceService);
        let runtime = CoreNodeLocalRuntime::new(
            client.clone(),
            client.clone(),
            Arc::new(TestRuntimeStorageHost::default()),
            Arc::new(|_| None),
            Arc::new(|_| panic!("local dispatch must not install tool runtime")),
            Arc::new(|_| panic!("Space dispatch must not use application push")),
            client,
        );
        let value = CoreValue::String("device-state".into());
        let response = runtime
            .callSpace(CoreCallRequest::new(
                "device-call",
                "device",
                "read",
                value.clone(),
            ))
            .await;
        assert_eq!(response.requestId.0, "device-call");
        assert_eq!(response.result.unwrap(), value);
        let request = CoreWatchRequest::new("device-watch", "device", "state", value.clone());
        let snapshot = runtime.watchSpaceSnapshot(request.clone()).await.unwrap();
        assert_eq!(snapshot.requestId.unwrap().0, "device-watch");
        assert_eq!(snapshot.value, value);
        let mut stream = runtime.watchSpace(request).await.unwrap();
        assert_eq!(stream.recv().await.unwrap().value, value);
        assert!(stream.recv().await.is_none());
    }

    /// Selects local sources without rewriting the persisted remote owner while offline.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn offline_binding_uses_local_source_without_changing_owner() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let router = testCoreNodeRouter("offline-client", "offline-owner", "offline-chat");
        let peers = TestPeerService::new(router.localNodeId(), "offline-owner".into(), Arc::new(TestClientEndpoint));
        peers.close();
        router.installNodeServices(NodeServices::new(peers)).unwrap();
        router.spaceStore.setDirectPeers(Vec::new()).unwrap();
        let args = CoreValue::Map(BTreeMap::from([
            ("chatId".to_string(), CoreValue::String("offline-chat".to_string())),
        ]));
        assert!(!router.nodeIsReachable("offline-owner").unwrap());
        let status = router.bindingRouteStatus("offline-chat".into()).unwrap();
        assert_eq!(status.ownerNodeId, "offline-owner");
        assert!(!status.ownerReachable);
        assert!(status.activeIsLocal);
        assert_eq!(router.bindingRouteNodeId("offline-chat").unwrap(), "offline-client");
        assert_eq!(router.bindingStore.bindingNodeId("offline-chat").unwrap(), "offline-owner");
        for method in ["chatMessagesFlow", "chatStateFlow"] {
            assert!(!router.shouldRouteWatch(method, &args).unwrap());
            assert!(router.shouldUseLocalWatchSource(method, &args).unwrap());
        }
    }

    #[tokio::test]
    async fn core_node_route_state_reports_direct_peers_reachable() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let mut router = testCoreNodeRouter("route-state-core", "route-state-peer", "route-state-chat");
        router.spaceStore.admitRemoteMember(
            "route-state-ghost".into(), "Ghost".into(), "test".into(), "core".into(), "1".into(),
        ).unwrap();
        router.networkControlStore.admitMember("route-state-ghost".into()).unwrap();
        let peers = TestPeerService::new(
            "route-state-core".into(),
            "route-state-peer".into(),
            Arc::new(TestClientEndpoint),
        );
        router.installNodeServices(NodeServices::new(peers)).unwrap();
        let runtime = CoreNodeToolRouteRuntime {
            peerServices: router.peerServices.clone(),
            localNodeId: router.localNodeId(),
            spaceStore: router.spaceStore.clone(),
            networkControlStore: router.networkControlStore.clone(),
        };
        let state = runtime.coreNodeRouteState().unwrap();
        assert_eq!(state.currentNodeId, "route-state-core");
        let status = |nodeId: &str| {
            state
                .nodes
                .iter()
                .find(|node| node.nodeId == nodeId)
                .cloned()
                .expect("every member must appear in the route state")
        };
        assert!(status("route-state-core").reachable);
        // A directly paired peer resolves a single-hop plan; it used to be folded
        // into the multi-hop-only relay metadata and misreported as unreachable.
        let direct = status("route-state-peer");
        assert!(direct.reachable, "a directly paired peer must be reported reachable");
        assert_eq!(direct.relayHops, None, "a direct peer is not relayed");
        let ghost = status("route-state-ghost");
        assert!(!ghost.reachable, "a member without any route must stay unreachable");
        assert_eq!(ghost.relayHops, None);
    }

    #[tokio::test]
    async fn peer_sync_keeps_pairing_space_and_management_boundaries() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let mut router = testCoreNodeRouter("sync-local", "sync-peer", "sync-binding");
        let peers = TestPeerService::new("sync-local".into(), "sync-peer".into(), Arc::new(TestClientEndpoint));
        router.installNodeServices(NodeServices::new(peers)).unwrap();
        let space = router.spaceStore.space().unwrap();
        let request = RoutedCoreRequest {
            spaceId: space.spaceId,
            originNodeId: "sync-peer".into(),
            targetNodeId: "sync-local".into(),
            ttl: 0,
            routeKind: RoutedCoreRequestKind::Target,
            payload: PeerSyncMethod::CoreVersion.request("sync-test".into(), CoreValue::Null),
        };
        assert!(router.validatePeerSyncRoute("sync-peer", &request).unwrap());
        assert!(router.routedCall("unpaired".into(), request.clone()).await.result.is_err());
        let mut wrongSpace = request.clone();
        wrongSpace.spaceId = "other-space".into();
        assert!(router.routedCall("sync-peer".into(), wrongSpace).await.result.is_err());
        let mut management = request.clone();
        management.payload.target = "core/server.application".into();
        assert!(router.routedCall("sync-peer".into(), management).await.result.is_err());
        let mut wrongMethod = request.clone();
        wrongMethod.payload.methodName = "startPairing".into();
        assert!(router.routedCall("sync-peer".into(), wrongMethod).await.result.is_err());
        router.networkControlStore.disconnectNode("sync-peer".into()).unwrap();
        assert!(router.routedCall("sync-peer".into(), request).await.result.is_err());
    }
    /// A handoff must use the admitted peer sync surface, never the local-only application target.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn route_handoff_installs_binding_through_peer_sync() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let sourceId = "handoff-source";
        let targetId = "handoff-target";
        let chatId = "handoff-chat";
        let space = CoreSpace {
            spaceId: "handoff-space".into(),
            spaceName: "Handoff test".into(),
            spaceRevision: 2,
            members: vec![sourceId.into(), targetId.into()],
        };
        let (source, _) = testCoreNodeRouterInJoinedSpace(
            sourceId, targetId, chatId, sourceId, space.clone(),
        );
        let (mut target, _) = testCoreNodeRouterInJoinedSpace(
            targetId, sourceId, chatId, sourceId, space,
        );
        let targetBindingStore = CoreNodeBindingStore::new(target.localCore.runtimeStorageHost()).unwrap();
        let targetRuntime = Arc::make_mut(&mut target.localCore);
        targetRuntime.targetForSchema = Arc::new(|schema: &str| {
            if schema == "application" { Some("core/server.application") } else { None }
        });
        targetRuntime.applicationClient = Arc::new(TestSpaceRuntimeSharedClient {
            spaceRuntime: targetRuntime.spaceRuntime.clone(),
            bindingStore: Some(targetBindingStore.clone()),
        });
        let sourceBindingStore = CoreNodeBindingStore::new(source.localCore.runtimeStorageHost()).unwrap();
        sourceBindingStore.create(chatId, sourceId).unwrap();
        let commit = sourceBindingStore.compareAndSet(chatId, sourceId, targetId).unwrap();
        let peer = installTestPeer(&source, targetId.into(), TestCoreNodeRouterEndpoint::new(target)).unwrap();
        let service = RuntimeRemoteLinkService::newWithRouter((*source.localCore).clone(), source.clone());

        assert!(targetBindingStore.bindingOptional(chatId).unwrap().is_none());

        service.installRouteBindingOnTarget(targetId, commit.operation.clone()).await
            .expect("handoff must install the committed binding through the authenticated sync protocol");
        assert_eq!(targetBindingStore.binding(chatId).unwrap(), commit.binding);
        service.installRouteBindingOnTarget(sourceId, commit.operation).await
            .expect("local binding installation must remain a no-op");
        peer.close();
    }

    #[tokio::test]
    async fn non_storage_members_cannot_enter_replication_or_blob_receivers() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let mut router = testCoreNodeRouter("storage-local", "storage-peer", "storage-binding");
        router.installNodeServices(NodeServices::new(TestPeerService::new(
            "storage-local".into(), "storage-peer".into(), Arc::new(TestClientEndpoint),
        ))).unwrap();
        let request = RoutedCoreRequest {
            spaceId: router.spaceStore.space().unwrap().spaceId,
            originNodeId: "storage-peer".into(), targetNodeId: "storage-local".into(),
            ttl: 0, routeKind: RoutedCoreRequestKind::Target,
            payload: PeerSyncMethod::SyncClock.request("storage-denied".into(), CoreValue::Null),
        };
        let before = router.localCore.runtimeStorageHost().list("runtime/").unwrap();
        for method in [PeerSyncMethod::SyncClock, PeerSyncMethod::SyncOperationsSince,
            PeerSyncMethod::SyncApplyOperations, PeerSyncMethod::SyncBlobExists, PeerSyncMethod::SyncReadBlobChunk] {
            let mut call = request.clone();
            call.payload = method.request("storage-denied".into(),
                operit_link::toCoreValue(serde_json::json!({"operations":[]})).unwrap());
            assert_eq!(router.routedCall("storage-peer".into(), call).await.result.unwrap_err().code,
                "SPACE_STORAGE_PERMISSION_DENIED");
        }
        let push = RoutedCoreRequest { spaceId: request.spaceId.clone(), originNodeId: request.originNodeId.clone(),
            targetNodeId: request.targetNodeId.clone(), ttl: 0, routeKind: RoutedCoreRequestKind::Target,
            payload: CorePushRequest::new("blob-denied", NODE_SYNC_TARGET, "syncReceiveBlob") };
        let error = match router.routedOpenPush("storage-peer".into(), push).await {
            Ok(_) => panic!("runner must not open a blob receiver"), Err(error) => error,
        };
        assert_eq!(error.code, "SPACE_STORAGE_PERMISSION_DENIED");
        let after = router.localCore.runtimeStorageHost().list("runtime/").unwrap();
        assert_eq!(before.iter().map(|entry| (&entry.path, entry.size)).collect::<Vec<_>>(),
            after.iter().map(|entry| (&entry.path, entry.size)).collect::<Vec<_>>());
        // Same-space authority exchange is independent of a storage identity.
        let mut control = request.clone();
        control.payload = CoreCallRequest::new("control-allowed", crate::NodeSpaceService::NODE_SPACE_CONTROL_TARGET,
            "exchange", operit_link::toCoreValue(Vec::<operit_store::SyncOperationStore::SyncOperation>::new()).unwrap());
        assert!(router.routedCall("storage-peer".into(), control).await.result.is_ok());
        router.networkControlStore.setIdentity(operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
            nodeId: "storage-peer".into(), roleId: "storage".into(),
        }).unwrap();
        assert!(router.requirePeerStorageProviders("storage-peer", "storage-local").is_ok());
        router.networkControlStore.clearIdentity("storage-peer".into()).unwrap();
        assert_eq!(router.routedCall("storage-peer".into(), request).await.result.unwrap_err().code,
            "SPACE_STORAGE_PERMISSION_DENIED");
    }

    struct CountingSyncPush(Arc<AtomicUsize>);
    #[async_trait]
    impl CoreLinkPushSession for CountingSyncPush {
        async fn send(&mut self, _value: CoreValue) -> Result<(), CoreLinkError> {
            self.0.fetch_add(1, Ordering::SeqCst); Ok(())
        }
        async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
            self.0.fetch_add(1, Ordering::SeqCst); Ok(())
        }
    }

    #[tokio::test]
    async fn storage_revocation_stops_an_already_open_blob_stream_before_commit() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let router = testCoreNodeRouter("stream-local", "stream-peer", "stream-binding");
        router.networkControlStore.setIdentity(operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
            nodeId: "stream-peer".into(), roleId: "storage".into(),
        }).unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let mut stream = router.guardPeerSyncPush("stream-peer".into(), "stream-local".into(),
            router.spaceStore.space().unwrap().spaceId, Box::new(CountingSyncPush(count.clone())));
        stream.send(CoreValue::Null).await.unwrap();
        router.networkControlStore.clearIdentity("stream-peer".into()).unwrap();
        assert_eq!(stream.send(CoreValue::Null).await.unwrap_err().code, "SPACE_STORAGE_PERMISSION_DENIED");
        assert_eq!(stream.close().await.unwrap_err().code, "SPACE_STORAGE_PERMISSION_DENIED");
        assert_eq!(count.load(Ordering::SeqCst), 1, "no chunk or final commit reaches persistence after revocation");
    }

    #[tokio::test]
    async fn persistence_worker_skips_non_storage_members_and_endpoint_engines() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let router = testCoreNodeRouter("worker-local", "worker-peer", "worker-binding");
        let endpoint = TestRoutedCallEndpoint::newPolicyOnly();
        let link = installTestPeer(&router, "worker-peer".into(), endpoint.clone()).unwrap();
        let worker = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
            router.localCore.clone(), router.clone(), router.spaceStore.clone(),
        );
        let before = router.localCore.runtimeStorageHost().readBytes("runtime/sync/clocks.json").unwrap();
        assert!(!worker.replicaBootstrapRequired(&router.spaceStore.space().unwrap().spaceId).unwrap(),
            "a creator must not discard its own Space's data");
        assert!(!worker.storageReplicationAllowed("worker-peer").unwrap());
        worker.synchronizeReachablePeer("worker-peer".into(), 512, true).await.unwrap();
        assert_eq!(endpoint.callCount.load(Ordering::SeqCst), 1);
        assert_eq!(router.localCore.runtimeStorageHost().readBytes("runtime/sync/clocks.json").unwrap(), before);
        router.networkControlStore.setIdentity(operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
            nodeId: "worker-peer".into(), roleId: "storage".into(),
        }).unwrap();
        assert!(router.requirePeerStorageProviders("worker-local", "worker-peer").is_ok());
        let mut profile = router.spaceStore.deviceProfiles().unwrap().remove("worker-peer").unwrap();
        profile.coreVersion = None; profile.updatedAt += 1;
        router.spaceStore.importDeviceProfiles(vec![profile.clone()]).unwrap();
        let before = router.localCore.runtimeStorageHost().readBytes("runtime/sync/clocks.json").unwrap();
        // A lightweight endpoint has no Core store even with a broad user role.
        assert!(!worker.storageReplicationAllowed("worker-peer").unwrap());
        worker.synchronizeReachablePeer("worker-peer".into(), 512, true).await.unwrap();
        assert_eq!(endpoint.methodNames(), vec!["exchange", "exchange"]);
        assert!(!router.localCore.runtimeStorageHost().exists("runtime/link_access/storage_replica.preferences.json").unwrap());
        assert_eq!(router.localCore.runtimeStorageHost().readBytes("runtime/sync/clocks.json").unwrap(), before,
            "skipped initial sync must not reset/advance replication clocks");
        profile.coreVersion = Some("test-core-version".into()); profile.updatedAt += 1;
        router.spaceStore.importDeviceProfiles(vec![profile]).unwrap();
        assert!(worker.storageReplicationAllowed("worker-peer").unwrap());
        link.close();
    }

    #[tokio::test]
    async fn deferred_storage_grant_still_requires_a_first_replica_bootstrap() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let creator = testCoreNodeRouter("bootstrap-creator", "bootstrap-joined", "bootstrap-binding");
        let joined = testCoreNodeRouter("bootstrap-joined", "bootstrap-creator", "bootstrap-binding");
        let mut adopted = creator.spaceStore.space().unwrap();
        adopted.spaceRevision = adopted.spaceRevision.max(joined.spaceStore.space().unwrap().spaceRevision) + 1;
        creator.spaceStore.adopt(adopted.clone()).unwrap();
        joined.spaceStore.adopt(adopted).unwrap();
        joined.networkControlStore.projectControlOperations(&creator.networkControlStore.currentSpaceOperations().unwrap()).unwrap();
        let worker = crate::SpacePersistenceSyncService::SpacePersistenceSyncService::new(
            joined.localCore.clone(), joined.clone(), joined.spaceStore.clone(),
        );
        let spaceId = joined.spaceStore.space().unwrap().spaceId;
        assert!(worker.replicaBootstrapRequired(&spaceId).unwrap());
        let marker = operit_store::PreferencesDataStore::CoreNodeStateStore::newWithStorage(
            joined.localCore.runtimeStorageHost(), "runtime/link_access/storage_replica.preferences.json");
        let mut preferences = operit_store::PreferencesDataStore::emptyPreferences();
        preferences.set(&operit_store::PreferencesDataStore::stringPreferencesKey("spaceId"), spaceId.clone());
        marker.replace(preferences).unwrap();
        assert!(!worker.replicaBootstrapRequired(&spaceId).unwrap());
        assert!(worker.replicaBootstrapRequired("new-space-without-a-replica").unwrap());
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
        let networkControlStore = NetworkControlStore::new(storage.clone()).unwrap();
        spaceStore.writeLocalDeviceProfile("Test".into(), "test".into(), "core".into(), "1".into()).unwrap();
        networkControlStore.initializeCurrentSpace().unwrap();
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
        let networkControlStore = NetworkControlStore::new(storage.clone())
            .expect("test network control store must initialize");
        spaceStore.admitRemoteMember(targetNodeId.into(), "Target".into(), "test".into(), "core".into(), "1".into()).unwrap();
        networkControlStore.admitMember(targetNodeId.to_string()).unwrap();
        networkControlStore.setIdentity(operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
            nodeId: targetNodeId.to_string(), roleId: "runner".into(),
        }).unwrap();
        let bindingStore: Arc<dyn CoreNodeBindingRuntime> = Arc::new(TestBindingRuntime::new(
            bindingKey,
            targetNodeId.to_string(),
        ));
        CoreNodeRouter {
            peerServices: Arc::new(OnceLock::new()),
            localCore: Arc::new(testLocalRuntime(storage)),
            bindingStore,
            localNodeId: localNodeId.to_string(),
            spaceStore,
            networkControlStore,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn edge_binding_ingress_resolves_to_existing_space_route() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let mut router = testCoreNodeRouter("edge-ingress-core", "edge-executor", "edge-chat");
        router.spaceStore.admitRemoteMember(
            "edge-client".into(), "Edge".into(), "test".into(), "edge".into(), "1".into(),
        ).unwrap();
        let target = TestRoutedCallEndpoint::new();
        let link = installTestPeer(&router, "edge-executor".into(), target.clone()).unwrap();
        let request = RoutedCoreRequest {
            spaceId: router.spaceStore.space().unwrap().spaceId,
            originNodeId: "edge-client".into(),
            targetNodeId: router.localNodeId(),
            ttl: 3,
            routeKind: RoutedCoreRequestKind::SpaceBinding,
            payload: CoreCallRequest::new("edge-send", CORE_INTERNAL_TARGET,
                "sendUserMessage", CoreValue::Map(BTreeMap::from([
                    ("chatIdOverride".into(), CoreValue::String("edge-chat".into())),
                ]))),
        };
        let response = router.routedCall("edge-client".into(), request.clone()).await;
        assert!(response.result.is_ok(), "{:?}", response.result);
        assert_eq!(target.callCount.load(Ordering::SeqCst), 1);
        // Clearing resets the target to the default user identity, which
        // still carries the runtime.execute capability this route needs, so
        // the call keeps succeeding instead of losing its local UI access.
        router
            .networkControlStore
            .clearIdentity("edge-executor".into())
            .unwrap();
        let retried = router.routedCall("edge-client".into(), request).await;
        assert!(retried.result.is_ok(), "{:?}", retried.result);
        assert_eq!(target.callCount.load(Ordering::SeqCst), 2);
        link.close();
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
        let mut singleton = joinedSpace.clone();
        singleton.members = vec![localNodeId.to_string()];
        spaceStore.adopt(singleton).unwrap();
        spaceStore.writeLocalDeviceProfile("Test".into(), "test".into(), "core".into(), "1".into()).unwrap();
        let networkControlStore = NetworkControlStore::new(storage.clone()).unwrap();
        networkControlStore.initializeCurrentSpace().unwrap();
        let members = joinedSpace.members.clone();
        spaceStore
            .adopt(joinedSpace)
            .expect("test joined Space must be adopted");
        for member in members.into_iter().filter(|member| member != localNodeId) {
            spaceStore.admitRemoteMember(member.clone(), "Target".into(), "test".into(), "core".into(), "1".into()).unwrap();
            networkControlStore.admitMember(member.clone()).unwrap();
            networkControlStore.setIdentity(operit_store::NetworkControlStore::NetworkControlIdentityAssignment {
                nodeId: member.clone(),
                roleId: if member == bindingNodeId { "runner".into() } else { "user".into() },
            }).unwrap();
        }
        spaceStore
            .setDirectPeers(vec![peerNodeId.to_string()])
            .expect("test joined Space topology must contain the direct peer");
        let networkControlStore = NetworkControlStore::new(storage.clone())
            .expect("test network control store must initialize");
        let (localRuntime, holder) = testLocalRuntimeWithHolder(storage);
        let bindingStore = Arc::new(TestBindingRuntime::new(
            bindingKey,
            bindingNodeId.to_string(),
        ));
        let bindingRuntime: Arc<dyn CoreNodeBindingRuntime> = bindingStore.clone();
        (
            CoreNodeRouter {
                peerServices: Arc::new(OnceLock::new()),
                localCore: Arc::new(localRuntime),
                bindingStore: bindingRuntime,
                localNodeId: localNodeId.to_string(),
                spaceStore,
                networkControlStore,
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
        /// Reads the test Host's process-local monotonic timer clock.
        fn monotonicTimeMillis(&self) -> HostResult<u64> {
            static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
            Ok(START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64)
        }


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

        /// Executes cooperative work on the host-owned asynchronous executor.
        fn scheduleHostRuntimeCooperativeAsyncTask(
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
        chatFlowOpened: AtomicBool,
    }

    impl TestSpaceEndpoint {
        /// Creates an empty test Space endpoint.
        fn new() -> Arc<Self> {
            Arc::new(Self {
                messages: mutableStateFlow(Vec::new()),
                streamPool: Arc::new(CoreStreamPool::new()),
                embeddedStreamOpened: AtomicBool::new(false),
                chatFlowOpened: AtomicBool::new(false),
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
            self.chatFlowOpened.store(true, Ordering::SeqCst);
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

    /// Captures annotation-routed calls without running any chat business logic.
    struct TestRoutedCallEndpoint {
        callCount: AtomicUsize,
        methods: StdMutex<Vec<String>>,
        policyOnly: bool,
    }

    impl TestRoutedCallEndpoint {
        /// Creates an endpoint that accepts SpaceRoute calls and records their method names.
        fn new() -> Arc<Self> {
            Arc::new(Self {
                callCount: AtomicUsize::new(0),
                methods: StdMutex::new(Vec::new()),
                policyOnly: false,
            })
        }

        fn newPolicyOnly() -> Arc<Self> {
            Arc::new(Self { callCount: AtomicUsize::new(0), methods: StdMutex::new(Vec::new()), policyOnly: true })
        }

        /// Returns every routed method name received by this endpoint.
        fn methodNames(&self) -> Vec<String> {
            self.methods
                .lock()
                .expect("test routed call methods mutex must not be poisoned")
                .clone()
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
    impl TestRouteTarget for TestRealChatSpaceEndpoint {








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
            if payload.target == CORE_STREAM_TARGET {
                self.embeddedStreamOpened.store(true, Ordering::SeqCst);
            } else {
                assert_eq!(payload.target, CORE_INTERNAL_TARGET);
                assert_eq!(payload.propertyName, "chatMessagesFlow");
            }
            self.spaceRuntime.watch(payload).await
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
    impl TestRouteTarget for TestSpaceEndpoint {








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
            if payload.target == CORE_STREAM_TARGET {
                return self.openEmbeddedStream(payload);
            }
            assert_eq!(payload.target, CORE_INTERNAL_TARGET);
            assert_eq!(payload.propertyName, "chatMessagesFlow");
            self.openChatMessagesFlow(payload)
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

    #[async_trait]
    impl TestRouteTarget for TestRoutedCallEndpoint {








        /// Records one annotation-routed Space call and returns a unit response.
        #[allow(non_snake_case)]
        async fn routedCall(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreCallRequest>,
        ) -> CoreCallResponse {
            if self.policyOnly {
                assert_eq!(request.routeKind, RoutedCoreRequestKind::Target);
                assert_eq!(request.payload.target, crate::NodeSpaceService::NODE_SPACE_CONTROL_TARGET);
                assert_eq!(request.payload.methodName, "exchange", "no business RPC is allowed");
                self.callCount.fetch_add(1, Ordering::SeqCst);
                self.methods.lock().unwrap().push(request.payload.methodName);
                return CoreCallResponse::ok(request.payload.requestId,
                    operit_link::toCoreValue(Vec::<operit_store::SyncOperationStore::SyncOperation>::new()).unwrap());
            }
            assert_eq!(request.routeKind, RoutedCoreRequestKind::SpaceRoute);
            assert_eq!(
                request.payload.target,
                CORE_INTERNAL_TARGET
            );
            self.callCount.fetch_add(1, Ordering::SeqCst);
            self.methods
                .lock()
                .expect("test routed call methods mutex must not be poisoned")
                .push(request.payload.methodName.clone());
            CoreCallResponse::ok(request.payload.requestId, CoreValue::Null)
        }

        /// Rejects routed snapshots because this endpoint only accepts routed Space calls.
        #[allow(non_snake_case)]
        async fn routedWatchSnapshot(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEvent, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.payload.registryKey()))
        }

        /// Rejects routed watches because this endpoint only accepts routed Space calls.
        #[allow(non_snake_case)]
        async fn routedWatch(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CoreWatchRequest>,
        ) -> Result<CoreEventStream, CoreLinkError> {
            Err(CoreLinkError::watchNotFound(&request.payload.registryKey()))
        }

        /// Rejects routed push streams because this endpoint only accepts routed Space calls.
        #[allow(non_snake_case)]
        async fn routedOpenPush(
            &self,
            _previousNodeId: String,
            request: RoutedCoreRequest<CorePushRequest>,
        ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
            Err(CoreLinkError::new(
                "UNEXPECTED_TEST_PUSH",
                format!(
                    "routed push is not part of this route wrapper test: {}",
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
    impl TestRouteTarget for TestCoreNodeRouterEndpoint {
        fn installServices(&self, services: NodeServices) -> Result<(), String> {
            self.router.installNodeServices(services)
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
    impl TestRouteTarget for TestClientEndpoint {








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
        peers: Arc<TestPeerService>,
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
            let peers = self.peers.clone();
            Box::pin(async move {
                peers.watch(&targetNodeId.clone(), RoutedCoreRequest {
                    spaceId,
                    originNodeId: localNodeId.clone(),
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
                                target: request.target.clone(),
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value: CoreValue::String(text),
                            });
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId),
                                target: request.target.clone(),
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
                                xml: None,
                            };
                            let value = operit_link::toCoreValue(event)
                                .expect("Markdown stream event must encode");
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId.clone()),
                                target: request.target.clone(),
                                propertyName: request.propertyName.clone(),
                                kind: CoreEventKind::Changed,
                                value,
                            });
                            let _ = sender.send(CoreEvent {
                                requestId: Some(request.requestId),
                                target: request.target.clone(),
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
        events: Vec<MarkdownStreamEvent>,
    ) -> Arc<CoreStreamSource> {
        Arc::new(CoreStreamSource::new(move |request| {
            let (sender, receiver) = CoreEventStream::channel();
            let events = events.clone();
            let requestId = request.requestId.clone();
            let target = request.target.clone();
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
                                    target: target.clone(),
                                    propertyName: propertyName.clone(),
                                    kind: CoreEventKind::Changed,
                                    value,
                                });
                            }
                            let _ = sender.send(CoreEvent {
                                requestId: Some(requestId),
                                target,
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

    /// Waits until the real routed chat message Flow contains a specific visible message.
    #[allow(non_snake_case)]
    async fn waitForRoutedChatText(flow: &StateFlow<Vec<ChatMessage>>, expectedText: &str) {
        for _ in 0..100 {
            let messages = flow.value();
            if messages
                .iter()
                .any(|message| message.displayText() == expectedText)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("real routed chat message Flow did not receive text: {expectedText}");
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

    /// Idle routed watches must not continually re-read bindings/topology from storage.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn binding_watch_is_idle_until_routing_evidence_changes() {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let local = "idle-watch-client";
        let remote = "idle-watch-owner";
        let key = "idle-watch-chat";
        let (router, _holder, binding) = testCoreNodeRouterInJoinedSpaceWithBindingRuntime(
            local, remote, key, remote,
            CoreSpace {
                spaceId: "idle-watch-space".into(),
                spaceName: "Idle watch".into(),
                spaceRevision: 2,
                members: vec![local.into(), remote.into()],
            },
        );
        let source = TestSpaceEndpoint::new();
        let peer = installTestPeer(&router, remote.into(), source).unwrap();
        let request = CoreWatchRequest::new(
            "idle-watch-request", CORE_INTERNAL_TARGET, "chatMessagesFlow",
            CoreValue::Map(BTreeMap::from([("chatId".into(), CoreValue::String(key.into()))])),
        );
        let mut watch = router.watchBindingFlow(key.into(), request, local.into(), None).unwrap();
        let first = tokio::time::timeout(Duration::from_secs(5), watch.recv()).await.unwrap().unwrap();
        assert_eq!(first.kind, CoreEventKind::Snapshot);
        // Let queued setup notifications drain before measuring a silent window.
        tokio::time::sleep(Duration::from_millis(100)).await;
        let reads = binding.reads.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(binding.reads.load(Ordering::SeqCst), reads,
            "an unchanged routed watch must wait for events, not poll storage");

        // A persistent mutation must still rebind an otherwise silent stream.
        binding.setNodeId(local.into());
        router.spaceStore.writeLocalDeviceProfile(local.into(), "test".into(), "test".into(), "1".into()).unwrap();
        let next = tokio::time::timeout(Duration::from_secs(5), watch.recv()).await.unwrap().unwrap();
        assert_eq!(next.kind, CoreEventKind::Changed);
        assert!(binding.reads.load(Ordering::SeqCst) > reads);
        drop(watch);
        peer.close();
    }

    /// A failed open must not become a permanently completed embedded response.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn embedded_binding_watch_recovers_after_initial_open_failure() {
        assertEmbeddedWatchRecovery(true, false).await;
    }

    /// A transport EOF is not the producer's explicit Completed event.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn embedded_binding_watch_recovers_after_transport_closes() {
        assertEmbeddedWatchRecovery(false, false).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn embedded_binding_watch_recovers_when_peer_change_precedes_eof() {
        assertEmbeddedWatchRecovery(false, true).await;
    }

    async fn assertEmbeddedWatchRecovery(failFirstOpen: bool, peerChangeBeforeEof: bool) {
        let _guard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let local = "embedded-recovery-client";
        let remote = "embedded-recovery-owner";
        let chatId = "embedded-recovery-chat";
        let streamId = "embedded-recovery-stream";
        let router = testCoreNodeRouter(local, remote, chatId);
        let endpoint = TestSpaceEndpoint::new();
        let opens = Arc::new(AtomicUsize::new(0));
        let (attempts, mut attempted) = tokio::sync::mpsc::unbounded_channel();
        let (segments, mut opened) = tokio::sync::mpsc::unbounded_channel();
        endpoint.streamPool.adoptAll(vec![operit_link::CoreStreamAttachment {
            streamId: streamId.into(),
            source: Arc::new(CoreStreamSource::new({
                let opens = opens.clone();
                move |request| {
                    let attempt = opens.fetch_add(1, Ordering::SeqCst);
                    attempts.send(attempt).unwrap();
                    if failFirstOpen && attempt == 0 {
                        return Err(CoreLinkError::new("CORE_NODE_UNREACHABLE", "simulated opening race"));
                    }
                    let (sender, receiver) = CoreEventStream::channel();
                    segments.send((request, sender)).unwrap();
                    Ok(receiver)
                }
            })),
        }]);
        let peer = TestPeerService::new(local.into(), remote.into(), endpoint.clone());
        router.installNodeServices(NodeServices::new(peer.clone())).unwrap();
        let request = CoreWatchRequest::new(
            "embedded-recovery-watch", CORE_STREAM_TARGET, "openCoreStream",
            CoreValue::Map(BTreeMap::from([
                ("streamId".into(), CoreValue::String(streamId.into())),
                (CORE_ROUTE_STREAM_SOURCE_MODE_ARGUMENT.into(), CoreValue::String("watch".into())),
                (CORE_ROUTE_STREAM_SOURCE_METHOD_ARGUMENT.into(), CoreValue::String("chatMessagesFlow".into())),
                (CORE_ROUTE_STREAM_SOURCE_ARGS_ARGUMENT.into(), CoreValue::Map(BTreeMap::from([
                    ("chatId".into(), CoreValue::String(chatId.into())),
                ]))),
            ])),
        );
        let mut watch = router.watchSpace(request).await.expect("logical watch must survive an opening race");
        tokio::time::timeout(Duration::from_secs(5), attempted.recv()).await.unwrap().unwrap();
        if !failFirstOpen {
            let (request, sender) = tokio::time::timeout(Duration::from_secs(5), opened.recv()).await.unwrap().unwrap();
            sender.send(CoreEvent {
                requestId: Some(request.requestId), target: request.target,
                propertyName: request.propertyName, kind: CoreEventKind::Changed,
                value: CoreValue::String("before disconnect".into()),
            }).unwrap();
            let event = tokio::time::timeout(Duration::from_secs(5), watch.recv()).await.unwrap().unwrap();
            assert_eq!(event.value, CoreValue::String("before disconnect".into()));
            if peerChangeBeforeEof {
                peer.attach(endpoint.clone());
                // The router's biased select consumes peer evidence before
                // forwarding this event; close only after it was forwarded.
                sender.send(CoreEvent {
                    requestId: None, target: CORE_STREAM_TARGET.into(),
                    propertyName: "openCoreStream".into(), kind: CoreEventKind::Changed,
                    value: CoreValue::String("still connected".into()),
                }).unwrap();
                let event = tokio::time::timeout(Duration::from_secs(5), watch.recv()).await.unwrap().unwrap();
                assert_eq!(event.value, CoreValue::String("still connected".into()));
            }
            drop(sender); // Physical connection closes without a producer Completed event.
        }
        // The same peer can replace a connection without changing the persisted owner.
        if !peerChangeBeforeEof {
            peer.attach(endpoint);
        }
        let (request, sender) = tokio::time::timeout(Duration::from_secs(5), opened.recv())
            .await.expect("embedded watch must reopen after peer recovery").unwrap();
        sender.send(CoreEvent {
            requestId: Some(request.requestId.clone()), target: request.target.clone(),
            propertyName: request.propertyName.clone(), kind: CoreEventKind::Changed,
            value: CoreValue::String("after reconnect".into()),
        }).unwrap();
        let event = tokio::time::timeout(Duration::from_secs(5), watch.recv()).await.unwrap().unwrap();
        assert_eq!(event.value, CoreValue::String("after reconnect".into()));
        sender.send(CoreEvent {
            requestId: Some(request.requestId), target: request.target,
            propertyName: request.propertyName, kind: CoreEventKind::Completed,
            value: CoreValue::Null,
        }).unwrap();
        let event = tokio::time::timeout(Duration::from_secs(5), watch.recv()).await.unwrap().unwrap();
        assert_eq!(event.kind, CoreEventKind::Completed);
        assert!(tokio::time::timeout(Duration::from_secs(5), watch.recv()).await.unwrap().is_none());
        assert_eq!(opens.load(Ordering::SeqCst), 2, "normal completion must not reopen the producer");
        peer.close();
    }

    /// Verifies an async annotation wrapper routes public Rust calls to the selected remote Core.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rust_internal_annotated_call_routes_when_binding_owner_is_remote() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-call-wrapper-client".to_string();
        let targetNodeId = "core-node-call-wrapper-source".to_string();
        let chatId = "chat-call-wrapper-route-a".to_string();
        let joinedSpace = CoreSpace {
            spaceId: "space-call-wrapper-route-test".to_string(),
            spaceName: "Route Call Wrapper Test Space".to_string(),
            spaceRevision: 2,
            members: vec![localNodeId.clone(), targetNodeId.clone()],
        };
        let (localRouter, localHolder, _localBinding) =
            testCoreNodeRouterInJoinedSpaceWithBindingRuntime(
                &localNodeId,
                &targetNodeId,
                &chatId,
                &targetNodeId,
                joinedSpace,
            );
        let target = TestRoutedCallEndpoint::new();
        let peerHandle = installTestPeer(&localRouter, targetNodeId.clone(), target.clone())
        .expect("in-memory PeerLink must connect route wrapper call endpoints");
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(localRouter));

        {
            let mut holder = localHolder.lock().await;
            holder
                .getCore(ChatRuntimeSlot::MAIN)
                .sendUserMessage(
                    PromptFunctionType::CHAT,
                    None,
                    Some(chatId),
                    "route wrapper send probe".to_string(),
                    None,
                    None,
                    None,
                    Vec::new(),
                    None,
                    ChatTurnOptions::default(),
                )
                .await;
        }

        assert_eq!(target.callCount.load(Ordering::SeqCst), 1);
        assert_eq!(target.methodNames(), vec!["sendUserMessage".to_string()]);
        peerHandle.close();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn routed_space_flow_embedded_stream_opens_through_peer_link_pool() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-client".to_string();
        let targetNodeId = "core-node-source".to_string();
        let spaceId = "space-route-test".to_string();
        let source = TestSpaceEndpoint::new();
        let peerHandle = TestPeerService::new(localNodeId.clone(), targetNodeId.clone(), source.clone());
        let routeRuntime = Arc::new(TestPeerRouteRuntime {
            peers: peerHandle.clone(),
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
                CORE_STREAM_TARGET,
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
        let peerHandle = installTestPeer(&router, targetNodeId.clone(), source.clone())
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
                CORE_STREAM_TARGET,
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
        let peerHandle = installTestPeer(&router, targetNodeId.clone(), source.clone())
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
                CORE_STREAM_TARGET,
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
    async fn routed_real_chat_messages_flow_embedded_stream_replays_history_and_receives_live_chunks() {
        let _globalGuard = routeTestGlobalLock().lock().await;
        installTestRuntimeScheduler();
        let localNodeId = "core-node-real-chat-live-client".to_string();
        let targetNodeId = "core-node-real-chat-live-source".to_string();
        let chatId = "chat-real-flow-live".to_string();
        let router = testCoreNodeRouter(&localNodeId, &targetNodeId, &chatId);
        let source = TestRealChatSpaceEndpoint::new();
        let peerHandle = installTestPeer(&router, targetNodeId.clone(), source.clone())
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
        liveStream.emit_chunk("already streaming before joining".to_string());
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
                CORE_STREAM_TARGET,
                "openCoreStream",
                stream.descriptor.args.clone(),
            ))
            .expect("real live embedded ChatMessage stream must reopen through CoreNodeRouter");
        let reset = receiveEvent(&mut openedStream).await;
        assert_eq!(reset.kind, CoreEventKind::Changed);
        let resetEvent: MarkdownStreamEvent =
            operit_link::fromCoreValue(reset.value).expect("Markdown reset event must decode");
        assert_eq!(resetEvent.eventType, "reset");
        let replay = receiveEvent(&mut openedStream).await;
        let replayEvent: MarkdownStreamEvent = operit_link::fromCoreValue(replay.value).unwrap();
        assert_eq!(replayEvent.eventType, "chunk");
        assert_eq!(replayEvent.value.as_deref(), Some("already streaming before joining"));
        // Emit only after replay arrived, so this verifies a live subscription,
        // not just buffered chunks collected after an already-completed turn.
        liveStream.emit_chunk("hello after live open".to_string());
        liveStream.close();
        let event: MarkdownStreamEvent = loop {
            let chunk = tokio::time::timeout(Duration::from_secs(5), openedStream.recv())
                .await.unwrap().unwrap();
            assert_eq!(chunk.kind, CoreEventKind::Changed);
            let event: MarkdownStreamEvent = operit_link::fromCoreValue(chunk.value).unwrap();
            if event.eventType == "chunk" {
                break event;
            }
        };
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
        let peerHandle = installTestPeer(&localRouter, targetNodeId.clone(), TestCoreNodeRouterEndpoint::new(targetRouter))
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
                CORE_STREAM_TARGET,
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
        let peerHandle = installTestPeer(&localRouter, targetNodeId.clone(), TestCoreNodeRouterEndpoint::new(targetRouter))
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
                xml: None,
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
                xml: None,
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
                xml: None,
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
                xml: None,
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
                xml: None,
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
                xml: None,
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
            let source = testStructuredMarkdownStreamSource(expectedEvents.clone());
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
                CORE_STREAM_TARGET,
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
        // 注入服务与连接上线是两件事：先安装离线服务，再测试同一服务的晚到连接。
        let endpoint = TestCoreNodeRouterEndpoint::new(targetRouter);
        let peers = TestPeerService::new(localNodeId.clone(), targetNodeId.clone(), endpoint.clone());
        peers.close();
        localRouter.installNodeServices(NodeServices::new(peers.clone())).unwrap();
        let _routeGuard = installTestCoreRouteRuntime(Arc::new(localRouter.clone()));
        let (routedFlow, ()) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(
                async {
                    let mut holder = localHolder.lock().await;
                    holder.getCore(ChatRuntimeSlot::MAIN).chatStateFlow(chatId.clone()).await
                },
                async {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    peers.attach(endpoint);
                }
            )
        }).await.expect("the first snapshot must survive failed opening attempts");
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
        {
            let mut holder = targetHolder.lock().await;
            holder.getCore(ChatRuntimeSlot::MAIN).messageProcessingDelegate
                .finishChatExecution(chatId.clone(), InputProcessingState::Completed);
        }
        let completed = waitForRoutedChatState(&routedFlow, &InputProcessingState::Completed).await;
        assert!(!completed.isLoading);
        peers.close();
    }

    /// Verifies a routed chat StateFlow opens from the local Core while its selected owner has no live channel.
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
    mod chat_input_menu_tests { use super::*; include!("router_chat_input_menu_tests.rs"); }
    mod device_space_tests { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/device_space/mod.rs")); }

}

#[cfg(test)]
mod edge_event_origin_tests {
    use super::*;
    #[test]
    fn edge_event_sender_is_authenticated_not_a_self_reported_node_or_target_bypass() {
        let args =
            operit_link::toCoreValue(serde_json::json!({"chatId":"chat","nodeId":"edge"})).unwrap();
        let request =
            CoreCallRequest::new("event", CORE_INTERNAL_TARGET, "chatEdgeEvent", args);
        for kind in [
            RoutedCoreRequestKind::SpaceBinding,
            RoutedCoreRequestKind::SpaceRoute,
        ] {
            assert!(validateEdgeEventOrigin(&request, "edge", kind).is_ok());
            assert!(validateEdgeEventOrigin(&request, "impostor", kind).is_err());
        }
        assert!(validateEdgeEventOrigin(&request, "edge", RoutedCoreRequestKind::Target).is_err());
        let unrelated = CoreCallRequest::new("device", "device.status", "read", CoreValue::Null);
        assert!(
            validateEdgeEventOrigin(&unrelated, "edge", RoutedCoreRequestKind::Target).is_ok()
        );
    }
}
