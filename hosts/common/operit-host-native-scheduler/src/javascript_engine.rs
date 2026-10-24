//! V8 execution behind the platform-independent Host JavaScript contract.
use crate::javascript_interrupt::JavaScriptInterrupt;
use crate::javascript_values::{self, engine_error, text, JsonIntrinsics};
use operit_host_api::{
    HostError, HostJavaScriptAsyncJsonCallback, HostJavaScriptBootstrap, HostJavaScriptCodeCache,
    HostJavaScriptInterruptHandler, HostJavaScriptCallback, HostJavaScriptRuntime, HostResult, HostJavaScriptValue,
};
use serde_json::Value as JsonValue;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, Once};

pub(super) const MAX_PENDING_PROMISES: usize = 4096;

pub(super) struct NativePendingPromise {
    scope: String,
    resolver: v8::Global<v8::PromiseResolver>,
}

type NativePromises = Rc<RefCell<BTreeMap<u64, NativePendingPromise>>>;

#[derive(Clone)]
enum HostBinding {
    Snapshot,
    Synchronous {
        callback: HostJavaScriptCallback,
        profile: bool,
    },
    Asynchronous(HostJavaScriptAsyncJsonCallback),
    Constructor {
        prefix: String,
        cache: Arc<HostJavaScriptCodeCache>,
    },
}

struct RuntimeBindings {
    intrinsics: JsonIntrinsics,
    functions: RefCell<Vec<HostBinding>>,
    promises: NativePromises,
    next_promise: Cell<u64>,
}

pub(super) struct NativeHostJavaScriptRuntime {
    bootstrap: Arc<HostJavaScriptBootstrap>,
    code_cache: Arc<HostJavaScriptCodeCache>,
    pub(super) promises: NativePromises,
    bindings: Option<Rc<RuntimeBindings>>,
    context: Option<v8::Global<v8::Context>>,
    interrupt: Arc<JavaScriptInterrupt>,
    isolate: Option<v8::OwnedIsolate>,
}

/// Balances every temporary isolate entry, including unwinding from a Host callback.
struct IsolateEntry(*const v8::Isolate);

impl IsolateEntry {
    /// Enters an affine isolate for the duration of a single Host operation.
    fn new(isolate: &v8::Isolate) -> Self {
        // SAFETY: Host runtimes and their handles remain on their owning executor.
        unsafe {
            isolate.enter();
        }
        Self(isolate)
    }
}

impl Drop for IsolateEntry {
    /// Restores the previously entered isolate before another runtime is used.
    fn drop(&mut self) {
        // SAFETY: this guard outlives its handle scopes and the isolate still exists.
        unsafe {
            (*self.0).exit();
        }
    }
}

impl NativeHostJavaScriptRuntime {
    /// Initializes V8 once and captures all realm-local state before plugin execution.
    pub(super) fn new(
        bootstrap: Arc<HostJavaScriptBootstrap>,
        code_cache: Arc<HostJavaScriptCodeCache>,
    ) -> HostResult<Self> {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
        let mut parameters = v8::CreateParams::default();
        if crate::runtime_task_scheduler::hasManagedWorkerStack() {
            let stack_marker = 0u32;
            let limit = (std::ptr::addr_of!(stack_marker) as usize - 4 * 1024 * 1024) as *mut u32;
            // SAFETY: Host-managed workers reserve 16 MiB, leaving room for Rust callbacks.
            parameters = unsafe { parameters.set_stack_limit(limit) };
        }
        let mut isolate = v8::Isolate::new(parameters);
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        let interrupt = JavaScriptInterrupt::new(isolate.thread_safe_handle())?;
        isolate.set_slot(interrupt.clone());
        let promises = NativePromises::default();
        let (context, bindings) = {
            v8::scope!(let scope, &mut isolate);
            let context = v8::Context::new(scope, Default::default());
            let scope = &mut v8::ContextScope::new(scope, context);
            v8::tc_scope!(let scope, scope);
            let intrinsics =
                JsonIntrinsics::new(scope).map_err(|error| match scope.exception() {
                    Some(exception) => HostError::new(exception.to_rust_string_lossy(scope)),
                    None => error,
                })?;
            let bindings = Rc::new(RuntimeBindings {
                intrinsics,
                functions: RefCell::new(Vec::new()),
                promises: promises.clone(),
                next_promise: Cell::new(0),
            });
            scope.set_slot(bindings.clone());
            install_binding(
                scope,
                "__operitHostSnapshotJsonObject",
                HostBinding::Snapshot,
                &bindings,
                true,
            )?;
            (v8::Global::new(scope, context), bindings)
        };
        // SAFETY: creation entered the isolate; each operation explicitly re-enters it.
        unsafe {
            isolate.exit();
        }
        Ok(Self {
            bootstrap,
            code_cache,
            promises,
            bindings: Some(bindings),
            context: Some(context),
            interrupt,
            isolate: Some(isolate),
        })
    }

    /// Executes one operation with the owning context, exact JS exceptions and balanced scopes.
    pub(super) fn with_scope<R>(
        &mut self,
        operation: impl FnOnce(&mut v8::PinScope<'_, '_>) -> HostResult<R>,
    ) -> HostResult<R> {
        let isolate = self.isolate.as_mut().expect("live V8 isolate");
        let _entry = IsolateEntry::new(isolate);
        v8::scope!(let scope, isolate);
        let context = v8::Local::new(scope, self.context.as_ref().expect("live V8 context"));
        let scope = &mut v8::ContextScope::new(scope, context);
        v8::tc_scope!(let scope, scope);
        let result = operation(scope);
        if let Some(panic) = self.interrupt.take_panic() {
            scope.cancel_terminate_execution();
            std::panic::resume_unwind(panic);
        }
        if scope.has_terminated() || scope.is_execution_terminating() {
            scope.cancel_terminate_execution();
            return Err(HostError::new("JavaScript execution was interrupted"));
        }
        if let Some(exception) = scope.exception() {
            return Err(HostError::new(exception.to_rust_string_lossy(scope)));
        }
        result
    }

    /// Installs a function whose Rust callback and engine handles belong to this isolate.
    fn register(&mut self, name: &str, binding: HostBinding) -> HostResult<()> {
        let bindings = self.bindings.as_ref().expect("live V8 bindings").clone();
        self.with_scope(|scope| install_binding(scope, name, binding, &bindings, false))
    }
}

impl Drop for NativeHostJavaScriptRuntime {
    /// Releases all persistent handles while their owning isolate is entered and alive.
    fn drop(&mut self) {
        self.interrupt.set(None);
        let mut isolate = self.isolate.take().expect("live V8 isolate");
        // SAFETY: this affine runtime is being destroyed on its owning executor.
        unsafe {
            isolate.enter();
        }
        self.promises.borrow_mut().clear();
        isolate.remove_slot::<Arc<JavaScriptInterrupt>>();
        isolate.remove_slot::<Rc<RuntimeBindings>>();
        self.bindings.take();
        self.context.take();
        drop(isolate);
    }
}

impl HostJavaScriptRuntime for NativeHostJavaScriptRuntime {
    /// Shares trusted compiled code while keeping each bootstrap's globals in its own context.
    fn evaluateHostJavaScriptBootstrap(&mut self, source: &'static str) -> HostResult<()> {
        let bootstrap = self.bootstrap.clone();
        self.with_scope(|scope| {
            let strict_source = format!("\"use strict\";\n{source}");
            let bytes = bootstrap.bytecode(source, || compile_bytes(scope, &strict_source))?;
            let script = consume_cache(scope, &strict_source, bytes)?;
            script.run(scope).ok_or_else(engine_error)?;
            Ok(())
        })
    }

    /// Creates realm-local module factories from a shared source-keyed V8 code cache.
    fn registerHostJavaScriptFunctionConstructor(
        &mut self,
        name: &str,
        parameters: &[&str],
    ) -> HostResult<()> {
        self.register(
            name,
            HostBinding::Constructor {
                prefix: format!("(function({}){{\n", parameters.join(",")),
                cache: self.code_cache.clone(),
            },
        )
    }

    /// Evaluates a named V8 script and discards its result.
    fn evaluateHostJavaScriptVoid(&mut self, scriptName: &str, script: &str) -> HostResult<()> {
        self.with_scope(|scope| {
            evaluate(scope, scriptName, script)?;
            Ok(())
        })
    }

    /// Evaluates a named V8 script and requires a string result.
    fn evaluateHostJavaScriptString(
        &mut self,
        scriptName: &str,
        script: &str,
    ) -> HostResult<String> {
        self.with_scope(|scope| {
            let value = evaluate(scope, scriptName, script)?;
            if !value.is_string() {
                return Err(javascript_values::type_error(
                    scope,
                    "JavaScript result must be a string",
                ));
            }
            Ok(value.to_rust_string_lossy(scope))
        })
    }

    /// Drains explicit V8 microtasks on the owning executor.
    fn executePendingHostJavaScriptJobs(&mut self) -> HostResult<()> {
        self.with_scope(|scope| {
            scope.perform_microtask_checkpoint();
            Ok(())
        })
    }

    /// Replaces cancellation without moving the isolate or any JavaScript handles.
    fn setHostJavaScriptInterruptHandler(
        &mut self,
        handler: Option<HostJavaScriptInterruptHandler>,
    ) -> HostResult<()> {
        self.interrupt.set(handler);
        Ok(())
    }

    /// Invokes a required lifecycle callback using direct structured argument and result conversion.
    fn callHostJavaScriptFunction(
        &mut self,
        name: &str,
        arguments: &[JsonValue],
    ) -> HostResult<JsonValue> {
        let bindings = self.bindings.as_ref().expect("live V8 bindings").clone();
        self.with_scope(|scope| {
            let context = scope.get_current_context();
            let key = text(scope, name)?;
            let value = context
                .global(scope)
                .get(scope, key.into())
                .ok_or_else(engine_error)?;
            let function = v8::Local::<v8::Function>::try_from(value).map_err(|_| {
                javascript_values::type_error(
                    scope,
                    &format!("Required JavaScript function is not callable: {name}"),
                )
            })?;
            let profile = name == "__operitExecuteScriptFunction"
                && arguments.get(6) == Some(&JsonValue::Bool(true))
                && std::env::var_os("OPERIT_PROFILE_COMPOSE_STAGES").is_some();
            let started = profile.then(std::time::Instant::now);
            let values = arguments
                .iter()
                .map(|value| javascript_values::to_js(scope, value))
                .collect::<HostResult<Vec<_>>>()?;
            if let Some(started) = started {
                profile_stage("rust-to-js-input", started, arguments.first().and_then(JsonValue::as_str));
            }
            let receiver = v8::undefined(scope).into();
            let value = function
                .call(scope, receiver, &values)
                .ok_or_else(engine_error)?;
            if value.is_undefined() {
                return Ok(JsonValue::Null);
            }
            bindings.intrinsics.from_js(scope, value)?.ok_or_else(|| {
                javascript_values::type_error(
                    scope,
                    "JavaScript lifecycle result must be a JSON-compatible value",
                )
            })
        })
    }

    /// Registers a structured Promise binding owned by this runtime.
    fn registerHostJavaScriptAsyncJsonFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptAsyncJsonCallback,
    ) -> HostResult<()> {
        self.register(name, HostBinding::Asynchronous(callback))
    }

    /// Settles a Promise exactly once and rejects invalid results instead of leaving awaits suspended.
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
        self.with_scope(|scope| {
            let resolver = v8::Local::new(scope, &pending.resolver);
            let converted = {
                v8::tc_scope!(let scope, scope);
                javascript_values::to_js(scope, &value).map_err(|error| match scope.exception() {
                    Some(exception) => exception.to_rust_string_lossy(scope),
                    None => error.to_string(),
                })
            };
            let settled = match converted {
                Ok(value) => {
                    if reject {
                        resolver.reject(scope, value)
                    } else {
                        resolver.resolve(scope, value)
                    }
                }
                Err(error) => {
                    let message = text(scope, &error)?;
                    resolver.reject(scope, message.into())
                }
            };
            if settled != Some(true) {
                return Err(engine_error());
            }
            Ok(())
        })
    }

    /// Rejects only the cancelled execution scope and releases its completion handles.
    fn cancelHostJavaScriptPromises(&mut self, scope: &str) -> HostResult<()> {
        let ids = self
            .promises
            .borrow()
            .iter()
            .filter_map(|(id, pending)| (pending.scope == scope).then_some(*id))
            .collect::<Vec<_>>();
        for id in ids {
            self.settleHostJavaScriptPromise(
                id,
                JsonValue::String("JavaScript execution was cancelled".into()),
                true,
            )?;
        }
        Ok(())
    }

    /// Registers a synchronous structured binding without JSON text transport.
    fn registerHostJavaScriptFunction(
        &mut self,
        name: &str,
        callback: HostJavaScriptCallback,
    ) -> HostResult<()> {
        let profile = name == "__operitNativeSendStructuredIntermediate"
            && std::env::var_os("OPERIT_PROFILE_COMPOSE_STAGES").is_some();
        self.register(name, HostBinding::Synchronous { callback, profile })
    }
}

/// Installs callback metadata in the isolate and exposes its function as an own property.
fn install_binding(
    scope: &mut v8::PinScope<'_, '_>,
    name: &str,
    binding: HostBinding,
    bindings: &RuntimeBindings,
    immutable: bool,
) -> HostResult<()> {
    let index = bindings.functions.borrow().len();
    let data = v8::Integer::new(
        scope,
        i32::try_from(index).map_err(|error| HostError::new(error.to_string()))?,
    );
    let function = v8::Function::builder(host_callback)
        .data(data.into())
        .build(scope)
        .ok_or_else(engine_error)?;
    let name = text(scope, name)?;
    let global = scope.get_current_context().global(scope);
    let attributes = if immutable {
        v8::PropertyAttribute::READ_ONLY
            | v8::PropertyAttribute::DONT_DELETE
            | v8::PropertyAttribute::DONT_ENUM
    } else {
        v8::PropertyAttribute::NONE
    };
    if global.define_own_property(scope, name.into(), function.into(), attributes) != Some(true) {
        return Err(engine_error());
    }
    bindings.functions.borrow_mut().push(binding);
    Ok(())
}

/// Converts callback failures into the original JavaScript exception or a Host Error.
fn host_callback<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    arguments: v8::FunctionCallbackArguments<'s>,
    mut result: v8::ReturnValue,
) {
    v8::tc_scope!(let scope, scope);
    let output =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dispatch(scope, arguments)));
    match output {
        Ok(Ok(value)) => result.set(value),
        Err(panic) => {
            scope
                .get_slot::<Arc<JavaScriptInterrupt>>()
                .expect("V8 interrupt state")
                .capture_panic(panic);
            scope.terminate_execution();
        }
        Ok(Err(error)) => {
            if scope.has_caught() {
                scope.rethrow();
            } else {
                let message =
                    v8::String::new(scope, &error.to_string()).expect("Host error message");
                let exception = v8::Exception::error(scope, message);
                scope.throw_exception(exception);
                scope.rethrow();
            }
        }
    }
}

/// Dispatches cloned callback metadata without retaining a RefCell borrow across JavaScript.
fn dispatch<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    arguments: v8::FunctionCallbackArguments<'s>,
) -> HostResult<v8::Local<'s, v8::Value>> {
    let bindings = scope
        .get_slot::<Rc<RuntimeBindings>>()
        .expect("V8 Host bindings")
        .clone();
    let index = arguments
        .data()
        .int32_value(scope)
        .ok_or_else(engine_error)? as usize;
    let binding = bindings.functions.borrow()[index].clone();
    match binding {
        HostBinding::Snapshot => bindings.intrinsics.snapshot_owned(scope, arguments.get(0)),
        HostBinding::Synchronous { callback, profile } => {
            let started = profile.then(std::time::Instant::now);
            let values = bindings.intrinsics.arguments_from_js(
                scope, (0..arguments.length()).map(|index| arguments.get(index)),
            )?;
            if let Some(started) = started {
                profile_stage("js-to-rust", started, values.first().and_then(operit_host_api::HostJavaScriptValue::as_json).and_then(JsonValue::as_str));
            }
            let result = callback(values)?;
            javascript_values::to_js(scope, &result)
        }
        HostBinding::Asynchronous(callback) => {
            if bindings.promises.borrow().len() >= MAX_PENDING_PROMISES {
                return Err(javascript_values::range_error(
                    scope,
                    "Too many pending structured tool calls",
                ));
            }
            let owner = arguments.get(0);
            if !owner.is_string() {
                return Err(javascript_values::type_error(
                    scope,
                    "Structured request scope must be a string",
                ));
            }
            let owner = owner.to_rust_string_lossy(scope);
            let values = bindings.intrinsics.arguments_from_js(
                scope,
                (1..arguments.length()).map(|index| arguments.get(index)),
            )?;
            let values = values.into_iter().map(HostJavaScriptValue::into_json).collect::<HostResult<Vec<_>>>()?;
            if bindings.promises.borrow().len() >= MAX_PENDING_PROMISES {
                return Err(javascript_values::range_error(
                    scope,
                    "Too many pending structured tool calls",
                ));
            }
            let id = bindings.next_promise.get().checked_add(1).ok_or_else(|| {
                javascript_values::range_error(scope, "Structured request id exhausted")
            })?;
            bindings.next_promise.set(id);
            let resolver = v8::PromiseResolver::new(scope).ok_or_else(engine_error)?;
            let promise = resolver.get_promise(scope);
            bindings.promises.borrow_mut().insert(
                id,
                NativePendingPromise {
                    scope: owner,
                    resolver: v8::Global::new(scope, resolver),
                },
            );
            if let Err(error) = callback(id, values) {
                let pending = bindings
                    .promises
                    .borrow_mut()
                    .remove(&id)
                    .expect("just submitted V8 Promise");
                let resolver = v8::Local::new(scope, &pending.resolver);
                let message = text(scope, &error.to_string())?;
                if resolver.reject(scope, message.into()) != Some(true) {
                    return Err(engine_error());
                }
            }
            Ok(promise.into())
        }
        HostBinding::Constructor { prefix, cache } => {
            let body = arguments.get(0);
            if !body.is_string() {
                return Err(javascript_values::type_error(
                    scope,
                    "JavaScript function body must be a string",
                ));
            }
            let source = format!("{prefix}{}\n}})", body.to_rust_string_lossy(scope));
            let bytes = cache.getOrCompile(&source, || compile_bytes(scope, &source))?;
            let value = consume_cache(scope, &source, &bytes)?
                .run(scope)
                .ok_or_else(engine_error)?;
            let function =
                v8::Local::<v8::Function>::try_from(value).map_err(|_| engine_error())?;
            function.set_name(text(scope, "anonymous")?);
            Ok(value)
        }
    }
}

/// Compiles a named script in the current realm without executing it.
fn compile<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    name: &str,
    script: &str,
) -> HostResult<v8::Local<'s, v8::Script>> {
    let source = text(scope, script)?;
    let name = text(scope, name)?;
    let origin = v8::ScriptOrigin::new(
        scope,
        name.into(),
        0,
        0,
        false,
        -1,
        None,
        false,
        false,
        false,
        None,
    );
    let mut source = v8::script_compiler::Source::new(source, Some(&origin));
    v8::script_compiler::compile(
        scope,
        &mut source,
        v8::script_compiler::CompileOptions::EagerCompile,
        v8::script_compiler::NoCacheReason::NoReason,
    )
    .ok_or_else(engine_error)
}

/// Runs a named script and retains its original exception in the enclosing TryCatch.
fn evaluate<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    name: &str,
    script: &str,
) -> HostResult<v8::Local<'s, v8::Value>> {
    compile(scope, name, script)?
        .run(scope)
        .ok_or_else(engine_error)
}

/// Serializes trusted V8 compiled code and preserves exact compilation failures in the shared cache.
fn compile_bytes(scope: &mut v8::PinScope<'_, '_>, source: &str) -> HostResult<Vec<u8>> {
    v8::tc_scope!(let scope, scope);
    let result = compile(scope, "operit.compiled.js", source).and_then(|script| {
        script
            .get_unbound_script(scope)
            .create_code_cache()
            .map(|bytes| bytes.to_vec())
            .ok_or_else(engine_error)
    });
    match scope.exception() {
        Some(exception) => {
            let error = HostError::new(exception.to_rust_string_lossy(scope));
            scope.rethrow();
            Err(error)
        }
        None => result,
    }
}

/// Consumes only code produced by this Host's V8 build and rejects an invalid cache explicitly.
fn consume_cache<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    source: &str,
    bytes: &[u8],
) -> HostResult<v8::Local<'s, v8::Script>> {
    let source = text(scope, source)?;
    let name = text(scope, "operit.compiled.js")?;
    let origin = v8::ScriptOrigin::new(
        scope,
        name.into(),
        0,
        0,
        false,
        -1,
        None,
        false,
        false,
        false,
        None,
    );
    let mut source = v8::script_compiler::Source::new_with_cached_data(
        source,
        Some(&origin),
        v8::script_compiler::CachedData::new(bytes),
    );
    let script = v8::script_compiler::compile(
        scope,
        &mut source,
        v8::script_compiler::CompileOptions::ConsumeCodeCache,
        v8::script_compiler::NoCacheReason::NoReason,
    )
    .ok_or_else(engine_error)?;
    if source
        .get_cached_data()
        .expect("supplied V8 code cache")
        .rejected()
    {
        return Err(HostError::new("V8 rejected the Host JavaScript code cache"));
    }
    Ok(script)
}

/// Preserves the compose pipeline timings emitted at the structured engine boundary.
fn profile_stage(
    stage: &str,
    started: std::time::Instant,
    call: Option<&str>,
) {
    let duration = started.elapsed().as_secs_f64() * 1000.0;
    let wall = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_micros();
    let mut line = format!(
        "event=compose_pipeline_stage stage={stage} durationMs={duration:.3} wallMicros={wall}"
    );
    if let Some(call) = call {
        line.push_str(" call=");
        line.push_str(call);
    }
    line.push('\n');
    let _ = std::io::Write::write_all(&mut std::io::stderr().lock(), line.as_bytes());
}
