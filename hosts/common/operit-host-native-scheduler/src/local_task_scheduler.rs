//! Small Host scheduler with separate asynchronous and blocking capacity for device embeddings.
use crate::runtime_task_scheduler::{spawnTaskPoolWithOptions, RuntimeTaskQueue};
use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use std::sync::Arc;

pub const DEVICE_ASYNC_STACK_BYTES: usize = 24 * 1024;
pub const DEVICE_BLOCKING_STACK_BYTES: usize = 20 * 1024;

#[derive(Clone)]
pub struct LocalHostRuntimeTaskSchedulerHost {
    tasks: Arc<RuntimeTaskQueue>,
    blocking: Arc<RuntimeTaskQueue>,
}

impl LocalHostRuntimeTaskSchedulerHost {
    /// Shares process-wide device workers while keeping blocking and asynchronous work separate.
    pub fn new() -> HostResult<Self> {
        static SHARED: std::sync::OnceLock<Result<LocalHostRuntimeTaskSchedulerHost, String>> =
            std::sync::OnceLock::new();
        SHARED
            .get_or_init(|| {
                Self::newWithIo(!cfg!(target_os = "espidf")).map_err(|error| error.to_string())
            })
            .as_ref()
            .cloned()
            .map_err(|error| HostError::new(error.clone()))
    }

    /// Selects timer-only execution for Host sockets that do not use the Tokio I/O driver.
    fn newWithIo(enableIo: bool) -> HostResult<Self> {
        let stack = if cfg!(any(target_os = "espidf", feature = "esp32-compat")) {
            DEVICE_ASYNC_STACK_BYTES
        } else {
            2 * 1024 * 1024
        };
        let tasks = spawnTaskPoolWithOptions("operit-ha", 1, stack, enableIo)
            .map_err(HostError::new)?;
        // Consent queries use projections; evidence is decoded only during command execution.
        let blockingStack = if cfg!(any(target_os = "espidf", feature = "esp32-compat")) {
            DEVICE_BLOCKING_STACK_BYTES
        } else { stack };
        let blocking = spawnTaskPoolWithOptions("operit-host-blocking", 1, blockingStack, enableIo)
            .map_err(HostError::new)?;
        Ok(Self {
            tasks: Arc::new(tasks),
            blocking: Arc::new(blocking),
        })
    }
}

impl HostRuntimeTaskSchedulerHost for LocalHostRuntimeTaskSchedulerHost {
    /// Reads the process-wide monotonic timer clock.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        crate::NativeHostRuntimeTaskSchedulerHost.monotonicTimeMillis()
    }

    /// Executes synchronous device work on bounded blocking capacity.
    fn scheduleHostRuntimeTask(&self, name: &str, task: HostRuntimeTask) -> HostResult<()> {
        self.scheduleHostRuntimeBlockingAsyncTask(
            name,
            Box::new(move || Box::pin(async move { task() })),
        )
    }

    /// Creates an affine future on the device's asynchronous worker.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.tasks
            .send(task)
            .map_err(|_| HostError::new(format!("Host async queue is closed or full: {name}")))
    }

    /// Keeps synchronous calls inside asynchronous workflows off the coordination worker.
    fn scheduleHostRuntimeBlockingAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.blocking
            .send(task)
            .map_err(|_| HostError::new(format!("Host blocking queue is closed or full: {name}")))
    }

    /// Shares the asynchronous worker for cooperative callers.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(name, task)
    }

    /// Waits without occupying a thread and runs the synchronous callback on blocking capacity.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        name: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        let delay = self.waitForHostRuntimeDelay(delayMs);
        self.scheduleHostRuntimeBlockingAsyncTask(
            name,
            Box::new(move || {
                Box::pin(async move {
                    delay.await.expect("Host timer must remain available");
                    task();
                })
            }),
        )
    }

    /// Yields one asynchronous executor turn.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        crate::NativeHostRuntimeTaskSchedulerHost.waitForHostRuntimeTaskTurn()
    }

    /// Waits on the timer driver owned by the current Host worker.
    fn waitForHostRuntimeDelay(&self, delayMs: u64) -> HostRuntimeTurnFuture {
        crate::NativeHostRuntimeTaskSchedulerHost.waitForHostRuntimeDelay(delayMs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Device HTTP work must share the guarded async worker instead of creating desktop runtimes.
    #[cfg(feature = "esp32-compat")]
    #[test]
    fn network_tasks_use_the_device_async_worker() {
        let local = LocalHostRuntimeTaskSchedulerHost::new().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let deviceSender = sender.clone();
        local.scheduleHostRuntimeAsyncTask("device-worker", Box::new(move || Box::pin(async move {
            deviceSender.send(std::thread::current().id()).unwrap();
        }))).unwrap();
        crate::NativeHostRuntimeTaskSchedulerHost::new()
            .scheduleNetworkAsyncTask("device-http", Box::new(move || Box::pin(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                sender.send(std::thread::current().id()).unwrap();
            }))).unwrap();
        let device = receiver.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        let network = receiver.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(device, network);
    }
    /// A blocked device call must leave the asynchronous timer worker available.
    #[test]
    fn blocking_device_work_does_not_stall_async_timers() {
        let scheduler = LocalHostRuntimeTaskSchedulerHost::newWithIo(false).unwrap();
        let (release, blocked) = std::sync::mpsc::channel();
        let (started, ready) = std::sync::mpsc::channel();
        scheduler
            .scheduleHostRuntimeTask(
                "device-blocked",
                Box::new(move || {
                    started.send(()).unwrap();
                    let _ = blocked.recv_timeout(std::time::Duration::from_secs(5));
                }),
            )
            .unwrap();
        ready
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        let (sent, received) = std::sync::mpsc::channel();
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "device-timer",
                Box::new(move || {
                    Box::pin(async move {
                        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                        sent.send(()).unwrap();
                    })
                }),
            )
            .unwrap();
        let completed = received.recv_timeout(std::time::Duration::from_secs(2));
        release.send(()).unwrap();
        completed.unwrap();
    }

    #[test]
    fn timer_only_worker_runs_host_delays_without_an_io_driver() {
        let scheduler = LocalHostRuntimeTaskSchedulerHost::newWithIo(false).unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let delay = scheduler.waitForHostRuntimeDelay(5);
        scheduler
            .scheduleHostRuntimeAsyncTask(
                "test-timer-only",
                Box::new(move || {
                    Box::pin(async move {
                        delay.await.unwrap();
                        tokio::task::yield_now().await;
                        sender.send(()).unwrap();
                    })
                }),
            )
            .unwrap();
        receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
    }

    #[test]
    fn local_futures_share_one_worker_and_survive_request_return() {
        let scheduler = LocalHostRuntimeTaskSchedulerHost::new().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        for _ in 0..2 {
            let sender = sender.clone();
            let delay = scheduler.waitForHostRuntimeDelay(1);
            scheduler
                .scheduleHostRuntimeAsyncTask(
                    "test-local",
                    Box::new(move || {
                        Box::pin(async move {
                            let local = std::rc::Rc::new(42);
                            let thread = std::thread::current().id();
                            delay.await.unwrap();
                            assert_eq!(*local, 42);
                            assert_eq!(thread, std::thread::current().id());
                            sender.send(thread).unwrap();
                        })
                    }),
                )
                .unwrap();
        }
        let a = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let b = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert_eq!(a, b);
    }
}
