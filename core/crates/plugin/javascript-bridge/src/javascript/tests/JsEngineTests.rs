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
    JsToolPkgResourceRequest, JsToolPkgWasmRequest, JsToolPkgWasmResult, ToolPkgConfigScope,
    ToolPkgExecutionContext, ToolPkgTextResourceHost,
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

/// Creates a package-bound UI engine for asynchronous cross-runtime IPC tests.
#[allow(non_snake_case)]
fn newTestIpcEngine(executionHost: Arc<dyn JsExecutionHost>) -> super::JsEngine {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    super::JsEngine::new_toolpkg_execution_engine(
        executionHost,
        ToolPkgExecutionContext {
            context_key: "toolpkg_compose_dsl:test.package:test".to_string(),
            container_package_name: "test.package".to_string(),
            api_version: "2.0.0".to_string(),
            text_resource_host: Arc::new(StaticToolPkgTextResourceHost {
                resources: BTreeMap::new(),
            }),
        },
    )
}

/// Returns the repository root above core/crates/plugin/javascript-bridge.
#[allow(non_snake_case)]
fn testRepositoryRoot() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("repository root")
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
    JsEngineState::newWithRuntime(runtime, executionHost, None, Arc::new(Mutex::new(None)))
        .expect("test JavaScript state must initialize")
}

#[derive(Default)]
struct TestPluginConfigExecutionHost {
    gatedToolCalls: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<JsToolCallResult>>>>,
    edgePortCalls: Arc<Mutex<Vec<Value>>>,
    gatedToolStarted: Arc<tokio::sync::Notify>,
    toolPkgTextResourceReads: AtomicUsize,
    registrationConfigReads: AtomicUsize,
    packageManagerLock: Mutex<()>,
    environment: Mutex<BTreeMap<String, String>>,
    #[cfg(not(target_arch = "wasm32"))]
    toolPkgIpcThreadName: Arc<Mutex<Option<String>>>,
    #[cfg(not(target_arch = "wasm32"))]
    toolPkgIpcTarget: Mutex<Option<(super::JsEngine, String)>>,
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
    ) -> Result<Option<String>, String> {
        let normalizedPath = resource_path.trim().to_ascii_lowercase();
        Ok(self.resources.get(&normalizedPath).cloned())
    }
}

crate::impl_rejecting_js_tools_host!(TestPluginConfigExecutionHost);

impl JsExecutionHost for TestPluginConfigExecutionHost {
    /// Returns an empty catalog for tests that do not install runtime tools.
    fn get_tool_catalog(&self) -> Result<Value, String> {
        Ok(serde_json::json!({ "tools": [] }))
    }

    /// Executes the System sleep call used by the JavaScript worker regression test.
    fn execute_tool_call(
        &self,
        request: JsToolCallRequest,
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<JsToolCallResult> {
        let gatedToolCalls = self.gatedToolCalls.clone();
        let gatedToolStarted = self.gatedToolStarted.clone();
        let edgePortCalls = self.edgePortCalls.clone();
        Box::pin(async move {
            if matches!(request.tool_name.as_str(),"edge_execute"|"io_execute") {
                edgePortCalls.lock().unwrap().push(serde_json::json!({"tool":request.tool_name,"params":request.parameters}));
                let node = request.parameters["node_id"].as_str().unwrap_or("");
                return if node == "unauthorized" {
                    JsToolCallResult {success:false,data:JsToolCallResultData::Value(Value::Null),error:Some("Edge node access denied".into())}
                } else {
                    JsToolCallResult {success:true,data:JsToolCallResultData::Value(serde_json::json!({"nodeId":node,"data":{"ok":true}})),error:None}
                };
            }

            if request.tool_name == "echo" {
                return JsToolCallResult {
                    success: true,
                    data: JsToolCallResultData::Value(
                        serde_json::to_value(request.parameters).unwrap(),
                    ),
                    error: None,
                };
            }
            if request.tool_name == "binary" {
                return JsToolCallResult {
                    success: true,
                    data: JsToolCallResultData::Binary(vec![1, 2, 3]),
                    error: None,
                };
            }
            if request.tool_name == "gate" {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                gatedToolCalls.lock().unwrap().push(sender);
                gatedToolStarted.notify_one();
                return receiver
                    .await
                    .expect("test must finish the gated Host call");
            }
            if request.tool_name == "get_device_location" {
                return JsToolCallResult {
                    success: false,
                    data: JsToolCallResultData::Value(Value::Null),
                    error: Some(
                        "Error getting location information: location permission denied"
                            .to_string(),
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
            let (sender, receiver) = tokio::sync::oneshot::channel();
            operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost()
                .scheduleDelayedHostRuntimeTask(
                    "test-tool-delay",
                    requestedMs,
                    Box::new(move || {
                        let _ = sender.send(());
                    }),
                )
                .expect("Host must schedule the requested test delay");
            receiver
                .await
                .expect("Host must complete the requested test delay");
            JsToolCallResult {
                success: true,
                data: JsToolCallResultData::Value(serde_json::json!({
                    "requestedMs": requestedMs,
                    "sleptMs": requestedMs,
                })),
                error: None,
            }
        })
    }

    /// Returns the language used by the plugin config test.
    fn package_language(&self) -> Result<String, String> {
        Ok("zh-CN".to_string())
    }

    /// Keeps environment state isolated from the application's actual preferences.
    fn read_environment_variable(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.environment.lock().unwrap().get(key).cloned())
    }

    /// Supports call-context tests without writing application configuration.
    fn write_environment_variable(&self, key: &str, value: &str) -> Result<(), String> {
        self.environment
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    /// Resolves plugin configuration through the real runtime path contract.
    fn plugin_config_dir(&self, plugin_id: &str) -> Result<String, String> {
        let safeBaseName = plugin_id.trim().replace(':', "_");
        Ok(format!(
            "/app/data/extensions/device/plugins/configs/{safeBaseName}"
        ))
    }

    /// Resolves scoped configuration through the explicit test contract.
    fn scoped_plugin_config_dir(&self, _owner_id: &str, plugin_id: &str) -> Result<String, String> {
        let _manager = self
            .packageManagerLock
            .lock()
            .expect("test package manager lock");
        self.plugin_config_dir(plugin_id)
    }

    /// Creates paths from registration scope without accessing the installed package manager.
    fn registration_plugin_config_dir(
        &self,
        owner_id: &str,
        plugin_id: &str,
        scope: ToolPkgConfigScope,
    ) -> Result<String, String> {
        self.registrationConfigReads.fetch_add(1, Ordering::Relaxed);
        let root = operit_store::ExtensionStore::ExtensionStore::configPathForScope(
            owner_id,
            scope.as_str(),
        )?;
        let path = if plugin_id == owner_id {
            root
        } else {
            let name = operit_util::OperitPaths::pluginConfigDirName(plugin_id)?;
            format!("{root}/namespaces/{name}")
        };
        Ok(format!(
            "/app/data/{}",
            path.strip_prefix("runtime/")
                .ok_or("invalid test config path")?
        ))
    }

    /// Records direct ToolPkg text resource reads rejected by this test host.
    fn read_toolpkg_text_resource(
        &self,
        _package_name_or_subpackage_id: &str,
        _resource_path: &str,
    ) -> Result<Option<String>, String> {
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
        _payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        panic!("Compose DSL WebView control is not part of the plugin config test")
    }

    /// Rejects unexpected Compose DSL file-picker requests.
    fn open_compose_file_picker(
        &self,
        _payload: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
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
        let target = self
            .toolPkgIpcTarget
            .lock()
            .expect("test IPC target lock")
            .clone();
        std::thread::Builder::new()
            .name("OperitToolPkgIpc".to_string())
            .spawn(move || {
                *threadName
                    .lock()
                    .expect("ToolPkg IPC thread-name mutex poisoned") =
                    std::thread::current().name().map(str::to_string);
                if let Some((engine, mainScript)) = target {
                    let mut params = testParams();
                    for (name, value) in [
                        ("__operit_ui_package_name", request.package_target.clone()),
                        ("__operit_execution_context_key", format!("toolpkg_main:{}", request.package_target)),
                        ("__operit_toolpkg_runtime_kind", "main".to_string()),
                        ("__operit_script_screen", "dist/main.js".to_string()),
                        ("__operit_inline_function_name", "__test_ipc_dispatch__".to_string()),
                        ("__operit_inline_function_source", r#"async function(params) {
                            return await globalThis.__operitInvokeToolPkgIpcLocal(params.channel, params.payload, {
                                callerContextKey: params.callerContextKey,
                                currentContextKey: params.__operit_execution_context_key,
                                currentRuntime: 'main'
                            });
                        }"#.to_string()),
                    ] { params.insert(name.to_string(), Value::String(value)); }
                    params.insert("channel".to_string(), Value::String(request.channel));
                    params.insert("payload".to_string(), request.payload);
                    params.insert("callerContextKey".to_string(), Value::String(request.caller_context_key));
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()
                        .expect("test target runtime must start");
                    let output = runtime.block_on(engine.execute_script_function_async(
                        mainScript, "__test_ipc_dispatch__".to_string(), params,
                        BTreeMap::new(), None, true, 2_000, None,
                    ));
                    let result = output.map_err(|error| error.to_string()).and_then(|output| {
                        operit_plugin_sdk::execution_result::decode_js_execution_result_value(output.as_deref())
                            .map_err(|error| error.to_string())
                    });
                    completion(result);
                    return;
                }
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
#[tokio::test(flavor = "current_thread")]
async fn structured_tool_failure_rejects_with_message() {
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
        .await
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
#[tokio::test(flavor = "current_thread")]
async fn synchronous_timeout_interrupts_quickjs_worker() {
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
        .await
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
        .await
        .expect("worker must accept execution after an interrupt");

    assert_eq!(output.as_deref(), Some("\"ready\""));
    engine.destroy();
}

/// Verifies a host System sleep call returns control to the JavaScript worker for later calls.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn system_sleep_host_call_releases_quickjs_worker() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let params = testParams();

    let sleepOutput = expect_js_output(
        engine
            .execute_script_function_with_timeout_millis(
                "exports.sleep = function() { return Tools.System.sleep(37); };",
                "sleep",
                &params,
                &BTreeMap::new(),
                None,
                true,
                250,
                None,
            )
            .await,
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
        .await
        .expect("worker must accept execution after a System.sleep host call");

    assert_eq!(nextOutput.as_deref(), Some("\"ready\""));
    engine.destroy();
}

/// Verifies a pending tool call cannot block already-ready JavaScript promise work.
#[tokio::test(flavor = "current_thread")]
async fn async_tool_call_yields_to_ready_javascript_promise() {
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
        .await
        .expect("ready JavaScript promise must win the tool-call race");

    assert_eq!(output.as_deref(), Some("\"ready\""));
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "tool execution must not block the QuickJS worker"
    );
    engine.destroy();
}

/// Verifies the host tool catalog bridge returns structured schemas to package JavaScript.
#[tokio::test(flavor = "current_thread")]
async fn tool_catalog_bridge_returns_structured_response() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let output = engine
        .execute_script_function(
            "exports.catalog = function() { return getToolCatalog(); };",
            "catalog",
            &testParams(),
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .await
        .expect("tool catalog bridge must complete");

    assert_eq!(output.as_deref(), Some(r#"{"tools":[]}"#));
    engine.destroy();
}

/// Verifies JavaScript timers race independently from pending Host tool work.
#[tokio::test(flavor = "current_thread")]
async fn javascript_timer_can_win_race_against_async_tool_call() {
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
        .await
        .expect("JavaScript timer must complete while the Host tool is pending");

    assert_eq!(output.as_deref(), Some("\"timeout\""));
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "timer completion must not wait for tool execution"
    );
    engine.destroy();
}

/// Verifies detached-call lifecycle queries return structured booleans without JSON text.
#[test]
fn detached_call_preparation_accepts_boolean_result() {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    let mut state = newTestJsEngineState(None);
    state
        .initJavaScriptEnvironment()
        .expect("JavaScript test environment must initialize");
    state
        .evalJavaScriptVoid(
            "var call = __operitRegisterCallSession('detached-call', {}); call.detached = true; call.pendingReferences = 1; JSON.parse = JSON.stringify = function() { throw Error('JSON transport used'); };",
        )
        .expect("detached JavaScript call must initialize");

    state
        .advanceDetachedJavaScriptExecution()
        .expect("detached JavaScript call preparation must accept a boolean predicate");
}

/// Verifies clearing a timer from another call releases the timer owner's reference.
#[test]
fn clearing_timer_from_another_call_releases_timer_owner_reference() {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    let mut state = newTestJsEngineState(None);
    state
        .initJavaScriptEnvironment()
        .expect("JavaScript test environment must initialize");
    let result = state
        .evalJavaScriptString(
            r#"(function() {
                globalThis.__operitNativeScheduleJavaScriptTimer = function() {};
                var owner = __operitRegisterCallSession('timer-owner', {});
                globalThis.__operitCurrentCallId = 'timer-owner';
                var timerId = setInterval(function() {}, 60000);
                var ownerPendingBeforeClear = owner.pendingReferences;
                var clearer = __operitRegisterCallSession('timer-clearer', {});
                globalThis.__operitCurrentCallId = 'timer-clearer';
                clearInterval(timerId);
                return JSON.stringify({
                    ownerPendingBeforeClear: ownerPendingBeforeClear,
                    ownerPendingAfterClear: owner.pendingReferences,
                    clearerPendingAfterClear: clearer.pendingReferences
                });
            })()"#,
        )
        .expect("cross-call timer cleanup must return reference counts");
    let counts: Value = serde_json::from_str(&result).expect("timer reference count JSON");
    assert_eq!(counts["ownerPendingBeforeClear"], 1);
    assert_eq!(counts["ownerPendingAfterClear"], 0);
    assert_eq!(counts["clearerPendingAfterClear"], 0);
}

/// Verifies an asynchronous callback failure cleans its call state before the next request.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn failed_async_callback_does_not_poison_quickjs_engine() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let params = testParams();
    let error = engine
        .execute_script_function(
            r#"
                exports.fail_timer = function() {
                    return new Promise(function(resolve) {
                        setTimeout(function() {
                            throw new Error('timer callback failed');
                        }, 1);
                        setTimeout(function() {
                            resolve('must not complete');
                        }, 50);
                    });
                };
            "#,
            "fail_timer",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .await
        .expect_err("timer callback failure must reject its request");

    assert_eq!(error.kind, JsExecutionErrorKind::Runtime);

    let output = engine
        .execute_script_function(
            "exports.ready_after_failure = function() { return 'ready'; };",
            "ready_after_failure",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .await
        .expect("the next request must run after an asynchronous callback failure");

    assert_eq!(output.as_deref(), Some("\"ready\""));
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
            super::JsTextResourceSource::ExecutionHost,
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
        super::JsTextResourceSource::ExecutionHost,
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
        super::JsTextResourceSource::ExecutionHost,
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
        super::JsTextResourceSource::ExecutionHost,
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
        super::JsTextResourceSource::ExecutionHost,
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
        super::JsTextResourceSource::ExecutionHost,
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
#[tokio::test(flavor = "current_thread")]
async fn toolpkg_ipc_cross_runtime_dispatch_is_asynchronous() {
    ensure_test_runtime_root();
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestIpcEngine(host.clone());
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

    let output = engine
        .execute_script_function(
            script,
            "remote_ipc",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .await;

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

/// Loads the real packaged workflow; only persistence is replaced with an isolated memory DB.
#[cfg(not(target_arch = "wasm32"))]
fn workflowIpcFixture() -> (
    super::JsEngine,
    Arc<TestPluginConfigExecutionHost>,
    String,
    BTreeMap<String, Value>,
) {
    let root = testRepositoryRoot().join("plugins/packages/buildin/workflow/dist");
    fn collect(root: &Path, directory: &Path, resources: &mut BTreeMap<String, String>) {
        for entry in std::fs::read_dir(directory).expect("workflow modules directory") {
            let path = entry.expect("workflow module entry").path();
            if path.is_dir() {
                collect(root, &path, resources);
            } else if path.extension().is_some_and(|ext| ext == "js") {
                let relative = path
                    .strip_prefix(root)
                    .expect("relative workflow module")
                    .to_string_lossy()
                    .replace('\\', "/");
                resources.insert(
                    format!("dist/{relative}"),
                    std::fs::read_to_string(path).expect("workflow module source"),
                );
            }
        }
    }
    let mut resources = BTreeMap::new();
    collect(&root, &root, &mut resources);
    let tools = resources["dist/tools.js"].clone();
    let main = format!(
        r#"
        globalThis.PluginConfig = {{
            use: async function(_key, initial) {{ return initial; }},
            flush: async function() {{}}
        }};
        {}
    "#,
        resources["dist/main.js"]
    );
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let package = "workflow.ipc.regression";
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    let engine = super::JsEngine::new_toolpkg_execution_engine(
        host.clone(),
        ToolPkgExecutionContext {
            context_key: format!("toolpkg_main:{package}"),
            container_package_name: package.to_string(),
            api_version: "2.0.0".to_string(),
            text_resource_host: Arc::new(StaticToolPkgTextResourceHost { resources }),
        },
    );
    *host.toolPkgIpcTarget.lock().unwrap() = Some((engine.clone(), main));
    let mut params = testParams();
    for (key, value) in [
        ("__operit_ui_package_name", package.to_string()),
        (
            "__operit_execution_context_key",
            format!("toolpkg_main:{package}"),
        ),
        ("__operit_toolpkg_runtime_kind", "sandbox".to_string()),
        ("__operit_toolpkg_subpackage_id", "workflow".to_string()),
        ("__operit_script_screen", "dist/tools.js".to_string()),
    ] {
        params.insert(key.to_string(), Value::String(value));
    }
    (engine, host, tools, params)
}

/// AI tools share the main engine, but their awaited IPC must still load a cold main service.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn workflow_list_cold_main_same_engine_ipc_completes() {
    let (engine, host, tools, params) = workflowIpcFixture();
    let started = Instant::now();
    let result = engine
        .execute_script_function_with_timeout_millis(
            &tools,
            "list",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2_000,
            None,
        )
        .await;
    let output = expect_js_output(result, "cold workflow:list same-engine IPC");
    let snapshot: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(snapshot["workflows"], serde_json::json!([]));
    assert_eq!(snapshot["runs"], serde_json::json!([]));
    // A warmed main module must remain usable on subsequent calls as well.
    let again = engine
        .execute_script_function_with_timeout_millis(
            &tools,
            "list",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2_000,
            None,
        )
        .await;
    host.toolPkgIpcTarget.lock().unwrap().take();
    let again: Value =
        serde_json::from_str(&expect_js_output(again, "repeated workflow:list")).unwrap();
    assert_eq!(again, snapshot);
    assert!(started.elapsed() < Duration::from_secs(2));
    engine.destroy();
}

/// Event-driven callers release the state so nested same-engine IPC can finish.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn workflow_list_async_same_engine_ipc_completes() {
    let (engine, host, tools, params) = workflowIpcFixture();
    let result = engine
        .execute_script_function_async(
            tools,
            "list".to_string(),
            params,
            BTreeMap::new(),
            None,
            true,
            2_000,
            None,
        )
        .await;
    host.toolPkgIpcTarget.lock().unwrap().take();
    let output = expect_js_output(result, "async workflow:list same-engine IPC");
    let snapshot: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(snapshot["workflows"], serde_json::json!([]));
    engine.destroy();
}

/// Another call can finish while the first awaits a host tool, without borrowing its env/progress.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn pending_call_releases_state_and_preserves_call_context() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let waiting = engine.clone();
    let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
    let callback = Arc::new(move |value: String| {
        sent.send(value).unwrap();
    });
    let first = tokio::spawn(waiting.execute_script_function(
        r#"exports.first = async function() {
            globalThis.firstFinished = false;
            __operitSendIntermediateResult(globalThis.__operitCurrentCallId, 'waiting');
            await Tools.System.sleep(200);
            __operitSendIntermediateResult(globalThis.__operitCurrentCallId, getEnv('CALL_OWNER'));
            globalThis.firstFinished = true;
            return getEnv('CALL_OWNER');
        };"#,
        "first",
        &testParams(),
        &BTreeMap::from([("CALL_OWNER".to_string(), "first".to_string())]),
        Some(callback),
        true,
        2,
        None,
    ));
    assert_eq!(received.recv().await.unwrap(), "waiting");
    let output = engine.execute_script_function(
        r#"exports.second = function() { return { owner: getEnv('CALL_OWNER'), firstFinished: globalThis.firstFinished }; };"#,
        "second", &testParams(),
        &BTreeMap::from([("CALL_OWNER".to_string(), "second".to_string())]),
        None, true, 2, None,
    ).await;
    assert_eq!(
        expect_js_output(output, "interleaved second call"),
        r#"{"owner":"second","firstFinished":false}"#
    );
    assert_eq!(
        expect_js_output(first.await.unwrap(), "interleaved first call"),
        "\"first\""
    );
    assert_eq!(received.recv().await.unwrap(), "first");
    engine.destroy();
}

/// Per-call overrides must keep writes made before an await, rather than restoring an old snapshot.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn call_environment_updates_survive_state_turns() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let output = engine
        .execute_script_function(
            r#"exports.update = async function() {
            __operitNativeSetEnv(String(globalThis.__operitCurrentCallId), 'CALL_OWNER', 'updated');
            __operitNativeSetEnvs(String(globalThis.__operitCurrentCallId), { SECOND: 'second' });
            await Tools.System.sleep(5);
            return { owner: getEnv('CALL_OWNER'), second: getEnv('SECOND') };
        };"#,
            "update",
            &testParams(),
            &BTreeMap::from([("CALL_OWNER".to_string(), "original".to_string())]),
            None,
            true,
            2,
            None,
        )
        .await;
    assert_eq!(
        expect_js_output(output, "environment across state turns"),
        r#"{"owner":"updated","second":"second"}"#
    );
    engine.destroy();
}

/// Abandoning an async request must cancel its session on the owning affine thread.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn abandoned_async_request_cancels_affine_session() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let caller = engine.clone();
    let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal = started.clone();
    let callback = Arc::new(move |_: String| {
        signal.store(true, Ordering::Release);
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async move {
        let task = tokio::task::spawn_local(async move {
            caller
                .execute_script_function_async(
                    r#"exports.wait = async function() {
                    __operitSendIntermediateResult(globalThis.__operitCurrentCallId, 'started');
                    return await new Promise(function() {});
                };"#
                    .to_string(),
                    "wait".to_string(),
                    testParams(),
                    BTreeMap::new(),
                    Some(callback),
                    true,
                    2_000,
                    None,
                )
                .await
        });
        let deadline = Instant::now() + Duration::from_secs(1);
        while !started.load(Ordering::Acquire) && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(
            started.load(Ordering::Acquire),
            "aborted request must have started"
        );
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let output = engine
            .worker
            .runtimeHost
            .executeHostJavaScriptRuntimeStateTask(
                engine.worker.stateHandle,
                1_000,
                Box::new(|state, _| {
                    let state = state.downcast_mut::<JsEngineState>().unwrap();
                    Ok(Box::new(state.pendingScriptExecutions.len()))
                }),
            )
            .unwrap();
        if *output.downcast::<usize>().unwrap() == 0 {
            break;
        }
        assert!(Instant::now() < deadline, "abandoned affine session leaked");
        std::thread::sleep(Duration::from_millis(1));
    }
    engine.destroy();
}

/// Verifies `withContext` resolves through the asynchronous cross-runtime execution contract.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn runtime_context_cross_runtime_dispatch_is_asynchronous() {
    ensure_test_runtime_root();
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestIpcEngine(host.clone());
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
        super::JsTextResourceSource::ExecutionHost,
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

#[tokio::test(flavor = "current_thread")]
/// Verifies Compose rendering waits for the CommonJS module to initialize lexical bindings.
async fn compose_dsl_default_export_can_capture_later_lexical_constants() {
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
        engine
            .execute_compose_dsl_script(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
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
#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_resource_snapshot_avoids_host_reentry_for_render_and_action() {
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
        engine
            .execute_compose_dsl_script(script, &params, &BTreeMap::new(), textResources)
            .await,
        "compose resource snapshot render result",
    );
    let rendered = serde_json::from_str::<Value>(&raw).expect("compose render json");
    assert_eq!(rendered["tree"]["props"]["label"], "resource-snapshot");
    let actionId = rendered["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .expect("compose snapshot action id");

    let actionRaw = expect_js_output(
        engine
            .execute_compose_dsl_action(actionId, None, &params, &BTreeMap::new(), None)
            .await,
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

#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_action_uses_rendered_runtime() {
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
        engine
            .execute_compose_dsl_script(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
        "compose render result",
    );
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");
    let actionId = parsed["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .expect("action id");

    let actionRaw = expect_js_output(
        engine
            .execute_compose_dsl_action(actionId, None, &params, &BTreeMap::new(), None)
            .await,
        "compose action result",
    );
    let actionParsed = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");
    assert_eq!(actionParsed["actionResult"], 1);
}

#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_action_updates_runtime_options_state_store() {
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
        engine
            .execute_compose_dsl_script(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
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
        engine
            .execute_compose_dsl_action(
                &actionId,
                Some(Value::Bool(true)),
                &params,
                &BTreeMap::new(),
                None,
            )
            .await,
        "compose action result",
    );
    let actionParsed = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");

    assert_eq!(actionParsed["state"]["enabled"], true);
    assert_eq!(actionParsed["tree"]["props"]["checked"], true);
}

/// Verifies that the final DSL tree is rendered after an asynchronous toggle action settles.
#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_async_toggle_action_renders_settled_state() {
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
        engine
            .execute_compose_dsl_script(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
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
        engine
            .execute_compose_dsl_action(
                &actionId,
                Some(Value::Bool(true)),
                &params,
                &BTreeMap::new(),
                None,
            )
            .await,
        "compose async toggle action result",
    );
    let action =
        serde_json::from_str::<Value>(&actionRaw).expect("compose async toggle action json");

    assert_eq!(action["state"]["enabled"], true);
    assert_eq!(action["tree"]["props"]["checked"], true);
}

#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_action_can_access_bootstrap_globals() {
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
        engine
            .execute_compose_dsl_script(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
        "compose render result",
    );
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");
    let actionId = parsed["tree"]["props"]["onLoad"]["__actionId"]
        .as_str()
        .expect("action id");

    let actionRaw = expect_js_output(
        engine
            .execute_compose_dsl_action(actionId, None, &params, &BTreeMap::new(), None)
            .await,
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
        super::JsTextResourceSource::ExecutionHost,
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
        super::JsTextResourceSource::ExecutionHost,
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
    let repoRoot = testRepositoryRoot();
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
    let repoRoot = testRepositoryRoot();
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
    let repoRoot = testRepositoryRoot();
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
    let output = state.execute_script_function_on_current_thread(
        super::JsTextResourceSource::Snapshot(Arc::new(textResources)),
        &script,
        "load_shared",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );
    let output = output.expect("message_insert shared module should execute");
    assert!(output.is_some());
}

/// Verifies the message-insert input-menu hook loads the main entry and its UI dependency chain.
#[test]
fn execute_message_insert_input_menu_hook_from_main_entry() {
    ensure_test_runtime_root();
    let repoRoot = testRepositoryRoot();
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
    let output = state.execute_script_function_on_current_thread(
        super::JsTextResourceSource::Snapshot(Arc::new(textResources)),
        &script,
        "onInputMenuToggle",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );
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
            api_version: "2.0.0".to_string(),
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

/// Exercises the packaged plan question screen through the asynchronous host used by TUI.
#[test]
fn render_planask_through_async_compose_host() {
    ensure_test_runtime_root();
    let root = testRepositoryRoot();
    let dist = root.join("plugins/packages/buildin/plan_mode/dist");
    let script = std::fs::read_to_string(dist.join("ui/planask/index.ui.js")).unwrap();
    let mut resources = BTreeMap::new();
    collect_message_insert_text_resources(&dist, &dist, &mut resources);
    let mut params = testParams();
    params.insert(
        "packageName".into(),
        serde_json::json!("com.operit.plan_mode_bundle"),
    );
    params.insert(
        "toolPkgId".into(),
        serde_json::json!("com.operit.plan_mode_bundle"),
    );
    params.insert(
        "__operit_toolpkg_runtime_kind".into(),
        serde_json::json!("ui"),
    );
    params.insert(
        "__operit_script_screen".into(),
        serde_json::json!("dist/ui/planask/index.ui.js"),
    );
    params.insert("routeInstanceId".into(), serde_json::json!("test-planask"));
    params.insert("state".into(), serde_json::json!({"xmlContent": "<planask><title>Plan</title><question id=\"q1\"><title>Choose</title><option id=\"a\">A</option><option id=\"b\">B</option></question></planask>"}));
    params.insert("memo".into(), serde_json::json!({}));
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let raw = expect_js_output(
        runtime.block_on(engine.execute_compose_dsl_script_async(
            script,
            params.clone(),
            BTreeMap::new(),
            Arc::new(resources),
        )),
        "planask async render",
    );
    let result: Value = serde_json::from_str(&raw).unwrap();
    assert!(
        result["tree"].is_object(),
        "Unexpected render result: {raw}"
    );
    params.insert("state".into(), result["state"].clone());
    params.insert("memo".into(), result["memo"].clone());
    let action = result["tree"]["props"]["onLoad"]["__actionId"]
        .as_str()
        .unwrap()
        .to_string();
    let intermediate = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = intermediate.clone();
    let raw = expect_js_output(
        runtime.block_on(engine.dispatch_compose_dsl_action_result_async(
            action,
            None,
            params,
            BTreeMap::new(),
            Some(Arc::new(move |value| captured.lock().unwrap().push(value))),
        )),
        "planask onLoad",
    );
    let result: Value = serde_json::from_str(&raw).unwrap();
    assert!(
        result["tree"].is_object(),
        "Unexpected action result: {raw}"
    );
    let intermediate = intermediate.lock().unwrap();
    assert!(
        !intermediate.is_empty(),
        "onLoad must deliver intermediate renders"
    );
    for raw in intermediate.iter() {
        let result: Value = serde_json::from_str(raw).unwrap();
        assert!(
            result["tree"].is_object(),
            "Unexpected intermediate result: {raw}"
        );
    }
    engine.destroy();
}

/// Verifies a detached Compose timer can publish a render after its action settles.
#[test]
fn compose_timer_state_change_reaches_intermediate_render_after_action_completion() {
    ensure_test_runtime_root();
    let script = r#"
        function Screen(ctx) {
            const [count, setCount] = ctx.useState('count', 0);
            const timer = ctx.useRef('timer', null);
            const start = function() {
                timer.current = setInterval(function() {
                    setCount(1);
                    clearInterval(timer.current);
                }, 5);
            };
            return ctx.UI.Column({ onLoad: start }, [
                ctx.UI.Text({ text: String(count) })
            ]);
        }
        exports.default = Screen;
    "#;
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let runtime = tokio::runtime::Runtime::new().expect("JavaScript async test runtime must start");
    let params = testParams();
    let renderedRaw = expect_js_output(
        runtime.block_on(engine.execute_compose_dsl_script_async(
            script.to_string(),
            params.clone(),
            BTreeMap::new(),
            Arc::new(BTreeMap::new()),
        )),
        "Compose timer initial render",
    );
    let rendered: Value = serde_json::from_str(&renderedRaw).expect("initial render JSON");
    let actionId = rendered["tree"]["props"]["onLoad"]["__actionId"]
        .as_str()
        .expect("Compose onLoad action id")
        .to_string();
    let mut actionParams = params;
    actionParams.insert("state".to_string(), rendered["state"].clone());
    actionParams.insert("memo".to_string(), rendered["memo"].clone());
    let intermediate = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let intermediateForCallback = intermediate.clone();
    let finalRaw = expect_js_output(
        runtime.block_on(engine.dispatch_compose_dsl_action_result_async(
            actionId,
            None,
            actionParams,
            BTreeMap::new(),
            Some(Arc::new(move |value| {
                intermediateForCallback
                    .lock()
                    .expect("Compose timer intermediate mutex poisoned")
                    .push(value);
            })),
        )),
        "Compose timer action",
    );
    let finalResult: Value = serde_json::from_str(&finalRaw).expect("final action JSON");
    assert_eq!(finalResult["state"]["count"], 0);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if intermediate
            .lock()
            .expect("Compose timer intermediate mutex poisoned")
            .iter()
            .any(|raw| {
                serde_json::from_str::<Value>(raw)
                    .ok()
                    .and_then(|value| value["state"]["count"].as_i64())
                    == Some(1)
            })
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "detached timer render was not delivered"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    engine.destroy();
}

/// Verifies the real extra-info Compose screen resolves its parent shared module.
#[tokio::test(flavor = "current_thread")]
async fn render_message_insert_compose_dsl_screen() {
    ensure_test_runtime_root();
    let repoRoot = testRepositoryRoot();
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
    let output = engine
        .execute_compose_dsl_script(&script, &params, &BTreeMap::new(), Arc::new(textResources))
        .await;
    let raw = expect_js_output(output, "message_insert compose render");
    let rendered = serde_json::from_str::<Value>(&raw).expect("message_insert compose render JSON");
    assert!(rendered["tree"].is_object());
    engine.destroy();
}

/// Verifies the real message-insert master switch renders the immediate local state change.
#[tokio::test(flavor = "current_thread")]
async fn message_insert_compose_master_switch_updates_before_async_persistence() {
    ensure_test_runtime_root();
    let repoRoot = testRepositoryRoot();
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
        engine
            .execute_compose_dsl_script(&script, &params, &BTreeMap::new(), resources)
            .await,
        "message_insert compose render",
    );
    let rendered = serde_json::from_str::<Value>(&renderedRaw).expect("rendered JSON");
    let actionId = find_switch_action_for_text(&rendered["tree"], "额外信息注入")
        .expect("master switch action id");
    let mut actionParams = params.clone();
    actionParams.insert("state".to_string(), rendered["state"].clone());
    actionParams.insert("memo".to_string(), rendered["memo"].clone());
    let actionRaw = expect_js_output(
        engine
            .execute_compose_dsl_action(
                &actionId,
                Some(Value::Bool(true)),
                &actionParams,
                &BTreeMap::new(),
                None,
            )
            .await,
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
        super::JsTextResourceSource::ExecutionHost,
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

/// Allows both module-top-level and registerToolPkg configuration calls while a manager lock is held.
#[test]
fn registration_config_directory_works_before_installation_without_reentering_manager() {
    for scope in [ToolPkgConfigScope::Device, ToolPkgConfigScope::Space] {
        let host = Arc::new(TestPluginConfigExecutionHost::default());
        let engine = newTestJsEngine(host.clone());
        let _manager = host
            .packageManagerLock
            .lock()
            .expect("loading holds package manager");
        let mut params = testParams();
        params.insert(
            "toolPkgId".to_string(),
            Value::String("first_import".to_string()),
        );
        params.insert(
            "__operit_registration_config_scope".to_string(),
            Value::String(scope.as_str().to_string()),
        );
        let script = r#"
            const directory = ToolPkg.getConfigDir();
            exports.registerToolPkg = function() {
                ToolPkg.registerNavigationEntry({
                    id: 'config-check',
                    directory: directory,
                    aliasDirectory: ToolPkg.getConfigDir('named_alias')
                });
                return true;
            };
        "#;
        let capture = engine
            .executeToolPkgMainRegistrationWithTimeout(script, "registerToolPkg", &params, None, 1)
            .expect("registration must resolve paths while its caller holds the manager lock");
        let entry: Value = serde_json::from_str(&capture.navigationEntries[0]).unwrap();
        let root = format!(
            "/app/data/extensions/{}/plugins/configs/first_import",
            scope.as_str()
        );
        assert_eq!(entry["directory"], root);
        assert_eq!(
            entry["aliasDirectory"],
            format!("{root}/namespaces/named_alias")
        );
        assert_eq!(host.registrationConfigReads.load(Ordering::Relaxed), 2);
    }
}

/// Clears registration-only configuration routing before the engine executes a runtime function.
#[tokio::test(flavor = "current_thread")]
async fn registration_config_context_does_not_leak_into_runtime_execution() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("first_import".to_string()),
    );
    params.insert(
        "__operit_registration_config_scope".to_string(),
        Value::String("space".to_string()),
    );
    engine.execute_toolpkg_main_registration_function(
        "const path = ToolPkg.getConfigDir(); exports.registerToolPkg = function() { return true; };",
        "registerToolPkg", &params,
    ).expect("space registration configuration");
    let output = engine
        .execute_script_function(
            "exports.read_config = function() { return ToolPkg.getConfigDir(); };",
            "read_config",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .await
        .expect("runtime configuration must use the installed-owner callback")
        .expect("runtime configuration result");
    assert_eq!(
        serde_json::from_str::<String>(&output).unwrap(),
        "/app/data/extensions/device/plugins/configs/first_import"
    );
    assert_eq!(host.registrationConfigReads.load(Ordering::Relaxed), 1);
}

/// Rejects an unspecified registration scope instead of choosing an arbitrary storage location.
#[test]
fn registration_config_requires_the_selected_scope() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("first_import".to_string()),
    );
    let error = engine.execute_toolpkg_main_registration_function(
        "const directory = ToolPkg.getConfigDir(); exports.registerToolPkg = function() { return true; };",
        "registerToolPkg", &params,
    ).expect_err("configuration requires an explicit registration scope");
    assert!(error
        .message
        .find("ToolPkg registration configuration scope is missing")
        .is_some());
    assert_eq!(host.registrationConfigReads.load(Ordering::Relaxed), 0);
}

/// Verifies scoped configuration failures are thrown rather than returned as path text.
#[test]
fn scoped_config_directory_throws_host_failure() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        exports.read_config = function() {
            try {
                __operitNativeGetScopedPluginConfigDir('missing', 'missing');
                return 'unexpected success';
            } catch (error) {
                return error.message;
            }
        };
    "#;
    let output = state.execute_script_function_on_current_thread(
        super::JsTextResourceSource::ExecutionHost,
        script,
        "read_config",
        &testParams(),
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );
    let output = expect_js_output(output, "scoped configuration failure");
    assert_eq!(
        serde_json::from_str::<String>(&output).expect("serialized error"),
        "JavaScript execution host is unavailable"
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
        super::JsTextResourceSource::ExecutionHost,
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
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("test.package".to_string()),
    );

    let output = state.execute_script_function_on_current_thread(
        super::JsTextResourceSource::ExecutionHost,
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

    assert_eq!(
        path,
        "/app/data/extensions/device/plugins/configs/plugin_name"
    );
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
        super::JsTextResourceSource::ExecutionHost,
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

/// Reads session ownership on the affine executor without driving JavaScript work.
async fn executionSessionCounts(engine: &super::JsEngine) -> (usize, usize) {
    let output = engine
        .worker
        .runtimeHost
        .executeHostJavaScriptRuntimeStateAsyncTask(
            engine.worker.stateHandle,
            1_000,
            Box::new(|state, _interrupt| {
                Box::pin(async move {
                    let state = state
                        .downcast_mut::<JsEngineState>()
                        .expect("test engine state");
                    Ok(Box::new((
                        state.pendingScriptExecutions.len(),
                        state.detachedCallContexts.len(),
                    ))
                        as operit_host_api::HostJavaScriptRuntimeStateOutput)
                })
            }),
        )
        .await
        .expect("Host must inspect the test state");
    *output
        .downcast::<(usize, usize)>()
        .expect("session count output")
}

/// Finishes a deliberately suspended tool without a delay or polling loop.
fn finishGatedTool(host: &TestPluginConfigExecutionHost) {
    host.gatedToolCalls
        .lock()
        .unwrap()
        .pop()
        .expect("one gated tool must be pending")
        .send(JsToolCallResult {
            success: true,
            ..JsToolCallResult::default()
        })
        .expect("gated tool must still own its receiver");
}

/// Primary completion releases the request while a detached Promise retains its original context.
#[tokio::test(flavor = "current_thread")]
async fn completion_retains_detached_promise_context_until_host_event() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let (sender, mut progress) = tokio::sync::mpsc::unbounded_channel();
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("workflow".to_string()),
    );
    let output = engine.execute_script_function(
        r#"exports.main = function() {
            globalThis.detachedRuns = 0;
            toolCall('gate', {}).then(function() {
                globalThis.detachedRuns++;
                __operitNativeSetEnv(String(globalThis.__operitCurrentCallId), 'DETACHED_HOST_OWNER', 'original');
                sendIntermediateResult({
                    owner: getEnv('CALL_OWNER'),
                    runs: globalThis.detachedRuns,
                    config: ToolPkg.getConfigDir()
                });
            });
            return 'primary';
        };"#, "main", &params,
        &BTreeMap::from([("CALL_OWNER".to_string(), "original".to_string())]),
        Some(Arc::new(move |value| { sender.send(value).unwrap(); })), true, 2, None,
    ).await;
    assert_eq!(
        expect_js_output(output, "primary completion"),
        "\"primary\""
    );
    host.gatedToolStarted.notified().await;
    assert_eq!(executionSessionCounts(&engine).await, (0, 1));
    let output = engine
        .execute_script_function(
            "exports.next = function() { return getEnv('CALL_OWNER'); };",
            "next",
            &testParams(),
            &BTreeMap::from([("CALL_OWNER".to_string(), "next".to_string())]),
            None,
            true,
            2,
            None,
        )
        .await;
    assert_eq!(expect_js_output(output, "next request"), "\"next\"");
    finishGatedTool(&host);
    let resumed: Value = serde_json::from_str(
        &tokio::time::timeout(Duration::from_secs(2), progress.recv())
            .await
            .expect("Host must resume the detached continuation")
            .expect("detached progress"),
    )
    .unwrap();
    assert_eq!(
        resumed,
        serde_json::json!({
            "owner": "original", "runs": 1,
            "config": "/app/data/extensions/device/plugins/configs/workflow",
        })
    );
    assert_eq!(
        host.environment
            .lock()
            .unwrap()
            .get("DETACHED_HOST_OWNER")
            .map(String::as_str),
        Some("original"),
    );
    assert_eq!(executionSessionCounts(&engine).await, (0, 0));
    assert!(
        progress.try_recv().is_err(),
        "continuation must execute exactly once"
    );
    engine.destroy();
}

/// Destruction completes a pending request rather than leaving its caller suspended.
#[tokio::test(flavor = "current_thread")]
async fn destruction_completes_pending_request() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let request = tokio::spawn(engine.execute_script_function(
        "exports.main = async function() { return await toolCall('gate', {}); };",
        "main",
        &testParams(),
        &BTreeMap::new(),
        None,
        true,
        2,
        None,
    ));
    host.gatedToolStarted.notified().await;
    assert_eq!(executionSessionCounts(&engine).await, (1, 0));
    engine.destroy();
    let error = request
        .await
        .unwrap()
        .expect_err("destroyed state must complete with an error");
    assert_eq!(error.kind, JsExecutionErrorKind::WorkerUnavailable);
    finishGatedTool(&host);
}

/// A CPU-bound Promise continuation obeys the original request deadline, not a polling interval.
#[tokio::test(flavor = "current_thread")]
async fn promise_continuation_respects_execution_interrupt() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let started = Instant::now();
    let error = engine.execute_script_function_with_timeout_millis(
        "exports.main = async function() { await new Promise(function(resolve) { setTimeout(resolve, 1); }); while (true) {} };",
        "main", &testParams(), &BTreeMap::new(), None, true, 40, None,
    ).await.expect_err("CPU-bound continuation must time out");
    assert_eq!(error.kind, JsExecutionErrorKind::Timeout);
    let output = engine
        .execute_script_function(
            "exports.next = function() { return 'usable'; };",
            "next",
            &testParams(),
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .await;
    assert_eq!(
        expect_js_output(output, "post-timeout request"),
        "\"usable\""
    );
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(executionSessionCounts(&engine).await, (0, 0));
    engine.destroy();
}

/// Restores the engine Host for native config and environment calls made by Promise jobs.
#[tokio::test(flavor = "current_thread")]
async fn promise_jobs_restore_execution_host_for_native_calls() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let mut params = testParams();
    params.insert(
        "toolPkgId".to_string(),
        Value::String("workflow".to_string()),
    );
    let output = engine
        .execute_script_function(
            r#"exports.main = async function() {
            await Promise.resolve();
            var config = ToolPkg.getConfigDir();
            __operitNativeSetEnv(String(globalThis.__operitCurrentCallId), 'CONTINUATION_OWNER', 'workflow');
            return { config: config, owner: getEnv('CONTINUATION_OWNER') };
        };"#,
            "main",
            &params,
            &BTreeMap::new(),
            None,
            true,
            2,
            None,
        )
        .await;
    let value: Value =
        serde_json::from_str(&expect_js_output(output, "Promise native Host access")).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "config": "/app/data/extensions/device/plugins/configs/workflow",
            "owner": "workflow",
        })
    );
    assert_eq!(
        host.environment
            .lock()
            .unwrap()
            .get("CONTINUATION_OWNER")
            .map(String::as_str),
        Some("workflow")
    );
    engine.destroy();
}

/// Exercises real QuickJS bootstrap and generated positional-to-tool bindings.
#[tokio::test(flavor = "current_thread")]
async fn edge_io_sdk_aliases_forward_node_interface_args_and_reject_failures() {
    let host=Arc::new(TestPluginConfigExecutionHost::default());
    let engine=newTestJsEngine(host.clone());
    let output=engine.execute_script_function(r#"
        exports.ports = async function() {
            if (tools !== Tools) throw Error('different Tools object');
            const edge=await tools.edge.execute('edge-one',{pluginId:'device.status',action:'read'},{text:'ping'});
            const io=await Tools.io.execute('edge-two',{port:'gpio',operation:'read'},{pin:2});
            let failure='';
            try { await tools.edge.execute('unauthorized',{pluginId:'device.status',action:'read'}); }
            catch(error) { failure=String(error.message || error); }
            return {edge:edge.nodeId,io:io.nodeId,failure};
        };
    "#,"ports",&testParams(),&BTreeMap::new(),None,true,2000,None).await.unwrap().unwrap();
    let result:Value=serde_json::from_str(&output).unwrap();
    assert_eq!(result,serde_json::json!({"edge":"edge-one","io":"edge-two","failure":"Edge node access denied"}));
    let calls=host.edgePortCalls.lock().unwrap();
    assert_eq!(calls.len(),3);
    assert_eq!(calls[0],serde_json::json!({"tool":"edge_execute","params":{"node_id":"edge-one","interface_info":{"pluginId":"device.status","action":"read"},"args":{"text":"ping"}}}));
    assert_eq!(calls[1],serde_json::json!({"tool":"io_execute","params":{"node_id":"edge-two","interface_info":{"port":"gpio","operation":"read"},"args":{"pin":2}}}));
    assert!(calls[2]["params"].get("args").is_none()||calls[2]["params"]["args"].is_null());
    drop(calls);engine.destroy();
}

/// Preserves JSON-compatible data without executing a JSON text transport.
#[tokio::test(flavor = "current_thread")]
async fn structured_tool_calls_preserve_values_and_avoid_json_text() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let result = engine.execute_script_function(
            r#"exports.main = async function(params) {
                var savedStringify = JSON.stringify, savedParse = JSON.parse;
                var parameters = {
                    text: '\"\n\u2028中'.repeat(16384),
                    integer: 7, large: 1e16, date: new Date('2020-01-01T00:00:00Z'),
                    nested: {x:true, omit:undefined}, array: [undefined, , NaN, 2],
                    boxed: new Boolean(false), ['__proto__']: {polluted:true},
                    custom: {toJSON:function(key){return {key:key};}}
                };
                JSON.stringify = function(value) {
                    if(value && value.text===parameters.text) throw new Error('Tool request serialized as text');
                    return savedStringify.apply(this,arguments);
                };
                JSON.parse = function(text) {
                    if(typeof text==='string' && text.indexOf('"text":')>=0) throw new Error('Tool result parsed from text');
                    return savedParse.apply(this,arguments);
                };
                var output;
                try { output = await toolCall('echo', parameters); }
                finally { JSON.stringify=savedStringify; JSON.parse=savedParse; }
                return {
                    textMatches:output.text===parameters.text,
                    integer:output.integer, large:output.large,
                    date:output.date, nested:output.nested, array:output.array,
                    boxed:output.boxed, custom:output.custom,
                    ownProto:Object.prototype.hasOwnProperty.call(output,'__proto__'),
                    noPollution:output.polluted===undefined
                };
            };"#,
            "main", &BTreeMap::new(),
            &BTreeMap::new(), None, true, 5, None,
        ).await;
    let output: Value = serde_json::from_str(&expect_js_output(result, "structured echo")).unwrap();
    assert_eq!(
        output,
        serde_json::json!({
            "textMatches":true, "integer":7, "large":10000000000000000u64,
            "date":"2020-01-01T00:00:00.000Z", "nested":{"x":true},
            "array":[null,null,null,2], "boxed":false, "custom":{"key":"custom"},
            "ownProto":true,"noPollution":true
        })
    );
    engine.destroy();
}

#[tokio::test(flavor = "current_thread")]
async fn structured_tool_calls_recover_from_invalid_parameters_and_preserve_binary_results() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let result = engine
        .execute_script_function(
            r#"exports.main = async function() {
            var failures=[]; var cycle={};cycle.self=cycle;
            for (var p of [cycle,{big:1n},[],{get x(){throw new Error('getter');}}]) {
                try {await toolCall('echo',p);failures.push(false);}catch(e){failures.push(true);}
            }
            return {failures:failures, good:(await toolCall('echo',{x:42})).x,
                binary:await toolCall('binary',{})};
        };"#,
            "main",
            &BTreeMap::new(),
            &BTreeMap::new(),
            None,
            true,
            5,
            None,
        )
        .await;
    let output: Value =
        serde_json::from_str(&expect_js_output(result, "invalid parameters recovery")).unwrap();
    assert_eq!(
        output,
        serde_json::json!({"failures":[true,true,true,true],"good":42,"binary":"AQID"})
    );
    engine.destroy();
}

/// Measures structured round trips through the real engine without installing retired adapters.
#[tokio::test(flavor = "current_thread")]
#[ignore = "manual release bridge benchmark"]
async fn bridge_roundtrip_benchmark() {
    let sizes = std::env::var("OPERIT_JS_BRIDGE_BENCH_SIZES")
        .map(|sizes| {
            sizes
                .split(',')
                .map(|size| {
                    size.trim()
                        .parse::<usize>()
                        .expect("benchmark payload size")
                })
                .collect::<Vec<_>>()
        })
        .expect("OPERIT_JS_BRIDGE_BENCH_SIZES must explicitly specify payload sizes");
    let iterations = std::env::var("OPERIT_JS_BRIDGE_BENCH_ITERATIONS")
        .map(|value| value.parse::<usize>().expect("benchmark iterations"))
        .expect("OPERIT_JS_BRIDGE_BENCH_ITERATIONS must explicitly specify iteration count");
    assert!(iterations > 0, "benchmark iterations must be positive");
    for size in sizes {
        let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
        let params = BTreeMap::from([
            ("size".to_string(), serde_json::json!(size)),
            ("iterations".to_string(), serde_json::json!(iterations)),
        ]);
        let script = r#"exports.main=async function(params){
            var data={text:'x'.repeat(params.size)}, result;
            for(var i=0;i<params.iterations;i++)result=await toolCall('echo',data);
            return result.text.length;
        };"#;
        expect_js_output(
            engine
                .execute_script_function(
                    script,
                    "main",
                    &params,
                    &BTreeMap::new(),
                    None,
                    true,
                    30,
                    None,
                )
                .await,
            "warmup",
        );
        let mut times = Vec::new();
        for _ in 0..5 {
            let start = Instant::now();
            let output = expect_js_output(
                engine
                    .execute_script_function(
                        script,
                        "main",
                        &params,
                        &BTreeMap::new(),
                        None,
                        true,
                        30,
                        None,
                    )
                    .await,
                "benchmark",
            );
            assert_eq!(output, size.to_string());
            times.push(start.elapsed().as_secs_f64() * 1e6 / iterations as f64);
        }
        times.sort_by(|a, b| a.total_cmp(b));
        eprintln!(
            "BRIDGE_BENCH payload={size}B mode=structured median_us_per_roundtrip={:.3}",
            times[2]
        );
        engine.destroy();
    }
}
