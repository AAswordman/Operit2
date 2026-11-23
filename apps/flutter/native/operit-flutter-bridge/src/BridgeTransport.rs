//! Bounded connection mailboxes and independently ready Push/Watch continuations.
//! Affine opens are transient; ongoing sources share the Host pool's ready queue.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::{future::BoxFuture, FutureExt};
use operit_host_api::{HostRuntimePooledTask, HostRuntimeTaskSchedulerHost};
use operit_link::{
    CoreEvent, CoreEventKind, CoreLinkError, CoreLinkSharedClient, CorePushItem, CorePushRequest,
    CoreWatchRequest, CoreEventStream, CoreLinkPushSession,
};
use tokio::sync::{mpsc, oneshot, watch, Mutex};

const COMMAND_CAPACITY: usize = 64;
const PUSH_CAPACITY: usize = 16;
pub(crate) const WATCH_BATCH_CAPACITY: usize = 64;

type Reply = oneshot::Sender<Result<Option<String>, CoreLinkError>>;
pub(crate) type StreamReply = oneshot::Receiver<Result<Option<String>, CoreLinkError>>;
pub(crate) type StreamSubmission = BoxFuture<'static, Result<Option<String>, CoreLinkError>>;
type Client = Arc<dyn CoreLinkSharedClient + Send + Sync>;
type Completion = (StreamKey, Arc<()>);
type Opened<T> = oneshot::Receiver<Result<T, CoreLinkError>>;

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
    admission: Arc<Mutex<()>>,
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

struct StreamOwner<T = WatchDelivery> {
    core: Client,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<T>,
    encode: fn(WatchDelivery) -> T,
    shutdown: watch::Receiver<bool>,
    pushes: HashMap<String, PushRegistration>,
    watches: HashMap<String, WatchRegistration>,
    complete: mpsc::Sender<Completion>,
    completed: mpsc::Receiver<Completion>,
}

impl BridgeStreams {
    /// Starts exactly one owner task using the runtime's installed Host scheduler.
    pub(crate) fn start<T: Send + 'static>(
        core: Client,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
        events: mpsc::Sender<T>,
        encode: fn(WatchDelivery) -> T,
    ) -> Result<Self, CoreLinkError> {
        let (client, owner) = Self::channel(core, scheduler.clone(), events, encode);
        scheduler
            .scheduleHostRuntimePooledTask(
                "operit-bridge-streams",
                None,
                Box::pin(owner.run()),
            )
            .map_err(|error| CoreLinkError::internal(error.to_string()))?;
        Ok(client)
    }

    /// Creates bounded connection queues before moving their owner onto its executor.
    fn channel<T>(
        core: Client,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
        events: mpsc::Sender<T>,
        encode: fn(WatchDelivery) -> T,
    ) -> (Self, StreamOwner<T>) {
        let (commands, receiver) = mpsc::channel(COMMAND_CAPACITY);
        let (shutdown, stopped) = watch::channel(false);
        let (complete, completed) = mpsc::channel(COMMAND_CAPACITY);
        let owner = StreamOwner {
            core,
            scheduler,
            commands: receiver,
            events,
            encode,
            shutdown: stopped,
            pushes: HashMap::new(),
            watches: HashMap::new(),
            complete,
            completed,
        };
        (
            Self {
                commands,
                shutdown,
                admission: Arc::new(Mutex::new(())),
            },
            owner,
        )
    }

    /// Orders native ingress before scheduling, waiting for capacity rather than failing a session.
    pub(crate) fn enqueue(&self, action: StreamAction) -> StreamSubmission {
        let (reply, response) = oneshot::channel();
        // Poll the fair lock once at ingress so Host task scheduling cannot reorder waiters.
        // Disabling cooperative budgeting here ensures even a busy caller registers its place.
        let mut admission = Box::pin(tokio::task::unconstrained(
            self.admission.clone().lock_owned(),
        ));
        let ready = admission.as_mut().now_or_never();
        let commands = self.commands.clone();
        let mut shutdown = self.shutdown.subscribe();
        async move {
            if *shutdown.borrow() {
                return Err(closed());
            }
            let admitted = async move {
                let _guard = match ready {
                    Some(guard) => guard,
                    None => admission.await,
                };
                commands
                    .send(Command { action, reply })
                    .await
                    .map_err(|_| closed())
            };
            tokio::select! {
                biased;
                _ = shutdown.changed() => return Err(closed()),
                sent = admitted => sent?,
            }
            // Release admission before awaiting Core: pending opens must remain cancellable.
            response.await.map_err(|_| closed())?
        }
        .boxed()
    }

    /// Immediately submits browser cancellation where the synchronous API cannot wait.
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

    /// Cancels pending opens, active sources, and queued pushes as one connection lifetime.
    pub(crate) fn close(&self) {
        self.shutdown.send_replace(true);
    }

    /// Lets transport writers stop without waiting for another source event.
    pub(crate) fn stopped(&self) -> watch::Receiver<bool> {
        self.shutdown.subscribe()
    }
}

impl Drop for BridgeStreams {
    /// Releases all streams when their connection owner is dropped.
    fn drop(&mut self) {
        self.close();
    }
}

impl<T: Send + 'static> StreamOwner<T> {
    /// Polls commands and Host job completions without executing Core source code.
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
                Some((key, generation)) = self.completed.recv() => {
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
                let encode = self.encode;
                let name = format!("operit-bridge-watch id={id} tag={tag} target={} property={}", request.target, request.propertyName);
                self.spawn(&name, StreamKey::Watch(id.clone()), generation, reply,
                    move || Box::pin(async move { core.watch(request).await }),
                    move |reply, opened| Box::pin(forward_watch(opened, id, tag, reply, cancelled, events, encode)),
                );
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
                let name = format!("operit-bridge-push id={id} target={} method={}", request.target, request.methodName);
                self.spawn(&name, StreamKey::Push(id.clone()), generation, reply,
                    move || Box::pin(async move { core.openPush(request).await }),
                    move |reply, opened| Box::pin(run_push(opened, id, reply, receiver)),
                );
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

    /// Reserves a movable continuation and a transient affine open in the same bounded Host pool.
    fn spawn<S: Send + 'static>(
        &mut self,
        name: &str,
        key: StreamKey,
        generation: Arc<()>,
        reply: Reply,
        open: impl FnOnce() -> Pin<Box<dyn Future<Output = Result<S, CoreLinkError>>>> + Send + 'static,
        task: impl FnOnce(Reply, Opened<S>) -> HostRuntimePooledTask + Send + 'static,
    ) {
        let (start, started) = oneshot::channel::<Reply>();
        let complete = self.complete.clone();
        let completion = (key.clone(), generation);
        let (mut opened, source) = oneshot::channel();
        let mut shutdown = self.shutdown.clone();
        let scheduled = self.scheduler.scheduleHostRuntimePooledTask(
            name,
            Some(Box::new(move || Box::pin(async move {
                let result = std::panic::AssertUnwindSafe(async {
                    tokio::select! {
                        biased;
                        _ = opened.closed() => None,
                        result = async move { open().await } => Some(result),
                    }
                }).catch_unwind().await.unwrap_or_else(|payload| Some(Err(panic_error(payload))));
                if let Some(result) = result { let _ = opened.send(result); }
            }))),
            Box::pin(async move {
                let Ok(reply) = started.await else { return; };
                if *shutdown.borrow() { return; }
                let source = async move { task(reply, source).await };
                let outcome = std::panic::AssertUnwindSafe(async {
                    tokio::select! {
                        biased;
                        _ = shutdown.changed() => {},
                        _ = source => {},
                    }
                }).catch_unwind().await;
                if let Err(payload) = outcome {
                    operit_host_api::logHostError(
                        "BridgeStreams",
                        &crate::BridgeExports::panic_payload_message(payload.as_ref()),
                    );
                }
                let _ = complete.send(completion).await;
            }),
        );
        if let Err(error) = scheduled {
            match &key {
                StreamKey::Push(id) => { self.pushes.remove(id); }
                StreamKey::Watch(id) => { self.watches.remove(id); }
            }
            let _ = reply.send(Err(CoreLinkError::internal(error.to_string())));
            return;
        }
        // Transfer the opening reply only after scheduling succeeds so admission errors keep their cause.
        if let Err(reply) = start.send(reply) {
            let _ = reply.send(Err(closed()));
        }
    }
}

/// Opens and forwards one source with cancellation during both opening and delivery.
async fn forward_watch<T: Send>(
    mut opened: Opened<CoreEventStream>,
    id: String,
    tag: i64,
    reply: Reply,
    mut cancelled: oneshot::Receiver<()>,
    events: mpsc::Sender<T>,
    encode: fn(WatchDelivery) -> T,
) {
    let mut streaming = false;
    let mut reply = Some(reply);
    let outcome = std::panic::AssertUnwindSafe(async {
        let opened = tokio::select! {
            biased;
            _ = &mut cancelled => Err(CoreLinkError::new("WATCH_CLOSED", "Watch was cancelled while opening")),
            opened = &mut opened => opened.unwrap_or_else(|_| Err(closed())),
        };
        let mut source = match opened {
            Ok(source) => source,
            Err(error) => { let _ = reply.take().expect("Opening reply").send(Err(error)); return; }
        };
        if reply.take().expect("Opening reply").send(Ok(Some(id.clone()))).is_err() { return; }
        streaming = true;
        loop {
            let event = tokio::select! {
                biased;
                _ = &mut cancelled => break,
                event = source.recv() => match event { Some(event) => event, None => break },
            };
            let completed = event.kind == CoreEventKind::Completed;
            let delivery = encode(WatchDelivery { id: id.clone(), tag, event: Ok(Some(event)) });
            tokio::select! {
                biased;
                _ = &mut cancelled => break,
                sent = events.send(delivery) => { if sent.is_err() { return; } }
            }
            if completed { break; }
        }
    }).catch_unwind().await;
    // Failed opens are settled by their reply; only opened sources publish completion.
    if !streaming && outcome.is_ok() { return; }
    let event = outcome.map(|_| None).map_err(panic_error);
    if let Some(reply) = reply {
        let _ = reply.send(event.map(|_| Some(id)));
        return;
    }
    let _ = events.send(encode(WatchDelivery { id, tag, event })).await;
}

/// Serializes one Push's values while other streams remain independently pollable.
async fn run_push(
    opened: Opened<Box<dyn CoreLinkPushSession>>,
    id: String,
    reply: Reply,
    mut commands: mpsc::Receiver<Command>,
) {
    let mut session = match opened.await.unwrap_or_else(|_| Err(closed())) {
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

/// Preserves source panic details in the opening reply or terminal event.
fn panic_error(payload: Box<dyn std::any::Any + Send>) -> CoreLinkError {
    CoreLinkError::new("FATAL_CORE_PANIC", crate::BridgeExports::panic_payload_message(payload.as_ref()))
}

/// Reports a stopped owner instead of leaving an operation waiting indefinitely.
fn closed() -> CoreLinkError {
    CoreLinkError::new("STREAM_CLOSED", "Connection stream owner is closed")
}

#[cfg(test)]
#[path = "BridgeTransportTests.rs"]
mod tests;
