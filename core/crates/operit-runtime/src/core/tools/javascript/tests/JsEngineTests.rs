use super::JsEngineState;
use crate::data::preferences::EnvPreferences::EnvPreferences;
use operit_host_api::{HostError, HostResult, RuntimeStorageEntry, RuntimeStorageHost};
use operit_store::RuntimeStorageHost::setDefaultRuntimeStorageHost;
use operit_store::RuntimeStorePaths::setDefaultRuntimeStoreRoot;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[allow(non_snake_case)]
fn testParams() -> BTreeMap<String, Value> {
    let mut params = BTreeMap::new();
    params.insert(
        "__operit_package_lang".to_string(),
        Value::String("zh-CN".to_string()),
    );
    params
}

#[derive(Clone, Debug)]
struct TestRuntimeStorageHost {
    root: PathBuf,
}

impl TestRuntimeStorageHost {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn resolve(&self, path: &str) -> HostResult<PathBuf> {
        let path = Path::new(path);
        if path.is_absolute() {
            return Err(HostError::new(format!(
                "Runtime storage path must be relative: {}",
                path.display()
            )));
        }
        let mut resolved = self.root.clone();
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
    fn rootDir(&self) -> Option<PathBuf> {
        Some(self.root.clone())
    }

    fn readBytes(&self, path: &str) -> HostResult<Vec<u8>> {
        Ok(std::fs::read(self.resolve(path)?)?)
    }

    fn writeBytes(&self, path: &str, content: &[u8]) -> HostResult<()> {
        let path = self.resolve(path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }

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

    fn exists(&self, path: &str) -> HostResult<bool> {
        Ok(self.resolve(path)?.exists())
    }

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
                .strip_prefix(&self.root)
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

fn ensure_test_runtime_root() {
    let root = std::env::temp_dir().join("operit-runtime-js-engine-tests");
    std::fs::create_dir_all(&root).expect("test runtime root");
    let host = Arc::new(TestRuntimeStorageHost::new(root.clone()));
    setDefaultRuntimeStoreRoot(root);
    setDefaultRuntimeStorageHost(host);
}

#[test]
fn execute_promise_script_repeatedly_on_same_engine() {
    let mut state = JsEngineState::new(None);
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
        let output = state.executeScriptFunctionOnCurrentThread(
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
            output.as_deref(),
            Some(format!("\"ASYNC_ECHO:same-engine-{index}\"").as_str())
        );
    }
}

#[test]
fn execute_complete_finishes_call_before_return_value() {
    let mut state = JsEngineState::new(None);
    let script = r#"
        exports.complete_first = function(_params) {
            complete("first");
            return "second";
        };
    "#;
    let params = testParams();

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "complete_first",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(output.as_deref(), Some("\"first\""));
}

#[test]
fn execute_function_with_active_module_context() {
    let mut state = JsEngineState::new(None);
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

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "inspect_context",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(output.as_deref(), Some("\"true:true:root-marker\""));
}

#[test]
fn bootstrap_exposes_ui_android_okhttp_api() {
    let mut state = JsEngineState::new(None);
    let script = r#"
        exports.inspect_bootstrap_api = function(_params) {
            return [
                typeof UINode,
                typeof Android,
                typeof Intent,
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

    let output = state.executeScriptFunctionOnCurrentThread(
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
        output.as_deref(),
        Some("\"function:function:function:function:function:function:function:object:object:function:object:object:object:function:function:function:function\"")
    );
}

#[test]
fn toolpkg_ipc_local_call_returns_handler_result() {
    let mut state = JsEngineState::new(None);
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

    let output = state.executeScriptFunctionOnCurrentThread(
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
        output.as_deref(),
        Some("{\"value\":42,\"channel\":\"test.local\",\"runtime\":\"main\"}")
    );
}

#[test]
fn runtime_context_with_context_runs_local_main_runner() {
    let mut state = JsEngineState::new(None);
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

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "context_runner",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(output.as_deref(), Some("{\"value\":42}"));
}

#[test]
fn execute_inline_hook_function_source() {
    let mut state = JsEngineState::new(None);
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

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "__operit_inline_test",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(output.as_deref(), Some("\"inline-root\""));
}

#[test]
fn compose_dsl_action_uses_rendered_runtime() {
    let engine = super::JsEngine::newToolPkgRegistrationEngine();
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
    let raw = engine
        .executeComposeDslScript(script, &params, &BTreeMap::new())
        .expect("compose render result");
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");
    let actionId = parsed["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .expect("action id");

    let actionRaw = engine
        .executeComposeDslAction(actionId, None, &params, &BTreeMap::new(), None)
        .expect("compose action result");
    let actionParsed = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");
    assert_eq!(actionParsed["actionResult"], 1);
}

#[test]
fn compose_dsl_action_updates_runtime_options_state_store() {
    let engine = super::JsEngine::newToolPkgRegistrationEngine();
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
    let raw = engine
        .executeComposeDslScript(script, &params, &BTreeMap::new())
        .expect("compose render result");
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");
    let actionId = parsed["tree"]["props"]["onCheckedChange"]["__actionId"]
        .as_str()
        .expect("action id")
        .to_string();
    params.insert("state".to_string(), parsed["state"].clone());
    params.insert("memo".to_string(), parsed["memo"].clone());

    let actionRaw = engine
        .executeComposeDslAction(
            &actionId,
            Some(Value::Bool(true)),
            &params,
            &BTreeMap::new(),
            None,
        )
        .expect("compose action result");
    let actionParsed = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");

    assert_eq!(actionParsed["state"]["enabled"], true);
    assert_eq!(actionParsed["tree"]["props"]["checked"], true);
}

#[test]
fn compose_dsl_action_can_access_bootstrap_globals() {
    let engine = super::JsEngine::newToolPkgRegistrationEngine();
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
    let raw = engine
        .executeComposeDslScript(script, &params, &BTreeMap::new())
        .expect("compose render result");
    let parsed = serde_json::from_str::<Value>(&raw).expect("compose render json");
    let actionId = parsed["tree"]["props"]["onLoad"]["__actionId"]
        .as_str()
        .expect("action id");

    let actionRaw = engine
        .executeComposeDslAction(actionId, None, &params, &BTreeMap::new(), None)
        .expect("compose action result");
    let actionParsed = serde_json::from_str::<Value>(&actionRaw).expect("compose action json");

    assert_eq!(actionParsed["actionResult"]["readResource"], "function");
    assert_eq!(actionParsed["actionResult"]["icon"], "SportsEsports");
}

#[test]
fn execute_function_from_module_exports() {
    let mut state = JsEngineState::new(None);
    let script = r#"
        module.exports = {
            module_only: function(params) {
                return "module:" + params.text;
            }
        };
    "#;
    let mut params = testParams();
    params.insert("text".to_string(), Value::String("exports".to_string()));

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "module_only",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(output.as_deref(), Some("\"module:exports\""));
}

#[test]
fn register_thinking_guidance_toolpkg_main() {
    let engine = super::JsEngine::newToolPkgRegistrationEngine();
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
        .executeToolPkgMainRegistrationFunction(&script, "registerToolPkg", &params)
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
    let engine = super::JsEngine::newToolPkgRegistrationEngine();
    let repoRoot = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root");
    let scriptPath = repoRoot.join("plugins/packages/external/message_insert/dist/main.js");
    let script = std::fs::read_to_string(&scriptPath).expect("message_insert main.js");
    let distRoot = repoRoot.join("plugins/packages/external/message_insert/dist");
    let mut textResources = BTreeMap::new();
    for entry in std::fs::read_dir(&distRoot).expect("message_insert dist") {
        let entry = entry.expect("message_insert dist entry");
        let path = entry.path();
        if path.is_file() {
            let name = path
                .file_name()
                .expect("dist file name")
                .to_string_lossy()
                .to_string();
            if let Ok(text) = std::fs::read_to_string(&path) {
                textResources.insert(format!("dist/{name}").to_ascii_lowercase(), text);
            }
        }
    }
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
        .executeToolPkgMainRegistrationFunctionWithTextResources(
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
}

#[test]
fn execute_script_can_require_axios_and_uuid() {
    let mut state = JsEngineState::new(None);
    let script = r#"
        exports.inspect_require = function(_params) {
            var axios = require('axios');
            var uuid = require('uuid');
            return typeof axios.get + ":" + typeof axios.post + ":" + uuid.v4().length;
        };
    "#;
    let params = testParams();

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "inspect_require",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(output.as_deref(), Some("\"function:function:36\""));
}

#[test]
fn registration_mode_uses_ui_module_placeholder() {
    let engine = super::JsEngine::newToolPkgRegistrationEngine();
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
        .executeToolPkgMainRegistrationFunction(script, "registerToolPkg", &params)
        .expect("ui registration");

    assert_eq!(capture.uiRoutes.len(), 1);
    let route = serde_json::from_str::<Value>(&capture.uiRoutes[0]).unwrap();
    assert_eq!(route["screen"], "screens/main.ui.js");
}

#[test]
fn native_interface_reads_env_for_call() {
    ensure_test_runtime_root();
    let key = "OPERIT_JS_NATIVE_ENV_TEST";
    std::env::set_var(key, "enabled");
    EnvPreferences::getInstance()
        .setEnv(key, "enabled")
        .expect("set env");
    let mut state = JsEngineState::new(None);
    let script = r#"
        exports.read_env = function(_params) {
            return getEnv("OPERIT_JS_NATIVE_ENV_TEST");
        };
    "#;
    let params = testParams();

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "read_env",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert_eq!(output.as_deref(), Some("\"enabled\""));
    EnvPreferences::getInstance()
        .removeEnv(key)
        .expect("remove env");
    std::env::remove_var(key);
}

#[test]
fn native_interface_resolves_plugin_config_dir() {
    ensure_test_runtime_root();
    let mut state = JsEngineState::new(None);
    let script = r#"
        exports.config_dir = function(_params) {
            return getPluginConfigDir('plugin:name');
        };
    "#;
    let params = testParams();

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "config_dir",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );
    let path = serde_json::from_str::<String>(&output.expect("config dir"))
        .expect("serialized config dir");

    let normalized = path.replace('\\', "/");
    assert!(normalized.contains("/plugins/plugin_name-"));
    assert!(std::path::Path::new(&path).is_dir());
}

#[test]
fn probe_async_function_declaration_inside_iife() {
    let mut state = JsEngineState::new(None);
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

    let output = state.executeScriptFunctionOnCurrentThread(
        script,
        "get_device_info",
        &params,
        &BTreeMap::new(),
        None,
        true,
        60,
        None,
    );

    assert!(output.is_some());
}
