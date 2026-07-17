use std::collections::VecDeque;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost;

use crate::stream::Stream::Stream;

/// Stream that can have multiple collectors and optional replayed items.
pub trait SharedStream<T>: Stream<Item = T>
where
    T: Clone,
{
    /// Returns the number of active collectors currently subscribed.
    fn subscription_count(&self) -> usize;

    /// Returns the items retained for replay to new collectors.
    fn replay_cache(&self) -> Vec<T>;
}

/// Mutable shared stream whose producer can emit items.
pub trait MutableSharedStream<T>: SharedStream<T>
where
    T: Clone,
{
    /// Emits one item to active collectors and stores it in replay when configured.
    fn emit(&mut self, value: T);

    /// Attempts to emit one item, returning false when the stream is closed.
    fn try_emit(&mut self, value: T) -> bool;

    /// Clears retained replay items without closing the stream.
    fn reset_replay_cache(&mut self);
}

/// Shared stream with a current value.
pub trait StateStream<T>: SharedStream<T>
where
    T: Clone,
{
    /// Returns the latest value.
    fn value(&self) -> T;
}

/// Mutable state stream.
pub trait MutableStateStream<T>: StateStream<T> + MutableSharedStream<T>
where
    T: Clone + PartialEq,
{
    /// Replaces the latest value and emits it when it changed.
    fn set_value(&mut self, value: T);

    /// Replaces the latest value only when it currently equals `expect`.
    fn compare_and_set(&mut self, expect: T, update: T) -> bool;
}

/// Controls when a shared stream starts collecting from its upstream source.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StreamStart {
    /// Start upstream collection immediately.
    Eagerly,
    /// Start upstream collection when the runtime explicitly decides to do so.
    Lazily,
}

/// Hot shared stream implementation used by AI response streams and event channels.
///
/// Collection blocks until `close` is called or all senders disconnect. Closing
/// this stream is a producer-side action: it ends every active collector and
/// rejects later emits.
#[derive(Debug, Clone)]
pub struct MutableSharedStreamImpl<T>
where
    T: Clone,
{
    inner: Arc<Mutex<MutableSharedStreamState<T>>>,
}

#[derive(Debug)]
struct MutableSharedStreamState<T>
where
    T: Clone,
{
    replay_limit: usize,
    replay_buffer: VecDeque<T>,
    subscribers: Vec<(usize, Sender<SharedEvent<T>>)>,
    subscription_count: usize,
    next_subscriber_id: usize,
    closed: bool,
}

#[derive(Debug, Clone)]
enum SharedEvent<T>
where
    T: Clone,
{
    Value(T),
    Close,
}

impl<T> MutableSharedStreamImpl<T>
where
    T: Clone,
{
    /// Creates a shared stream retaining at most `replay` emitted items.
    pub fn new(replay: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MutableSharedStreamState {
                replay_limit: replay,
                replay_buffer: VecDeque::new(),
                subscribers: Vec::new(),
                subscription_count: 0,
                next_subscriber_id: 0,
                closed: false,
            })),
        }
    }

    /// Closes the producer side of this stream.
    ///
    /// All active collectors receive a close event and return from `collect`.
    /// Later calls to `emit` are ignored and `try_emit` returns false.
    pub fn close(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            if guard.closed {
                return;
            }
            guard.closed = true;
            let subscribers = guard.subscribers.clone();
            drop(guard);
            for (_, sender) in subscribers {
                let _ = sender.send(SharedEvent::Close);
            }
        }
    }

    fn append_to_replay_buffer(state: &mut MutableSharedStreamState<T>, value: T) {
        if state.replay_limit == 0 {
            return;
        }
        state.replay_buffer.push_back(value);
        while state.replay_buffer.len() > state.replay_limit {
            state.replay_buffer.pop_front();
        }
    }

    /// Emits one value to active collectors.
    pub fn emit(&self, value: T) {
        if let Ok(mut guard) = self.inner.lock() {
            if guard.closed {
                return;
            }
            Self::append_to_replay_buffer(&mut guard, value.clone());
            let subscribers = guard.subscribers.clone();
            drop(guard);
            for (_, sender) in subscribers {
                let _ = sender.send(SharedEvent::Value(value.clone()));
            }
        }
    }

    /// Attempts to emit one value unless the stream is already closed.
    pub fn try_emit(&self, value: T) -> bool {
        if let Ok(guard) = self.inner.lock() {
            if guard.closed {
                return false;
            }
        }
        self.emit(value);
        true
    }

    /// Clears retained replay items without notifying collectors.
    pub fn reset_replay_cache(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.replay_buffer.clear();
        }
    }

    /// Returns a snapshot of the replay buffer.
    pub fn replay_cache(&self) -> Vec<T> {
        self.inner
            .lock()
            .map(|guard| guard.replay_buffer.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns the number of active collectors.
    pub fn subscription_count(&self) -> usize {
        self.inner
            .lock()
            .map(|guard| guard.subscription_count)
            .unwrap_or(0)
    }
}

impl<T> Stream for MutableSharedStreamImpl<T>
where
    T: Clone,
{
    type Item = T;

    /// Subscribes to this shared stream and blocks until `close` is observed.
    ///
    /// The collector first receives a replay snapshot, then live values. When the
    /// producer closes the stream, this collector is removed from the subscriber
    /// list and the method returns.
    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        let (subscriber_id, receiver, replay_snapshot, closed_immediately) = match self.inner.lock()
        {
            Ok(mut guard) => {
                let replay_snapshot = guard.replay_buffer.iter().cloned().collect::<Vec<_>>();
                if guard.closed {
                    (None, None, replay_snapshot, true)
                } else {
                    let (tx, rx) = channel::<SharedEvent<T>>();
                    let subscriber_id = guard.next_subscriber_id;
                    guard.next_subscriber_id += 1;
                    guard.subscribers.push((subscriber_id, tx));
                    guard.subscription_count = guard.subscribers.len();
                    (Some(subscriber_id), Some(rx), replay_snapshot, false)
                }
            }
            Err(_) => return,
        };

        for value in replay_snapshot {
            collector(value);
        }

        if closed_immediately {
            return;
        }

        if let Some(receiver) = receiver {
            while let Ok(event) = receiver.recv() {
                match event {
                    SharedEvent::Value(value) => collector(value),
                    SharedEvent::Close => break,
                }
            }
        }

        if let Some(subscriber_id) = subscriber_id {
            if let Ok(mut guard) = self.inner.lock() {
                guard.subscribers.retain(|(id, _)| *id != subscriber_id);
                guard.subscription_count = guard.subscribers.len();
            }
        }
    }
}

impl<T> SharedStream<T> for MutableSharedStreamImpl<T>
where
    T: Clone,
{
    fn subscription_count(&self) -> usize {
        MutableSharedStreamImpl::subscription_count(self)
    }

    fn replay_cache(&self) -> Vec<T> {
        MutableSharedStreamImpl::replay_cache(self)
    }
}

impl<T> MutableSharedStream<T> for MutableSharedStreamImpl<T>
where
    T: Clone,
{
    fn emit(&mut self, value: T) {
        MutableSharedStreamImpl::emit(self, value);
    }

    fn try_emit(&mut self, value: T) -> bool {
        MutableSharedStreamImpl::try_emit(self, value)
    }

    fn reset_replay_cache(&mut self) {
        MutableSharedStreamImpl::reset_replay_cache(self);
    }
}

#[derive(Debug, Clone)]
pub struct MutableStateStreamImpl<T>
where
    T: Clone,
{
    current: Arc<Mutex<T>>,
    shared: MutableSharedStreamImpl<T>,
}

impl<T> MutableStateStreamImpl<T>
where
    T: Clone,
{
    pub fn new(initial_value: T) -> Self {
        let mut shared = MutableSharedStreamImpl::new(1);
        shared.emit(initial_value.clone());
        Self {
            current: Arc::new(Mutex::new(initial_value)),
            shared,
        }
    }
}

impl<T> Stream for MutableStateStreamImpl<T>
where
    T: Clone,
{
    type Item = T;

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.shared.collect(collector);
    }
}

impl<T> SharedStream<T> for MutableStateStreamImpl<T>
where
    T: Clone,
{
    fn subscription_count(&self) -> usize {
        self.shared.subscription_count()
    }

    fn replay_cache(&self) -> Vec<T> {
        self.shared.replay_cache()
    }
}

impl<T> MutableSharedStream<T> for MutableStateStreamImpl<T>
where
    T: Clone,
{
    fn emit(&mut self, value: T) {
        if let Ok(mut current) = self.current.lock() {
            *current = value.clone();
        }
        self.shared.emit(value);
    }

    fn try_emit(&mut self, value: T) -> bool {
        if let Ok(mut current) = self.current.lock() {
            *current = value.clone();
        }
        self.shared.try_emit(value)
    }

    fn reset_replay_cache(&mut self) {}
}

impl<T> StateStream<T> for MutableStateStreamImpl<T>
where
    T: Clone,
{
    fn value(&self) -> T {
        self.current
            .lock()
            .map(|current| current.clone())
            .unwrap_or_else(|_| {
                self.shared
                    .replay_cache()
                    .last()
                    .cloned()
                    .expect("state stream must have value")
            })
    }
}

impl<T> MutableStateStream<T> for MutableStateStreamImpl<T>
where
    T: Clone + PartialEq,
{
    fn set_value(&mut self, value: T) {
        self.emit(value);
    }

    fn compare_and_set(&mut self, expect: T, update: T) -> bool {
        let matches = self
            .current
            .lock()
            .map(|current| *current == expect)
            .unwrap_or(false);
        if !matches {
            return false;
        }
        self.set_value(update);
        true
    }
}

pub fn mutable_shared_stream<T>(replay: usize) -> MutableSharedStreamImpl<T>
where
    T: Clone,
{
    MutableSharedStreamImpl::new(replay)
}

pub fn mutable_state_stream<T>(initial_value: T) -> MutableStateStreamImpl<T>
where
    T: Clone,
{
    MutableStateStreamImpl::new(initial_value)
}

pub fn share<S>(stream: S, replay: usize, started: StreamStart) -> MutableSharedStreamImpl<S::Item>
where
    S: Stream + Send + 'static,
    S::Item: Clone + Send + 'static,
{
    let shared = MutableSharedStreamImpl::new(replay);
    scheduleSharedStreamCollection(Arc::new(Mutex::new(Some(stream))), shared.clone(), started);
    shared
}

pub fn state<S>(
    stream: S,
    initial_value: S::Item,
    started: StreamStart,
) -> MutableStateStreamImpl<S::Item>
where
    S: Stream + Send + 'static,
    S::Item: Clone + PartialEq + Send + 'static,
{
    let state_stream = MutableStateStreamImpl::new(initial_value);
    scheduleStateStreamCollection(
        Arc::new(Mutex::new(Some(stream))),
        state_stream.clone(),
        started,
    );
    state_stream
}

/// Schedules collection into a shared stream through the configured host task scheduler.
fn scheduleSharedStreamCollection<S>(
    source: Arc<Mutex<Option<S>>>,
    shared: MutableSharedStreamImpl<S::Item>,
    started: StreamStart,
) where
    S: Stream + Send + 'static,
    S::Item: Clone + Send + 'static,
{
    let retrySource = source.clone();
    let retryShared = shared.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeTask(
            "operit-shared-stream-collection",
            Box::new(move || {
                if matches!(started, StreamStart::Lazily) && retryShared.subscription_count() == 0 {
                    scheduleSharedStreamCollectionAfterDelay(retrySource, retryShared, started);
                    return;
                }
                let mut stream = retrySource
                    .lock()
                    .expect("shared stream source mutex poisoned")
                    .take()
                    .expect("shared stream collection must start exactly once");
                stream.collect(&mut |value| retryShared.emit(value));
                retryShared.close();
            }),
        )
        .expect("shared stream collection task must be scheduled");
}

/// Re-enqueues lazy shared-stream collection after a host-owned short delay.
fn scheduleSharedStreamCollectionAfterDelay<S>(
    source: Arc<Mutex<Option<S>>>,
    shared: MutableSharedStreamImpl<S::Item>,
    started: StreamStart,
) where
    S: Stream + Send + 'static,
    S::Item: Clone + Send + 'static,
{
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleDelayedHostRuntimeTask(
            "operit-shared-stream-lazy-collection",
            10,
            Box::new(move || scheduleSharedStreamCollection(source, shared, started)),
        )
        .expect("lazy shared stream collection task must be scheduled");
}

/// Schedules collection into a state stream through the configured host task scheduler.
fn scheduleStateStreamCollection<S>(
    source: Arc<Mutex<Option<S>>>,
    stateStream: MutableStateStreamImpl<S::Item>,
    started: StreamStart,
) where
    S: Stream + Send + 'static,
    S::Item: Clone + PartialEq + Send + 'static,
{
    let retrySource = source.clone();
    let retryStateStream = stateStream.clone();
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleHostRuntimeTask(
            "operit-state-stream-collection",
            Box::new(move || {
                if matches!(started, StreamStart::Lazily)
                    && retryStateStream.subscription_count() == 0
                {
                    scheduleStateStreamCollectionAfterDelay(retrySource, retryStateStream, started);
                    return;
                }
                let mut stream = retrySource
                    .lock()
                    .expect("state stream source mutex poisoned")
                    .take()
                    .expect("state stream collection must start exactly once");
                let mut target = retryStateStream.clone();
                stream.collect(&mut |value| target.set_value(value));
            }),
        )
        .expect("state stream collection task must be scheduled");
}

/// Re-enqueues lazy state-stream collection after a host-owned short delay.
fn scheduleStateStreamCollectionAfterDelay<S>(
    source: Arc<Mutex<Option<S>>>,
    stateStream: MutableStateStreamImpl<S::Item>,
    started: StreamStart,
) where
    S: Stream + Send + 'static,
    S::Item: Clone + PartialEq + Send + 'static,
{
    defaultHostRuntimeTaskSchedulerHost()
        .scheduleDelayedHostRuntimeTask(
            "operit-state-stream-lazy-collection",
            10,
            Box::new(move || scheduleStateStreamCollection(source, stateStream, started)),
        )
        .expect("lazy state stream collection task must be scheduled");
}
