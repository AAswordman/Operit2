use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

use operit_host_api::HostManager::{
    defaultHostJavaScriptRuntimeHost, defaultHostRuntimeTaskSchedulerHost,
};
use operit_host_api::{
    HostError, HostJavaScriptRuntime, HostJavaScriptRuntimeHost, HostJavaScriptRuntimeStateHandle,
    HostJavaScriptRuntimeStateOutput, HostResult,
};
use operit_plugin_sdk::toolpkg::ToolPkgComposeDslBridge::{
    buildComposeDslCompilerDefinition, readComposeDslParserSource,
};
use operit_util::AppLogger::AppLogger;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Argument, CallExpression, Expression, IdentifierReference};
use oxc_ast_visit::{walk, Visit};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;
use serde_json::Value;
use tokio::sync::{oneshot, watch, OnceCell};

const COMPILER_COUNT: usize = 4;
const CACHE_BYTES: usize = 16 * 1024 * 1024;
const CACHE_ENTRIES: usize = 128;
type CompiledSource = HostResult<Arc<CompiledModule>>;
type Completion = watch::Receiver<Option<CompiledSource>>;

/// Describes the complete resource scope of synchronous CommonJS module loading.
#[derive(Debug, Default, Eq, PartialEq)]
struct ModuleDependencies {
    paths: BTreeSet<String>,
    dynamic: bool,
}

/// Shares lowering and dependency analysis as one immutable preparation result.
#[derive(Debug, Eq, PartialEq)]
struct CompiledModule {
    script: Arc<str>,
    dependencies: ModuleDependencies,
}

impl<'a> Visit<'a> for ModuleDependencies {
    /// Records direct literal imports; computed calls and escaped loaders require the resource scope.
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = &call.callee {
            if callee.name == "require" {
                match call.arguments.as_slice() {
                    [Argument::StringLiteral(path)] => {
                        self.paths.insert(path.value.to_string());
                    }
                    _ => self.dynamic = true,
                }
                for argument in &call.arguments {
                    self.visit_argument(argument);
                }
                return;
            }
            if callee.name == "eval" {
                self.dynamic = true;
            }
        }
        walk::walk_call_expression(self, call);
    }

    /// Treats loader aliases as dynamic without trying to infer arbitrary plugin control flow.
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if identifier.name == "require" {
            self.dynamic = true;
        }
    }
}

/// Normalizes module paths using the same segment rules as the CommonJS loader.
pub(super) fn normalizeModulePath(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

/// Resolves the loader's ordered candidates against the immutable resource snapshot.
fn resolveModule<'a>(
    resources: &'a BTreeMap<String, String>,
    from: &str,
    request: &str,
) -> Option<(&'a String, &'a String)> {
    let request = request.replace('\\', "/");
    let request = request.trim();
    if !request.starts_with('.') && !request.starts_with('/') {
        return None;
    }
    let path = if request.starts_with('/') {
        normalizeModulePath(request)
    } else {
        let base = from
            .rsplit_once('/')
            .map(|(base, _)| base)
            .unwrap_or_default();
        normalizeModulePath(&format!("{base}/{request}"))
    }
    .to_ascii_lowercase();
    let extension = path.rsplit_once('.').map(|(_, extension)| extension);
    let explicit = extension.is_some_and(|extension| {
        !extension.is_empty() && extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
    });
    let candidates = if explicit {
        vec![path]
    } else {
        vec![
            path.clone(),
            format!("{path}.js"),
            format!("{path}.json"),
            format!("{path}/index.js"),
            format!("{path}/index.json"),
        ]
    };
    candidates
        .iter()
        .find_map(|path| resources.get_key_value(path))
}

/// Parses dependencies structurally, retaining syntax errors for the actual module load.
fn moduleDependencies(source: &str) -> ModuleDependencies {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::cjs())
        .with_options(ParseOptions {
            allow_return_outside_function: true,
            ..ParseOptions::default()
        })
        .parse();
    let mut dependencies = ModuleDependencies::default();
    if parsed.diagnostics.is_empty() {
        dependencies.visit_program(&parsed.program);
    }
    dependencies
}

/// Identifies exact source revisions without trusting hashes or realm-local source IDs.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
struct ModuleKey {
    identity: Arc<str>,
    source: Arc<str>,
}

/// Keeps lowered results alive for an immutable page snapshot, including unused failures.
pub(super) struct PreparedComposeSources {
    pub(super) resources: Arc<BTreeMap<String, String>>,
    root: ModuleKey,
    modules: BTreeMap<ModuleKey, CompiledSource>,
}

impl PreparedComposeSources {
    /// Reuses preparation only for the same immutable resources and exact root revision.
    pub(super) fn matches(
        &self,
        resources: &Arc<BTreeMap<String, String>>,
        script: &str,
        package: &str,
        screen: &str,
    ) -> bool {
        Arc::ptr_eq(&self.resources, resources)
            && self.root.source.as_ref() == script
            && self.root.identity.as_ref()
                == format!("{package}:{screen}:main").to_ascii_lowercase()
    }

    /// Reads only the exact prepared revision; module loading never starts compilation.
    pub(super) fn read(&self, source: &str, identity: &str) -> HostResult<String> {
        let key = ModuleKey {
            identity: identity.to_ascii_lowercase().into(),
            source: source.into(),
        };
        self.modules
            .get(&key)
            .ok_or_else(|| HostError::new(format!("Compose module was not prepared: {identity}")))?
            .clone()
            .map(|module| module.script.to_string())
    }
}

/// Owns compiler realms while sharing only immutable source results across pages.
struct ComposeCompiler {
    host: Arc<dyn HostJavaScriptRuntimeHost>,
    states: Vec<HostJavaScriptRuntimeStateHandle>,
    cache: Mutex<CompilerCache>,
}

struct CompilerCache {
    entries: VecDeque<(ModuleKey, CompiledSource, usize)>,
    pending: BTreeMap<ModuleKey, Completion>,
    loads: [usize; COMPILER_COUNT],
    bytes: usize,
}

struct CompilerState {
    runtime: Box<dyn HostJavaScriptRuntime>,
}

/// Awaits a shared compilation without owning or cancelling another page's preparation.
struct CompilationRequest {
    completion: Completion,
    #[cfg(test)]
    cache_hit: bool,
}

/// Completes shared waiters even when the Host drops an unstarted task or unwinds a compiler panic.
struct CompilationLease {
    compiler: Arc<ComposeCompiler>,
    key: ModuleKey,
    worker: usize,
    sender: Mutex<Option<watch::Sender<Option<CompiledSource>>>>,
}

impl CompilationLease {
    /// Publishes a terminal result exactly once and releases the compiler reservation.
    fn finish(&self, result: CompiledSource) {
        let sender = self
            .sender
            .lock()
            .expect("Compilation completion poisoned")
            .take();
        if let Some(sender) = sender {
            self.compiler
                .complete(self.key.clone(), self.worker, sender, result);
        }
    }
}

impl Drop for CompilationLease {
    /// Rejects abandoned tasks instead of leaving a permanent in-flight cache entry.
    fn drop(&mut self) {
        self.finish(Err(HostError::new(
            "Compose compilation task was abandoned",
        )));
    }
}

impl CompilationRequest {
    /// Waits cooperatively for exact results, including scheduling and compiler failures.
    async fn finish(mut self) -> CompiledSource {
        loop {
            if let Some(result) = self.completion.borrow_and_update().clone() {
                return result;
            }
            self.completion
                .changed()
                .await
                .map_err(|_| HostError::new("Compose compilation completion disconnected"))?;
        }
    }
}

impl CompilerCache {
    /// Returns a completed result without entering any compiler queue.
    fn get(&mut self, key: &ModuleKey) -> Option<CompiledSource> {
        let index = self.entries.iter().position(|(entry, _, _)| entry == key)?;
        let entry = self
            .entries
            .remove(index)
            .expect("Located compiler cache entry");
        let result = entry.1.clone();
        self.entries.push_back(entry);
        Some(result)
    }

    /// Publishes exact successes and failures under bounded LRU accounting.
    fn insert(&mut self, key: ModuleKey, result: CompiledSource) {
        let bytes = key.identity.len()
            + key.source.len()
            + match &result {
                Ok(module) => {
                    module.script.len()
                        + module
                            .dependencies
                            .paths
                            .iter()
                            .map(String::len)
                            .sum::<usize>()
                }
                Err(error) => error.to_string().len(),
            };
        if bytes > CACHE_BYTES {
            return;
        }
        while self.bytes + bytes > CACHE_BYTES || self.entries.len() >= CACHE_ENTRIES {
            let (_, _, bytes) = self
                .entries
                .pop_front()
                .expect("Compiler cache eviction requires an entry");
            self.bytes -= bytes;
        }
        self.bytes += bytes;
        self.entries.push_back((key, result, bytes));
    }
}

impl ComposeCompiler {
    /// Creates isolated parser realms; partial initialization releases every created Host state.
    fn new(host: Arc<dyn HostJavaScriptRuntimeHost>) -> HostResult<Arc<Self>> {
        let mut compiler = Self {
            host,
            states: Vec::with_capacity(COMPILER_COUNT),
            cache: Mutex::new(CompilerCache {
                entries: VecDeque::new(),
                pending: BTreeMap::new(),
                loads: [0; COMPILER_COUNT],
                bytes: 0,
            }),
        };
        for _ in 0..COMPILER_COUNT {
            let owner = compiler.host.clone();
            compiler.states.push(compiler.host.createHostJavaScriptRuntimeState(
                "ComposeCompiler", operit_host_api::HostJavaScriptRuntimeStateKind::Compilation,
                Box::new(move || {
                    let mut runtime = owner.createHostJavaScriptRuntime()?;
                    runtime.registerHostJavaScriptFunction("__operitReadComposeParserSource", Arc::new(|arguments| {
                        if !arguments.is_empty() { return Err(HostError::new("Parser source takes no arguments")); }
                        Ok(Value::String(readComposeDslParserSource().to_owned()))
                    }))?;
                    runtime.evaluateHostJavaScriptVoid("operit.compose.compiler.js", &format!(
                        "{}\n/** Lowers source without evaluating the module. */\nfunction __operitCompileModule(source, identity) {{ return OperitComposeCompiler.prepare(source, identity); }}",
                        buildComposeDslCompilerDefinition("__operitReadComposeParserSource"),
                    ))?;
                    Ok(Box::new(CompilerState { runtime }))
                }),
            )?);
        }
        Ok(Arc::new(compiler))
    }

    /// Coalesces identical requests and assigns distinct misses to the least loaded compiler.
    fn request(self: &Arc<Self>, key: ModuleKey) -> CompilationRequest {
        let mut cache = self.cache.lock().expect("Compose compiler cache poisoned");
        if let Some(result) = cache.get(&key) {
            return CompilationRequest {
                completion: watch::channel(Some(result)).1,
                #[cfg(test)]
                cache_hit: true,
            };
        }
        if let Some(completion) = cache.pending.get(&key) {
            return CompilationRequest {
                completion: completion.clone(),
                #[cfg(test)]
                cache_hit: true,
            };
        }
        let worker = (0..COMPILER_COUNT)
            .min_by_key(|index| cache.loads[*index])
            .expect("Compiler pool is nonempty");
        cache.loads[worker] += 1;
        let (sender, completion) = watch::channel(None);
        cache.pending.insert(key.clone(), completion.clone());
        drop(cache);
        let lease = Arc::new(CompilationLease {
            compiler: self.clone(),
            key: key.clone(),
            worker,
            sender: Mutex::new(Some(sender)),
        });
        let taskLease = lease.clone();
        let scheduled = self.host.enqueueHostJavaScriptRuntimeStateTask(
            self.states[worker],
            operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(),
            Box::new(move |state, interrupt| {
                Box::pin(async move {
                    let lease = taskLease;
                    let result = (|| {
                        let state = state
                            .downcast_mut::<CompilerState>()
                            .ok_or_else(|| HostError::new("Invalid Compose compiler state"))?;
                        let controller = state.runtime.interruptController();
                        controller.replace(vec![interrupt]);
                        let result = state
                            .runtime
                            .callHostJavaScriptFunction(
                                "__operitCompileModule",
                                &[
                                    Value::String(lease.key.source.to_string()),
                                    Value::String(lease.key.identity.to_string()),
                                ],
                            )
                            .and_then(|value| match value {
                                Value::String(script) => Ok(Arc::new(CompiledModule {
                                    script: script.into(),
                                    dependencies: moduleDependencies(&lease.key.source),
                                })),
                                _ => Err(HostError::new(
                                    "Compose compiler must return JavaScript source",
                                )),
                            });
                        controller.replace(Vec::new());
                        result
                    })();
                    lease.finish(result);
                    Ok(Box::new(()) as HostJavaScriptRuntimeStateOutput)
                })
            }),
        );
        // Retain the outer lease until admission returns so exact Host errors win over task disposal.
        if let Err(error) = scheduled {
            lease.finish(Err(error));
        }
        CompilationRequest {
            completion,
            #[cfg(test)]
            cache_hit: false,
        }
    }

    /// Releases worker accounting and publishes outside the cache lock after caller cancellation too.
    fn complete(
        &self,
        key: ModuleKey,
        worker: usize,
        sender: watch::Sender<Option<CompiledSource>>,
        result: CompiledSource,
    ) {
        let mut cache = self.cache.lock().expect("Compose compiler cache poisoned");
        cache.loads[worker] -= 1;
        cache.pending.remove(&key);
        cache.insert(key, result.clone());
        drop(cache);
        sender.send_replace(Some(result));
    }
}

impl Drop for ComposeCompiler {
    /// Releases every affine compiler state on its owning Host.
    fn drop(&mut self) {
        for state in &self.states {
            if let Err(error) = self.host.destroyHostJavaScriptRuntimeState(*state) {
                AppLogger::e(
                    "ComposeCompiler",
                    &format!("Compiler shutdown failed: {error}"),
                );
            }
        }
    }
}

/// Initializes compiler realms on Host blocking capacity, never on a page's JS worker.
async fn sharedCompiler() -> HostResult<Arc<ComposeCompiler>> {
    static COMPILER: OnceCell<HostResult<Arc<ComposeCompiler>>> = OnceCell::const_new();
    COMPILER
        .get_or_init(|| async {
            let (sender, receiver) = oneshot::channel();
            defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeTask(
                "ComposeCompilerPrepare",
                Box::new(move || {
                    let _ = sender.send(ComposeCompiler::new(defaultHostJavaScriptRuntimeHost()));
                }),
            )?;
            receiver
                .await
                .map_err(|_| HostError::new("Compose compiler initialization disconnected"))?
        })
        .await
        .clone()
}

/// Prepares the exact page snapshot asynchronously while preserving synchronous CommonJS loading.
pub(super) async fn prepareComposeSources(
    resources: Arc<BTreeMap<String, String>>,
    script: String,
    package: String,
    screen: String,
) -> HostResult<Arc<PreparedComposeSources>> {
    let compiler = sharedCompiler().await?;
    prepareSources(&compiler, resources, script, package, screen).await
}

/// Walks a page's dependency graph and admits all independent compilations before awaiting results.
async fn prepareSources(
    compiler: &Arc<ComposeCompiler>,
    resources: Arc<BTreeMap<String, String>>,
    script: String,
    package: String,
    screen: String,
) -> HostResult<Arc<PreparedComposeSources>> {
    let root = ModuleKey {
        identity: format!("{package}:{screen}:main")
            .to_ascii_lowercase()
            .into(),
        source: script.into(),
    };
    let mut requests =
        VecDeque::from([(screen.clone(), root.clone(), compiler.request(root.clone()))]);
    // Actions use an empty entry module with the same bundled runtime exports.
    let action = ModuleKey {
        identity: root.identity.clone(),
        source: "".into(),
    };
    requests.push_back((screen, action.clone(), compiler.request(action)));
    let mut visited = BTreeSet::new();
    let mut completeScope = false;
    let mut prepared = BTreeMap::new();
    while let Some((path, key, request)) = requests.pop_front() {
        let result = request.finish().await;
        let result = prepared.entry(key).or_insert(result);
        let Ok(module) = result else {
            continue;
        };
        let modules: Vec<_> = if module.dependencies.dynamic {
            if completeScope {
                continue;
            }
            completeScope = true;
            resources.iter().collect()
        } else {
            module
                .dependencies
                .paths
                .iter()
                .filter_map(|request| resolveModule(&resources, &path, request))
                .collect()
        };
        for (path, source) in modules {
            if path.ends_with(".json") || !visited.insert(path.clone()) {
                continue;
            }
            let key = ModuleKey {
                identity: format!("{package}:{path}").to_ascii_lowercase().into(),
                source: source.as_str().into(),
            };
            requests.push_back((path.clone(), key.clone(), compiler.request(key)));
        }
    }
    Ok(Arc::new(PreparedComposeSources {
        resources,
        root,
        modules: prepared,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_native_scheduler::NativeHostJavaScriptRuntimeHost;

    /// Creates a real Host compiler service with independent parser realms.
    fn compiler() -> Arc<ComposeCompiler> {
        operit_host_api::HostManager::setDefaultHostRuntimeTaskSchedulerHost(Arc::new(
            operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
        ));
        ComposeCompiler::new(Arc::new(NativeHostJavaScriptRuntimeHost::new())).unwrap()
    }

    /// Constructs an exact revision key shared by production requests and cache paths.
    fn key(identity: &str, source: &str) -> ModuleKey {
        ModuleKey {
            identity: identity.into(),
            source: source.into(),
        }
    }

    /// Bounds regressions using the same Host timer as production scheduling.
    async fn bounded<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::select! {
            result = future => result,
            _ = defaultHostRuntimeTaskSchedulerHost().waitForHostRuntimeDelay(5_000) => panic!("Independent work was blocked"),
        }
    }

    /// A held compiler cannot block another compiler, warm cache hits, or execution workers.
    #[tokio::test(flavor = "current_thread")]
    async fn blocked_compiler_leaves_other_compilers_cache_and_execution_available() {
        let compiler = compiler();
        let warm = key("pkg:warm", "exports.value='warm';");
        compiler.request(warm.clone()).finish().await.unwrap();
        let (release, waiting) = oneshot::channel::<()>();
        let (entered, started) = oneshot::channel();
        compiler
            .host
            .enqueueHostJavaScriptRuntimeStateTask(
                compiler.states[0],
                operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(),
                Box::new(move |_, _| {
                    Box::pin(async move {
                        let _ = entered.send(());
                        let _ = waiting.await;
                        Ok(Box::new(()) as HostJavaScriptRuntimeStateOutput)
                    })
                }),
            )
            .unwrap();
        bounded(started).await.unwrap();
        let blocked = compiler.request(key("pkg:cold0", "exports.value='blocked';"));
        assert_eq!(compiler.cache.lock().unwrap().loads[0], 1);
        let other = compiler.request(key("pkg:cold1", "exports.value='parallel';"));
        bounded(other.finish()).await.unwrap();
        let cached = compiler.request(warm);
        assert!(cached.cache_hit);
        bounded(cached.finish()).await.unwrap();
        let (executed, execution) = oneshot::channel();
        let state = compiler
            .host
            .createHostJavaScriptRuntimeState(
                "ComposeParallelProbe",
                operit_host_api::HostJavaScriptRuntimeStateKind::Execution,
                Box::new(|| Ok(Box::new(()))),
            )
            .unwrap();
        compiler
            .host
            .enqueueHostJavaScriptRuntimeStateTask(
                state,
                operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(),
                Box::new(move |_, _| {
                    Box::pin(async move {
                        let _ = executed.send(());
                        Ok(Box::new(()) as HostJavaScriptRuntimeStateOutput)
                    })
                }),
            )
            .unwrap();
        bounded(execution).await.unwrap();
        compiler
            .host
            .destroyHostJavaScriptRuntimeState(state)
            .unwrap();
        drop(release);
        bounded(blocked.finish()).await.unwrap();
    }

    /// Literal dependencies retain candidate ordering, case-insensitive paths, cycles and lazy errors.
    #[tokio::test(flavor = "current_thread")]
    async fn preparation_follows_dependencies_and_preserves_unused_errors() {
        let compiler = compiler();
        let resources = Arc::new(BTreeMap::from([
            (
                "dist/child.js".into(),
                "require('./nested'); exports.value=1;".into(),
            ),
            ("dist/nested/index.js".into(), "require('../child');".into()),
            ("dist/broken.js".into(), "function {".into()),
            (
                "assets/unused.html".into(),
                "<html>unused asset</html>".into(),
            ),
        ]));
        let source = "require('./CHILD'); if(false) require('./broken'); exports.default=ctx=>ctx.UI.Text({text:'ready'});";
        let prepared = prepareSources(
            &compiler,
            resources.clone(),
            source.into(),
            "pkg".into(),
            "dist/main.js".into(),
        )
        .await
        .unwrap();
        assert!(prepared.read(source, "pkg:dist/main.js:main").is_ok());
        assert!(prepared
            .read(&resources["dist/child.js"], "pkg:dist/CHILD.js")
            .is_ok());
        assert!(prepared
            .read(
                &resources["dist/nested/index.js"],
                "pkg:dist/nested/index.js"
            )
            .is_ok());
        assert!(prepared
            .read(&resources["dist/broken.js"], "pkg:dist/broken.js")
            .is_err());
        assert!(!prepared
            .modules
            .keys()
            .any(|key| key.identity.as_ref() == "pkg:assets/unused.html"));
        assert!(prepared.read("", "pkg:dist/main.js:main").is_ok());
        assert!(prepared.matches(&resources, source, "pkg", "dist/main.js"));
    }

    /// Computed and escaped require functions explicitly prepare their complete resource scope.
    #[tokio::test(flavor = "current_thread")]
    async fn dynamic_loaders_prepare_modules_without_executing_unused_code() {
        let compiler = compiler();
        let resources = Arc::new(BTreeMap::from([
            (
                "dist/custom.ext".into(),
                "throw Error('must not execute'); exports.value=1;".into(),
            ),
            ("dist/data.json".into(), "{}".into()),
        ]));
        for source in [
            "exports.load=name=>require(name);",
            "const load=require; exports.load=load;",
        ] {
            let prepared = prepareSources(
                &compiler,
                resources.clone(),
                source.into(),
                "pkg".into(),
                "dist/main.js".into(),
            )
            .await
            .unwrap();
            assert!(prepared
                .read(&resources["dist/custom.ext"], "pkg:dist/custom.ext")
                .is_ok());
            assert!(!prepared
                .modules
                .keys()
                .any(|key| key.identity.as_ref() == "pkg:dist/data.json"));
        }
    }

    /// Reuses exact revisions without evaluating plugin code or conflating module identities.
    #[tokio::test(flavor = "current_thread")]
    async fn exact_source_cache_keeps_module_identity_and_never_executes_plugin_code() {
        let compiler = compiler();
        let source = "throw Error('plugin must not run'); exports.default = ctx => ctx.UI.Text({text:'one'});";
        let first = compiler
            .request(key("pkg:screen", source))
            .finish()
            .await
            .unwrap();
        let repeated = compiler.request(key("pkg:screen", source));
        assert!(repeated.cache_hit);
        assert!(Arc::ptr_eq(&first, &repeated.finish().await.unwrap()));
        assert_ne!(
            first,
            compiler
                .request(key("pkg:screen", &source.replace("one", "two")))
                .finish()
                .await
                .unwrap()
        );
        assert_ne!(
            first,
            compiler
                .request(key("pkg:other", source))
                .finish()
                .await
                .unwrap()
        );
    }

    /// Concurrent pages share one compilation and cancellation cannot strand the remaining page.
    #[tokio::test(flavor = "current_thread")]
    async fn concurrent_pages_share_one_compilation() {
        let compiler = compiler();
        let key = key(
            "pkg:shared",
            "exports.default = ctx => ctx.UI.Text({text:'shared'});",
        );
        let first = compiler.request(key.clone());
        assert!(!first.cache_hit);
        let second = compiler.request(key.clone());
        assert!(second.cache_hit);
        drop(first);
        let result = second.finish().await.unwrap();
        assert!(Arc::ptr_eq(
            &result,
            &compiler.request(key).finish().await.unwrap()
        ));
    }

    /// Rejected Host admission completes once with the exact error and clears all reservations.
    #[tokio::test(flavor = "current_thread")]
    async fn rejected_compilation_keeps_exact_error_and_releases_waiters() {
        let compiler = compiler();
        compiler
            .host
            .destroyHostJavaScriptRuntimeState(compiler.states[0])
            .unwrap();
        let key = key("pkg:rejected", "exports.value=1;");
        let first = bounded(compiler.request(key.clone()).finish())
            .await
            .unwrap_err();
        assert!(first
            .to_string()
            .starts_with("JavaScript runtime state does not exist:"));
        let repeated = compiler.request(key);
        assert!(repeated.cache_hit);
        assert_eq!(
            first.to_string(),
            bounded(repeated.finish()).await.unwrap_err().to_string()
        );
        let cache = compiler.cache.lock().unwrap();
        assert!(cache.pending.is_empty());
        assert_eq!(cache.loads, [0; COMPILER_COUNT]);
    }

    /// Retains exact parser failures without poisoning unrelated modules.
    #[tokio::test(flavor = "current_thread")]
    async fn failures_remain_exact_and_do_not_poison_other_modules() {
        let compiler = compiler();
        for source in [
            "function {",
            "// @operit-compose-artifact 999\nexports.value=1;",
        ] {
            let first = compiler
                .request(key("pkg:broken", source))
                .finish()
                .await
                .unwrap_err();
            let repeated = compiler.request(key("pkg:broken", source));
            assert!(repeated.cache_hit);
            assert_eq!(
                first.to_string(),
                repeated.finish().await.unwrap_err().to_string()
            );
        }
        let artifact = "// @operit-compose-artifact 4\nexports.value=1;";
        assert_eq!(
            compiler
                .request(key("pkg:valid", artifact))
                .finish()
                .await
                .unwrap()
                .script
                .as_ref(),
            artifact
        );
    }

    /// Eviction never invalidates a result retained by a mounted page.
    #[tokio::test(flavor = "current_thread")]
    async fn compilation_cache_is_bounded_and_keeps_recent_entries() {
        let compiler = compiler();
        let artifact = "// @operit-compose-artifact 4\nexports.value=1;";
        let retained = compiler
            .request(key("pkg:0", artifact))
            .finish()
            .await
            .unwrap();
        for index in 1..=CACHE_ENTRIES {
            compiler
                .request(key(&format!("pkg:{index}"), artifact))
                .finish()
                .await
                .unwrap();
        }
        assert!(
            compiler
                .request(key(&format!("pkg:{CACHE_ENTRIES}"), artifact))
                .cache_hit
        );
        assert!(!compiler.request(key("pkg:0", artifact)).cache_hit);
        assert_eq!(retained.script.as_ref(), artifact);
        let cache = compiler.cache.lock().unwrap();
        assert!(cache.entries.len() <= CACHE_ENTRIES && cache.bytes <= CACHE_BYTES);
    }
}
