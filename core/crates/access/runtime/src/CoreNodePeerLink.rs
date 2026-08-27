use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
#[cfg(feature = "test-support")]
use std::sync::Weak;
use std::sync::{Arc, Mutex as StdMutex, OnceLock};

use async_trait::async_trait;
use operit_host_api::HostManager::{defaultHostRuntimeTaskSchedulerHost, defaultHttpHost};
use operit_host_api::TimeUtils::currentTimeMillis;
use operit_host_api::{HostRuntimeTaskSchedulerHost, HttpRequestData};
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreEventStream,
    CoreHandoffRequest, CoreHandoffResponse, CoreLinkError, CoreLinkPushSession, CorePushItem,
    CorePushRequest, CoreWatchRequest,
};
use operit_store::CoreSpaceStore::CoreSpaceStore;
use operit_store::SyncOperationStore::SyncOperation;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};
use uuid::Uuid;

use crate::{
    sign, LinkTransportPreference, PairedRemoteSession, RemoteWsConnection, RemoteWsPayload,
    RemoteWsResponse,
};

const PEER_HEARTBEAT_INTERVAL_MS: u64 = 1_000;
const PEER_HEARTBEAT_TIMEOUT_MS: i64 = 4_000;

/// Wraps one existing Core request with the Space route required to reach a CoreNode.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum RoutedCoreRequestKind {
    /// Addresses the destination through the local Core object namespace.
    #[default]
    ObjectId,
    /// Addresses the destination through the annotation-generated Space route namespace.
    SpaceRoute,
}

/// Wraps one existing Core request with the Space route required to reach a CoreNode.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutedCoreRequest<T> {
    pub spaceId: String,
    pub targetNodeId: String,
    pub ttl: u32,
    #[serde(default)]
    pub routeKind: RoutedCoreRequestKind,
    pub payload: T,
}

/// Carries one committed Binding operation directly to its selected CoreNode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreNodeBindingApplyRequest {
    pub bindingKey: String,
    pub nodeId: String,
    pub generation: i64,
    pub operation: SyncOperation,
}

/// Exposes local and routed Core operations to Link Access without fixing a transport direction.
#[async_trait(?Send)]
pub trait CoreNodeLinkClient: operit_link::CoreLinkClient {
    /// Clones the router so transport code can release shared state before network waits.
    #[allow(non_snake_case)]
    fn cloneCoreNodeLinkClient(&self) -> Box<dyn CoreNodeLinkClient + Send>;

    /// Executes a routed call received from one directly paired CoreNode.
    #[allow(non_snake_case)]
    async fn routedCall(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse;

    /// Executes a routed EnhanceAI continuation handoff.
    #[allow(non_snake_case)]
    async fn routedHandoff(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreHandoffRequest>,
    ) -> Result<CoreHandoffResponse, CoreLinkError>;

    /// Reads a routed watch snapshot received from one directly paired CoreNode.
    #[allow(non_snake_case)]
    async fn routedWatchSnapshot(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError>;

    /// Opens a routed watch received from one directly paired CoreNode.
    #[allow(non_snake_case)]
    async fn routedWatch(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError>;

    /// Applies or forwards one committed Binding operation.
    #[allow(non_snake_case)]
    async fn routedBindingApply(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreNodeBindingApplyRequest>,
    ) -> Result<(), CoreLinkError>;

    /// Opens a routed push stream received from one directly paired CoreNode.
    #[allow(non_snake_case)]
    async fn routedOpenPush(
        &mut self,
        previousNodeId: String,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;
}

/// Provides Send-safe access to one CoreNode router from a transport callback.
#[async_trait]
pub trait CoreNodeTransportClient: Send + Sync {
    /// Executes a local Core call.
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse;

    /// Reads a local Core watch snapshot.
    #[allow(non_snake_case)]
    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError>;

    /// Opens a local Core watch.
    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError>;

    /// Opens a local Core push stream.
    #[allow(non_snake_case)]
    async fn openPush(
        &self,
        request: CorePushRequest,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;

    /// Executes a routed call from one adjacent CoreNode.
    #[allow(non_snake_case)]
    async fn routedCall(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse;

    /// Executes a routed EnhanceAI continuation handoff from one adjacent CoreNode.
    #[allow(non_snake_case)]
    async fn routedHandoff(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreHandoffRequest>,
    ) -> Result<CoreHandoffResponse, CoreLinkError>;

    /// Reads a routed watch snapshot from one adjacent CoreNode.
    #[allow(non_snake_case)]
    async fn routedWatchSnapshot(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError>;

    /// Opens a routed watch from one adjacent CoreNode.
    #[allow(non_snake_case)]
    async fn routedWatch(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError>;

    /// Applies one routed committed Binding operation.
    #[allow(non_snake_case)]
    async fn routedBindingApply(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CoreNodeBindingApplyRequest>,
    ) -> Result<(), CoreLinkError>;

    /// Opens a routed push stream from one adjacent CoreNode.
    #[allow(non_snake_case)]
    async fn routedOpenPush(
        &self,
        previousNodeId: String,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError>;
}

/// Opens the long-lived server-to-client half of one authenticated Peer Link.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerChannelOpenEnvelope {
    pub channelId: String,
}

/// Carries one asynchronous request, response, or watch event across a Peer Link.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerFrame {
    pub messageId: String,
    pub payload: PeerFramePayload,
}

/// Carries an ordered batch of Peer Link frames through one HTTP request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerFrameBatch {
    pub frames: Vec<PeerFrame>,
}

/// Defines every message exchanged by the bidirectional CoreNode carrier.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum PeerFramePayload {
    Request(PeerRequest),
    Response(PeerResponse),
    WatchEvent(PeerWatchEvent),
    WatchClosed(PeerWatchClosed),
    Heartbeat,
}

/// Defines one routed Core operation requested by an adjacent CoreNode.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum PeerRequest {
    Call(RoutedCoreRequest<CoreCallRequest>),
    Handoff(RoutedCoreRequest<CoreHandoffRequest>),
    BindingApply(RoutedCoreRequest<CoreNodeBindingApplyRequest>),
    WatchSnapshot(RoutedCoreRequest<CoreWatchRequest>),
    WatchOpen(PeerWatchOpenRequest),
    WatchClose(PeerWatchCloseRequest),
    PushOpen(PeerPushOpenRequest),
    PushItem(CorePushItem),
    PushClose(PeerPushCloseRequest),
}

/// Defines the typed response to one Peer Link request.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum PeerResponse {
    Call(CoreCallResponse),
    Handoff(Result<CoreHandoffResponse, CoreLinkError>),
    WatchSnapshot(Result<CoreEvent, CoreLinkError>),
    Operation(Result<(), CoreLinkError>),
}

/// Opens one routed watch under a stable subscription identifier.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerWatchOpenRequest {
    pub subscriptionId: String,
    pub request: RoutedCoreRequest<CoreWatchRequest>,
}

/// Closes one routed watch opened on the same Peer Link.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerWatchCloseRequest {
    pub subscriptionId: String,
}

/// Delivers one event produced by a routed watch.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerWatchEvent {
    pub subscriptionId: String,
    pub event: CoreEvent,
}

/// Reports that a routed watch source ended without a completion event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerWatchClosed {
    pub subscriptionId: String,
    pub error: CoreLinkError,
}

/// Opens one routed push under a stable push identifier.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerPushOpenRequest {
    pub pushId: String,
    pub request: RoutedCoreRequest<CorePushRequest>,
}

/// Closes one routed push opened on the same Peer Link.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerPushCloseRequest {
    pub pushId: String,
}

/// Sends one encoded Peer Link frame through its concrete carrier direction.
#[async_trait]
pub(crate) trait PeerFrameSender: Send + Sync {
    /// Sends one complete structured frame.
    async fn send(&self, frame: PeerFrame) -> Result<(), String>;

    /// Closes the concrete carrier owned by this frame direction.
    fn close(&self);
}

/// Owns the shared state for one authenticated direct CoreNode connection.
pub(crate) struct PeerConnection {
    localNodeId: String,
    peerNodeId: String,
    channelId: String,
    sender: Arc<dyn PeerFrameSender>,
    core: Arc<dyn CoreNodeTransportClient>,
    pending: StdMutex<BTreeMap<String, oneshot::Sender<PeerResponse>>>,
    outgoingWatches: StdMutex<BTreeMap<String, mpsc::UnboundedSender<CoreEvent>>>,
    incomingWatches: Mutex<BTreeMap<String, oneshot::Sender<()>>>,
    incomingPushes: Mutex<BTreeMap<String, IncomingPushState>>,
    topologyStore: Option<CoreSpaceStore>,
    lastReceivedAt: AtomicI64,
    closed: AtomicBool,
}

/// Stores an incoming routed push while preserving sequence order.
struct IncomingPushState {
    session: Box<dyn CoreLinkPushSession>,
    nextSequence: u64,
}

/// Provides routed Core operations over one active direct Peer Link.
#[derive(Clone)]
pub struct PeerLinkClient {
    connection: Arc<PeerConnection>,
}

static PEER_LINKS: OnceLock<StdMutex<BTreeMap<(String, String), Arc<PeerConnection>>>> =
    OnceLock::new();

static PEER_LINK_CHANGES: OnceLock<broadcast::Sender<()>> = OnceLock::new();

/// Returns the process-wide Peer Link topology change broadcaster.
fn peerLinkChangeSender() -> &'static broadcast::Sender<()> {
    PEER_LINK_CHANGES.get_or_init(|| {
        let (sender, _) = broadcast::channel(256);
        sender
    })
}

/// Subscribes to direct Peer Link topology changes for route lifecycle observers.
#[allow(non_snake_case)]
pub fn subscribePeerLinkChanges() -> broadcast::Receiver<()> {
    peerLinkChangeSender().subscribe()
}

/// Publishes one direct Peer Link topology change without coupling route logic to chat logic.
#[allow(non_snake_case)]
fn publishPeerLinkChange() {
    let _ = peerLinkChangeSender().send(());
}

/// Sends test Peer Link frames directly into the opposite in-memory connection.
#[cfg(feature = "test-support")]
struct InMemoryPeerFrameSender {
    target: StdMutex<Option<Weak<PeerConnection>>>,
}

#[cfg(feature = "test-support")]
impl InMemoryPeerFrameSender {
    /// Creates an unbound in-memory frame sender.
    fn new() -> Self {
        Self {
            target: StdMutex::new(None),
        }
    }

    /// Binds this sender to the opposite Peer Link connection.
    fn bind(&self, target: &Arc<PeerConnection>) -> Result<(), String> {
        let mut slot = self.target.lock().map_err(|error| error.to_string())?;
        if slot.is_some() {
            return Err("in-memory Peer Link sender is already bound".to_string());
        }
        *slot = Some(Arc::downgrade(target));
        Ok(())
    }
}

#[cfg(feature = "test-support")]
#[async_trait]
impl PeerFrameSender for InMemoryPeerFrameSender {
    /// Delivers one frame through the same receive path used by concrete carriers.
    async fn send(&self, frame: PeerFrame) -> Result<(), String> {
        let target = self
            .target
            .lock()
            .map_err(|error| error.to_string())?
            .as_ref()
            .and_then(Weak::upgrade)
            .ok_or_else(|| "in-memory Peer Link target is closed".to_string())?;
        target.receiveFrame(frame).await
    }

    /// Leaves in-memory carrier ownership to its explicit test handle.
    fn close(&self) {}
}

/// Owns both registered directions of one in-memory Peer Link used by integration tests.
#[cfg(feature = "test-support")]
pub struct InMemoryPeerLinkHandle {
    left: Arc<PeerConnection>,
    right: Arc<PeerConnection>,
}

#[cfg(feature = "test-support")]
impl InMemoryPeerLinkHandle {
    /// Closes both directions and removes them from route resolution.
    pub fn close(self) {
        self.left.close("in-memory Peer Link closed".to_string());
        self.right.close("in-memory Peer Link closed".to_string());
    }
}

/// Connects two CoreNode transports through a bidirectional in-memory Peer Link.
#[cfg(feature = "test-support")]
#[allow(non_snake_case)]
pub fn connectInMemoryPeerLinks(
    leftNodeId: String,
    leftCore: Arc<dyn CoreNodeTransportClient>,
    rightNodeId: String,
    rightCore: Arc<dyn CoreNodeTransportClient>,
) -> Result<InMemoryPeerLinkHandle, String> {
    connectInMemoryPeerLinksWithTopology(leftNodeId, leftCore, None, rightNodeId, rightCore, None)
}

/// Connects two in-memory peers and persists active edges through supplied topology stores.
#[cfg(feature = "test-support")]
#[allow(non_snake_case)]
pub fn connectInMemoryPeerLinksWithTopology(
    leftNodeId: String,
    leftCore: Arc<dyn CoreNodeTransportClient>,
    leftTopologyStore: Option<CoreSpaceStore>,
    rightNodeId: String,
    rightCore: Arc<dyn CoreNodeTransportClient>,
    rightTopologyStore: Option<CoreSpaceStore>,
) -> Result<InMemoryPeerLinkHandle, String> {
    if leftNodeId.trim().is_empty() || rightNodeId.trim().is_empty() {
        return Err("in-memory Peer Link node ids must not be empty".to_string());
    }
    if leftNodeId == rightNodeId {
        return Err("in-memory Peer Link requires two distinct CoreNodes".to_string());
    }
    let leftSender = Arc::new(InMemoryPeerFrameSender::new());
    let rightSender = Arc::new(InMemoryPeerFrameSender::new());
    let left = PeerConnection::new(
        leftNodeId.clone(),
        rightNodeId.clone(),
        format!("peer-memory-{}", Uuid::new_v4().simple()),
        leftSender.clone(),
        leftCore,
        leftTopologyStore,
    );
    let right = PeerConnection::new(
        rightNodeId,
        leftNodeId,
        format!("peer-memory-{}", Uuid::new_v4().simple()),
        rightSender.clone(),
        rightCore,
        rightTopologyStore,
    );
    leftSender.bind(&right)?;
    rightSender.bind(&left)?;
    registerPeerLink(left.clone())?;
    match registerPeerLink(right.clone()) {
        Ok(_) => Ok(InMemoryPeerLinkHandle { left, right }),
        Err(error) => {
            left.close("in-memory Peer Link registration failed".to_string());
            Err(error)
        }
    }
}

impl PeerLinkClient {
    /// Returns the adjacent CoreNode reached by this Peer Link.
    #[allow(non_snake_case)]
    pub fn peerNodeId(&self) -> String {
        self.connection.peerNodeId.clone()
    }

    /// Executes one routed call through the adjacent CoreNode.
    #[allow(non_snake_case)]
    pub async fn routedCall(
        &self,
        request: RoutedCoreRequest<CoreCallRequest>,
    ) -> CoreCallResponse {
        let requestId = request.payload.requestId.clone();
        match self.connection.request(PeerRequest::Call(request)).await {
            Ok(PeerResponse::Call(response)) => response,
            Ok(_) => CoreCallResponse::err(
                requestId,
                CoreLinkError::new(
                    "PEER_PROTOCOL_ERROR",
                    "Peer Link returned the wrong response",
                ),
            ),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    /// Executes one EnhanceAI continuation handoff through the adjacent CoreNode.
    #[allow(non_snake_case)]
    pub async fn routedHandoff(
        &self,
        request: RoutedCoreRequest<CoreHandoffRequest>,
    ) -> Result<CoreHandoffResponse, CoreLinkError> {
        match self
            .connection
            .request(PeerRequest::Handoff(request))
            .await?
        {
            PeerResponse::Handoff(result) => result,
            _ => Err(CoreLinkError::new(
                "PEER_PROTOCOL_ERROR",
                "Peer Link returned the wrong response",
            )),
        }
    }

    /// Applies one committed Binding operation through the adjacent CoreNode.
    #[allow(non_snake_case)]
    pub async fn routedBindingApply(
        &self,
        request: RoutedCoreRequest<CoreNodeBindingApplyRequest>,
    ) -> Result<(), CoreLinkError> {
        match self
            .connection
            .request(PeerRequest::BindingApply(request))
            .await?
        {
            PeerResponse::Operation(result) => result,
            _ => Err(CoreLinkError::new(
                "PEER_PROTOCOL_ERROR",
                "Peer Link returned the wrong response",
            )),
        }
    }

    /// Reads one routed watch snapshot through the adjacent CoreNode.
    #[allow(non_snake_case)]
    pub async fn routedWatchSnapshot(
        &self,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEvent, CoreLinkError> {
        match self
            .connection
            .request(PeerRequest::WatchSnapshot(request))
            .await?
        {
            PeerResponse::WatchSnapshot(result) => result,
            _ => Err(CoreLinkError::new(
                "PEER_PROTOCOL_ERROR",
                "Peer Link returned the wrong response",
            )),
        }
    }

    /// Opens one routed watch through the adjacent CoreNode.
    #[allow(non_snake_case)]
    pub async fn routedWatch(
        &self,
        request: RoutedCoreRequest<CoreWatchRequest>,
    ) -> Result<CoreEventStream, CoreLinkError> {
        let subscriptionId = format!("peer-watch-{}", Uuid::new_v4().simple());
        let requestId = request.payload.requestId.0.clone();
        let targetObjectId = request.payload.targetObjectId;
        let propertyName = request.payload.propertyName.clone();
        let (sender, receiver) = mpsc::unbounded_channel();
        self.connection
            .outgoingWatches
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .insert(subscriptionId.clone(), sender);
        let opened = self
            .connection
            .request(PeerRequest::WatchOpen(PeerWatchOpenRequest {
                subscriptionId: subscriptionId.clone(),
                request,
            }))
            .await;
        match opened {
            Ok(PeerResponse::Operation(Ok(()))) => {}
            Ok(PeerResponse::Operation(Err(error))) => {
                self.connection.removeOutgoingWatch(&subscriptionId)?;
                return Err(error);
            }
            Ok(_) => {
                self.connection.removeOutgoingWatch(&subscriptionId)?;
                return Err(CoreLinkError::new(
                    "PEER_PROTOCOL_ERROR",
                    "Peer Link returned the wrong response",
                ));
            }
            Err(error) => {
                self.connection.removeOutgoingWatch(&subscriptionId)?;
                return Err(error);
            }
        }
        operit_util::AppLogger::AppLogger::i(
            "PeerWatchTrace",
            &format!(
                "outgoing_watch_opened local={} peer={} subscription={} requestId={} target={} property={}",
                self.connection.localNodeId,
                self.connection.peerNodeId,
                subscriptionId,
                requestId,
                targetObjectId,
                propertyName
            ),
        );
        let connection = self.connection.clone();
        Ok(CoreEventStream::new(receiver).withOnClose(move || {
            operit_util::AppLogger::AppLogger::i(
                "PeerWatchTrace",
                &format!(
                    "outgoing_watch_drop local={} peer={} subscription={} requestId={} target={} property={}",
                    connection.localNodeId,
                    connection.peerNodeId,
                    subscriptionId,
                    requestId,
                    targetObjectId,
                    propertyName
                ),
            );
            let _ = connection.removeOutgoingWatch(&subscriptionId);
            let connection = connection.clone();
            let _ = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
                "peer-watch-close",
                Box::new(move || {
                    Box::pin(async move {
                        let _ = connection
                            .request(PeerRequest::WatchClose(PeerWatchCloseRequest {
                                subscriptionId,
                            }))
                            .await;
                    })
                }),
            );
        }))
    }

    /// Opens one routed push through the adjacent CoreNode.
    #[allow(non_snake_case)]
    pub async fn routedOpenPush(
        &self,
        request: RoutedCoreRequest<CorePushRequest>,
    ) -> Result<Box<dyn CoreLinkPushSession>, CoreLinkError> {
        let pushId = request.payload.requestId.0.clone();
        match self
            .connection
            .request(PeerRequest::PushOpen(PeerPushOpenRequest {
                pushId: pushId.clone(),
                request,
            }))
            .await?
        {
            PeerResponse::Operation(Ok(())) => Ok(Box::new(PeerLinkPushSession {
                connection: self.connection.clone(),
                pushId,
                nextSequence: 0,
            })),
            PeerResponse::Operation(Err(error)) => Err(error),
            _ => Err(CoreLinkError::new(
                "PEER_PROTOCOL_ERROR",
                "Peer Link returned the wrong response",
            )),
        }
    }
}

/// Owns one caller-side routed push session across a Peer Link.
struct PeerLinkPushSession {
    connection: Arc<PeerConnection>,
    pushId: String,
    nextSequence: u64,
}

#[async_trait]
impl CoreLinkPushSession for PeerLinkPushSession {
    /// Sends one ordered value to the remote routed push.
    async fn send(&mut self, value: operit_link::CoreValue) -> Result<(), CoreLinkError> {
        let sequence = self.nextSequence;
        match self
            .connection
            .request(PeerRequest::PushItem(CorePushItem {
                pushId: self.pushId.clone(),
                sequence,
                args: value,
            }))
            .await?
        {
            PeerResponse::Operation(Ok(())) => {
                self.nextSequence += 1;
                Ok(())
            }
            PeerResponse::Operation(Err(error)) => Err(error),
            _ => Err(CoreLinkError::new(
                "PEER_PROTOCOL_ERROR",
                "Peer Link returned the wrong response",
            )),
        }
    }

    /// Closes the remote routed push.
    async fn close(self: Box<Self>) -> Result<(), CoreLinkError> {
        match self
            .connection
            .request(PeerRequest::PushClose(PeerPushCloseRequest {
                pushId: self.pushId,
            }))
            .await?
        {
            PeerResponse::Operation(result) => result,
            _ => Err(CoreLinkError::new(
                "PEER_PROTOCOL_ERROR",
                "Peer Link returned the wrong response",
            )),
        }
    }
}

impl PeerConnection {
    /// Creates one direct CoreNode connection over an explicit frame sender.
    pub(crate) fn new(
        localNodeId: String,
        peerNodeId: String,
        channelId: String,
        sender: Arc<dyn PeerFrameSender>,
        core: Arc<dyn CoreNodeTransportClient>,
        topologyStore: Option<CoreSpaceStore>,
    ) -> Arc<Self> {
        Arc::new(Self {
            localNodeId,
            peerNodeId,
            channelId,
            sender,
            core,
            pending: StdMutex::new(BTreeMap::new()),
            outgoingWatches: StdMutex::new(BTreeMap::new()),
            incomingWatches: Mutex::new(BTreeMap::new()),
            incomingPushes: Mutex::new(BTreeMap::new()),
            topologyStore,
            lastReceivedAt: AtomicI64::new(currentTimeMillis()),
            closed: AtomicBool::new(false),
        })
    }

    /// Sends one request and waits for the response with the same message identifier.
    async fn request(&self, request: PeerRequest) -> Result<PeerResponse, CoreLinkError> {
        let messageId = format!("peer-message-{}", Uuid::new_v4().simple());
        let (sender, receiver) = oneshot::channel();
        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|error| CoreLinkError::internal(error.to_string()))?;
            if self.closed.load(Ordering::Acquire) {
                return Err(peerClosedError());
            }
            pending.insert(messageId.clone(), sender);
        }
        if let Err(error) = self
            .sender
            .send(PeerFrame {
                messageId: messageId.clone(),
                payload: PeerFramePayload::Request(request),
            })
            .await
        {
            self.pending
                .lock()
                .map_err(|lockError| CoreLinkError::internal(lockError.to_string()))?
                .remove(&messageId);
            return Err(CoreLinkError::new("PEER_SEND_FAILED", error));
        }
        receiver
            .await
            .map_err(|error| CoreLinkError::new("PEER_RESPONSE_CLOSED", error.to_string()))
    }

    /// Handles one complete frame received from the adjacent CoreNode.
    pub(crate) async fn receiveFrame(self: &Arc<Self>, frame: PeerFrame) -> Result<(), String> {
        if self.closed.load(Ordering::Acquire) {
            return Err("Peer Link is closed".to_string());
        }
        self.lastReceivedAt
            .store(currentTimeMillis(), Ordering::Release);
        match frame.payload {
            PeerFramePayload::Response(response) => {
                let sender = self
                    .pending
                    .lock()
                    .map_err(|error| error.to_string())?
                    .remove(&frame.messageId)
                    .ok_or_else(|| {
                        format!("Peer response has no pending request: {}", frame.messageId)
                    })?;
                sender
                    .send(response)
                    .map_err(|_| "Peer response receiver is closed".to_string())
            }
            PeerFramePayload::WatchEvent(event) => self.receiveWatchEvent(event),
            PeerFramePayload::WatchClosed(closed) => self.receiveWatchClosed(closed),
            PeerFramePayload::Request(request) => {
                self.scheduleIncomingRequest(frame.messageId, request)
            }
            PeerFramePayload::Heartbeat => Ok(()),
        }
    }

    /// Executes one incoming request without blocking ordered frame delivery.
    fn scheduleIncomingRequest(
        self: &Arc<Self>,
        messageId: String,
        request: PeerRequest,
    ) -> Result<(), String> {
        let connection = self.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "peer-request-dispatch",
                Box::new(move || {
                    Box::pin(async move {
                        let response = connection.dispatchRequest(request).await;
                        if let Err(error) = connection
                            .sender
                            .send(PeerFrame {
                                messageId,
                                payload: PeerFramePayload::Response(response),
                            })
                            .await
                        {
                            connection.close(format!("Peer response send failed: {error}"));
                        }
                    })
                }),
            )
            .map_err(|error| error.to_string())
    }

    /// Dispatches one incoming request through the local CoreNode router.
    async fn dispatchRequest(self: &Arc<Self>, request: PeerRequest) -> PeerResponse {
        match request {
            PeerRequest::Call(request) => {
                PeerResponse::Call(self.core.routedCall(self.peerNodeId.clone(), request).await)
            }
            PeerRequest::Handoff(request) => PeerResponse::Handoff(
                self.core
                    .routedHandoff(self.peerNodeId.clone(), request)
                    .await,
            ),
            PeerRequest::BindingApply(request) => PeerResponse::Operation(
                self.core
                    .routedBindingApply(self.peerNodeId.clone(), request)
                    .await,
            ),
            PeerRequest::WatchSnapshot(request) => PeerResponse::WatchSnapshot(
                self.core
                    .routedWatchSnapshot(self.peerNodeId.clone(), request)
                    .await,
            ),
            PeerRequest::WatchOpen(request) => {
                PeerResponse::Operation(self.openIncomingWatch(request).await)
            }
            PeerRequest::WatchClose(request) => {
                PeerResponse::Operation(self.closeIncomingWatch(request).await)
            }
            PeerRequest::PushOpen(request) => {
                PeerResponse::Operation(self.openIncomingPush(request).await)
            }
            PeerRequest::PushItem(item) => {
                PeerResponse::Operation(self.sendIncomingPushItem(item).await)
            }
            PeerRequest::PushClose(request) => {
                PeerResponse::Operation(self.closeIncomingPush(request).await)
            }
        }
    }

    /// Opens and pumps one incoming routed watch back to its requesting peer.
    async fn openIncomingWatch(
        self: &Arc<Self>,
        request: PeerWatchOpenRequest,
    ) -> Result<(), CoreLinkError> {
        let watchRequestId = request.request.payload.requestId.0.clone();
        let watchProperty = request.request.payload.propertyName.clone();
        let mut stream = self
            .core
            .routedWatch(self.peerNodeId.clone(), request.request)
            .await?;
        let (cancelSender, mut cancelReceiver) = oneshot::channel();
        {
            let mut watches = self.incomingWatches.lock().await;
            if self.closed.load(Ordering::Acquire) {
                return Err(peerClosedError());
            }
            if watches.contains_key(&request.subscriptionId) {
                return Err(CoreLinkError::new(
                    "PEER_WATCH_ALREADY_EXISTS",
                    "Peer watch subscription already exists",
                ));
            }
            watches.insert(request.subscriptionId.clone(), cancelSender);
        }
        let connection = self.clone();
        let subscriptionId = request.subscriptionId;
        let taskSubscriptionId = subscriptionId.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "peer-watch-events",
                Box::new(move || {
                    Box::pin(async move {
                        loop {
                            let event = tokio::select! {
                                biased;
                                _ = &mut cancelReceiver => {
                                    operit_util::AppLogger::AppLogger::i(
                                        "PeerWatchTrace",
                                        &format!(
                                            "incoming_watch_cancelled local={} peer={} subscription={} requestId={} property={}",
                                            connection.localNodeId,
                                            connection.peerNodeId,
                                            taskSubscriptionId,
                                            watchRequestId,
                                            watchProperty
                                        ),
                                    );
                                    break
                                },
                                event = stream.recv() => event,
                            };
                            let Some(event) = event else {
                                let error = CoreLinkError::new(
                                    "PEER_WATCH_SOURCE_CLOSED",
                                    "Routed watch source closed before a completion event",
                                );
                                operit_util::AppLogger::AppLogger::w(
                                    "PeerWatchTrace",
                                    &format!(
                                        "incoming_watch_source_closed local={} peer={} subscription={} requestId={} property={}",
                                        connection.localNodeId,
                                        connection.peerNodeId,
                                        taskSubscriptionId,
                                        watchRequestId,
                                        watchProperty
                                    ),
                                );
                                let closeResult = connection
                                    .sender
                                    .send(PeerFrame {
                                        messageId: format!(
                                            "peer-watch-closed-{}",
                                            Uuid::new_v4().simple()
                                        ),
                                        payload: PeerFramePayload::WatchClosed(PeerWatchClosed {
                                            subscriptionId: taskSubscriptionId.clone(),
                                            error,
                                        }),
                                    })
                                    .await;
                                if let Err(sendError) = closeResult {
                                    operit_util::AppLogger::AppLogger::e(
                                        "PeerWatchTrace",
                                        &format!(
                                            "incoming_watch_close_notify_failed local={} peer={} subscription={} requestId={} property={} error={}",
                                            connection.localNodeId,
                                            connection.peerNodeId,
                                            taskSubscriptionId,
                                            watchRequestId,
                                            watchProperty,
                                            sendError
                                        ),
                                    );
                                    connection.close(
                                        "Peer watch closure notification failed".to_string(),
                                    );
                                }
                                break;
                            };
                            let completed = event.kind == CoreEventKind::Completed;
                            let sendResult = connection
                                .sender
                                .send(PeerFrame {
                                    messageId: format!("peer-event-{}", Uuid::new_v4().simple()),
                                    payload: PeerFramePayload::WatchEvent(PeerWatchEvent {
                                        subscriptionId: taskSubscriptionId.clone(),
                                        event,
                                    }),
                                })
                                .await;
                            if let Err(sendError) = sendResult {
                                operit_util::AppLogger::AppLogger::e(
                                    "PeerWatchTrace",
                                    &format!(
                                        "peer_watch_send_failed local={} peer={} subscription={} requestId={} property={} error={}",
                                        connection.localNodeId,
                                        connection.peerNodeId,
                                        taskSubscriptionId,
                                        watchRequestId,
                                        watchProperty,
                                        sendError
                                    ),
                                );
                                break;
                            }
                            if completed {
                                break;
                            }
                        }
                        connection
                            .incomingWatches
                            .lock()
                            .await
                            .remove(&taskSubscriptionId);
                    })
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(())
    }

    /// Closes one incoming routed watch.
    async fn closeIncomingWatch(
        &self,
        request: PeerWatchCloseRequest,
    ) -> Result<(), CoreLinkError> {
        operit_util::AppLogger::AppLogger::i(
            "PeerWatchTrace",
            &format!(
                "incoming_watch_close_request local={} peer={} subscription={}",
                self.localNodeId, self.peerNodeId, request.subscriptionId
            ),
        );
        let sender = self
            .incomingWatches
            .lock()
            .await
            .remove(&request.subscriptionId)
            .ok_or_else(|| {
                CoreLinkError::new("PEER_WATCH_NOT_FOUND", "Peer watch subscription not found")
            })?;
        let _ = sender.send(());
        Ok(())
    }

    /// Opens one incoming routed push stream.
    async fn openIncomingPush(&self, request: PeerPushOpenRequest) -> Result<(), CoreLinkError> {
        let session = self
            .core
            .routedOpenPush(self.peerNodeId.clone(), request.request)
            .await?;
        let mut pushes = self.incomingPushes.lock().await;
        if self.closed.load(Ordering::Acquire) {
            drop(pushes);
            session.close().await?;
            return Err(peerClosedError());
        }
        if pushes.contains_key(&request.pushId) {
            drop(pushes);
            session.close().await?;
            return Err(CoreLinkError::new(
                "PEER_PUSH_ALREADY_EXISTS",
                "Peer push stream already exists",
            ));
        }
        pushes.insert(
            request.pushId,
            IncomingPushState {
                session,
                nextSequence: 0,
            },
        );
        Ok(())
    }

    /// Sends one ordered item into an incoming routed push without holding the map lock.
    async fn sendIncomingPushItem(&self, item: CorePushItem) -> Result<(), CoreLinkError> {
        let mut state = self
            .incomingPushes
            .lock()
            .await
            .remove(&item.pushId)
            .ok_or_else(|| {
                CoreLinkError::new("PEER_PUSH_NOT_FOUND", "Peer push stream not found")
            })?;
        if item.sequence != state.nextSequence {
            let expectedSequence = state.nextSequence;
            self.incomingPushes
                .lock()
                .await
                .insert(item.pushId.clone(), state);
            return Err(CoreLinkError::new(
                "PEER_PUSH_SEQUENCE_MISMATCH",
                format!(
                    "Peer push sequence is {}, expected {}",
                    item.sequence, expectedSequence
                ),
            ));
        }
        let pushId = item.pushId;
        if let Err(sendError) = state.session.send(item.args).await {
            state.session.close().await.map_err(|closeError| {
                CoreLinkError::new(
                    "PEER_PUSH_ABORT_FAILED",
                    format!("push send failed: {sendError}; push close failed: {closeError}"),
                )
            })?;
            return Err(sendError);
        }
        state.nextSequence += 1;
        let mut pushes = self.incomingPushes.lock().await;
        if self.closed.load(Ordering::Acquire) {
            drop(pushes);
            state.session.close().await?;
            return Err(peerClosedError());
        }
        pushes.insert(pushId, state);
        Ok(())
    }

    /// Closes one incoming routed push without holding the map lock during completion.
    async fn closeIncomingPush(&self, request: PeerPushCloseRequest) -> Result<(), CoreLinkError> {
        let state = self
            .incomingPushes
            .lock()
            .await
            .remove(&request.pushId)
            .ok_or_else(|| {
                CoreLinkError::new("PEER_PUSH_NOT_FOUND", "Peer push stream not found")
            })?;
        state.session.close().await
    }

    /// Delivers one remote watch event to its local subscriber.
    fn receiveWatchEvent(&self, event: PeerWatchEvent) -> Result<(), String> {
        let completed = event.event.kind == CoreEventKind::Completed;
        let mut watches = self
            .outgoingWatches
            .lock()
            .map_err(|error| error.to_string())?;
        let Some(sender) = watches.get(&event.subscriptionId) else {
            return Ok(());
        };
        if sender.send(event.event).is_err() {
            watches.remove(&event.subscriptionId);
            return Ok(());
        }
        if completed {
            watches.remove(&event.subscriptionId);
        }
        Ok(())
    }

    /// Closes one outgoing watch after its remote routed source ended unexpectedly.
    #[allow(non_snake_case)]
    fn receiveWatchClosed(&self, closed: PeerWatchClosed) -> Result<(), String> {
        self.outgoingWatches
            .lock()
            .map_err(|error| error.to_string())?
            .remove(&closed.subscriptionId);
        operit_util::AppLogger::AppLogger::w(
            "CoreNodePeerLink",
            &format!(
                "Peer watch closed local={} peer={} subscription={} error={}",
                self.localNodeId, self.peerNodeId, closed.subscriptionId, closed.error
            ),
        );
        Ok(())
    }

    /// Removes one local watch subscriber from this connection.
    fn removeOutgoingWatch(&self, subscriptionId: &str) -> Result<(), CoreLinkError> {
        self.outgoingWatches
            .lock()
            .map_err(|error| CoreLinkError::internal(error.to_string()))?
            .remove(subscriptionId);
        Ok(())
    }

    /// Fails all requests and closes every stream owned by this carrier.
    pub(crate) fn close(self: &Arc<Self>, reason: String) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        operit_util::AppLogger::AppLogger::w(
            "PeerWatchTrace",
            &format!(
                "peer_connection_close local={} peer={} channel={} reason={}",
                self.localNodeId, self.peerNodeId, self.channelId, reason
            ),
        );
        self.sender.close();
        let pending = std::mem::take(
            &mut *self
                .pending
                .lock()
                .expect("Peer Link pending map mutex poisoned"),
        );
        drop(pending);
        self.outgoingWatches
            .lock()
            .expect("Peer Link watch map mutex poisoned")
            .clear();
        let remainingPeers =
            unregisterPeerLink(&self.localNodeId, &self.peerNodeId, &self.channelId)
                .expect("Peer Link close must update the active connection registry");
        if let Some(remainingPeers) = remainingPeers {
            if let Some(topologyStore) = &self.topologyStore {
                topologyStore
                    .setDirectPeers(remainingPeers)
                    .expect("Peer Link close must persist active topology withdrawal");
            }
        }
        let connection = self.clone();
        defaultHostRuntimeTaskSchedulerHost()
            .scheduleHostRuntimeAsyncTask(
                "peer-link-close-streams",
                Box::new(move || {
                    Box::pin(async move {
                        let watches = std::mem::take(&mut *connection.incomingWatches.lock().await);
                        for (_, cancel) in watches {
                            let _ = cancel.send(());
                        }
                        let pushes = std::mem::take(&mut *connection.incomingPushes.lock().await);
                        for (_, state) in pushes {
                            state.session.close().await.unwrap_or_else(|error| {
                                panic!("Peer Link push cleanup failed after {reason}: {error}")
                            });
                        }
                    })
                }),
            )
            .expect("Peer Link stream cleanup task must be scheduled");
    }
}

/// Creates the canonical error returned after a Peer Link carrier closes.
fn peerClosedError() -> CoreLinkError {
    CoreLinkError::new("PEER_LINK_CLOSED", "Peer Link is closed")
}

/// Registers one active direct Peer Link for route resolution.
#[allow(non_snake_case)]
pub(crate) fn registerPeerLink(connection: Arc<PeerConnection>) -> Result<PeerLinkClient, String> {
    if connection.closed.load(Ordering::Acquire) {
        return Err("CoreNode Peer Link carrier closed before registration".to_string());
    }
    let key = (
        connection.localNodeId.clone(),
        connection.peerNodeId.clone(),
    );
    let mut links = PEER_LINKS
        .get_or_init(|| StdMutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| error.to_string())?;
    if let Some(existing) = links.get(&key) {
        if existing.channelId != connection.channelId {
            return Err(format!(
                "CoreNode Peer Link is already active: {} -> {}",
                key.0, key.1
            ));
        }
    }
    if let Some(topologyStore) = &connection.topologyStore {
        let mut peerNodeIds = activePeerNodeIdsFromLinks(&links, &connection.localNodeId);
        peerNodeIds.insert(connection.peerNodeId.clone());
        topologyStore.setDirectPeers(peerNodeIds.into_iter().collect())?;
    }
    links.insert(key, connection.clone());
    drop(links);
    publishPeerLinkChange();
    startPeerHeartbeat(connection.clone())?;
    Ok(PeerLinkClient { connection })
}

/// Starts independent heartbeat transmission and expiry tasks for one registered Peer Link.
#[allow(non_snake_case)]
fn startPeerHeartbeat(connection: Arc<PeerConnection>) -> Result<(), String> {
    let watchdogConnection = connection.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "peer-link-heartbeat-watchdog",
            Box::new(move || {
                Box::pin(async move {
                    loop {
                        defaultHostRuntimeTaskSchedulerHost()
                            .waitForHostRuntimeDelay(PEER_HEARTBEAT_INTERVAL_MS)
                            .await;
                        if watchdogConnection.closed.load(Ordering::Acquire) {
                            return;
                        }
                        let elapsed = currentTimeMillis().saturating_sub(
                            watchdogConnection.lastReceivedAt.load(Ordering::Acquire),
                        );
                        if elapsed >= PEER_HEARTBEAT_TIMEOUT_MS {
                            watchdogConnection
                                .close(format!("Peer Link heartbeat expired after {elapsed} ms"));
                            return;
                        }
                    }
                })
            }),
        )
        .map_err(|error| error.to_string())?;

    let senderConnection = connection.clone();
    if let Err(error) = defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
        "peer-link-heartbeat-send",
        Box::new(move || {
            Box::pin(async move {
                loop {
                    defaultHostRuntimeTaskSchedulerHost()
                        .waitForHostRuntimeDelay(PEER_HEARTBEAT_INTERVAL_MS)
                        .await;
                    if senderConnection.closed.load(Ordering::Acquire) {
                        return;
                    }
                    let result = senderConnection
                        .sender
                        .send(PeerFrame {
                            messageId: format!("peer-heartbeat-{}", Uuid::new_v4().simple()),
                            payload: PeerFramePayload::Heartbeat,
                        })
                        .await;
                    if let Err(error) = result {
                        senderConnection.close(format!("Peer Link heartbeat send failed: {error}"));
                        return;
                    }
                }
            })
        }),
    ) {
        connection.close(format!("Peer Link heartbeat task failed: {error}"));
        return Err(error.to_string());
    }
    Ok(())
}

/// Resolves one active direct Peer Link by adjacent CoreNode identity.
#[allow(non_snake_case)]
pub fn peerLink(localNodeId: &str, peerNodeId: &str) -> Result<PeerLinkClient, String> {
    let links = PEER_LINKS
        .get_or_init(|| StdMutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| error.to_string())?;
    let connection = links
        .get(&(localNodeId.to_string(), peerNodeId.to_string()))
        .cloned()
        .ok_or_else(|| format!("CoreNode Peer Link is not active: {peerNodeId}"))?;
    Ok(PeerLinkClient { connection })
}

/// Returns whether one adjacent CoreNode currently has an active Peer Link.
#[allow(non_snake_case)]
pub fn isPeerLinkActive(localNodeId: &str, peerNodeId: &str) -> Result<bool, String> {
    Ok(PEER_LINKS
        .get_or_init(|| StdMutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| error.to_string())?
        .contains_key(&(localNodeId.to_string(), peerNodeId.to_string())))
}

/// Closes the active direct connection to one paired device when it exists.
#[allow(non_snake_case)]
pub fn disconnectPeerLink(localNodeId: &str, peerNodeId: &str) -> Result<(), String> {
    let connection = PEER_LINKS
        .get_or_init(|| StdMutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| error.to_string())?
        .get(&(localNodeId.to_string(), peerNodeId.to_string()))
        .cloned();
    if let Some(connection) = connection {
        connection.close("Paired device connection removed".to_string());
    }
    Ok(())
}

/// Closes one active direct connection at the request of the local device owner.
#[allow(non_snake_case)]
pub fn kickPeerLink(localNodeId: &str, peerNodeId: &str) -> Result<(), String> {
    let connection = PEER_LINKS
        .get_or_init(|| StdMutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| error.to_string())?
        .get(&(localNodeId.to_string(), peerNodeId.to_string()))
        .cloned();
    if let Some(connection) = connection {
        connection.close("Peer Link disconnected by local device owner".to_string());
    }
    Ok(())
}

/// Returns the adjacent CoreNodes currently connected to one local CoreNode.
#[allow(non_snake_case)]
pub fn activePeerNodeIds(localNodeId: &str) -> Result<BTreeSet<String>, String> {
    let links = PEER_LINKS
        .get_or_init(|| StdMutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| error.to_string())?;
    Ok(activePeerNodeIdsFromLinks(&links, localNodeId))
}

/// Collects the active adjacent CoreNodes for one local node from a locked registry.
#[allow(non_snake_case)]
fn activePeerNodeIdsFromLinks(
    links: &BTreeMap<(String, String), Arc<PeerConnection>>,
    localNodeId: &str,
) -> BTreeSet<String> {
    links
        .keys()
        .filter(|(registeredLocalNodeId, _)| registeredLocalNodeId == localNodeId)
        .map(|(_, peerNodeId)| peerNodeId.clone())
        .collect()
}

/// Dispatches one frame received by the server-side HTTP carrier.
#[allow(non_snake_case)]
pub(crate) async fn receivePeerFrame(
    localNodeId: &str,
    peerNodeId: &str,
    frame: PeerFrame,
) -> Result<(), String> {
    let connection = PEER_LINKS
        .get_or_init(|| StdMutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| error.to_string())?
        .get(&(localNodeId.to_string(), peerNodeId.to_string()))
        .cloned()
        .ok_or_else(|| format!("CoreNode Peer Link is not active: {peerNodeId}"))?;
    connection.receiveFrame(frame).await
}

/// Removes one Peer Link only when the closing carrier still owns the registered channel.
#[allow(non_snake_case)]
fn unregisterPeerLink(
    localNodeId: &str,
    peerNodeId: &str,
    channelId: &str,
) -> Result<Option<Vec<String>>, String> {
    let mut links = PEER_LINKS
        .get_or_init(|| StdMutex::new(BTreeMap::new()))
        .lock()
        .map_err(|error| error.to_string())?;
    let key = (localNodeId.to_string(), peerNodeId.to_string());
    if links
        .get(&key)
        .map(|connection| connection.channelId.as_str())
        == Some(channelId)
    {
        links.remove(&key);
        publishPeerLinkChange();
        return Ok(Some(
            activePeerNodeIdsFromLinks(&links, localNodeId)
                .into_iter()
                .collect(),
        ));
    }
    Ok(None)
}

const OUTBOUND_PEER_FRAME_BATCH_MAX_FRAMES: usize = 128;

/// Stores the shared state of one queued outbound HTTP Peer carrier.
struct OutboundPeerFrameBatchState {
    session: PairedRemoteSession,
    streamId: String,
    closed: AtomicBool,
    failure: StdMutex<Option<String>>,
    closeSender: StdMutex<Option<oneshot::Sender<()>>>,
}

/// Queues client-originated frames for ordered HTTP batch delivery.
struct OutboundPeerFrameSender {
    state: Arc<OutboundPeerFrameBatchState>,
    frameSender: mpsc::UnboundedSender<PeerFrame>,
}

#[async_trait]
impl PeerFrameSender for OutboundPeerFrameSender {
    /// Queues one frame without waiting for a separate HTTP request.
    async fn send(&self, frame: PeerFrame) -> Result<(), String> {
        if let Some(error) = self
            .state
            .failure
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
        {
            return Err(error);
        }
        if self.state.closed.load(Ordering::Acquire) {
            return Err("Outbound Peer frame sender is closed".to_string());
        }
        self.frameSender
            .send(frame)
            .map_err(|_| "Outbound Peer frame queue is closed".to_string())
    }

    /// Stops the batch worker and closes the Host-owned response stream.
    fn close(&self) {
        if self.state.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(sender) = self
            .state
            .closeSender
            .lock()
            .expect("Outbound Peer close mutex poisoned")
            .take()
        {
            let _ = sender.send(());
        }
        let _ = defaultHttpHost().closeHttpByteStream(&self.state.streamId);
    }
}

/// Records one terminal error and closes the queued outbound carrier.
fn failOutboundPeerFrameBatch(state: &Arc<OutboundPeerFrameBatchState>, error: String) {
    if let Ok(mut failure) = state.failure.lock() {
        *failure = Some(error.clone());
    }
    state.closed.store(true, Ordering::Release);
    let _ = defaultHttpHost().closeHttpByteStream(&state.streamId);
    operit_util::AppLogger::AppLogger::e(
        "PeerCarrierTrace",
        &format!("outbound_peer_frame_batch_failed error={error}"),
    );
}

/// Drains queued Peer frames into ordered bounded HTTP batches.
async fn runOutboundPeerFrameBatchSender(
    state: Arc<OutboundPeerFrameBatchState>,
    mut receiver: mpsc::UnboundedReceiver<PeerFrame>,
    mut closeReceiver: oneshot::Receiver<()>,
) {
    loop {
        let firstFrame = tokio::select! {
            biased;
            _ = &mut closeReceiver => return,
            frame = receiver.recv() => frame,
        };
        let Some(firstFrame) = firstFrame else {
            return;
        };
        let mut frames = Vec::with_capacity(OUTBOUND_PEER_FRAME_BATCH_MAX_FRAMES);
        frames.push(firstFrame);
        while frames.len() < OUTBOUND_PEER_FRAME_BATCH_MAX_FRAMES {
            match receiver.try_recv() {
                Ok(frame) => frames.push(frame),
                Err(mpsc::error::TryRecvError::Empty)
                | Err(mpsc::error::TryRecvError::Disconnected) => break,
            }
        }
        let body = match operit_link::encodeLink(&PeerFrameBatch { frames }) {
            Ok(body) => body,
            Err(error) => {
                failOutboundPeerFrameBatch(&state, error.to_string());
                return;
            }
        };
        if let Err(error) = state.session.signedRemotePost("peer/channel/frame", body) {
            failOutboundPeerFrameBatch(&state, error);
            return;
        }
    }
}

/// Opens and registers the outbound side of one bidirectional Peer Link.
#[allow(non_snake_case)]
pub async fn openOutboundPeerLink(
    session: PairedRemoteSession,
    core: Arc<dyn CoreNodeTransportClient>,
    topologyStore: CoreSpaceStore,
) -> Result<PeerLinkClient, String> {
    match session.transport {
        LinkTransportPreference::Http => {
            openOutboundPeerLinkHttp(session, core, topologyStore).await
        }
        LinkTransportPreference::WebSocket => {
            openOutboundPeerLinkWebSocket(session, core, topologyStore).await
        }
    }
}

/// Opens and registers the HTTP-carried outbound Peer Link.
#[allow(non_snake_case)]
async fn openOutboundPeerLinkHttp(
    session: PairedRemoteSession,
    core: Arc<dyn CoreNodeTransportClient>,
    topologyStore: CoreSpaceStore,
) -> Result<PeerLinkClient, String> {
    let localNodeId = session.deviceId.clone();
    let peerNodeId = session.coreDeviceId.clone();
    let channelId = format!("peer-channel-{}", Uuid::new_v4().simple());
    let streamId = format!("peer-http-{}", Uuid::new_v4().simple());
    let (batchFrameSender, batchFrameReceiver) = mpsc::unbounded_channel();
    let (batchCloseSender, batchCloseReceiver) = oneshot::channel();
    let batchState = Arc::new(OutboundPeerFrameBatchState {
        session: session.clone(),
        streamId: streamId.clone(),
        closed: AtomicBool::new(false),
        failure: StdMutex::new(None),
        closeSender: StdMutex::new(Some(batchCloseSender)),
    });
    let sender = Arc::new(OutboundPeerFrameSender {
        state: batchState.clone(),
        frameSender: batchFrameSender,
    });
    let senderForWorkerError = sender.clone();
    let connection = PeerConnection::new(
        localNodeId.clone(),
        peerNodeId.clone(),
        channelId.clone(),
        sender,
        core,
        Some(topologyStore),
    );
    let (frameQueueSender, mut frameQueueReceiver) = mpsc::unbounded_channel::<PeerFrame>();
    let frameQueueSender = Arc::new(StdMutex::new(Some(frameQueueSender)));
    let frameDispatchConnection = connection.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "peer-frame-ordered-receive",
            Box::new(move || {
                Box::pin(async move {
                    while let Some(frame) = frameQueueReceiver.recv().await {
                        if let Err(error) = frameDispatchConnection.receiveFrame(frame).await {
                            frameDispatchConnection
                                .close(format!("Peer Link frame dispatch failed: {error}"));
                            return;
                        }
                    }
                })
            }),
        )
        .map_err(|error| error.to_string())?;
    let body = operit_link::encodeLink(&PeerChannelOpenEnvelope {
        channelId: channelId.clone(),
    })
    .map_err(|error| error.to_string())?;
    let signature = sign(&session.sessionSecret, &body);
    let buffer = Arc::new(StdMutex::new(Vec::new()));
    let chunkBuffer = buffer.clone();
    let closedConnection = connection.clone();
    let (openedSender, openedReceiver) = oneshot::channel();
    let openedSender = Arc::new(StdMutex::new(Some(openedSender)));
    let openedSignal = openedSender.clone();
    let closedSignal = openedSender.clone();
    let chunkFrameQueueSender = frameQueueSender.clone();
    let closedFrameQueueSender = frameQueueSender.clone();
    let openedLocalNodeId = localNodeId.clone();
    let openedPeerNodeId = peerNodeId.clone();
    let openedChannelId = channelId.clone();
    let chunkLocalNodeId = localNodeId.clone();
    let chunkPeerNodeId = peerNodeId.clone();
    let chunkChannelId = channelId.clone();
    let closedLocalNodeId = localNodeId.clone();
    let closedPeerNodeId = peerNodeId.clone();
    let closedChannelId = channelId.clone();
    let openResult = defaultHttpHost().openHttpByteStream(
        streamId,
        HttpRequestData {
            url: format!("{}/link/peer/channel/events", session.baseUrl),
            method: "POST".to_string(),
            headers: vec![
                ("x-operit-link-version".to_string(), "3".to_string()),
                ("x-operit-session".to_string(), session.sessionId.clone()),
                ("x-operit-device".to_string(), session.deviceId.clone()),
                ("x-operit-signature".to_string(), signature),
            ],
            body,
            formFields: Vec::new(),
            fileParts: Vec::new(),
            connectTimeoutSeconds: 10,
            readTimeoutSeconds: 0,
            followRedirects: false,
            ignoreSsl: false,
            proxyHost: String::new(),
            proxyPort: 0,
        },
        Arc::new(move || {
            operit_util::AppLogger::AppLogger::i(
                "PeerCarrierTrace",
                &format!(
                    "http_stream_opened local={} peer={} channel={}",
                    openedLocalNodeId, openedPeerNodeId, openedChannelId
                ),
            );
            if let Some(sender) = openedSignal
                .lock()
                .expect("Peer Link open signal lock poisoned")
                .take()
            {
                let _ = sender.send(Ok(()));
            }
        }),
        Arc::new(move |chunk| {
            let frames = decodePeerFrameChunks(&chunkBuffer, chunk)
                .expect("Peer Link frame stream must decode");
            operit_util::AppLogger::AppLogger::i(
                "PeerCarrierTrace",
                &format!(
                    "http_stream_chunk local={} peer={} channel={} frames={}",
                    chunkLocalNodeId,
                    chunkPeerNodeId,
                    chunkChannelId,
                    frames.len()
                ),
            );
            let sender = chunkFrameQueueSender
                .lock()
                .expect("Peer Link frame queue lock poisoned");
            let sender = sender
                .as_ref()
                .expect("Peer Link frame queue must remain open while receiving chunks");
            for frame in frames {
                sender
                    .send(frame)
                    .expect("Peer Link ordered frame receiver must remain active");
            }
        }),
        Arc::new(move |result| {
            closedFrameQueueSender
                .lock()
                .expect("Peer Link frame queue lock poisoned")
                .take();
            let reason = match result {
                Ok(()) => "Peer Link stream closed".to_string(),
                Err(error) => error,
            };
            operit_util::AppLogger::AppLogger::w(
                "PeerCarrierTrace",
                &format!(
                    "http_stream_closed local={} peer={} channel={} reason={}",
                    closedLocalNodeId, closedPeerNodeId, closedChannelId, reason
                ),
            );
            if let Some(sender) = closedSignal
                .lock()
                .expect("Peer Link close signal lock poisoned")
                .take()
            {
                let _ = sender.send(Err(reason.clone()));
            }
            closedConnection.close(reason);
        }),
    );
    if let Err(error) = openResult {
        frameQueueSender
            .lock()
            .map_err(|lockError| lockError.to_string())?
            .take();
        connection.close(error.to_string());
        return Err(error.to_string());
    }
    openedReceiver
        .await
        .map_err(|error| format!("Peer Link open signal closed: {error}"))??;
    let workerState = batchState.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "peer-frame-batch-send",
            Box::new(move || {
                Box::pin(runOutboundPeerFrameBatchSender(
                    workerState,
                    batchFrameReceiver,
                    batchCloseReceiver,
                ))
            }),
        )
        .map_err(|error| {
            senderForWorkerError.close();
            error.to_string()
        })?;
    registerPeerLink(connection)
}

/// Opens and registers the WebSocket-carried outbound Peer Link.
#[allow(non_snake_case)]
async fn openOutboundPeerLinkWebSocket(
    session: PairedRemoteSession,
    core: Arc<dyn CoreNodeTransportClient>,
    topologyStore: CoreSpaceStore,
) -> Result<PeerLinkClient, String> {
    let localNodeId = session.deviceId.clone();
    let peerNodeId = session.coreDeviceId.clone();
    let channelId = format!("peer-channel-{}", Uuid::new_v4().simple());
    let websocket = RemoteWsConnection::open(&session, "peer").await?;
    websocket.sendPayload(RemoteWsPayload::PeerChannelOpen(PeerChannelOpenEnvelope {
        channelId: channelId.clone(),
    }))?;
    match websocket.nextResponse().await? {
        RemoteWsResponse::PeerOpened(openedChannelId) if openedChannelId == channelId => {}
        RemoteWsResponse::Error(error) => return Err(error.to_string()),
        _ => return Err("unexpected WebSocket Peer Link open response".to_string()),
    }
    let sender = Arc::new(OutboundPeerWebSocketSender {
        connection: websocket.clone(),
        channelId: channelId.clone(),
        closed: AtomicBool::new(false),
    });
    let connection = PeerConnection::new(
        localNodeId.clone(),
        peerNodeId.clone(),
        channelId,
        sender.clone(),
        core,
        Some(topologyStore),
    );
    let frameDispatchConnection = connection.clone();
    let frameConnection = websocket.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeAsyncTask(
            "peer-websocket-ordered-receive",
            Box::new(move || {
                Box::pin(async move {
                    loop {
                        let response = match frameConnection.nextResponse().await {
                            Ok(value) => value,
                            Err(error) => {
                                frameDispatchConnection.close(error);
                                return;
                            }
                        };
                        match response {
                            RemoteWsResponse::PeerFrame(frame) => {
                                if let Err(error) =
                                    frameDispatchConnection.receiveFrame(frame).await
                                {
                                    frameDispatchConnection
                                        .close(format!("Peer Link frame dispatch failed: {error}"));
                                    return;
                                }
                            }
                            RemoteWsResponse::PeerClosed(_) | RemoteWsResponse::Error(_) => {
                                frameDispatchConnection
                                    .close("Peer WebSocket carrier closed".to_string());
                                return;
                            }
                            _ => {}
                        }
                    }
                })
            }),
        )
        .map_err(|error| {
            sender.close();
            error.to_string()
        })?;
    registerPeerLink(connection)
}

/// Sends Peer frames directly through one authenticated WebSocket carrier.
struct OutboundPeerWebSocketSender {
    connection: Arc<RemoteWsConnection>,
    channelId: String,
    closed: AtomicBool,
}

#[async_trait]
impl PeerFrameSender for OutboundPeerWebSocketSender {
    /// Sends one ordered Peer frame without creating a per-frame HTTP request.
    async fn send(&self, frame: PeerFrame) -> Result<(), String> {
        if self.closed.load(Ordering::Acquire) {
            return Err("Outbound Peer WebSocket is closed".to_string());
        }
        self.connection.sendPayload(RemoteWsPayload::PeerFrame {
            channelId: self.channelId.clone(),
            frame,
        })
    }

    /// Closes the Peer WebSocket carrier exactly once.
    fn close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let _ = self
            .connection
            .sendPayload(RemoteWsPayload::PeerChannelClose(self.channelId.clone()));
        let _ = self.connection.close();
    }
}

/// Decodes complete length-prefixed Peer Link frames from one HTTP stream chunk.
#[allow(non_snake_case)]
fn decodePeerFrameChunks(
    buffer: &Arc<StdMutex<Vec<u8>>>,
    chunk: Vec<u8>,
) -> Result<Vec<PeerFrame>, String> {
    let mut buffer = buffer.lock().map_err(|error| error.to_string())?;
    buffer.extend_from_slice(&chunk);
    let mut frames = Vec::new();
    while buffer.len() >= 4 {
        let frameLength = u32::from_be_bytes(
            buffer[..4]
                .try_into()
                .expect("Peer Link frame prefix must contain four bytes"),
        ) as usize;
        if buffer.len() < 4 + frameLength {
            break;
        }
        let encoded = buffer.drain(..4 + frameLength).collect::<Vec<_>>();
        frames.push(
            operit_link::decodeLink::<PeerFrame>(&encoded[4..])
                .map_err(|error| error.to_string())?,
        );
    }
    Ok(frames)
}

/// Encodes one Peer Link frame with the stream carrier length prefix.
#[allow(non_snake_case)]
pub(crate) fn encodePeerFrame(frame: &PeerFrame) -> Result<Vec<u8>, String> {
    let payload = operit_link::encodeLink(frame).map_err(|error| error.to_string())?;
    let length = u32::try_from(payload.len())
        .map_err(|_| "Peer Link frame exceeds u32 length".to_string())?;
    let mut encoded = Vec::with_capacity(4 + payload.len());
    encoded.extend_from_slice(&length.to_be_bytes());
    encoded.extend_from_slice(&payload);
    Ok(encoded)
}
