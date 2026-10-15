use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};

use operit_host_api::HostManager::defaultHostJavaScriptRuntimeHost;
use operit_host_api::{
    HostError, HostJavaScriptRuntime, HostJavaScriptRuntimeHost, HostJavaScriptRuntimeStateHandle,
    HostResult,
};
use operit_plugin_sdk::toolpkg::ToolPkgComposeDslBridge::{
    buildComposeDslCompilerDefinition, readComposeDslParserSource,
};
use operit_util::AppLogger::AppLogger;
use serde_json::Value;

const COMPILATION_TIMEOUT_MS: u64 = 10_000;
const CACHE_BYTES: usize = 16 * 1024 * 1024;
const CACHE_ENTRIES: usize = 128;

/// Owns the only runtime source-lowering service; page heaps never enter this isolate.
struct ComposeCompiler {
    host: Arc<dyn HostJavaScriptRuntimeHost>,
    state: HostJavaScriptRuntimeStateHandle,
}

/// Retains exact source identities, including failures, under a bounded memory budget.
struct CompiledModule {
    identity: String,
    source: String,
    result: HostResult<String>,
    bytes: usize,
}

/// Keeps compiler globals and its source cache on their host-owned executor.
struct CompilerState {
    runtime: Box<dyn HostJavaScriptRuntime>,
    entries: VecDeque<CompiledModule>,
    bytes: usize,
}

/// Returns lowered source and exposes cache reuse to regression tests.
struct Compilation {
    script: String,
    #[cfg(test)]
    cache_hit: bool,
}

impl CompilerState {
    /// Lowers source without evaluating plugin code and reuses only exact source/identity matches.
    fn prepare(&mut self, source: String, identity: String) -> HostResult<Compilation> {
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.identity == identity && entry.source == source)
        {
            let entry = self
                .entries
                .remove(index)
                .expect("Located compiler cache entry");
            let result = entry.result.clone();
            self.entries.push_back(entry);
            return result.map(|script| Compilation {
                script,
                #[cfg(test)]
                cache_hit: true,
            });
        }
        let result = self
            .runtime
            .callHostJavaScriptFunction(
                "__operitCompileModule",
                &[
                    Value::String(source.clone()),
                    Value::String(identity.clone()),
                ],
            )
            .and_then(|value| match value {
                Value::String(script) => Ok(script),
                _ => Err(HostError::new(
                    "Compose compiler must return JavaScript source",
                )),
            });
        let bytes = identity.len()
            + source.len()
            + match &result {
                Ok(script) => script.len(),
                Err(error) => error.to_string().len(),
            };
        if bytes <= CACHE_BYTES {
            while self.bytes + bytes > CACHE_BYTES || self.entries.len() >= CACHE_ENTRIES {
                let entry = self
                    .entries
                    .pop_front()
                    .expect("Compiler cache eviction requires an entry");
                self.bytes -= entry.bytes;
            }
            self.bytes += bytes;
            self.entries.push_back(CompiledModule {
                identity,
                source,
                result: result.clone(),
                bytes,
            });
        }
        result.map(|script| Compilation {
            script,
            #[cfg(test)]
            cache_hit: false,
        })
    }
}

impl ComposeCompiler {
    /// Creates a host-affine compiler with no plugin execution, storage or business bindings.
    fn new(host: Arc<dyn HostJavaScriptRuntimeHost>) -> HostResult<Self> {
        let owner = host.clone();
        let state = host.createHostJavaScriptRuntimeState("ComposeCompiler", Box::new(move || {
            let mut runtime = owner.createHostJavaScriptRuntime()?;
            runtime.registerHostJavaScriptJsonFunction("__operitReadComposeParserSource", Arc::new(|arguments| {
                if !arguments.is_empty() { return Err(HostError::new("Parser source takes no arguments")); }
                Ok(Value::String(readComposeDslParserSource().to_owned()))
            }))?;
            runtime.evaluateHostJavaScriptVoid("operit.compose.compiler.js", &format!(
                "{}\n/** Lowers source text without executing the module. */\nfunction __operitCompileModule(source, identity) {{ return OperitComposeCompiler.prepare(source, identity); }}",
                buildComposeDslCompilerDefinition("__operitReadComposeParserSource"),
            ))?;
            Ok(Box::new(CompilerState { runtime, entries: VecDeque::new(), bytes: 0 }))
        }))?;
        Ok(Self { host, state })
    }

    /// Serializes compiler misses on the host executor so concurrent pages compile a module only once.
    fn prepare(&self, source: String, identity: String) -> HostResult<Compilation> {
        let result = self.host.executeHostJavaScriptRuntimeStateTask(
            self.state,
            COMPILATION_TIMEOUT_MS,
            Box::new(move |state, interrupt| {
                let state = state
                    .downcast_mut::<CompilerState>()
                    .ok_or_else(|| HostError::new("Invalid Compose compiler state"))?;
                state
                    .runtime
                    .setHostJavaScriptInterruptHandler(Some(Arc::new(move || {
                        interrupt.shouldInterrupt()
                    })))?;
                let result = state.prepare(source, identity);
                state.runtime.setHostJavaScriptInterruptHandler(None)?;
                Ok(Box::new(result?))
            }),
        )?;
        result
            .downcast::<Compilation>()
            .map(|result| *result)
            .map_err(|_| HostError::new("Invalid Compose compiler result"))
    }
}

impl Drop for ComposeCompiler {
    /// Releases the compiler executor with its owning service.
    fn drop(&mut self) {
        if let Err(error) = self.host.destroyHostJavaScriptRuntimeState(self.state) {
            AppLogger::e(
                "ComposeCompiler",
                &format!("Compiler shutdown failed: {error}"),
            );
        }
    }
}

/// Shares immutable lowered code process-wide while keeping parser failures and page state isolated.
pub(crate) fn prepareComposeModule(source: String, identity: String) -> HostResult<String> {
    static COMPILER: OnceLock<HostResult<ComposeCompiler>> = OnceLock::new();
    let compiler = COMPILER
        .get_or_init(|| ComposeCompiler::new(defaultHostJavaScriptRuntimeHost()))
        .as_ref()
        .map_err(Clone::clone)?;
    let result = compiler.prepare(source, identity)?;
    Ok(result.script)
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_native_scheduler::NativeHostJavaScriptRuntimeHost;

    /// Creates a real host-isolated compiler rather than replacing source lowering in tests.
    fn compiler() -> ComposeCompiler {
        ComposeCompiler::new(Arc::new(NativeHostJavaScriptRuntimeHost::new())).unwrap()
    }

    /// Reuses exact sources without executing top-level plugin code or confusing equal-length revisions.
    #[test]
    fn exact_source_cache_keeps_module_identity_and_never_executes_plugin_code() {
        let compiler = compiler();
        let source = "throw Error('plugin must not run'); exports.default = ctx => ctx.UI.Text({text:'one'});";
        let first = compiler
            .prepare(source.into(), "pkg:screen".into())
            .unwrap();
        assert!(!first.cache_hit);
        assert!(first.script.contains("OperitComposeGeneration"));
        let repeated = compiler
            .prepare(source.into(), "pkg:screen".into())
            .unwrap();
        assert!(repeated.cache_hit);
        assert_eq!(first.script, repeated.script);
        let changed = compiler
            .prepare(source.replace("one", "two"), "pkg:screen".into())
            .unwrap();
        assert!(!changed.cache_hit);
        assert_ne!(first.script, changed.script);
        let other = compiler.prepare(source.into(), "pkg:other".into()).unwrap();
        assert!(!other.cache_hit);
        assert_ne!(first.script, other.script);
    }

    /// Shares one lowering result across simultaneous independent page requests.
    #[test]
    fn concurrent_pages_share_one_compilation() {
        let compiler = Arc::new(compiler());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let compiler = compiler.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    compiler
                        .prepare(
                            "exports.default = ctx => ctx.UI.Text({text:'shared'});".into(),
                            "pkg:shared".into(),
                        )
                        .unwrap()
                        .cache_hit
                })
            })
            .collect();
        let hits = threads
            .into_iter()
            .map(|thread| usize::from(thread.join().unwrap()))
            .sum::<usize>();
        assert_eq!(hits, 7);
    }

    /// Rejects malformed artifacts and source while allowing distinct valid modules afterward.
    #[test]
    fn failures_remain_exact_and_do_not_poison_other_modules() {
        let compiler = compiler();
        for source in [
            "function {",
            "// @operit-compose-artifact 999\nexports.value=1;",
        ] {
            let first = compiler
                .prepare(source.into(), "pkg:broken".into())
                .err()
                .unwrap();
            let next = compiler
                .prepare(source.into(), "pkg:broken".into())
                .err()
                .unwrap();
            assert_eq!(first.to_string(), next.to_string());
        }
        let artifact = "// @operit-compose-artifact 1\nexports.value=1;";
        assert_eq!(
            compiler
                .prepare(artifact.into(), "pkg:valid".into())
                .unwrap()
                .script,
            artifact
        );
    }

    /// Evicts least-recently used entries instead of retaining every historical script revision.
    #[test]
    fn compilation_cache_is_bounded_and_keeps_recent_entries() {
        let compiler = compiler();
        let artifact = "// @operit-compose-artifact 1\nexports.value=1;";
        for index in 0..=CACHE_ENTRIES {
            assert!(
                !compiler
                    .prepare(artifact.into(), format!("pkg:{index}"))
                    .unwrap()
                    .cache_hit
            );
        }
        assert!(
            compiler
                .prepare(artifact.into(), format!("pkg:{CACHE_ENTRIES}"))
                .unwrap()
                .cache_hit
        );
        assert!(
            !compiler
                .prepare(artifact.into(), "pkg:0".into())
                .unwrap()
                .cache_hit
        );
    }
}
