use std::collections::VecDeque;

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
    replay_limit: usize,
    replay_buffer: VecDeque<T>,
    pending: VecDeque<T>,
    subscription_count: usize,
    closed: bool,
}

impl<T> MutableSharedStreamImpl<T>
where
    T: Clone,
{
    pub fn new(replay: usize) -> Self {
        Self {
            replay_limit: replay,
            replay_buffer: VecDeque::new(),
            pending: VecDeque::new(),
            subscription_count: 0,
            closed: false,
        }
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    fn append_to_replay_buffer(&mut self, value: T) {
        if self.replay_limit == 0 {
            return;
        }
        self.replay_buffer.push_back(value);
        while self.replay_buffer.len() > self.replay_limit {
            self.replay_buffer.pop_front();
        }
    }
}

impl<T> Stream for MutableSharedStreamImpl<T>
where
    T: Clone,
{
    type Item = T;

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.subscription_count += 1;
        for value in self.replay_buffer.iter().cloned() {
            collector(value);
        }
        while let Some(value) = self.pending.pop_front() {
            collector(value);
        }
        self.subscription_count = self.subscription_count.saturating_sub(1);
    }
}

impl<T> SharedStream<T> for MutableSharedStreamImpl<T>
where
    T: Clone,
{
    fn subscription_count(&self) -> usize {
        self.subscription_count
    }

    fn replay_cache(&self) -> Vec<T> {
        self.replay_buffer.iter().cloned().collect()
    }
}

impl<T> MutableSharedStream<T> for MutableSharedStreamImpl<T>
where
    T: Clone,
{
    fn emit(&mut self, value: T) {
        if !self.closed {
            self.append_to_replay_buffer(value.clone());
            self.pending.push_back(value);
        }
    }

    fn try_emit(&mut self, value: T) -> bool {
        if self.closed {
            return false;
        }
        self.emit(value);
        true
    }

    fn reset_replay_cache(&mut self) {
        self.replay_buffer.clear();
    }
}

#[derive(Debug, Clone)]
pub struct MutableStateStreamImpl<T>
where
    T: Clone,
{
    current: T,
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
            current: initial_value,
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
        self.current = value.clone();
        self.shared.emit(value);
    }

    fn try_emit(&mut self, value: T) -> bool {
        self.current = value.clone();
        self.shared.try_emit(value)
    }

    fn reset_replay_cache(&mut self) {}
}

impl<T> StateStream<T> for MutableStateStreamImpl<T>
where
    T: Clone,
{
    fn value(&self) -> T {
        self.current.clone()
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
        if self.current == expect {
            self.set_value(update);
            true
        } else {
            false
        }
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

pub fn share<S>(mut stream: S, replay: usize, _started: StreamStart) -> MutableSharedStreamImpl<S::Item>
where
    S: Stream,
    S::Item: Clone,
{
    let mut shared = MutableSharedStreamImpl::new(replay);
    stream.collect(&mut |value| shared.emit(value));
    shared.close();
    shared
}

pub fn state<S>(mut stream: S, initial_value: S::Item, _started: StreamStart) -> MutableStateStreamImpl<S::Item>
where
    S: Stream,
    S::Item: Clone + PartialEq,
{
    let mut state_stream = MutableStateStreamImpl::new(initial_value);
    stream.collect(&mut |value| state_stream.set_value(value));
    state_stream
}
