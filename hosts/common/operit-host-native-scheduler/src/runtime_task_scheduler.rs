use operit_host_api::{HostError, HostResult, HostRuntimeTask, HostRuntimeTaskSchedulerHost};

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
}
