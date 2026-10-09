use crate::javascript_values::{self, JsonIntrinsics};
use operit_host_api::{
    HostError, HostJavaScriptAsyncJsonCallback, HostJavaScriptExecutionInterrupt,
    HostJavaScriptInterruptHandler, HostJavaScriptRuntime, HostJavaScriptRuntimeHost,
    HostJavaScriptRuntimeStateAsyncTask, HostJavaScriptRuntimeStateFactory,
    HostJavaScriptRuntimeStateHandle, HostJavaScriptRuntimeStateOutput,
    HostJavaScriptRuntimeStateOutputFuture, HostJavaScriptRuntimeStateTask,
    HostJavaScriptStringCallback, HostJavaScriptVoidCallback, HostResult,
};
use rquickjs::function::Rest;
use rquickjs::{
    CatchResultExt, Context, Error as QuickJsError, Exception, Function, Persistent, Promise,
    Runtime, Value,
};
use serde_json::Value as JsonValue;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
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

struct NativePendingPromise {
    scope: String,
    resolve: Persistent<Function<'static>>,
    reject: Persistent<Function<'static>>,
}

const MAX_PENDING_PROMISES: usize = 4096;

type NativePromises = Rc<RefCell<BTreeMap<u64, NativePendingPromise>>>;

struct NativeHostJavaScriptRuntime {
    runtime: Runtime,
    context: Context,
    promises: NativePromises,
    nextPromiseId: Rc<Cell<u64>>,
    jsonIntrinsics: Rc<RefCell<Option<JsonIntrinsics>>>,
}

impl Drop for NativeHostJavaScriptRuntime {
    fn drop(&mut self) {
        // Persistent handles must be released before the context/runtime. This
        // also breaks the registry -> resolver -> runtime -> binding -> registry cycle.
        self.promises.borrow_mut().clear();
        self.jsonIntrinsics.borrow_mut().take();
    }
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

    fn callHostJavaScriptFunction(
        &mut self,
        name: &str,
        arguments: &[JsonValue],
    ) -> HostResult<()> {
        self.context.with(|ctx| {
            let result = (|| {
                let value: Value = ctx.globals().get(name)?;
                if let Some(function) = value.into_function() {
                    let values = arguments
                        .iter()
                        .map(|arg| javascript_values::to_js(&ctx, arg))
                        .collect::<rquickjs::Result<Vec<_>>>()?;
                    function.call::<_, ()>((Rest(values),))?;
                }
                Ok::<_, QuickJsError>(())
            })();
            result
                .catch(&ctx)
                .map_err(|error| HostError::new(error.to_string()))
        })
    }

    fn registerHostJavaScriptAsyncJsonFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptAsyncJsonCallback,
    ) -> HostResult<bool> {
        let promises = self.promises.clone();
        let nextId = self.nextPromiseId.clone();
        let intrinsics = self.jsonIntrinsics.clone();
        self.context
            .with(|ctx| {
                if intrinsics.borrow().is_none() {
                    *intrinsics.borrow_mut() = Some(JsonIntrinsics::new(&ctx)?);
                }
                let function = structuredFunction(
                    ctx.clone(),
                    promises,
                    nextId,
                    Rc::downgrade(&intrinsics),
                    callback,
                )?;
                ctx.globals().set(name, function)?;
                Ok::<_, QuickJsError>(true)
            })
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn settleHostJavaScriptPromise(
        &mut self,
        id: u64,
        value: &JsonValue,
        reject: bool,
    ) -> HostResult<()> {
        let pending = self.promises.borrow_mut().remove(&id);
        let Some(pending) = pending else {
            return Ok(());
        };
        self.context
            .with(|ctx| {
                // A failed result conversion must still settle the Promise rather
                // than removing its handle and leaving the JS await pending forever.
                let value = match javascript_values::to_js(&ctx, value).catch(&ctx) {
                    Ok(value) => value,
                    Err(error) => {
                        pending
                            .reject
                            .restore(&ctx)?
                            .call::<_, ()>((error.to_string(),))?;
                        return Ok(());
                    }
                };
                let function = if reject {
                    pending.reject
                } else {
                    pending.resolve
                };
                function.restore(&ctx)?.call::<_, ()>((value,))
            })
            .map_err(|error| HostError::new(error.to_string()))
    }

    fn cancelHostJavaScriptPromises(&mut self, scope: &str) -> HostResult<()> {
        self.promises
            .borrow_mut()
            .retain(|_, pending| pending.scope != scope);
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
        Ok(Box::new(NativeHostJavaScriptRuntime {
            runtime,
            context,
            promises: Rc::new(RefCell::new(BTreeMap::new())),
            nextPromiseId: Rc::new(Cell::new(0)),
            jsonIntrinsics: Rc::new(RefCell::new(None)),
        }))
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

fn structuredFunction<'js>(
    ctx: rquickjs::Ctx<'js>,
    promises: NativePromises,
    nextId: Rc<Cell<u64>>,
    intrinsics: Weak<RefCell<Option<JsonIntrinsics>>>,
    callback: HostJavaScriptAsyncJsonCallback,
) -> rquickjs::Result<Function<'js>> {
    Function::new(
        ctx.clone(),
        move |ctx: rquickjs::Ctx<'js>, scope: String, args: Rest<Value<'js>>| {
            if promises.borrow().len() >= MAX_PENDING_PROMISES {
                return Err(Exception::throw_range(
                    &ctx,
                    "Too many pending structured tool calls",
                ));
            }
            let intrinsics = intrinsics.upgrade().ok_or(QuickJsError::Unknown)?;
            let intrinsicsGuard = intrinsics.borrow();
            let intrinsics = intrinsicsGuard.as_ref().ok_or(QuickJsError::Unknown)?;
            let values = args
                .0
                .into_iter()
                .map(|arg| {
                    // Legacy top-level undefined/function/symbol parameters became
                    // '{}'; objects omit them and arrays turn them into null.
                    intrinsics
                        .from_js(&ctx, arg)
                        .map(|v| v.unwrap_or_else(|| serde_json::json!({})))
                })
                .collect::<rquickjs::Result<Vec<_>>>()?;
            drop(intrinsicsGuard);
            // Getters/toJSON may re-enter this binding and create more requests.
            // Recheck after conversion before retaining another pair of handles.
            if promises.borrow().len() >= MAX_PENDING_PROMISES {
                return Err(Exception::throw_range(
                    &ctx,
                    "Too many pending structured tool calls",
                ));
            }
            let id = nextId
                .get()
                .checked_add(1)
                .ok_or_else(|| Exception::throw_range(&ctx, "Structured request id exhausted"))?;
            nextId.set(id);
            let (promise, resolve, reject) = Promise::new(&ctx)?;
            promises.borrow_mut().insert(
                id,
                NativePendingPromise {
                    scope,
                    resolve: Persistent::save(&ctx, resolve),
                    reject: Persistent::save(&ctx, reject),
                },
            );
            // No RefCell borrow is held across a callback that can re-enter JS.
            if let Err(error) = callback(id, values) {
                let pending = promises
                    .borrow_mut()
                    .remove(&id)
                    .expect("just inserted promise");
                pending
                    .reject
                    .restore(&ctx)?
                    .call::<_, ()>((error.to_string(),))?;
            }
            Ok::<_, QuickJsError>(promise)
        },
    )
}

#[cfg(test)]
mod bridge_tests {
    use super::*;
    use std::sync::Mutex;

    fn runtime() -> NativeHostJavaScriptRuntime {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        NativeHostJavaScriptRuntime {
            runtime,
            context,
            promises: Rc::new(RefCell::new(BTreeMap::new())),
            nextPromiseId: Rc::new(Cell::new(0)),
            jsonIntrinsics: Rc::new(RefCell::new(None)),
        }
    }

    #[test]
    fn direct_callback_preserves_arguments_and_does_not_evaluate_source() {
        let mut runtime = runtime();
        runtime
            .evaluateHostJavaScriptVoid(
                "test",
                "globalThis.called=[]; globalThis.cb=function(v,e){called.push([v,e]);};",
            )
            .unwrap();
        let text = "\"; globalThis.injected=true; //\n中\u{2028}".repeat(4096);
        runtime
            .callHostJavaScriptFunction(
                "cb",
                &[JsonValue::String(text.clone()), JsonValue::Bool(true)],
            )
            .unwrap();
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "called[0][0]")
                .unwrap(),
            text
        );
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "String(called[0][1])+':'+typeof injected")
                .unwrap(),
            "true:undefined"
        );
        runtime.callHostJavaScriptFunction("missing", &[]).unwrap();
        runtime
            .evaluateHostJavaScriptVoid("test", "globalThis.noncallable=42")
            .unwrap();
        runtime
            .callHostJavaScriptFunction("noncallable", &[])
            .unwrap();
        runtime
            .evaluateHostJavaScriptVoid(
                "test",
                "globalThis.fail=function(){throw new Error('failure');}",
            )
            .unwrap();
        let error = runtime.callHostJavaScriptFunction("fail", &[]).unwrap_err();
        assert!(error.to_string().contains("failure"));
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "String(1+1)")
                .unwrap(),
            "2"
        );
    }

    #[test]
    fn structured_promises_settle_once_and_cancellation_is_scoped() {
        let mut runtime = runtime();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = requests.clone();
        assert!(runtime
            .registerHostJavaScriptAsyncJsonFunction(
                "request",
                Arc::new(move |id, args| {
                    received.lock().unwrap().push((id, args));
                    Ok(())
                })
            )
            .unwrap());
        runtime.evaluateHostJavaScriptVoid("test", "globalThis.results=[]; request('first',{i:7}).then(v=>results.push(v)); request('second',{i:8}).then(v=>results.push(v));").unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests[0].1, vec![serde_json::json!({"i":7})]);
        assert_eq!(runtime.promises.borrow().len(), 2);
        runtime.cancelHostJavaScriptPromises("first").unwrap();
        assert_eq!(runtime.promises.borrow().len(), 1);
        runtime
            .settleHostJavaScriptPromise(
                requests[0].0,
                &serde_json::json!({"cancelled":true}),
                false,
            )
            .unwrap();
        runtime
            .settleHostJavaScriptPromise(
                requests[1].0,
                &serde_json::json!({"success":false,"message":"business"}),
                false,
            )
            .unwrap();
        runtime
            .settleHostJavaScriptPromise(
                requests[1].0,
                &serde_json::json!({"duplicate":true}),
                false,
            )
            .unwrap();
        runtime.executePendingHostJavaScriptJobs().unwrap();
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "JSON.stringify(results)")
                .unwrap(),
            "[{\"message\":\"business\",\"success\":false}]"
        );
        assert!(runtime.promises.borrow().is_empty());
    }

    #[test]
    fn pending_limit_survives_reentrant_parameter_getters() {
        let mut runtime = runtime();
        runtime
            .registerHostJavaScriptAsyncJsonFunction("request", Arc::new(|_, _| Ok(())))
            .unwrap();
        runtime.evaluateHostJavaScriptVoid("test", "for(let i=0;i<4095;i++)request('scope',{}); globalThis.limitFailure=false; try { request('scope',{get x(){request('scope',{});return 7;}}); } catch(e) {limitFailure=e instanceof RangeError;}").unwrap();
        assert_eq!(runtime.promises.borrow().len(), MAX_PENDING_PROMISES);
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "String(limitFailure)")
                .unwrap(),
            "true"
        );
        runtime.cancelHostJavaScriptPromises("scope").unwrap();
        assert!(runtime.promises.borrow().is_empty());
        runtime
            .evaluateHostJavaScriptVoid("test", "request('next',{})")
            .unwrap();
        assert_eq!(runtime.promises.borrow().len(), 1);
    }

    #[test]
    fn oversized_result_rejects_instead_of_leaving_a_pending_promise() {
        let mut runtime = runtime();
        let id = Arc::new(AtomicU64::new(0));
        let received = id.clone();
        runtime
            .registerHostJavaScriptAsyncJsonFunction(
                "request",
                Arc::new(move |id, _| {
                    received.store(id, Ordering::Relaxed);
                    Ok(())
                }),
            )
            .unwrap();
        runtime
            .evaluateHostJavaScriptVoid(
                "test",
                "globalThis.failure=''; request('scope',{}).catch(e=>failure=String(e));",
            )
            .unwrap();
        let mut result = JsonValue::Null;
        for _ in 0..140 {
            result = serde_json::json!({"child":result});
        }
        runtime
            .settleHostJavaScriptPromise(id.load(Ordering::Relaxed), &result, false)
            .unwrap();
        runtime.executePendingHostJavaScriptJobs().unwrap();
        assert!(runtime
            .evaluateHostJavaScriptString("test", "failure")
            .unwrap()
            .contains("limit"));
        assert!(runtime.promises.borrow().is_empty());
    }

    #[test]
    fn submission_failure_rejects_and_drop_releases_pending_handles() {
        let mut runtime = runtime();
        runtime
            .registerHostJavaScriptAsyncJsonFunction(
                "request",
                Arc::new(|_, _| Err(HostError::new("submission failure"))),
            )
            .unwrap();
        runtime
            .evaluateHostJavaScriptVoid(
                "test",
                "globalThis.failure=''; request('scope',{}).catch(e=>failure=String(e));",
            )
            .unwrap();
        runtime.executePendingHostJavaScriptJobs().unwrap();
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "failure")
                .unwrap(),
            "submission failure"
        );
        assert!(runtime.promises.borrow().is_empty());
        runtime
            .registerHostJavaScriptAsyncJsonFunction("pending", Arc::new(|_, _| Ok(())))
            .unwrap();
        runtime
            .evaluateHostJavaScriptVoid("test", "pending('scope',{});")
            .unwrap();
        assert_eq!(runtime.promises.borrow().len(), 1);
        // Dropping here must not abort QuickJS due to live Persistent handles.
    }
}
