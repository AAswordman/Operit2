use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
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

/// Future returned by an asynchronous stream collection.
pub type CollectFuture<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

/// Future returned by a cold stream producer on its owning executor.
pub type ProducerFuture<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

/// Pull-style asynchronous runtime stream.
///
/// A `Stream` represents a producer of ordered items. Cancellation is normally
/// owned by the producer, while dropping a collection future only stops that
/// collector.
pub trait Stream {
    type Item: Send;

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
    /// Asynchronously collects items until the producer finishes or closes.
    fn collect<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(Self::Item),
    ) -> CollectFuture<'a>;
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

    fn collect<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(Self::Item),
    ) -> CollectFuture<'a> {
        (**self).collect(collector)
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

impl<T> Stream for VecStream<T>
where
    T: Send,
{
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

    fn collect<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(Self::Item),
    ) -> CollectFuture<'a> {
        Box::pin(async move {
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
        })
    }
}

/// Cold stream implemented by an asynchronous producer callback.
pub struct FnStream<T> {
    block:
        Option<Box<dyn for<'a> FnMut(&'a mut dyn FnMut(T)) -> ProducerFuture<'a>>>,
    locked: bool,
    buffer: VecDeque<T>,
    closed: bool,
}

impl<T> FnStream<T> {
    /// Creates a cold stream whose producer runs during asynchronous collection.
    pub fn new(
        block: impl for<'a> FnMut(&'a mut dyn FnMut(T)) -> ProducerFuture<'a> + 'static,
    ) -> Self {
        Self {
            block: Some(Box::new(block)),
            locked: false,
            buffer: VecDeque::new(),
            closed: false,
        }
    }
}

impl<T> Stream for FnStream<T>
where
    T: Send + 'static,
{
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

    fn collect<'a>(
        &'a mut self,
        collector: &'a mut dyn FnMut(Self::Item),
    ) -> CollectFuture<'a> {
        Box::pin(async move {
            while let Some(value) = self.buffer.pop_front() {
                collector(value);
            }
            let locked = self.locked;
            let buffer = &mut self.buffer;
            let mut block = self
                .block
                .take()
                .expect("FnStream producer must only be collected once");
            let mut emit = |value| {
                if locked {
                    buffer.push_back(value);
                } else {
                    collector(value);
                }
            };
            block(&mut emit).await;
            self.closed = true;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{FnStream, Stream};
    use operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost;
    use operit_host_api::{
        HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Once};

    /// Schedules test tasks on isolated current-thread Tokio runtimes.
    #[derive(Clone, Copy, Debug, Default)]
    struct TestHostRuntimeTaskScheduler;

    impl HostRuntimeTaskSchedulerHost for TestHostRuntimeTaskScheduler {
        /// Starts a synchronous test task on an isolated thread.
        fn scheduleHostRuntimeTask(
            &self,
            _task_name: &str,
            task: HostRuntimeTask,
        ) -> HostResult<()> {
            std::thread::spawn(task);
            Ok(())
        }

        /// Starts an asynchronous test task on an isolated current-thread runtime.
        fn scheduleHostRuntimeAsyncTask(
            &self,
            _task_name: &str,
            task: HostRuntimeAsyncTask,
        ) -> HostResult<()> {
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("create test stream task runtime failed");
                runtime.block_on(task());
            });
            Ok(())
        }

        /// Starts a delayed test task after the requested interval.
        fn scheduleDelayedHostRuntimeTask(
            &self,
            _task_name: &str,
            delay_ms: u64,
            task: HostRuntimeTask,
        ) -> HostResult<()> {
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                task();
            });
            Ok(())
        }
    }

    /// Installs the stream test scheduler once for this process.
    fn install_test_host_runtime_task_scheduler() {
        static INITIALIZED: Once = Once::new();
        INITIALIZED.call_once(|| {
            setDefaultHostRuntimeTaskSchedulerHost(Arc::new(TestHostRuntimeTaskScheduler));
        });
    }

    /// Verifies cold producer work starts only after asynchronous collection begins.
    #[tokio::test]
    async fn fn_stream_starts_producer_during_collection() {
        install_test_host_runtime_task_scheduler();
        let started = Arc::new(AtomicBool::new(false));
        let producerStarted = started.clone();
        let mut stream = FnStream::new(move |emit| {
            producerStarted.store(true, Ordering::Release);
            emit("value".to_string());
            Box::pin(async {})
        });
        assert!(!started.load(Ordering::Acquire));

        let mut values = Vec::new();
        stream.collect(&mut |value| values.push(value)).await;

        assert!(started.load(Ordering::Acquire));
        assert_eq!(values, ["value"]);
    }
}
