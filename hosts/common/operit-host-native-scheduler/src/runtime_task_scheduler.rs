use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
};
use std::sync::OnceLock;

static TIMER_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

/// Returns the asynchronous timer runtime used only to trigger delayed named tasks.
fn timerRuntime() -> HostResult<&'static tokio::runtime::Runtime> {
    let runtime = TIMER_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .worker_threads(1)
            .thread_name("operit-runtime-timer")
            .build()
            .map_err(|error| error.to_string())
    });
    runtime
        .as_ref()
        .map_err(|error| HostError::new(format!("create runtime timer executor failed: {error}")))
}

/// Runs one asynchronous runtime task on its dedicated named native thread.
fn runAsyncRuntimeTask(task: HostRuntimeAsyncTask) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("create runtime task executor failed");
    runtime.block_on(task());
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

    /// Starts a named native task after the requested delay.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        taskName: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        let taskName = taskName.to_string();
        timerRuntime()?.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delayMs)).await;
            std::thread::Builder::new()
                .name(taskName)
                .spawn(task)
                .expect("delayed runtime task thread must start");
        });
        Ok(())
    }
}
