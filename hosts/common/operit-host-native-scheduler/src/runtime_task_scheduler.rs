use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
};
use std::sync::OnceLock;

static DELAY_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

/// Returns the shared asynchronous timer runtime used by delayed host tasks.
fn delayRuntime() -> HostResult<&'static tokio::runtime::Runtime> {
    let runtime = DELAY_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .worker_threads(1)
            .thread_name("operit-runtime-delay")
            .build()
            .map_err(|error| error.to_string())
    });
    runtime
        .as_ref()
        .map_err(|error| HostError::new(format!("create runtime delay executor failed: {error}")))
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

    /// Schedules an asynchronous task on the shared native runtime.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        _taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        delayRuntime()?.spawn(task);
        Ok(())
    }

    /// Starts a named native task after the requested delay.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        taskName: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        let name = taskName.to_string();
        delayRuntime()?.spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(delayMs)).await;
            std::thread::Builder::new()
                .name(name)
                .spawn(task)
                .expect("delayed runtime task thread must start");
        });
        Ok(())
    }
}
