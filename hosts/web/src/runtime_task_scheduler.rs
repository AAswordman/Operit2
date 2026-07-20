use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

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
        self.scheduleDelayedHostRuntimeTask(_taskName, 0, task)
    }

    /// Starts an asynchronous task on the browser's wasm future executor.
    fn scheduleHostRuntimeAsyncTask(
        &self,
        _taskName: &str,
        task: HostRuntimeAsyncTask,
    ) -> HostResult<()> {
        wasm_bindgen_futures::spawn_local(task());
        Ok(())
    }

    /// Enqueues the task through the browser timer queue after the requested delay.
    fn scheduleDelayedHostRuntimeTask(
        &self,
        _taskName: &str,
        delayMs: u64,
        task: HostRuntimeTask,
    ) -> HostResult<()> {
        let delayMs: i32 = delayMs
            .try_into()
            .map_err(|_| HostError::new("browser runtime task delay exceeds i32 milliseconds"))?;
        let callback = Closure::once_into_js(task);
        let function: &js_sys::Function = callback.unchecked_ref();
        let global = js_sys::global();
        let setTimeout = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
            .map_err(|error| {
                HostError::new(format!("read browser runtime timer failed: {error:?}"))
            })?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| HostError::new("browser runtime timer is unavailable"))?;
        setTimeout
            .call2(&global, function, &JsValue::from_f64(f64::from(delayMs)))
            .map(|_| ())
            .map_err(|error| {
                HostError::new(format!(
                    "schedule delayed browser runtime task failed: {error:?}"
                ))
            })
    }
}
