use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

const RUNTIME_WORKER_COUNT: usize = 16;
const BLOCKING_WORKER_COUNT: usize = 16;
const NETWORK_WORKER_COUNT: usize = 4;
const SERVICE_WORKER_COUNT: usize = 16;
const TASK_QUEUE_CAPACITY: usize = 4096;

thread_local! {
    static HOST_RUNTIME_WORKER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Identifies workers whose native stack reservation is owned by this scheduler.
pub(crate) fn hasManagedWorkerStack() -> bool {
    HOST_RUNTIME_WORKER.get()
}

const RUNTIME_WORKER_PREFIX: &str = "operit-runtime-worker";
const BLOCKING_WORKER_PREFIX: &str = "operit-runtime-blocking-worker";

/// Owns the process-wide coordination and blocking pools.
///
/// Every pool hands the next task to whichever worker is free. A worker stuck
/// in a blocking call therefore stops taking new tasks instead of accumulating
/// a private backlog that later stalls unrelated callers.
struct RuntimeTaskPool {
    coordination: RuntimeTaskQueue,
    blocking: RuntimeTaskQueue,
}

/// Shares ready tasks without assigning the receive wait to any single worker.
pub(crate) struct RuntimeTaskQueue {
    pending: Arc<Mutex<TaskQueueState>>,
    changed: Arc<tokio::sync::watch::Sender<()>>,
}

struct TaskQueueState {
    pending: VecDeque<HostRuntimeAsyncTask>,
    active: Vec<usize>,
    polling: Vec<bool>,
    cursor: usize,
}

struct TrackedTask {
    future: std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>,
    state: Arc<Mutex<TaskQueueState>>,
    changed: std::sync::Weak<tokio::sync::watch::Sender<()>>,
    worker: usize,
}

struct PollLease<'a> {
    state: &'a Mutex<TaskQueueState>,
    changed: &'a std::sync::Weak<tokio::sync::watch::Sender<()>>,
    worker: usize,
}

impl Drop for PollLease<'_> {
    /// Publishes worker availability even when a user future panics during its poll.
    fn drop(&mut self) {
        self.state
            .lock()
            .expect("runtime task queue mutex poisoned")
            .polling[self.worker] = false;
        if let Some(changed) = self.changed.upgrade() {
            let _ = changed.send(());
        }
    }
}

impl std::future::Future for TrackedTask {
    type Output = ();
    /// Excludes a currently executing worker from assignments until its future yields.
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        let this = self.get_mut();
        this.state
            .lock()
            .expect("runtime task queue mutex poisoned")
            .polling[this.worker] = true;
        let _poll = PollLease {
            state: &this.state,
            changed: &this.changed,
            worker: this.worker,
        };
        if let Some(changed) = this.changed.upgrade() {
            let _ = changed.send(());
        }
        this.future.as_mut().poll(cx)
    }
}

impl Drop for TrackedTask {
    /// Releases load accounting after success, cancellation, or a factory panic.
    fn drop(&mut self) {
        self.state
            .lock()
            .expect("runtime task queue mutex poisoned")
            .active[self.worker] -= 1;
        if let Some(changed) = self.changed.upgrade() {
            let _ = changed.send(());
        }
    }
}

impl RuntimeTaskQueue {
    /// Wakes every dispatcher so a blocked worker cannot retain the only notification.
    pub(crate) fn send(&self, task: HostRuntimeAsyncTask) -> Result<(), ()> {
        self.sendMany(vec![task])
    }

    /// Publishes an entire admitted pipeline under one queue lock.
    fn sendMany(&self, tasks: Vec<HostRuntimeAsyncTask>) -> Result<(), ()> {
        let mut pending = self
            .pending
            .lock()
            .expect("runtime task queue mutex poisoned");
        if self.changed.is_closed()
            || tasks.len() > TASK_QUEUE_CAPACITY.saturating_sub(pending.pending.len())
        {
            return Err(());
        }
        pending.pending.extend(tasks);
        drop(pending);
        self.changed.send(()).map_err(|_| ())
    }
}

/// Creates one shared task queue consumed by the requested number of affine workers.
fn spawnTaskPool(prefix: &'static str, count: usize) -> Result<RuntimeTaskQueue, String> {
    spawnTaskPoolWithOptions(prefix, count, 16 * 1024 * 1024, true)
}

/// Creates bounded affine workers with the stack and I/O policy selected by the Host.
pub(crate) fn spawnTaskPoolWithOptions(
    prefix: &'static str,
    count: usize,
    stackSize: usize,
    enableIo: bool,
) -> Result<RuntimeTaskQueue, String> {
    let pending = Arc::new(Mutex::new(TaskQueueState {
        pending: VecDeque::new(),
        active: vec![0; count],
        polling: vec![false; count],
        cursor: 0,
    }));
    let (changed, updates) = tokio::sync::watch::channel(());
    let changed = Arc::new(changed);
    for index in 0..count {
        let pending = pending.clone();
        let mut updates = updates.clone();
        let changed = Arc::downgrade(&changed);
        let (ready, started) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name(format!("{prefix}-{index}"))
            .stack_size(stackSize)
            .spawn(move || {
                HOST_RUNTIME_WORKER.set(stackSize >= 16 * 1024 * 1024);
                let mut builder = tokio::runtime::Builder::new_current_thread();
                builder.max_blocking_threads(1).enable_time();
                if enableIo {
                    builder.enable_io();
                }
                let runtime = match builder.build() {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready.send(Err(error.to_string()));
                        return;
                    }
                };
                let _ = ready.send(Ok(()));
                // The runtime outlives every request and keeps its I/O and
                // spawned children alive without additional runtime threads.
                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, async {
                    loop {
                        // Never hold the queue lock across an await: a local
                        // future may resume and block this thread while the
                        // dispatcher waits, stranding the entire shared queue.
                        let task = {
                            let mut state =
                                pending.lock().expect("runtime task queue mutex poisoned");
                            let selected = (0..count)
                                .map(|offset| (state.cursor + offset) % count)
                                .filter(|worker| !state.polling[*worker])
                                .min_by_key(|worker| state.active[*worker]);
                            if selected == Some(index) {
                                let task = state.pending.pop_front();
                                if task.is_some() {
                                    state.active[index] += 1;
                                    state.cursor = (index + 1) % count;
                                }
                                task
                            } else {
                                None
                            }
                        };
                        let Some(task) = task else {
                            if updates.changed().await.is_err() {
                                break;
                            }
                            continue;
                        };
                        // Both factory and future stay inside the LocalSet panic boundary.
                        local.spawn_local(TrackedTask {
                            future: Box::pin(async move { task().await }),
                            state: pending.clone(),
                            changed: changed.clone(),
                            worker: index,
                        });
                        if let Some(changed) = changed.upgrade() {
                            let _ = changed.send(());
                        }
                        // Let the LocalSet start the task and keep the receive
                        // handoff fair when many tasks arrive in one burst.
                        tokio::task::yield_now().await;
                    }
                });
            })
            .map_err(|e| e.to_string())?;
        started.recv().map_err(|e| e.to_string())??;
    }
    Ok(RuntimeTaskQueue { pending, changed })
}

/// Initializes the process-wide coordination and blocking pools exactly once.
fn runtimeTaskPool() -> HostResult<&'static RuntimeTaskPool> {
    static POOL: OnceLock<Result<RuntimeTaskPool, String>> = OnceLock::new();
    POOL.get_or_init(|| {
        let coordination = spawnTaskPool(RUNTIME_WORKER_PREFIX, RUNTIME_WORKER_COUNT)?;
        let blocking = spawnTaskPool(BLOCKING_WORKER_PREFIX, BLOCKING_WORKER_COUNT)?;
        Ok(RuntimeTaskPool {
            coordination,
            blocking,
        })
    })
    .as_ref()
    .map_err(|e| HostError::new(format!("create runtime task pool failed: {e}")))
}

impl RuntimeTaskPool {
    /// Enqueues one coordination task on a free persistent worker.
    fn scheduleCoordination(&self, taskName: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        self.coordination
            .send(task)
            .map_err(|_| HostError::new(format!("coordination task worker closed: {taskName}")))
    }

    /// Enqueues one possibly blocking task on a free blocking worker.
    fn scheduleBlocking(&self, taskName: &str, task: HostRuntimeAsyncTask) -> HostResult<()> {
        self.blocking
            .send(task)
            .map_err(|_| HostError::new(format!("blocking task worker closed: {taskName}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{rc::Rc, sync::mpsc, time::Duration};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A resumed blocking call must not retain the queue needed by an idle worker.
    #[test]
    fn resumed_blocking_task_does_not_stall_idle_workers() {
        let tasks = spawnTaskPool("test-resumed-blocking", 2).unwrap();
        let (firstEntered, firstStarted) = mpsc::channel();
        let (resumeFirst, resume) = tokio::sync::oneshot::channel();
        let (releaseFirst, firstRelease) = mpsc::channel();
        let (firstBlocked, firstBlocking) = mpsc::channel();
        tasks
            .send(Box::new(move || {
                Box::pin(async move {
                    firstEntered.send(()).unwrap();
                    resume.await.unwrap();
                    // Model an async package call entering a synchronous terminal wait.
                    firstBlocked.send(()).unwrap();
                    firstRelease.recv_timeout(Duration::from_secs(5)).unwrap();
                })
            }))
            .unwrap();
        firstStarted.recv_timeout(Duration::from_secs(2)).unwrap();

        let (secondEntered, secondStarted) = mpsc::channel();
        let (releaseSecond, secondRelease) = mpsc::channel();
        let (secondExited, secondFinished) = mpsc::channel();
        tasks
            .send(Box::new(move || {
                Box::pin(async move {
                    secondEntered.send(()).unwrap();
                    secondRelease.recv_timeout(Duration::from_secs(5)).unwrap();
                    secondExited.send(()).unwrap();
                })
            }))
            .unwrap();
        secondStarted.recv_timeout(Duration::from_secs(2)).unwrap();
        // Give the first worker's dispatcher a turn to wait for more tasks.
        std::thread::sleep(Duration::from_millis(50));
        resumeFirst.send(()).unwrap();
        firstBlocking.recv_timeout(Duration::from_secs(2)).unwrap();
        releaseSecond.send(()).unwrap();
        secondFinished.recv_timeout(Duration::from_secs(2)).unwrap();

        let (probeSent, probeReceived) = mpsc::channel();
        tasks
            .send(Box::new(move || {
                Box::pin(async move {
                    probeSent.send(()).unwrap();
                })
            }))
            .unwrap();
        let result = probeReceived.recv_timeout(Duration::from_millis(500));
        // Always release the terminal wait, including on the unfixed implementation.
        releaseFirst.send(()).unwrap();
        assert!(
            result.is_ok(),
            "An idle worker could not dequeue an unrelated task: {result:?}"
        );
    }

    /// Waits for one pooled request to finish before accessing shared resources.
    fn runRequest(task: HostRuntimeAsyncTask) {
        let (done, completion) = mpsc::channel();
        NativeHostRuntimeTaskSchedulerHost
            .scheduleHostRuntimeAsyncTask(
                "test-request",
                Box::new(move || {
                    Box::pin(async move {
                        task().await;
                        done.send(()).unwrap();
                    })
                }),
            )
            .unwrap();
        completion.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    /// Verifies socket I/O remains valid after the creating request ends.
    #[test]
    fn connectionSurvivesBetweenRequests() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        runRequest(Box::new(move || {
            Box::pin(async move {
                let stream = tokio::net::TcpStream::connect(address).await.unwrap();
                sender.send(stream).unwrap();
            })
        }));
        let mut stream = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        let (mut peer, _) = listener.accept().unwrap();
        runRequest(Box::new(move || {
            Box::pin(async move {
                // Start/finish pairing must be able to use the very same socket.
                std::io::Write::write_all(&mut peer, b"code").unwrap();
                let mut received = [0; 4];
                tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut received))
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(&received, b"code");
                stream.write_all(b"done").await.unwrap();
            })
        }));
    }

    /// Verifies spawned receivers outlive their initiating request.
    #[test]
    fn spawnedReceiverSurvivesRequestCompletion() {
        let (release, released) = tokio::sync::oneshot::channel();
        let (done, completion) = mpsc::channel();
        runRequest(Box::new(move || {
            Box::pin(async move {
                tokio::spawn(async move {
                    released.await.unwrap();
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    done.send(()).unwrap();
                });
            })
        }));
        release
            .send(())
            .expect("receiver was aborted when request ended");
        completion.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    /// Verifies non-Send futures remain on the same pooled owner across waits.
    #[test]
    fn requestFutureCanRemainNonSend() {
        runRequest(Box::new(|| {
            Box::pin(async {
                let local = Rc::new(42);
                let thread = std::thread::current().id();
                tokio::time::sleep(Duration::from_millis(1)).await;
                assert_eq!(*local, 42);
                assert_eq!(std::thread::current().id(), thread);
            })
        }));
    }
}

/// Schedules native runtime tasks on process-wide coordination and blocking pools.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeHostRuntimeTaskSchedulerHost;

impl NativeHostRuntimeTaskSchedulerHost {
    /// Creates the native runtime task scheduler host.
    pub fn new() -> Self {
        Self
    }
}

impl NativeHostRuntimeTaskSchedulerHost {
    /// Runs all native network futures on the same four affine Host workers.
    pub fn scheduleNetworkAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        static NETWORK: OnceLock<Result<RuntimeTaskQueue, String>> = OnceLock::new();
        let queue = NETWORK
            .get_or_init(|| spawnTaskPool("operit-network-worker", NETWORK_WORKER_COUNT))
            .as_ref()
            .map_err(|error| HostError::new(format!("create network task pool failed: {error}")))?;
        queue
            .send(task)
            .map_err(|_| HostError::new(format!("network task queue is closed or full: {name}")))
    }

    /// Isolates Java owner callbacks from Rust callers synchronously waiting for their reply.
    pub fn scheduleOwnerTask(&self, name: &str, task: HostRuntimeTask) -> HostResult<()> {
        static OWNERS: OnceLock<Result<RuntimeTaskQueue, String>> = OnceLock::new();
        let queue = OWNERS
            .get_or_init(|| spawnTaskPool("operit-owner-worker", 8))
            .as_ref()
            .map_err(|error| HostError::new(format!("create owner task pool failed: {error}")))?;
        queue
            .send(Box::new(move || {
                Box::pin(async move {
                    task();
                })
            }))
            .map_err(|_| HostError::new(format!("owner task queue is closed or full: {name}")))
    }

    /// Keeps synchronous source compilation independent of script workers waiting for its result.
    pub fn scheduleJavaScriptStateTask(
        &self,
        name: &str,
        kind: operit_host_api::HostJavaScriptRuntimeStateKind,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        static ENGINES: OnceLock<Result<RuntimeTaskQueue, String>> = OnceLock::new();
        static COMPILERS: OnceLock<Result<RuntimeTaskQueue, String>> = OnceLock::new();
        let queue = match kind {
            operit_host_api::HostJavaScriptRuntimeStateKind::Execution => {
                ENGINES.get_or_init(|| spawnTaskPool("operit-javascript-worker", 4))
            }
            operit_host_api::HostJavaScriptRuntimeStateKind::Compilation => {
                COMPILERS.get_or_init(|| spawnTaskPool("operit-javascript-compiler", 1))
            }
        }
        .as_ref()
        .map_err(|error| HostError::new(format!("create JavaScript task pool failed: {error}")))?;
        queue
            .send(task)
            .map_err(|_| HostError::new(format!("JavaScript task queue is closed or full: {name}")))
    }

    /// Reserves bounded Host capacity for an OS API that must block for its service lifetime.
    pub fn scheduleServiceTask(&self, name: &str, task: HostRuntimeTask) -> HostResult<()> {
        self.scheduleServiceTasks(name, vec![task])
    }

    /// Reserves every worker needed by a blocking pipeline before starting any component.
    pub fn scheduleServiceTasks(&self, name: &str, tasks: Vec<HostRuntimeTask>) -> HostResult<()> {
        if tasks.is_empty() {
            return Err(HostError::new("Host service group is empty"));
        }
        let count = u32::try_from(tasks.len())
            .map_err(|_| HostError::new("Host service group exceeds capacity"))?;
        static SERVICES: OnceLock<Result<RuntimeTaskQueue, String>> = OnceLock::new();
        static CAPACITY: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
        let permit = Arc::new(
            CAPACITY
                .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(SERVICE_WORKER_COUNT)))
                .clone()
                .try_acquire_many_owned(count)
                .map_err(|_| HostError::new(format!("Host service capacity exhausted: {name}")))?,
        );
        let queue = SERVICES
            .get_or_init(|| spawnTaskPool("operit-service-worker", SERVICE_WORKER_COUNT))
            .as_ref()
            .map_err(|error| HostError::new(format!("create service task pool failed: {error}")))?;
        let admitted = tasks
            .into_iter()
            .map(|task| {
                let permit = permit.clone();
                Box::new(
                    move || -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> {
                        Box::pin(async move {
                            let _permit = permit;
                            task();
                        })
                    },
                ) as HostRuntimeAsyncTask
            })
            .collect();
        queue
            .sendMany(admitted)
            .map_err(|_| HostError::new(format!("service task queue is closed or full: {name}")))?;
        Ok(())
    }

    /// Executes platform-affine blocking calls on an isolated reusable Host worker.
    pub fn runPlatformTask<T: Send + 'static>(
        &self,
        name: &str,
        work: impl FnOnce() -> HostResult<T> + Send + 'static,
    ) -> HostResult<T> {
        static PLATFORM: OnceLock<Result<RuntimeTaskQueue, String>> = OnceLock::new();
        let queue = PLATFORM
            .get_or_init(|| spawnTaskPool("operit-platform-worker", 1))
            .as_ref()
            .map_err(|error| {
                HostError::new(format!("create platform task pool failed: {error}"))
            })?;
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        queue
            .send(Box::new(move || {
                Box::pin(async move {
                    let _ = sender.send(work());
                })
            }))
            .map_err(|_| {
                HostError::new(format!("platform task queue is closed or full: {name}"))
            })?;
        receiver
            .recv()
            .map_err(|error| HostError::new(format!("platform task stopped: {error}")))?
    }

    /// Returns a synchronous Host result while network waits stay on the shared network executor.
    pub fn runNetworkTask<T, F>(
        &self,
        name: &str,
        factory: impl FnOnce() -> F + Send + 'static,
    ) -> HostResult<T>
    where
        T: Send + 'static,
        F: std::future::Future<Output = HostResult<T>> + 'static,
    {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        self.scheduleNetworkAsyncTask(
            name,
            Box::new(move || {
                Box::pin(async move {
                    let _ = sender.send(factory().await);
                })
            }),
        )?;
        receiver
            .recv()
            .map_err(|error| HostError::new(format!("network task stopped: {error}")))?
    }
}

impl HostRuntimeTaskSchedulerHost for NativeHostRuntimeTaskSchedulerHost {
    /// Reads the process-local monotonic clock used for native elapsed-time measurements.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        Ok(START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_millis() as u64)
    }

    /// Executes synchronous work on the dedicated blocking pool.
    fn scheduleHostRuntimeTask(&self, taskName: &str, task: HostRuntimeTask) -> HostResult<()> {
        runtimeTaskPool()?
            .scheduleBlocking(taskName, Box::new(move || Box::pin(async move { task() })))
    }

    /// Creates and polls an asynchronous request on a free coordination worker.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        runtimeTaskPool()?.scheduleCoordination(taskName, task)
    }

    /// Creates and polls a possibly blocking request on a free blocking worker.
    fn scheduleHostRuntimeBlockingAsyncTask(
        &self,
        taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        runtimeTaskPool()?.scheduleBlocking(taskName, task)
    }

    /// Uses the coordination pool for cooperative callers without creating another executor.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(taskName, task)
    }

    /// Waits asynchronously and executes delayed work on the blocking pool.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        taskName: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeBlockingAsyncTask(
            taskName,
            Box::new(move || {
                Box::pin(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(delayMs)).await;
                    task();
                })
            }),
        )
    }

    /// Waits for a later native executor turn without occupying the caller thread.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        Box::pin(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            Ok(())
        })
    }

    /// Waits through the native timer executor for one platform-owned delay.
    fn waitForHostRuntimeDelay(&self, delayMs: u64) -> HostRuntimeTurnFuture {
        Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delayMs)).await;
            Ok(())
        })
    }
}

#[cfg(test)]
mod timer_clock_tests {
    use super::*;

    /// Verifies wall-clock timestamps are not used and readings advance with native delays.
    #[test]
    fn scheduler_clock_is_monotonic_and_shared_between_instances() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost::new();
        let before = scheduler.monotonicTimeMillis().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let after = NativeHostRuntimeTaskSchedulerHost::new()
            .monotonicTimeMillis()
            .unwrap();
        assert!(after >= before + 1);
    }

    /// Verifies the local executor and its delegated timer use the same clock origin.
    #[test]
    fn local_scheduler_uses_the_native_timer_clock() {
        let local = crate::LocalHostRuntimeTaskSchedulerHost::new().unwrap();
        let before = NativeHostRuntimeTaskSchedulerHost
            .monotonicTimeMillis()
            .unwrap();
        let now = local.monotonicTimeMillis().unwrap();
        let after = NativeHostRuntimeTaskSchedulerHost
            .monotonicTimeMillis()
            .unwrap();
        assert!(before <= now && now <= after);
    }
}

#[cfg(test)]
mod cooperative_tests {
    use super::*;
    use std::{collections::HashSet, rc::Rc, sync::mpsc, time::Duration};

    /// Verifies cooperative tasks share the native pool and keep non-Send values affine.
    #[test]
    fn cooperative_tasks_reuse_threads_and_keep_non_send_futures_affine() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost;
        let (send, receive) = mpsc::channel();
        for _ in 0..64 {
            let send = send.clone();
            scheduler
                .scheduleHostRuntimeCooperativeAsyncTask(
                    "test-cooperative",
                    Box::new(move || {
                        Box::pin(async move {
                            let local = Rc::new(7);
                            let thread = std::thread::current().id();
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            assert_eq!(thread, std::thread::current().id());
                            assert_eq!(*local, 7);
                            send.send(thread).unwrap();
                        })
                    }),
                )
                .unwrap();
        }
        let threads = (0..64)
            .map(|_| receive.recv_timeout(Duration::from_secs(5)).unwrap())
            .collect::<HashSet<_>>();
        assert!(threads.len() <= RUNTIME_WORKER_COUNT);
    }

    /// Verifies suspended work leaves every owner available to another ready task.
    #[test]
    fn cooperative_waiting_task_does_not_block_other_tasks_on_the_same_worker() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost;
        let (release, wait) = tokio::sync::oneshot::channel();
        let (done, receive) = mpsc::channel();
        scheduler
            .scheduleHostRuntimeCooperativeAsyncTask(
                "waiting",
                Box::new(move || {
                    Box::pin(async move {
                        wait.await.unwrap();
                    })
                }),
            )
            .unwrap();
        // Cover the whole pool so ready tasks also reach the waiting task's owner.
        for _ in 0..RUNTIME_WORKER_COUNT {
            let done = done.clone();
            scheduler
                .scheduleHostRuntimeCooperativeAsyncTask(
                    "ready",
                    Box::new(move || {
                        Box::pin(async move {
                            done.send(()).unwrap();
                        })
                    }),
                )
                .unwrap();
        }
        for _ in 0..RUNTIME_WORKER_COUNT {
            receive.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        release.send(()).unwrap();
    }

    /// Verifies a panicking factory cannot destroy a worker or force a new thread.
    #[test]
    fn cooperative_factory_panic_does_not_kill_its_owner_thread() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost;
        let (entered, receive) = mpsc::channel();
        scheduler
            .scheduleHostRuntimeCooperativeAsyncTask(
                "panicking-factory",
                Box::new(move || {
                    entered.send(std::thread::current().id()).unwrap();
                    panic!("test task factory panic");
                }),
            )
            .unwrap();
        let original = receive.recv_timeout(Duration::from_secs(5)).unwrap();
        let (done, receive) = mpsc::channel();
        // Enough consecutive submissions to cover all owners, even when other
        // scheduler tests concurrently advance the round-robin index.
        for _ in 0..64 {
            let done = done.clone();
            scheduler
                .scheduleHostRuntimeCooperativeAsyncTask(
                    "after-panic",
                    Box::new(move || {
                        Box::pin(async move {
                            done.send(std::thread::current().id()).unwrap();
                        })
                    }),
                )
                .unwrap();
        }
        let threads = (0..64)
            .map(|_| receive.recv_timeout(Duration::from_secs(5)).unwrap())
            .collect::<HashSet<_>>();
        assert!(threads.contains(&original));
        assert!(threads.len() <= RUNTIME_WORKER_COUNT);
    }

    /// Verifies spawned I/O children survive the completion of their parent request.
    #[test]
    fn cooperative_spawned_io_child_survives_parent_completion() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost;
        let (release, wait) = tokio::sync::oneshot::channel();
        let (done, receive) = mpsc::channel();
        scheduler
            .scheduleHostRuntimeCooperativeAsyncTask(
                "parent",
                Box::new(move || {
                    Box::pin(async move {
                        tokio::spawn(async move {
                            wait.await.unwrap();
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            done.send(()).unwrap();
                        });
                    })
                }),
            )
            .unwrap();
        release.send(()).unwrap();
        receive.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}

#[cfg(test)]
mod pool_tests {
    use super::*;
    use std::{
        collections::HashSet,
        rc::Rc,
        sync::{mpsc, Mutex},
        time::{Duration, Instant},
    };

    /// Serializes tests that deliberately occupy the shared blocking pool.
    static BLOCKING_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Returns the worker name so tests can prove which pool ran a task.
    fn threadName() -> String {
        std::thread::current()
            .name()
            .unwrap_or_default()
            .to_string()
    }

    /// Verifies task kinds stay on their declared pool and the pools never overlap.
    #[test]
    fn task_kinds_use_disjoint_persistent_pools() {
        let _guard = BLOCKING_TEST_LOCK.lock().unwrap();
        let scheduler = NativeHostRuntimeTaskSchedulerHost::new();
        let (sender, receiver) = mpsc::channel();
        let mut first_workers = HashSet::new();
        for round in 0..2 {
            for index in 0..320 {
                let sender = sender.clone();
                let other = NativeHostRuntimeTaskSchedulerHost::new();
                match index % 5 {
                    0 => scheduler.scheduleHostRuntimeTask(
                        "pool-sync",
                        Box::new(move || {
                            sender.send(("blocking", threadName())).unwrap();
                        }),
                    ),
                    1 => other.scheduleHostRuntimeBlockingAsyncTask(
                        "pool-blocking-async",
                        Box::new(move || {
                            Box::pin(async move {
                                sender.send(("blocking", threadName())).unwrap();
                            })
                        }),
                    ),
                    2 => other.scheduleHostRuntimeAsyncTask(
                        "pool-async",
                        Box::new(move || {
                            Box::pin(async move {
                                let local = Rc::new(42);
                                let owner = std::thread::current().id();
                                tokio::time::sleep(Duration::from_millis(1)).await;
                                assert_eq!(owner, std::thread::current().id());
                                assert_eq!(*local, 42);
                                sender.send(("coordination", threadName())).unwrap();
                            })
                        }),
                    ),
                    3 => other.scheduleHostRuntimeCooperativeAsyncTask(
                        "pool-cooperative",
                        Box::new(move || {
                            Box::pin(async move {
                                sender.send(("coordination", threadName())).unwrap();
                            })
                        }),
                    ),
                    _ => scheduler.scheduleDelayedHostRuntimeTask(
                        "pool-delayed",
                        1,
                        Box::new(move || {
                            sender.send(("blocking", threadName())).unwrap();
                        }),
                    ),
                }
                .unwrap();
            }
            let workers = (0..320)
                .map(|_| receiver.recv_timeout(Duration::from_secs(10)).unwrap())
                .collect::<HashSet<_>>();
            let coordination = workers
                .iter()
                .filter(|(pool, _)| *pool == "coordination")
                .map(|(_, name)| name.clone())
                .collect::<HashSet<_>>();
            let blocking = workers
                .iter()
                .filter(|(pool, _)| *pool == "blocking")
                .map(|(_, name)| name.clone())
                .collect::<HashSet<_>>();
            assert_eq!(coordination.len(), RUNTIME_WORKER_COUNT);
            assert_eq!(blocking.len(), BLOCKING_WORKER_COUNT);
            assert!(coordination.is_disjoint(&blocking));
            assert!(coordination
                .iter()
                .all(|name| name.starts_with(RUNTIME_WORKER_PREFIX)));
            assert!(blocking
                .iter()
                .all(|name| name.starts_with(BLOCKING_WORKER_PREFIX)));
            if round == 0 {
                first_workers = workers;
            } else {
                assert_eq!(workers, first_workers);
            }
        }
    }

    /// Verifies a fully occupied blocking pool cannot delay coordination tasks.
    #[test]
    fn saturated_blocking_pool_does_not_delay_coordination_tasks() {
        let _guard = BLOCKING_TEST_LOCK.lock().unwrap();
        let scheduler = NativeHostRuntimeTaskSchedulerHost;
        let (entered, started) = mpsc::channel();
        for _ in 0..BLOCKING_WORKER_COUNT {
            let entered = entered.clone();
            scheduler
                .scheduleHostRuntimeBlockingAsyncTask(
                    "occupy-blocking",
                    Box::new(move || {
                        Box::pin(async move {
                            entered.send(()).unwrap();
                            std::thread::sleep(Duration::from_millis(300));
                        })
                    }),
                )
                .unwrap();
        }
        for _ in 0..BLOCKING_WORKER_COUNT {
            started.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        let (done, finished) = mpsc::channel();
        let start = Instant::now();
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "coordination-probe",
                Box::new(move || {
                    Box::pin(async move {
                        done.send(()).unwrap();
                    })
                }),
            )
            .unwrap();
        finished.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(
            start.elapsed() < Duration::from_millis(200),
            "coordination task waited behind blocked workers: {:?}",
            start.elapsed()
        );
    }

    /// Verifies more than sixteen pending streams suspend without blocking new requests.
    #[test]
    fn suspended_tasks_do_not_exhaust_the_worker_pool() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost;
        let (entered, started) = mpsc::channel();
        let (completed, done) = mpsc::channel();
        let mut releases = Vec::new();
        for _ in 0..128 {
            let (release, wait) = tokio::sync::oneshot::channel();
            releases.push(release);
            let entered = entered.clone();
            let completed = completed.clone();
            scheduler
                .scheduleHostRuntimeAsyncTask(
                    "pool-waiting",
                    Box::new(move || {
                        Box::pin(async move {
                            entered.send(()).unwrap();
                            wait.await.unwrap();
                            completed.send(()).unwrap();
                        })
                    }),
                )
                .unwrap();
        }
        for _ in 0..128 {
            started.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        let (sender, receiver) = mpsc::channel();
        for _ in 0..64 {
            let sender = sender.clone();
            scheduler
                .scheduleHostRuntimeAsyncTask(
                    "pool-ready",
                    Box::new(move || Box::pin(async move { sender.send(()).unwrap() })),
                )
                .unwrap();
        }
        for _ in 0..64 {
            receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        for release in releases {
            release.send(()).unwrap();
        }
        for _ in 0..128 {
            done.recv_timeout(Duration::from_secs(5)).unwrap();
        }
    }
}

#[cfg(test)]
mod javascript_dependency_tests {
    use super::*;
    use operit_host_api::HostJavaScriptRuntimeStateKind;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Compiles source while every script worker synchronously waits for compilation capacity.
    #[test]
    fn compilation_progresses_with_all_execution_workers_waiting() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost::new();
        let (ready, started) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        let mut releases = Vec::new();
        for _ in 0..4 {
            let (release, released) = mpsc::channel();
            releases.push(release);
            let ready = ready.clone();
            let done = done.clone();
            scheduler
                .scheduleJavaScriptStateTask(
                    "waiting-script",
                    HostJavaScriptRuntimeStateKind::Execution,
                    Box::new(move || {
                        Box::pin(async move {
                            ready.send(()).unwrap();
                            released.recv_timeout(Duration::from_secs(3)).unwrap();
                            let (compiled, result) = mpsc::channel();
                            scheduler
                                .scheduleJavaScriptStateTask(
                                    "source-compiler",
                                    HostJavaScriptRuntimeStateKind::Compilation,
                                    Box::new(move || {
                                        Box::pin(async move {
                                            compiled.send(()).unwrap();
                                        })
                                    }),
                                )
                                .unwrap();
                            let result = result.recv_timeout(Duration::from_secs(3));
                            done.send(result).unwrap();
                        })
                    }),
                )
                .unwrap();
        }
        for _ in 0..4 {
            started.recv_timeout(Duration::from_secs(3)).unwrap();
        }
        for release in releases {
            release.send(()).unwrap();
        }
        for _ in 0..4 {
            completed
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                .unwrap();
        }
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Many independent network waits reuse at most the configured four workers.
    #[test]
    fn network_futures_share_four_workers() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost::new();
        let (sender, receiver) = mpsc::channel();
        let mut releases = Vec::new();
        for _ in 0..128 {
            let (release, wait) = tokio::sync::oneshot::channel();
            releases.push(release);
            let sender = sender.clone();
            scheduler
                .scheduleNetworkAsyncTask(
                    "network-affinity",
                    Box::new(move || {
                        Box::pin(async move {
                            let owner = std::thread::current().id();
                            sender
                                .send((owner, std::thread::current().name().unwrap().to_owned()))
                                .unwrap();
                            wait.await.unwrap();
                            assert_eq!(owner, std::thread::current().id());
                            sender
                                .send((owner, std::thread::current().name().unwrap().to_owned()))
                                .unwrap();
                        })
                    }),
                )
                .unwrap();
        }
        let owners = (0..128)
            .map(|_| receiver.recv_timeout(Duration::from_secs(5)).unwrap())
            .collect::<std::collections::HashSet<_>>();
        assert!(!owners.is_empty() && owners.len() <= NETWORK_WORKER_COUNT);
        assert!(owners
            .iter()
            .all(|(_, name)| name.starts_with("operit-network-worker-")));
        for release in releases {
            release.send(()).unwrap();
        }
        for _ in 0..128 {
            assert!(owners.contains(&receiver.recv_timeout(Duration::from_secs(5)).unwrap()));
        }
    }

    /// An oversized service group must fail before any of its jobs can start.
    #[test]
    fn service_group_reservation_is_atomic() {
        let scheduler = NativeHostRuntimeTaskSchedulerHost::new();
        let (sender, receiver) = mpsc::channel();
        let too_many = (0..SERVICE_WORKER_COUNT + 1)
            .map(|_| {
                let sender = sender.clone();
                Box::new(move || {
                    sender.send(()).unwrap();
                }) as HostRuntimeTask
            })
            .collect();
        assert!(scheduler
            .scheduleServiceTasks("oversized", too_many)
            .is_err());
        assert!(receiver.try_recv().is_err());
        let mut releases = Vec::new();
        let admitted = (0..SERVICE_WORKER_COUNT)
            .map(|_| {
                let sender = sender.clone();
                let (release, wait) = mpsc::channel();
                releases.push(release);
                Box::new(move || {
                    sender.send(()).unwrap();
                    let _ = wait.recv_timeout(Duration::from_secs(5));
                }) as HostRuntimeTask
            })
            .collect();
        scheduler
            .scheduleServiceTasks("complete-pipeline", admitted)
            .unwrap();
        for _ in 0..SERVICE_WORKER_COUNT {
            receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        let rejected = scheduler.scheduleServiceTask(
            "over-capacity",
            Box::new(|| panic!("rejected task started")),
        );
        for release in releases {
            release.send(()).unwrap();
        }
        assert!(rejected.is_err());
    }

    /// Releasing a non-global scheduler stops workers and drops their suspended futures.
    #[test]
    fn dropping_pool_releases_affine_tasks() {
        struct Released(mpsc::Sender<()>);
        impl Drop for Released {
            /// Reports destruction of a worker-owned future.
            fn drop(&mut self) {
                self.0.send(()).unwrap();
            }
        }
        let pool = spawnTaskPoolWithOptions("test-owned", 2, 2 * 1024 * 1024, false).unwrap();
        let (started, ready) = mpsc::channel();
        let (released, dropped) = mpsc::channel();
        for _ in 0..2 {
            let started = started.clone();
            let released = released.clone();
            pool.send(Box::new(move || {
                Box::pin(async move {
                    let _released = Released(released);
                    started.send(()).unwrap();
                    std::future::pending::<()>().await;
                })
            }))
            .unwrap();
        }
        for _ in 0..2 {
            ready.recv_timeout(Duration::from_secs(3)).unwrap();
        }
        drop(pool);
        for _ in 0..2 {
            dropped.recv_timeout(Duration::from_secs(3)).unwrap();
        }
    }
}
