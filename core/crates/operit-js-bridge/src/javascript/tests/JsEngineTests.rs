use super::JsEngineState;
use crate::javascript::TestJsToolsHost::{expect_js_output, register_test_runtime_storage};
use operit_host_api::HostManager::{
    setDefaultHostJavaScriptRuntimeHost, setDefaultHostRuntimeTaskSchedulerHost,
};
use operit_host_api::{
    HostError, HostJavaScriptRuntimeHost, HostResult, RuntimeStorageEntry, RuntimeStorageHost,
};
use operit_host_native_scheduler::{
    NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost,
};
use operit_plugin_sdk::execution_result::JsExecutionErrorKind;
use operit_plugin_sdk::javascript::{
    JsExecutionHost, JsToolCallRequest, JsToolCallResult, JsToolCallResultData,
    JsToolNameResolutionRequest, JsToolPkgIpcCompletion, JsToolPkgIpcRequest,
    JsToolPkgResourceRequest, JsToolPkgWasmRequest, JsToolPkgWasmResult, ToolPkgExecutionContext,
    ToolPkgTextResourceHost,
};
use operit_plugin_sdk::JsPackageLoader::JsPackageLoader;
use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
use operit_util::RuntimeStorageLayout::{RUNTIME_ROOT_DIR_PATH, WORKSPACE_DIR_PATH};
use operit_util::RuntimeStoreRoot::{setDefaultRuntimeStoreRootConfig, RuntimeStoreRootConfig};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

/// Returns the native JavaScript Host shared by this test process.
#[allow(non_snake_case)]
fn testJavaScriptRuntimeHost() -> Arc<NativeHostJavaScriptRuntimeHost> {
    static HOST: OnceLock<Arc<NativeHostJavaScriptRuntimeHost>> = OnceLock::new();
    let host = HOST
        .get_or_init(|| Arc::new(NativeHostJavaScriptRuntimeHost::new()))
        .clone();
    setDefaultHostJavaScriptRuntimeHost(host.clone());
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    host
}

/// Creates one test engine after installing the concrete native Host.
#[allow(non_snake_case)]
fn newTestJsEngine(executionHost: Arc<dyn JsExecutionHost>) -> super::JsEngine {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    super::JsEngine::new(executionHost)
}

/// Creates one ToolPkg registration engine after installing the concrete native Host.
#[allow(non_snake_case)]
fn newTestToolPkgRegistrationEngine() -> super::JsEngine {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    super::JsEngine::new_toolpkg_registration_engine()
}

/// Creates one directly accessible JavaScript state through the concrete native Host.
#[allow(non_snake_case)]
pub(super) fn newTestJsEngineState(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
) -> JsEngineState {
    let runtime = testJavaScriptRuntimeHost()
        .createHostJavaScriptRuntime()
        .expect("test JavaScript runtime must start");
    JsEngineState::newWithRuntime(runtime, executionHost, None)
        .expect("test JavaScript state must initialize")
}

#[derive(Default)]
struct TestPluginConfigExecutionHost {
    toolPkgTextResourceReads: AtomicUsize,
    #[cfg(not(target_arch = "wasm32"))]
    toolPkgIpcThreadName: Arc<Mutex<Option<String>>>,
}

/// Resolves ToolPkg modules from a fixed test resource map.
struct StaticToolPkgTextResourceHost {
    resources: BTreeMap<String, String>,
}

impl ToolPkgTextResourceHost for StaticToolPkgTextResourceHost {
    /// Reads one normalized module from the fixed test resource map.
    fn read_toolpkg_text_resource(
        &self,
        _package_name_or_subpackage_id: &str,
        resource_path: &str,
    ) -> Result<String, String> {
        let normalizedPath = resource_path.trim().to_ascii_lowercase();
        self.resources
            .get(&normalizedPath)
            .cloned()
            .ok_or_else(|| format!("ToolPkg text resource not found: {normalizedPath}"))
    }
}

crate::impl_rejecting_js_tools_host!(TestPluginConfigExecutionHost);

impl JsExecutionHost for TestPluginConfigExecutionHost {
    /// Executes the System sleep call used by the JavaScript worker regression test.
    fn execute_tool_call(&self, request: JsToolCallRequest) -> JsToolCallResult {
        if request.tool_name == "get_device_location" {
            return JsToolCallResult {
                success: false,
                data: JsToolCallResultData::Value(Value::Null),
                error: Some(
                    "Error getting location information: location permission denied".to_string(),
                ),
            };
        }
        if request.tool_name != "sleep" {
            panic!(
                "Unexpected tool execution in JavaScript engine test: {}",
                request.tool_name
            );
        }
        let requestedMs = request
            .parameters
            .get("duration_ms")
            .and_then(Value::as_u64)
            .expect("System.sleep must forward duration_ms to the host");
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(Duration::from_millis(requestedMs));
        JsToolCallResult {
            success: true,
            data: JsToolCallResultData::Value(serde_json::json!({
                "requestedMs": requestedMs,
                "sleptMs": requestedMs,
            })),
            error: None,
        }
    }

    /// Returns the language used by the plugin config test.
    fn package_language(&self) -> Result<String, String> {
        Ok("zh-CN".to_string())
    }

    /// Rejects unexpected environment access.
    fn read_environment_variable(&self, _key: &str) -> Result<Option<String>, String> {
        panic!("Environment access is not part of the plugin config test")
    }

    /// Resolves plugin configuration through the real runtime path contract.
    fn plugin_config_dir(&self, plugin_id: &str) -> Result<String, String> {
        let safeBaseName = plugin_id.trim().replace(':', "_");
        Ok(format!(
            "/app/data/extensions/plugins/configs/{safeBaseName}"
        ))
    }

    /// Records direct ToolPkg text resource reads rejected by this test host.
    fn read_toolpkg_text_resource(
        &self,
        _package_name_or_subpackage_id: &str,
        _resource_path: &str,
    ) -> Result<String, String> {
        self.toolPkgTextResourceReads
            .fetch_add(1, Ordering::Relaxed);
        Err("ToolPkg text resources are not part of this test host".to_string())
    }

    /// Rejects unexpected ToolPkg resource materialization.
    fn materialize_toolpkg_resource(
        &self,
        _request: JsToolPkgResourceRequest,
    ) -> Result<String, String> {
        panic!("ToolPkg resources are not part of the plugin config test")
    }

    /// Rejects unexpected ToolPkg WASM calls.
    fn call_toolpkg_wasm(
        &self,
        _request: JsToolPkgWasmRequest,
    ) -> Result<JsToolPkgWasmResult, String> {
        panic!("ToolPkg WASM is not part of the plugin config test")
    }

    /// Rejects unexpected Compose DSL controller commands.
    fn handle_compose_webview_controller_command(
        &self,
        _payload_json: &str,
    ) -> Result<String, String> {
        panic!("Compose DSL WebView control is not part of the plugin config test")
    }

    /// Rejects unexpected Compose DSL file-picker requests.
    fn open_compose_file_picker(&self, _payload_json: &str) -> Result<String, String> {
        panic!("Compose DSL file picking is not part of the plugin config test")
    }

    /// Rejects unexpected package state access.
    fn is_package_imported(&self, _package_name: &str) -> Result<bool, String> {
        panic!("Package state is not part of the plugin config test")
    }

    /// Rejects unexpected package import.
    fn import_package(&self, _package_name: &str) -> Result<String, String> {
        panic!("Package import is not part of the plugin config test")
    }

    /// Rejects unexpected package removal.
    fn remove_package(&self, _package_name: &str) -> Result<String, String> {
        panic!("Package removal is not part of the plugin config test")
    }

    /// Rejects unexpected package activation.
    fn use_package(&self, _package_name: &str) -> Result<String, String> {
        panic!("Package activation is not part of the plugin config test")
    }

    /// Rejects unexpected package listing.
    fn list_imported_packages(&self) -> Result<Vec<String>, String> {
        panic!("Package listing is not part of the plugin config test")
    }

    /// Rejects unexpected tool name resolution.
    fn resolve_tool_name(&self, _request: JsToolNameResolutionRequest) -> Result<String, String> {
        panic!("Tool name resolution is not part of the plugin config test")
    }

    /// Handles the asynchronous ToolPkg IPC regression request.
    fn invoke_toolpkg_ipc_async(
        &self,
        request: JsToolPkgIpcRequest,
        completion: JsToolPkgIpcCompletion,
    ) -> Result<(), String> {
        let threadName = self.toolPkgIpcThreadName.clone();
        std::thread::Builder::new()
            .name("OperitToolPkgIpc".to_string())
            .spawn(move || {
                *threadName
                    .lock()
                    .expect("ToolPkg IPC thread-name mutex poisoned") =
                    std::thread::current().name().map(str::to_string);
                let valueSource = if request.channel == "operit.context.run" {
                    request.payload.get("envs")
                } else {
                    Some(&request.payload)
                };
                let result = valueSource
                    .and_then(|value| value.get("value"))
                    .and_then(Value::as_i64)
                    .map(|value| serde_json::json!({ "value": value + 1 }))
                    .unwrap_or_else(|| serde_json::json!({}));
                completion(Ok(result))
            })
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Completes one JavaScript runtime turn immediately in native unit tests.
    fn wait_for_javascript_runtime_turn(&self) -> operit_host_api::HostRuntimeTurnFuture {
        Box::pin(async { Ok(()) })
    }
}

/// Verifies failed structured tool envelopes reject with their message instead of leaking JSON.
#[test]
fn structured_tool_failure_rejects_with_message() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let output = engine
        .execute_script_function(
            r#"
                exports.read_location = async function() {
                    try {
                        await Tools.System.getLocation();
                        return "unexpected-success";
                    } catch (error) {
                        return String(error.message || error);
                    }
                };
            "#,
            "read_location",
            &testParams(),
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .expect("location tool failure must be handled by JavaScript");
    assert_eq!(
        output.as_deref(),
        Some("\"Error getting location information: location permission denied\"")
    );
    engine.destroy();
}

#[allow(non_snake_case)]
fn testParams() -> BTreeMap<String, Value> {
    let mut params = BTreeMap::new();
    params.insert(
        "__operit_package_lang".to_string(),
        Value::String("zh-CN".to_string()),
    );
    params
}

/// Verifies a synchronous loop is interrupted and does not pin the worker afterward.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn synchronous_timeout_interrupts_quickjs_worker() {
    let engine = newTestToolPkgRegistrationEngine();
    let params = testParams();
    let started = Instant::now();
    let error = engine
        .execute_script_function(
            "exports.block = function() { while (true) {} };",
            "block",
            &params,
            &BTreeMap::new(),
            None,
            true,
            1,
            None,
        )
        .expect_err("synchronous loop must time out");

    assert_eq!(error.kind, JsExecutionErrorKind::Timeout);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));

    let output = engine
        .execute_script_function(
            "exports.next = function() { return 'ready'; };",
            "next",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .expect("worker must accept execution after an interrupt");

    assert_eq!(output.as_deref(), Some("\"ready\""));
    engine.destroy();
}

/// Verifies a host System sleep call returns control to the JavaScript worker for later calls.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn system_sleep_host_call_releases_quickjs_worker() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let params = testParams();

    let sleepOutput = expect_js_output(
        engine.execute_script_function_with_timeout_millis(
            "exports.sleep = function() { return Tools.System.sleep(37); };",
            "sleep",
            &params,
            &BTreeMap::new(),
            None,
            true,
            250,
            None,
        ),
        "System.sleep host call",
    );
    let sleepPayload = serde_json::from_str::<Value>(&sleepOutput)
        .expect("System.sleep host result must serialize as JSON");
    assert_eq!(sleepPayload["requestedMs"], 37);
    assert_eq!(sleepPayload["sleptMs"], 37);

    let nextOutput = engine
        .execute_script_function(
            "exports.next = function() { return 'ready'; };",
            "next",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .expect("worker must accept execution after a System.sleep host call");

    assert_eq!(nextOutput.as_deref(), Some("\"ready\""));
    engine.destroy();
}

/// Verifies a pending tool call cannot block already-ready JavaScript promise work.
#[test]
fn async_tool_call_yields_to_ready_javascript_promise() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let params = testParams();
    let started = Instant::now();
    let output = engine
        .execute_script_function(
            r#"
                exports.race = async function() {
                    var toolResult = toolCall("sleep", { duration_ms: 150 })
                        .then(function() { return "tool"; });
                    var readyResult = Promise.resolve("ready");
                    return Promise.race([toolResult, readyResult]);
                };
            "#,
            "race",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .expect("ready JavaScript promise must win the tool-call race");

    assert_eq!(output.as_deref(), Some("\"ready\""));
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "tool execution must not block the QuickJS worker"
    );
    engine.destroy();
}

/// Verifies JavaScript timers race independently from pending Host tool work.
#[test]
fn javascript_timer_can_win_race_against_async_tool_call() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let params = testParams();
    let started = Instant::now();
    let output = engine
        .execute_script_function(
            r#"
                exports.race = async function() {
                    var toolResult = toolCall("sleep", { duration_ms: 150 })
                        .then(function() { return "tool"; });
                    var timeoutResult = new Promise(function(resolve) {
                        setTimeout(function() { resolve("timeout"); }, 20);
                    });
                    return Promise.race([toolResult, timeoutResult]);
                };
            "#,
            "race",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .expect("JavaScript timer must complete while the Host tool is pending");

    assert_eq!(output.as_deref(), Some("\"timeout\""));
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "timer completion must not wait for tool execution"
    );
    engine.destroy();
}

/// Verifies ToolPkg registration timeout interrupts synchronous code and releases the worker.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn toolpkg_registration_timeout_interrupts_quickjs_worker() {
    let engine = newTestToolPkgRegistrationEngine();
    let params = testParams();
    let error = engine
        .executeToolPkgMainRegistrationWithTimeout(
            "exports.registerToolPkg = function() { while (true) {} };",
            "registerToolPkg",
            &params,
            None,
            1,
        )
        .expect_err("synchronous ToolPkg registration must time out");

    assert_eq!(error.kind, JsExecutionErrorKind::Timeout);

    let capture = engine
        .executeToolPkgMainRegistrationWithTimeout(
            "exports.registerToolPkg = function() { return true; };",
            "registerToolPkg",
            &params,
            None,
            2,
        )
        .expect("worker must accept ToolPkg registration after an interrupt");

    assert!(capture.toolboxUiModules.is_empty());
    engine.destroy();
}

#[derive(Clone, Debug)]
struct TestRuntimeStorageHost {
    runtime_root: PathBuf,
    workspace_root: PathBuf,
}

impl TestRuntimeStorageHost {
    /// Creates a runtime storage host with explicit runtime and workspace roots.
    fn new(runtime_root: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            runtime_root,
            workspace_root,
        }
    }

    /// Resolves a virtual runtime storage path into the test runtime root.
    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        let path = Path::new(path);
        if path.is_absolute() {
            return Err(HostError::new(format!(
                "Runtime storage path must be relative: {}",
                path.display()
            )));
        }
        let mut resolved = self.runtime_root.clone();
        for component in path.components() {
            match component {
                Component::Normal(segment) => resolved.push(segment),
                Component::CurDir => {}
                _ => {
                    return Err(HostError::new(format!(
                        "Invalid runtime storage path: {}",
                        path.display()
                    )))
                }
            }
        }
        Ok(resolved)
    }
}

impl RuntimeStorageHost for TestRuntimeStorageHost {
    /// Returns the test runtime root directory.
    fn runtimeRootDir(&self) -> Option<PathBuf> {
        Some(self.runtime_root.clone())
    }

    /// Returns the test workspace root directory.
    fn workspaceRootDir(&self) -> Option<PathBuf> {
        Some(self.workspace_root.clone())
    }

    /// Reads bytes from the test runtime root.
    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(std::fs::read(self.resolve(path)?)?)
    }

    /// Writes bytes into the test runtime root.
    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Appends bytes into the test runtime root.
    fn appendBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        std::io::Write::write_all(&mut file, content)?;
        Ok(())
    }

    /// Deletes an entry from the test runtime root.
    fn delete(&self, path: &str, recursive: bool) -> HostResult<()> {
        let path = self.resolve(path)?;
        if !path.exists() {
            return Ok(());
        }
        if path.is_dir() {
            if recursive {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_dir(path)?;
            }
        } else {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Checks whether an entry exists inside the test runtime root.
    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.resolve(path)?.exists())
    }

    /// Lists entries under a prefix inside the test runtime root.
    fn list(&self, prefix: &str) -> HostResult<Vec<RuntimeStorageEntry>> {
        let directory = self.resolve(prefix)?;
        let mut entries = Vec::new();
        if !directory.exists() {
            return Ok(entries);
        }
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry
                .path()
                .strip_prefix(&self.runtime_root)
                .map_err(|error| HostError::new(error.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            entries.push(RuntimeStorageEntry {
                path,
                isDirectory: metadata.is_dir(),
                size: metadata.len() as i64,
            });
        }
        Ok(entries)
    }
}

/// Registers process-wide test runtime storage roots.
pub(super) fn ensure_test_runtime_root() {
    let root = std::env::temp_dir().join("operit-runtime-js-engine-tests");
    let runtime_root = root.join(RUNTIME_ROOT_DIR_PATH);
    let workspace_root = root.join(WORKSPACE_DIR_PATH);
    std::fs::create_dir_all(&runtime_root).expect("test runtime root");
    std::fs::create_dir_all(&workspace_root).expect("test workspace root");
    let host = Arc::new(TestRuntimeStorageHost::new(
        runtime_root.clone(),
        workspace_root.clone(),
    ));
    setDefaultRuntimeStoreRootConfig(RuntimeStoreRootConfig::new(runtime_root, workspace_root));
    setDefaultRuntimeStorageHost(host);
}

#[test]
fn execute_promise_script_repeatedly_on_same_engine() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        globalThis.__operit_cached_async_echo = globalThis.__operit_cached_async_echo || function(params) {
            return Promise.resolve("ASYNC_ECHO:" + params.text);
        };
        exports.async_echo = globalThis.__operit_cached_async_echo;
    "#;

    for index in 0..16 {
        let mut params = testParams();
        params.insert(
            "text".to_string(),
            Value::String(format!("same-engine-{index}")),
        );
        let output = state.execute_script_function_on_current_thread(
            script,
            "async_echo",
            &params,
            &BTreeMap::new(),
            None,
            true,
            60,
            None,
        );
        assert_eq!(
            expect_js_output(output, "async echo script execution"),
            format!("\"ASYNC_ECHO:same-engine-{index}\"")
        );
    }
}

#[test]
fn execute_complete_finishes_call_before_return_value() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.complete_first = function(_params) {
            complete("first");
            return "second";
        };
    "#;
    let params = testParams();

    let output = state.execute_script_function_on_current_thread(
        script,
        "complete_first",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "complete-first execution"),
        "\"first\""
    );
}

#[test]
fn execute_function_with_active_module_context() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.marker = "root-marker";
        exports.inspect_context = function(_params) {
            return String(globalThis.__operitActiveModuleExports === exports) +
                ":" +
                String(globalThis.__operitActiveModule && globalThis.__operitActiveModule.exports === exports) +
                ":" +
                globalThis.__operitActiveModuleExports.marker;
        };
    "#;
    let params = testParams();

    let output = state.execute_script_function_on_current_thread(
        script,
        "inspect_context",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "active module context execution"),
        "\"true:true:root-marker\""
    );
}

#[test]
fn bootstrap_exposes_ui_android_okhttp_api() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.inspect_bootstrap_api = function(_params) {
            return [
                typeof UINode,
                typeof Android,
                typeof PackageManager,
                typeof ContentProvider,
                typeof SystemManager,
                typeof DeviceController,
                typeof PluginConfig,
                typeof RuntimeContext,
                typeof withContext,
                typeof ToolPkg,
                typeof ToolPkg.ipc,
                typeof OkHttp,
                typeof OkHttp.newClient,
                typeof OkHttpClientBuilder,
                typeof OkHttpClient,
                typeof RequestBuilder
            ].join(":");
        };
    "#;
    let params = testParams();

    let output = state.execute_script_function_on_current_thread(
        script,
        "inspect_bootstrap_api",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "bootstrap API inspection"),
        "\"function:function:function:function:function:function:object:object:function:object:object:object:function:function:function:function\""
    );
}

#[test]
fn toolpkg_ipc_local_call_returns_handler_result() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.local_ipc = async function(_params) {
            ToolPkg.ipc.on('test.local', function(payload, meta) {
                return {
                    value: payload.value + 1,
                    channel: meta.channel,
                    runtime: meta.currentRuntime
                };
            });
            return await ToolPkg.ipc.call('test.local', { value: 41 });
        };
    "#;
    let params = testParams();

    let output = state.execute_script_function_on_current_thread(
        script,
        "local_ipc",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "ToolPkg IPC local call"),
        "{\"value\":42,\"channel\":\"test.local\",\"runtime\":\"main\"}"
    );
}

#[test]
fn runtime_context_with_context_runs_local_main_runner() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.context_runner = async function(_params) {
            function addOne(value) {
                return value + 1;
            }
            RuntimeContext.register({ addOne: addOne });
            return await withContext('main', { value: 41 }, function() {
                return { value: addOne(value) };
            });
        };
    "#;
    let params = testParams();

    let output = state.execute_script_function_on_current_thread(
        script,
        "context_runner",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "runtime context execution"),
        "{\"value\":42}"
    );
}

/// Verifies cross-runtime ToolPkg IPC leaves the source QuickJS worker and resolves by callback.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn toolpkg_ipc_cross_runtime_dispatch_is_asynchronous() {
    ensure_test_runtime_root();
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let script = r#"
        exports.remote_ipc = async function(_params) {
            return await ToolPkg.ipc.call('test.async', { value: 41 });
        };
    "#;
    let mut params = testParams();
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("test.package".to_string()),
    );
    params.insert(
        "__operit_execution_context_key".to_string(),
        Value::String("toolpkg_compose_dsl:test.package:test".to_string()),
    );
    params.insert(
        "__operit_toolpkg_runtime_kind".to_string(),
        Value::String("ui".to_string()),
    );

    let output = engine.execute_script_function(
        script,
        "remote_ipc",
        &params,
        &BTreeMap::new(),
        None,
        true,
        2,
        None,
    );

    assert_eq!(
        expect_js_output(output, "cross-runtime ToolPkg IPC"),
        "{\"value\":42}"
    );
    assert_eq!(
        host.toolPkgIpcThreadName
            .lock()
            .expect("ToolPkg IPC thread-name mutex poisoned")
            .as_deref(),
        Some("OperitToolPkgIpc")
    );
    engine.destroy();
}

/// Verifies `withContext` resolves through the asynchronous cross-runtime execution contract.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn runtime_context_cross_runtime_dispatch_is_asynchronous() {
    ensure_test_runtime_root();
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let script = r#"
        exports.remote_context = async function(_params) {
            return await withContext('main', { value: 41 }, function() {
                return { value: value + 1 };
            });
        };
    "#;
    let mut params = testParams();
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("test.package".to_string()),
    );
    params.insert(
        "__operit_execution_context_key".to_string(),
        Value::String("toolpkg_compose_dsl:test.package:test".to_string()),
    );
    params.insert(
        "__operit_toolpkg_runtime_kind".to_string(),
        Value::String("ui".to_string()),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("JavaScript async test runtime must start");
    let output = runtime.block_on(engine.execute_script_function_async(
        script.to_string(),
        "remote_context".to_string(),
        params,
        BTreeMap::new(),
        None,
        true,
        2_000,
        None,
    ));

    assert_eq!(
        expect_js_output(output, "cross-runtime withContext"),
        "{\"value\":42}"
    );
    assert_eq!(
        host.toolPkgIpcThreadName
            .lock()
            .expect("ToolPkg IPC thread-name mutex poisoned")
            .as_deref(),
        Some("OperitToolPkgIpc")
    );
    engine.destroy();
}

#[test]
fn execute_inline_hook_function_source() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.marker = "inline-root";
    "#;
    let mut params = testParams();
    params.insert(
        "__operit_inline_function_name".to_string(),
        Value::String("__operit_inline_test".to_string()),
    );
    params.insert(
        "__operit_inline_function_source".to_string(),
        Value::String(
            r#"function(_params) { return globalThis.__operitActiveModuleExports.marker; }"#
                .to_string(),
        ),
    );

    let output = state.execute_script_function_on_current_thread(
        script,
        "__operit_inline_test",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "inline hook function execution"),
        "\"inline-root\""
    );
}

#[test]
/// Verifies Compose rendering waits for the CommonJS module to initialize lexical bindings.
fn compose_dsl_default_export_can_capture_later_lexical_constants() {
    ensure_test_runtime_root();
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        exports.default = function(ctx) {
            return ctx.h('Text', {
                fontSize: FONT_TITLE,
                hintFontSize: FONT_HINT,
                reasonFontSize: FONT_REASON,
                iconSize: ICON_SIZE,
                chevronSize: CHEVRON_SIZE
            }, []);
        };
        const FONT_TITLE = 13;
        const FONT_HINT = 11;
        const FONT_REASON = 11;
        const ICON_SIZE = 22;
        const CHEVRON_SIZE = 16;
    "#;
    let mut params = testParams();
    params.insert(
        "packageName".to_string(),
        Value::String("compose_lexical_initialization_test".to_string()),
    );
    params.insert(
        "routeInstanceId".to_string(),
        Value::String("compose_lexical_initialization_route".to_string()),
    );

    let raw = expect_js_output(
        engine.execute_compose_dsl_script(
            script,
            &params,
            &BTreeMap::new(),
            Arc::new(BTreeMap::new()),
        ),
        "compose lexical initialization render result",
    );
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");

    assert_eq!(parsed["tree"]["props"]["fontSize"], 13);
    assert_eq!(parsed["tree"]["props"]["hintFontSize"], 11);
    assert_eq!(parsed["tree"]["props"]["reasonFontSize"], 11);
    assert_eq!(parsed["tree"]["props"]["iconSize"], 22);
    assert_eq!(parsed["tree"]["props"]["chevronSize"], 16);
}

/// Ensures Compose render and actions resolve package modules from the page snapshot without host reentry.
#[test]
fn compose_dsl_resource_snapshot_avoids_host_reentry_for_render_and_action() {
    ensure_test_runtime_root();
    let executionHost = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(executionHost.clone());
    let script = r#"
        const shared = require("../shared");
        exports.default = function(ctx) {
            return ctx.h('Button', {
                label: shared.label,
                onClick: function() {
                    return require("../shared").label;
                }
            }, []);
        };
    "#;
    let mut params = testParams();
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("compose_snapshot_test".to_string()),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String("dist/ui/index.ui.js".to_string()),
    );
    params.insert(
        "routeInstanceId".to_string(),
        Value::String("compose_snapshot_route".to_string()),
    );
    let textResources = Arc::new(BTreeMap::from([(
        "dist/shared.js".to_string(),
        "module.exports = { label: 'resource-snapshot' };".to_string(),
    )]));

    let raw = expect_js_output(
        engine.execute_compose_dsl_script(script, &params, &BTreeMap::new(), textResources),
        "compose resource snapshot render result",
    );
    let rendered = serde_json::from_str::<Value>(&raw).expect("compose render json");
    assert_eq!(rendered["tree"]["props"]["label"], "resource-snapshot");
    let actionId = rendered["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .expect("compose snapshot action id");

    let actionRaw = expect_js_output(
        engine.execute_compose_dsl_action(actionId, None, &params, &BTreeMap::new(), None),
        "compose resource snapshot action result",
    );
    let action = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");
    assert_eq!(action["actionResult"], "resource-snapshot");
    assert_eq!(
        executionHost
            .toolPkgTextResourceReads
            .load(Ordering::Relaxed),
        0,
        "Compose module reads must not call the manager-backed host while its mutex is held",
    );
}

#[test]
fn compose_dsl_action_uses_rendered_runtime() {
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        exports.default = function(ctx) {
            var pair = ctx.useState('count', 0);
            return ctx.h('Button', {
                label: 'count:' + pair[0],
                onClick: function() {
                    pair[1](pair[0] + 1);
                    return pair[0] + 1;
                }
            }, []);
        };
    "#;
    let mut params = testParams();
    params.insert(
        "packageName".to_string(),
        Value::String("compose_test".to_string()),
    );
    params.insert(
        "routeInstanceId".to_string(),
        Value::String("compose_route".to_string()),
    );
    let raw = expect_js_output(
        engine.execute_compose_dsl_script(
            script,
            &params,
            &BTreeMap::new(),
            Arc::new(BTreeMap::new()),
        ),
        "compose render result",
    );
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");
    let actionId = parsed["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .expect("action id");

    let actionRaw = expect_js_output(
        engine.execute_compose_dsl_action(actionId, None, &params, &BTreeMap::new(), None),
        "compose action result",
    );
    let actionParsed = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");
    assert_eq!(actionParsed["actionResult"], 1);
}

#[test]
fn compose_dsl_action_updates_runtime_options_state_store() {
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        exports.default = function(ctx) {
            var pair = ctx.useState('enabled', false);
            return ctx.h('Switch', {
                checked: pair[0],
                onCheckedChange: function(value) {
                    pair[1](value);
                }
            }, []);
        };
    "#;
    let mut params = testParams();
    params.insert(
        "packageName".to_string(),
        Value::String("compose_test".to_string()),
    );
    params.insert(
        "routeInstanceId".to_string(),
        Value::String("compose_route".to_string()),
    );
    let raw = expect_js_output(
        engine.execute_compose_dsl_script(
            script,
            &params,
            &BTreeMap::new(),
            Arc::new(BTreeMap::new()),
        ),
        "compose render result",
    );
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");
    let actionId = parsed["tree"]["props"]["onCheckedChange"]["__actionId"]
        .as_str()
        .expect("action id")
        .to_string();
    params.insert("state".to_string(), parsed["state"].clone());
    params.insert("memo".to_string(), parsed["memo"].clone());

    let actionRaw = expect_js_output(
        engine.execute_compose_dsl_action(
            &actionId,
            Some(Value::Bool(true)),
            &params,
            &BTreeMap::new(),
            None,
        ),
        "compose action result",
    );
    let actionParsed = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");

    assert_eq!(actionParsed["state"]["enabled"], true);
    assert_eq!(actionParsed["tree"]["props"]["checked"], true);
}

/// Verifies that the final DSL tree is rendered after an asynchronous toggle action settles.
#[test]
fn compose_dsl_async_toggle_action_renders_settled_state() {
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        exports.default = function(ctx) {
            var pair = ctx.useState('enabled', false);
            return ctx.h('Switch', {
                checked: pair[0],
                onCheckedChange: function(value) {
                    return Promise.resolve().then(function() {
                        pair[1](value);
                    });
                }
            }, []);
        };
    "#;
    let mut params = testParams();
    params.insert(
        "packageName".to_string(),
        Value::String("compose_async_toggle_test".to_string()),
    );
    params.insert(
        "routeInstanceId".to_string(),
        Value::String("compose_async_toggle_route".to_string()),
    );
    let raw = expect_js_output(
        engine.execute_compose_dsl_script(
            script,
            &params,
            &BTreeMap::new(),
            Arc::new(BTreeMap::new()),
        ),
        "compose async toggle render result",
    );
    let rendered = serde_json::from_str::<Value>(&raw).expect("compose async toggle render json");
    let actionId = rendered["tree"]["props"]["onCheckedChange"]["__actionId"]
        .as_str()
        .expect("async toggle action id")
        .to_string();
    params.insert("state".to_string(), rendered["state"].clone());
    params.insert("memo".to_string(), rendered["memo"].clone());

    let actionRaw = expect_js_output(
        engine.execute_compose_dsl_action(
            &actionId,
            Some(Value::Bool(true)),
            &params,
            &BTreeMap::new(),
            None,
        ),
        "compose async toggle action result",
    );
    let action =
        serde_json::from_str::<Value>(&actionRaw).expect("compose async toggle action json");

    assert_eq!(action["state"]["enabled"], true);
    assert_eq!(action["tree"]["props"]["checked"], true);
}

#[test]
fn compose_dsl_action_can_access_bootstrap_globals() {
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        exports.default = function(ctx) {
            return ctx.h('Box', {
                onLoad: function() {
                    return {
                        readResource: typeof ToolPkg.readResource,
                        icon: Icons.SportsEsports
                    };
                }
            }, []);
        };
    "#;
    let mut params = testParams();
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("compose_test".to_string()),
    );
    params.insert(
        "routeInstanceId".to_string(),
        Value::String("compose_route".to_string()),
    );
    let raw = expect_js_output(
        engine.execute_compose_dsl_script(
            script,
            &params,
            &BTreeMap::new(),
            Arc::new(BTreeMap::new()),
        ),
        "compose render result",
    );
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");
    let actionId = parsed["tree"]["props"]["onLoad"]["__actionId"]
        .as_str()
        .expect("action id");

    let actionRaw = expect_js_output(
        engine.execute_compose_dsl_action(actionId, None, &params, &BTreeMap::new(), None),
        "compose action result",
    );
    let actionParsed = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");

    assert_eq!(actionParsed["actionResult"]["readResource"], "function");
    assert_eq!(actionParsed["actionResult"]["icon"], "SportsEsports");
}

#[test]
fn execute_function_from_module_exports() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        module.exports = {
            module_only: function(params) {
                return "module:" + params.text;
            }
        };
    "#;
    let mut params = testParams();
    params.insert("text".to_string(), Value::String("exports".to_string()));

    let output = state.execute_script_function_on_current_thread(
        script,
        "module_only",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "module exports execution"),
        "\"module:exports\""
    );
}

/// Verifies a package script keeps metadata readable while its executable body is minified.
#[test]
fn execute_minified_package_script_with_metadata() {
    ensure_test_runtime_root();
    let script = r#"/* METADATA
{
  name: minified_package
  displayName: Minified Package
  tools: [
    {
      name: echo
      description: Echo text
      parameters: [
        { name: text, description: Text to echo, type: string, required: true }
      ]
    }
  ]
}
*/"use strict";Object.defineProperty(exports,"__esModule",{value:!0});exports.echo=function(t){if(typeof Tools!="object")throw new Error("Tools global missing");return"echo:"+t.text};"#;
    let package = JsPackageLoader::parse(script).expect("minified package metadata should parse");
    assert_eq!(package.name, "minified_package");
    assert_eq!(package.tools.len(), 1);
    assert_eq!(package.tools[0].name, "echo");

    let mut state = newTestJsEngineState(None);
    let mut params = testParams();
    params.insert("text".to_string(), Value::String("metadata".to_string()));

    let output = state.execute_script_function_on_current_thread(
        &package.tools[0].script,
        &package.tools[0].name,
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "minified package script execution"),
        "\"echo:metadata\""
    );
}

#[test]
fn register_thinking_guidance_toolpkg_main() {
    ensure_test_runtime_root();
    let engine = newTestToolPkgRegistrationEngine();
    let repoRoot = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root");
    let scriptPath = repoRoot.join("plugins/packages/buildin/thinking_guidance/dist/main.js");
    let script = std::fs::read_to_string(&scriptPath).expect("thinking_guidance main.js");
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("thinking_guidance".to_string()),
    );

    let capture = engine
        .execute_toolpkg_main_registration_function(&script, "registerToolPkg", &params)
        .expect("thinking_guidance registration");

    assert_eq!(capture.inputMenuTogglePlugins.len(), 1);
    assert_eq!(capture.systemPromptComposeHooks.len(), 1);
    let menu = serde_json::from_str::<Value>(&capture.inputMenuTogglePlugins[0]).unwrap();
    assert_eq!(menu["function"], "onInputMenuToggle");
    let prompt = serde_json::from_str::<Value>(&capture.systemPromptComposeHooks[0]).unwrap();
    assert_eq!(prompt["function"], "onSystemPromptCompose");
}

#[test]
fn register_message_insert_toolpkg_main() {
    ensure_test_runtime_root();
    let engine = newTestToolPkgRegistrationEngine();
    let repoRoot = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root");
    let scriptPath = repoRoot.join("plugins/packages/external/message_insert/dist/main.js");
    let script = std::fs::read_to_string(&scriptPath).expect("message_insert main.js");
    let distRoot = repoRoot.join("plugins/packages/external/message_insert/dist");
    let mut textResources = BTreeMap::new();
    collect_message_insert_text_resources(&distRoot, &distRoot, &mut textResources);
    assert!(textResources.contains_key("dist/ui/index.ui.js"));
    assert!(textResources.contains_key("dist/shared.js"));
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String("dist/main.js".to_string()),
    );

    let capture = engine
        .execute_toolpkg_main_registration_function_with_text_resources(
            &script,
            "registerToolPkg",
            &params,
            Some(Arc::new(textResources)),
        )
        .expect("message_insert registration");

    assert_eq!(capture.toolboxUiModules.len(), 1);
    assert_eq!(capture.promptInputHooks.len(), 1);
    assert_eq!(capture.promptFinalizeHooks.len(), 1);
    assert_eq!(capture.inputMenuTogglePlugins.len(), 1);
    let inputMenuHook = serde_json::from_str::<Value>(&capture.inputMenuTogglePlugins[0])
        .expect("message_insert input-menu hook registration");
    assert_eq!(inputMenuHook["function"], "onInputMenuToggle");
    assert!(
        inputMenuHook.get("function_source").is_none(),
        "main-module hook must be registered by export name rather than a generated module reference"
    );
}

/// Verifies that the message-insert shared module can be parsed and required.
#[test]
fn execute_message_insert_shared_module_loads() {
    ensure_test_runtime_root();
    let repoRoot = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root");
    let distRoot = repoRoot.join("plugins/packages/external/message_insert/dist");
    let script = r#"
        exports.load_shared = function() {
            var shared = require('./shared');
            return shared.createDefaultSettings();
        };
    "#;
    let mut textResources = BTreeMap::new();
    collect_message_insert_text_resources(&distRoot, &distRoot, &mut textResources);
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String("dist/main.js".to_string()),
    );
    let mut state = newTestJsEngineState(None);
    let output = super::executeWithToolPkgTextResources(Arc::new(textResources), || {
        state.execute_script_function_on_current_thread(
            &script,
            "load_shared",
            &params,
            &BTreeMap::new(),
            None,
            true,
            60,
            None,
        )
    });
    let output = output.expect("message_insert shared module should execute");
    assert!(output.is_some());
}

/// Verifies the message-insert input-menu hook loads the main entry and its UI dependency chain.
#[test]
fn execute_message_insert_input_menu_hook_from_main_entry() {
    ensure_test_runtime_root();
    let repoRoot = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root");
    let distRoot = repoRoot.join("plugins/packages/external/message_insert/dist");
    let script = format!(
        "{}\nTools.Files.exists = function(path) {{ return Promise.resolve({{ path: path, exists: false }}); }};",
        std::fs::read_to_string(distRoot.join("main.js")).expect("message_insert main.js")
    );
    let mut textResources = BTreeMap::new();
    collect_message_insert_text_resources(&distRoot, &distRoot, &mut textResources);
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String("dist/main.js".to_string()),
    );
    params.insert(
        "eventPayload".to_string(),
        serde_json::json!({ "action": "create" }),
    );
    let mut state = newTestJsEngineState(Some(Arc::new(TestPluginConfigExecutionHost::default())));
    let output = super::executeWithToolPkgTextResources(Arc::new(textResources), || {
        state.execute_script_function_on_current_thread(
            &script,
            "onInputMenuToggle",
            &params,
            &BTreeMap::new(),
            None,
            true,
            60,
            None,
        )
    });
    let raw = expect_js_output(output, "message_insert input menu hook");
    let definitions =
        serde_json::from_str::<Value>(&raw).expect("message_insert input menu definitions JSON");
    assert_eq!(definitions[0]["id"], "message_extra_info_injection");
}

/// Verifies an IPC-style main runtime request resolves modules from its bound ToolPkg host.
#[test]
fn toolpkg_ipc_main_request_uses_bound_resource_host() {
    ensure_test_runtime_root();
    let executionHost = Arc::new(TestPluginConfigExecutionHost::default());
    let script = r#"
        var shared = require('./shared');
        exports.dispatch = function() {
            return shared.value;
        };
    "#;
    let mut params = testParams();
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String("dist/main.js".to_string()),
    );
    testJavaScriptRuntimeHost();
    let engine = super::JsEngine::new_toolpkg_execution_engine(
        executionHost.clone(),
        ToolPkgExecutionContext {
            context_key: "toolpkg_main:message_insert".to_string(),
            container_package_name: "message_insert".to_string(),
            text_resource_host: Arc::new(StaticToolPkgTextResourceHost {
                resources: BTreeMap::from([(
                    "dist/shared.js".to_string(),
                    "exports.value = 'host-module';".to_string(),
                )]),
            }),
        },
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("JavaScript async test runtime must start");
    let output = runtime.block_on(engine.execute_script_function_async(
        script.to_string(),
        "dispatch".to_string(),
        params,
        BTreeMap::new(),
        None,
        true,
        60_000,
        None,
    ));
    assert_eq!(
        expect_js_output(output, "ToolPkg IPC main request"),
        "\"host-module\""
    );
    assert_eq!(
        executionHost
            .toolPkgTextResourceReads
            .load(Ordering::Relaxed),
        0,
        "IPC package module reads must use the context-bound resource host",
    );
    engine.destroy();
}

/// Verifies the real extra-info Compose screen resolves its parent shared module.
#[test]
fn render_message_insert_compose_dsl_screen() {
    ensure_test_runtime_root();
    let repoRoot = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root");
    let distRoot = repoRoot.join("plugins/packages/external/message_insert/dist");
    let script = std::fs::read_to_string(distRoot.join("ui/index.ui.js"))
        .expect("message_insert compose screen");
    let mut textResources = BTreeMap::new();
    collect_message_insert_text_resources(&distRoot, &distRoot, &mut textResources);
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String("dist/ui/index.ui.js".to_string()),
    );
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let output = engine.execute_compose_dsl_script(
        &script,
        &params,
        &BTreeMap::new(),
        Arc::new(textResources),
    );
    let raw = expect_js_output(output, "message_insert compose render");
    let rendered = serde_json::from_str::<Value>(&raw).expect("message_insert compose render JSON");
    assert!(rendered["tree"].is_object());
    engine.destroy();
}

/// Verifies the real message-insert master switch renders the immediate local state change.
#[test]
fn message_insert_compose_master_switch_updates_before_async_persistence() {
    ensure_test_runtime_root();
    let repoRoot = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root");
    let distRoot = repoRoot.join("plugins/packages/external/message_insert/dist");
    let script = std::fs::read_to_string(distRoot.join("ui/index.ui.js"))
        .expect("message_insert compose screen");
    let mut textResources = BTreeMap::new();
    collect_message_insert_text_resources(&distRoot, &distRoot, &mut textResources);
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_ui_package_name".to_string(),
        Value::String("message_insert".to_string()),
    );
    params.insert(
        "__operit_script_screen".to_string(),
        Value::String("dist/ui/index.ui.js".to_string()),
    );
    params.insert(
        "__operit_toolpkg_runtime_kind".to_string(),
        Value::String("ui".to_string()),
    );
    params.insert(
        "__operit_execution_context_key".to_string(),
        Value::String("toolpkg_compose_dsl:message_insert:message_insert_settings".to_string()),
    );
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let resources = Arc::new(textResources);
    let renderedRaw = expect_js_output(
        engine.execute_compose_dsl_script(&script, &params, &BTreeMap::new(), resources),
        "message_insert compose render",
    );
    let rendered = serde_json::from_str::<Value>(&renderedRaw).expect("rendered JSON");
    let actionId = find_switch_action_for_text(&rendered["tree"], "额外信息注入")
        .expect("master switch action id");
    let mut actionParams = params.clone();
    actionParams.insert("state".to_string(), rendered["state"].clone());
    actionParams.insert("memo".to_string(), rendered["memo"].clone());
    let actionRaw = expect_js_output(
        engine.execute_compose_dsl_action(
            &actionId,
            Some(Value::Bool(true)),
            &actionParams,
            &BTreeMap::new(),
            None,
        ),
        "message_insert master toggle action",
    );
    let action = serde_json::from_str::<Value>(&actionRaw).expect("action JSON");
    assert_eq!(action["state"]["masterEnabled"], true);
    assert_eq!(find_switch_checked_for_text(&action["tree"]), Some(true));
    engine.destroy();
}

/// Finds one action id belonging to the toggle row with the requested title.
fn find_switch_action_for_text(node: &Value, title: &str) -> Option<String> {
    if contains_text(node, title) {
        if let Some(actionId) = find_switch_action(node) {
            return Some(actionId);
        }
    }
    for key in ["children", "slots"] {
        if let Some(value) = node.get(key) {
            if let Some(found) = find_switch_action_in_value(value, title) {
                return Some(found);
            }
        }
    }
    None
}

/// Recursively finds the switch action in a Compose subtree containing a title.
fn find_switch_action_in_value(value: &Value, title: &str) -> Option<String> {
    if let Some(items) = value.as_array() {
        for item in items {
            if let Some(found) = find_switch_action_for_text(item, title) {
                return Some(found);
            }
        }
    } else if let Some(object) = value.as_object() {
        for child in object.values() {
            if let Some(found) = find_switch_action_in_value(child, title) {
                return Some(found);
            }
        }
    }
    None
}

/// Finds the checked state of the first switch in a Compose subtree.
fn find_switch_checked_for_text(node: &Value) -> Option<bool> {
    if node.get("type").and_then(Value::as_str) == Some("Switch") {
        return node["props"]["checked"].as_bool();
    }
    for key in ["children", "slots"] {
        if let Some(value) = node.get(key) {
            if let Some(found) = find_switch_checked_in_value(value) {
                return Some(found);
            }
        }
    }
    None
}

/// Returns whether a Compose subtree contains the requested text node.
fn contains_text(node: &Value, title: &str) -> bool {
    if node["type"] == "Text" && node["props"]["text"] == title {
        return true;
    }
    ["children", "slots"].iter().any(|key| {
        node.get(*key)
            .map(|value| contains_text_in_value(value, title))
            .unwrap_or(false)
    })
}

/// Recursively checks text nodes in a Compose tree value.
fn contains_text_in_value(value: &Value, title: &str) -> bool {
    if let Some(items) = value.as_array() {
        return items.iter().any(|item| contains_text(item, title));
    }
    value
        .as_object()
        .map(|object| {
            object
                .values()
                .any(|child| contains_text_in_value(child, title))
        })
        .unwrap_or(false)
}

/// Finds the first switch action id in a Compose subtree.
fn find_switch_action(node: &Value) -> Option<String> {
    if node.get("type").and_then(Value::as_str) == Some("Switch") {
        return node["props"]["onCheckedChange"]["__actionId"]
            .as_str()
            .map(ToString::to_string);
    }
    ["children", "slots"].iter().find_map(|key| {
        node.get(*key)
            .and_then(|value| find_switch_action_in_value_any(value))
    })
}

/// Recursively finds a switch action in a Compose tree value.
fn find_switch_action_in_value_any(value: &Value) -> Option<String> {
    if let Some(items) = value.as_array() {
        return items.iter().find_map(find_switch_action);
    }
    value
        .as_object()
        .and_then(|object| object.values().find_map(find_switch_action_in_value_any))
}

/// Recursively finds one switch checked property in a Compose tree value.
fn find_switch_checked_in_value(value: &Value) -> Option<bool> {
    if let Some(items) = value.as_array() {
        for item in items {
            if let Some(found) = find_switch_checked_for_text(item) {
                return Some(found);
            }
        }
    } else if let Some(object) = value.as_object() {
        for child in object.values() {
            if let Some(found) = find_switch_checked_in_value(child) {
                return Some(found);
            }
        }
    }
    None
}

/// Collects all message-insert JavaScript resources using package-relative paths.
fn collect_message_insert_text_resources(
    root: &Path,
    directory: &Path,
    textResources: &mut BTreeMap<String, String>,
) {
    for entry in std::fs::read_dir(directory).expect("message_insert dist") {
        let entry = entry.expect("message_insert dist entry");
        let path = entry.path();
        if path.is_dir() {
            collect_message_insert_text_resources(root, &path, textResources);
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("message_insert relative resource")
            .to_string_lossy()
            .replace('\\', "/");
        if let Ok(text) = std::fs::read_to_string(&path) {
            textResources.insert(format!("dist/{relative}").to_ascii_lowercase(), text);
        }
    }
}

#[test]
fn execute_script_can_require_axios_and_uuid() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.inspect_require = function(_params) {
            var axios = require('axios');
            var uuid = require('uuid');
            return typeof axios.get + ":" + typeof axios.post + ":" + uuid.v4().length;
        };
    "#;
    let params = testParams();

    let output = state.execute_script_function_on_current_thread(
        script,
        "inspect_require",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "require API inspection"),
        "\"function:function:36\""
    );
}

#[test]
fn registration_mode_uses_ui_module_placeholder() {
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        var Screen = require('./screens/main.ui.js');
        exports.registerToolPkg = function(_params) {
            ToolPkg.registerUiRoute({
                id: "main",
                path: "/main",
                screen: Screen
            });
            return true;
        };
    "#;
    let mut params = testParams();
    params.insert("toolPkgId".to_string(), Value::String("ui_pkg".to_string()));

    let capture = engine
        .execute_toolpkg_main_registration_function(script, "registerToolPkg", &params)
        .expect("ui registration");

    assert_eq!(capture.uiRoutes.len(), 1);
    let route = serde_json::from_str::<Value>(&capture.uiRoutes[0]).unwrap();
    assert_eq!(route["screen"], "screens/main.ui.js");
}

/// Verifies registration blocks resource and WASM execution before native host access.
#[test]
fn registration_mode_blocks_resource_and_wasm_calls() {
    ensure_test_runtime_root();
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        exports.registerToolPkg = function() {
            var resourceError = '';
            var wasmError = '';
            try {
                ToolPkg.readResource('blocked-resource');
            } catch (error) {
                resourceError = error.message;
            }
            try {
                ToolPkg.wasm.call('blocked-module', 'blocked-export', []);
            } catch (error) {
                wasmError = error.message;
            }
            ToolPkg.registerNavigationEntry({
                id: 'registration-capability-check',
                resourceError: resourceError,
                wasmError: wasmError
            });
        };
    "#;

    let capture = engine
        .execute_toolpkg_main_registration_function(script, "registerToolPkg", &testParams())
        .expect("registration must reject forbidden runtime capabilities");

    assert_eq!(capture.navigationEntries.len(), 1);
    let entry = serde_json::from_str::<Value>(&capture.navigationEntries[0])
        .expect("registration capability check entry");
    assert_eq!(
        entry["resourceError"],
        "ToolPkg.readResource is unavailable during ToolPkg registration"
    );
    assert_eq!(
        entry["wasmError"],
        "ToolPkg.wasm.call is unavailable during ToolPkg registration"
    );
}

/// Verifies call-scoped environment overrides are visible through `getEnv`.
#[test]
fn native_interface_reads_env_override_for_call() {
    ensure_test_runtime_root();
    let key = "OPERIT_JS_NATIVE_ENV_TEST";
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.read_env = function(_params) {
            return getEnv("OPERIT_JS_NATIVE_ENV_TEST");
        };
    "#;
    let params = testParams();
    let envOverrides = BTreeMap::from([(key.to_string(), "enabled".to_string())]);

    let output = state.execute_script_function_on_current_thread(
        script,
        "read_env",
        &params,
        &envOverrides,
        None,
        true,
        60,
        None,
    );

    assert_eq!(
        expect_js_output(output, "environment override read"),
        "\"enabled\""
    );
}

/// Verifies plugin configuration directories use the runtime storage layout.
#[test]
fn native_interface_resolves_plugin_config_dir() {
    ensure_test_runtime_root();
    let mut state = newTestJsEngineState(Some(Arc::new(TestPluginConfigExecutionHost::default())));
    let script = r#"
        exports.config_dir = function(_params) {
            return getPluginConfigDir('plugin:name');
        };
    "#;
    let params = testParams();

    let output = state.execute_script_function_on_current_thread(
        script,
        "config_dir",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );
    let output = expect_js_output(output, "config dir execution");
    let path = serde_json::from_str::<String>(&output).expect("serialized config dir");

    assert_eq!(path, "/app/data/extensions/plugins/configs/plugin_name");
}

#[test]
fn probe_async_function_declaration_inside_iife() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        const SystemTools = (function () {
            async function get_device_info(_params) {
                const result = Tools.System.getDeviceInfo();
                return { success: true, data: result };
            }
            async function wrapToolExecution(func, params) {
                const result = await func(params);
                complete(result);
            }
            return {
                get_device_info: (params) => wrapToolExecution(get_device_info, params),
            };
        })();
        exports.get_device_info = SystemTools.get_device_info;
    "#;
    let params = testParams();

    let output = state.execute_script_function_on_current_thread(
        script,
        "get_device_info",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert!(output
        .expect("async function declaration probe execution")
        .is_some());
}
