//! One authenticated reader handles control frames before logical request dispatch.
use super::crypto::{error, Channel};
use super::heartbeat::{Heartbeat, HeartbeatAction};
use futures_util::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_link::*;
use operit_peer_link::{PeerConnection, PeerMessage};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Mutex as AsyncMutex};

const CONTROL_TARGET: &str = "$peer.channel";
const TICK_MS: u64 = 1_000;

/// Keeps control traffic and channel failure independent of routed operation execution.
pub(super) struct LiveChannel {
    pub raw: Arc<dyn PeerConnection>,
    channel: Arc<Channel>,
    inbox: AsyncMutex<mpsc::UnboundedReceiver<PeerMessage>>,
    heartbeat: Mutex<Heartbeat>,
    failure: Mutex<Option<CoreLinkError>>,
    transaction: AsyncMutex<()>,
    changed: Box<dyn Fn() + Send + Sync>,
}

impl LiveChannel {
    /// Starts one authenticated reader on the installed cross-platform Host scheduler.
    pub(super) fn start(
        channel: Arc<Channel>, scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
        changed: impl Fn() + Send + Sync + 'static,
    ) -> Result<Arc<Self>, CoreLinkError> {
        let now = scheduler.monotonicTimeMillis().map_err(|error| super::crypto::error(error.to_string()))?;
        let (sender, receiver) = mpsc::unbounded_channel();
        let live = Arc::new(Self {
            raw: channel.raw.clone(), channel, inbox: AsyncMutex::new(receiver),
            heartbeat: Mutex::new(Heartbeat::new(now)),
            failure: Mutex::new(None), transaction: AsyncMutex::new(()), changed: Box::new(changed),
        });
        let reader = live.clone();
        let tasks = scheduler.clone();
        scheduler.scheduleHostRuntimeAsyncTask("peer-channel-reader", Box::new(move || Box::pin(async move {
            reader.readLoop(sender, tasks).await;
        }))).map_err(|e| error(e.to_string()))?;
        Ok(live)
    }

    /// Reports whether this connection generation has reached a terminal failure.
    pub(super) fn isAvailable(&self) -> bool { self.failure.lock().unwrap().is_none() }

    /// Returns the authenticated channel these frames belong to.
    pub(super) fn channel(&self) -> Arc<Channel> { self.channel.clone() }

    /// Sends an authenticated frame and makes explicit write failures terminal for this channel.
    pub(super) async fn send(&self, message: PeerMessage) -> Result<(), CoreLinkError> {
        if let Some(error) = self.failure.lock().unwrap().clone() { return Err(error); }
        match self.channel.send(message).await {
            Ok(()) => Ok(()),
            Err(error) => { self.terminate(error.clone()).await; Err(error) }
        }
    }

    /// Receives only data frames; authenticated control frames never enter the logical dispatcher.
    pub(super) async fn receive(&self) -> Result<Option<PeerMessage>, CoreLinkError> {
        let message = self.inbox.lock().await.recv().await;
        match message {
            Some(message) => Ok(Some(message)),
            None => match self.failure.lock().unwrap().clone() {
                Some(error) => Err(error),
                None => Ok(None),
            },
        }
    }

    /// Exchanges setup messages before the multiplexed response reader is started.
    pub(super) async fn exchange(&self, request: CoreLinkRequest) -> Result<CoreLinkResponse, CoreLinkError> {
        let _transaction = self.transaction.lock().await;
        self.send(PeerMessage::Request(request)).await?;
        match self.receive().await? {
            Some(PeerMessage::Response(response)) => Ok(response),
            _ => Err(error("Missing Link response")),
        }
    }

    /// Publishes a terminal failure once and wakes pending raw I/O.
    async fn terminate(&self, error: CoreLinkError) {
        {
            let mut failure = self.failure.lock().unwrap();
            if failure.is_some() { return; }
            *failure = Some(error);
        }
        (self.changed)();
        self.raw.close().await;
    }

    /// Encodes heartbeat controls in the existing authenticated Link envelope.
    fn control(id: String, method: &str) -> PeerMessage {
        PeerMessage::Request(CoreLinkRequest::Call(CoreCallRequest::new(id, CONTROL_TARGET, method, CoreValue::Null)))
    }

    /// Reads independently of operation execution, with one idle probe per connection.
    async fn readLoop(self: Arc<Self>, sender: mpsc::UnboundedSender<PeerMessage>, scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>) {
        let mut writes = FuturesUnordered::<LocalBoxFuture<'static, Result<(), CoreLinkError>>>::new();
        let mut tick = scheduler.waitForHostRuntimeDelay(TICK_MS);
        let result: Result<(), CoreLinkError> = loop {
            tokio::select! {
                message = self.channel.receive() => {
                    let message = match message {
                        Ok(Some(message)) => message,
                        Ok(None) => break Err(error("Authenticated channel closed")),
                        Err(error) => break Err(error),
                    };
                    let now = match scheduler.monotonicTimeMillis() {
                        Ok(now) => now,
                        Err(error) => break Err(super::crypto::error(error.to_string())),
                    };
                    self.heartbeat.lock().unwrap().received(now);
                    match message {
                        PeerMessage::Request(CoreLinkRequest::Call(request)) if request.target == CONTROL_TARGET => {
                            if request.args != CoreValue::Null { break Err(error("Invalid channel control payload")); }
                            match request.methodName.as_str() {
                                "ping" => {
                                    let channel = self.clone();
                                    writes.push(Box::pin(async move { channel.send(Self::control(request.requestId.0, "pong")).await }));
                                }
                                "pong" => self.heartbeat.lock().unwrap().pong(&request.requestId.0, now),
                                _ => break Err(error("Unknown channel control operation")),
                            }
                        }
                        message => if sender.send(message).is_err() { break Err(error("Authenticated channel consumer closed")); },
                    }
                }
                result = &mut tick => {
                    if let Err(error) = result { break Err(super::crypto::error(error.to_string())); }
                    tick = scheduler.waitForHostRuntimeDelay(TICK_MS);
                    let now = match scheduler.monotonicTimeMillis() {
                        Ok(now) => now,
                        Err(error) => break Err(super::crypto::error(error.to_string())),
                    };
                    let action = self.heartbeat.lock().unwrap().tick(now);
                    match action {
                        HeartbeatAction::Ping(id) => {
                            let channel = self.clone();
                            writes.push(Box::pin(async move { channel.send(Self::control(id, "ping")).await }));
                        }
                        HeartbeatAction::Expired => break Err(CoreLinkError::new("PEER_HEARTBEAT_TIMEOUT", "Authenticated channel remained silent through both heartbeat deadlines")),
                        HeartbeatAction::Suspect | HeartbeatAction::None => {},
                    }
                }
                result = writes.next(), if !writes.is_empty() => {
                    if let Some(Err(error)) = result { break Err(error); }
                }
            }
        };
        if let Err(error) = result { self.terminate(error).await; }
    }
}
