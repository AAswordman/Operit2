#![allow(non_snake_case)]

use async_trait::async_trait;
use operit_link::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventStream, CoreLinkError,
    CoreLinkSharedClient, CoreWatchRequest, LinkFrame, LinkFramePayload,
};
use operit_node_edge::EdgeNode;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

pub mod auth;
pub mod pairing;
pub mod serial;
pub mod serial_codec;
pub mod tcp;

pub use auth::AuthenticatedLinkChannel;
pub use pairing::{
    finishPairAsClient, linkTokenHash, startPairAsClient, EdgePairStartState, EdgePairingAuthority,
    EdgePairingPersistentState, EdgePairingStore, EdgeSession, EDGE_PAIRING_SERVICE_VERSION,
};

/// Abstracts a bidirectional carrier while keeping all operations in Link types.
#[async_trait]
pub trait LinkChannel: Send + Sync {
    async fn send(&self, frame: LinkFrame) -> Result<(), String>;
    async fn receive(&self) -> Result<Option<LinkFrame>, String>;
    async fn close(&self);
}

struct EdgeLinkState {
    channel: Arc<dyn LinkChannel>,
    pending: Mutex<BTreeMap<String, oneshot::Sender<Result<LinkFramePayload, CoreLinkError>>>>,
    watches: Mutex<BTreeMap<String, mpsc::UnboundedSender<CoreEvent>>>,
    connected: AtomicBool,
}

/// A normal Core-side Link client for one lightweight Edge node.
#[derive(Clone)]
pub struct EdgeLinkClient {
    state: Arc<EdgeLinkState>,
}

impl EdgeLinkClient {
    /// Starts a Link client over an already established carrier.
    pub fn new(channel: Arc<dyn LinkChannel>) -> Self {
        let state = Arc::new(EdgeLinkState {
            channel,
            pending: Mutex::new(BTreeMap::new()),
            watches: Mutex::new(BTreeMap::new()),
            connected: AtomicBool::new(true),
        });
        let client = Self { state };
        client.startReceiver();
        client
    }

    /// Runs the carrier receive loop and dispatches responses to Link callers.
    fn startReceiver(&self) {
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            loop {
                let frame = match state.channel.receive().await {
                    Ok(Some(frame)) => frame,
                    Ok(None) => break,
                    Err(error) => {
                        failPending(&state, error).await;
                        break;
                    }
                };
                dispatchIncomingFrame(&state, frame).await;
            }
            state.connected.store(false, Ordering::Release);
            failPending(&state, "Edge Link carrier closed".to_string()).await;
        });
    }

    /// Returns whether the carrier receive loop is still alive.
    pub fn isConnected(&self) -> bool {
        self.state.connected.load(Ordering::Acquire)
    }

    async fn request(&self, payload: LinkFramePayload) -> Result<LinkFramePayload, CoreLinkError> {
        if !self.isConnected() {
            return Err(CoreLinkError::new(
                "LINK_CLOSED",
                "Edge Link carrier is closed",
            ));
        }
        let messageId = format!("edge-link-{}", Uuid::new_v4().simple());
        let (sender, receiver) = oneshot::channel();
        self.state
            .pending
            .lock()
            .await
            .insert(messageId.clone(), sender);
        if let Err(error) = self
            .state
            .channel
            .send(LinkFrame {
                messageId: messageId.clone(),
                payload,
            })
            .await
        {
            self.state.pending.lock().await.remove(&messageId);
            self.state.connected.store(false, Ordering::Release);
            return Err(CoreLinkError::new("LINK_SEND_FAILED", error));
        }
        receiver
            .await
            .map_err(|error| CoreLinkError::new("LINK_RESPONSE_CLOSED", error.to_string()))?
    }

    async fn closeWatch(&self, subscriptionId: String) {
        self.state.watches.lock().await.remove(&subscriptionId);
        let _ = self
            .state
            .channel
            .send(LinkFrame {
                messageId: format!("edge-link-close-{}", Uuid::new_v4().simple()),
                payload: LinkFramePayload::WatchClose { subscriptionId },
            })
            .await;
    }
}

#[async_trait(?Send)]
impl CoreLinkSharedClient for EdgeLinkClient {
    async fn call(&self, request: CoreCallRequest) -> CoreCallResponse {
        let requestId = request.requestId.clone();
        match self.request(LinkFramePayload::Call(request)).await {
            Ok(LinkFramePayload::CallResponse(response)) => response,
            Ok(_) => CoreCallResponse::err(
                requestId,
                CoreLinkError::new("LINK_PROTOCOL_ERROR", "unexpected Edge call response"),
            ),
            Err(error) => CoreCallResponse::err(requestId, error),
        }
    }

    async fn watchSnapshot(&self, request: CoreWatchRequest) -> Result<CoreEvent, CoreLinkError> {
        match self
            .request(LinkFramePayload::WatchSnapshot(request))
            .await?
        {
            LinkFramePayload::WatchSnapshotResponse(result) => result,
            _ => Err(CoreLinkError::new(
                "LINK_PROTOCOL_ERROR",
                "unexpected Edge watch snapshot response",
            )),
        }
    }

    async fn watch(&self, request: CoreWatchRequest) -> Result<CoreEventStream, CoreLinkError> {
        let subscriptionId = request.requestId.0.clone();
        let (sender, receiver) = mpsc::unbounded_channel();
        self.state
            .watches
            .lock()
            .await
            .insert(subscriptionId.clone(), sender);
        if let Err(error) = self
            .request(LinkFramePayload::WatchOpen {
                subscriptionId: subscriptionId.clone(),
                request,
            })
            .await
        {
            self.state.watches.lock().await.remove(&subscriptionId);
            return Err(error);
        }
        let client = self.clone();
        Ok(CoreEventStream::new(receiver).withOnClose(move || {
            tokio::spawn(async move { client.closeWatch(subscriptionId).await });
        }))
    }
}

/// Serves one lightweight Edge node over a standard Link carrier.
pub struct EdgeLinkServer {
    node: Arc<EdgeNode>,
    channel: Arc<dyn LinkChannel>,
    watches: Arc<Mutex<BTreeMap<String, oneshot::Sender<()>>>>,
}

impl EdgeLinkServer {
    pub fn new(node: Arc<EdgeNode>, channel: Arc<dyn LinkChannel>) -> Self {
        Self {
            node,
            channel,
            watches: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Processes Link frames until the carrier closes.
    pub async fn run(self) -> Result<(), String> {
        while let Some(frame) = self.channel.receive().await? {
            self.dispatch(frame).await?;
        }
        Ok(())
    }

    /// Runs the server after the carrier's first frame was consumed while
    /// selecting pairing or authenticated-session resume.
    pub async fn runWithFirstFrame(self, frame: LinkFrame) -> Result<(), String> {
        self.dispatch(frame).await?;
        self.run().await
    }

    async fn dispatch(&self, frame: LinkFrame) -> Result<(), String> {
        match frame.payload {
            LinkFramePayload::Call(request) => {
                self.send(LinkFrame {
                    messageId: frame.messageId,
                    payload: LinkFramePayload::CallResponse(self.node.dispatchCall(request)),
                })
                .await
            }
            LinkFramePayload::WatchSnapshot(request) => {
                self.send(LinkFrame {
                    messageId: frame.messageId,
                    payload: LinkFramePayload::WatchSnapshotResponse(
                        self.node.dispatchWatchSnapshot(request),
                    ),
                })
                .await
            }
            LinkFramePayload::WatchOpen {
                subscriptionId,
                request,
            } => {
                self.openWatch(frame.messageId, subscriptionId, request)
                    .await
            }
            LinkFramePayload::WatchClose { subscriptionId } => {
                if let Some(sender) = self.watches.lock().await.remove(&subscriptionId) {
                    let _ = sender.send(());
                }
                self.send(LinkFrame {
                    messageId: frame.messageId,
                    payload: LinkFramePayload::Operation(Ok(())),
                })
                .await
            }
            LinkFramePayload::Heartbeat { sequence } => {
                self.send(LinkFrame {
                    messageId: frame.messageId,
                    payload: LinkFramePayload::Heartbeat { sequence },
                })
                .await
            }
            LinkFramePayload::PairStart(_)
            | LinkFramePayload::PairStartResponse(_)
            | LinkFramePayload::PairFinish(_)
            | LinkFramePayload::PairFinishResponse(_)
            | LinkFramePayload::Authenticated { .. }
            | LinkFramePayload::Close { .. }
            | LinkFramePayload::CallResponse(_)
            | LinkFramePayload::WatchSnapshotResponse(_)
            | LinkFramePayload::WatchEvent { .. }
            | LinkFramePayload::Operation(_) => Ok(()),
        }
    }

    async fn openWatch(
        &self,
        messageId: String,
        subscriptionId: String,
        request: CoreWatchRequest,
    ) -> Result<(), String> {
        let stream = match self.node.dispatchWatch(request) {
            Ok(stream) => stream,
            Err(error) => {
                return self
                    .send(LinkFrame {
                        messageId,
                        payload: LinkFramePayload::Operation(Err(error)),
                    })
                    .await;
            }
        };
        self.send(LinkFrame {
            messageId,
            payload: LinkFramePayload::Operation(Ok(())),
        })
        .await?;
        let (cancelSender, mut cancelReceiver) = oneshot::channel();
        self.watches
            .lock()
            .await
            .insert(subscriptionId.clone(), cancelSender);
        let channel = Arc::clone(&self.channel);
        tokio::spawn(async move {
            let mut stream = stream;
            loop {
                tokio::select! {
                    event = stream.recv() => {
                        let Some(event) = event else { break; };
                        if channel.send(LinkFrame {
                            messageId: format!("edge-link-event-{}", Uuid::new_v4().simple()),
                            payload: LinkFramePayload::WatchEvent { subscriptionId: subscriptionId.clone(), event },
                        }).await.is_err() { break; }
                    }
                    _ = &mut cancelReceiver => break,
                }
            }
        });
        Ok(())
    }

    async fn send(&self, frame: LinkFrame) -> Result<(), String> {
        self.channel.send(frame).await
    }
}

async fn dispatchIncomingFrame(state: &Arc<EdgeLinkState>, frame: LinkFrame) {
    if let LinkFramePayload::WatchEvent {
        subscriptionId,
        event,
    } = frame.payload.clone()
    {
        if let Some(sender) = state.watches.lock().await.get(&subscriptionId) {
            let _ = sender.send(event);
        }
        return;
    }
    if let Some(sender) = state.pending.lock().await.remove(&frame.messageId) {
        let result = match frame.payload {
            LinkFramePayload::CallResponse(_)
            | LinkFramePayload::WatchSnapshotResponse(_)
            | LinkFramePayload::Operation(_)
            | LinkFramePayload::Heartbeat { .. } => Ok(frame.payload),
            LinkFramePayload::Close { code, message } => Err(CoreLinkError::new(code, message)),
            _ => Err(CoreLinkError::new(
                "LINK_PROTOCOL_ERROR",
                "unexpected frame payload",
            )),
        };
        let _ = sender.send(result);
    }
}

async fn failPending(state: &Arc<EdgeLinkState>, message: String) {
    let error = CoreLinkError::new("LINK_CLOSED", message);
    for (_, sender) in std::mem::take(&mut *state.pending.lock().await) {
        let _ = sender.send(Err(error.clone()));
    }
    state.watches.lock().await.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::{DeviceDigitalOutputRequest, DeviceDigitalOutputState};
    use operit_link::{CoreValue, LinkFrame};
    use operit_node_edge::service::{DeviceIoService, DeviceIoStateStream, EdgeServiceError};
    use std::sync::mpsc;
    use tokio::sync::mpsc as async_mpsc;

    struct MemoryChannel {
        tx: async_mpsc::UnboundedSender<LinkFrame>,
        rx: Mutex<async_mpsc::UnboundedReceiver<LinkFrame>>,
    }

    #[async_trait]
    impl LinkChannel for MemoryChannel {
        async fn send(&self, frame: LinkFrame) -> Result<(), String> {
            self.tx.send(frame).map_err(|error| error.to_string())
        }

        async fn receive(&self) -> Result<Option<LinkFrame>, String> {
            Ok(self.rx.lock().await.recv().await)
        }

        async fn close(&self) {}
    }

    struct TestDevice;

    impl DeviceIoService for TestDevice {
        fn setDigitalOutput(
            &self,
            request: DeviceDigitalOutputRequest,
        ) -> Result<DeviceDigitalOutputState, EdgeServiceError> {
            Ok(DeviceDigitalOutputState {
                pin: request.pin,
                level: request.level,
            })
        }
        fn getDigitalOutput(&self, pin: u8) -> Result<DeviceDigitalOutputState, EdgeServiceError> {
            Ok(DeviceDigitalOutputState { pin, level: false })
        }
        fn watchDigitalOutput(&self, pin: u8) -> Result<DeviceIoStateStream, EdgeServiceError> {
            let (sender, receiver) = mpsc::channel();
            sender
                .send(DeviceDigitalOutputState { pin, level: false })
                .unwrap();
            Ok(DeviceIoStateStream::new(receiver))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn serverDispatchesStandardLinkCall() {
        let (clientTx, serverRx) = async_mpsc::unbounded_channel();
        let (serverTx, clientRx) = async_mpsc::unbounded_channel();
        let clientChannel = Arc::new(MemoryChannel {
            tx: clientTx,
            rx: Mutex::new(clientRx),
        });
        let serverChannel = Arc::new(MemoryChannel {
            tx: serverTx,
            rx: Mutex::new(serverRx),
        });
        let node = Arc::new(EdgeNode::new(Arc::new(TestDevice)));
        let server = EdgeLinkServer::new(node, serverChannel);
        tokio::spawn(async move {
            server.run().await.unwrap();
        });
        let client = EdgeLinkClient::new(clientChannel);
        let response = client
            .call(CoreCallRequest::new(
                "call-1",
                1,
                "getDigitalOutput",
                CoreValue::Map(std::collections::BTreeMap::from([(
                    "pin".to_string(),
                    CoreValue::Unsigned(2),
                )])),
            ))
            .await;
        assert!(response.result.is_ok());
    }
}
