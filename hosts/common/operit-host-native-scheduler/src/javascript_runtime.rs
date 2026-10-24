use crate::javascript_engine::{NativeHostJavaScriptRuntime, MAX_PENDING_PROMISES};
use operit_host_api::{
    HostError, HostJavaScriptBootstrap, HostJavaScriptCodeCache, HostJavaScriptExecutionInterrupt,
    HostJavaScriptRuntime, HostJavaScriptRuntimeHost, HostJavaScriptRuntimeStateAsyncTask,
    HostJavaScriptRuntimeStateFactory, HostJavaScriptRuntimeStateHandle,
    HostJavaScriptRuntimeStateOutput, HostJavaScriptRuntimeStateOutputFuture,
    HostJavaScriptRuntimeStateTask, HostResult,
};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

enum NativeJavaScriptStateRequest {
    Enqueue {
        task: HostJavaScriptRuntimeStateTask,
        interrupt: HostJavaScriptExecutionInterrupt,
    },
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

/// Owns native V8 runtimes and their dedicated affine state threads.
#[derive(Clone, Default)]
pub struct NativeHostJavaScriptRuntimeHost {
    bootstrap: Arc<HostJavaScriptBootstrap>,
    codeCache: Arc<HostJavaScriptCodeCache>,
    nextStateId: Arc<AtomicU64>,
    stateWorkers:
        Arc<Mutex<BTreeMap<u64, tokio::sync::mpsc::UnboundedSender<NativeJavaScriptStateRequest>>>>,
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
    ) -> HostResult<tokio::sync::mpsc::UnboundedSender<NativeJavaScriptStateRequest>> {
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
    /// Creates one native V8 runtime on its owning thread.
    fn createHostJavaScriptRuntime(&self) -> HostResult<Box<dyn HostJavaScriptRuntime>> {
        Ok(Box::new(NativeHostJavaScriptRuntime::new(
            self.bootstrap.clone(),
            self.codeCache.clone(),
        )?))
    }

    /// Creates one affine JavaScript state on the shared Host engine executor.
    fn createHostJavaScriptRuntimeState(
        &self,
        taskName: &str,
        kind: operit_host_api::HostJavaScriptRuntimeStateKind,
        factory: HostJavaScriptRuntimeStateFactory,
    ) -> HostResult<HostJavaScriptRuntimeStateHandle> {
        let stateId = self.nextStateId.fetch_add(1, Ordering::Relaxed) + 1;
        let (requestSender, mut requestReceiver) =
            tokio::sync::mpsc::unbounded_channel::<NativeJavaScriptStateRequest>();
        let (createdSender, createdReceiver) = mpsc::channel::<HostResult<()>>();
        crate::NativeHostRuntimeTaskSchedulerHost::new().scheduleJavaScriptStateTask(
            taskName,
            kind,
            Box::new(move || {
                Box::pin(async move {
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
                    while let Some(request) = requestReceiver.recv().await {
                        match request {
                            NativeJavaScriptStateRequest::Enqueue { task, interrupt } => {
                                if let Err(error) = task(state.as_mut(), interrupt) {
                                    operit_host_api::logHostError(
                                        "JavaScriptRuntime",
                                        &error.to_string(),
                                    );
                                }
                            }
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
                                let result = task(state.as_mut(), interrupt).await;
                                let _ = response.send(result);
                            }
                            NativeJavaScriptStateRequest::Shutdown => break,
                        }
                    }
                })
            }),
        )?;
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
        interrupt: HostJavaScriptExecutionInterrupt,
        task: HostJavaScriptRuntimeStateTask,
    ) -> HostResult<HostJavaScriptRuntimeStateOutput> {
        let worker = self.stateWorker(handle)?;
        let (response, receiver) = mpsc::channel();
        worker
            .send(NativeJavaScriptStateRequest::Execute {
                task,
                interrupt: interrupt.clone(),
                response,
            })
            .map_err(|error| HostError::new(error.to_string()))?;
        receiver
            .recv()
            .map_err(|_| HostError::new("JavaScript runtime state worker disconnected"))?
    }

    /// Posts directly to the existing affine owner without a pool task or completion channel.
    fn enqueueHostJavaScriptRuntimeStateTask(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
        interrupt: HostJavaScriptExecutionInterrupt,
        task: HostJavaScriptRuntimeStateTask,
    ) -> HostResult<()> {
        let worker = self.stateWorker(handle)?;
        worker
            .send(NativeJavaScriptStateRequest::Enqueue { task, interrupt })
            .map_err(|error| HostError::new(error.to_string()))
    }

    /// Executes one asynchronous operation on a native JavaScript state thread.
    fn executeHostJavaScriptRuntimeStateAsyncTask(
        &self,
        handle: HostJavaScriptRuntimeStateHandle,
        interrupt: HostJavaScriptExecutionInterrupt,
        task: HostJavaScriptRuntimeStateAsyncTask,
    ) -> HostJavaScriptRuntimeStateOutputFuture {
        let worker = self.stateWorker(handle);
        Box::pin(async move {
            let worker = worker?;
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

#[cfg(test)]
mod bridge_tests {
    use super::*;
    use std::sync::Mutex;

    /// Creates an isolated native runtime for the required host contract tests.
    fn runtime() -> NativeHostJavaScriptRuntime {
        NativeHostJavaScriptRuntime::new(
            Arc::new(HostJavaScriptBootstrap::default()),
            Arc::new(HostJavaScriptCodeCache::default()),
        )
        .unwrap()
    }

    /// Keeps mutable globals and host bindings independent across contexts sharing compiled bytes.
    #[test]
    fn shared_bootstrap_preserves_runtime_isolation_and_host_binding_ownership() {
        let host = NativeHostJavaScriptRuntimeHost::new();
        let mut first = host.createHostJavaScriptRuntime().unwrap();
        let mut second = host.createHostJavaScriptRuntime().unwrap();
        for (runtime, owner) in [(&mut first, "first"), (&mut second, "second")] {
            runtime
                .registerHostJavaScriptFunction(
                    "readOwner",
                    Arc::new(move |_| Ok(JsonValue::String(owner.into()))),
                )
                .unwrap();
            runtime.evaluateHostJavaScriptBootstrap(
                "var owned = { count: 0 }; function next() { return readOwner() + ':' + (++owned.count); }").unwrap();
        }
        assert_eq!(
            first
                .evaluateHostJavaScriptString("probe", "next()")
                .unwrap(),
            "first:1"
        );
        assert_eq!(
            first
                .evaluateHostJavaScriptString("probe", "next()")
                .unwrap(),
            "first:2"
        );
        assert_eq!(
            second
                .evaluateHostJavaScriptString("probe", "next()")
                .unwrap(),
            "second:1"
        );
    }

    /// Deserialized functions resolve globals and callbacks in their own runtime.
    #[test]
    fn shared_module_code_cache_keeps_globals_exports_and_host_bindings_local() {
        let host = NativeHostJavaScriptRuntimeHost::new();
        for owner in ["first", "second"] {
            let mut runtime = host.createHostJavaScriptRuntime().unwrap();
            runtime
                .registerHostJavaScriptFunction(
                    "readOwner",
                    Arc::new(move |_| Ok(JsonValue::String(owner.into()))),
                )
                .unwrap();
            runtime
                .registerHostJavaScriptFunctionConstructor(
                    "compileModule",
                    &["module", "exports", "require", "runtime"],
                )
                .unwrap();
            runtime.evaluateHostJavaScriptVoid("test", r#"
                var count = 0;
                var module = {exports:{}};
                var factory = compileModule('exports.next = () => readOwner() + ":" + (++count); module.exports.required = require(runtime);');
                factory(module, module.exports, value => value + ':required', 'context');
            "#).unwrap();
            assert_eq!(runtime.evaluateHostJavaScriptString("probe", "module.exports.next() + ':' + module.exports.next() + ':' + module.exports.required").unwrap(),
                format!("{owner}:1:{owner}:2:context:required"));
        }
    }

    /// A code_cache hit still creates a fresh function and preserves Function body strictness.
    #[test]
    fn module_constructor_preserves_function_scope_and_recovers_after_syntax_errors() {
        let mut runtime = runtime();
        runtime
            .registerHostJavaScriptFunctionConstructor("compileModule", &["value"])
            .unwrap();
        let result = runtime.evaluateHostJavaScriptString("probe", r#"
            (function() {
                var local = 'lexical';
                var a = compileModule('return [this === globalThis, typeof local, typeof anonymous, value];');
                var b = compileModule('return [this === globalThis, typeof local, typeof anonymous, value];');
                var strict = compileModule('"use strict"; return this === undefined;');
                var syntax;
                try { compileModule('return ('); } catch (e) { syntax = e instanceof SyntaxError; }
                return JSON.stringify([a !== b, a.name, a(7), strict(), syntax, compileModule('return value + 1;')(9)]);
            })()
        "#).unwrap();
        assert_eq!(
            result,
            r#"[true,"anonymous",[true,"undefined","undefined",7],true,true,10]"#
        );
    }

    /// Rejects a changed host bootstrap instead of executing bytes for another source.
    #[test]
    fn shared_bootstrap_rejects_source_changes() {
        let host = NativeHostJavaScriptRuntimeHost::new();
        let mut first = host.createHostJavaScriptRuntime().unwrap();
        first
            .evaluateHostJavaScriptBootstrap("var version = 1;")
            .unwrap();
        let mut second = host.createHostJavaScriptRuntime().unwrap();
        let failure = second
            .evaluateHostJavaScriptBootstrap("var version = 2;")
            .unwrap_err();
        assert_eq!(
            failure.message,
            "A JavaScript host cannot change its bootstrap source"
        );
        assert_eq!(
            second
                .evaluateHostJavaScriptString("probe", "typeof version")
                .unwrap(),
            "undefined"
        );
    }

    /// Preserves a compilation failure across contexts without recompiling or replacing it.
    #[test]
    fn shared_bootstrap_keeps_exact_compilation_errors() {
        let host = NativeHostJavaScriptRuntimeHost::new();
        let mut first = host.createHostJavaScriptRuntime().unwrap();
        let failure = first
            .evaluateHostJavaScriptBootstrap("function {")
            .unwrap_err();
        let mut second = host.createHostJavaScriptRuntime().unwrap();
        assert_eq!(
            second
                .evaluateHostJavaScriptBootstrap("function {")
                .unwrap_err(),
            failure
        );
    }

    /// Rejects absent lifecycle bindings instead of silently ignoring a required engine contract.
    #[test]
    fn lifecycle_callbacks_require_a_callable_binding() {
        let mut runtime = runtime();
        assert!(runtime.callHostJavaScriptFunction("missing", &[]).is_err());
        runtime
            .evaluateHostJavaScriptVoid("test", "globalThis.callback=42;")
            .unwrap();
        assert!(runtime.callHostJavaScriptFunction("callback", &[]).is_err());
        runtime
            .evaluateHostJavaScriptVoid(
                "test",
                "globalThis.callback=function(value){globalThis.delivered=value;};",
            )
            .unwrap();
        runtime
            .callHostJavaScriptFunction("callback", &[JsonValue::String("data".to_string())])
            .unwrap();
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "delivered")
                .unwrap(),
            "data"
        );
    }

    /// Uses the same structured contract for synchronous arguments, direct results and host exceptions.
    #[test]
    fn synchronous_bindings_preserve_values_and_throw_errors_without_json_text() {
        let mut runtime = runtime();
        runtime
            .registerHostJavaScriptFunction("echo", Arc::new(|mut args| args.remove(0).into_json()))
            .unwrap();
        runtime
            .registerHostJavaScriptFunction(
                "literal",
                Arc::new(|_| {
                    Ok(JsonValue::String(
                        r#"{"success":false,"message":"literal"}"#.to_string(),
                    ))
                }),
            )
            .unwrap();
        runtime
            .registerHostJavaScriptFunction(
                "fail",
                Arc::new(|_| Err(HostError::new("operation denied"))),
            )
            .unwrap();
        runtime.evaluateHostJavaScriptVoid("test", r#"
            JSON.parse = JSON.stringify = function() { throw Error('JSON text used'); };
            globalThis.copied = echo({ nested: {value:7}, array:[undefined, , NaN], ['__proto__']:{safe:true} });
            globalThis.literalValue = literal();
            try { fail(); } catch (error) { globalThis.failureMessage = error.message; }
        "#).unwrap();
        assert_eq!(runtime.evaluateHostJavaScriptString("test", "String(copied.nested.value===7 && copied.array.every(value=>value===null) && Object.hasOwn(copied,'__proto__') && copied.safe===undefined)").unwrap(), "true");
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "literalValue")
                .unwrap(),
            r#"{"success":false,"message":"literal"}"#
        );
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "failureMessage")
                .unwrap(),
            "operation denied"
        );
        assert!(runtime.promises.borrow().is_empty());
    }

    /// Rejects unsupported top-level values before retaining a native Promise handle.
    #[test]
    fn structured_bindings_reject_non_json_arguments() {
        let mut runtime = runtime();
        runtime
            .registerHostJavaScriptAsyncJsonFunction("request", Arc::new(|_, _| Ok(())))
            .unwrap();
        runtime
            .evaluateHostJavaScriptVoid(
                "test",
                r#"
            globalThis.failures=0;
            for (const value of [undefined, function(){}, Symbol('value')]) {
                try { request('scope', value); } catch (error) { failures++; }
            }
        "#,
            )
            .unwrap();
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "String(failures)")
                .unwrap(),
            "3"
        );
        assert!(runtime.promises.borrow().is_empty());
    }

    /// Cancellation rejects only its scope, runs cleanup once, and ignores late host results.
    #[test]
    fn structured_promises_settle_once_and_cancellation_is_scoped() {
        let mut runtime = runtime();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received = requests.clone();
        runtime
            .registerHostJavaScriptAsyncJsonFunction(
                "request",
                Arc::new(move |id, args| {
                    received.lock().unwrap().push((id, args));
                    Ok(())
                }),
            )
            .unwrap();
        runtime
            .evaluateHostJavaScriptVoid(
                "test",
                r#"
            globalThis.results=[];
            globalThis.cancelledReason='';
            globalThis.cleaned=0;
            request('first',{i:7}).then(
                value=>results.push(value),
                reason=>{cancelledReason=reason;},
            ).finally(()=>{cleaned++;});
            request('second',{i:8}).then(value=>results.push(value));
        "#,
            )
            .unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests[0].1, vec![serde_json::json!({"i":7})]);
        assert_eq!(runtime.promises.borrow().len(), 2);
        runtime.cancelHostJavaScriptPromises("first").unwrap();
        assert_eq!(runtime.promises.borrow().len(), 1);
        runtime
            .settleHostJavaScriptPromise(
                requests[0].0,
                serde_json::json!({"cancelled":true}),
                false,
            )
            .unwrap();
        runtime
            .settleHostJavaScriptPromise(
                requests[1].0,
                serde_json::json!({"success":false,"message":"business"}),
                false,
            )
            .unwrap();
        runtime
            .settleHostJavaScriptPromise(
                requests[1].0,
                serde_json::json!({"duplicate":true}),
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
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "cancelledReason")
                .unwrap(),
            "JavaScript execution was cancelled"
        );
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("test", "String(cleaned)")
                .unwrap(),
            "1"
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
            .settleHostJavaScriptPromise(id.load(Ordering::Relaxed), result, false)
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
        // Dropping here must not abort V8 due to live Persistent handles.
    }
    /// Interrupts CPU-bound V8 execution and preserves the realm for the next operation.
    #[test]
    fn interrupts_busy_v8_execution_and_reuses_the_runtime() {
        let mut runtime = runtime();
        let armed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let predicate = armed.clone();
        let owner_thread = std::thread::current().id();
        let checked_thread = Arc::new(Mutex::new(None));
        let observed_thread = checked_thread.clone();
        runtime
            .setHostJavaScriptInterruptHandler(Some(Arc::new(move || {
                *observed_thread.lock().unwrap() = Some(std::thread::current().id());
                predicate.load(Ordering::SeqCst)
            })))
            .unwrap();
        let cancel = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            armed.store(true, Ordering::SeqCst);
        });
        let failure = runtime
            .evaluateHostJavaScriptVoid("busy", "globalThis.saved=7;while(true){}")
            .unwrap_err();
        cancel.join().unwrap();
        assert_eq!(*checked_thread.lock().unwrap(), Some(owner_thread));
        assert_eq!(failure.message, "JavaScript execution was interrupted");
        runtime.setHostJavaScriptInterruptHandler(None).unwrap();
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("resumed", "String(saved+1)")
                .unwrap(),
            "8"
        );
    }

    /// Permits independent runtimes on one executor to be dropped in any order.
    #[test]
    fn runtimes_keep_isolate_entries_balanced_when_dropped_out_of_order() {
        let host = NativeHostJavaScriptRuntimeHost::new();
        let mut first = host.createHostJavaScriptRuntime().unwrap();
        let mut second = host.createHostJavaScriptRuntime().unwrap();
        first
            .evaluateHostJavaScriptVoid("first", "var owner='first'")
            .unwrap();
        second
            .evaluateHostJavaScriptVoid("second", "var owner='second'")
            .unwrap();
        drop(first);
        assert_eq!(
            second
                .evaluateHostJavaScriptString("second", "owner")
                .unwrap(),
            "second"
        );
    }
    /// Propagates Rust callback panics after leaving the V8 ABI without aborting the process.
    #[test]
    fn rust_callback_panics_resume_on_the_owning_rust_stack() {
        let mut runtime = runtime();
        runtime
            .registerHostJavaScriptFunction(
                "panicHost",
                Arc::new(|_| std::panic::panic_any(17u32)),
            )
            .unwrap();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.evaluateHostJavaScriptVoid("panic", "panicHost()")
        }))
        .unwrap_err();
        assert_eq!(*panic.downcast::<u32>().unwrap(), 17);
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("resumed", "String(1+1)")
                .unwrap(),
            "2"
        );
    }

    /// Propagates interrupt predicate panics without unwinding through a V8 safepoint callback.
    #[test]
    fn rust_interrupt_panics_resume_on_the_owning_rust_stack() {
        let mut runtime = runtime();
        runtime
            .setHostJavaScriptInterruptHandler(Some(Arc::new(|| std::panic::panic_any(23u32))))
            .unwrap();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            runtime.evaluateHostJavaScriptVoid("panic", "while(true){}")
        }))
        .unwrap_err();
        assert_eq!(*panic.downcast::<u32>().unwrap(), 23);
        runtime.setHostJavaScriptInterruptHandler(None).unwrap();
        assert_eq!(
            runtime
                .evaluateHostJavaScriptString("resumed", "String(1+1)")
                .unwrap(),
            "2"
        );
    }
}
