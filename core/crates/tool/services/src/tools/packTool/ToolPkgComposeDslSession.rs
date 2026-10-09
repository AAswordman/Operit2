use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use operit_host_api::{HostManager::defaultHostRuntimeTaskSchedulerHost, HostRuntimeTaskSchedulerHost};
use operit_plugin_sdk::javascript::JsExecutionEngine;
use operit_util::stream::ReverseStream::ReverseStream;
use operit_util::stream::Stream::{CollectFuture, Stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{mpsc, Notify};

/// Describes one retained node without embedding its descendants.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslNodeRecord {
    pub id: String,
    pub nodeType: String,
    pub props: BTreeMap<String, Value>,
    pub children: Vec<String>,
    pub slots: BTreeMap<String, Vec<String>>,
}

/// Applies one atomic composition commit to the retained Flutter node store.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslNodeUpdate {
    pub reset: bool,
    pub revision: u64,
    pub rootId: String,
    pub upserts: Vec<ToolPkgComposeDslNodeRecord>,
    pub removed: Vec<String>,
}

/// Carries UI commands on the existing automatic ReverseStream proxy.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslCommand {
    pub requestId: String,
    pub operation: String,
    pub script: Option<String>,
    pub actionId: Option<String>,
    pub payload: Option<Value>,
    pub runtimeOptions: BTreeMap<String, Value>,
    pub envOverrides: BTreeMap<String, String>,
}

/// Carries typed updates and request completion on the existing automatic Stream proxy.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct ToolPkgComposeDslEvent {
    pub requestId: String,
    pub phase: String,
    pub update: Option<ToolPkgComposeDslNodeUpdate>,
    pub state: Option<BTreeMap<String, Value>>,
    pub memo: Option<BTreeMap<String, Value>>,
    pub actionResult: Option<Value>,
    pub navigationCommands: Vec<Value>,
    pub error: Option<String>,
}

/// Reads the owned DSL result without parsing a JSON string or a recursive UI tree.
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ComposeResponse {
    update: Option<ToolPkgComposeDslNodeUpdate>,
    state: BTreeMap<String, Value>,
    memo: BTreeMap<String, Value>,
    actionResult: Option<Value>,
    navigationCommands: Vec<Value>,
}

#[derive(Default)]
struct ComposeSnapshots {
    state: Option<BTreeMap<String, Value>>,
    memo: Option<BTreeMap<String, Value>>,
}

struct ComposeSessionState {
    id: String,
    engine: Arc<dyn JsExecutionEngine>,
    resources: Arc<BTreeMap<String, String>>,
    sender: Mutex<Option<mpsc::UnboundedSender<ToolPkgComposeDslEvent>>>,
    receiver: Mutex<Option<mpsc::UnboundedReceiver<ToolPkgComposeDslEvent>>>,
    snapshots: Mutex<ComposeSnapshots>,
    closed: AtomicBool,
    cancellation: Notify,
    commandOwner: AtomicBool,
}

/// Owns an application-level composition session over unchanged CoreLink streams.
#[derive(Clone)]
pub struct ToolPkgComposeDslSession {
    state: Arc<ComposeSessionState>,
}

/// Streams one session's updates without retaining an unbounded replay history.
pub struct ToolPkgComposeDslEventStream {
    receiver: mpsc::UnboundedReceiver<ToolPkgComposeDslEvent>,
}

impl Stream for ToolPkgComposeDslEventStream {
    type Item = ToolPkgComposeDslEvent;

    /// Collects updates queued before attachment and every later session event in order.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        Box::pin(async move {
            while let Some(event) = self.receiver.recv().await {
                collector(event);
            }
        })
    }
}

impl ToolPkgComposeDslSession {
    /// Creates shared session state once for an explicitly owned page execution context.
    pub(super) fn new(engine: Arc<dyn JsExecutionEngine>, resources: Arc<BTreeMap<String, String>>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let state = Arc::new(ComposeSessionState {
            id: uuid::Uuid::new_v4().to_string(), engine, resources,
            sender: Mutex::new(Some(sender)), receiver: Mutex::new(Some(receiver)),
            snapshots: Mutex::new(ComposeSnapshots::default()),
            closed: AtomicBool::new(false), cancellation: Notify::new(), commandOwner: AtomicBool::new(false),
        });
        sessionRegistry().lock().expect("Compose registry mutex poisoned").insert(state.id.clone(), Arc::downgrade(&state));
        Self { state }
    }

    /// Exposes the opaque capability used by the application stream service.
    pub fn id(&self) -> String { self.state.id.clone() }

    /// Resolves a live page-owned session without extending registry ownership.
    pub fn resolve(id: &str) -> Result<Self, String> {
        let state = sessionRegistry().lock().map_err(|error| error.to_string())?
            .get(id).and_then(Weak::upgrade).ok_or("Compose session does not exist or has closed")?;
        if state.closed.load(Ordering::Acquire) { return Err("Compose session has closed".into()); }
        Ok(Self { state })
    }

    /// Attaches the session's single typed update consumer before submitting UI commands.
    pub fn updates(&self) -> Result<ToolPkgComposeDslEventStream, String> {
        let receiver = self.state.receiver.lock().map_err(|error| error.to_string())?.take().ok_or("Compose session updates have already been attached")?;
        Ok(ToolPkgComposeDslEventStream { receiver })
    }

    /// Submits commands in input order while ordinary asynchronous actions remain independent.
    pub async fn submit(&self, mut commands: ReverseStream<ToolPkgComposeDslCommand>) -> Result<(), String> {
        if self.state.commandOwner.swap(true, Ordering::AcqRel) {
            return Err("Compose session command stream has already been attached".into());
        }
        let _owner = ComposeCommandOwner(self.clone());
        loop {
            let cancellation = self.state.cancellation.notified();
            if self.state.closed.load(Ordering::Acquire) { return Ok(()); }
            let command = tokio::select! {
                command = commands.recv() => command,
                _ = cancellation => return Ok(()),
            };
            let Some(command) = command else { return Ok(()); };
            let session = self.clone();
            defaultHostRuntimeTaskSchedulerHost().scheduleHostRuntimeAsyncTask(
                "operit-compose-session-command",
                Box::new(move || Box::pin(async move { session.execute(command).await; })),
            ).map_err(|error| error.to_string())?;
        }
    }

    /// Cancels owned work and closes the update stream when its page context is released.
    pub fn close(&self) {
        if self.state.closed.swap(true, Ordering::AcqRel) { return; }
        sessionRegistry().lock().expect("Compose registry mutex poisoned").remove(&self.state.id);
        self.state.sender.lock().expect("Compose sender mutex poisoned").take();
        self.state.cancellation.notify_waiters();
    }

    /// Emits one event while the execution context still owns its stream.
    fn emit(&self, event: ToolPkgComposeDslEvent) {
        let sent = self.state.sender.lock().expect("Compose sender mutex poisoned").as_ref().map(|sender| sender.send(event));
        if matches!(sent, Some(Err(_))) { self.close(); }
    }

    /// Converts one structured response and suppresses unchanged state and memo snapshots.
    fn deliver(&self, requestId: &str, phase: &str, response: Value) -> Result<(), String> {
        let response: ComposeResponse = serde_json::from_value(response).map_err(|error| error.to_string())?;
        let mut snapshots = self.state.snapshots.lock().expect("Compose snapshot mutex poisoned");
        let state = if snapshots.state.as_ref() == Some(&response.state) { None } else { snapshots.state = Some(response.state.clone()); Some(response.state) };
        let memo = if snapshots.memo.as_ref() == Some(&response.memo) { None } else { snapshots.memo = Some(response.memo.clone()); Some(response.memo) };
        drop(snapshots);
        self.emit(ToolPkgComposeDslEvent { requestId: requestId.into(), phase: phase.into(), update: response.update, state, memo, actionResult: response.actionResult, navigationCommands: response.navigationCommands, error: None });
        Ok(())
    }

    /// Reports malformed results and execution errors through the same request-correlated stream.
    fn fail(&self, requestId: &str, error: String) {
        self.emit(ToolPkgComposeDslEvent { requestId: requestId.into(), phase: "error".into(), update: None, state: None, memo: None, actionResult: None, navigationCommands: Vec::new(), error: Some(error) });
    }

    /// Runs one command with an explicit operation and a cancellation lease owned by the page.
    async fn execute(&self, mut command: ToolPkgComposeDslCommand) {
        let cancellation = self.state.cancellation.notified();
        if self.state.closed.load(Ordering::Acquire) { return; }
        command.runtimeOptions.insert("__operit_compose_retained_session".into(), Value::Bool(true));
        let work = async {
            let result = match command.operation.as_str() {
                "render" => {
                    let script = command.script.ok_or("Compose render command requires a script")?;
                    self.state.engine.execute_compose_dsl_script_async(script, command.runtimeOptions, command.envOverrides, self.state.resources.clone()).await
                },
                "action" => {
                    let actionId = command.actionId.ok_or("Compose action command requires an action id")?;
                    let session = self.clone();
                    let requestId = command.requestId.clone();
                    self.state.engine.dispatch_compose_dsl_action_result_async(actionId, command.payload, command.runtimeOptions, command.envOverrides, Some(Arc::new(move |response| {
                        if let Err(error) = session.deliver(&requestId, "intermediate", response) { session.fail(&requestId, error); }
                    }))).await
                },
                _ => return Err(format!("Unsupported Compose session operation: {}", command.operation)),
            }.map_err(|error| error.to_string())?;
            if let Some(response) = result { self.deliver(&command.requestId, "final", response)?; }
            Ok::<(), String>(())
        };
        tokio::select! {
            result = work => {
                if let Err(error) = result { self.fail(&command.requestId, error); }
                self.emit(ToolPkgComposeDslEvent { requestId: command.requestId, phase: "complete".into(), update: None, state: None, memo: None, actionResult: None, navigationCommands: Vec::new(), error: None });
            },
            _ = cancellation => {},
        }
    }
}

struct ComposeCommandOwner(ToolPkgComposeDslSession);

impl Drop for ComposeCommandOwner {
    /// Ends a session when the caller closes or abandons its reverse command stream.
    fn drop(&mut self) { self.0.close(); }
}

/// Stores non-owning capabilities so stream proxies cannot leak execution contexts.
fn sessionRegistry() -> &'static Mutex<BTreeMap<String, Weak<ComposeSessionState>>> {
    static REGISTRY: OnceLock<Mutex<BTreeMap<String, Weak<ComposeSessionState>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(BTreeMap::new()))
}

impl Drop for ComposeSessionState {
    /// Removes an abandoned capability when its last real owner is released.
    fn drop(&mut self) {
        sessionRegistry().lock().expect("Compose registry mutex poisoned").remove(&self.id);
    }
}
