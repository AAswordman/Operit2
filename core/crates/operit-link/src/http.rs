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

use crate::client::CoreLinkClient;
use crate::protocol::{
    CoreCallRequest, CoreCallResponse, CoreEvent, CoreEventKind, CoreLinkError, CoreWatchRequest,
};

#[derive(Clone)]
pub struct CoreLinkHttpDispatcher {
    state: Arc<CoreLinkHttpState>,
}

struct CoreLinkHttpState {
    core: Arc<Mutex<Box<dyn CoreLinkClient + Send>>>,
    watchChannels: Arc<Mutex<BTreeMap<String, LinkWatchChannel>>>,
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
                let mut line =
                    serde_json::to_vec(&event).expect("LinkWatchChannelEvent must serialize");
                line.push(b'\n');
                Poll::Ready(Some(Ok(Bytes::from(line))))
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
pub struct LinkWatchChannelEvent {
    pub subscriptionId: String,
    pub event: CoreEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkWsPayload {
    Call(LinkCallEnvelope),
    WatchSnapshot(LinkWatchEnvelope),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "body")]
pub enum CoreLinkWsResponse {
    Call(CoreCallResponse),
    WatchSnapshot(CoreEvent),
    Error(CoreLinkError),
}

impl CoreLinkHttpDispatcher {
    pub fn new(core: impl CoreLinkClient + Send + 'static) -> Self {
        Self {
            state: Arc::new(CoreLinkHttpState {
                core: Arc::new(Mutex::new(Box::new(core))),
                watchChannels: Arc::new(Mutex::new(BTreeMap::new())),
            }),
        }
    }

    pub async fn call(&self, body: Bytes) -> Response {
        let envelope = match serde_json::from_slice::<LinkCallEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        let mut core = self.state.core.lock().await;
        JsonResponse(core.call(envelope.request).await).into_response()
    }

    #[allow(non_snake_case)]
    pub async fn watchSnapshot(&self, body: Bytes) -> Response {
        let envelope = match serde_json::from_slice::<LinkWatchEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        let mut core = self.state.core.lock().await;
        match core.watchSnapshot(envelope.request).await {
            Ok(event) => JsonResponse(event).into_response(),
            Err(error) => (StatusCode::BAD_REQUEST, JsonResponse(error)).into_response(),
        }
    }

    #[allow(non_snake_case)]
    pub async fn watchChannelEvents(&self, body: Bytes) -> Response {
        let envelope = match serde_json::from_slice::<LinkWatchChannelEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        self.openWatchChannelEvents(envelope.channelId).await
    }

    #[allow(non_snake_case)]
    pub async fn watchChannelOpen(&self, body: Bytes) -> Response {
        let envelope = match serde_json::from_slice::<LinkWatchChannelOpenEnvelope>(&body) {
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
            Ok(response) => JsonResponse(response).into_response(),
            Err(error) => (StatusCode::BAD_REQUEST, JsonResponse(error)).into_response(),
        }
    }

    #[allow(non_snake_case)]
    pub async fn watchChannelClose(&self, body: Bytes) -> Response {
        let envelope = match serde_json::from_slice::<LinkWatchChannelCloseEnvelope>(&body) {
            Ok(value) => value,
            Err(error) => return bad_request(error.to_string()),
        };
        self.closeWatchChannelSubscription(&envelope.channelId, &envelope.subscriptionId)
            .await;
        JsonResponse(serde_json::json!({})).into_response()
    }

    pub async fn ws(&self, upgrade: WebSocketUpgrade) -> Response {
        let dispatcher = self.clone();
        upgrade
            .on_upgrade(move |socket| async move {
                dispatcher.handleWs(socket).await;
            })
            .into_response()
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
            .header("content-type", "application/x-ndjson")
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
        let mut core = self.state.core.lock().await;
        let receiver = core.watch(request).await?;
        drop(core);
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
                Message::Text(text) => {
                    let response = self.handleWsText(text).await;
                    let _ = socket.send(Message::Text(response)).await;
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
    async fn handleWsText(&self, text: String) -> String {
        let response = match serde_json::from_str::<CoreLinkWsPayload>(&text) {
            Ok(payload) => self.handleWsPayload(payload).await,
            Err(error) => {
                CoreLinkWsResponse::Error(CoreLinkError::new("BAD_REQUEST", error.to_string()))
            }
        };
        serde_json::to_string(&response).expect("CoreLinkWsResponse must serialize")
    }

    #[allow(non_snake_case)]
    async fn handleWsPayload(&self, payload: CoreLinkWsPayload) -> CoreLinkWsResponse {
        match payload {
            CoreLinkWsPayload::Call(request) => {
                let mut core = self.state.core.lock().await;
                CoreLinkWsResponse::Call(core.call(request.request).await)
            }
            CoreLinkWsPayload::WatchSnapshot(request) => {
                let mut core = self.state.core.lock().await;
                match core.watchSnapshot(request.request).await {
                    Ok(event) => CoreLinkWsResponse::WatchSnapshot(event),
                    Err(error) => CoreLinkWsResponse::Error(error),
                }
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
    (
        StatusCode::BAD_REQUEST,
        JsonResponse(CoreLinkError::new("BAD_REQUEST", message.into())),
    )
        .into_response()
}

struct JsonResponse<T>(T);

impl<T> IntoResponse for JsonResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}
