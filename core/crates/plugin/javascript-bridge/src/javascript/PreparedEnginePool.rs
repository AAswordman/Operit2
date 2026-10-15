use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use operit_host_api::HostRuntimeTask;
use tokio::sync::oneshot;

const READY_ENGINE_COUNT: usize = 2;
type Schedule = Arc<dyn Fn(HostRuntimeTask) -> Result<(), String> + Send + Sync>;
type Create<T> = Arc<dyn Fn() -> Result<T, String> + Send + Sync>;

/// Keeps two pristine resources ready using bounded Host-scheduled creation tasks.
pub(super) struct PreparedEnginePool<T> {
    state: Mutex<PoolState<T>>,
    create: Create<T>,
    schedule: Schedule,
}

struct PoolState<T> {
    ready: VecDeque<T>,
    creating: usize,
    waiters: VecDeque<oneshot::Sender<Result<T, String>>>,
    error: Option<String>,
}

impl<T: Send + 'static> PreparedEnginePool<T> {
    /// Starts bounded preparation immediately without binding any package owner.
    pub(super) fn new(create: Create<T>, schedule: Schedule) -> Arc<Self> {
        let pool = Arc::new(Self {
            state: Mutex::new(PoolState {
                ready: VecDeque::new(),
                creating: 0,
                waiters: VecDeque::new(),
                error: None,
            }),
            create,
            schedule,
        });
        pool.replenish();
        pool
    }

    /// Takes one ready resource or queues a cancellable asynchronous request.
    pub(super) fn request(self: &Arc<Self>) -> oneshot::Receiver<Result<T, String>> {
        let (sender, receiver) = oneshot::channel();
        let immediate = {
            let mut state = self
                .state
                .lock()
                .expect("Prepared engine pool mutex poisoned");
            if let Some(error) = &state.error {
                Some((sender, Err(error.clone())))
            } else if let Some(engine) = state.ready.pop_front() {
                Some((sender, Ok(engine)))
            } else {
                state.waiters.push_back(sender);
                None
            }
        };
        if let Some((sender, result)) = immediate {
            // A dropped receiver owns and destroys any undelivered resource.
            let _ = sender.send(result);
        }
        self.replenish();
        receiver
    }

    /// Reserves creation slots before scheduling, so simultaneous requests cannot overfill.
    fn replenish(self: &Arc<Self>) {
        let slots = {
            let mut state = self
                .state
                .lock()
                .expect("Prepared engine pool mutex poisoned");
            if state.error.is_some() {
                return;
            }
            let slots = READY_ENGINE_COUNT - state.ready.len() - state.creating;
            state.creating += slots;
            slots
        };
        for _ in 0..slots {
            let weak = Arc::downgrade(self);
            let create = self.create.clone();
            let scheduled = (self.schedule)(Box::new(move || {
                // The provider may have shut down before this Host task started.
                if let Some(pool) = weak.upgrade() {
                    pool.complete(create());
                }
            }));
            if let Err(error) = scheduled {
                self.complete(Err(error));
            }
        }
    }

    /// Delivers a prepared resource in FIFO order and replenishes after every consumption.
    fn complete(self: &Arc<Self>, result: Result<T, String>) {
        let mut state = self
            .state
            .lock()
            .expect("Prepared engine pool mutex poisoned");
        state.creating -= 1;
        if state.error.is_some() {
            drop(state);
            drop(result);
            return;
        }
        match result {
            Err(error) => {
                state.error = Some(error.clone());
                let waiters = std::mem::take(&mut state.waiters);
                let ready = std::mem::take(&mut state.ready);
                drop(state);
                // Native runtime destruction must never execute under the pool lock.
                drop(ready);
                for waiter in waiters {
                    let _ = waiter.send(Err(error.clone()));
                }
            }
            Ok(mut engine) => {
                // Cancelled waiters do not consume a resource or a refill slot.
                while let Some(waiter) = state.waiters.pop_front() {
                    match waiter.send(Ok(engine)) {
                        Ok(()) => {
                            drop(state);
                            self.replenish();
                            return;
                        }
                        Err(Ok(returned)) => engine = returned,
                        Err(Err(_)) => unreachable!("Only successful resources are delivered here"),
                    }
                }
                state.ready.push_back(engine);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture {
        pool: Arc<PreparedEnginePool<usize>>,
        tasks: Arc<Mutex<VecDeque<HostRuntimeTask>>>,
        creations: Arc<AtomicUsize>,
    }

    impl Fixture {
        /// Uses a deterministic task queue to exercise the production pool state machine.
        fn new() -> Self {
            let tasks = Arc::new(Mutex::new(VecDeque::<HostRuntimeTask>::new()));
            let creations = Arc::new(AtomicUsize::new(0));
            let queue = tasks.clone();
            let count = creations.clone();
            let pool = PreparedEnginePool::new(
                Arc::new(move || Ok(count.fetch_add(1, Ordering::SeqCst))),
                Arc::new(move |task| {
                    queue.lock().unwrap().push_back(task);
                    Ok(())
                }),
            );
            Self {
                pool,
                tasks,
                creations,
            }
        }

        /// Advances a single scheduled creation without sleeping or starting another executor.
        fn run(&self) {
            let task = self
                .tasks
                .lock()
                .unwrap()
                .pop_front()
                .expect("Expected a scheduled preparation");
            task();
        }
    }

    /// Startup prepares two engines and consuming one schedules exactly one replacement.
    #[test]
    fn prepares_two_and_replenishes_after_consumption() {
        let fixture = Fixture::new();
        assert_eq!(fixture.tasks.lock().unwrap().len(), 2);
        fixture.run();
        fixture.run();
        let mut request = fixture.pool.request();
        assert_eq!(request.try_recv().unwrap().unwrap(), 0);
        assert_eq!(fixture.tasks.lock().unwrap().len(), 1);
        fixture.run();
        let state = fixture.pool.state.lock().unwrap();
        assert_eq!(state.ready.len(), 2);
        assert_eq!(state.creating, 0);
    }

    /// Burst requests await bounded preparation in order, including a cancelled waiter.
    #[test]
    fn burst_waits_without_overfilling_and_skips_cancelled_waiters() {
        let fixture = Fixture::new();
        let cancelled = fixture.pool.request();
        let mut first = fixture.pool.request();
        let mut second = fixture.pool.request();
        drop(cancelled);
        assert_eq!(fixture.tasks.lock().unwrap().len(), 2);
        fixture.run();
        fixture.run();
        assert_eq!(first.try_recv().unwrap().unwrap(), 0);
        assert_eq!(second.try_recv().unwrap().unwrap(), 1);
        fixture.run();
        fixture.run();
        assert_eq!(fixture.creations.load(Ordering::SeqCst), 4);
        assert_eq!(fixture.pool.state.lock().unwrap().ready.len(), 2);
    }

    /// Preparation failures reject current and future requests without hidden retries.
    #[test]
    fn preparation_error_is_retained_and_reported() {
        let tasks = Arc::new(Mutex::new(VecDeque::<HostRuntimeTask>::new()));
        let queue = tasks.clone();
        let pool = PreparedEnginePool::<usize>::new(
            Arc::new(|| Err("bootstrap failed".into())),
            Arc::new(move |task| {
                queue.lock().unwrap().push_back(task);
                Ok(())
            }),
        );
        let mut waiting = pool.request();
        let task = tasks.lock().unwrap().pop_front().unwrap();
        task();
        assert_eq!(waiting.try_recv().unwrap().unwrap_err(), "bootstrap failed");
        assert_eq!(
            pool.request().try_recv().unwrap().unwrap_err(),
            "bootstrap failed"
        );
        let task = tasks.lock().unwrap().pop_front().unwrap();
        task();
        assert!(tasks.lock().unwrap().is_empty());
        assert_eq!(pool.state.lock().unwrap().creating, 0);
    }

    /// Host scheduling errors are surfaced before any request can become stranded.
    #[test]
    fn scheduling_error_rejects_acquisition() {
        let pool = PreparedEnginePool::<usize>::new(
            Arc::new(|| Ok(0)),
            Arc::new(|_| Err("scheduler stopped".into())),
        );
        assert_eq!(
            pool.request().try_recv().unwrap().unwrap_err(),
            "scheduler stopped"
        );
        assert_eq!(pool.state.lock().unwrap().creating, 0);
    }

    struct DropProbe(Arc<AtomicUsize>);

    impl Drop for DropProbe {
        /// Counts release of an owned pristine resource without starting a native runtime.
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Cancelled ready delivery and provider shutdown both destroy owned resources.
    #[test]
    fn cancellation_and_shutdown_release_ready_resources() {
        let tasks = Arc::new(Mutex::new(VecDeque::<HostRuntimeTask>::new()));
        let destroyed = Arc::new(AtomicUsize::new(0));
        let queue = tasks.clone();
        let drops = destroyed.clone();
        let pool = PreparedEnginePool::new(
            Arc::new(move || Ok(DropProbe(drops.clone()))),
            Arc::new(move |task| {
                queue.lock().unwrap().push_back(task);
                Ok(())
            }),
        );
        for _ in 0..2 {
            let task = tasks.lock().unwrap().pop_front().unwrap();
            task();
        }
        drop(pool.request());
        assert_eq!(destroyed.load(Ordering::SeqCst), 1);
        drop(pool);
        assert_eq!(destroyed.load(Ordering::SeqCst), 2);
        let task = tasks.lock().unwrap().pop_front().unwrap();
        task();
        assert_eq!(destroyed.load(Ordering::SeqCst), 2);
    }

    /// Provider shutdown releases ready resources and queued tasks do not create new ones.
    #[test]
    fn shutdown_cancels_unstarted_preparation() {
        let fixture = Fixture::new();
        let Fixture {
            pool,
            tasks,
            creations,
        } = fixture;
        drop(pool);
        while let Some(task) = { tasks.lock().unwrap().pop_front() } {
            task();
        }
        assert_eq!(creations.load(Ordering::SeqCst), 0);
    }
}
