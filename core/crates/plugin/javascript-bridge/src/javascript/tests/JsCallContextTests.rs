use super::super as engine;
use super::*;

/// Bounds assertions with the timer owned by the configured runtime Host.
async fn with_host_timeout<T>(future: impl std::future::Future<Output = T>) -> T {
    let scheduler = operit_host_api::HostManager::defaultHostRuntimeTaskSchedulerHost();
    tokio::select! {
        result = future => result,
        result = scheduler.waitForHostRuntimeDelay(2_000) => {
            result.expect("Host timeout timer");
            panic!("Runtime assertion did not complete within two seconds");
        }
    }
}

/// Builds an independently owned native context for capability-boundary assertions.
fn native_context(host: Arc<dyn JsExecutionHost>, owner: &str) -> engine::JsCallContext {
    engine::JsCallContext {
        interrupt: operit_host_api::HostJavaScriptExecutionInterrupt::unlimited(),
        executionHost: Some(host),
        intermediateCallback: engine::JsIntermediateCallback::Text(None),
        executionListener: None,
        envOverrides: Arc::new(Mutex::new(BTreeMap::new())),
        environmentOwner: None,
        textResourceSource: engine::JsTextResourceSource::Snapshot(page_resources(owner)),
        ipcTiming: None,
    }
}

/// Activates the production native boundary with an exact call identity.
fn activate(call_id: &str, cancelled: bool) {
    engine::executeHostOperation(
        "__operitNativeActivateCall",
        vec![Value::String(call_id.into()), Value::Bool(cancelled)],
    )
    .unwrap();
}

/// Reads through the real module capability rather than inspecting its thread-local representation.
fn read_cold_module() -> operit_host_api::HostResult<Value> {
    engine::executeHostOperation(
        "__operitNativeReadToolPkgTextResource",
        vec![
            Value::String("test.resource.owner".into()),
            Value::String("dist/cold.js".into()),
        ],
    )
}

/// Activation restores the exact Host and resources, and an unowned identity grants neither capability.
#[test]
fn activation_switches_complete_context_and_clears_unowned_access() {
    engine::clearThreadLocalCallState();
    let first: Arc<dyn JsExecutionHost> = Arc::new(TestPluginConfigExecutionHost::default());
    let second: Arc<dyn JsExecutionHost> = Arc::new(TestPluginConfigExecutionHost::default());
    engine::CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| {
        *contexts.borrow_mut() = BTreeMap::from([
            ("first".into(), native_context(first.clone(), "first")),
            ("second".into(), native_context(second.clone(), "second")),
        ]);
    });
    for (owner, host) in [("first", &first), ("second", &second), ("first", &first)] {
        activate(owner, false);
        assert!(Arc::ptr_eq(&engine::currentExecutionHost().unwrap(), host));
        assert_eq!(
            read_cold_module().unwrap(),
            page_resources(owner)["dist/cold.js"]
        );
    }
    for owner in ["", "missing-call"] {
        activate(owner, false);
        assert!(engine::currentExecutionHost().is_err());
        assert_eq!(
            read_cold_module().unwrap_err().message,
            "Module read requires an active resource owner"
        );
        assert_eq!(
            engine::CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| contexts.borrow().len()),
            2
        );
    }
    activate("second", false);
    activate("retired-call", true);
    assert!(engine::currentExecutionHost().is_err());
    assert!(read_cold_module().is_err());
    assert!(engine::CURRENT_SCRIPT_OWNER.with(|owner| owner
        .borrow()
        .as_ref()
        .unwrap()
        .1
        .shouldInterrupt()));
    engine::clearThreadLocalCallState();
}

/// A nested runtime scope restores its caller's capabilities even after another realm changes the registry.
#[test]
fn nested_execution_scope_restores_its_enclosing_context() {
    engine::clearThreadLocalCallState();
    let state = newTestJsEngineState(None);
    let outer: Arc<dyn JsExecutionHost> = Arc::new(TestPluginConfigExecutionHost::default());
    let inner: Arc<dyn JsExecutionHost> = Arc::new(TestPluginConfigExecutionHost::default());
    engine::CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| {
        contexts
            .borrow_mut()
            .insert("outer".into(), native_context(outer.clone(), "outer"));
    });
    activate("outer", false);
    let scope = engine::ExecutionScope::enter(&state, None);
    engine::CURRENT_ACTIVE_CALL_CONTEXTS.with(|contexts| {
        *contexts.borrow_mut() =
            BTreeMap::from([("inner".into(), native_context(inner.clone(), "inner"))]);
    });
    activate("inner", false);
    assert!(Arc::ptr_eq(
        &engine::currentExecutionHost().unwrap(),
        &inner
    ));
    drop(scope);
    assert!(Arc::ptr_eq(
        &engine::currentExecutionHost().unwrap(),
        &outer
    ));
    assert_eq!(
        read_cold_module().unwrap(),
        page_resources("outer")["dist/cold.js"]
    );
    engine::clearThreadLocalCallState();
}

/// Creates a cold page module whose value identifies the resource snapshot that supplied it.
fn page_resources(owner: &str) -> Arc<BTreeMap<String, String>> {
    Arc::new(BTreeMap::from([(
        "dist/cold.js".into(),
        format!(
            "module.exports = {{ owner: {} }};",
            serde_json::to_string(owner).unwrap()
        ),
    )]))
}

/// Supplies the same page identity to rendering, asynchronous actions and cancellation.
fn page_params() -> BTreeMap<String, Value> {
    let mut params = testParams();
    params.insert(
        "__operit_ui_package_name".into(),
        Value::String("test.resource.owner".into()),
    );
    params.insert(
        "__operit_script_screen".into(),
        Value::String("dist/ui/index.ui.js".into()),
    );
    params.insert(
        "routeInstanceId".into(),
        Value::String("resource-owner-route".into()),
    );
    params.insert("__test_retained_response".into(), Value::Bool(true));
    params
}

/// A suspended action must keep its original resource snapshot when another render updates the page.
#[tokio::test(flavor = "current_thread")]
async fn async_action_reads_its_snapshot_after_overlapping_render() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let params = page_params();
    let script = r#"
        exports.default = function(ctx) {
            return ctx.UI.Button({text: 'Load', onClick: async function() {
                await toolCall('gate', {});
                return require('../cold').owner;
            }});
        };
    "#;
    let initial = expect_js_output(
        engine
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                page_resources("original-action"),
            )
            .await,
        "initial resource page",
    );
    let action_id = initial["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let action =
        engine.actionForAssertionsAsync(action_id, None, params.clone(), BTreeMap::new(), None);
    tokio::pin!(action);
    tokio::select! {
        _ = host.gatedToolStarted.notified() => {},
        result = &mut action => panic!("Action completed before its host wait: {result:?}"),
    }
    expect_js_output(
        engine
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                page_resources("later-render"),
            )
            .await,
        "overlapping resource page",
    );
    finishGatedTool(&host);
    let result = expect_js_output(action.await, "resource read after host completion");
    assert_eq!(result["actionResult"], "original-action");
    assert_eq!(host.toolPkgTextResourceReads.load(Ordering::Relaxed), 0);
    engine.destroy();
}

/// Timer callbacks must restore their page resources before the first module read.
#[tokio::test(flavor = "current_thread")]
async fn timer_action_reads_its_page_resources() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let params = page_params();
    let script = r#"
        exports.default = function(ctx) {
            return ctx.UI.Button({text: 'Load', onClick: function() {
                return new Promise(function(resolve) {
                    setTimeout(function() { resolve(require('../cold').owner); }, 1);
                });
            }});
        };
    "#;
    let initial = expect_js_output(
        engine
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                page_resources("timer-owner"),
            )
            .await,
        "timer resource page",
    );
    let action_id = initial["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .unwrap();
    let result = expect_js_output(
        engine
            .actionForAssertions(action_id, None, &params, &BTreeMap::new(), None)
            .await,
        "resource read inside timer",
    );
    assert_eq!(result["actionResult"], "timer-owner");
    assert_eq!(host.toolPkgTextResourceReads.load(Ordering::Relaxed), 0);
    engine.destroy();
}

/// Theme subscriptions installed by a page effect retain resource ownership across their Host wait.
#[tokio::test(flavor = "current_thread")]
async fn theme_effect_reads_its_page_resources_after_await() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let mut params = page_params();
    params.insert(
        "theme".into(),
        serde_json::json!({"brightness":"light", "colors":{}}),
    );
    let script = r#"
        exports.default = function(ctx) {
            ctx.useEffect('theme-owner', function() {
                return ctx.Theme.subscribe(async function() {
                    await toolCall('gate', {});
                    globalThis.themeResourceOwner = require('../cold').owner;
                });
            }, []);
            return ctx.UI.Text({text: 'Theme page'});
        };
    "#;
    expect_js_output(
        engine
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                page_resources("theme-effect-owner"),
            )
            .await,
        "theme subscription page",
    );
    let action = engine.actionForAssertionsAsync(
        "__operit_theme_changed".into(),
        Some(serde_json::json!({"brightness":"dark", "colors":{}})),
        params,
        BTreeMap::new(),
        None,
    );
    tokio::pin!(action);
    tokio::select! {
        _ = host.gatedToolStarted.notified() => {},
        result = &mut action => panic!("Theme listener completed before its host wait: {result:?}"),
    }
    finishGatedTool(&host);
    expect_js_output(action.await, "theme subscription module read");
    let result = expect_js_output(
        engine
            .execute_script_function(
                "exports.inspect = function() { return globalThis.themeResourceOwner; };",
                "inspect",
                &testParams(),
                &BTreeMap::new(),
                None,
                true,
                None,
            )
            .await,
        "theme subscription owner",
    );
    assert_eq!(result, "\"theme-effect-owner\"");
    assert_eq!(host.toolPkgTextResourceReads.load(Ordering::Relaxed), 0);
    engine.destroy();
}

/// Retained WebView interface actions restore resources before their asynchronous callback resumes.
#[tokio::test(flavor = "current_thread")]
async fn webview_callback_reads_its_page_resources_after_await() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let params = page_params();
    let script = r#"
        __operitNativeComposeWebViewControllerCommand = function(command) {
            globalThis.resourceInterfaceCommand = command;
            return {success: true, data: null};
        };
        exports.default = function(ctx) {
            const controller = ctx.createWebViewController('resource-owner-webview');
            return ctx.UI.Box({onLoad: function() {
                controller.addJavascriptInterface('ResourceHost', {read: async function() {
                    await toolCall('gate', {});
                    return require('../cold').owner;
                }});
                return globalThis.resourceInterfaceCommand.payload.object.read.__actionId;
            }}, ctx.UI.Text({text: 'WebView callback'}));
        };
    "#;
    let initial = expect_js_output(
        engine
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                page_resources("webview-owner"),
            )
            .await,
        "WebView resource page",
    );
    let on_load = initial["tree"]["props"]["onLoad"]["__actionId"]
        .as_str()
        .unwrap();
    let loaded = expect_js_output(
        engine
            .actionForAssertions(on_load, None, &params, &BTreeMap::new(), None)
            .await,
        "WebView resource interface",
    );
    let action = engine.actionForAssertionsAsync(
        loaded["actionResult"].as_str().unwrap().into(),
        None,
        params,
        BTreeMap::new(),
        None,
    );
    tokio::pin!(action);
    tokio::select! {
        _ = host.gatedToolStarted.notified() => {},
        result = &mut action => panic!("WebView callback completed before its host wait: {result:?}"),
    }
    finishGatedTool(&host);
    let result = expect_js_output(action.await, "WebView callback module read");
    assert_eq!(result["actionResult"], "webview-owner");
    assert_eq!(host.toolPkgTextResourceReads.load(Ordering::Relaxed), 0);
    engine.destroy();
}

/// Background reactions retain their package module capability after the primary result is published.
#[tokio::test(flavor = "current_thread")]
async fn detached_callback_reads_its_package_resources() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    testJavaScriptRuntimeHost();
    register_test_runtime_storage("js-context-owner-tests");
    let engine = engine::JsEngine::new_toolpkg_execution_engine(
        host.clone(),
        ToolPkgExecutionContext {
            context_key: "toolpkg_main:test.resource.owner".into(),
            container_package_name: "test.resource.owner".into(),
            api_version: "2.0.0".into(),
            text_resource_host: Arc::new(StaticToolPkgTextResourceHost {
                resources: (*page_resources("detached-owner")).clone(),
            }),
        },
    )
    .unwrap();
    let mut params = page_params();
    params.insert(
        "__operit_script_screen".into(),
        Value::String("dist/main.js".into()),
    );
    let (sender, mut progress) = tokio::sync::mpsc::unbounded_channel();
    let output = engine.execute_script_function(
        r#"exports.start = function() {
            toolCall('gate', {}).then(function() { sendIntermediateResult(require('./cold').owner); });
            return 'primary';
        };"#,
        "start", &params, &BTreeMap::new(),
        Some(Arc::new(move |value| { sender.send(value).unwrap(); })), true, None,
    ).await;
    assert_eq!(
        expect_js_output(output, "detached primary result"),
        "\"primary\""
    );
    host.gatedToolStarted.notified().await;
    assert_eq!(executionSessionCounts(&engine).await, (0, 1));
    expect_js_output(
        engine
            .execute_script_function(
                "exports.other = function() { return 'other'; };",
                "other",
                &testParams(),
                &BTreeMap::new(),
                None,
                true,
                None,
            )
            .await,
        "interleaved ordinary call",
    );
    finishGatedTool(&host);
    assert_eq!(
        with_host_timeout(progress.recv())
            .await
            .expect("Detached module read must complete"),
        "detached-owner"
    );
    assert_eq!(executionSessionCounts(&engine).await, (0, 0));
    assert_eq!(host.toolPkgTextResourceReads.load(Ordering::Relaxed), 0);
    engine.destroy();
}

/// Cancelling an action must execute its finally block with the action's captured page resources.
#[tokio::test(flavor = "current_thread")]
async fn cancelled_action_reads_its_snapshot_during_cleanup() {
    let host = Arc::new(TestPluginConfigExecutionHost::default());
    let engine = newTestJsEngine(host.clone());
    let params = page_params();
    let script = r#"
        exports.default = function(ctx) {
            return ctx.UI.Button({text: 'Load', onClick: async function() {
                globalThis.__operit_call_runtime_ref.onCleanup(function() {
                    globalThis.subscriptionResourceOwner = require('../cold').owner;
                });
                try { await toolCall('gate', {}); }
                finally { globalThis.cleanupResourceOwner = require('../cold').owner; }
            }});
        };
    "#;
    let initial = expect_js_output(
        engine
            .renderForAssertions(
                script,
                &params,
                &BTreeMap::new(),
                page_resources("cancelled-action"),
            )
            .await,
        "cancellation resource page",
    );
    let action_id = initial["tree"]["props"]["onClick"]["__actionId"]
        .as_str()
        .unwrap()
        .to_owned();
    {
        let action =
            engine.actionForAssertionsAsync(action_id, None, params.clone(), BTreeMap::new(), None);
        tokio::pin!(action);
        tokio::select! {
            _ = host.gatedToolStarted.notified() => {},
            result = &mut action => panic!("Action completed before cancellation: {result:?}"),
        }
        expect_js_output(
            engine
                .renderForAssertions(
                    script,
                    &params,
                    &BTreeMap::new(),
                    page_resources("unrelated-render"),
                )
                .await,
            "render before cancellation",
        );
    }
    with_host_timeout(async {
        while executionSessionCounts(&engine).await.0 != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let result = expect_js_output(engine.execute_script_function(
        "exports.inspect = function() { return [globalThis.cleanupResourceOwner, globalThis.subscriptionResourceOwner]; };",
        "inspect", &testParams(), &BTreeMap::new(), None, true, None,
    ).await, "cancelled action cleanup result");
    assert_eq!(result, "[\"cancelled-action\",\"cancelled-action\"]");
    assert_eq!(host.toolPkgTextResourceReads.load(Ordering::Relaxed), 0);
    finishGatedTool(&host);
    engine.destroy();
}
