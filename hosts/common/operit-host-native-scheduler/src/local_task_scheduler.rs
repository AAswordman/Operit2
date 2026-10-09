//! Small Host executor for device embeddings: all !Send futures stay on one
//! worker instead of allocating a native thread for every peer task.
use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct LocalHostRuntimeTaskSchedulerHost {
    tasks: mpsc::UnboundedSender<HostRuntimeAsyncTask>,
}

impl LocalHostRuntimeTaskSchedulerHost {
    pub fn new() -> HostResult<Self> {
        // ESP Host sockets use nonblocking std I/O and Host timers, not Mio.
        // enable_all() reserves 1024 I/O events and creates a select/poll
        // driver that can abort on ENOMEM during concurrent authentication.
        Self::newWithIo(!cfg!(target_os = "espidf"))
    }

    fn newWithIo(enableIo: bool) -> HostResult<Self> {
        let (sender, mut receiver) = mpsc::unbounded_channel::<HostRuntimeAsyncTask>();
        let (ready, started) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("operit-host-tasks".into())
            .stack_size(
                if cfg!(any(target_os = "espidf", feature = "esp32-compat")) {
                    32 * 1024
                } else {
                    2 * 1024 * 1024
                },
            )
            .spawn(move || {
                let mut builder = tokio::runtime::Builder::new_current_thread();
                builder.enable_time();
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
                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, async {
                    while let Some(task) = receiver.recv().await {
                        // Use the LocalSet handle explicitly. Calling the free
                        // spawn_local function here can lose the LocalSet
                        // context on ESP-IDF and abort the firmware.
                        local.spawn_local(async move { task().await });
                    }
                });
            })
            .map_err(|error| HostError::new(format!("start Host task worker: {error}")))?;
        started
            .recv()
            .map_err(|error| HostError::new(error.to_string()))?
            .map_err(HostError::new)?;
        Ok(Self { tasks: sender })
    }
}

impl HostRuntimeTaskSchedulerHost for LocalHostRuntimeTaskSchedulerHost {
    /// Reads the same native timer clock as the delegated delay implementation.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        crate::NativeHostRuntimeTaskSchedulerHost.monotonicTimeMillis()
    }

    fn scheduleHostRuntimeTask(&self, name: &str, task: HostRuntimeTask) -> HostResult<()> {
        self.scheduleHostRuntimeAsyncTask(name, Box::new(|| Box::pin(async move { task() })))
    }
    fn scheduleHostRuntimeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.tasks
            .send(task)
            .map_err(|_| HostError::new(format!("Host task worker closed: {name}")))
    }

    /// Executes cooperative work on the host-owned asynchronous executor.
    fn scheduleHostRuntimeCooperativeAsyncTask(
        &self,
        name: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        self.tasks
            .send(task)
            .map_err(|_| HostError::new(format!("Host task worker closed: {name}")))
    }
    fn scheduleDelayedHostRuntimeTask(
        &self,
        name: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        let delay = self.waitForHostRuntimeDelay(delayMs);
        self.scheduleHostRuntimeAsyncTask(
            name,
            Box::new(|| {
                Box::pin(async move {
                    if delay.await.is_ok() {
                        task();
                    }
                })
            }),
        )
    }
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        crate::NativeHostRuntimeTaskSchedulerHost.waitForHostRuntimeTaskTurn()
    }
    fn waitForHostRuntimeDelay(&self, delayMs: u64) -> HostRuntimeTurnFuture {
        crate::NativeHostRuntimeTaskSchedulerHost.waitForHostRuntimeDelay(delayMs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
