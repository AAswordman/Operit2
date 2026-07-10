use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::AppLogger::AppLogger;

pub struct StreamLogger;

impl StreamLogger {
    /// Enables or disables stream diagnostics.
    pub fn set_enabled(enabled: bool) {
        ENABLED.store(enabled, Ordering::Relaxed);
    }

    /// Enables or disables verbose stream diagnostics.
    pub fn set_verbose_enabled(enabled: bool) {
        VERBOSE_ENABLED.store(enabled, Ordering::Relaxed);
    }

    /// Writes a debug-level stream diagnostic.
    pub fn d(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) {
            AppLogger::d("StreamFramework", &format!("[{component}] {message}"));
        }
    }

    /// Writes an info-level stream diagnostic.
    pub fn i(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) {
            AppLogger::i("StreamFramework", &format!("[{component}] {message}"));
        }
    }

    /// Writes a verbose stream diagnostic.
    pub fn v(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) && VERBOSE_ENABLED.load(Ordering::Relaxed) {
            AppLogger::v("StreamFramework", &format!("[{component}] {message}"));
        }
    }

    /// Writes a warning-level stream diagnostic.
    pub fn w(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) {
            AppLogger::w("StreamFramework", &format!("[{component}] {message}"));
        }
    }

    /// Writes an error-level stream diagnostic.
    pub fn e(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) {
            AppLogger::e("StreamFramework", &format!("[{component}] {message}"));
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static VERBOSE_ENABLED: AtomicBool = AtomicBool::new(false);

/// Pull-style runtime stream.
///
/// A `Stream` represents a producer of ordered items. Unlike DataStore `Flow`,
/// cancellation is normally owned by the producer: a provider/service stops
/// producing, closes a shared stream, or returns from `collect`. Dropping a UI
/// watch subscription should not by itself be interpreted as cancelling the
/// upstream operation.
pub trait Stream {
    type Item;

    /// Returns whether this stream is currently buffering emitted items.
    fn is_locked(&self) -> bool {
        false
    }

    /// Returns the number of items held in this stream's local buffer.
    fn buffered_count(&self) -> usize {
        0
    }

    /// Starts local buffering for stream implementations that support it.
    fn lock(&mut self) {}

    /// Stops local buffering and allows later collection to emit normally.
    fn unlock(&mut self) {}

    /// Drops locally buffered items without affecting the upstream producer.
    fn clear_buffer(&mut self) {}

    /// Collects items from this stream until the producer finishes or closes.
    ///
    /// Implementations may block while waiting for producer events. A caller that
    /// needs cancellation must cancel/close the producer-side object rather than
    /// assuming the collector can be interrupted externally.
    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item));
}

impl<S> Stream for Box<S>
where
    S: ?Sized + Stream,
{
    type Item = S::Item;

    fn is_locked(&self) -> bool {
        (**self).is_locked()
    }

    fn buffered_count(&self) -> usize {
        (**self).buffered_count()
    }

    fn lock(&mut self) {
        (**self).lock();
    }

    fn unlock(&mut self) {
        (**self).unlock();
    }

    fn clear_buffer(&mut self) {
        (**self).clear_buffer();
    }

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        (**self).collect(collector);
    }
}

/// Minimal emitter interface used by stream builders and adapters.
pub trait StreamCollector<T> {
    /// Emits one item into the downstream collector.
    fn emit(&mut self, value: T);
}

impl<T, F> StreamCollector<T> for F
where
    F: FnMut(T),
{
    fn emit(&mut self, value: T) {
        self(value);
    }
}

/// Finite stream backed by an in-memory queue.
pub struct VecStream<T> {
    values: VecDeque<T>,
    locked: bool,
    buffer: VecDeque<T>,
    closed: bool,
}

impl<T> VecStream<T> {
    /// Creates a finite stream that emits `values` in iteration order.
    pub fn new(values: impl IntoIterator<Item = T>) -> Self {
        Self {
            values: values.into_iter().collect(),
            locked: false,
            buffer: VecDeque::new(),
            closed: false,
        }
    }

    fn try_buffer(&mut self, value: T) -> Result<(), T> {
        if self.locked && !self.closed {
            self.buffer.push_back(value);
            Ok(())
        } else {
            Err(value)
        }
    }
}

impl<T> Stream for VecStream<T> {
    type Item = T;

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn buffered_count(&self) -> usize {
        self.buffer.len()
    }

    fn lock(&mut self) {
        if !self.closed {
            self.locked = true;
        }
    }

    fn unlock(&mut self) {
        self.locked = false;
    }

    fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        while let Some(value) = self.buffer.pop_front() {
            collector(value);
        }
        while let Some(value) = self.values.pop_front() {
            match self.try_buffer(value) {
                Ok(()) => {}
                Err(value) => collector(value),
            }
        }
        self.closed = true;
    }
}

/// Stream implemented by a callback that synchronously emits values.
pub struct FnStream<T> {
    block: Box<dyn FnMut(&mut dyn FnMut(T)) + Send>,
    locked: bool,
    buffer: VecDeque<T>,
    closed: bool,
}

impl<T> FnStream<T> {
    /// Creates a stream from a callback invoked during `collect`.
    pub fn new(block: impl FnMut(&mut dyn FnMut(T)) + Send + 'static) -> Self {
        Self {
            block: Box::new(block),
            locked: false,
            buffer: VecDeque::new(),
            closed: false,
        }
    }
}

impl<T> Stream for FnStream<T> {
    type Item = T;

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn buffered_count(&self) -> usize {
        self.buffer.len()
    }

    fn lock(&mut self) {
        if !self.closed {
            self.locked = true;
        }
    }

    fn unlock(&mut self) {
        self.locked = false;
    }

    fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        while let Some(value) = self.buffer.pop_front() {
            collector(value);
        }
        let locked = self.locked;
        let buffer = &mut self.buffer;
        (self.block)(&mut |value| {
            if locked {
                buffer.push_back(value);
            } else {
                collector(value);
            }
        });
        self.closed = true;
    }
}
