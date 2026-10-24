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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[path = "../../../../../tool/services/src/tools/packTool/ToolPkgComposeDslProtocol.rs"]
mod ToolPkgComposeDslProtocol;
#[path = "../../../../../tool/services/src/tools/packTool/ToolPkgComposeDslProgramStore.rs"]
mod ToolPkgComposeDslProgramStore;

/// Uses the production program interpreter for tree-oriented behavior assertions.
#[derive(Default)]
struct AssertionGraph {
    store: ToolPkgComposeDslProgramStore::ToolPkgComposeDslProgramStore,
}

impl AssertionGraph {
    /// Applies the binary arena before projecting only the fields needed by assertions.
    fn accept(&mut self, envelope: operit_host_api::HostJavaScriptValue) -> (bool, Value) {
        let operit_host_api::HostJavaScriptValue::Buffer(bytes) = envelope else { panic!("Expected Compose buffer") };
        let arena = ToolPkgComposeDslProtocol::ComposeBuffer::new(&bytes).unwrap();
        let mut response = serde_json::json!({"navigationCommands": arena.navigation().unwrap()});
        if let Some(result) = arena.action_result().unwrap() { response["actionResult"] = result; }
        if arena.has_update().unwrap() {
            let update = arena.transaction().unwrap();
            response["update"] = serde_json::to_value(&update).unwrap();
            self.store.apply_decoded(update).unwrap();
            response["tree"] = self.tree(self.store.root().unwrap());
        }
        (arena.is_final().unwrap(), response)
    }

    /// Expands finite views only inside assertions outside production delivery.
    fn tree(&self, reference: &ToolPkgComposeDslProtocol::ToolPkgComposeDslReference) -> Value {
        let node = self.store.view(reference).unwrap();
        let children: Vec<Value> = node.children.iter().map(|child| self.tree(child)).collect();
        let slots: BTreeMap<String, Vec<Value>> = node.slots.iter().map(|(name, refs)| (name.clone(), refs.iter().map(|child| self.tree(child)).collect())).collect();
        serde_json::json!({"type": node.node_type, "props": node.props, "children": children, "slots": slots})
    }
}

/// Visits a finite assertion tree without depending on the removed transport records.
fn assertionNodes(response: &Value) -> Vec<&Value> {
    /// Collects ordinary children and explicit slots in visible order.
    fn visit<'a>(node: &'a Value, nodes: &mut Vec<&'a Value>) {
        nodes.push(node);
        for child in node["children"].as_array().unwrap() { visit(child, nodes); }
        for children in node["slots"].as_object().unwrap().values() {
            for child in children.as_array().unwrap() { visit(child, nodes); }
        }
    }
    let mut nodes = Vec::new();
    visit(&response["tree"], &mut nodes);
    nodes
}

/// Resolves a test-owned retained graph for the concrete native engine handle.
fn assertionGraph(engine: &super::JsEngine) -> Arc<Mutex<AssertionGraph>> {
    static GRAPHS: OnceLock<Mutex<BTreeMap<u64, Arc<Mutex<AssertionGraph>>>>> = OnceLock::new();
    GRAPHS.get_or_init(|| Mutex::new(BTreeMap::new())).lock().unwrap()
        .entry(engine.worker.stateHandle.id).or_default().clone()
}

impl super::JsEngine {
    /// Renders through the sole production stream API and projects its response for assertions.
    fn renderForAssertions(&self, script: &str, options: &BTreeMap<String, Value>, env: &BTreeMap<String, String>, resources: Arc<BTreeMap<String, String>>) -> operit_plugin_sdk::javascript::JsExecutionFuture<operit_plugin_sdk::execution_result::JsExecutionResult<Option<Value>>> {
        self.assertionCommand(Some(script.into()), None, None, options.clone(), env.clone(), Some(resources), None)
    }

    /// Adapts owned test arguments without introducing another production execution path.
    fn renderForAssertionsAsync(&self, script: String, options: BTreeMap<String, Value>, env: BTreeMap<String, String>, resources: Arc<BTreeMap<String, String>>) -> operit_plugin_sdk::javascript::JsExecutionFuture<operit_plugin_sdk::execution_result::JsExecutionResult<Option<Value>>> {
        self.assertionCommand(Some(script), None, None, options, env, Some(resources), None)
    }

    /// Dispatches through the sole production action stream and projects the response for assertions.
    fn actionForAssertions(&self, id: &str, payload: Option<Value>, options: &BTreeMap<String, Value>, env: &BTreeMap<String, String>, intermediate: Option<Arc<dyn Fn(Value) + Send + Sync>>) -> operit_plugin_sdk::javascript::JsExecutionFuture<operit_plugin_sdk::execution_result::JsExecutionResult<Option<Value>>> {
        self.assertionCommand(None, Some(id.into()), payload, options.clone(), env.clone(), None, intermediate)
    }

    /// Adapts owned action test arguments to the same assertion capture.
    fn actionForAssertionsAsync(&self, id: String, payload: Option<Value>, options: BTreeMap<String, Value>, env: BTreeMap<String, String>, intermediate: Option<Arc<dyn Fn(Value) + Send + Sync>>) -> operit_plugin_sdk::javascript::JsExecutionFuture<operit_plugin_sdk::execution_result::JsExecutionResult<Option<Value>>> {
        self.assertionCommand(None, Some(id), payload, options, env, None, intermediate)
    }

    /// Captures real synchronous stream responses and inspects JS storage only when a test asks for it.
    fn assertionCommand(&self, script: Option<String>, id: Option<String>, payload: Option<Value>, mut options: BTreeMap<String, Value>, env: BTreeMap<String, String>, resources: Option<Arc<BTreeMap<String, String>>>, intermediate: Option<Arc<dyn Fn(Value) + Send + Sync>>) -> operit_plugin_sdk::javascript::JsExecutionFuture<operit_plugin_sdk::execution_result::JsExecutionResult<Option<Value>>> {
        let engine = self.clone();
        let retained_only = options.remove("__test_retained_response") == Some(Value::Bool(true));
        let graph = assertionGraph(self);
        let final_response = Arc::new(Mutex::new(None));
        let capture = final_response.clone();
        let sink = Arc::new(move |envelope: operit_host_api::HostJavaScriptValue| {
            let (is_final, response) = graph.lock().unwrap().accept(envelope);
            if is_final { *capture.lock().unwrap() = Some(response); }
            else if let Some(callback) = &intermediate { callback(response); }
        });
        Box::pin(async move {
            let result = match script {
                Some(script) => engine.execute_compose_dsl_script_stream_async(script, options.clone(), env.clone(), resources.unwrap(), sink).await?,
                None => engine.dispatch_compose_dsl_action_stream_async(id.unwrap(), payload, options.clone(), env.clone(), sink).await?,
            };
            assert_eq!(result, Some(Value::Null));
            let mut response = final_response.lock().unwrap().take().expect("Command must publish a final response");
            if !retained_only {
                let storage = engine.execute_script_function(
                    "exports.inspectComposeStorage = function() { const bundle = globalThis.__operit_compose_bundle; return { state: bundle.stateStore, memo: bundle.memoStore }; };",
                    "inspectComposeStorage", &options, &env, None, true, None,
                ).await?.unwrap();
                let storage: Value = serde_json::from_str(&storage).unwrap();
                response["state"] = storage["state"].clone();
                response["memo"] = storage["memo"].clone();
            }
            Ok(Some(response))
        })
    }
}

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
    super::JsEngine::new(executionHost).expect("test engine must initialize")
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
    ).expect("test engine must initialize")
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
    super::JsEngine::new_toolpkg_registration_engine().expect("test engine must initialize")
}

/// Creates one directly accessible JavaScript state through the concrete native Host.
#[allow(non_snake_case)]
pub(super) fn newTestJsEngineState(
    executionHost: Option<Arc<dyn JsExecutionHost>>,
) -> JsEngineState {
    let runtime = testJavaScriptRuntimeHost()
        .createHostJavaScriptRuntime()
        .expect("test JavaScript runtime must start");
    JsEngineState::newWithRuntime(
        runtime,
        executionHost,
        None,
        Arc::new(Mutex::new(None)),
        operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(),
        Arc::new(AtomicBool::new(true)),
    )
        .expect("test JavaScript state must initialize")
}

#[derive(Clone, Default)]
struct TestPluginConfigExecutionHost {
    storage: Option<StorageTestFixture>,
    routedPluginCalls: Arc<Mutex<Vec<Value>>>,
    gatedToolCalls: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<JsToolCallResult>>>>,
    blockingToolCalls: Arc<Mutex<Vec<std::sync::mpsc::Sender<JsToolCallResult>>>>,
    edgePortCalls: Arc<Mutex<Vec<Value>>>,
    gatedToolStarted: Arc<tokio::sync::Notify>,
    toolPkgTextResourceReads: Arc<AtomicUsize>,
    registrationConfigReads: Arc<AtomicUsize>,
    toolPkgContextBindings: Arc<AtomicUsize>,
    packageManagerLock: Arc<Mutex<()>>,
    environment: Arc<Mutex<BTreeMap<String, String>>>,
    toolCatalogError: Arc<Mutex<Option<String>>>,
    #[cfg(not(target_arch = "wasm32"))]
    toolPkgIpcThreadName: Arc<Mutex<Option<String>>>,
    #[cfg(not(target_arch = "wasm32"))]
    toolPkgIpcTarget: Arc<Mutex<Option<(super::JsEngine, String)>>>,
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

impl JsExecutionHost for TestPluginConfigExecutionHost {
    /// Uses a concrete Binding identity to exercise native decorator selection.
    fn should_route_toolpkg(&self, binding: String, _streaming: bool) -> Result<bool, String> { Ok(binding == "remote") }

    /// Captures the private native envelope produced by an ordinary decorated call.
    fn call_core_route(&self, target: String, method: String, args: Value) -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        self.routedPluginCalls.lock().unwrap().push(serde_json::json!({"target":target, "method":method, "args":args}));
        Box::pin(async { Ok(serde_json::json!({"chatId":"remote", "count":42})) })
    }

    /// Opens a deterministic host-owned output stream for native bridge verification.
    fn open_core_route_watch(&self, target: String, method: String, args: Value) -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        self.routedPluginCalls.lock().unwrap().push(serde_json::json!({"target":target, "method":method, "args":args}));
        Box::pin(async { Ok(serde_json::json!({"subscriptionId":"native-route-watch"})) })
    }

    /// Returns one standard iterator value through the asynchronous native callback.
    fn next_core_route_watch(&self, _id: String) -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        Box::pin(async { Ok(serde_json::json!({"done":false,"value":"native-stream-value"})) })
    }

    /// Opens the private Push transport underneath an ordinary async iterable input.
    fn open_toolpkg_route_push(&self, method: String, args: Value) -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        self.routedPluginCalls.lock().unwrap().push(serde_json::json!({"reverse":method,"args":args}));
        Box::pin(async {Ok(serde_json::json!({"subscriptionId":"native-input"}))})
    }

    /// Records one ordered value accepted by the existing native input transport.
    fn send_toolpkg_route_push(&self, id: String, value: Value) -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        self.routedPluginCalls.lock().unwrap().push(serde_json::json!({"input":id,"value":value}));
        Box::pin(async {Ok(Value::Null)})
    }

    /// Records completion or cancellation of an input collector.
    fn close_toolpkg_route_push(&self, id: String, abort: bool) -> operit_plugin_sdk::javascript::JsExecutionFuture<Result<Value, String>> {
        self.routedPluginCalls.lock().unwrap().push(serde_json::json!({"inputClosed":id,"abort":abort}));
        Box::pin(async {Ok(Value::Null)})
    }

    /// Records native collector release after the consumer exits its loop.
    fn close_core_route_watch(&self, id: String) -> Result<(), String> {
        self.routedPluginCalls.lock().unwrap().push(serde_json::json!({"closed":id}));
        Ok(())
    }

        /// Captures this fixture's shared state through the same native invocation boundary.
        fn capture_tool_invocation_host(&self) -> Arc<dyn JsExecutionHost> { Arc::new(self.clone()) }



    /// Retains shared host fixtures while accepting only concrete engine-owned package contexts.
    fn for_toolpkg_execution_context(
        &self,
        context: &ToolPkgExecutionContext,
    ) -> Result<Arc<dyn JsExecutionHost>, String> {
        self.toolPkgContextBindings.fetch_add(1, Ordering::SeqCst);
        if context.container_package_name.trim().is_empty() || context.context_key.trim().is_empty()
        {
            return Err("Invalid test execution context".to_string());
        }
        Ok(Arc::new(self.clone()))
    }

    /// Returns an empty catalog for tests that do not install runtime tools.
    fn get_tool_catalog(&self) -> Result<Value, String> {
        if let Some(error) = self.toolCatalogError.lock().unwrap().clone() { return Err(error); }
        Ok(serde_json::json!({ "tools": [] }))
    }

    /// Executes the System sleep call used by the JavaScript worker regression test.
    fn execute_tool_call(
        &self,
        request: JsToolCallRequest,
    ) -> operit_plugin_sdk::javascript::JsExecutionCompletion<JsToolCallResult> {
        if request.tool_name == "storage_request" {
            let decoded = request.parameters.get("request").cloned()
                .ok_or_else(|| "Missing storage request".to_string())
                .and_then(|value| serde_json::from_value(value).map_err(|error| error.to_string()));
            let operation = match decoded {
                Ok(request) => storageTestRequest(self, request),
                Err(error) => return Box::pin(async move { JsToolCallResult {
                    success: false, data: JsToolCallResultData::Value(Value::Null), error: Some(error),
                } }),
            };
            return Box::pin(async move { match operation.await {
                Ok(value) => JsToolCallResult { success: true, data: JsToolCallResultData::Value(value), error: None },
                Err(error) => JsToolCallResult { success: false, data: JsToolCallResultData::Value(Value::Null), error: Some(error.to_string()) },
            } });
        }
        let gatedToolCalls = self.gatedToolCalls.clone();
        let blockingToolCalls = self.blockingToolCalls.clone();
        let gatedToolStarted = self.gatedToolStarted.clone();
        let edgePortCalls = self.edgePortCalls.clone();
        Box::pin(async move {
            if request.device.as_ref().is_some_and(|device| device.action != operit_plugin_sdk::javascript::JsDeviceToolAction::Port) {
                return JsToolCallResult {success:true, data:JsToolCallResultData::Value(serde_json::to_value(request).unwrap()), error:None};
            }
            if matches!(request.tool_name.as_str(),"edge_execute"|"io_execute") {
                edgePortCalls.lock().unwrap().push(serde_json::json!({"tool":request.tool_name,"params":request.parameters,"device":request.device}));
                let node = request.device.as_ref().map(|device| device.node_id.as_str()).unwrap_or("");
                return if node == "unauthorized" {
                    JsToolCallResult {success:false,data:JsToolCallResultData::Value(Value::Null),error:Some("Edge node access denied".into())}
                } else {
                    JsToolCallResult {success:true,data:JsToolCallResultData::Value(serde_json::json!({"nodeId":node,"data":{"ok":true}})),error:None}
                };
            }

            if request.tool_name == "echo_owned" {
                return JsToolCallResult {
                    success: true,
                    data: JsToolCallResultData::Value(Value::Object(request.parameters.into_iter().collect())),
                    error: None,
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
            if request.tool_name == "blocking_gate" {
                let (sender, receiver) = std::sync::mpsc::channel();
                blockingToolCalls.lock().unwrap().push(sender);
                gatedToolStarted.notify_one();
                return receiver.recv_timeout(Duration::from_secs(5))
                    .expect("test must finish the blocking Host call");
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

    /// Returns actual Host-visible local data without acquiring the package manager lock.
    fn plugin_local_data_dir(&self, owner_id: &str) -> Result<String, String> {
        let path = operit_store::ExtensionStore::ExtensionStore::localDataPath(owner_id)?;
        Ok(format!("/app/data/{}", path.strip_prefix("runtime/").ok_or("invalid local data path")?))
    }

    /// Creates registration-local paths independently of the selected installation scope.
    fn registration_plugin_local_data_dir(&self, owner_id: &str) -> Result<String, String> {
        self.plugin_local_data_dir(owner_id)
    }

    /// Returns shared data independently of the installed package scope.
    fn plugin_space_data_dir(&self, owner_id: &str) -> Result<String, String> {
        let path = operit_store::ExtensionStore::ExtensionStore::spaceDataPath(owner_id)?;
        Ok(format!("/app/data/{}", path.strip_prefix("runtime/").ok_or("invalid shared data path")?))
    }

    /// Creates registration-shared paths without consulting installed state.
    fn registration_plugin_space_data_dir(&self, owner_id: &str) -> Result<String, String> {
        self.plugin_space_data_dir(owner_id)
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

    /// Rejects unexpected Compose DSL file-save requests.
    fn save_compose_file(&self, _payload: &serde_json::Value) -> Result<serde_json::Value, String> {
        panic!("Compose DSL file saving is not part of the plugin config test")
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
                        BTreeMap::new(), None, true, None,
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
async fn before_hook_timeout_interrupts_quickjs_worker() {
    let engine = newTestToolPkgRegistrationEngine();
    let params = testParams();
    let started = Instant::now();
    let error = engine
        .execute_script_function_with_timeout_millis(
            "exports.block = function() { while (true) {} };",
            "block",
            &params,
            &BTreeMap::new(),
            None,
            true, 1_000,
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

/// Verifies the promise-job pump resumes execution when a callback wake is lost.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn lost_callback_wake_is_recovered_by_the_promise_job_pump() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    engine
        .worker
        .asyncCallbackWakeEnabled
        .store(false, Ordering::Release);
    let started = Instant::now();
    let output = engine
        .execute_script_function(
            r#"
                exports.wait = async function() {
                    var slept = await Tools.System.sleep(20);
                    return slept.sleptMs;
                };
            "#,
            "wait",
            &testParams(),
            &BTreeMap::new(),
            None,
            true,
            None,
        )
        .await
        .expect("the promise job pump must resume a lost callback wake");
    assert_eq!(output.as_deref(), Some("20"));
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "the pump must recover the lost wake well before the 2 second deadline"
    );
    engine
        .worker
        .asyncCallbackWakeEnabled
        .store(true, Ordering::Release);
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
            None,
        )
        .await
        .expect("tool catalog bridge must complete");

    assert_eq!(output.as_deref(), Some(r#"{"tools":[]}"#));
    engine.destroy();
}

/// Verifies JavaScript timers finish before explicitly released Host tool work.
#[tokio::test(flavor = "current_thread")]
async fn javascript_timer_can_win_race_against_async_tool_call() {
    ensure_test_runtime_root();
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let params = testParams();
    let (sender, mut progress) = tokio::sync::mpsc::unbounded_channel();
    let (output, ()) = tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(
            engine.execute_script_function(
                r#"
                    exports.race = async function() {
                        var toolResult = toolCall("gate", {}).then(function() {
                            sendIntermediateResult({ completed: "tool" });
                            return "tool";
                        });
                        var timeoutResult = new Promise(function(resolve) {
                            setTimeout(function() { resolve("timeout"); }, 20);
                        });
                        return Promise.race([toolResult, timeoutResult]);
                    };
                "#,
                "race",
                &params,
                &BTreeMap::new(),
                Some(Arc::new(move |value| { sender.send(value).unwrap(); })),
                true,
                None,
            ),
            host.gatedToolStarted.notified(),
        )
    })
    .await
    .expect("JavaScript timer must finish while the Host tool remains suspended");

    assert_eq!(
        output.expect("JavaScript timer must complete").as_deref(),
        Some("\"timeout\""),
    );
    assert_eq!(host.gatedToolCalls.lock().unwrap().len(), 1);
    assert!(progress.try_recv().is_err(), "Host tool must remain suspended");
    finishGatedTool(&host);
    let completion = tokio::time::timeout(Duration::from_secs(2), progress.recv())
        .await
        .expect("released Host tool must resume its JavaScript continuation")
        .expect("Host tool completion must be delivered");
    assert_eq!(
        serde_json::from_str::<Value>(&completion).unwrap(),
        serde_json::json!({ "completed": "tool" }),
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
                globalThis.__operitNativeScheduleJavaScriptTimer = function() { return new Promise(function() {}); };
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
            None,
        )
        .await
        .expect("the next request must run after an asynchronous callback failure");

    assert_eq!(output.as_deref(), Some("\"ready\""));
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
    /// Flushes the existing test file through its backing filesystem.
    fn flushFile(&self, path: &str) -> operit_host_api::HostResult<()> {
        std::fs::OpenOptions::new().write(true).open(self.resolve(path)?)?.sync_all()?;
        Ok(())
    }

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

    /// Atomically publishes one complete in-memory test entry.
    fn writeBytesAtomically(&self, path: &str, content: &[u8]) -> operit_host_api::HostResult<()> {
        self.writeBytes(path, content)
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

/// Keeps large hot calls on source handles and preserves the module's mutable state.
#[test]
fn registered_script_hot_calls_transfer_only_handles_without_hashing() {
    let mut state = newTestJsEngineState(None);
    state.evalJavaScriptVoid(r#"
        globalThis.__sourceRegistrations = 0;
        globalThis.__sourceInvocations = 0;
        const registerSource = __operitRegisterScriptSource;
        __operitRegisterScriptSource = function(id, text) {
            globalThis.__sourceRegistrations += 1;
            return registerSource(id, text);
        };
        const invokeSource = __operitExecuteScriptFunction;
        __operitExecuteScriptFunction = function() {
            if (typeof arguments[2] !== 'string' || arguments[2].length > 32) {
                throw new Error('Invocation transferred source text instead of its handle');
            }
            globalThis.__sourceInvocations += 1;
            return invokeSource.apply(this, arguments);
        };
    "#).unwrap();
    let script = format!("/*{}*/ let calls = 0; exports.count = () => ++calls;", "x".repeat(524_000));
    for index in 1..=10 {
        let output = state.execute_script_function_on_current_thread(
            super::JsTextResourceSource::ExecutionHost,
            &script, "count", &testParams(), &BTreeMap::new(), None, true, 60, None,
        );
        assert_eq!(expect_js_output(output, "registered source counter"), index.to_string());
        if index == 1 {
            state.evalJavaScriptVoid(r#"
                __operitNativeHashText = function() { throw new Error('Hot call rescanned source'); };
            "#).unwrap();
        }
    }
    state.evalJavaScriptVoid(r#"
        if (__sourceRegistrations !== 1 || __sourceInvocations !== 10) {
            throw new Error('Source registration or invocation count changed');
        }
    "#).unwrap();
}

/// Distinguishes equal-length legacy fingerprint collisions and restores each source's own state.
#[test]
fn registered_script_replacement_preserves_exact_source_identity() {
    let mut state = newTestJsEngineState(None);
    let first = "let count = 0; exports.read = () => 'Aa:' + ++count;";
    let second = "let count = 0; exports.read = () => 'BB:' + ++count;";
    assert_eq!(super::javaScriptSourceFingerprint(first), super::javaScriptSourceFingerprint(second));
    for (script, expected) in [(first, "\"Aa:1\""), (second, "\"BB:1\""), (first, "\"Aa:2\"")] {
        let output = state.execute_script_function_on_current_thread(
            super::JsTextResourceSource::ExecutionHost,
            script, "read", &testParams(), &BTreeMap::new(), None, true, 60, None,
        );
        assert_eq!(expect_js_output(output, "source replacement"), expected);
    }
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

/// Exercises real QuickJS logging, native severity/depth metadata, and strict level validation.
#[test]
fn plugin_console_uses_shared_diagnostic_depths() {
    let mut state = newTestJsEngineState(None);
    let script = r#"
        /** Exercises console forwarding, diagnostic depths, and strict validation. */
        exports.verify_logging = function() {
            const callId = String(globalThis.__operitCurrentCallId);
            for (let level = 1; level <= 6; level++) console.verbose(level, 'log-policy-v' + level);
            console.debug('log-policy-debug');
            console.log('log-policy-log');
            console.info('log-policy-info');
            console.warn('log-policy-warn');
            console.error('log-policy-error');
            let rejected = 0;
            for (const level of [0, 7, 1.5, '3', null]) {
                try { console.verbose(level, 'invalid-policy'); }
                catch (error) { if (error instanceof RangeError) rejected++; }
            }
            let nativeRejected = false;
            try { __operitNativeLogVerbose(7, callId, 'invalid-native-policy'); }
            catch (_) { nativeRejected = true; }
            return { callId, rejected, nativeRejected };
        };
    "#;
    let output = state.execute_script_function_on_current_thread(
        super::JsTextResourceSource::ExecutionHost,
        script,
        "verify_logging",
        &testParams(),
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );
    let result: serde_json::Value =
        serde_json::from_str(&expect_js_output(output, "plugin logging policy")).unwrap();
    assert_eq!(result["rejected"], 5);
    assert_eq!(result["nativeRejected"], true);
    let call_id = result["callId"].as_str().unwrap();
    let entries = operit_util::AppLogger::AppLogger::entries();
    for level in 1..=6 {
        let message = format!("[{call_id}] log-policy-v{level}");
        let entry = entries
            .iter()
            .find(|entry| entry.tag == "ToolPkg" && entry.message == message)
            .unwrap();
        assert_eq!(entry.priority, operit_util::AppLogger::VERBOSE);
        assert_eq!(entry.verbose_level, Some(level));
    }
    for (label, priority) in [
        ("debug", operit_util::AppLogger::DEBUG),
        ("log", operit_util::AppLogger::INFO),
        ("info", operit_util::AppLogger::INFO),
        ("warn", operit_util::AppLogger::WARN),
        ("error", operit_util::AppLogger::ERROR),
    ] {
        let message = format!("[{call_id}] log-policy-{label}");
        let entry = entries
            .iter()
            .find(|entry| entry.tag == "ToolPkg" && entry.message == message)
            .unwrap();
        assert_eq!(entry.priority, priority);
        assert_eq!(entry.verbose_level, None);
    }
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
        serde_json::from_str::<Value>(&expect_js_output(output, "ToolPkg IPC local call")).unwrap(),
        serde_json::json!({"value":42,"channel":"test.local","runtime":"main"})
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
    ).expect("test engine must initialize");
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
        None,
    ));
    assert_eq!(received.recv().await.unwrap(), "waiting");
    let output = engine.execute_script_function(
        r#"exports.second = function() { return { owner: getEnv('CALL_OWNER'), firstFinished: globalThis.firstFinished }; };"#,
        "second", &testParams(),
        &BTreeMap::from([("CALL_OWNER".to_string(), "second".to_string())]),
        None, true, None,
    ).await;
    assert_eq!(
        serde_json::from_str::<Value>(&expect_js_output(output, "interleaved second call")).unwrap(),
        serde_json::json!({"owner":"second","firstFinished":false})
    );
    assert_eq!(
        expect_js_output(first.await.unwrap(), "interleaved first call"),
        "\"first\""
    );
    assert_eq!(received.recv().await.unwrap(), "first");
    engine.destroy();
}

/// A package awaiting a synchronous host tool must leave another package's tools and timers usable.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn blocking_host_call_leaves_other_package_runtime_responsive() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let firstEngine = newTestJsEngine(host.clone());
    let secondEngine = newTestJsEngine(host.clone());
    let first = tokio::spawn(firstEngine.execute_script_function(
        "exports.first = async function() { return await toolCall('blocking_gate', {}); };",
        "first", &testParams(), &BTreeMap::new(), None, true, None,
    ));
    host.gatedToolStarted.notified().await;
    let output = tokio::time::timeout(Duration::from_secs(1), secondEngine.execute_script_function(
        "exports.second = async function() { await Tools.System.sleep(5); return await toolCall('echo', {value: 'other-package'}); };",
        "second", &testParams(), &BTreeMap::new(), None, true, None,
    )).await;
    host.blockingToolCalls.lock().unwrap().pop().unwrap().send(JsToolCallResult {
        success: true,
        data: JsToolCallResultData::Value(Value::String("terminal-done".into())),
        error: None,
    }).unwrap();
    let firstOutput = first.await.unwrap();
    firstEngine.destroy();
    secondEngine.destroy();
    assert_eq!(expect_js_output(firstOutput, "blocking terminal result"), "\"terminal-done\"");
    assert_eq!(expect_js_output(output.expect("Other packages must progress before the terminal finishes"), "other package"), r#"{"value":"other-package"}"#);
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
            None,
        )
        .await;
    assert_eq!(
        expect_js_output(output, "environment across state turns"),
        r#"{"owner":"updated","second":"second"}"#
    );
    engine.destroy();
}

/// Abandoning a host wait must run JavaScript cleanup before the affine cancellation step finishes.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn abandoned_async_request_cancels_affine_session() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let toolHost = host.clone();
    let engine = newTestJsEngine(host.clone());
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
                    r#"/** Releases shared state when the owning host wait is cancelled. */
                    exports.wait = async function() {
                    globalThis.cancelledCleanupCount = 0;
                    try {
                        __operitSendIntermediateResult(globalThis.__operitCurrentCallId, 'started');
                        return await toolCall('gate', {});
                    } finally {
                        globalThis.cancelledCleanupCount++;
                    }
                };"#
                    .to_string(),
                    "wait".to_string(),
                    testParams(),
                    BTreeMap::new(),
                    Some(callback),
                    true,
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
        tokio::time::timeout(Duration::from_secs(1), toolHost.gatedToolStarted.notified())
            .await
            .expect("host wait must have started before cancellation");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let output = engine
            .worker
            .runtimeHost
            .executeHostJavaScriptRuntimeStateTask(
                engine.worker.stateHandle, operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(),
                Box::new(|state, _| {
                    let state = state.downcast_mut::<JsEngineState>().unwrap();
                    let cleanup = state.runtime.evaluateHostJavaScriptString(
                        "cancellation-cleanup",
                        "String(globalThis.cancelledCleanupCount)",
                    )?;
                    Ok(Box::new((state.pendingScriptExecutions.len(), cleanup)))
                }),
            )
            .unwrap();
        let (pending, cleanup) = *output.downcast::<(usize, String)>().unwrap();
        if pending == 0 {
            assert_eq!(cleanup, "1", "cancelled host wait must run its finally block");
            break;
        }
        assert!(Instant::now() < deadline, "abandoned affine session leaked");
        std::thread::sleep(Duration::from_millis(1));
    }
    finishGatedTool(&host);
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
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
        "compose lexical initialization render result",
    );
    let parsed = raw.clone();

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
            .renderForAssertions(script, &params, &BTreeMap::new(), textResources)
            .await,
        "compose resource snapshot render result",
    );
    let rendered = raw.clone();
    assert_eq!(rendered["tree"]["props"]["label"], "resource-snapshot");
    let actionId = rendered["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .expect("compose snapshot action id");

    let actionRaw = expect_js_output(
        engine
            .actionForAssertions(actionId, None, &params, &BTreeMap::new(), None)
            .await,
        "compose resource snapshot action result",
    );
    let action = actionRaw.clone();
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
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
        "compose render result",
    );
    let parsed = raw.clone();
    let actionId = parsed["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .expect("action id");

    let actionRaw = expect_js_output(
        engine
            .actionForAssertions(actionId, None, &params, &BTreeMap::new(), None)
            .await,
        "compose action result",
    );
    let actionParsed = actionRaw.clone();
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
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
        "compose render result",
    );
    let parsed = raw.clone();
    let actionId = parsed["tree"]["props"]["onCheckedChange"]["__actionId"]
        .as_str()
        .expect("action id")
        .to_string();
    params.insert("state".to_string(), parsed["state"].clone());
    params.insert("memo".to_string(), parsed["memo"].clone());

    let actionRaw = expect_js_output(
        engine
            .actionForAssertions(
                &actionId,
                Some(Value::Bool(true)),
                &params,
                &BTreeMap::new(),
                None,
            )
            .await,
        "compose action result",
    );
    let actionParsed = actionRaw.clone();

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
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
        "compose async toggle render result",
    );
    let rendered = raw.clone();
    let actionId = rendered["tree"]["props"]["onCheckedChange"]["__actionId"]
        .as_str()
        .expect("async toggle action id")
        .to_string();
    params.insert("state".to_string(), rendered["state"].clone());
    params.insert("memo".to_string(), rendered["memo"].clone());

    let actionRaw = expect_js_output(
        engine
            .actionForAssertions(
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
        actionRaw.clone();

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
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
        "compose render result",
    );
    let parsed = raw.clone();
    let actionId = parsed["tree"]["props"]["onLoad"]["__actionId"]
        .as_str()
        .expect("action id");

    let actionRaw = expect_js_output(
        engine
            .actionForAssertions(actionId, None, &params, &BTreeMap::new(), None)
            .await,
        "compose action result",
    );
    let actionParsed = actionRaw.clone();

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
    ).expect("test engine must initialize");
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
        runtime.block_on(engine.renderForAssertionsAsync(
            script,
            params.clone(),
            BTreeMap::new(),
            Arc::new(resources),
        )),
        "planask async render",
    );
    let result: Value = raw.clone();
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
        runtime.block_on(engine.actionForAssertionsAsync(
            action,
            None,
            params,
            BTreeMap::new(),
            Some(Arc::new(move |value| captured.lock().unwrap().push(value))),
        )),
        "planask onLoad",
    );
    let result: Value = raw.clone();
    assert!(
        result["tree"].is_object(),
        "Unexpected action result: {raw}"
    );
    let intermediate = intermediate.lock().unwrap();
    assert!(
        intermediate.is_empty(),
        "Synchronous onLoad must publish its state through the final render only"
    );
    assert_eq!(result["state"]["closed"], true);
    assert_eq!(result["tree"]["props"]["key"], "planask-ready");
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
        runtime.block_on(engine.renderForAssertionsAsync(
            script.to_string(),
            params.clone(),
            BTreeMap::new(),
            Arc::new(BTreeMap::new()),
        )),
        "Compose timer initial render",
    );
    let rendered: Value = renderedRaw.clone();
    let actionId = rendered["tree"]["props"]["onLoad"]["__actionId"]
        .as_str()
        .expect("Compose onLoad action id")
        .to_string();
    let mut actionParams = params;
    actionParams.insert("state".to_string(), rendered["state"].clone());
    actionParams.insert("memo".to_string(), rendered["memo"].clone());
    let intermediate = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let intermediateForCallback = intermediate.clone();
    let finalRaw = expect_js_output(
        runtime.block_on(engine.actionForAssertionsAsync(
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
    let finalResult: Value = finalRaw.clone();
    assert_eq!(finalResult["state"]["count"], 0);
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if intermediate
            .lock()
            .expect("Compose timer intermediate mutex poisoned")
            .iter()
            .any(|raw| {
                raw["tree"]["children"][0]["props"]["text"].as_str() == Some("1")
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
        .renderForAssertions(&script, &params, &BTreeMap::new(), Arc::new(textResources))
        .await;
    let raw = expect_js_output(output, "message_insert compose render");
    let rendered = raw.clone();
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
            .renderForAssertions(&script, &params, &BTreeMap::new(), resources)
            .await,
        "message_insert compose render",
    );
    let rendered = renderedRaw.clone();
    let actionId = find_switch_action_for_text(&rendered["tree"], "额外信息注入")
        .expect("master switch action id");
    let mut actionParams = params.clone();
    actionParams.insert("state".to_string(), rendered["state"].clone());
    actionParams.insert("memo".to_string(), rendered["memo"].clone());
    let actionRaw = expect_js_output(
        engine
            .actionForAssertions(
                &actionId,
                Some(Value::Bool(true)),
                &actionParams,
                &BTreeMap::new(),
                None,
            )
            .await,
        "message_insert master toggle action",
    );
    let action = actionRaw.clone();
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
                surface: 'toolbox',
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
            const localDirectory = ToolPkg.getLocalDataDir();
            const sharedDirectory = ToolPkg.getSpaceDataDir();
            exports.registerToolPkg = function() {
                ToolPkg.registerNavigationEntry({
                    id: 'config-check',
                    surface: 'toolbox',
                    directory: directory,
                    localDirectory: localDirectory,
                    sharedDirectory: sharedDirectory,
                    localAgain: ToolPkg.getLocalDataDir(),
                    aliasDirectory: ToolPkg.getConfigDir('named_alias')
                });
                return true;
            };
        "#;
        let capture = engine
            .execute_toolpkg_main_registration_function_with_text_resources(script, "registerToolPkg", &params, None)
            .expect("registration must resolve paths while its caller holds the manager lock");
        let entry: Value = serde_json::from_str(&capture.navigationEntries[0]).unwrap();
        let root = format!(
            "/app/data/extensions/{}/plugins/configs/first_import",
            scope.as_str()
        );
        assert_eq!(entry["directory"], root);
        assert_eq!(entry["localDirectory"], "/app/data/plugin_data/device/first_import");
        assert_eq!(entry["localAgain"], entry["localDirectory"]);
        assert_eq!(entry["sharedDirectory"], "/app/data/plugin_data/space/first_import");
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

/// Keeps both data paths identical during registration and runtime independently of installation scope.
#[tokio::test(flavor = "current_thread")]
async fn local_data_directory_is_stable_across_registration_and_runtime() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let mut params = testParams();
    params.insert("toolPkgId".into(), Value::String("first_import".into()));
    params.insert("__operit_registration_config_scope".into(), Value::String("space".into()));
    engine.execute_toolpkg_main_registration_function(
        "const local = ToolPkg.getLocalDataDir(); exports.registerToolPkg = function() { return true; };",
        "registerToolPkg", &params,
    ).expect("local data is available before installation");
    let output = engine.execute_script_function(
        "exports.local_data = function() { return {local:ToolPkg.getLocalDataDir(),shared:ToolPkg.getSpaceDataDir()}; };",
        "local_data", &params, &BTreeMap::new(), None, true, None,
    ).await.expect("local runtime directory").expect("local directory output");
    assert_eq!(serde_json::from_str::<Value>(&output).unwrap(), serde_json::json!({"local":"/app/data/plugin_data/device/first_import","shared":"/app/data/plugin_data/space/first_import"}));
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
            engine.worker.stateHandle, operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(),
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
    let output = engine
        .execute_script_function(
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
        };"#,
            "main",
            &params,
            &BTreeMap::from([("CALL_OWNER".to_string(), "original".to_string())]),
            Some(Arc::new(move |value| {
                sender.send(value).unwrap();
            })),
            true,
            None,
        )
        .await;
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
async fn device_tools_hardware_uses_selected_node_and_rejects_failures() {
    let host=Arc::new(TestPluginConfigExecutionHost::default());
    let engine=newTestJsEngine(host.clone());
    let output=engine.execute_script_function(r#"
        exports.ports = async function() {
            if (tools !== Tools) throw Error('different Tools object');
            if (Tools.Edge !== undefined || Tools.Io !== undefined) throw Error('old global hardware APIs remain');
            if (Tools.edge !== undefined || Tools.io !== undefined) throw Error('noncanonical namespace');
            const edge=await Tools.device('edge-one').Edge.execute({target:'edge.status',method:'read'},{text:'ping'});
            const io=await Tools.device('edge-two').Io.execute({port:'gpio',operation:'read'},{pin:2});
            let failure='';
            try { await Tools.device('unauthorized').Edge.execute({target:'edge.status',method:'read'}); }
            catch(error) { failure=String(error.message || error); }
            return {edge:edge.nodeId,io:io.nodeId,failure};
        };
    "#,"ports",&testParams(),&BTreeMap::new(),None,true,None).await.unwrap().unwrap();
    let result:Value=serde_json::from_str(&output).unwrap();
    assert_eq!(result,serde_json::json!({"edge":"edge-one","io":"edge-two","failure":"Edge node access denied"}));
    let calls=host.edgePortCalls.lock().unwrap();
    assert_eq!(calls.len(),3);
    assert_eq!(calls[0]["params"], serde_json::json!({"interface_info":{"target":"edge.status","method":"read"},"args":{"text":"ping"}}));
    assert_eq!(calls[0]["device"]["nodeId"], "edge-one");
    assert_eq!(calls[1]["params"], serde_json::json!({"interface_info":{"port":"gpio","operation":"read"},"args":{"pin":2}}));
    assert_eq!(calls[1]["device"]["nodeId"], "edge-two");
    assert_eq!(calls[2]["params"]["args"], serde_json::json!({}));
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
                    integer: 7, large: 1e16, date: '2020-01-01T00:00:00.000Z',
                    nested: {x:true}, array: [null, null, null, 2],
                    boxed: false, ['__proto__']: {polluted:true},
                    custom: {key: "custom"}
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
            &BTreeMap::new(), None, true, None,
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

/// Rejects unsupported JSON values without preventing subsequent valid or binary calls.
#[tokio::test(flavor = "current_thread")]
async fn structured_tool_calls_recover_from_invalid_parameters_and_preserve_binary_results() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let result = engine
        .execute_script_function(
            r#"exports.main = async function() {
            var failures=[]; var cycle={};cycle.self=cycle;
            for (var p of [cycle,{big:1n},[],{get x(){throw new Error('getter');}},
                {date:new Date()},{boxed:new Boolean(false)},{omit:undefined},
                {array:[undefined]},{array:[,]},{number:NaN},{number:Infinity},
                {custom:{toJSON:function(){return {};}}}]) {
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
            None,
        )
        .await;
    let output: Value =
        serde_json::from_str(&expect_js_output(result, "invalid parameters recovery")).unwrap();
    assert_eq!(
        output,
        serde_json::json!({"failures":[true,true,true,true,true,true,true,true,true,true,true,true],"good":42,"binary":"AQID"})
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

/// Separates strict request snapshots, Promise jobs and complete host round trips.
/// Run explicitly in release mode; timing is diagnostic, never a correctness assertion.
#[tokio::test]
#[ignore = "manual release bridge payload benchmark"]
async fn bridge_payload_benchmark() {
    let iterations: usize = std::env::var("OPERIT_JS_BRIDGE_BENCH_ITERATIONS")
        .expect("set OPERIT_JS_BRIDGE_BENCH_ITERATIONS")
        .parse().expect("positive iteration count");
    assert!(iterations > 0);
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let script = r#"exports.main = async function(params) {
        var data = {rows: []}, result;
        for (var row = 0; row < params.rows; row++)
            data.rows.push({id: row, name: 'role-' + row, enabled: true,
                settings: {prompt: 'x'.repeat(64), tags: ['a', 'b', 'c']}});
        for (var i = 0; i < params.iterations; i++) {
            if (params.mode === 'input') { __operitHostSnapshotJsonObject(data); result = data; }
            else if (params.mode === 'snapshot') result = __operitSnapshotJsonObject(data);
            else if (params.mode === 'promise') result = await Promise.resolve(data);
            else result = await toolCall(params.mode === 'hostOwned' || params.mode === 'hostReadAll' ? 'echo_owned' : 'echo', data);
            if (params.mode === 'hostReadAll') {
                for (var row = 0; row < params.rows; row++) {
                    var value = result.rows[row];
                    if (value.id !== row || value.name !== 'role-' + row || value.enabled !== true ||
                        value.settings.prompt !== 'x'.repeat(64) || value.settings.tags.join(',') !== 'a,b,c')
                        throw new Error('Payload changed in transit at row ' + row);
                }
            }
        }
        for (var row = 0; row < params.rows; row++) {
            var value = result.rows[row];
            if (value.id !== row || value.name !== 'role-' + row || value.enabled !== true ||
                value.settings.prompt !== 'x'.repeat(64) || value.settings.tags.join(',') !== 'a,b,c')
                throw new Error('Payload changed in transit at row ' + row);
        }
        return result.rows.length;
    };"#;
    let rows: Vec<usize> = std::env::var("OPERIT_JS_BRIDGE_BENCH_ROWS").unwrap_or_else(|_| "0,1,32,256".into())
        .split(',').map(|value| value.parse().expect("row count")).collect();
    let modes = std::env::var("OPERIT_JS_BRIDGE_BENCH_MODES").unwrap_or_else(|_| "promise,snapshot,input,host,hostOwned,hostReadAll".into());
    for rows in rows {
        for mode in modes.split(',') {
            let params = BTreeMap::from([
                ("rows".to_owned(), serde_json::json!(rows)),
                ("mode".to_owned(), serde_json::json!(mode)),
                ("iterations".to_owned(), serde_json::json!(iterations)),
            ]);
            let mut times = Vec::new();
            for sample in 0..6 {
                let started = Instant::now();
                let output = expect_js_output(engine.execute_script_function(
                    script, "main", &params, &BTreeMap::new(), None, true, None,
                ).await, "payload benchmark");
                assert_eq!(output, rows.to_string());
                if sample > 0 {
                    times.push(started.elapsed().as_secs_f64() * 1e6 / iterations as f64);
                }
            }
            times.sort_by(f64::total_cmp);
            eprintln!("BRIDGE_PAYLOAD rows={rows} mode={mode} median_us_per_call={:.3} samples_us={times:?}", times[2]);
        }
    }
    engine.destroy();
}

/// Creates an actual immutable SDK execution context without authenticating any owner through JavaScript data.
fn chatExtensionFixtureContext(owner: &str) -> ToolPkgExecutionContext {
    ToolPkgExecutionContext {
        context_key: format!("toolpkg_main:{owner}"),
        container_package_name: owner.to_string(),
        api_version: "2.0.0".to_string(),
        text_resource_host: Arc::new(StaticToolPkgTextResourceHost {
            resources: BTreeMap::new(),
        }),
    }
}

/// Keeps two real engine context identities isolated when tool parameters attempt to forge a namespace.
#[test]
fn chat_extension_owner_ignores_forged_tool_parameters() {
    let contextA = chatExtensionFixtureContext("org.example.owner-a");
    let contextB = chatExtensionFixtureContext("org.example.owner-b");
    let request = JsToolCallRequest {
        tool_type: "Chat".to_string(),
        tool_name: "writeExtension".to_string(),
        device: None,
        parameters: BTreeMap::from([
            (
                "owner".to_string(),
                Value::String(contextB.container_package_name.clone()),
            ),
            (
                "namespace".to_string(),
                Value::String(contextB.container_package_name.clone()),
            ),
            (
                "pluginId".to_string(),
                Value::String(contextB.container_package_name.clone()),
            ),
        ]),
    };
    assert_eq!(
        { contextA.validate().unwrap(); contextA.container_package_name.as_str() },
        "org.example.owner-a"
    );
    assert_eq!(
        { contextB.validate().unwrap(); contextB.container_package_name.as_str() },
        "org.example.owner-b"
    );
    assert_ne!(
        { contextA.validate().unwrap(); contextA.container_package_name.as_str() },
        request.parameters["owner"].as_str().unwrap()
    );
}

/// Rejects missing native execution identity even when JavaScript can supply a nonempty owner string.
#[test]
fn chat_extension_owner_requires_real_execution_context() {
    let mut context = chatExtensionFixtureContext("org.example.owner-a");
    context.context_key.clear();
    assert!(context.validate().is_err());
    let context = chatExtensionFixtureContext(" ");
    assert!(context.validate().is_err());
}

/// Lowered async functions subscribing to one retained service Promise keep distinct call owners.
#[tokio::test(flavor = "current_thread")]
async fn shared_initialization_promise_preserves_each_waiting_call_context() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let first_engine = engine.clone();
    let first = tokio::spawn(first_engine.execute_script_function(
        r#"exports.initialize = function() {
            const owner = globalThis.__operitCurrentCallId;
            globalThis.sharedInitialization = toolCall('gate', {});
            return globalThis.sharedInitialization.then(function() {
                if (globalThis.__operitCurrentCallId !== owner) throw new Error('Shared Promise resumed under a different execution call');
                return getEnv('CALL_OWNER');
            });
        };"#,
        "initialize", &testParams(),
        &BTreeMap::from([("CALL_OWNER".to_string(), "initializer".to_string())]),
        None, true, None,
    ));
    host.gatedToolStarted.notified().await;
    let (sender, mut started) = tokio::sync::mpsc::unbounded_channel();
    let second_engine = engine.clone();
    let second = tokio::spawn(second_engine.execute_script_function(
        r#"exports.useService = function() {
            const owner = globalThis.__operitCurrentCallId;
            sendIntermediateResult('awaiting-initialization');
            return globalThis.sharedInitialization.then(function() { return Promise.resolve().then(function() {
                if (globalThis.__operitCurrentCallId !== owner) throw new Error('Shared Promise resumed under a different execution call');
                return getEnv('CALL_OWNER');
            }); });
        };"#,
        "useService", &testParams(),
        &BTreeMap::from([("CALL_OWNER".to_string(), "consumer".to_string())]),
        Some(Arc::new(move |value| { sender.send(value).unwrap(); })), true, None,
    ));
    started.recv().await.expect("consumer must await initialization before its completion");
    finishGatedTool(&host);
    assert_eq!(expect_js_output(first.await.unwrap(), "shared initialization owner"), "\"initializer\"");
    assert_eq!(expect_js_output(second.await.unwrap(), "shared initialization consumer"), "\"consumer\"");
    assert_eq!(executionSessionCounts(&engine).await, (0, 0));
    engine.destroy();
}

/// A completed asynchronous call must not resurrect its cleaned id for a suspended caller.
///
/// The reported failure was a memory query that resumed after an unrelated async hook had
/// finished: the hook's completion restored the suspended call's id, then the continuation
/// wrapper wrote the hook's now-deleted id back over it.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn completed_async_call_keeps_a_suspended_call_context_alive() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let awaiter_engine = engine.clone();
    let awaiter = tokio::spawn(async move {
        awaiter_engine
            .execute_script_function(
                r#"exports.awaiter = async function() {
                    await new Promise(function(resolve) { setTimeout(resolve, 100); });
                    var callId = String(globalThis.__operitCurrentCallId || '');
                    return JSON.stringify({
                        callId: callId,
                        callStateAvailable: !!(callId && globalThis.__operitGetCallState(callId))
                    });
                };"#,
                "awaiter",
                &testParams(),
                &BTreeMap::new(),
                None,
                true,
                None,
            )
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    engine
        .execute_script_function(
            r#"exports.hook = async function() { return 'hook'; };"#,
            "hook",
            &testParams(),
            &BTreeMap::new(),
            None,
            true,
            None,
        )
        .await
        .expect("independent async call must complete");
    let output = awaiter.await.expect("awaiter task must not panic");
    let encoded = expect_js_output(output, "suspended call context");
    let json = serde_json::from_str::<String>(&encoded).expect("suspended call context string");
    let report: Value =
        serde_json::from_str(&json).expect("suspended call context must be JSON");
    assert_eq!(
        report["callStateAvailable"], true,
        "suspended call must keep a live call state: {report}"
    );
    engine.destroy();
}

/// A hook that finishes while another call is suspended must hand the execution id back
/// to the suspended call instead of clearing it.
#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "current_thread")]
async fn completed_hook_restores_a_concurrently_suspended_call() {
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let (started_sender, mut started) = tokio::sync::mpsc::unbounded_channel();
    let hook_engine = engine.clone();
    let hook = tokio::spawn(async move {
        hook_engine
            .execute_script_function(
                r#"exports.hook = async function() {
                    sendIntermediateResult('hook-started');
                    await new Promise(function(resolve) { setTimeout(resolve, 25); });
                    return 'hook';
                };"#,
                "hook",
                &testParams(),
                &BTreeMap::new(),
                Some(Arc::new(move |value| { started_sender.send(value).unwrap(); })),
                true,
                None,
            )
            .await
    });
    started.recv().await.expect("hook must start before the probe");
    let probe_engine = engine.clone();
    let probe = tokio::spawn(async move {
        probe_engine
            .execute_script_function(
                r#"exports.probe = async function() {
                    var owner = String(globalThis.__operitCurrentCallId || '');
                    await new Promise(function(resolve) { setTimeout(resolve, 100); });
                    var current = String(globalThis.__operitCurrentCallId || '');
                    return JSON.stringify({
                        owner: owner,
                        current: current,
                        sameOwner: current === owner,
                        callStateAvailable: !!(current && globalThis.__operitGetCallState(current))
                    });
                };"#,
                "probe",
                &testParams(),
                &BTreeMap::new(),
                None,
                true,
                None,
            )
            .await
    });
    expect_js_output(
        hook.await.expect("hook task must not panic"),
        "hook completion",
    );
    let encoded = expect_js_output(
        probe.await.expect("probe task must not panic"),
        "suspended call after hook completion",
    );
    let json = serde_json::from_str::<String>(&encoded).expect("probe report string");
    let report: Value = serde_json::from_str(&json).expect("probe report JSON");
    assert_eq!(
        report["callStateAvailable"], true,
        "suspended call must keep a live state: {report}"
    );
    assert_eq!(
        report["sameOwner"], true,
        "suspended call must keep its own execution id: {report}"
    );
    engine.destroy();
}

/// Holds explicitly installed real Host storage for the native JavaScript integration test.
#[derive(Clone)]
struct StorageTestFixture {
    host: Arc<operit_host_native_storage::NativeRuntimeStorageHost>,
    session: operit_store::PluginStorage::PluginStorageSession,
}

/// Routes only the explicitly installed storage fixture through its authenticated session.
#[allow(non_snake_case)]
fn storageTestRequest(host: &TestPluginConfigExecutionHost, request: operit_plugin_sdk::js_sdk::storage::StorageRequest) -> operit_plugin_sdk::js_sdk::JsFuture<Value> {
    let fixture = host.storage.clone();
    Box::pin(async move {
        let fixture = fixture.ok_or_else(|| operit_plugin_sdk::js_sdk::JsHostError::new("Storage fixture is not installed"))?;
        let response = fixture.session.request(fixture.host.clone(), fixture.host, request, "native-js-test")
            .map_err(|error| operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string()))?;
        serde_json::to_value(operit_plugin_sdk::js_sdk::results::ToolResultData::StorageRequestResultData(response.into()))
            .map_err(|error| operit_plugin_sdk::js_sdk::JsHostError::new(error.to_string()))
    })
}

/// Exercises structured callbacks against real SQLite and native JSON preferences with text codecs disabled.
#[tokio::test(flavor = "current_thread")]
async fn storage_structured_promise_roundtrips_real_sqlite_and_record_proxies() {
    let root = std::env::temp_dir().join(format!("operit-js-storage-{}",uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let host=Arc::new(operit_host_native_storage::NativeRuntimeStorageHost::new(root.join("runtime"),root.join("workspaces")));
    let config=operit_store::ExtensionStore::ExtensionStore::localDataPath("storage_test").unwrap();
    let fixture=StorageTestFixture {host,session:Default::default()};
    let execution=TestPluginConfigExecutionHost {storage:Some(fixture),..Default::default()};
    let engine=newTestJsEngine(Arc::new(execution));
    let script=r#"
        exports.storage_roundtrip=async function(params) {
            /** Blocks text codecs only while the storage boundary owns the synchronous call. */
            function withoutTextCodecs(action) {
                const stringify=JSON.stringify,parse=JSON.parse;
                JSON.stringify=function(){throw new Error('Storage unexpectedly encoded JSON text');};
                JSON.parse=function(){throw new Error('Storage unexpectedly decoded JSON text');};
                try {return action();} finally {JSON.stringify=stringify;JSON.parse=parse;}
            }
            const originalRequest=__operitStorageRequest;
            /** Verifies request conversion remains structured while unrelated engine bookkeeping runs normally. */
            __operitStorageRequest=function(request){return withoutTextCodecs(()=>originalRequest(request));};
            const native=__operitNativeCallToolStructured;
            /** Requires a real host-owned Promise, rather than a JS callback identity. */
            __operitNativeCallToolStructured=function(scope,type,name,parameters,callId) {
                const promise=withoutTextCodecs(()=>native(scope,type,name,parameters,callId));
                if (!promise || typeof promise.then !== 'function') throw new Error('Storage requires a host Promise');
                return promise;
            };
            const objects=await Tools.Storage.objects.open({path:params.config+'/objects.sqlite'});
            const records=objects.collection('cards');
            await records.put('one',{nested:{count:1}},{expectedVersion:null});
            const editor=await records.edit('one');editor.value.nested.count=2;await editor.flush();
            const stored=await records.get('one');
            const sql=await Tools.Storage.sqlite.open({path:params.config+'/sql.sqlite'});
            await sql.defineTable({name:'records',primaryKey:'id',columns:[{name:'id',affinity:'text',nullable:false},{name:'count',affinity:'integer',nullable:false},{name:'bytes',affinity:'blob',nullable:false}]});
            await sql.execute('INSERT INTO records VALUES(?,?,?)',['one',9223372036854775807n,new Uint8Array([0,255])]);
            const rows=await sql.query('SELECT count,bytes FROM records');
            const preferencesPath=params.config+'/keys.preferences.json';
            const keys=await Tools.Storage.dataStore.open({path:preferencesPath});
            await keys.edit(p=>{p.set('nullable',null);p.set('a',1);});
            const competingKeys=await Tools.Storage.dataStore.open({path:preferencesPath});
            let conflict=false;
            try {
                await keys.edit(async p=>{
                    p.set('a',2);p.set('b',3);
                    await competingKeys.edit(other=>other.set('concurrent',true));
                });
            } catch(error) {conflict=error.message.startsWith('DataStore edit conflict:');}
            const data=await keys.data();
            await objects.close();await sql.close();await keys.close();await competingKeys.close();
            return {count:stored.value.nested.count,int64:String(rows[0].count),bytes:Array.from(rows[0].bytes),
                conflict,a:data.get('a'),hasB:data.contains('b'),nullable:data.get('nullable'),
                hasNullable:data.contains('nullable'),concurrent:data.get('concurrent')};
        };
    "#;
    let mut params=testParams();params.insert("config".into(),Value::String(config));
    let output=engine.execute_script_function(script,"storage_roundtrip",&params,&BTreeMap::new(),None,true,None).await;
    let output=expect_js_output(output,"real structured storage bridge");
    let value:Value=serde_json::from_str(&output).unwrap();
    assert_eq!(value,serde_json::json!({"count":2,"int64":"9223372036854775807","bytes":[0,255],"conflict":true,"a":1,"hasB":false,"nullable":null,"hasNullable":true,"concurrent":true}));
    engine.destroy(); drop(engine);
    assert_eq!(root.parent().unwrap(),std::env::temp_dir());std::fs::remove_dir_all(root).unwrap();
}

/// Exercises JS-owned state through the native QuickJS structured callback and real script reloads.
#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_retained_state_stays_in_native_js_session() {
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        exports.default = function(ctx) {
            const [count, setCount] = ctx.useState('count', 0);
            const [label] = ctx.useState('label', 'initial');
            const ref = ctx.useRef('owned', { value: 1 });
            const memo = ctx.useMemo('owned', () => ({ value: 7 }), []);
            if (!globalThis.savedRef) globalThis.savedRef = ref;
            const secret = ctx.useRef('never-export', {});
            Object.defineProperty(secret.current, 'poison', {
                configurable: true, enumerable: true,
                get: function() { throw Error('memo must not cross the Host'); }
            });
            return ctx.UI.Text({ text: label + ':' + count,
                onClick: async function() {
                    await Promise.resolve();
                    ref.current.value++;
                    setCount(count + 1);
                    return { sameRef: ref === globalThis.savedRef, ref: ref.current.value, memo: memo.value, count: count + 1 };
                }
            });
        };
    "#;
    let mut params = testParams();
    params.insert(
        "__test_retained_response".into(),
        Value::Bool(true),
    );
    params.insert(
        "executionContextKey".into(),
        Value::String("retained-state-test".into()),
    );
    let resources = Arc::new(BTreeMap::new());
    let first = expect_js_output(
        engine
            .renderForAssertions(script, &params, &BTreeMap::new(), resources.clone())
            .await,
        "initial retained render",
    );
    assert!(first.get("state").is_none() && first.get("memo").is_none());
    let action = assertionNodes(&first)[0]["props"]["onClick"]["__actionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let changed = expect_js_output(
        engine
            .actionForAssertions(
                &action,
                None,
                &params,
                &BTreeMap::new(),
                Some(Arc::new(move |value| {
                    captured.lock().unwrap().push(value);
                })),
            )
            .await,
        "retained async action",
    );
    assert_eq!(changed["actionResult"]["sameRef"], true);
    assert_eq!(changed["actionResult"]["ref"], 2);
    assert!(changed.get("state").is_none() && changed.get("memo").is_none());
    for event in events.lock().unwrap().iter() {
        assert!(event.get("state").is_none() && event.get("memo").is_none());
    }
    let reloaded = expect_js_output(
        engine
            .renderForAssertions(script, &params, &BTreeMap::new(), resources.clone())
            .await,
        "same-context reload",
    );
    assert_eq!(reloaded["update"]["reset"], false);
    let next = expect_js_output(
        engine
            .actionForAssertions(&action, None, &params, &BTreeMap::new(), None)
            .await,
        "action after reload",
    );
    assert_eq!(next["actionResult"]["sameRef"], true);
    assert_eq!(next["actionResult"]["ref"], 3);
    assert_eq!(next["actionResult"]["count"], 2);
    params.insert("__operit_update_inputs".into(), Value::Bool(true));
    params.insert(
        "__operit_input_state".into(),
        serde_json::json!({"label": "host"}),
    );
    let input = expect_js_output(
        engine
            .renderForAssertions(script, &params, &BTreeMap::new(), resources)
            .await,
        "host input update",
    );
    assert!(assertionNodes(&input)
        .iter()
        .any(|node| node["props"]["text"] == "host:2"));
    assert!(input.get("state").is_none() && input.get("memo").is_none());

    // Release the Host worker exactly as the package manager releases its final page lease.
    engine.destroy();
    assert!(engine
        .actionForAssertions(&action, None, &params, &BTreeMap::new(), None)
        .await
        .is_err());
    drop(engine);
    let reopened = newTestToolPkgRegistrationEngine();
    params.remove("__operit_update_inputs");
    params.remove("__operit_input_state");
    let fresh = expect_js_output(
        reopened
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                Arc::new(BTreeMap::new()),
            )
            .await,
        "reopened retained page",
    );
    assert_eq!(fresh["update"]["reset"], true);
    assert_eq!(fresh["update"]["revision"], 1);
    assert!(assertionNodes(&fresh)
        .iter()
        .any(|node| node["props"]["text"] == "initial:0"));
    let fresh_action = assertionNodes(&fresh)[0]["props"]["onClick"]["__actionId"]
        .as_str()
        .unwrap();
    let fresh_result = expect_js_output(
        reopened
            .actionForAssertions(fresh_action, None, &params, &BTreeMap::new(), None)
            .await,
        "action in reopened page",
    );
    assert_eq!(fresh_result["actionResult"]["ref"], 2);
    assert_eq!(fresh_result["actionResult"]["count"], 1);
    assert!(fresh_result.get("state").is_none() && fresh_result.get("memo").is_none());
    reopened.destroy();
}

/// Keeps WebView interface callbacks alive across native retained commits and explicit replacements.
#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_webview_interface_actions_survive_retained_commits() {
    let engine = newTestToolPkgRegistrationEngine();
    let script = r#"
        __operitNativeComposeWebViewControllerCommand = function(command) {
            globalThis.lastWebViewCommand = command;
            return {success: true, data: null};
        };
        exports.default = function(ctx) {
            const [ready, setReady] = ctx.useState('ready', false);
            const controller = ctx.createWebViewController('character-memory-web');
            return ctx.UI.Box({onLoad: function() {
                controller.addJavascriptInterface('CharacterMemoryHost', {currentTheme: () => 'dark'});
                const id = globalThis.lastWebViewCommand.payload.object.currentTheme.__actionId;
                setReady(true);
                return id;
            }}, ready ? ctx.UI.WebView({controller, url:'https://characters.operit.local/'}) : ctx.UI.Text({text:'loading'}));
        };
    "#;
    let mut params = testParams();
    params.insert("__test_retained_response".into(), Value::Bool(true));
    params.insert("executionContextKey".into(), Value::String("webview-actions".into()));
    let first = expect_js_output(engine.renderForAssertions(script, &params, &BTreeMap::new(), Arc::new(BTreeMap::new())).await, "WebView initial render");
    let on_load = assertionNodes(&first).iter().find_map(|node| node["props"]["onLoad"]["__actionId"].as_str()).unwrap();
    let loaded = expect_js_output(engine.actionForAssertions(on_load, None, &params, &BTreeMap::new(), None).await, "WebView initialization");
    let id = loaded["actionResult"].as_str().unwrap();
    for _ in 0..5 {
        let result = expect_js_output(engine.actionForAssertions(id, None, &params, &BTreeMap::new(), None).await, "WebView currentTheme after retained commit");
        assert_eq!(result["actionResult"], "dark");
        assert!(result.get("state").is_none() && result.get("memo").is_none());
    }
    engine.destroy();
}

/// Exercises the bundled character-card onLoad and its retained WebView callback on native QuickJS.
#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_character_card_webview_interface_survives_native_commits() {
    let engine = newTestToolPkgRegistrationEngine();
    let source = std::fs::read_to_string(testRepositoryRoot().join("plugins/packages/buildin/character_cards/dist/ui/main/index.ui.js")).unwrap();
    let script = format!(r#"
        __operitNativeComposeWebViewControllerCommand = function(command) {{
            globalThis.characterWebViewCommand = command;
            return {{success: true, data: null}};
        }};
        ToolPkg.readResource = async function() {{return '/fixture/character-memory.html';}};
        ToolPkg.ipc.on('character-memory.request', function(request, metadata) {{
            if (request.action !== 'snapshot' || metadata.currentRuntime !== 'main')
                throw Error('Unexpected character snapshot request');
            return {{cards: []}};
        }});
        {source}
    "#);
    let mut params = testParams();
    params.insert("__test_retained_response".into(), Value::Bool(true));
    params.insert("executionContextKey".into(), Value::String("character-webview-actions".into()));
    params.insert("theme".into(), serde_json::json!({"brightness":"dark", "colors":{"primary":"#ff102030"}}));
    let first = expect_js_output(engine.renderForAssertions(&script, &params, &BTreeMap::new(), Arc::new(BTreeMap::new())).await, "character initial render");
    let on_load = assertionNodes(&first).iter().find_map(|node| node["props"]["onLoad"]["__actionId"].as_str()).unwrap();
    expect_js_output(engine.actionForAssertions(on_load, None, &params, &BTreeMap::new(), None).await, "character onLoad");
    let raw = engine.execute_script_function(
        "exports.inspect = function() {return globalThis.characterWebViewCommand.payload.object.loadScreen.__actionId;};",
        "inspect", &params, &BTreeMap::new(), None, true, None
    ).await.unwrap().unwrap();
    let id: String = serde_json::from_str(&raw).unwrap();
    for _ in 0..5 {
        let result = expect_js_output(engine.actionForAssertions(&id, Some(serde_json::json!([])), &params, &BTreeMap::new(), None).await, "character loadScreen");
        assert_eq!(result["actionResult"]["theme"]["brightness"], "dark");
        assert_eq!(result["actionResult"]["snapshot"], serde_json::json!({"cards": []}));
        assert_eq!(result["actionResult"]["screen"]["input"]["mode"], "manage");
        assert!(result.get("state").is_none() && result.get("memo").is_none());
    }
    engine.destroy();
}


/// Preserves retained revision order when an asynchronous action overlaps a same-context render.
#[tokio::test(flavor = "current_thread")]
async fn compose_dsl_reload_drains_intermediate_revisions_before_final() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let source = r#"
        exports.default = function(ctx) {
            const [count, setCount] = ctx.useState('count', 0);
            return ctx.UI.Column({onLoad: async function() {
                setCount(count + 1);
                await toolCall('gate', {});
                setCount(count + 2);
                return 'done';
            }}, [ctx.UI.Text({text: String(count)})]);
        };
    "#;
    let mut params = testParams();
    params.insert("__test_retained_response".into(), Value::Bool(true));
    params.insert("executionContextKey".into(), Value::String("revision-order".into()));
    let initial = expect_js_output(engine.renderForAssertionsAsync(source.into(), params.clone(), BTreeMap::new(), Arc::new(BTreeMap::new())).await, "initial revision");
    let action_id = assertionNodes(&initial).iter().find_map(|node| node["props"]["onLoad"]["__actionId"].as_str()).unwrap().to_owned();
    let delivered = Arc::new(Mutex::new(vec![initial["update"]["revision"].as_u64().unwrap()]));
    let captured = delivered.clone();
    let intermediate_ready = Arc::new(tokio::sync::Notify::new());
    let notify_intermediate = intermediate_ready.clone();
    let action = engine.actionForAssertionsAsync(action_id, None, params.clone(), BTreeMap::new(), Some(Arc::new(move |value| {
        captured.lock().unwrap().push(value["update"]["revision"].as_u64().unwrap());
        notify_intermediate.notify_one();
    })));
    tokio::pin!(action);
    tokio::select! {
        _ = host.gatedToolStarted.notified() => {},
        result = &mut action => panic!("gated action completed before release: {result:?}"),
    }
    // Keep the action pending until its real pre-await change has streamed.
    // The post-await state is coalesced into final and need not publish twice.
    tokio::select! {
        _ = intermediate_ready.notified() => {},
        result = &mut action => panic!("gated action completed before its intermediate: {result:?}"),
    }
    let reloaded = expect_js_output(engine.renderForAssertionsAsync(source.into(), params, BTreeMap::new(), Arc::new(BTreeMap::new())).await, "same-context render");
    delivered.lock().unwrap().push(reloaded["update"]["revision"].as_u64().unwrap());
    finishGatedTool(&host);
    let final_response = expect_js_output(action.await, "action final revision");
    assert_eq!(final_response["actionResult"], "done");
    assert!(assertionNodes(&final_response).iter().any(|node| node["props"]["text"] == "2"));
    delivered.lock().unwrap().push(final_response["update"]["revision"].as_u64().unwrap());
    let revisions = delivered.lock().unwrap().clone();
    assert!(revisions.len() >= 4, "both action intermediates and final updates must be exercised: {revisions:?}");
    assert_eq!(revisions, (1..=revisions.len() as u64).collect::<Vec<_>>());
    engine.destroy();
}

/// Package authentication runs once and remains shared by the worker and runtime state.
#[test]
fn toolpkg_engine_binds_its_execution_host_once() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestIpcEngine(host.clone());
    assert_eq!(host.toolPkgContextBindings.load(Ordering::SeqCst), 1);
    engine.destroy();
}

/// Prepared engines authenticate once and their preinstalled IPC callback uses the newly bound owner.
#[tokio::test(flavor = "current_thread")]
async fn prepared_engine_binds_once_and_uses_bound_ipc_host() {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let prepared = super::PreparedJsEngine::new().unwrap();
    assert_eq!(host.toolPkgContextBindings.load(Ordering::SeqCst), 0);
    let engine = prepared.bind(host.clone(), ToolPkgExecutionContext {
        context_key: "prepared-ipc-test".into(), container_package_name: "test.package".into(),
        api_version: "2.0.0".into(),
        text_resource_host: Arc::new(StaticToolPkgTextResourceHost { resources: BTreeMap::new() }),
    }).await.unwrap();
    assert_eq!(host.toolPkgContextBindings.load(Ordering::SeqCst), 1);
    let mut params = testParams();
    params.insert("__operit_ui_package_name".into(), Value::String("test.package".into()));
    params.insert("__operit_execution_context_key".into(), Value::String("prepared-ipc-test".into()));
    params.insert("__operit_toolpkg_runtime_kind".into(), Value::String("ui".into()));
    let output = expect_js_output(engine.execute_script_function(
        "exports.probe = async function() { return await ToolPkg.ipc.call('test.async', {value:41}, {targetRuntime:'main'}); };",
        "probe", &params, &BTreeMap::new(), None, false, None,
    ).await, "prepared engine IPC");
    assert_eq!(serde_json::from_str::<Value>(&output).unwrap()["value"], 42);
    engine.destroy();
}

/// Consecutive acquisitions consume replenished pristine runtimes rather than a previous page's globals.
#[tokio::test(flavor = "current_thread")]
async fn prepared_provider_replenishes_without_reusing_page_state() {
    use operit_plugin_sdk::javascript::JsExecutionProvider as _;
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let provider = crate::javascript::JsExecutionProvider::HostJsExecutionProvider::new();
    for index in 0..5 {
        let engine = provider.acquire_toolpkg_execution_engine(host.clone(), ToolPkgExecutionContext {
            context_key: format!("prepared-page-{index}"), container_package_name: "test.package".into(),
            api_version: "2.0.0".into(),
            text_resource_host: Arc::new(StaticToolPkgTextResourceHost { resources: BTreeMap::new() }),
        }).await.unwrap();
        let output = expect_js_output(engine.execute_script_function(
            "exports.probe = function() { globalThis.pageCounter = (globalThis.pageCounter || 0) + 1; return {counter:globalThis.pageCounter}; };",
            "probe", &testParams(), &BTreeMap::new(), None, false,
        ).await, "fresh prepared page");
        assert_eq!(serde_json::from_str::<Value>(&output).unwrap()["counter"], 1);
        engine.destroy();
    }
    assert_eq!(host.toolPkgContextBindings.load(Ordering::SeqCst), 5);
}

/// An invalid package identity destroys its claimed runtime without consuming a context binding.
#[tokio::test(flavor = "current_thread")]
async fn prepared_engine_rejects_invalid_owner_and_destroys_runtime() {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let prepared = super::PreparedJsEngine::new().unwrap();
    let alive = prepared.engine.as_ref().unwrap().worker.alive.clone();
    let result = prepared.bind(host.clone(), ToolPkgExecutionContext {
        context_key: String::new(), container_package_name: "test.package".into(),
        api_version: "2.0.0".into(),
        text_resource_host: Arc::new(StaticToolPkgTextResourceHost { resources: BTreeMap::new() }),
    }).await;
    assert!(result.is_err());
    assert!(!alive.load(Ordering::Acquire));
    assert_eq!(host.toolPkgContextBindings.load(Ordering::SeqCst), 0);
}

/// Dropping an unused prepared engine destroys its Host state instead of retaining an idle worker.
#[test]
fn unclaimed_prepared_engine_destroys_runtime() {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    let prepared = super::PreparedJsEngine::new().unwrap();
    let alive = prepared.engine.as_ref().unwrap().worker.alive.clone();
    drop(prepared);
    assert!(!alive.load(Ordering::Acquire));
}

/// Preserves catalog errors from explicit discovery and keeps the same Host worker usable.
#[test]
fn catalog_initialization_failure_returns_error_without_killing_host_worker() {
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-engine-tests");
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let reason = "A ToolPkg package with name 'com.operit.daily_life' is already registered";
    *host.toolCatalogError.lock().unwrap() = Some(reason.into());
    let create = || super::JsEngine::new_toolpkg_execution_engine(host.clone(), ToolPkgExecutionContext {
        context_key: "duplicate-catalog-test".into(), container_package_name: "test.package".into(),
        api_version: "2.0.0".into(), text_resource_host: Arc::new(StaticToolPkgTextResourceHost { resources: BTreeMap::new() }),
    });
    let engine = create().expect("Catalog discovery must remain explicit");
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let script = "exports.read = function() { return getToolCatalog(); };";
    let error = runtime.block_on(engine.execute_script_function(
        script, "read", &BTreeMap::new(), &BTreeMap::new(), None, false, None,
    )).expect_err("Catalog errors must reject discovery");
    assert_eq!(error.kind, JsExecutionErrorKind::Runtime);
    assert!(error.message.contains(reason), "{}", error.message);
    *host.toolCatalogError.lock().unwrap() = None;
    let output = expect_js_output(runtime.block_on(engine.execute_script_function(
        script, "read", &BTreeMap::new(), &BTreeMap::new(), None, false, None,
    )), "engine after catalog repair");
    assert_eq!(serde_json::from_str::<Value>(&output).unwrap(), serde_json::json!({"tools": []}));
    engine.destroy();
}

/// A registration's explicit failed result must not be mistaken for a successful capture.
#[test]
fn registration_failure_result_survives_the_ordinary_text_adapter() {
    ensure_test_runtime_root();
    let mut state = newTestJsEngineState(None);
    let result = state.execute_toolpkg_main_registration_function_on_current_thread(
        "exports.registerToolPkg = function() { return {success:false,message:'exact registration failure'}; };",
        "registerToolPkg", &testParams(), None,
    );
    assert_eq!(result.expect_err("failed registration must not publish a capture").message, "exact registration failure");
}

/// Provides trusted parser bytes explicitly for SDK-only tests without an application host.
fn testComposeParserSourceProvider() -> String {
    format!("function() {{ return {}; }}", serde_json::to_string(
        operit_plugin_sdk::toolpkg::ToolPkgComposeDslBridge::readComposeDslParserSource()).unwrap())
}

/// Exercises the real SDK bootstrap in QuickJS, not a Node-only approximation of laziness.
#[test]
fn native_compose_compiler_defers_parser_until_first_source_compilation() {
    let mut runtime = testJavaScriptRuntimeHost().createHostJavaScriptRuntime().unwrap();
    runtime.evaluateHostJavaScriptVoid("observe-parser", r#"
        globalThis.parserInitializations = 0;
        var originalFunction = Function;
        globalThis.Function = function(...args) { parserInitializations++; return originalFunction(...args); };
    "#).unwrap();
    runtime.evaluateHostJavaScriptVoid("compose-bootstrap",
        &operit_plugin_sdk::toolpkg::ToolPkgComposeDslBridge::buildComposeDslCompilerDefinition(&testComposeParserSourceProvider())).unwrap();
    let result = runtime.evaluateHostJavaScriptString("probe-lazy-parser", r#"JSON.stringify((function() {
        var startup = parserInitializations;
        var artifact = '// @operit-compose-artifact 4\nexports.default = ctx => ctx.UI.Text({text:"artifact"});';
        OperitComposeCompiler.prepare(artifact, 'artifact');
        var afterArtifact = parserInitializations;
        OperitComposeCompiler.prepare('exports.default = ctx => ctx.UI.Text({text:"source"});', 'first');
        var afterSource = parserInitializations;
        try { OperitComposeCompiler.prepare('function {', 'bad'); } catch (_) {}
        OperitComposeCompiler.prepare('exports.default = ctx => ctx.UI.Text({text:"changed"});', 'second');
        return [startup, afterArtifact, afterSource, parserInitializations];
    })())"#).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&result).unwrap(), serde_json::json!([0, 0, 1, 1]));
}

/// Keeps interrupted parser initialization terminal without exposing partial exports.
#[test]
fn native_compose_parser_rejects_after_interrupted_initialization() {
    let mut runtime = testJavaScriptRuntimeHost().createHostJavaScriptRuntime().unwrap();
    let armed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal = armed.clone();
    runtime.registerHostJavaScriptFunction("__armParserInterrupt", Arc::new(move |_| {
        signal.store(true, Ordering::SeqCst);
        Ok(Value::Null)
    })).unwrap();
    let sdk = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sdk/src/toolpkg");
    let parser = format!(r#"
        globalThis.parserAttempts = (globalThis.parserAttempts || 0) + 1;
        if (globalThis.parserAttempts == 1) {{
            exports.parse = function() {{ throw Error('Partial parser published'); }};
            __armParserInterrupt();
            while (true) {{}}
        }}
        {}
    "#, std::fs::read_to_string(sdk.join("vendor/acorn.js")).unwrap());
    let loader = std::fs::read_to_string(sdk.join("compose/Parser.js")).unwrap()
        .replace("__OPERIT_COMPOSE_PARSER_SOURCE__", &format!("function() {{ return {}; }}", serde_json::to_string(&parser).unwrap()));
    runtime.evaluateHostJavaScriptVoid("lazy-parser", &loader).unwrap();
    runtime.setHostJavaScriptInterruptHandler(Some(Arc::new(move || armed.load(Ordering::SeqCst)))).unwrap();
    assert!(runtime.evaluateHostJavaScriptVoid("interrupted-parse", "__operitAcorn.parse('let value = 1;', {ecmaVersion:2022});").is_err());
    runtime.setHostJavaScriptInterruptHandler(None).unwrap();
    assert!(runtime.evaluateHostJavaScriptVoid("rejected-parse", "__operitAcorn.parse('let value = 1;', {ecmaVersion:2022});").is_err());
    assert_eq!(runtime.evaluateHostJavaScriptString("one-parser-attempt", "String(parserAttempts)").unwrap(), "1");
    assert_eq!(runtime.evaluateHostJavaScriptString("no-parser-global-leak", "[typeof exports, typeof module, typeof acorn].join(',')").unwrap(), "undefined,undefined,undefined");
}

/// Validates deferred grouped exports in the actual host-owned QuickJS engine.
#[test]
fn native_bootstrap_libraries_preserve_globals_and_initialize_once() {
    let mut state = newTestJsEngineState(None);
    let output = state.evalJavaScriptString(r#"
        var libraryReads = [];
        var originalLibrarySource = __operitNativeReadBootstrapLibrary;
        __operitNativeReadBootstrapLibrary = function(name) {
            libraryReads.push(name);
            return originalLibrarySource(name);
        };
        var initial = Object.getOwnPropertyDescriptor(globalThis, 'UINode');
        var first = UINode;
        var again = UINode;
        var types = [typeof Android, typeof PackageManager, typeof DeviceController,
            typeof OkHttp, typeof OkHttpClient, typeof CryptoJS, typeof Jimp, typeof pako];
        JSON.stringify({lazy: typeof initial.get === 'function', same: first === again,
            reads: libraryReads, types: types});
    "#).unwrap();
    let output: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(output["lazy"], true);
    assert_eq!(output["same"], true);
    assert_eq!(output["reads"], serde_json::json!([
        "assets/js/UINode.js", "assets/js/AndroidUtils.js", "assets/js/OkHttp3.js",
        "assets/js/CryptoJS.js", "assets/js/Jimp.js", "assets/js/pako.js"
    ]));
    assert_eq!(output["types"], serde_json::json!([
        "function", "function", "function", "object", "function", "object", "object", "object"
    ]));
}

/// Compares initialization policies in fresh native contexts without business queries or Flutter.
#[test]
#[ignore = "Run explicitly to measure native bootstrap bytecode reuse"]
fn native_bootstrap_bytecode_probe() {
    use crate::javascript::JsLibraries::{buildEagerLibraryProbeSource, buildLazyLibraryBootstrapScript};
    use operit_plugin_sdk::toolpkg::ToolPkgComposeDslBridge::buildComposeDslContextBridgeDefinition;
    let host = NativeHostJavaScriptRuntimeHost::new();
    let source = crate::javascript::JsLibraries::buildRuntimeBootstrapScript();
    let eager_libraries = buildEagerLibraryProbeSource();
    let host_context = buildComposeDslContextBridgeDefinition();
    assert_eq!(source.matches(&host_context).count(), 1);
    // Use identical current SDK modules, changing only the eager initialization policy.
    let eager = source.replace(&buildLazyLibraryBootstrapScript(), &eager_libraries);
    let mut eager_samples = Vec::new();
    let mut source_samples = Vec::new();
    let mut bytecode_samples = Vec::new();
    for index in 0..22 {
        for (name, script, samples) in [
            ("eager-source", eager.as_str(), &mut eager_samples),
            ("deferred-source", source, &mut source_samples),
            ("shared-bytecode", source, &mut bytecode_samples),
        ] {
            let mut runtime = host.createHostJavaScriptRuntime().unwrap();
            let start = Instant::now();
            if name == "shared-bytecode" { runtime.evaluateHostJavaScriptBootstrap(source).unwrap(); }
            else { runtime.evaluateHostJavaScriptVoid("operit.bootstrap.js", script).unwrap(); }
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            if index >= 2 { samples.push(elapsed); }
            if index == 0 && name == "shared-bytecode" {
                eprintln!("native_bootstrap_bytecode_probe firstCompileAndExecuteMs={elapsed:.3}");
            }
        }
    }
    for (name, bytes, mut samples) in [
        ("eager-source", eager.len(), eager_samples),
        ("deferred-source", source.len(), source_samples),
        ("shared-bytecode", source.len(), bytecode_samples),
    ] {
        samples.sort_by(f64::total_cmp);
        eprintln!("native_bootstrap_bytecode_probe mode={name} sourceBytes={bytes} samples={} p50Ms={:.3} p95Ms={:.3}",
            samples.len(), samples[9], samples[18]);
    }
}


/// Compares only compiler ownership in otherwise identical fresh production page engines.
#[tokio::test(flavor = "current_thread")]
#[ignore = "Set OPERIT_DSL_PROBE_SCRIPT to measure an explicit script without reading user state"]
async fn native_compose_page_load_probe() {
    let path = std::env::var("OPERIT_DSL_PROBE_SCRIPT").expect("Explicit DSL probe script required");
    let source = std::fs::read_to_string(path).unwrap();
    let isolated_compiler = operit_plugin_sdk::toolpkg::ToolPkgComposeDslBridge::buildComposeDslCompilerDefinition(&testComposeParserSourceProvider());
    let mut samples = Vec::new();
    for index in 0..14 {
        for mode in ["page-compiler", "shared-compiler"] {
            testJavaScriptRuntimeHost();
            let bootstrap = Instant::now();
            let engine = super::JsEngine::new_toolpkg_registration_engine().unwrap();
            let bootstrap_ms = bootstrap.elapsed().as_secs_f64() * 1000.0;
            let setup = format!("exports.install = function() {{ {} return true; }};", if mode == "page-compiler" {
                format!("{isolated_compiler}\n__operitNativePrepareComposeModule = function(source, identity) {{ return OperitComposeCompiler.prepare(source, identity); }};")
            } else { String::new() });
            let options = BTreeMap::from([
                ("executionContextKey".into(), Value::String("compiler-comparison".into())),
                ("__operit_package_lang".into(), Value::String("en".into())),
                ("theme".into(), serde_json::json!({"brightness":"dark", "colors":{"primary":"#ff102030"}})),
            ]);
            engine.execute_script_function(&setup, "install", &options, &BTreeMap::new(), None, false, None).await.unwrap();
            let updates = Arc::new(Mutex::new(Vec::<operit_host_api::HostJavaScriptValue>::new()));
            let sink = updates.clone();
            let started = Instant::now();
            let result = engine.execute_compose_dsl_script_stream_async(
                source.clone(), options, BTreeMap::new(), Arc::new(BTreeMap::new()),
                Arc::new(move |value| sink.lock().unwrap().push(value)),
            ).await.unwrap();
            let render_ms = started.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(result, Some(Value::Null));
            let nodes: usize = updates.lock().unwrap().iter().map(|event| {
                let operit_host_api::HostJavaScriptValue::Buffer(bytes) = event else { panic!("Expected Compose buffer") };
                ToolPkgComposeDslProtocol::ComposeBuffer::new(bytes).unwrap().transaction().unwrap().instances.len()
            }).sum();
            assert!(nodes > 0, "Page must publish a real retained tree");
            samples.push(serde_json::json!({"mode":mode,"sample":index,"sourceBytes":source.len(),"bootstrapMs":bootstrap_ms,"renderMs":render_ms,"nodes":nodes}));
            engine.destroy();
        }
    }
    eprintln!("dsl_compiler_comparison {}", serde_json::to_string(&samples).unwrap());
}

/// Verifies shared lowering never shares page state, closures or globals between engines.
#[tokio::test(flavor = "current_thread")]
async fn compose_shared_compilation_keeps_page_heaps_independent() {
    let first = newTestToolPkgRegistrationEngine();
    let second = newTestToolPkgRegistrationEngine();
    let source = r#"exports.default = function(ctx) {
        const [count, setCount] = ctx.useState('count', 0);
        return ctx.UI.Button({label: String(count), onClick: function() { setCount(count + 1); }});
    };"#;
    let options = testParams();
    let one = first.renderForAssertions(source, &options, &BTreeMap::new(), Arc::new(BTreeMap::new())).await.unwrap().unwrap();
    let two = second.renderForAssertions(source, &options, &BTreeMap::new(), Arc::new(BTreeMap::new())).await.unwrap().unwrap();
    assert_eq!(one["tree"]["props"]["label"], "0");
    assert_eq!(two["tree"]["props"]["label"], "0");
    let action = one["tree"]["props"]["onClick"]["__actionId"].as_str().unwrap();
    let changed = first.actionForAssertions(action, None, &options, &BTreeMap::new(), None).await.unwrap().unwrap();
    assert_eq!(changed["tree"]["props"]["label"], "1");
    let action = two["tree"]["props"]["onClick"]["__actionId"].as_str().unwrap();
    let independent = second.actionForAssertions(action, None, &options, &BTreeMap::new(), None).await.unwrap().unwrap();
    assert_eq!(independent["tree"]["props"]["label"], "1");
    first.destroy(); second.destroy();
}

/// Uses the real generated namespaces and native bridge, including overload dispatch.
#[tokio::test(flavor = "current_thread")]
async fn device_tools_cross_native_bridge_without_leaking_target_into_parameters() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let mut params = testParams();
    params.insert("__operit_toolpkg_api_version".into(), Value::String("2.0.0".into()));
    let output = engine.execute_script_function(
        r#"exports.run = async function() {
            return await Promise.all([
                Tools.device('desktop').Files.read('/desktop.txt'),
                Tools.device('phone').Files.read({path:'/phone.txt'}),
                Tools.device('desktop').call('example:echo', {value:'{\"literal\":true}'}),
                Tools.device('phone').catalog()
            ]);
        };"#, "run", &params, &BTreeMap::new(), None, true, None,
    ).await.unwrap().unwrap();
    let values: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(values[0]["device"]["nodeId"], "desktop");
    assert_eq!(values[1]["device"]["nodeId"], "phone");
    assert_eq!(values[0]["parameters"], serde_json::json!({"path":"/desktop.txt"}));
    assert_eq!(values[1]["parameters"], serde_json::json!({"path":"/phone.txt"}));
    assert_eq!(values[2]["tool_name"], "example:echo");
    assert_eq!(values[2]["parameters"]["value"], "{\"literal\":true}");
    assert_eq!(values[3]["device"]["action"], "catalog");
    engine.destroy();
}

/// Runs actual TypeScript decorator output through the concrete native JavaScript engine.
#[tokio::test(flavor = "current_thread")]
async fn implemented_toolpkg_decorators_use_native_call_watch_and_cancellation() {
    ensure_test_runtime_root();
    let host = TestPluginConfigExecutionHost::default();
    let calls = host.routedPluginCalls.clone();
    let engine = newTestJsEngine(Arc::new(host));
    let service = std::fs::read_to_string(testRepositoryRoot().join("tools/tests/fixtures/compiled/toolpkg_route.js")).unwrap();
    let script = format!("{}\n{}", service, r#"
        exports.run = async function() {
            const local = await ExampleService.read('local', 2);
            const remote = await ExampleService.read('remote', 3);
            const values = [];
            for await (const value of ExampleService.values('remote')) { values.push(value); break; }
            const collector = ExampleService.values('local');
            const first = await collector.next();
            const pending = collector.next();
            await Promise.resolve();
            await collector.return();
            const completed = await pending;
            return { local, remote, values, first, completed };
        };
    "#);
    let output = engine.execute_script_function(&script, "run", &testParams(), &BTreeMap::new(), None, true, None).await.unwrap().unwrap();
    let result: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(result["local"]["count"], 2);
    assert_eq!(result["remote"]["count"], 42);
    assert_eq!(result["values"], serde_json::json!(["native-stream-value"]));
    assert_eq!(result["completed"]["done"], true);
    let calls = calls.lock().unwrap();
    assert_eq!(calls[0]["method"], "ExampleService.read");
    assert_eq!(calls[0]["args"]["arguments"], serde_json::json!(["remote", 3]));
    assert_eq!(calls[1]["method"], "ExampleService.values");
    assert_eq!(calls[2]["closed"], "native-route-watch");
    engine.destroy();
}

/// Preserves ordinary async iterable syntax through native transport and actual destination execution.
#[tokio::test(flavor = "current_thread")]
async fn implemented_toolpkg_reverse_stream_uses_native_transport_and_actual_consumer() {
    ensure_test_runtime_root();
    let host=TestPluginConfigExecutionHost::default();let calls=host.routedPluginCalls.clone();
    let engine=newTestJsEngine(Arc::new(host));
    let fixture=std::fs::read_to_string(testRepositoryRoot().join("tools/tests/fixtures/compiled/toolpkg_route.js")).unwrap();
    let script=format!("{}\n{}",fixture,r#"
        exports.run=async function(){
            async function* input(){yield 2;yield 3;}
            await ExampleService.consume('remote',input());
            const id=await __operitInvokeToolPkgRouteLocal({op:'pushOpen',methodName:'ExampleService.consume',bindingKey:'local',arguments:['local',null],inputIndex:1});
            await __operitInvokeToolPkgRouteLocal({op:'pushItem',id,value:4});
            await __operitInvokeToolPkgRouteLocal({op:'pushItem',id,value:5});
            await __operitInvokeToolPkgRouteLocal({op:'pushClose',id});
            return await ExampleService.read('local',0);
        };
    "#);
    let output=engine.execute_script_function(&script,"run",&testParams(),&BTreeMap::new(),None,true,None).await.unwrap().unwrap();
    let value:Value=serde_json::from_str(&output).unwrap();assert_eq!(value["count"],9);
    let calls=calls.lock().unwrap();assert_eq!(calls[0]["reverse"],"ExampleService.consume");
    assert_eq!(calls[0]["args"]["bindingKey"],"remote");assert_eq!(calls[0]["args"]["inputIndex"],1);
    assert_eq!(calls[1]["value"],2);assert_eq!(calls[2]["value"],3);assert_eq!(calls[3]["abort"],false);
    engine.destroy();
}

/// Keeps ordinary asynchronous execution alive until its collector explicitly cancels it.
#[tokio::test(flavor = "current_thread")]
async fn collector_owned_execution_has_no_wall_deadline_and_cancels_cleanly() {
    ensure_test_runtime_root();let engine=newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let mut waiting=engine.worker.execute_script_function_async(
        "exports.wait=async function(){await new Promise(function(){});};".into(),"wait".into(),
        testParams(),BTreeMap::new(),None,true, None,None,None,false,
    );
    tokio::select! {
        result=&mut waiting=>panic!("Collector wait acquired a wall deadline: {result:?}"),
        _=tokio::time::sleep(Duration::from_millis(120))=>{},
    }
    let output=engine.worker.runtimeHost.executeHostJavaScriptRuntimeStateAsyncTask(
        engine.worker.stateHandle, operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(),Box::new(|state,_|Box::pin(async move {
            let state=state.downcast_mut::<JsEngineState>().unwrap();
            Ok(Box::new(state.pendingScriptExecutions.len()==1 && state.pendingScriptExecutions.values().all(|pending|pending.deadlineMillis.is_none())) as operit_host_api::HostJavaScriptRuntimeStateOutput)
        })),
    ).await.unwrap();assert!(*output.downcast::<bool>().unwrap());
    drop(waiting);
    tokio::time::timeout(Duration::from_secs(2),async {
        while executionSessionCounts(&engine).await.0 != 0 {tokio::task::yield_now().await;}
    }).await.expect("Collector cancellation leaked its session");
    engine.destroy();
}


/// Queues continuations without blocking, preserving the original affine owner and FIFO order.
#[test]
fn host_notification_queue_preserves_affinity_and_order() {
    let host = NativeHostJavaScriptRuntimeHost::new();
    let (owner_sender, owner_receiver) = std::sync::mpsc::channel();
    let handle = host.createHostJavaScriptRuntimeState("notification-test", operit_host_api::HostJavaScriptRuntimeStateKind::Execution, Box::new(move || {
        owner_sender.send(std::thread::current().id()).unwrap();
        Ok(Box::new(Vec::<usize>::new()))
    })).unwrap();
    let owner = owner_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    let (started_sender, started_receiver) = std::sync::mpsc::channel();
    let (release_sender, release_receiver) = std::sync::mpsc::channel();
    host.enqueueHostJavaScriptRuntimeStateTask(handle, operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(), Box::new(move |state, _| {
        assert_eq!(std::thread::current().id(), owner);
        started_sender.send(()).unwrap();
        release_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        state.downcast_mut::<Vec<usize>>().unwrap().push(1);
        Ok(Box::new(()))
    })).unwrap();
    started_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    host.enqueueHostJavaScriptRuntimeStateTask(handle, operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(), Box::new(move |state, _| {
        assert_eq!(std::thread::current().id(), owner);
        state.downcast_mut::<Vec<usize>>().unwrap().push(2);
        Ok(Box::new(()))
    })).unwrap();
    release_sender.send(()).unwrap();
    let result = host.executeHostJavaScriptRuntimeStateTask(handle, operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(), Box::new(move |state, _| {
        assert_eq!(std::thread::current().id(), owner);
        Ok(Box::new(state.downcast_ref::<Vec<usize>>().unwrap().clone()))
    })).unwrap();
    assert_eq!(*result.downcast::<Vec<usize>>().unwrap(), [1, 2]);
    host.destroyHostJavaScriptRuntimeState(handle).unwrap();
    assert!(host.enqueueHostJavaScriptRuntimeStateTask(handle, operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(), Box::new(|_, _| Ok(Box::new(())))).is_err());
}


/// Completes an owner-lifetime call when its Host discards the queued start before execution.
#[tokio::test(flavor = "current_thread")]
async fn discarded_queued_start_finishes_without_a_wall_deadline() {
    use operit_host_api::{
        HostJavaScriptRuntime, HostJavaScriptRuntimeStateFactory,
        HostJavaScriptRuntimeStateHandle, HostJavaScriptRuntimeStateTask,
        HostJavaScriptRuntimeStateOutput, HostJavaScriptRuntimeStateAsyncTask,
        HostJavaScriptRuntimeStateOutputFuture,
    };

    struct DiscardQueuedHost(Arc<dyn HostJavaScriptRuntimeHost>);

    impl HostJavaScriptRuntimeHost for DiscardQueuedHost {
        /// Keeps runtime creation on the real native Host.
        fn createHostJavaScriptRuntime(&self) -> HostResult<Box<dyn HostJavaScriptRuntime>> {
            self.0.createHostJavaScriptRuntime()
        }
        /// Keeps state creation on the real native Host.
        fn createHostJavaScriptRuntimeState(&self, name: &str, kind: operit_host_api::HostJavaScriptRuntimeStateKind, factory: HostJavaScriptRuntimeStateFactory) -> HostResult<HostJavaScriptRuntimeStateHandle> {
            self.0.createHostJavaScriptRuntimeState(name, kind, factory)
        }
        /// Preserves synchronous lifecycle operations on the existing state.
        fn executeHostJavaScriptRuntimeStateTask(&self, handle: HostJavaScriptRuntimeStateHandle, timeout: operit_host_api::HostJavaScriptExecutionInterrupt, task: HostJavaScriptRuntimeStateTask) -> HostResult<HostJavaScriptRuntimeStateOutput> {
            self.0.executeHostJavaScriptRuntimeStateTask(handle, timeout, task)
        }
        /// Models destruction after admission but before the owner runs the queued start.
        fn enqueueHostJavaScriptRuntimeStateTask(&self, _: HostJavaScriptRuntimeStateHandle, _: operit_host_api::HostJavaScriptExecutionInterrupt, task: HostJavaScriptRuntimeStateTask) -> HostResult<()> {
            drop(task);
            Ok(())
        }
        /// Preserves real cancellation and cleanup operations.
        fn executeHostJavaScriptRuntimeStateAsyncTask(&self, handle: HostJavaScriptRuntimeStateHandle, timeout: operit_host_api::HostJavaScriptExecutionInterrupt, task: HostJavaScriptRuntimeStateAsyncTask) -> HostJavaScriptRuntimeStateOutputFuture {
            self.0.executeHostJavaScriptRuntimeStateAsyncTask(handle, timeout, task)
        }
        /// Releases the same native state during cleanup.
        fn destroyHostJavaScriptRuntimeState(&self, handle: HostJavaScriptRuntimeStateHandle) -> HostResult<()> {
            self.0.destroyHostJavaScriptRuntimeState(handle)
        }
    }

    let engine = newTestToolPkgRegistrationEngine();
    let mut worker = engine.worker.clone();
    worker.runtimeHost = Arc::new(DiscardQueuedHost(worker.runtimeHost.clone()));
    let result = tokio::time::timeout(Duration::from_secs(2), worker.execute_script_function_async(
        "exports.run = function() { return true; };".into(), "run".into(),
        BTreeMap::new(), BTreeMap::new(), None, false, None,
        None, None, false,
    )).await;
    engine.destroy();
    let error = result.expect("A discarded start must disconnect without a wall-clock timer").unwrap_err();
    assert_eq!(error.kind, JsExecutionErrorKind::WorkerUnavailable);
}

/// Keeps an ordinary await alive when another call in the same runtime exhausts its before-hook deadline.
#[tokio::test(flavor = "current_thread")]
async fn before_hook_timeout_does_not_cancel_an_unrelated_ordinary_await() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let ordinary = engine.execute_script_function(
        "exports.run = async function() { await Tools.System.sleep(200); return 'ordinary-done'; };",
        "run", &testParams(), &BTreeMap::new(), None, true, None,
    );
    let before = engine.execute_script_function_with_timeout_millis(
        "exports.before = async function() { await Tools.System.sleep(500); return 'late'; };",
        "before", &testParams(), &BTreeMap::new(), None, true, 50, None,
    );
    let (ordinary, before) = tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(ordinary, before)
    }).await.expect("Before-hook cancellation must leave ordinary work responsive");
    assert_eq!(before.unwrap_err().kind, JsExecutionErrorKind::Timeout);
    assert_eq!(ordinary.unwrap().as_deref(), Some("\"ordinary-done\""));
    engine.destroy();
}

/// Attributes an interrupted Promise continuation to its own before hook instead of every pending call.
#[tokio::test(flavor = "current_thread")]
async fn before_hook_busy_continuation_preserves_other_pending_calls() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let ordinary = engine.execute_script_function(
        "exports.run = async function() { await Tools.System.sleep(200); return 'survived'; };",
        "run", &testParams(), &BTreeMap::new(), None, true, None,
    );
    let before = engine.execute_script_function_with_timeout_millis(
        "exports.before = async function() { await Tools.System.sleep(5); while (true) {} };",
        "before", &testParams(), &BTreeMap::new(), None, true, 50, None,
    );
    let (ordinary, before) = tokio::time::timeout(Duration::from_secs(3), async {
        tokio::join!(ordinary, before)
    }).await.expect("A busy before hook must not monopolize the runtime after its deadline");
    assert_eq!(before.unwrap_err().kind, JsExecutionErrorKind::Timeout);
    assert_eq!(ordinary.unwrap().as_deref(), Some("\"survived\""));
    engine.destroy();
}

/// Interrupts active ordinary JavaScript only when its waiting owner explicitly cancels it.
#[tokio::test(flavor = "current_thread")]
async fn ordinary_busy_script_can_be_cancelled_by_its_owner() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let mut execution = engine.execute_script_function(
        "exports.run = function() { while (true) {} };", "run", &testParams(),
        &BTreeMap::new(), None, true, None,
    );
    tokio::select! {
        result = &mut execution => panic!("Ordinary execution ended without owner cancellation: {result:?}"),
        _ = tokio::time::sleep(Duration::from_millis(50)) => {},
    }
    drop(execution);
    let next = engine.execute_script_function(
        "exports.next = function() { return 'ready'; };", "next", &testParams(),
        &BTreeMap::new(), None, true, None,
    );
    let result = tokio::time::timeout(Duration::from_secs(3), next).await
        .expect("Explicit cancellation must release the JS owner").unwrap();
    assert_eq!(result.as_deref(), Some("\"ready\""));
    engine.destroy();
}

/// Keeps a timed-out await's rejection cleanup from monopolizing subsequent ordinary calls.
#[tokio::test(flavor = "current_thread")]
async fn before_hook_timeout_interrupts_a_busy_rejection_continuation() {
    ensure_test_runtime_root();
    let engine = newTestJsEngine(Arc::new(TestPluginConfigExecutionHost::default()));
    let before = engine.execute_script_function_with_timeout_millis(
        "exports.before = async function() { try { await Tools.System.sleep(10000); } catch (error) { while (true) {} } };",
        "before", &testParams(), &BTreeMap::new(), None, true, 50, None,
    ).await;
    assert_eq!(before.unwrap_err().kind, JsExecutionErrorKind::Timeout);
    let next = engine.execute_script_function(
        "exports.next = function() { return 'ready'; };", "next", &testParams(),
        &BTreeMap::new(), None, true, None,
    );
    let result = tokio::time::timeout(Duration::from_secs(2), next).await;
    engine.destroy();
    assert_eq!(result.expect("A cancelled before hook must release its rejection continuation").unwrap().as_deref(), Some("\"ready\""));
}
