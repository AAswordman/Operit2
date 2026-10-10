use super::*;
use std::path::Path;
use operit_host_api::HostManager::{setDefaultHostJavaScriptRuntimeHost, setDefaultHostRuntimeTaskSchedulerHost};
use operit_host_native_scheduler::{NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost};
use operit_js_bridge::javascript::JsEngine::JsEngine;

/// Installs the actual Host runtime and wraps its engine in the production session adapter.
fn native_session() -> (ToolPkgComposeDslSession, JsEngine) {
    setDefaultHostJavaScriptRuntimeHost(Arc::new(NativeHostJavaScriptRuntimeHost::new()));
    setDefaultHostRuntimeTaskSchedulerHost(Arc::new(NativeHostRuntimeTaskSchedulerHost::new()));
    let engine = JsEngine::new_toolpkg_registration_engine().unwrap();
    (ToolPkgComposeDslSession::new(Arc::new(engine.clone()), Arc::new(BTreeMap::new())), engine)
}

/// Builds the same page-owned options for every independently executing command.
fn options() -> BTreeMap<String, Value> {
    BTreeMap::from([
        ("executionContextKey".into(), Value::String("native-session-order".into())),
        ("__operit_package_lang".into(), Value::String("en".into())),
        ("theme".into(), serde_json::json!({"brightness":"dark", "colors":{"primary":"#ff102030"}})),
    ])
}

/// Constructs the real command rather than bypassing session publication and completion.
fn command(id: &str, script: Option<String>, action: Option<String>, payload: Option<Value>) -> ToolPkgComposeDslCommand {
    ToolPkgComposeDslCommand {
        requestId: id.into(), operation: if script.is_some() { "render" } else { "action" }.into(),
        script, actionId: action, payload, runtimeOptions: options(), envOverrides: BTreeMap::new(),
    }
}

/// Collects emitted commits and completion events without changing their original stream order.
fn drain(stream: &mut ToolPkgComposeDslEventStream) -> Vec<ToolPkgComposeDslEvent> {
    let mut events = Vec::new();
    while let Ok(event) = stream.receiver.try_recv() { events.push(event); }
    events
}

/// Checks every emitted revision and rejects errors, gaps, duplicates and out-of-order delivery.
fn assert_revisions(events: &[ToolPkgComposeDslEvent]) {
    let mut expected = 1;
    for event in events {
        assert!(event.error.is_none(), "{}: {:?}", event.requestId, event.error);
        if let Some(update) = &event.update {
            assert_eq!(update.revision, expected, "{} {}", event.requestId, event.phase);
            expected += 1;
        }
    }
    assert!(expected > 2);
    for event in events.iter().filter(|event| event.phase == "complete") {
        let complete = events.iter().position(|candidate| std::ptr::eq(candidate, event)).unwrap();
        let final_index = events.iter().position(|candidate| candidate.requestId == event.requestId && candidate.phase == "final").unwrap();
        assert!(final_index < complete, "completion must follow its streamed final response");
    }
}

/// Exercises the real JS-to-Rust-to-session path with concurrently finishing UI commands.
#[tokio::test(flavor = "current_thread")]
async fn native_concurrent_actions_publish_every_commit_before_command_completion() {
    let (session, engine) = native_session();
    let mut stream = session.updates().unwrap();
    let source = r#"exports.default = function(ctx) {
        const [count, setCount] = ctx.useState('count', 0);
        return ctx.UI.Column({change: async function(next) {
            setCount(next); await Promise.resolve(); return next;
        }}, [ctx.UI.Text({text: String(count)})]);
    };"#;
    session.execute(command("render", Some(source.into()), None, None)).await;
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let action = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.upserts)
        .find_map(|node| node.props.get("change").and_then(|value| value["__actionId"].as_str())).unwrap().to_string();
    let mut tasks = Vec::new();
    for index in 0..24 {
        let session = session.clone();
        let request = command(&format!("change-{index}"), None, Some(action.clone()), Some(Value::from(index + 1)));
        tasks.push(async move { session.execute(request).await; });
    }
    futures_util::future::join_all(tasks).await;
    events.extend(drain(&mut stream));
    assert_eq!(events.iter().filter(|event| event.phase == "complete").count(), 25);
    assert_eq!(events.iter().filter(|event| event.phase == "final").count(), 25);
    for index in 0..24 {
        let event = events.iter().find(|event| event.requestId == format!("change-{index}") && event.phase == "final").unwrap();
        assert_eq!(event.actionResult, Some(Value::from(index + 1)));
    }
    assert_revisions(&events);
    session.close(); engine.destroy();
}

/// Replays the actual character-card editor's onLoad and concurrent WebView interface callbacks.
#[tokio::test(flavor = "current_thread")]
async fn native_character_editor_webview_actions_preserve_session_revision_order() {
    let (session, engine) = native_session();
    let mut stream = session.updates().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let source = std::fs::read_to_string(root.join("plugins/packages/buildin/character_cards/dist/ui/main/index.ui.js")).unwrap();
    let script = format!(r#"
        NativeInterface.composeWebViewControllerCommand = function(raw) {{
            globalThis.characterControllerCommand = JSON.parse(raw);
            return {{success:true, data:null}};
        }};
        ToolPkg.readResource = async function() {{ await Promise.resolve(); return '/fixture/character-memory.html'; }};
        {source}
    "#);
    let render_started = std::time::Instant::now();
    session.execute(command("render", Some(script), None, None)).await;
    eprintln!("character_editor_probe phase=cold_script_to_retained_commit elapsedMs={:.3}", render_started.elapsed().as_secs_f64() * 1000.0);
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let on_load = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.upserts)
        .find_map(|node| node.props.get("onLoad").and_then(|value| value["__actionId"].as_str())).unwrap().to_string();
    let load_started = std::time::Instant::now();
    session.execute(command("onLoad", None, Some(on_load), None)).await;
    eprintln!("character_editor_probe phase=onLoad_fixture_resource elapsedMs={:.3}", load_started.elapsed().as_secs_f64() * 1000.0);
    events.extend(drain(&mut stream));
    let methods = engine.execute_script_function(
        "exports.inspect = function() { return globalThis.characterControllerCommand.payload.object; };",
        "inspect", &options(), &BTreeMap::new(), None, false, 60, None,
    ).await.unwrap().unwrap();
    let methods: Value = serde_json::from_str(&methods).unwrap();
    let mut tasks = Vec::new();
    for index in 0..12 {
        let session = session.clone();
        let method = if index % 2 == 0 { "currentTheme" } else { "currentScreen" };
        let action = methods[method]["__actionId"].as_str().unwrap().to_string();
        let request = command(&format!("web-{index}"), None, Some(action), Some(serde_json::json!([])));
        tasks.push(async move { session.execute(request).await; });
    }
    futures_util::future::join_all(tasks).await;
    events.extend(drain(&mut stream));
    assert_revisions(&events);
    assert_eq!(events.iter().filter(|event| event.phase == "complete").count(), 14);
    assert_eq!(events.iter().filter(|event| event.phase == "final").count(), 14);
    session.close(); engine.destroy();
}

/// Checks streamed async renders and RPC actions that explicitly request no UI commit.
#[tokio::test(flavor = "current_thread")]
async fn native_async_render_and_no_render_action_keep_final_results_on_the_stream() {
    let (session, engine) = native_session();
    let mut stream = session.updates().unwrap();
    let source = r#"exports.default = async function(ctx) {
        await Promise.resolve();
        return ctx.UI.Column({reply: async function(payload) {
            await Promise.resolve(); return payload.value;
        }}, [ctx.UI.Text({text:'ready'})]);
    };"#;
    session.execute(command("render", Some(source.into()), None, None)).await;
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let action = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.upserts)
        .find_map(|node| node.props.get("reply").and_then(|value| value["__actionId"].as_str())).unwrap().to_string();
    session.execute(command("rpc", None, Some(action.clone()), Some(serde_json::json!({"__no_render":true,"value":"rpc-result"})))).await;
    events.extend(drain(&mut stream));
    let rpc = events.iter().find(|event| event.requestId == "rpc" && event.phase == "final").unwrap();
    assert_eq!(rpc.actionResult, Some(Value::String("rpc-result".into())));
    assert!(events.iter().filter(|event| event.requestId == "rpc").all(|event| event.update.is_none()), "a no-render action must not consume a UI revision");
    session.execute(command("reply", None, Some(action), Some(serde_json::json!({"value":"rendered-result"})))).await;
    events.extend(drain(&mut stream));
    let reply = events.iter().find(|event| event.requestId == "reply" && event.phase == "final").unwrap();
    assert_eq!(reply.actionResult, Some(Value::String("rendered-result".into())));
    let first_reply_commit = events.iter().filter(|event| event.requestId == "reply").find_map(|event| event.update.as_ref()).unwrap();
    assert_eq!(first_reply_commit.revision, 2);
    assert_revisions(&events);
    session.close(); engine.destroy();
}

/// Verifies detached timer commits retain their original sink after a same-context input refresh.
#[tokio::test(flavor = "current_thread")]
async fn native_detached_timer_keeps_commit_order_and_request_owner_after_input_refresh() {
    let (session, engine) = native_session();
    let mut stream = session.updates().unwrap();
    let source = r#"exports.default = function(ctx) {
        const [count, setCount] = ctx.useState('count', 0);
        const timer = ctx.useRef('timer', null);
        return ctx.UI.Column({start: function() {
            timer.current = setInterval(function() {
                setCount(1); clearInterval(timer.current);
            }, 300);
        }}, [ctx.UI.Text({text:String(count)})]);
    };"#;
    session.execute(command("render", Some(source.into()), None, None)).await;
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let action = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.upserts)
        .find_map(|node| node.props.get("start").and_then(|value| value["__actionId"].as_str())).unwrap().to_string();
    session.execute(command("timer-owner", None, Some(action), None)).await;
    events.extend(drain(&mut stream));
    let mut refresh = command("input-refresh", Some(source.into()), None, None);
    refresh.runtimeOptions.insert("__operit_update_inputs".into(), Value::Bool(true));
    session.execute(refresh).await;
    events.extend(drain(&mut stream));
    assert_eq!(events.iter().filter(|event| event.phase == "complete").count(), 3);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let event = stream.receiver.recv().await.expect("session must remain open for its detached timer");
            let changed = event.update.as_ref().is_some_and(|update| update.upserts.iter().any(|node| node.props.get("text") == Some(&Value::String("1".into()))));
            assert!(event.error.is_none(), "{event:?}");
            if changed {
                assert_eq!(event.requestId, "timer-owner", "detached commits must keep the action sink, not the newest render sink");
                assert_eq!(event.phase, "intermediate");
            }
            events.push(event);
            if changed { break; }
        }
    }).await.expect("detached timer must publish its retained state change");
    assert_revisions(&events);
    session.close(); engine.destroy();
}

/// Measures repeated package invocation in native QuickJS while isolating source hashing from function work.
#[tokio::test(flavor = "current_thread")]
#[ignore = "Run explicitly to record performance samples on the local native Host"]
async fn native_package_source_hash_performance_probe() {
    let (session, engine) = native_session();
    let install = r#"exports.install = function() {
        const original = globalThis.__operitHashText;
        globalThis.__probeHashMs = 0;
        globalThis.__probeHashBytes = 0;
        globalThis.__operitHashText = function(value) {
            const start = Date.now(); const result = original(value);
            globalThis.__probeHashMs += Date.now() - start;
            globalThis.__probeHashBytes += value.length;
            return result;
        };
        return true;
    };"#;
    engine.execute_script_function(install, "install", &options(), &BTreeMap::new(), None, false, 60, None).await.unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let main_bytes = std::fs::metadata(root.join("plugins/packages/buildin/character_cards/dist/main.js")).unwrap().len() as usize;
    for bytes in [1024, main_bytes] {
        let body = r#"exports.probe = function() {
            const result = {hashMs: globalThis.__probeHashMs, hashBytes: globalThis.__probeHashBytes};
            globalThis.__probeHashMs = 0; globalThis.__probeHashBytes = 0;
            return result;
        };"#;
        let source = format!("{}{}", body, " ".repeat(bytes - body.len()));
        for sample in 0..6 {
            let started = std::time::Instant::now();
            let result = engine.execute_script_function(&source, "probe", &options(), &BTreeMap::new(), None, false, 60, None).await.unwrap().unwrap();
            eprintln!("package_source_probe bytes={bytes} sample={sample} totalMs={:.3} details={result}", started.elapsed().as_secs_f64() * 1000.0);
        }
    }
    session.close(); engine.destroy();
}

/// Checks cache fingerprint compatibility and same-length source invalidation through the real Host.
#[tokio::test(flavor = "current_thread")]
async fn native_source_fingerprint_preserves_utf16_cache_identity_and_source_changes() {
    let (session, engine) = native_session();
    let source = r#"exports.check = function() {
        const fixtures = ['', 'ASCII', '中文', '😀', 'a'.repeat(20000)];
        return fixtures.map(function(value) {
            let expected = 0;
            for (let i = 0; i < value.length; i++) {
                expected = (((expected << 5) - expected) + value.charCodeAt(i)) | 0;
            }
            return globalThis.__operitHashText(value) === (expected >>> 0).toString(16);
        });
    };"#;
    let result = engine.execute_script_function(source, "check", &options(), &BTreeMap::new(), None, false, 60, None).await.unwrap().unwrap();
    assert_eq!(serde_json::from_str::<Value>(&result).unwrap(), serde_json::json!([true, true, true, true, true]));
    let first = "exports.value = function() { return 'first'; };";
    let second = "exports.value = function() { return 'other'; };";
    assert_eq!(first.len(), second.len());
    for (script, expected) in [(first, "first"), (second, "other"), (first, "first")] {
        let result = engine.execute_script_function(script, "value", &options(), &BTreeMap::new(), None, false, 60, None).await.unwrap().unwrap();
        assert_eq!(serde_json::from_str::<Value>(&result).unwrap(), Value::String(expected.into()));
    }
    session.close(); engine.destroy();
}

/// Splits real editor startup into bootstrap, compiler, JS render and retained-commit measurements.
#[tokio::test(flavor = "current_thread")]
#[ignore = "Run explicitly to inspect native editor cold-load stages without opening personal data"]
async fn native_character_editor_load_stage_performance_probe() {
    let (session, engine) = native_session();
    let mut stream = session.updates().unwrap();
    let install = r#"exports.install = function() {
        globalThis.__editorProbe = {parseMs:0, compileMs:0, compileCalls:0, renderMs:0, commitMs:0};
        const parse = __operitAcorn.parse;
        /** Measures AST parsing separately from the compiler transformation. */
        __operitAcorn.parse = function(source, options) {
            const start = Date.now(); const result = parse(source, options);
            globalThis.__editorProbe.parseMs += Date.now() - start; return result;
        };
        const compile = OperitComposeCompiler.compile;
        /** Measures compilation while preserving the exact compiled output. */
        OperitComposeCompiler.compile = function(source, identity) {
            const start = Date.now(); const result = compile(source, identity);
            globalThis.__editorProbe.compileMs += Date.now() - start; globalThis.__editorProbe.compileCalls++; return result;
        };
        const render = OperitComposeReactive.render;
        /** Measures synchronous execution of the actual editor root function. */
        OperitComposeReactive.render = function(ctx, entry) {
            const start = Date.now(); const result = render(ctx, entry);
            globalThis.__editorProbe.renderMs += Date.now() - start; return result;
        };
        const create = OperitComposeDslRuntime.createContext;
        /** Measures retained commits inside the actual session's context. */
        OperitComposeDslRuntime.createContext = function(options) {
            const bundle = create(options); const commit = bundle.composition.commit;
            /** Preserves the exact commit and revision while observing elapsed time. */
            bundle.composition.commit = function(tree) {
                const start = Date.now(); const result = commit(tree);
                globalThis.__editorProbe.commitMs += Date.now() - start; return result;
            };
            return bundle;
        };
        return true;
    };"#;
    let bootstrap = std::time::Instant::now();
    engine.execute_script_function(install, "install", &options(), &BTreeMap::new(), None, false, 60, None).await.unwrap();
    eprintln!("editor_stage_probe bootstrapMs={:.3}", bootstrap.elapsed().as_secs_f64() * 1000.0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let source = std::fs::read_to_string(root.join("plugins/packages/buildin/character_cards/dist/ui/main/index.ui.js")).unwrap();
    let render_start = std::time::Instant::now();
    session.execute(command("render", Some(source.clone()), None, None)).await;
    let render_ms = render_start.elapsed().as_secs_f64() * 1000.0;
    let events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    assert!(events.iter().any(|event| event.phase == "final" && event.update.is_some()));
    let inspect = "exports.inspect = function() { return globalThis.__editorProbe; };";
    let result = engine.execute_script_function(inspect, "inspect", &options(), &BTreeMap::new(), None, false, 60, None).await.unwrap().unwrap();
    eprintln!("editor_stage_probe scriptToCommitMs={render_ms:.3} stages={result}");
    for sample in 0..3 {
        let mut refresh = command(&format!("refresh-{sample}"), Some(source.clone()), None, None);
        refresh.runtimeOptions.insert("__operit_update_inputs".into(), Value::Bool(true));
        let started = std::time::Instant::now();
        session.execute(refresh).await;
        eprintln!("editor_stage_probe warmInputRefreshMs={:.3}", started.elapsed().as_secs_f64() * 1000.0);
        assert!(drain(&mut stream).iter().all(|event| event.error.is_none()));
    }
    let result = engine.execute_script_function(inspect, "inspect", &options(), &BTreeMap::new(), None, false, 60, None).await.unwrap().unwrap();
    assert_eq!(serde_json::from_str::<Value>(&result).unwrap()["compileCalls"], Value::from(1));
    eprintln!("editor_stage_probe afterRefreshStages={result}");
    session.close(); engine.destroy();
}

/// Replays the compiled native sidebar through repeated metadata reorders and input refreshes.
#[tokio::test(flavor = "current_thread")]
async fn native_character_sidebar_reorder_keeps_every_retained_reference_mounted() {
    use crate::tools::packTool::ToolPkgComposeDslNodeStore::ToolPkgComposeDslNodeStore;
    let (session, engine) = native_session();
    let mut stream = session.updates().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let plugin = std::fs::read_to_string(root.join("plugins/packages/buildin/character_cards/dist/ui/chat-sidebar/index.ui.js")).unwrap();
    let source = format!(r#"
        ToolPkg.readResource = async function() {{ return '/fixture/avatar.png'; }};
        Tools.Files.readBinary = async function(path) {{ return {{path:path, size:12, contentBase64:'iVBORw0KGgo='}}; }};
        ToolPkg.ipc.call = async function(name, request) {{
            if (name !== 'character-sidebar.catalog') throw new Error('Unexpected sidebar IPC: ' + name);
            const chats = request.chatSidebar.chats;
            return {{view:'characters', sections:[{{id:'card:travel', title:'Travel', avatarUri:null,
                kind:'card', selection:'card:travel', chats:chats,
                conversationGroups:[{{id:'1', name:'Group', pinned:true, displayOrder:0, chats:chats.slice(0,2)}}],
                ungroupedChats:chats.slice(2)}}]}};
        }};
        {plugin}
    "#);
    let chats: Vec<Value> = (1..=3).map(|index| serde_json::json!({
        "id":format!("c{index}"), "title":format!("Chat {index}"), "updatedAt":"1791059541109",
        "displayOrder":index, "workspaceId":null, "workspaceName":null, "locked":false,
        "pinned":false, "group":if index < 3 { Some("Group") } else { None },
    })).collect();
    let input = serde_json::json!({"input":{"view":"characters"}, "chatSidebar":{
        "chats":chats, "currentChatId":"c1", "activeStreamingChatIds":[],
    }});
    let mut render = command("sidebar-render", Some(source.clone()), None, None);
    render.runtimeOptions.insert("state".into(), input.clone());
    session.execute(render).await;
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let mut store = ToolPkgComposeDslNodeStore::default();
    for event in &events { if let Some(update) = &event.update { store.apply(update.clone()).unwrap(); } }
    let action = store.root().unwrap().props["onLoad"]["__actionId"].as_str().unwrap().to_string();
    session.execute(command("sidebar-load", None, Some(action), None)).await;
    let loaded = drain(&mut stream);
    for event in &loaded { assert!(event.error.is_none(), "{event:?}"); if let Some(update) = &event.update { store.apply(update.clone()).unwrap(); } }
    events.extend(loaded);
    for sample in 0..12 {
        let mut next = input.clone();
        let chats = next["chatSidebar"]["chats"].as_array_mut().unwrap();
        chats.rotate_left(sample % 3);
        for (index, chat) in chats.iter_mut().enumerate() {
            chat["displayOrder"] = Value::from(3 - index);
            chat["title"] = Value::String(format!("Updated {sample} {}", chat["id"].as_str().unwrap()));
            chat["updatedAt"] = Value::String(format!("new-{sample}"));
            chat["pinned"] = Value::Bool(sample % 2 == 0);
            chat["locked"] = Value::Bool(sample % 3 == 0);
        }
        let mut refresh = command(&format!("sidebar-input-{sample}"), Some(source.clone()), None, None);
        refresh.runtimeOptions.insert("__operit_update_inputs".into(), Value::Bool(true));
        refresh.runtimeOptions.insert("__operit_input_state".into(), next);
        session.execute(refresh).await;
        let updates = drain(&mut stream);
        for event in &updates { assert!(event.error.is_none(), "{event:?}"); if let Some(update) = &event.update { store.apply(update.clone()).unwrap(); } }
        events.extend(updates);
        let action = store.root().unwrap().props["onInputsChanged"]["__actionId"].as_str().unwrap().to_string();
        session.execute(command(&format!("sidebar-refresh-{sample}"), None, Some(action), None)).await;
        let updates = drain(&mut stream);
        for event in &updates { assert!(event.error.is_none(), "{event:?}"); if let Some(update) = &event.update { store.apply(update.clone()).unwrap(); } }
        events.extend(updates);
        let snapshot = store.snapshot().unwrap();
        let nodes: BTreeMap<_, _> = snapshot.upserts.iter().map(|record| (&record.id, record)).collect();
        for record in nodes.values() {
            for child in record.children.iter().chain(record.slots.values().flatten()) {
                assert!(nodes.contains_key(child), "{} references missing {child}", record.id);
            }
        }
    }
    assert_revisions(&events);
    session.close(); engine.destroy();
}
