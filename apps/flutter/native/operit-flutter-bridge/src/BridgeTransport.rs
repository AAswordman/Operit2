//! Connection-owned Push/Watch multiplexing on one Host executor task.
//! Stream futures share that task; transports only submit commands and deliver batches.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::{stream::FuturesUnordered, FutureExt, StreamExt};
use operit_host_api::HostRuntimeTaskSchedulerHost;
use operit_link::{
    CoreEvent, CoreEventKind, CoreLinkError, CoreLinkSharedClient, CorePushItem, CorePushRequest,
    CoreWatchRequest,
};
use tokio::sync::{mpsc, oneshot, watch};

const COMMAND_CAPACITY: usize = 64;
const PUSH_CAPACITY: usize = 16;
pub(crate) const WATCH_BATCH_CAPACITY: usize = 64;

type Reply = oneshot::Sender<Result<Option<String>, CoreLinkError>>;
pub(crate) type StreamReply = oneshot::Receiver<Result<Option<String>, CoreLinkError>>;
type Client = Arc<dyn CoreLinkSharedClient + Send + Sync>;
type StreamTask = Pin<Box<dyn Future<Output = (StreamKey, Arc<()>)>>>;

/// Identifies a logical stream without conflating Push and Watch namespaces.
#[derive(Clone, Hash, PartialEq, Eq)]
enum StreamKey {
    Push(String),
    Watch(String),
}

/// Carries one ordered operation across the connection's bounded mailbox.
pub(crate) enum StreamAction {
    OpenPush(CorePushRequest),
    PushItem(CorePushItem),
    ClosePush(String),
    OpenWatch {
        id: String,
        request: CoreWatchRequest,
        tag: i64,
    },
    CloseWatch {
        id: String,
        tag: i64,
    },
}

struct Command {
    action: StreamAction,
    reply: Reply,
}

/// Carries one watch value or terminal notification through the shared outbox.
pub(crate) struct WatchDelivery {
    pub(crate) id: String,
    pub(crate) tag: i64,
    pub(crate) event: Result<Option<CoreEvent>, CoreLinkError>,
}

/// Submits stream operations without owning sockets, threads, or source futures.
pub(crate) struct BridgeStreams {
    commands: mpsc::Sender<Command>,
    shutdown: watch::Sender<bool>,
}

struct PushRegistration {
    generation: Arc<()>,
    commands: mpsc::Sender<Command>,
}

struct WatchRegistration {
    tag: i64,
    generation: Arc<()>,
    cancel: oneshot::Sender<()>,
}

struct StreamOwner {
    core: Client,
    commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<WatchDelivery>,
    shutdown: watch::Receiver<bool>,
    pushes: HashMap<String, PushRegistration>,
    watches: HashMap<String, WatchRegistration>,
    tasks: FuturesUnordered<StreamTask>,
}

impl BridgeStreams {
    /// Starts exactly one owner task using the runtime's installed Host scheduler.
    pub(crate) fn start(
        core: Client,
        scheduler: &dyn HostRuntimeTaskSchedulerHost,
    ) -> Result<(Self, mpsc::Receiver<WatchDelivery>), CoreLinkError> {
        let (client, events, owner) = Self::channel(core);
        // Only the mailbox crosses threads; local stream futures are created by its executor.
        let StreamOwner {
            core,
            commands,
            events: sender,
            shutdown,
            ..
        } = owner;
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "operit-bridge-streams",
                Box::new(move || {
                    Box::pin(
                        StreamOwner {
                            core,
                            commands,
                            events: sender,
                            shutdown,
                            pushes: HashMap::new(),
                            watches: HashMap::new(),
                            tasks: FuturesUnordered::new(),
                        }
                        .run(),
                    )
                }),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok((client, events))
    }

    /// Creates bounded connection queues before moving their owner onto its executor.
    fn channel(core: Client) -> (Self, mpsc::Receiver<WatchDelivery>, StreamOwner) {
        let (commands, receiver) = mpsc::channel(COMMAND_CAPACITY);
        let (events, outbox) = mpsc::channel(WATCH_BATCH_CAPACITY);
        let (shutdown, stopped) = watch::channel(false);
        let owner = StreamOwner {
            core,
            commands: receiver,
            events,
            shutdown: stopped,
            pushes: HashMap::new(),
            watches: HashMap::new(),
            tasks: FuturesUnordered::new(),
        };
        (Self { commands, shutdown }, outbox, owner)
    }

    /// Reserves ingress order synchronously for native FFI submissions.
    pub(crate) fn submit(&self, action: StreamAction) -> Result<StreamReply, CoreLinkError> {
        if *self.shutdown.borrow() {
            return Err(closed());
        }
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command { action, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => CoreLinkError::new(
                    "STREAM_BACKPRESSURE",
                    "Connection stream command queue is full",
                ),
                mpsc::error::TrySendError::Closed(_) => closed(),
            })?;
        Ok(response)
    }

    /// Applies asynchronous backpressure while submitting a browser stream operation.
    pub(crate) async fn request(
        &self,
        action: StreamAction,
    ) -> Result<Option<String>, CoreLinkError> {
        if *self.shutdown.borrow() {
            return Err(closed());
        }
        let (reply, response) = oneshot::channel();
        self.commands
            .send(Command { action, reply })
            .await
            .map_err(|_| closed())?;
        response.await.map_err(|_| closed())?
    }

    /// Cancels pending opens, active sources, and queued pushes as one connection lifetime.
    pub(crate) fn close(&self) {
        self.shutdown.send_replace(true);
    }
}

impl Drop for BridgeStreams {
    /// Releases all streams when their connection owner is dropped.
    fn drop(&mut self) {
        self.close();
    }
}

impl StreamOwner {
    /// Polls every stream on this executor without spawning a task per subscription.
    async fn run(mut self) {
        loop {
            if *self.shutdown.borrow() {
                break;
            }
            tokio::select! {
                _ = self.shutdown.changed() => break,
                command = self.commands.recv() => match command {
                    Some(command) => self.dispatch(command),
                    None => break,
                },
                Some((key, generation)) = self.tasks.next(), if !self.tasks.is_empty() => {
                    match key {
                        StreamKey::Push(id) => {
                            if self.pushes.get(&id).is_some_and(|entry| Arc::ptr_eq(&entry.generation, &generation)) {
                                self.pushes.remove(&id);
                            }
                        }
                        StreamKey::Watch(id) => {
                            if self.watches.get(&id).is_some_and(|entry| Arc::ptr_eq(&entry.generation, &generation)) {
                                self.watches.remove(&id);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Registers operations in ingress order while independent streams keep making progress.
    fn dispatch(&mut self, command: Command) {
        let Command { action, reply } = command;
        match action {
            StreamAction::OpenWatch { id, request, tag } => {
                if self.watches.contains_key(&id) {
                    let _ = reply.send(Err(CoreLinkError::new(
                        "WATCH_ALREADY_EXISTS",
                        "Watch subscription already exists",
                    )));
                    return;
                }
                let (cancel, cancelled) = oneshot::channel();
                let generation = Arc::new(());
                self.watches.insert(
                    id.clone(),
                    WatchRegistration {
                        tag,
                        generation: generation.clone(),
                        cancel,
                    },
                );
                let core = self.core.clone();
                let events = self.events.clone();
                self.spawn(StreamKey::Watch(id.clone()), generation, async move {
                    forward_watch(core, id, request, tag, reply, cancelled, events).await;
                });
            }
            StreamAction::CloseWatch { id, tag } => {
                if self.watches.get(&id).is_some_and(|entry| entry.tag == tag) {
                    let entry = self.watches.remove(&id).expect("Matching watch generation");
                    let _ = entry.cancel.send(());
                }
                let _ = reply.send(Ok(None));
            }
            StreamAction::OpenPush(request) => {
                let id = request.requestId.0.clone();
                if self.pushes.contains_key(&id) {
                    let _ = reply.send(Err(CoreLinkError::new(
                        "PUSH_ALREADY_EXISTS",
                        "Push stream already exists",
                    )));
                    return;
                }
                let (commands, receiver) = mpsc::channel(PUSH_CAPACITY);
                let generation = Arc::new(());
                self.pushes.insert(
                    id.clone(),
                    PushRegistration {
                        generation: generation.clone(),
                        commands,
                    },
                );
                let core = self.core.clone();
                self.spawn(StreamKey::Push(id), generation, async move {
                    run_push(core, request, reply, receiver).await;
                });
            }
            action @ (StreamAction::PushItem(_) | StreamAction::ClosePush(_)) => {
                let (id, closing) = match &action {
                    StreamAction::PushItem(item) => (item.pushId.clone(), false),
                    StreamAction::ClosePush(id) => (id.clone(), true),
                    _ => unreachable!(),
                };
                let Some(entry) = self.pushes.get(&id) else {
                    let _ = reply.send(Err(CoreLinkError::new(
                        "PUSH_NOT_FOUND",
                        "Push is not owned by this connection",
                    )));
                    return;
                };
                match entry.commands.try_send(Command { action, reply }) {
                    Ok(()) => {
                        if closing {
                            self.pushes.remove(&id);
                        }
                    }
                    Err(error) => {
                        let (command, error) = match error {
                            mpsc::error::TrySendError::Full(command) => (
                                command,
                                CoreLinkError::new(
                                    "PUSH_BACKPRESSURE",
                                    "Push command queue is full",
                                ),
                            ),
                            mpsc::error::TrySendError::Closed(command) => (command, closed()),
                        };
                        let _ = command.reply.send(Err(error));
                    }
                }
            }
        }
    }

    /// Confines a source panic to its stream and keeps generation cleanup on the owner.
    fn spawn(
        &mut self,
        key: StreamKey,
        generation: Arc<()>,
        task: impl Future<Output = ()> + 'static,
    ) {
        self.tasks.push(Box::pin(async move {
            if let Err(payload) = std::panic::AssertUnwindSafe(task).catch_unwind().await {
                operit_host_api::logHostError(
                    "BridgeStreams",
                    &crate::BridgeExports::panic_payload_message(payload.as_ref()),
                );
            }
            (key, generation)
        }));
    }
}

/// Opens and forwards one source with cancellation during both opening and delivery.
async fn forward_watch(
    core: Client,
    id: String,
    request: CoreWatchRequest,
    tag: i64,
    reply: Reply,
    mut cancelled: oneshot::Receiver<()>,
    events: mpsc::Sender<WatchDelivery>,
) {
    let outcome = std::panic::AssertUnwindSafe(async {
        let opened = tokio::select! {
            biased;
            _ = &mut cancelled => Err(CoreLinkError::new("WATCH_CLOSED", "Watch was cancelled while opening")),
            opened = core.watch(request) => opened,
        };
        let mut source = match opened {
            Ok(source) => source,
            Err(error) => { let _ = reply.send(Err(error)); return; }
        };
        if reply.send(Ok(Some(id.clone()))).is_err() { return; }
        loop {
            let event = tokio::select! {
                biased;
                _ = &mut cancelled => break,
                event = source.recv() => match event { Some(event) => event, None => break },
            };
            let completed = event.kind == CoreEventKind::Completed;
            let delivery = WatchDelivery { id: id.clone(), tag, event: Ok(Some(event)) };
            tokio::select! {
                biased;
                _ = &mut cancelled => break,
                sent = events.send(delivery) => { if sent.is_err() { return; } }
            }
            if completed { break; }
        }
    }).catch_unwind().await;
    let event = outcome.map(|_| None).map_err(|payload| {
        CoreLinkError::new(
            "FATAL_CORE_PANIC",
            crate::BridgeExports::panic_payload_message(payload.as_ref()),
        )
    });
    let _ = events.send(WatchDelivery { id, tag, event }).await;
}

/// Serializes one Push's values while other streams remain independently pollable.
async fn run_push(
    core: Client,
    request: CorePushRequest,
    reply: Reply,
    mut commands: mpsc::Receiver<Command>,
) {
    let id = request.requestId.0.clone();
    let mut session = match core.openPush(request).await {
        Ok(session) => session,
        Err(error) => {
            let _ = reply.send(Err(error));
            return;
        }
    };
    if reply.send(Ok(Some(id))).is_err() {
        return;
    }
    let mut next_sequence = 0u64;
    while let Some(Command { action, reply }) = commands.recv().await {
        match action {
            StreamAction::PushItem(item) => {
                if item.sequence != next_sequence {
                    let _ = reply.send(Err(CoreLinkError::new(
                        "PUSH_SEQUENCE_MISMATCH",
                        "Push item is out of order",
                    )));
                    continue;
                }
                let Some(next) = next_sequence.checked_add(1) else {
                    let _ = reply.send(Err(CoreLinkError::new(
                        "PUSH_SEQUENCE_OVERFLOW",
                        "Push sequence overflowed",
                    )));
                    return;
                };
                let result = session.send(item.args).await;
                let failed = result.is_err();
                let _ = reply.send(result.map(|_| None));
                if failed {
                    return;
                }
                next_sequence = next;
            }
            StreamAction::ClosePush(_) => {
                let _ = reply.send(session.close().await.map(|_| None));
                return;
            }
            _ => unreachable!(),
        }
    }
}

/// Reports a stopped owner instead of leaving an operation waiting indefinitely.
fn closed() -> CoreLinkError {
    CoreLinkError::new("STREAM_CLOSED", "Connection stream owner is closed")
}

#[cfg(test)]
#[path = "BridgeTransportTests.rs"]
mod tests;
