use operit_host_api::{
    HostError, HostJavaScriptExecutionInterrupt, HostJavaScriptInterruptHandler,
    HostJavaScriptRuntime, HostJavaScriptRuntimeHost, HostJavaScriptRuntimeState,
    HostJavaScriptRuntimeStateAsyncTask, HostJavaScriptRuntimeStateFactory,
    HostJavaScriptRuntimeStateHandle, HostJavaScriptRuntimeStateOutput,
    HostJavaScriptRuntimeStateOutputFuture, HostJavaScriptRuntimeStateTask,
    HostJavaScriptStringCallback, HostJavaScriptVoidCallback, HostResult,
};
use rquickjs::function::Rest;
use rquickjs::{CatchResultExt, Context, Error as QuickJsError, Function, Runtime};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

enum NativeJavaScriptStateRequest {
    Execute {
        task: HostJavaScriptRuntimeStateTask,
        interrupt: HostJavaScriptExecutionInterrupt,
        response: mpsc::Sender<HostResult<HostJavaScriptRuntimeStateOutput>>,
    },
    ExecuteAsync {
        task: HostJavaScriptRuntimeStateAsyncTask,
        interrupt: HostJavaScriptExecutionInterrupt,
        response: tokio::sync::oneshot::Sender<HostResult<HostJavaScriptRuntimeStateOutput>>,
    },
    Shutdown,
}

struct NativeHostJavaScriptRuntime {
    runtime: Runtime,
    context: Context,
}

impl HostJavaScriptRuntime for NativeHostJavaScriptRuntime {
    /// Evaluates one native QuickJS script without reading its return value.
    fn evaluateHostJavaScriptVoid(&mut self, _scriptName: &str, script: &str) -> HostResult<()> {
        self.context.with(|ctx| {
            ctx.eval::<(), _>(script)
                .catch(&ctx)
                .map_err(|error| HostError::new(error.to_string()))
        })
    }

    /// Evaluates one native QuickJS script and converts its return value to a string.
    fn evaluateHostJavaScriptString(
        &mut self,
        _scriptName: &str,
        script: &str,
    ) -> HostResult<String> {
        self.context.with(|ctx| {
            ctx.eval::<String, _>(script)
                .catch(&ctx)
                .map_err(|error| HostError::new(error.to_string()))
        })
    }

    /// Executes every native QuickJS job currently ready in this runtime.
    fn executePendingHostJavaScriptJobs(&mut self) -> HostResult<()> {
        while self.context.with(|ctx| ctx.execute_pending_job()) {}
        Ok(())
    }

    /// Replaces the native QuickJS interrupt predicate.
    fn setHostJavaScriptInterruptHandler(
        &mut self,
        handler: Option<HostJavaScriptInterruptHandler>,
    ) -> HostResult<()> {
        self.runtime.set_interrupt_handler(
            handler
                .map(|handler| Box::new(move || handler()) as Box<dyn FnMut() -> bool + 'static>),
        );
        Ok(())
    }

    /// Registers one native QuickJS global function returning a string.
    fn registerHostJavaScriptStringFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptStringCallback,
    ) -> HostResult<()> {
        self.context.with(|ctx| {
            let function = Function::new(ctx.clone(), move |args: Rest<String>| {
                callback(args.0).map_err(|error| {
                    QuickJsError::new_from_js_message(
                        "Host callback",
                        "JavaScript",
                        error.to_string(),
                    )
                })
            })
            .map_err(|error| HostError::new(error.to_string()))?;
            ctx.globals()
                .set(name, function)
                .map_err(|error| HostError::new(error.to_string()))
        })
    }

    /// Registers one native QuickJS global function returning `undefined`.
    fn registerHostJavaScriptVoidFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptVoidCallback,
    ) -> HostResult<()> {
        self.context.with(|ctx| {
            let function = Function::new(ctx.clone(), move |args: Rest<String>| {
                callback(args.0).map_err(|error| {
                    QuickJsError::new_from_js_message(
                        "Host callback",
                        "JavaScript",
                        error.to_string(),
                    )
                })
            })
            .map_err(|error| HostError::new(error.to_string()))?;
            ctx.globals()
                .set(name, function)
                .map_err(|error| HostError::new(error.to_string()))
        })
    }
}

/// Owns native QuickJS runtimes and their dedicated affine state threads.
#[derive(Clone, Default)]
pub struct NativeHostJavaScriptRuntimeHost {
    nextStateId: Arc<AtomicU64>,
    stateWorkers: Arc<Mutex<BTreeMap<u64, mpsc::Sender<NativeJavaScriptStateRequest>>>>,
}

impl NativeHostJavaScriptRuntimeHost {
    /// Creates the native JavaScript runtime host.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the worker sender for one exact JavaScript runtime state.
    fn stateWorker(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
    ) -> HostResult<mpsc::Sender<NativeJavaScriptStateRequest>> {
        self.stateWorkers
            .lock()
            .expect("native JavaScript state registry mutex poisoned")
            .get(&handle.id)
            .cloned()
            .ok_or_else(|| {
                HostError::new(format!(
                    "JavaScript runtime state does not exist: {}",
                    handle.id
                ))
            })
    }
}

impl HostJavaScriptRuntimeHost for NativeHostJavaScriptRuntimeHost {
    /// Creates one native QuickJS runtime on its owning thread.
    fn createHostJavaScriptRuntime(&self) -> HostResult<Box<dyn HostJavaScriptRuntime>> {
        let runtime = Runtime::new().map_err(|error| HostError::new(error.to_string()))?;
        let context = Context::full(&runtime).map_err(|error| HostError::new(error.to_string()))?;
        Ok(Box::new(NativeHostJavaScriptRuntime { runtime, context }))
    }

    /// Creates one affine JavaScript state on a dedicated native thread.
    fn createHostJavaScriptRuntimeState(
        &self,
        taskName: &str,
        factory: HostJavaScriptRuntimeStateFactory,
    ) -> HostResult<HostJavaScriptRuntimeStateHandle> {
        let stateId = self.nextStateId.fetch_add(1, Ordering::Relaxed) + 1;
        let (requestSender, requestReceiver) = mpsc::channel::<NativeJavaScriptStateRequest>();
        let (createdSender, createdReceiver) = mpsc::channel::<HostResult<()>>();
        std::thread::Builder::new()
            .name(format!("{taskName}-{stateId}"))
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut state = match factory() {
                    Ok(state) => {
                        let _ = createdSender.send(Ok(()));
                        state
                    }
                    Err(error) => {
                        let _ = createdSender.send(Err(error));
                        return;
                    }
                };
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("native JavaScript state executor must start");
                for request in requestReceiver {
                    match request {
                        NativeJavaScriptStateRequest::Execute {
                            task,
                            interrupt,
                            response,
                        } => {
                            let _ = response.send(task(state.as_mut(), interrupt));
                        }
                        NativeJavaScriptStateRequest::ExecuteAsync {
                            task,
                            interrupt,
                            response,
                        } => {
                            let result = runtime.block_on(task(state.as_mut(), interrupt));
                            let _ = response.send(result);
                        }
                        NativeJavaScriptStateRequest::Shutdown => break,
                    }
                }
            })
            .map_err(|error| {
                HostError::new(format!(
                    "create JavaScript runtime state thread {taskName} failed: {error}"
                ))
            })?;
        createdReceiver.recv().map_err(|error| {
            HostError::new(format!(
                "JavaScript runtime state creation disconnected: {error}"
            ))
        })??;
        self.stateWorkers
            .lock()
            .expect("native JavaScript state registry mutex poisoned")
            .insert(stateId, requestSender);
        Ok(HostJavaScriptRuntimeStateHandle { id: stateId })
    }

    /// Executes one blocking operation on a native JavaScript state thread.
    fn executeHostJavaScriptRuntimeStateTask(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
        timeoutMillis: u64,
        task: HostJavaScriptRuntimeStateTask,
    ) -> HostResult<HostJavaScriptRuntimeStateOutput> {
        let worker = self.stateWorker(handle)?;
        let interrupt = HostJavaScriptExecutionInterrupt::new(timeoutMillis)?;
        let (response, receiver) = mpsc::channel();
        worker
            .send(NativeJavaScriptStateRequest::Execute {
                task,
                interrupt: interrupt.clone(),
                response,
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        match receiver.recv_timeout(Duration::from_millis(timeoutMillis)) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                interrupt.interrupt();
                Err(HostError::timeout(format!(
                    "JavaScript runtime state task timed out after {timeoutMillis} milliseconds"
                )))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(HostError::new(
                "JavaScript runtime state worker disconnected",
            )),
        }
    }

    /// Executes one asynchronous operation on a native JavaScript state thread.
    fn executeHostJavaScriptRuntimeStateAsyncTask(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
        timeoutMillis: u64,
        task: HostJavaScriptRuntimeStateAsyncTask,
    ) -> HostJavaScriptRuntimeStateOutputFuture {
        let worker = self.stateWorker(handle);
        Box::pin(async move {
            let worker = worker?;
            let interrupt = HostJavaScriptExecutionInterrupt::new(timeoutMillis)?;
            let (response, receiver) = tokio::sync::oneshot::channel();
            worker
                .send(NativeJavaScriptStateRequest::ExecuteAsync {
                    task,
                    interrupt,
                    response,
                })
                .map_err(|error| HostError::new(error.to_string()))?;
            receiver
                .await
                .map_err(|_| HostError::new("JavaScript runtime state worker disconnected"))?
        })
    }

    /// Destroys one native JavaScript state and stops its owning thread.
    fn destroyHostJavaScriptRuntimeState(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
    ) -> HostResult<()> {
        let worker = self
            .stateWorkers
            .lock()
            .expect("native JavaScript state registry mutex poisoned")
            .remove(&handle.id)
            .ok_or_else(|| {
                HostError::new(format!(
                    "JavaScript runtime state does not exist: {}",
                    handle.id
                ))
            })?;
        worker
            .send(NativeJavaScriptStateRequest::Shutdown)
            .map_err(|error| HostError::new(error.to_string()))
    }
}
