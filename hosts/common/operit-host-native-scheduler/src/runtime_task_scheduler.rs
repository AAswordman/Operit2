use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

static ASYNC_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

/// Owns I/O drivers, timers and spawned tasks for the native process lifetime.
/// Connections returned by one request may be reused by later requests.
fn asyncRuntime() -> HostResult<&'static tokio::runtime::Runtime> {
    let runtime = ASYNC_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("operit-runtime-worker")
            .build()
            .map_err(|error| error.to_string())
    });
    runtime
        .as_ref()
        .map_err(|error| HostError::new(format!("create runtime async executor failed: {error}")))
}

/// Only opt-in, nonblocking bridge tasks use these reusable owner threads.
/// Their !Send futures are created and polled inside one LocalSet; sockets and
/// tokio::spawn children still use the process-lifetime shared I/O runtime.
struct CooperativeExecutors {
    workers: Vec<tokio::sync::mpsc::UnboundedSender<HostRuntimeAsyncTask>>,
    next: AtomicUsize,
}

fn cooperativeExecutors() -> HostResult<&'static CooperativeExecutors> {
    static WORKERS: OnceLock<Result<CooperativeExecutors, String>> = OnceLock::new();
    WORKERS
        .get_or_init(|| {
            let runtime = asyncRuntime().map_err(|e| e.to_string())?;
            let mut workers = Vec::new();
            for index in 0..2 {
                let (sender, mut receiver) =
                    tokio::sync::mpsc::unbounded_channel::<HostRuntimeAsyncTask>();
                std::thread::Builder::new()
                    .name(format!("operit-bridge-worker-{index}"))
                    .spawn(move || {
                        tokio::task::LocalSet::new().block_on(runtime, async move {
                            while let Some(task) = receiver.recv().await {
                                // A factory panic is confined to its task, not the worker.
                                tokio::task::spawn_local(async move {
                                    task().await;
                                });
                            }
                        });
                    })
                    .map_err(|e| e.to_string())?;
                workers.push(sender);
            }
            Ok(CooperativeExecutors {
                workers,
                next: AtomicUsize::new(0),
            })
        })
        .as_ref()
        .map_err(|e| HostError::new(format!("create cooperative task workers failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{rc::Rc, sync::mpsc, time::Duration};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Waits for a request thread to finish before accessing shared resources.
    fn runRequest(task: HostRuntimeAsyncTask) {
        std::thread::spawn(move || runAsyncRuntimeTask(task))
            .join()
            .expect("request thread panicked");
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

    /// Verifies non-Send futures remain on the request thread.
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

/// Runs one asynchronous runtime task on its dedicated named native thread.
fn runAsyncRuntimeTask(task: HostRuntimeAsyncTask) {
    // block_on keeps the possibly !Send request future on its named thread;
    // the shared runtime keeps its sockets and tokio::spawn children alive.
    asyncRuntime()
        .expect("create runtime task executor failed")
        .block_on(async move { task().await });
}

/// Schedules one-shot runtime tasks on named native threads.
#[derive(Clone, Copy, Debug, Default)]
pub struct NativeHostRuntimeTaskSchedulerHost;

impl NativeHostRuntimeTaskSchedulerHost {
    /// Creates the native runtime task scheduler host.
    pub fn new() -> Self {
        Self
    }
}

impl HostRuntimeTaskSchedulerHost for NativeHostRuntimeTaskSchedulerHost {
    /// Reads the process-local monotonic clock used for native elapsed-time measurements.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        Ok(START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64)
    }

    /// Starts the task on a named native thread.
    fn scheduleHostRuntimeTask(&self, taskName: &str, task: HostRuntimeTask) -> HostResult<()> {
        std::thread::Builder::new()
            .name(taskName.to_string())
            .spawn(task)
            .map(|_| ())
            .map_err(|error| {
                HostError::new(format!(
                    "create runtime task thread {taskName} failed: {error}"
                ))
            })
    }

    /// Starts an asynchronous task on its own named native thread.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        // Report initialization errors to the caller, not just a detached thread.
        asyncRuntime()?;
        std::thread::Builder::new()
            .name(taskName.to_string())
            .spawn(move || runAsyncRuntimeTask(task))
            .map(|_| ())
            .map_err(|error| {
                HostError::new(format!(
                    "create runtime async task thread {taskName} failed: {error}"
                ))
            })
    }

    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        let executors = cooperativeExecutors()?;
        let index = executors.next.fetch_add(1, Ordering::Relaxed) % executors.workers.len();
        executors.workers[index]
            .send(task)
            .map_err(|_| HostError::new(format!("cooperative task worker closed: {taskName}")))
    }

    /// Starts a named native task after the requested delay.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        taskName: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        let taskName = taskName.to_string();
        asyncRuntime()?.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delayMs)).await;
            std::thread::Builder::new()
                .name(taskName)
                .spawn(task)
                .expect("delayed runtime task thread must start");
        });
        Ok(())
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
        let after = NativeHostRuntimeTaskSchedulerHost::new().monotonicTimeMillis().unwrap();
        assert!(after >= before + 1);
    }

    /// Verifies the local executor and its delegated timer use the same clock origin.
    #[test]
    fn local_scheduler_uses_the_native_timer_clock() {
        let local = crate::LocalHostRuntimeTaskSchedulerHost::new().unwrap();
        let before = NativeHostRuntimeTaskSchedulerHost.monotonicTimeMillis().unwrap();
        let now = local.monotonicTimeMillis().unwrap();
        let after = NativeHostRuntimeTaskSchedulerHost.monotonicTimeMillis().unwrap();
        assert!(before <= now && now <= after);
    }
}

#[cfg(test)]
mod cooperative_tests {
    use super::*;
    use std::{collections::HashSet, rc::Rc, sync::mpsc, time::Duration};

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
        assert_eq!(threads.len(), 2);
    }

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
        // Submit to both workers: one must share the waiting task's executor.
        for _ in 0..2 {
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
        receive.recv_timeout(Duration::from_secs(5)).unwrap();
        receive.recv_timeout(Duration::from_secs(5)).unwrap();
        release.send(()).unwrap();
    }

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
        // Enough consecutive submissions to cover both owners, even when other
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
        assert_eq!(threads.len(), 2);
    }

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
