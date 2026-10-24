//! QuickJS engine backend for mobile hosts (Android / iOS / OpenHarmony).
//!
//! The `v8` crate ships no prebuilt bindings for `aarch64-linux-android` (nor
//! any Android target), so those platforms compile QuickJS from source through
//! rquickjs's `bindgen` feature. This module mirrors the pre-V8 runtime
//! behaviour: same `HostJavaScriptRuntime` contract, same source-keyed
//! bootstrap bytecode, same bounded pending-Promise registry.

use crate::javascript_values::{self, JsonIntrinsics};
use operit_host_api::{
    HostError, HostJavaScriptAsyncJsonCallback, HostJavaScriptBootstrap, HostJavaScriptCallback,
    HostJavaScriptCodeCache, HostJavaScriptInterruptHandler, HostJavaScriptRuntime, HostResult,
};
use rquickjs::function::Rest;
use rquickjs::{
    qjs, CatchResultExt, Ctx, Context, Error as QuickJsError, Exception, Function, Persistent,
    Promise, Runtime, Value,
};
use serde_json::Value as JsonValue;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Bounds the pending structured-call registry so a stuck host cannot grow it.
pub(super) const MAX_PENDING_PROMISES: usize = 4096;

/// Keeps the QuickJS engine stack below the managed worker reservation.
const JAVASCRIPT_STATE_ENGINE_STACK_BYTES: usize = 4 * 1024 * 1024;

/// One pending structured call: its owning scope plus both completion handles.
struct NativePendingPromise {
    scope: String,
    resolve: Persistent<Function<'static>>,
    reject: Persistent<Function<'static>>,
}

type NativePromises = Rc<RefCell<BTreeMap<u64, NativePendingPromise>>>;

/// Owns one native QuickJS runtime on its owning executor thread.
pub(super) struct NativeHostJavaScriptRuntime {
    bootstrap: Arc<HostJavaScriptBootstrap>,
    #[allow(dead_code)]
    code_cache: Arc<HostJavaScriptCodeCache>,
    runtime: Runtime,
    context: Context,
    promises: NativePromises,
    nextPromiseId: Rc<Cell<u64>>,
    jsonIntrinsics: Rc<RefCell<Option<JsonIntrinsics>>>,
}

/// Registers one realm-local function factory backed by the shared code cache.
///
/// Kept as a free function with an explicit `'js` so the closure passed to
/// [`Function::new`] is typed as `Fn(Ctx<'js>, String) -> Result<Value<'js>>`
/// exactly: an elided `'_` in the closure's return type creates an independent
/// region that cannot satisfy `IntoJsFunc`'s bound.
fn register_function_constructor<'js>(
    ctx: Ctx<'js>,
    name: &str,
    prefix: String,
    cache: Arc<HostJavaScriptCodeCache>,
) -> rquickjs::Result<()> {
    let function = Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>, body: String| {
            let source = format!("{prefix}{body}\n}})");
            let bytes = cache
                .getOrCompile(&source, || compile_bootstrap(&ctx, &source))
                .map_err(|error| Exception::throw_message(&ctx, &error.to_string()))?;
            let raw = unsafe {
                qjs::JS_ReadObject(
                    ctx.as_raw().as_ptr(),
                    bytes.as_ptr(),
                    bytes.len() as _,
                    qjs::JS_READ_OBJ_BYTECODE as _,
                )
            };
            if unsafe { qjs::JS_IsException(raw) } {
                let error = bootstrap_exception(&ctx);
                return Err(Exception::throw_message(&ctx, &error.to_string()));
            }
            // JS_EvalFunction consumes the deserialized function.
            let result = unsafe { qjs::JS_EvalFunction(ctx.as_raw().as_ptr(), raw) };
            if unsafe { qjs::JS_IsException(result) } {
                let error = bootstrap_exception(&ctx);
                return Err(Exception::throw_message(&ctx, &error.to_string()));
            }
            Ok(unsafe { Value::from_raw(ctx.clone(), result) })
        },
    )?;
    ctx.globals().set(name, function)?;
    Ok(())
}

impl Drop for NativeHostJavaScriptRuntime {
    /// Releases engine-owned persistent handles before the runtime is destroyed.
    fn drop(&mut self) {
        // Persistent handles must be released before the context/runtime. This
        // also breaks the registry -> resolver -> runtime -> binding -> registry cycle.
        self.promises.borrow_mut().clear();
        self.jsonIntrinsics.borrow_mut().take();
    }
}

impl NativeHostJavaScriptRuntime {
    /// Creates one QuickJS runtime on the calling (owning) executor thread.
    pub(super) fn new(
        bootstrap: Arc<HostJavaScriptBootstrap>,
        code_cache: Arc<HostJavaScriptCodeCache>,
    ) -> HostResult<Self> {
        let runtime = Runtime::new().map_err(|error| HostError::new(error.to_string()))?;
        // Only managed workers have the known stack reservation; caller-owned
        // threads retain QuickJS's default bound rather than guessing.
        if crate::runtime_task_scheduler::hasManagedWorkerStack() {
            runtime.set_max_stack_size(JAVASCRIPT_STATE_ENGINE_STACK_BYTES);
        }
        let context = Context::full(&runtime).map_err(|error| HostError::new(error.to_string()))?;
        Ok(Self {
            bootstrap,
            code_cache,
            runtime,
            context,
            promises: Rc::new(RefCell::new(BTreeMap::new())),
            nextPromiseId: Rc::new(Cell::new(0)),
            jsonIntrinsics: Rc::new(RefCell::new(None)),
        })
    }
}

impl HostJavaScriptRuntime for NativeHostJavaScriptRuntime {
    /// Compiles the trusted bootstrap once per host and executes it in an isolated context.
    fn evaluateHostJavaScriptBootstrap(&mut self, source: &'static str) -> HostResult<()> {
        self.context.with(|ctx| {
            let bytes = self
                .bootstrap
                .bytecode(source, || compile_bootstrap(&ctx, source))?;
            // The bytes originate from this host's QuickJS build and never cross a storage or plugin boundary.
            let raw = unsafe {
                qjs::JS_ReadObject(
                    ctx.as_raw().as_ptr(),
                    bytes.as_ptr(),
                    bytes.len() as _,
                    qjs::JS_READ_OBJ_BYTECODE as _,
                )
            };
            if unsafe { qjs::JS_IsException(raw) } {
                return Err(bootstrap_exception(&ctx));
            }
            // JS_EvalFunction consumes the deserialized function, including on failure.
            let result = unsafe { qjs::JS_EvalFunction(ctx.as_raw().as_ptr(), raw) };
            if unsafe { qjs::JS_IsException(result) } {
                return Err(bootstrap_exception(&ctx));
            }
            drop(unsafe { Value::from_raw(ctx.clone(), result) });
            Ok(())
        })
    }

    /// Creates realm-local function factories from a shared source-keyed code cache.
    fn registerHostJavaScriptFunctionConstructor(
        &mut self,
        name: &str,
        parameters: &[&str],
    ) -> HostResult<()> {
        let prefix = format!("(function({}){{\n", parameters.join(","));
        let cache = self.code_cache.clone();
        self.context
            .with(|ctx| {
                register_function_constructor(ctx, name, prefix, cache)
                    .map_err(|error| HostError::new(error.to_string()))
            })
    }

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

    /// Calls a required lifecycle function using captured structured argument and result conversion.
    fn callHostJavaScriptFunction(
        &mut self,
        name: &str,
        arguments: &[JsonValue],
    ) -> HostResult<JsonValue> {
        let intrinsics = self.jsonIntrinsics.clone();
        self.context.with(|ctx| {
            let result = (|| {
                if intrinsics.borrow().is_none() {
                    *intrinsics.borrow_mut() = Some(JsonIntrinsics::new(&ctx)?);
                }
                let function: Function = ctx.globals().get(name)?;
                let values = arguments
                    .iter()
                    .map(|arg| javascript_values::to_js(&ctx, arg))
                    .collect::<rquickjs::Result<Vec<_>>>()?;
                let result: Value = function.call((Rest(values),))?;
                if result.is_undefined() {
                    return Ok(JsonValue::Null);
                }
                intrinsics
                    .borrow()
                    .as_ref()
                    .ok_or(QuickJsError::Unknown)?
                    .from_js(&ctx, result)?
                    .ok_or_else(|| {
                        Exception::throw_type(
                            &ctx,
                            "Host lifecycle result must be a JSON-compatible value",
                        )
                    })
            })();
            result
                .catch(&ctx)
                .map_err(|error| HostError::new(error.to_string()))
        })
    }

    /// Registers one native structured Promise binding on the owning executor.
    fn registerHostJavaScriptAsyncJsonFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptAsyncJsonCallback,
    ) -> HostResult<()> {
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
                Ok::<_, QuickJsError>(())
            })
            .map_err(|error| HostError::new(error.to_string()))
    }

    /// Settles one native Promise and releases both persistent completion handles.
    fn settleHostJavaScriptPromise(
        &mut self,
        id: u64,
        value: JsonValue,
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
                let value = match javascript_values::to_js(&ctx, &value).catch(&ctx) {
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

    /// Removes only the native Promise handles belonging to the cancelled scope.
    fn cancelHostJavaScriptPromises(&mut self, scope: &str) -> HostResult<()> {
        self.promises
            .borrow_mut()
            .retain(|_, pending| pending.scope != scope);
        Ok(())
    }

    /// Converts synchronous native host requests and results through captured value intrinsics.
    fn registerHostJavaScriptFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptCallback,
    ) -> HostResult<()> {
        let intrinsics = self.jsonIntrinsics.clone();
        self.context
            .with(|ctx| {
                if intrinsics.borrow().is_none() {
                    *intrinsics.borrow_mut() = Some(JsonIntrinsics::new(&ctx)?);
                }
                let intrinsics = Rc::downgrade(&intrinsics);
                let function = synchronousFunction(ctx.clone(), intrinsics, callback)?;
                ctx.globals().set(name, function)?;
                Ok::<_, QuickJsError>(())
            })
            .map_err(|error| HostError::new(error.to_string()))
    }
}

/// Captures the current QuickJS exception without replacing its message.
fn bootstrap_exception(ctx: &Ctx<'_>) -> HostError {
    let failure: Result<(), rquickjs::Error> = Err(rquickjs::Error::Exception);
    HostError::new(failure.catch(ctx).unwrap_err().to_string())
}

/// Serializes a trusted global script without executing bindings or retaining runtime handles.
fn compile_bootstrap(ctx: &Ctx<'_>, source: &str) -> HostResult<Vec<u8>> {
    let source = CString::new(source).map_err(|error| HostError::new(error.to_string()))?;
    let raw = unsafe {
        qjs::JS_Eval(
            ctx.as_raw().as_ptr(),
            source.as_ptr(),
            source.as_bytes().len() as _,
            c"operit.bootstrap.js".as_ptr(),
            (qjs::JS_EVAL_TYPE_GLOBAL | qjs::JS_EVAL_FLAG_STRICT | qjs::JS_EVAL_FLAG_COMPILE_ONLY)
                as _,
        )
    };
    if unsafe { qjs::JS_IsException(raw) } {
        return Err(bootstrap_exception(ctx));
    }
    let compiled = unsafe { Value::from_raw(ctx.clone(), raw) };
    let mut length = 0;
    let buffer = unsafe {
        qjs::JS_WriteObject(
            ctx.as_raw().as_ptr(),
            &mut length,
            compiled.as_raw(),
            qjs::JS_WRITE_OBJ_BYTECODE as _,
        )
    };
    if buffer.is_null() {
        return Err(bootstrap_exception(ctx));
    }
    // Copy host-owned bytes before freeing QuickJS's allocation on its owning executor.
    let bytes = unsafe { std::slice::from_raw_parts(buffer, length as usize).to_vec() };
    unsafe { qjs::js_free(ctx.as_raw().as_ptr(), buffer.cast()) };
    Ok(bytes)
}

/// Creates a synchronous value binding whose engine handles stay on the owning executor.
fn synchronousFunction<'js>(
    ctx: rquickjs::Ctx<'js>,
    intrinsics: Weak<RefCell<Option<JsonIntrinsics>>>,
    callback: HostJavaScriptCallback,
) -> rquickjs::Result<Function<'js>> {
    Function::new(
        ctx.clone(),
        move |ctx: rquickjs::Ctx<'js>, args: Rest<Value<'js>>| {
            let intrinsics = intrinsics.upgrade().ok_or(QuickJsError::Unknown)?;
            let guard = intrinsics.borrow();
            let converter = guard.as_ref().ok_or(QuickJsError::Unknown)?;
            let values = converter.arguments_from_js(&ctx, args.0)?;
            drop(guard);
            // The host boundary owns structured values; buffers cannot cross it.
            let arguments = values
                .into_iter()
                .map(operit_host_api::HostJavaScriptValue::from)
                .collect::<Vec<_>>();
            let result = callback(arguments)
                .map_err(|error| Exception::throw_message(&ctx, &error.to_string()))?;
            javascript_values::to_js(&ctx, &result)
        },
    )
}

/// Creates a scoped native Promise from owned JSON-compatible request arguments.
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
            let values = intrinsics.arguments_from_js(&ctx, args.0)?;
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
