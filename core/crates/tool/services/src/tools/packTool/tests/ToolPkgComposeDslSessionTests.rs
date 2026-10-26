use super::*;
use std::path::Path;
use operit_host_api::HostManager::{setDefaultHostJavaScriptRuntimeHost, setDefaultHostRuntimeTaskSchedulerHost};
use operit_host_native_scheduler::{NativeHostJavaScriptRuntimeHost, NativeHostRuntimeTaskSchedulerHost};
use operit_js_bridge::javascript::JsEngine::JsEngine;

use super::super::ToolPkgComposeDslProgramStore::ToolPkgComposeDslProgramStore;
use super::super::ToolPkgComposeDslProtocol::{ToolPkgComposeDslReference, ToolPkgComposeDslDecodedTransaction};

/// Projects instructions solely for behavior assertions outside production transport and timing.
#[derive(Debug)]
struct ProgramTestView {
    id: String,
    nodeType: String,
    props: BTreeMap<String, Value>,
    children: Vec<String>,
    slots: BTreeMap<String, Vec<String>>,
}

/// Retains the actual transaction alongside an assertion-only native view.
#[derive(Debug)]
struct ProgramTestUpdate {
    transaction: ToolPkgComposeDslDecodedTransaction,
    views: Vec<ProgramTestView>,
}
impl std::ops::Deref for ProgramTestUpdate {
    type Target = ToolPkgComposeDslDecodedTransaction;
    /// Exposes the unchanged transaction metadata for revision and transport assertions.
    fn deref(&self) -> &Self::Target { &self.transaction }
}

/// Keeps command lifecycle metadata exactly as published by the real session.
#[derive(Debug)]
struct ProgramTestEvent { event: ToolPkgComposeDslEvent, update: Option<ProgramTestUpdate> }
impl std::ops::Deref for ProgramTestEvent {
    type Target = ToolPkgComposeDslEvent;
    /// Exposes original command metadata without creating another event path.
    fn deref(&self) -> &Self::Target { &self.event }
}

/// Converts a logical address into a compact assertion identity.
fn reference_id(reference: &ToolPkgComposeDslReference) -> String { format!("{}#{}", reference.instanceId, reference.node) }

/// Reads finite host views after a transaction, never inside measured JS construction.
fn inspect_programs(store: &ToolPkgComposeDslProgramStore) -> ProgramTestUpdate {
    let transaction = store.snapshot().unwrap();
    let mut views = Vec::new();
    for instance in &transaction.instances {
        let program = transaction.programs.iter().find(|program| program.id == instance.programId).unwrap();
        for node in 0..program.nodes.len() {
            let reference = ToolPkgComposeDslReference {instanceId: instance.id.clone(), node: node as u32};
            let view = store.view(&reference).unwrap();
            views.push(ProgramTestView {id: reference_id(&reference), nodeType: view.node_type, props: view.props,
                children: view.children.iter().map(reference_id).collect(),
                slots: view.slots.into_iter().map(|(key, values)| (key, values.iter().map(reference_id).collect())).collect()});
        }
    }
    ProgramTestUpdate {transaction, views}
}

/// Interprets the real protocol only when tests inspect delivered events.
struct ProgramTestStream { stream: ToolPkgComposeDslEventStream, store: ToolPkgComposeDslProgramStore }
impl ProgramTestStream {
    /// Associates an assertion store with one session stream.
    fn new(stream: ToolPkgComposeDslEventStream) -> Self { Self {stream, store: ToolPkgComposeDslProgramStore::default()} }
    /// Projects the currently committed store while retaining the exact incoming transaction.
    fn inspect(&mut self, event: ToolPkgComposeDslEvent) -> ProgramTestEvent {
        let update = event.update.as_ref().map(|transaction| {
            self.store.apply(transaction.clone()).unwrap();
            let mut inspected = inspect_programs(&self.store);
            inspected.transaction = transaction.decode().unwrap();
            inspected
        });
        ProgramTestEvent {event, update}
    }
    /// Receives detached publications through their actual asynchronous stream.
    async fn recv(&mut self) -> Option<ProgramTestEvent> { let event = self.stream.receiver.recv().await?; Some(self.inspect(event)) }
}

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
        ("toolPkgId".into(), Value::String("com.operit.character_cards".into())),
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
fn drain(stream: &mut ProgramTestStream) -> Vec<ProgramTestEvent> {
    let mut events = Vec::new();
    while let Ok(event) = stream.stream.receiver.try_recv() { events.push(stream.inspect(event)); }
    events
}

/// Checks every emitted revision and rejects errors, gaps, duplicates and out-of-order delivery.
fn assert_revisions(events: &[ProgramTestEvent]) {
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
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
    let source = r#"exports.default = function(ctx) {
        const [count, setCount] = ctx.useState('count', 0);
        return ctx.UI.Column({change: async function(next) {
            setCount(next); await Promise.resolve(); return next;
        }}, [ctx.UI.Text({text: String(count)})]);
    };"#;
    session.execute(command("render", Some(source.into()), None, None)).await;
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let action = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views)
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
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let source = std::fs::read_to_string(root.join("plugins/packages/buildin/character_cards/dist/ui/main/index.ui.js")).unwrap();
    let setup = r#"exports.install = function() {
        __operitNativeComposeWebViewControllerCommand = function(command) {
            globalThis.characterControllerCommand = command;
            return {success:true, data:null};
        };
        ToolPkg.readResource = async function() { await Promise.resolve(); return '/fixture/character-memory.html'; };
        ToolPkg.ipc.on('character-memory.request', async function(request, meta) {
            if (request.action !== 'snapshot' || meta.channel !== 'character-memory.request') {
                throw new Error('Unexpected editor fixture IPC request');
            }
            await Promise.resolve();
            return {fixture: 'character-snapshot'};
        });
        return true;
    };"#;
    engine.execute_script_function(setup, "install", &options(), &BTreeMap::new(), None, false, None).await.unwrap();
    let script = source;
    let render_started = std::time::Instant::now();
    session.execute(command("render", Some(script), None, None)).await;
    eprintln!("character_editor_probe phase=cold_script_to_retained_commit elapsedMs={:.3}", render_started.elapsed().as_secs_f64() * 1000.0);
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let on_load = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views)
        .find_map(|node| node.props.get("onLoad").and_then(|value| value["__actionId"].as_str())).unwrap().to_string();
    let load_started = std::time::Instant::now();
    session.execute(command("onLoad", None, Some(on_load), None)).await;
    eprintln!("character_editor_probe phase=onLoad_fixture_resource elapsedMs={:.3}", load_started.elapsed().as_secs_f64() * 1000.0);
    events.extend(drain(&mut stream));
    let methods = engine.execute_script_function(
        "exports.inspect = function() { return globalThis.characterControllerCommand.payload.object; };",
        "inspect", &options(), &BTreeMap::new(), None, false, None,
    ).await.unwrap().unwrap();
    let methods: Value = serde_json::from_str(&methods).unwrap();
    let mut tasks = Vec::new();
    for index in 0..12 {
        let session = session.clone();
        let method = if index % 2 == 0 { "loadScreen" } else { "request" };
        let action = methods[method]["__actionId"].as_str().expect("current editor bridge method must be registered").to_string();
        let payload = if method == "loadScreen" { serde_json::json!([]) } else { serde_json::json!([{"action":"snapshot"}]) };
        let request = command(&format!("web-{index}"), None, Some(action), Some(payload));
        tasks.push(async move { session.execute(request).await; });
    }
    futures_util::future::join_all(tasks).await;
    events.extend(drain(&mut stream));
    assert_revisions(&events);
    for index in 0..12 {
        let result = events.iter().find(|event| event.requestId == format!("web-{index}") && event.phase == "final")
            .and_then(|event| event.actionResult.as_ref()).expect("editor bridge action must return its result");
        let snapshot = if index % 2 == 0 { &result["snapshot"] } else { result };
        assert_eq!(snapshot, &serde_json::json!({"fixture":"character-snapshot"}));
    }
    assert_eq!(events.iter().filter(|event| event.phase == "complete").count(), 14);
    assert_eq!(events.iter().filter(|event| event.phase == "final").count(), 14);
    session.close(); engine.destroy();
}

/// Checks streamed async renders and RPC actions that explicitly request no UI commit.
#[tokio::test(flavor = "current_thread")]
async fn native_async_render_and_no_render_action_keep_final_results_on_the_stream() {
    let (session, engine) = native_session();
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
    let source = r#"exports.default = async function(ctx) {
        await Promise.resolve();
        return ctx.UI.Column({reply: async function(payload) {
            await Promise.resolve(); return payload.value;
        }}, [ctx.UI.Text({text:'ready'})]);
    };"#;
    session.execute(command("render", Some(source.into()), None, None)).await;
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let action = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views)
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
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
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
    let action = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views)
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
            let event = stream.recv().await.expect("session must remain open for its detached timer");
            let changed = event.update.as_ref().is_some_and(|update| update.views.iter().any(|node| node.props.get("text") == Some(&Value::String("1".into()))));
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
    engine.execute_script_function(install, "install", &options(), &BTreeMap::new(), None, false, None).await.unwrap();
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
            let result = engine.execute_script_function(&source, "probe", &options(), &BTreeMap::new(), None, false, None).await.unwrap().unwrap();
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
    let result = engine.execute_script_function(source, "check", &options(), &BTreeMap::new(), None, false, None).await.unwrap().unwrap();
    assert_eq!(serde_json::from_str::<Value>(&result).unwrap(), serde_json::json!([true, true, true, true, true]));
    let first = "exports.value = function() { return 'first'; };";
    let second = "exports.value = function() { return 'other'; };";
    assert_eq!(first.len(), second.len());
    for (script, expected) in [(first, "first"), (second, "other"), (first, "first")] {
        let result = engine.execute_script_function(script, "value", &options(), &BTreeMap::new(), None, false, None).await.unwrap().unwrap();
        assert_eq!(serde_json::from_str::<Value>(&result).unwrap(), Value::String(expected.into()));
    }
    session.close(); engine.destroy();
}

/// Uses real built artifacts through the native factory/cache, with runtime lowering forbidden.
#[tokio::test(flavor = "current_thread")]
async fn native_compose_artifact_loads_without_parser_and_invalidates_changed_source() {
    let (session, engine) = native_session();
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
    let install = r#"exports.install = function() {
        if (typeof __operitAcorn !== 'undefined' || typeof OperitComposeCompiler !== 'undefined') throw Error('Page must not own the compiler');
        return true;
    };"#;
    engine.execute_script_function(install, "install", &options(), &BTreeMap::new(), None, false, None).await.unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let source = std::fs::read_to_string(root.join("plugins/packages/buildin/character_cards/dist/ui/main/index.ui.js")).unwrap();
    assert!(source.starts_with("// @operit-compose-artifact 4\n"));
    session.execute(command("artifact", Some(source), None, None)).await;
    let events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    assert!(events.iter().any(|event| event.update.is_some()));
    let first = "// @operit-compose-artifact 4\nexports.default = ctx => ctx.UI.Text({text:'first'});";
    let other = first.replace("first", "other");
    assert_eq!(first.len(), other.len());
    for (script, expected) in [(first, "first"), (other.as_str(), "other"), (first, "first")] {
        session.execute(command(expected, Some(script.into()), None, None)).await;
        let events = drain(&mut stream);
        assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
        assert!(events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views)
            .any(|node| node.props.get("text") == Some(&Value::from(expected))), "{events:?}");
    }
    session.execute(command("bad-version", Some(first.replace("artifact 4", "artifact 999")), None, None)).await;
    assert!(drain(&mut stream).iter().any(|event| event.error.as_ref().is_some_and(|error| error.contains("Unsupported Compose artifact version"))));
    // Rejection must not poison the engine or replace its valid factory cache.
    session.execute(command("recovery", Some(other), None, None)).await;
    assert!(drain(&mut stream).iter().all(|event| event.error.is_none()));
    session.close(); engine.destroy();
}

/// Measures fresh native page engines separately from source lowering and warm retained refreshes.
#[tokio::test(flavor = "current_thread")]
#[ignore = "Run explicitly to inspect native editor cold-load stages without opening personal data"]
async fn native_character_editor_load_stage_performance_probe() {
    // Source reads and instrumentation are deliberately outside the startup timer.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let source_dir = std::env::var_os("OPERIT_DSL_PROBE_SOURCE_DIR").map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("plugins/packages/buildin/character_cards/dist/ui"));
    let install = r#"exports.install = function() {
        globalThis.__editorProbe = {prepareMs:0, prepareCalls:0, renderMs:0, commitMs:0};
        const prepare = __operitNativePrepareComposeModule;
        /** Measures the shared compiler boundary, including queueing and exact-source reuse. */
        __operitNativePrepareComposeModule = function(source, identity) {
            const start = Date.now(); const result = prepare(source, identity);
            globalThis.__editorProbe.prepareMs += Date.now() - start; globalThis.__editorProbe.prepareCalls++; return result;
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
            const bundle = create(options); const commit = bundle.composition.finish;
            /** Preserves the exact commit and revision while observing elapsed time. */
            bundle.composition.finish = function(tree) {
                const start = Date.now(); const result = commit(tree);
                globalThis.__editorProbe.commitMs += Date.now() - start; return result;
            };
            return bundle;
        };
        return true;
    };"#;
    let inspect = "exports.inspect = function() { return globalThis.__editorProbe; };";
    let mut screens = Vec::new();
    for screen in ["main", "selection", "chat-sidebar"] {
        let source = std::fs::read_to_string(source_dir.join(format!("{screen}/index.ui.js"))).unwrap();
        let precompiled = source.starts_with("// @operit-compose-artifact ");
        let state = if screen == "selection" {
            serde_json::json!({"presentation":{"requestId":"perf-selection", "input":{"mode":"select", "kind":"all", "selected":null,"chatId":null}}})
        } else if screen == "chat-sidebar" {
            serde_json::json!({"input":{"view":"characters"}, "chatSidebar":{"chats":[],"currentChatId":null,"activeStreamingChatIds":[]}})
        } else { serde_json::json!({}) };
        let mut samples = Vec::new();
        for sample in 0..12 {
            let bootstrap = std::time::Instant::now();
            let (session, engine) = native_session();
            let bootstrap_ms = bootstrap.elapsed().as_secs_f64() * 1000.0;
            let mut stream = ProgramTestStream::new(session.updates().unwrap());
            engine.execute_script_function(install, "install", &options(), &BTreeMap::new(), None, false, None).await.unwrap();
            let render_start = std::time::Instant::now();
            let mut initial = command("render", Some(source.clone()), None, None);
            initial.runtimeOptions.insert("state".into(), state.clone());
            session.execute(initial).await;
            let render_ms = render_start.elapsed().as_secs_f64() * 1000.0;
            let events = drain(&mut stream);
            assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
            assert!(events.iter().any(|event| event.phase == "final" && event.update.is_some()));
            let node_count: usize = events.iter().filter_map(|event| event.update.as_ref()).map(|update| update.instances.len()).sum();
            let result = engine.execute_script_function(inspect, "inspect", &options(), &BTreeMap::new(), None, false, None).await.unwrap().unwrap();
            let stages: Value = serde_json::from_str(&result).unwrap();
            let mut warm = Vec::new();
            for refresh_sample in 0..10 {
                let mut refresh = command(&format!("refresh-{refresh_sample}"), Some(source.clone()), None, None);
                refresh.runtimeOptions.insert("state".into(), state.clone());
                refresh.runtimeOptions.insert("__operit_update_inputs".into(), Value::Bool(true));
                let started = std::time::Instant::now();
                session.execute(refresh).await;
                warm.push(started.elapsed().as_secs_f64() * 1000.0);
                assert!(drain(&mut stream).iter().all(|event| event.error.is_none()));
            }
            let result = engine.execute_script_function(inspect, "inspect", &options(), &BTreeMap::new(), None, false, None).await.unwrap().unwrap();
            let after: Value = serde_json::from_str(&result).unwrap();
            assert_eq!(after["prepareCalls"], Value::from(if precompiled { 0 } else { 1 }));
            samples.push(serde_json::json!({"sample":sample,"engineBootstrapMs":bootstrap_ms,"scriptToCommitMs":render_ms,
                "coldTotalMs":bootstrap_ms+render_ms,"instances":node_count,"stages":stages,"warmInputRefreshMs":warm}));
            session.close(); engine.destroy();
        }
        screens.push(serde_json::json!({"screen":screen,"sourceBytes":source.len(),"precompiled":precompiled,"samples":samples}));
    }
    eprintln!("dsl_native_probe {}", serde_json::json!({"scope":"native QuickJS, synthetic initial state; excludes file reads, instrumentation, Flutter and GPU; shared preparation/root/commit timers have 1 ms resolution; first shared compiler miss is distinct from later fresh-page loads",
        "profile":if cfg!(debug_assertions) {"debug"} else {"release"}, "screens":screens}));
}

/// Replays the compiled native sidebar through repeated metadata reorders and input refreshes.
#[tokio::test(flavor = "current_thread")]
async fn native_character_sidebar_reorder_keeps_every_retained_reference_mounted() {
    use crate::tools::packTool::ToolPkgComposeDslProgramStore::ToolPkgComposeDslProgramStore;
    let (session, engine) = native_session();
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let plugin = std::fs::read_to_string(root.join("plugins/packages/buildin/character_cards/dist/ui/chat-sidebar/index.ui.js")).unwrap();
    let setup = r#"exports.install = function() {
        ToolPkg.readResource = async function() { return '/fixture/avatar.png'; };
        Tools.Files.readBinary = async function(path) { return {path:path, size:12, contentBase64:'iVBORw0KGgo='}; };
        let catalogReads = 0;
        ToolPkg.ipc.on('character-sidebar.catalog', async function(request, meta) {
            if (meta.channel !== 'character-sidebar.catalog') throw new Error('Unexpected sidebar IPC channel');
            const chats = request.chatSidebar.chats;
            return {view:'characters', sections:[{id:'card:travel', title:'Travel ' + (++catalogReads), avatarUri:null,
                kind:'card', selection:'card:travel', chats:chats,
                conversationGroups:[{id:'1', name:'Group', pinned:true, displayOrder:0, chats:chats.slice(0,2)}],
                ungroupedChats:chats.slice(2)}]};
        });
        return true;
    };"#;
    engine.execute_script_function(setup, "install", &options(), &BTreeMap::new(), None, false, None).await.unwrap();
    let source = plugin;
    let chats: Vec<Value> = (1..=3).map(|index| serde_json::json!({
        "id":format!("c{index}"), "title":format!("Chat {index}"), "updatedAt":"1791059541109",
        "displayOrder":index, "workspaceId":null, "workspaceName":null, "locked":false, "parentChatId":null,
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
    let mut store = ToolPkgComposeDslProgramStore::default();
    for event in &events { if let Some(update) = &event.update { store.apply_decoded(update.transaction.clone()).unwrap(); } }
    let action = store.view(store.root().unwrap()).unwrap().props["onLoad"]["__actionId"].as_str().unwrap().to_string();
    session.execute(command("sidebar-load", None, Some(action), None)).await;
    let loaded = drain(&mut stream);
    for event in &loaded { assert!(event.error.is_none(), "{event:?}"); if let Some(update) = &event.update { store.apply_decoded(update.transaction.clone()).unwrap(); } }
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
        for event in &updates { assert!(event.error.is_none(), "{event:?}"); if let Some(update) = &event.update { store.apply_decoded(update.transaction.clone()).unwrap(); } }
        events.extend(updates);
        let snapshot = inspect_programs(&store);
        assert!(snapshot.views.iter().any(|record|
            record.props.get("text") == Some(&Value::String(format!("Travel {}", sample + 2)))
        ), "Host input updates must run the sidebar lifecycle without another action command; sample={sample}, texts={:?}",
            snapshot.views.iter().filter_map(|record| record.props.get("text")).collect::<Vec<_>>());
        let nodes: BTreeMap<_, _> = snapshot.views.iter().map(|record| (&record.id, record)).collect();
        for record in nodes.values() {
            for child in record.children.iter().chain(record.slots.values().flatten()) {
                assert!(nodes.contains_key(child), "{} references missing {child}", record.id);
            }
        }
    }
    assert_revisions(&events);
    session.close(); engine.destroy();
}

/// Reproduces a retained Rust cache after Dart restarts and abandons its command producer.
#[tokio::test(flavor = "current_thread")]
async fn native_restart_reopens_closed_cached_session_and_renders() {
    use crate::tools::packTool::RuntimePackageManager::openComposeDslSessionForContext;
    let (old, engine) = native_session();
    let key = ("restarted-dart-page:0".to_string(), "package".to_string());
    let mut cache = BTreeMap::from([(key.clone(), old.clone())]);
    let mut old_updates = ProgramTestStream::new(old.updates().unwrap());
    old.execute(command("render-before-restart", Some(
        "exports.default = ctx => ctx.UI.Text({text: 'before restart'});".into()
    ), None, None)).await;
    assert!(drain(&mut old_updates).iter().any(|event| event.phase == "final"));
    let (mut old_sender, old_commands) = ReverseStream::channel();
    old_sender.close();
    old.submit(old_commands).await.unwrap();
    assert!(old_updates.recv().await.is_none());
    assert!(ToolPkgComposeDslSession::resolve(&old.id()).is_err());
    let id = openComposeDslSessionForContext(&mut cache, key.clone(), || {
        Ok(ToolPkgComposeDslSession::new(Arc::new(engine.clone()), Arc::new(BTreeMap::new())))
    }).unwrap();
    assert_ne!(id, old.id(), "the live Rust manager must not return the closed capability");
    let reopened = ToolPkgComposeDslSession::resolve(&id).unwrap();
    let mut updates = ProgramTestStream::new(reopened.updates().unwrap());
    reopened.execute(command("render-after-restart", Some(
        "exports.default = ctx => ctx.UI.Text({text: 'usable after restart'});".into()
    ), None, None)).await;
    let events = drain(&mut updates);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    assert!(events.iter().any(|event| event.phase == "final"));
    assert!(events.iter().any(|event| event.phase == "complete"));
    let first_commit = events.iter().find_map(|event| event.update.as_ref()).unwrap();
    assert!(first_commit.reset, "a new capability needs a full reset, not the abandoned client's delta");
    assert_eq!(first_commit.revision, 1);
    old.close();
    assert!(ToolPkgComposeDslSession::resolve(&id).is_ok(), "late old cleanup cannot close its replacement");
    reopened.close(); engine.destroy();
}

/// Keeps background avatar state and overlapping picker actions on one contiguous page stream.
#[tokio::test(flavor = "current_thread")]
async fn native_detached_avatar_load_and_concurrent_picker_actions_keep_all_revisions() {
    let (session, engine) = native_session();
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
    let source = r#"exports.default = function(ctx) {
        const [avatar, setAvatar] = ctx.useState('avatar', 'initial');
        const [busy, setBusy] = ctx.useState('busy', false);
        return ctx.UI.Column({
            load: function() {
                void (async function() {
                    for (let i = 0; i < 8; i++) {
                        await new Promise(resolve => setTimeout(resolve, 1));
                        setAvatar('background-' + i);
                    }
                })();
            },
            pick: async function(index) {
                setBusy(true);
                await new Promise(resolve => setTimeout(resolve, 1));
                setAvatar('selected-' + index);
                setBusy(false);
                return index;
            }
        }, [ctx.UI.Text({text: avatar + ':' + busy})]);
    };"#;
    session.execute(command("render", Some(source.into()), None, None)).await;
    let mut events = drain(&mut stream);
    let actions = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views)
        .find(|node| node.nodeType == "Column").unwrap().props.clone();
    session.execute(command("avatar-load", None, Some(actions["load"]["__actionId"].as_str().unwrap().into()), None)).await;
    let mut tasks = Vec::new();
    for index in 0..12 {
        let session = session.clone();
        let request = command(&format!("pick-{index}"), None, Some(actions["pick"]["__actionId"].as_str().unwrap().into()), Some(Value::from(index)));
        tasks.push(async move { session.execute(request).await; });
    }
    futures_util::future::join_all(tasks).await;
    events.extend(drain(&mut stream));
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views)
            .any(|node| node.props.get("text").and_then(Value::as_str).is_some_and(|text| text.starts_with("background-7:"))) {
            events.push(stream.recv().await.expect("avatar owner must retain its stream"));
        }
    }).await.expect("background avatar load must finish");
    assert_revisions(&events);
    let mut store = super::super::ToolPkgComposeDslProgramStore::ToolPkgComposeDslProgramStore::default();
    for event in events { if let Some(update) = event.update { store.apply_decoded(update.transaction).unwrap(); } }
    session.close(); engine.destroy();
}

/// Exercises the shipped native entity editor while its background resources overlap avatar selection.
#[tokio::test(flavor = "current_thread")]
async fn native_character_avatar_picker_keeps_all_retained_revisions() {
    let (session, engine) = native_session();
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(4).unwrap();
    let source = std::fs::read_to_string(root.join("plugins/packages/buildin/character_cards/dist/ui/main/index.ui.js")).unwrap();
    let setup = r#"exports.install = function() {
        __operitNativeComposeWebViewControllerCommand = function(command) {
            if (command.command === 'addJavascriptInterface') globalThis.avatarControllerMethods = command.payload.object;
            return {success:true, data:null};
        };
        const card = {
            id:'avatar-card', name:'Original', description:'', characterSetting:'Setting', openingStatement:'',
            otherContentChat:'', otherContentVoice:'', avatarUri:null, attachedTagIds:[], advancedCustomPrompt:'', marks:'',
            chatModelBindingMode:'FOLLOW_GLOBAL', chatModelId:null, chatProviderId:null, ttsConfigId:null, themeConfigId:null,
            memoryBindingMode:'CHARACTER', sharedMemoryId:null, sharedMemoryMounts:[],
            toolAccessConfig:{enabled:false,allowedBuiltinTools:[],allowedPackages:[],allowedSkills:[],allowedMcpServers:[]},
            createdAt:0, updatedAt:0
        };
        ToolPkg.ipc.on('character-memory.request', async function(request) {
            await new Promise(resolve => setTimeout(resolve, 1));
            if (request.action === 'listThemeChoices') return [];
            if (request.action !== 'snapshot') throw new Error('Unexpected request: ' + request.action);
            return {cards:[card],groups:[],stores:[],tags:[],models:[],ttsConfigs:[],toolCatalog:{builtinTools:[],packages:[],skills:[],mcpServers:[]},active:{CharacterCard:{id:card.id}},memoryPreferences:{userDescriptionEnabled:true,autoUpdateEnabled:true}};
        });
        __operitNativeReadToolPkgResource = async function() { await new Promise(resolve => setTimeout(resolve, 2)); return '/app/default-avatar.png'; };
        __operitNativeGetScopedPluginConfigDir = function() { return '/app/character-config'; };
        __operitNativeCallToolStructured = async function(owner, type, name, params) {
            await new Promise(resolve => setTimeout(resolve, 1));
            if (name === 'get_terminal_info') return {success:true,data:{platform:'linux'}};
            if (name === 'read_file_binary') return {success:true,data:{size:21,contentBase64:'iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB'}};
            if (name === 'make_directory' || name === 'write_file_binary') return {success:true,data:{successful:true}};
            throw new Error('Unexpected avatar tool: ' + name);
        };
        __operitNativeComposeFilePickerCommand = async function() {
            await new Promise(resolve => setTimeout(resolve, 2));
            return {cancelled:false,files:[{path:'/mnt/avatar.png',name:'avatar.png'}]};
        };
        return true;
    };"#;
    engine.execute_script_function(setup, "install", &options(), &BTreeMap::new(), None, false, None).await.unwrap();
    session.execute(command("render", Some(source), None, None)).await;
    let mut events = drain(&mut stream);
    assert!(events.iter().all(|event| event.error.is_none()), "{events:?}");
    let load = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views)
        .find_map(|node| node.props.get("onLoad").and_then(|value| value["__actionId"].as_str())).unwrap().to_string();
    session.execute(command("load", None, Some(load), None)).await;
    events.extend(drain(&mut stream));
    let methods = engine.execute_script_function(
        "exports.inspect = function() { return globalThis.avatarControllerMethods; };",
        "inspect", &options(), &BTreeMap::new(), None, false, None,
    ).await.unwrap().unwrap();
    let methods: Value = serde_json::from_str(&methods).unwrap();
    let open = methods["openEntity"]["__actionId"].as_str().expect("management must register its native editor action");
    session.execute(command("open-editor", None, Some(open.into()), Some(serde_json::json!(["edit","card","avatar-card"])))).await;
    events.extend(drain(&mut stream));
    assert_revisions(&events);
    let load_entity = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views).rev()
        .find(|node| node.nodeType == "Dialog")
        .and_then(|node| node.props.get("onLoad")).and_then(|value| value["__actionId"].as_str()).unwrap().to_string();
    session.execute(command("load-entity", None, Some(load_entity), None)).await;
    events.extend(drain(&mut stream));
    for index in 0..3 {
        assert_revisions(&events);
        let pick = events.iter().filter_map(|event| event.update.as_ref()).flat_map(|update| &update.views).rev()
            .find(|node| node.props.get("key").and_then(Value::as_str) == Some("entity-choose-avatar"))
            .and_then(|node| node.props.get("onClick")).and_then(|value| value["__actionId"].as_str()).unwrap().to_string();
        session.execute(command(&format!("pick-{index}"), None, Some(pick), None)).await;
        events.extend(drain(&mut stream));
    }
    assert_revisions(&events);
    let mut store = super::super::ToolPkgComposeDslProgramStore::ToolPkgComposeDslProgramStore::default();
    for event in events { if let Some(update) = event.update { store.apply_decoded(update.transaction).unwrap(); } }
    let snapshot = inspect_programs(&store);
    assert!(snapshot.views.iter().any(|node| node.props.get("key").and_then(Value::as_str) == Some("entity-edit-avatar") && node.props.get("uri").and_then(Value::as_str).is_some_and(|uri| uri.starts_with("/app/character-config/avatars/"))), "avatar selection must finish");
    session.close(); engine.destroy();
}

/// Publishes an initial effect's state after render completion without a UI action or input refresh.
#[tokio::test(flavor = "current_thread")]
async fn native_initial_effect_publishes_after_render_completion() {
    let (session, engine) = native_session();
    let mut stream = ProgramTestStream::new(session.updates().unwrap());
    let source = r#"exports.default = function(ctx) {
        const [count, setCount] = ctx.useState('count', 0);
        ctx.useEffect('initial-watch', function() {
            const timer = setInterval(function() { setCount(1); clearInterval(timer); }, 50);
            return function() { clearInterval(timer); };
        }, []);
        return ctx.UI.Text({text: String(count)});
    };"#;
    session.execute(command("initial-owner", Some(source.into()), None, None)).await;
    let mut events = drain(&mut stream);
    assert!(events.iter().any(|event| event.phase == "complete"));
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let event = stream.recv().await.expect("the initial effect must keep the session observable");
            assert!(event.error.is_none(), "{event:?}");
            let changed = event.update.as_ref().is_some_and(|update| update.views.iter().any(|node| node.props.get("text") == Some(&Value::String("1".into()))));
            if changed {
                assert_eq!(event.requestId, "initial-owner");
                assert_eq!(event.phase, "intermediate");
            }
            events.push(event);
            if changed { break; }
        }
    }).await.expect("an effect state update must stream without a click");
    assert_revisions(&events);
    session.close(); engine.destroy();
}
