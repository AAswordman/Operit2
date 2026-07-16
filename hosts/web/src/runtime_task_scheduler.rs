use operit_host_api::{HostError, HostResult, HostRuntimeTask, HostRuntimeTaskSchedulerHost};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// Schedules one-shot runtime tasks through the browser event queue.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebHostRuntimeTaskSchedulerHost;

impl WebHostRuntimeTaskSchedulerHost {
    /// Creates the browser runtime task scheduler host.
    pub fn new() -> Self {
        Self
    }
}

impl HostRuntimeTaskSchedulerHost for WebHostRuntimeTaskSchedulerHost {
    /// Enqueues the task after the current browser event completes.
    fn scheduleHostRuntimeTask(&self, _taskName: &str, task: HostRuntimeTask) -> HostResult<()> {
        let window =
            web_sys::window().ok_or_else(|| HostError::new("browser window is unavailable"))?;
        let callback = Closure::once_into_js(task);
        let function: &js_sys::Function = callback.unchecked_ref();
        window
            .set_timeout_with_callback_and_timeout_and_arguments_0(function, 0)
            .map(|_| ())
            .map_err(|error| {
                HostError::new(format!("schedule browser runtime task failed: {error:?}"))
            })
    }
}
