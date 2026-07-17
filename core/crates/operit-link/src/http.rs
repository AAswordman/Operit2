use std::collections::BTreeMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::{Body, Bytes};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::client::CoreLinkTransportClient;
use crate::codec::{decodeLink, encodeLink};
use crate::protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreLinkError, CorePushItem,
    CorePushRequest, CoreWatchRequest,
};

#[derive(Clone)]
pub struct CoreLinkHttpDispatcher {
    state: Arc<CoreLinkHttpState>,
}

struct CoreLinkHttpState {
    core: CoreLinkHttpCore,
    watchChannels: Arc<Mutex<BTreeMap<String, LinkWatchChannel>>>,
    pushStreams: Arc<Mutex<BTreeMap<String, LinkPushState>>>,
}

struct LinkPushState {
    request: CorePushRequest,
    nextSequence: u64,
}

#[derive(Clone)]
enum CoreLinkHttpCore {
    Locked(Arc<Mutex<Box<dyn CoreLinkTransportClient>>>),
}

struct LinkWatchChannel {
    sender: mpsc::UnboundedSender<LinkWatchChannelEvent>,
    subscriptions: BTreeMap<String, JoinHandle<()>>,
}

struct LinkWatchChannelEventStream {
    receiver: mpsc::UnboundedReceiver<LinkWatchChannelEvent>,
    watchChannels: Arc<Mutex<BTreeMap<String, LinkWatchChannel>>>,
    channelId: String,
}

impl futures_util::Stream for LinkWatchChannelEventStream {
    type Item = Result<Bytes, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(context) {
            Poll::Ready(Some(event)) => {
                let payload = encodeLink(&event).expect("LinkWatchChannelEvent must serialize");
                let length = u32::try_from(payload.len())
                    .expect("Link watch event frame exceeds the protocol length limit");
                let mut frame = Vec::with_capacity(4 + payload.len());
                frame.extend_from_slice(&length.to_be_bytes());
                frame.extend_from_slice(&payload);
                Poll::Ready(Some(Ok(Bytes::from(frame))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for LinkWatchChannelEventStream {
    fn drop(&mut self) {
        let watchChannels = self.watchChannels.clone();
        let channelId = self.channelId.clone();
        tokio::spawn(async move {
            if let Some(channel) = watchChannels.lock().await.remove(&channelId) {
                abort_watch_channel(channel);
            }
        });
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkCallEnvelope {
    pub request: CoreCallRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchEnvelope {
    pub request: CoreWatchRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelEnvelope {
    pub channelId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelOpenEnvelope {
    pub channelId: String,
    pub subscriptionId: String,
    pub request: CoreWatchRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelCloseEnvelope {
    pub channelId: String,
    pub subscriptionId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelOpenResponse {
    pub subscriptionId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelCloseResponse {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkWatchChannelEvent {
    pub subscriptionId: String,
    pub event: CoreEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushOpenEnvelope {
    pub pushId: String,
    pub request: CorePushRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushCloseEnvelope {
    pub pushId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushOpenResponse {
    pub pushId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushItemResponse {
    pub pushId: String,
    pub sequence: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkPushCloseResponse {
    pub pushId: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkWsPayload {
    Call(LinkCallEnvelope),
    WatchSnapshot(LinkWatchEnvelope),
    PushOpen(LinkPushOpenEnvelope),
    PushItem(CorePushItem),
    PushClose(LinkPushCloseEnvelope),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkWsResponse {
    Call(CoreCallResponse),
    WatchSnapshot(CoreEvent),
    PushOpened(LinkPushOpenResponse),
    PushAccepted(LinkPushItemResponse),
    PushClosed(LinkPushCloseResponse),
    Error(CoreLinkError),
}

impl CoreLinkHttpDispatcher {
    /// Creates an HTTP/WebSocket dispatcher around a core link client.
    pub fn new(core: impl CoreLinkTransportClient + 'static) -> Self {
        Self {
            state: Arc::new(CoreLinkHttpState {
                core: CoreLinkHttpCore::Locked(Arc::new(Mutex::new(Box::new(core)))),
                watchChannels: Arc::new(Mutex::new(BTreeMap::new())),
                pushStreams: Arc::new(Mutex::new(BTreeMap::new())),
            }),
        }
    }

    /// Handles a MessagePack call request and returns a MessagePack response.
    pub async fn call(&self, body: Bytes) -> Response {
        let envelope = match decodeLink::<LinkCallEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        match encodeLink(self.executeCall(envelope.request).await) {
            Ok(bytes) => binary_response(StatusCode::OK, bytes),
            Err(error) => codec_error(error.to_string()),
        }
    }

    #[allow(non_snake_case)]
    async fn executeCall(&self, request: CoreCallRequest) -> CoreCallResponse {
        let CoreLinkHttpCore::Locked(core) = &self.state.core;
        let mut core = core.lock().await;
        core.call(request).await
    }

    /// Handles a MessagePack watch snapshot request and returns a MessagePack event.
    #[allow(non_snake_case)]
    pub async fn watchSnapshot(&self, body: Bytes) -> Response {
        let envelope = match decodeLink::<LinkWatchEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        match self.executeWatchSnapshot(envelope.request).await {
            Ok(event) => match encodeLink(event) {
                Ok(bytes) => binary_response(StatusCode::OK, bytes),
                Err(error) => codec_error(error.to_string()),
            },
            Err(error) => error_response(StatusCode::BAD_REQUEST, error),
        }
    }

    #[allow(non_snake_case)]
    async fn executeWatchSnapshot(
        &self,
        request: CoreWatchRequest,
    ) -> Result<CoreEvent, CoreLinkError> {
        let CoreLinkHttpCore::Locked(core) = &self.state.core;
        let mut core = core.lock().await;
        core.watchSnapshot(request).await
    }

    /// Drains queued events for an opened watch channel.
    #[allow(non_snake_case)]
    pub async fn watchChannelEvents(&self, body: Bytes) -> Response {
        let envelope = match decodeLink::<LinkWatchChannelEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        self.openWatchChannelEvents(envelope.channelId).await
    }

    /// Opens a watch stream and stores it under a channel identifier.
    #[allow(non_snake_case)]
    pub async fn watchChannelOpen(&self, body: Bytes) -> Response {
        let envelope = match decodeLink::<LinkWatchChannelOpenEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        match self
            .openWatchChannelSubscription(
                envelope.channelId,
                envelope.subscriptionId,
                envelope.request,
            )
            .await
        {
            Ok(response) => encode_response(StatusCode::OK, response),
            Err(error) => error_response(StatusCode::BAD_REQUEST, error),
        }
    }

    /// Closes a previously opened watch channel.
    #[allow(non_snake_case)]
    pub async fn watchChannelClose(&self, body: Bytes) -> Response {
        let envelope = match decodeLink::<LinkWatchChannelCloseEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        self.closeWatchChannelSubscription(&envelope.channelId, &envelope.subscriptionId)
            .await;
        encode_response(StatusCode::OK, LinkWatchChannelCloseResponse {})
    }

    /// Opens a client-owned input stream targeting one core method.
    #[allow(non_snake_case)]
    pub async fn pushOpen(&self, body: Bytes) -> Response {
        let envelope = match decodeLink::<LinkPushOpenEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        match self.openPushStream(envelope.pushId, envelope.request).await {
            Ok(response) => encode_response(StatusCode::OK, response),
            Err(error) => error_response(StatusCode::BAD_REQUEST, error),
        }
    }

    /// Accepts one ordered item for an opened client-owned input stream.
    #[allow(non_snake_case)]
    pub async fn pushItem(&self, body: Bytes) -> Response {
        let item = match decodeLink::<CorePushItem>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        match self.executePushItem(item).await {
            Ok(response) => encode_response(StatusCode::OK, response),
            Err(error) => error_response(StatusCode::BAD_REQUEST, error),
        }
    }

    /// Closes one client-owned input stream.
    #[allow(non_snake_case)]
    pub async fn pushClose(&self, body: Bytes) -> Response {
        let envelope = match decodeLink::<LinkPushCloseEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        if self
            .state
            .pushStreams
            .lock()
            .await
            .remove(&envelope.pushId)
            .is_none()
        {
            return error_response(
                StatusCode::BAD_REQUEST,
                CoreLinkError::new("PUSH_NOT_FOUND", "Link push stream not found"),
            );
        }
        encode_response(
            StatusCode::OK,
            LinkPushCloseResponse {
                pushId: envelope.pushId,
            },
        )
    }

    /// Upgrades an HTTP request into a WebSocket core link session.
    pub async fn ws(&self, upgrade: WebSocketUpgrade) -> Response {
        let dispatcher = self.clone();
        upgrade
            .on_upgrade(move |socket| async move {
                dispatcher.handleWs(socket).await;
            })
            .into_response()
    }

    /// Registers a logical push stream in this dispatcher.
    #[allow(non_snake_case)]
    async fn openPushStream(
        &self,
        pushId: String,
        request: CorePushRequest,
    ) -> Result<LinkPushOpenResponse, CoreLinkError> {
        let mut streams = self.state.pushStreams.lock().await;
        if streams.contains_key(&pushId) {
            return Err(CoreLinkError::new(
                "PUSH_ALREADY_EXISTS",
                "Link push stream already exists",
            ));
        }
        streams.insert(
            pushId.clone(),
            LinkPushState {
                request,
                nextSequence: 0,
            },
        );
        Ok(LinkPushOpenResponse { pushId })
    }

    /// Dispatches one push item through its registered method target.
    #[allow(non_snake_case)]
    async fn executePushItem(
        &self,
        item: CorePushItem,
    ) -> Result<LinkPushItemResponse, CoreLinkError> {
        let request = {
            let mut streams = self.state.pushStreams.lock().await;
            let state = streams.get_mut(&item.pushId).ok_or_else(|| {
                CoreLinkError::new("PUSH_NOT_FOUND", "Link push stream not found")
            })?;
            if item.sequence != state.nextSequence {
                return Err(CoreLinkError::new(
                    "PUSH_SEQUENCE_MISMATCH",
                    format!(
                        "Link push sequence is {}, expected {}",
                        item.sequence, state.nextSequence
                    ),
                ));
            }
            state.nextSequence += 1;
            state.request.clone()
        };
        let response = self
            .executeCall(request.itemCall(item.sequence, item.args))
            .await;
        match response.result {
            Ok(_) => Ok(LinkPushItemResponse {
                pushId: item.pushId,
                sequence: item.sequence,
            }),
            Err(error) => Err(error),
        }
    }

    #[allow(non_snake_case)]
    async fn openWatchChannelEvents(&self, channelId: String) -> Response {
        let (sender, receiver) = mpsc::unbounded_channel::<LinkWatchChannelEvent>();
        let watchChannels = self.state.watchChannels.clone();
        let previous = self.state.watchChannels.lock().await.insert(
            channelId.clone(),
            LinkWatchChannel {
                sender,
                subscriptions: BTreeMap::new(),
            },
        );
        if let Some(previous) = previous {
            abort_watch_channel(previous);
        }
        let stream = LinkWatchChannelEventStream {
            receiver,
            watchChannels,
            channelId,
        };
        Response::builder()
            .header("content-type", "application/msgpack-seq")
            .body(Body::from_stream(stream))
            .expect("watch channel event response must build")
    }

    #[allow(non_snake_case)]
    async fn openWatchChannelSubscription(
        &self,
        channelId: String,
        subscriptionId: String,
        request: CoreWatchRequest,
    ) -> Result<LinkWatchChannelOpenResponse, CoreLinkError> {
        let channel_sender = {
            let channels = self.state.watchChannels.lock().await;
            channels
                .get(&channelId)
                .map(|channel| channel.sender.clone())
                .ok_or_else(|| {
                    CoreLinkError::new("WATCH_CHANNEL_NOT_FOUND", "watch channel not found")
                })?
        };
        let CoreLinkHttpCore::Locked(core) = &self.state.core;
        let mut core = core.lock().await;
        let receiver = core.watch(request).await?;
        let task_subscription_id = subscriptionId.clone();
        let task_channel_id = channelId.clone();
        let task_watch_channels = self.state.watchChannels.clone();
        let task = tokio::spawn(async move {
            let mut receiver = receiver;
            while let Some(event) = receiver.recv().await {
                let completed = event.kind == CoreEventKind::Completed;
                if channel_sender
                    .send(LinkWatchChannelEvent {
                        subscriptionId: task_subscription_id.clone(),
                        event,
                    })
                    .is_err()
                {
                    let mut channels = task_watch_channels.lock().await;
                    if let Some(channel) = channels.get_mut(&task_channel_id) {
                        channel.subscriptions.remove(&task_subscription_id);
                    }
                    return;
                }
                if completed {
                    let mut channels = task_watch_channels.lock().await;
                    if let Some(channel) = channels.get_mut(&task_channel_id) {
                        channel.subscriptions.remove(&task_subscription_id);
                    }
                    return;
                }
            }
            let mut channels = task_watch_channels.lock().await;
            if let Some(channel) = channels.get_mut(&task_channel_id) {
                channel.subscriptions.remove(&task_subscription_id);
            }
        });
        let mut channels = self.state.watchChannels.lock().await;
        let Some(channel) = channels.get_mut(&channelId) else {
            task.abort();
            return Err(CoreLinkError::new(
                "WATCH_CHANNEL_NOT_FOUND",
                "watch channel not found",
            ));
        };
        channel.subscriptions.insert(subscriptionId.clone(), task);
        Ok(LinkWatchChannelOpenResponse { subscriptionId })
    }

    #[allow(non_snake_case)]
    async fn closeWatchChannelSubscription(&self, channelId: &str, subscriptionId: &str) {
        let mut channels = self.state.watchChannels.lock().await;
        if let Some(channel) = channels.get_mut(channelId) {
            if let Some(task) = channel.subscriptions.remove(subscriptionId) {
                task.abort();
            }
        }
    }

    #[allow(non_snake_case)]
    async fn handleWs(&self, mut socket: WebSocket) {
        while let Some(Ok(message)) = socket.recv().await {
            match message {
                Message::Binary(bytes) => {
                    let response = self.handleWsBinary(&bytes).await;
                    let _ = socket.send(Message::Binary(response)).await;
                }
                Message::Close(frame) => {
                    let _ = socket.send(Message::Close(frame)).await;
                    break;
                }
                _ => {}
            }
        }
    }

    #[allow(non_snake_case)]
    async fn handleWsBinary(&self, bytes: &[u8]) -> Vec<u8> {
        let response = match decodeLink::<CoreLinkWsPayload>(bytes) {
            Ok(payload) => self.handleWsPayload(payload).await,
            Err(error) => {
                CoreLinkWsResponse::Error(CoreLinkError::new("BAD_REQUEST", error.to_string()))
            }
        };
        encodeLink(response).expect("CoreLinkWsResponse must serialize")
    }

    #[allow(non_snake_case)]
    async fn handleWsPayload(&self, payload: CoreLinkWsPayload) -> CoreLinkWsResponse {
        match payload {
            CoreLinkWsPayload::Call(request) => {
                let CoreLinkHttpCore::Locked(core) = &self.state.core;
                let mut core = core.lock().await;
                let response = core.call(request.request).await;
                CoreLinkWsResponse::Call(response)
            }
            CoreLinkWsPayload::WatchSnapshot(request) => {
                let CoreLinkHttpCore::Locked(core) = &self.state.core;
                let mut core = core.lock().await;
                let response = core.watchSnapshot(request.request).await;
                match response {
                    Ok(event) => CoreLinkWsResponse::WatchSnapshot(event),
                    Err(error) => CoreLinkWsResponse::Error(error),
                }
            }
            CoreLinkWsPayload::PushOpen(envelope) => {
                match self.openPushStream(envelope.pushId, envelope.request).await {
                    Ok(response) => CoreLinkWsResponse::PushOpened(response),
                    Err(error) => CoreLinkWsResponse::Error(error),
                }
            }
            CoreLinkWsPayload::PushItem(item) => match self.executePushItem(item).await {
                Ok(response) => CoreLinkWsResponse::PushAccepted(response),
                Err(error) => CoreLinkWsResponse::Error(error),
            },
            CoreLinkWsPayload::PushClose(envelope) => {
                if self
                    .state
                    .pushStreams
                    .lock()
                    .await
                    .remove(&envelope.pushId)
                    .is_none()
                {
                    return CoreLinkWsResponse::Error(CoreLinkError::new(
                        "PUSH_NOT_FOUND",
                        "Link push stream not found",
                    ));
                }
                CoreLinkWsResponse::PushClosed(LinkPushCloseResponse {
                    pushId: envelope.pushId,
                })
            }
        }
    }
}

fn abort_watch_channel(channel: LinkWatchChannel) {
    for (_, task) in channel.subscriptions {
        task.abort();
    }
}

fn bad_request(message: impl Into<String>) -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        CoreLinkError::new("BAD_REQUEST", message.into()),
    )
}

/// Creates a MessagePack codec failure response.
fn codec_error(message: impl Into<String>) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        CoreLinkError::new("CODEC_ERROR", message.into()),
    )
}

/// Encodes a successful typed Link response.
fn encode_response(value_status: StatusCode, value: impl Serialize) -> Response {
    match encodeLink(value) {
        Ok(bytes) => binary_response(value_status, bytes),
        Err(error) => codec_error(error.to_string()),
    }
}

/// Encodes a structured Link error response.
fn error_response(status: StatusCode, error: CoreLinkError) -> Response {
    match encodeLink(error) {
        Ok(bytes) => binary_response(status, bytes),
        Err(encode_error) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("content-type", "text/plain; charset=utf-8")
            .body(Body::from(encode_error.to_string()))
            .expect("plain codec failure response must build"),
    }
}

/// Creates a MessagePack HTTP response from encoded Link bytes.
fn binary_response(status: StatusCode, bytes: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header("content-type", "application/msgpack")
        .body(Body::from(bytes))
        .expect("binary link response must build")
}
