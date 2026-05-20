use std::collections::VecDeque;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::util::stream::Stream::Stream;

pub trait SharedStream<T>: Stream<Item = T>
where
    T: Clone,
{
    fn subscription_count(&self) -> usize;
    fn replay_cache(&self) -> Vec<T>;
}

pub trait MutableSharedStream<T>: SharedStream<T>
where
    T: Clone,
{
    fn emit(&mut self, value: T);
    fn try_emit(&mut self, value: T) -> bool;
    fn reset_replay_cache(&mut self);
}

pub trait StateStream<T>: SharedStream<T>
where
    T: Clone,
{
    fn value(&self) -> T;
}

pub trait MutableStateStream<T>: StateStream<T> + MutableSharedStream<T>
where
    T: Clone + PartialEq,
{
    fn set_value(&mut self, value: T);
    fn compare_and_set(&mut self, expect: T, update: T) -> bool;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StreamStart {
    Eagerly,
    Lazily,
}

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

    pub fn try_emit(&self, value: T) -> bool {
        if let Ok(guard) = self.inner.lock() {
            if guard.closed {
                return false;
            }
        }
        self.emit(value);
        true
    }

    pub fn reset_replay_cache(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.replay_buffer.clear();
        }
    }

    pub fn replay_cache(&self) -> Vec<T> {
        self.inner
            .lock()
            .map(|guard| guard.replay_buffer.iter().cloned().collect())
            .unwrap_or_default()
    }

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

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        let (subscriber_id, receiver, replay_snapshot, closed_immediately) = match self.inner.lock() {
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
            .unwrap_or_else(|_| self.shared.replay_cache().last().cloned().expect("state stream must have value"))
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

pub fn share<S>(mut stream: S, replay: usize, started: StreamStart) -> MutableSharedStreamImpl<S::Item>
where
    S: Stream + Send + 'static,
    S::Item: Clone + Send + 'static,
{
    let shared = MutableSharedStreamImpl::new(replay);
    let shared_for_thread = shared.clone();
    thread::spawn(move || {
        if matches!(started, StreamStart::Lazily) {
            while shared_for_thread.subscription_count() == 0 {
                thread::sleep(Duration::from_millis(10));
            }
        }
        stream.collect(&mut |value| shared_for_thread.emit(value));
        shared_for_thread.close();
    });
    shared
}

pub fn state<S>(mut stream: S, initial_value: S::Item, started: StreamStart) -> MutableStateStreamImpl<S::Item>
where
    S: Stream + Send + 'static,
    S::Item: Clone + PartialEq + Send + 'static,
{
    let state_stream = MutableStateStreamImpl::new(initial_value);
    let mut state_stream_for_thread = state_stream.clone();
    thread::spawn(move || {
        if matches!(started, StreamStart::Lazily) {
            while state_stream_for_thread.subscription_count() == 0 {
                thread::sleep(Duration::from_millis(10));
            }
        }
        stream.collect(&mut |value| state_stream_for_thread.set_value(value));
    });
    state_stream
}
