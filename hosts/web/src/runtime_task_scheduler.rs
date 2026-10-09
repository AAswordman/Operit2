use operit_host_api::{
    HostError, HostResult, HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTaskSchedulerHost,
    HostRuntimeTurnFuture,
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

/// Waits for a browser-owned timer while exposing a scheduler-safe completion future.
fn waitForBrowserRuntimeDelay(delayMs: u64) -> HostRuntimeTurnFuture {
    let delayMs: i32 = match delayMs.try_into() {
        Ok(delayMs) => delayMs,
        Err(_) => {
            return Box::pin(async {
                Err(HostError::new(
                    "browser runtime delay exceeds i32 milliseconds",
                ))
            });
        }
    };
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let callback = Closure::once_into_js(move || {
        let _ = sender.send(());
    });
    let function: &js_sys::Function = callback.unchecked_ref();
    let global = js_sys::global();
    let scheduleResult = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
        .and_then(|value| value.dyn_into::<js_sys::Function>())
        .and_then(|setTimeout| {
            setTimeout.call2(&global, function, &JsValue::from_f64(f64::from(delayMs)))
        });
    match scheduleResult {
        Ok(_) => Box::pin(async move {
            receiver
                .await
                .map_err(|_| HostError::new("browser runtime timer was cancelled"))
        }),
        Err(error) => Box::pin(async move {
            Err(HostError::new(format!(
                "schedule browser runtime timer failed: {error:?}"
            )))
        }),
    }
}

impl HostRuntimeTaskSchedulerHost for WebHostRuntimeTaskSchedulerHost {
    /// Reads the browser's monotonic performance clock on the active worker.
    fn monotonicTimeMillis(&self) -> HostResult<u64> {
        let global = js_sys::global();
        let performance = js_sys::Reflect::get(&global, &JsValue::from_str("performance"))
            .map_err(|error| HostError::new(format!("read browser performance clock failed: {error:?}")))?;
        let now = js_sys::Reflect::get(&performance, &JsValue::from_str("now"))
            .map_err(|error| HostError::new(format!("read browser monotonic clock method failed: {error:?}")))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| HostError::new("browser monotonic clock is unavailable"))?;
        let value = now.call0(&performance)
            .map_err(|error| HostError::new(format!("read browser monotonic clock failed: {error:?}")))?
            .as_f64().ok_or_else(|| HostError::new("browser monotonic clock returned a nonnumeric value"))?;
        if !value.is_finite() || value < 0.0 || value >= u64::MAX as f64 {
            return Err(HostError::new("browser monotonic clock returned an invalid value"));
        }
        Ok(value as u64)
    }

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

    /// Executes cooperative work on the host-owned asynchronous executor.
    fn scheduleHostRuntimeCooperativeAsyncTask(
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

    /// Waits for the browser worker to advance to a later event-loop turn.
    fn waitForHostRuntimeTaskTurn(&self) -> HostRuntimeTurnFuture {
        waitForBrowserRuntimeDelay(0)
    }

    /// Waits through the browser timer queue for one platform-owned delay.
    fn waitForHostRuntimeDelay(&self, delayMs: u64) -> HostRuntimeTurnFuture {
        waitForBrowserRuntimeDelay(delayMs)
    }
}
