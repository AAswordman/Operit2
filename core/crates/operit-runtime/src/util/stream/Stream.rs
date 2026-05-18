use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::util::AppLogger::AppLogger;

pub struct StreamLogger;

impl StreamLogger {
    pub fn set_enabled(enabled: bool) {
        ENABLED.store(enabled, Ordering::Relaxed);
    }

    pub fn set_verbose_enabled(enabled: bool) {
        VERBOSE_ENABLED.store(enabled, Ordering::Relaxed);
    }

    pub fn d(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) {
            AppLogger::d("StreamFramework", &format!("[{component}] {message}"));
        }
    }

    pub fn i(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) {
            AppLogger::i("StreamFramework", &format!("[{component}] {message}"));
        }
    }

    pub fn v(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) && VERBOSE_ENABLED.load(Ordering::Relaxed) {
            AppLogger::v("StreamFramework", &format!("[{component}] {message}"));
        }
    }

    pub fn w(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) {
            AppLogger::w("StreamFramework", &format!("[{component}] {message}"));
        }
    }

    pub fn e(component: &str, message: &str) {
        if ENABLED.load(Ordering::Relaxed) {
            AppLogger::e("StreamFramework", &format!("[{component}] {message}"));
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(true);
static VERBOSE_ENABLED: AtomicBool = AtomicBool::new(false);

pub trait Stream {
    type Item;

    fn is_locked(&self) -> bool {
        false
    }

    fn buffered_count(&self) -> usize {
        0
    }

    fn lock(&mut self) {}

    fn unlock(&mut self) {}

    fn clear_buffer(&mut self) {}

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item));
}

pub trait StreamCollector<T> {
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

pub struct VecStream<T> {
    values: VecDeque<T>,
    locked: bool,
    buffer: VecDeque<T>,
    closed: bool,
}

impl<T> VecStream<T> {
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

pub struct FnStream<T> {
    block: Box<dyn FnMut(&mut dyn FnMut(T))>,
    locked: bool,
    buffer: VecDeque<T>,
    closed: bool,
}

impl<T> FnStream<T> {
    pub fn new(block: impl FnMut(&mut dyn FnMut(T)) + 'static) -> Self {
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
