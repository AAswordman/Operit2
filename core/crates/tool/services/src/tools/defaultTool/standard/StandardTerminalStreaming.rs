//! Concurrent, owner-scoped terminal observations over the common tool transport.
#![allow(non_snake_case)]

use super::StandardTerminalTools::{terminalCommandResultData, StandardTerminalTools};
use super::ToolArguments::decodeArguments;
use crate::tools::ToolInvocation::prepareInvocation;
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext, ToolExecutionManager,
};
use operit_host_api::{HostError, HostResult, HostRuntimeTaskSchedulerHost, TerminalHost};
use operit_plugin_sdk::js_sdk::results::{
    JsNullable, TerminalCommandResultData, TerminalImplementation, TerminalObservationResultData,
    TerminalType, ToolResultData,
};
use operit_plugin_sdk::js_sdk::system::{TerminalExecEvent, TerminalExecRequest, TerminalRunStreamRequest};
use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

const MAX_OBSERVATIONS: usize = 32;
const MAX_QUEUED_BYTES: usize = 1024 * 1024;
const DEFAULT_WAIT_MS: u64 = 20_000;

#[derive(serde::Deserialize)]
#[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)]
enum Operation {
    Run {
        sessionName: String,
        terminalType: Option<String>,
        profileId: Option<String>,
        command: String,
        waitMs: u64,
    },
    Wait {
        sessionId: String,
        waitMs: u64,
    },
    Open {
        request: TerminalExecRequest,
    },
    Runstream {
        request: TerminalRunStreamRequest,
    },
    Next {
        handle: String,
    },
    Close {
        handle: String,
    },
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    observation: Operation,
}

pub(crate) struct TerminalStreamExecutor {
    tools: StandardTerminalTools,
    managedAdmission: Arc<tokio::sync::Mutex<()>>,
    selfRef: std::sync::Weak<TerminalStreamExecutor>,
    host: Option<Arc<dyn TerminalHost>>,
    scheduler: Option<Arc<dyn HostRuntimeTaskSchedulerHost>>,
    observations: Mutex<HashMap<String, Arc<Observation>>>,
    workers: Arc<AtomicUsize>,
}
// A disposed handle still counts until its native command actually stops.
struct WorkerPermit(Arc<AtomicUsize>);
impl Drop for WorkerPermit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
fn chunkCost(event: &TerminalExecEvent) -> usize {
    match event {
        TerminalExecEvent::Chunk {
            sessionId,
            command,
            platform,
            chunk,
            ..
        } => {
            std::mem::size_of::<TerminalExecEvent>()
                + sessionId.len()
                + command.len()
                + platform.len()
                + chunk.len()
        }
        _ => 0,
    }
}
struct Observation {
    owner: String,
    sessionId: String,
    detached: AtomicBool,
    deadline: Option<u64>,
    metadata: TerminalCommandResultData,
    pulling: AtomicBool,
    ready: tokio::sync::Notify,
    inner: Mutex<ObservationState>,
}
struct ObservationState {
    receipt: Option<TerminalCommandResultData>,
    output: String,
    events: VecDeque<TerminalExecEvent>,
    queuedBytes: usize,
    result: Option<Result<(), String>>,
}
impl TerminalStreamExecutor {
    /// Reserves a safe session generation before starting a public output iterator.
    async fn openStream(
        &self,
        owner: String,
        request: TerminalExecRequest,
    ) -> Result<String, String> {
        let _admission = self.managedAdmission.lock().await;
        self.requireIdleSession(&request.sessionId)?;
        self.openExecution(owner, request, false, false)
    }

    /// Checks the exact original session before starting another command in its immutable host.
    fn requireIdleSession(&self, sessionId: &str) -> Result<(), String> {
        let host = self.host.as_ref().ok_or("Terminal host is unavailable")?;
        let session = host.listSessions().map_err(|error| error.message)?.into_iter()
            .find(|session| session.sessionId == sessionId).ok_or("Terminal session does not exist")?;
        if session.commandRunning { return Err("Terminal session already has a running command".into()); }
        for value in self.observations.lock().map_err(|_| "Terminal observations are poisoned")?.values() {
            if value.sessionId == sessionId && value.inner.lock().map_err(|_| "Terminal observation is poisoned")?.result.is_none() {
                return Err("Terminal session already has a running command".into());
            }
        }
        Ok(())
    }

    /// Reuses admission and observations whenever tool handlers share one terminal host.
    pub(crate) fn shared(tools: StandardTerminalTools, scheduler: Option<Arc<dyn HostRuntimeTaskSchedulerHost>>) -> Arc<Self> {
        static EXECUTORS: std::sync::OnceLock<Mutex<HashMap<usize, std::sync::Weak<TerminalStreamExecutor>>>> = std::sync::OnceLock::new();
        let Some(host) = tools.terminalHost.as_ref() else { return Self::new(tools, scheduler); };
        let key = Arc::as_ptr(host) as *const () as usize;
        let mut executors = EXECUTORS.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        executors.retain(|_, executor| executor.strong_count() > 0);
        if let Some(existing) = executors.get(&key).and_then(std::sync::Weak::upgrade) { return existing; }
        let executor = Self::new(tools, scheduler);
        executors.insert(key, Arc::downgrade(&executor));
        executor
    }

    /// Binds terminal execution to the host-owned cooperative task scheduler.
    pub(crate) fn new(
        tools: StandardTerminalTools,
        scheduler: Option<Arc<dyn HostRuntimeTaskSchedulerHost>>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|selfRef| Self {
            host: tools.terminalHost.clone(),
            tools,
            managedAdmission: Arc::new(tokio::sync::Mutex::new(())),
            selfRef: selfRef.clone(),
            scheduler,
            observations: Mutex::new(HashMap::new()),
            workers: Arc::new(AtomicUsize::new(0)),
        })
    }
    /// Validates one observation and schedules its command through the registered host.
    fn open(&self, owner: String, request: TerminalExecRequest) -> Result<String, String> {
        self.openExecution(owner, request, false, false)
    }

    /// Schedules either a disposable stream or a command whose lifetime belongs to its session.
    fn openExecution(
        &self,
        owner: String,
        request: TerminalExecRequest,
        receiptOnly: bool,
        continuous: bool,
    ) -> Result<String, String> {
        if request.sessionId.trim().is_empty()
            || request.sessionId.trim() != request.sessionId
            || request.sessionId.contains('\0')
        {
            return Err("Terminal stream requires an exact nonblank sessionId".into());
        }
        if request.command.trim().is_empty() || request.command.contains('\0') {
            return Err("Terminal stream requires a command without NUL bytes".into());
        }
        let host = self.host.clone().ok_or("Terminal host is unavailable")?;
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or("Runtime task scheduler host is unavailable")?;
        let sessions = host.listSessions().map_err(|error| error.message)?;
        let session = sessions
            .iter()
            .find(|session| session.sessionId == request.sessionId)
            .ok_or("Terminal session does not exist")?
            .clone();
        let terminal = TerminalImplementation::try_from(session.terminal.as_str())?;
        let terminalType = TerminalType::try_from(session.terminalType.as_str())?;
        let observation = Arc::new(Observation {
            owner,
            sessionId: request.sessionId.clone(),
            detached: AtomicBool::new(false),
            deadline: if receiptOnly || continuous {
                None
            } else {
                Some(
                    scheduler
                        .monotonicTimeMillis()
                        .map_err(|error| error.message)?
                        .checked_add(request.timeoutMs.unwrap_or(DEFAULT_WAIT_MS))
                        .ok_or("Terminal wait deadline overflow")?,
                )
            },
            metadata: TerminalCommandResultData {
                sessionId: request.sessionId.clone(),
                command: request.command.clone(),
                platform: session.platform.clone(),
                terminal: terminal.clone(),
                terminalType: terminalType.clone(),
                output: String::new(),
                exitCode: -1,
                timedOut: true,
            },
            pulling: AtomicBool::new(false),
            ready: tokio::sync::Notify::new(),
            inner: Mutex::new(ObservationState {
                receipt: None,
                output: String::new(),
                events: VecDeque::from([TerminalExecEvent::Start {
                    sessionId: request.sessionId.clone(),
                    command: request.command.clone(),
                    platform: session.platform.clone(),
                    terminal: terminal.clone(),
                    terminalType: terminalType.clone(),
                }]),
                queuedBytes: 0,
                result: None,
            }),
        });
        let handle = uuid::Uuid::new_v4().to_string();
        let permit = {
            let mut registry = self
                .observations
                .lock()
                .map_err(|_| "Terminal observation registry is poisoned")?;
            registry.retain(|_, value| {
                sessions
                    .iter()
                    .any(|session| session.sessionId == value.sessionId)
                    && value.sessionId != request.sessionId
            });
            if registry.len() >= MAX_OBSERVATIONS
                || self.workers.load(Ordering::Acquire) >= MAX_OBSERVATIONS
            {
                return Err("Too many terminal observations; close an existing stream".into());
            }
            registry.insert(handle.clone(), observation.clone());
            self.workers.fetch_add(1, Ordering::AcqRel);
            WorkerPermit(self.workers.clone())
        };
        let onOutput = {
            let mut decoder = OutputDecoder::default();
            let mut chunkIndex = 0;
            let mut receivedChars = 0;
            let request = request.clone();
            let observation = observation.clone();
            Box::new(move |data: &[u8], finalFrame: bool| -> HostResult<()> {
                let text = decoder.feed(data, finalFrame)?;
                if !text.is_empty() {
                    let mut inner = observation
                        .inner
                        .lock()
                        .map_err(|_| HostError::new("Terminal observation is poisoned"))?;
                    {
                        inner.output.push_str(&text);
                        if inner.output.len() > MAX_QUEUED_BYTES {
                            let mut start = inner.output.len() - MAX_QUEUED_BYTES;
                            while !inner.output.is_char_boundary(start) {
                                start += 1;
                            }
                            inner.output.drain(..start);
                        }
                    }
                    if receiptOnly || observation.detached.load(Ordering::Acquire) {
                        return Ok(());
                    }
                    receivedChars += text.encode_utf16().count() as u64;
                    let event = TerminalExecEvent::Chunk {
                        sessionId: request.sessionId.clone(),
                        command: request.command.clone(),
                        platform: session.platform.clone(),
                        terminal: terminal.clone(),
                        terminalType: terminalType.clone(),
                        chunk: text,
                        chunkIndex,
                        receivedChars,
                    };
                    let cost = chunkCost(&event);
                    while inner.queuedBytes + cost > MAX_QUEUED_BYTES {
                        let index = inner
                            .events
                            .iter()
                            .position(|event| matches!(event, TerminalExecEvent::Chunk { .. }));
                        let Some(index) = index else {
                            break;
                        };
                        let event = inner.events.remove(index).unwrap();
                        inner.queuedBytes -= chunkCost(&event);
                    }
                    inner.queuedBytes += cost;
                    inner.events.push_back(event);
                    chunkIndex += 1;
                    observation.ready.notify_one();
                }
                Ok(())
            }) as operit_host_api::TerminalOutputCallback
        };
        // A queued host task must retain the shared admission state even if its
        // originating tool handler is destroyed before the worker starts.
        let executorLease = self.selfRef.upgrade();
        let scheduled = host.scheduleInSessionStreaming(
            scheduler.clone(),
            request.sessionId,
            request.command,
            0,
            onOutput,
            Box::new(move |result| {
                let _executorLease = executorLease;
                drop(permit);
                if let Ok(mut inner) = observation.inner.lock() {
                    let result = result.and_then(|receipt| terminalCommandResultData(&receipt));
                    inner.result = Some(match result {
                        Ok(receipt) => {
                            inner.receipt = Some(receipt.clone());
                            inner
                                .events
                                .push_back(TerminalExecEvent::Completed { result: receipt });
                            Ok(())
                        }
                        Err(error) => Err(error.message),
                    });
                }
                observation.ready.notify_waiters();
            }),
        );
        if let Err(error) = scheduled {
            self.observations
                .lock()
                .map_err(|_| "Terminal observation registry is poisoned")?
                .remove(&handle);
            return Err(format!(
                "Cannot schedule terminal streaming task: {}",
                error.message
            ));
        }
        Ok(handle)
    }
    /// Starts a command and waits only for the requested foreground budget, retaining its session.
    async fn runManaged(
        &self,
        owner: String,
        name: String,
        terminalType: Option<String>,
        profileId: Option<String>,
        command: String,
        waitMs: u64,
    ) -> Result<TerminalCommandResultData, String> {
        let handle = self
            .startNamed(owner.clone(), name, terminalType, profileId, command, waitMs, true, false)
            .await?;
        let observation = self.owned(&owner, &handle)?;
        self.waitManaged(&owner, &observation.sessionId, waitMs)
            .await
    }

    /// Isolates synchronous session preparation from owner event collection on coordination workers.
    async fn startNamed(
        &self,
        owner: String,
        name: String,
        terminalType: Option<String>,
        profileId: Option<String>,
        command: String,
        waitMs: u64,
        receiptOnly: bool,
        continuous: bool,
    ) -> Result<String, String> {
        let executor = self.selfRef.upgrade().ok_or("Terminal executor was released")?;
        let context = ToolExecutionManager::currentToolInvocationContext();
        let admission = self.managedAdmission.clone().lock_owned().await;
        let scheduler = self.scheduler.clone().ok_or("Runtime task scheduler host is unavailable")?;
        operit_host_api::TerminalCommand::runTerminalBlocking(scheduler, "terminal-start", move || {
            let _admission = admission;
            let prepare = || {
                let reserved = executor.observations.lock().map_err(|_| "Terminal observation registry is poisoned")?
                    .values().filter_map(|value| match value.inner.lock() {
                        Ok(inner) if inner.result.is_none() => Some(Ok(value.sessionId.clone())),
                        Ok(_) => None,
                        Err(_) => Some(Err("Terminal observation is poisoned")),
                    }).collect::<Result<std::collections::BTreeSet<_>, _>>()?;
                let session = executor.tools
                    .availableSelectedSession(&name, terminalType.as_deref(), profileId.as_deref(), &reserved)
                    .map_err(|error| error.message)?;
                let opened = executor.openExecution(
                    owner,
                    TerminalExecRequest {
                        sessionId: session.sessionId.clone(), command, timeoutMs: Some(waitMs),
                    },
                    receiptOnly,
                    continuous,
                );
                if let Err(error) = &opened {
                    if session.isNewSession {
                        executor.host.as_ref().ok_or("Terminal host is unavailable")?
                            .closeSession(&session.sessionId)
                            .map_err(|close| format!("{error}; closing unused session failed: {}", close.message))?;
                    }
                }
                opened
            };
            match context { Some(context) => context.scopeSync(prepare), None => prepare() }.map_err(HostError::new)
        }).await.map_err(|error| error.message)
    }

    /// Runs an existing session through the same managed admission and generation rules.
    async fn runExisting(
        &self,
        owner: String,
        request: TerminalExecRequest,
    ) -> Result<TerminalCommandResultData, String> {
        let _admission = self.managedAdmission.lock().await;
        self.requireIdleSession(&request.sessionId)?;
        let sessionId = request.sessionId.clone();
        let waitMs = request.timeoutMs.unwrap_or(DEFAULT_WAIT_MS);
        self.openExecution(owner.clone(), request, true, false)?;
        drop(_admission);
        self.waitManaged(&owner, &sessionId, waitMs).await
    }

    /// Observes the existing command without injecting a second command into its busy PTY.
    async fn waitManaged(
        &self,
        owner: &str,
        sessionId: &str,
        waitMs: u64,
    ) -> Result<TerminalCommandResultData, String> {
        let observation = self
            .observations
            .lock()
            .map_err(|_| "Terminal observation registry is poisoned")?
            .values()
            .find(|value| value.owner == owner && value.sessionId == sessionId)
            .cloned()
            .ok_or("No managed terminal command exists for this session")?;
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or("Runtime task scheduler host is unavailable")?;
        let deadline = scheduler
            .monotonicTimeMillis()
            .map_err(|error| error.message)?
            .checked_add(waitMs)
            .ok_or("Terminal wait deadline overflow")?;
        let mut elapsed = scheduler.waitForHostRuntimeDelay(waitMs);
        let mut expired = false;
        loop {
            let ready = observation.ready.notified();
            tokio::pin!(ready);
            ready.as_mut().enable();
            {
                let inner = observation
                    .inner
                    .lock()
                    .map_err(|_| "Terminal observation is poisoned")?;
                if let Some(Err(error)) = &inner.result {
                    return Err(error.clone());
                }
                if let Some(result) = inner.receipt.as_ref() {
                    return Ok(result.clone());
                }
                if expired || scheduler
                    .monotonicTimeMillis()
                    .map_err(|error| error.message)?
                    >= deadline
                {
                    let mut result = observation.metadata.clone();
                    result.output = inner.output.clone();
                    return Ok(result);
                }
            }
            tokio::select! {
                _ = ready => {},
                result = &mut elapsed => { result.map_err(|error| error.message)?; expired = true; }
            }
        }
    }
    fn owned(&self, owner: &str, handle: &str) -> Result<Arc<Observation>, String> {
        let registry = self
            .observations
            .lock()
            .map_err(|_| "Terminal observation registry is poisoned")?;
        let observation = registry
            .get(handle)
            .filter(|value| value.owner == owner)
            .ok_or("Unknown terminal observation")?;
        Ok(observation.clone())
    }
    async fn next(&self, owner: &str, handle: &str) -> Result<Option<TerminalExecEvent>, String> {
        let observation = self.owned(owner, handle)?;
        if observation.pulling.swap(true, Ordering::AcqRel) {
            return Err("Terminal execution permits only one pending next call".into());
        }
        struct PullGuard<'a>(&'a AtomicBool);
        impl Drop for PullGuard<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _guard = PullGuard(&observation.pulling);
        let scheduler = self.scheduler.as_ref()
            .ok_or("Runtime task scheduler host is unavailable")?;
        let mut elapsed = match observation.deadline {
            Some(deadline) => Some(scheduler.waitForHostRuntimeDelay(deadline.saturating_sub(
                scheduler.monotonicTimeMillis().map_err(|error| error.message)?))),
            None => None,
        };
        let mut deadlineReached = false;
        loop {
            let notified = observation.ready.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let scheduler = self
                .scheduler
                .as_ref()
                .ok_or("Runtime task scheduler host is unavailable")?;
            let expired = deadlineReached || match observation.deadline {
                Some(deadline) => scheduler.monotonicTimeMillis().map_err(|error| error.message)? >= deadline,
                None => false,
            };
            if expired
                && observation
                    .inner
                    .lock()
                    .map_err(|_| "Terminal observation is poisoned")?
                    .result
                    .is_none()
            {
                observation.detached.store(true, Ordering::Release);
                let result = self.waitManaged(owner, &observation.sessionId, 0).await?;
                return Ok(Some(TerminalExecEvent::Completed { result }));
            }
            {
                let mut inner = observation
                    .inner
                    .lock()
                    .map_err(|_| "Terminal observation is poisoned")?;
                if observation.detached.load(Ordering::Acquire) {
                    return Ok(None);
                }
                if let Some(event) = inner.events.pop_front() {
                    inner.queuedBytes -= chunkCost(&event);
                    return Ok(Some(event));
                }
                if let Some(result) = &inner.result {
                    return result.clone().map(|_| None);
                }
            }
            match elapsed.as_mut() {
                Some(elapsed) => tokio::select! {
                    _ = notified => {},
                    result = elapsed => { result.map_err(|error| error.message)?; deadlineReached = true; }
                },
                None => notified.await,
            }
        }
    }
    fn close(&self, owner: &str, handle: &str) -> Result<(), String> {
        let observation = self.owned(owner, handle)?;
        observation.detached.store(true, Ordering::Release);
        observation.ready.notify_waiters();
        Ok(())
    }
}
impl SharedToolExecutor for TerminalStreamExecutor {
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        context: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        let arguments: Arguments = decodeArguments(request)?;
        let owner = terminalOwner(context)?;
        let effect = if matches!(
            &arguments.observation,
            Operation::Next { .. } | Operation::Wait { .. }
        ) {
            ToolEffect::READ
        } else {
            // Opening or detaching an output subscription changes terminal ownership state.
            ToolEffect::WRITE
        };
        Ok(prepareInvocation(
            request,
            checkAccess,
            effect,
            async move {
                let result = match arguments.observation {
                    Operation::Run {
                        sessionName,
                        terminalType,
                        profileId,
                        command,
                        waitMs,
                    } => TerminalObservationResultData::Run {
                        result: self
                            .runManaged(owner, sessionName, terminalType, profileId, command, waitMs)
                            .await?,
                    },
                    Operation::Wait { sessionId, waitMs } => TerminalObservationResultData::Run {
                        result: self.waitManaged(&owner, &sessionId, waitMs).await?,
                    },
                    Operation::Open { request } => TerminalObservationResultData::Open {
                        handle: self.openStream(owner, request).await?,
                    },
                    Operation::Runstream { request } => TerminalObservationResultData::Open {
                        handle: self.startNamed(owner, request.sessionName,
                            request.terminalType.map(|value| value.as_str().to_string()),
                            request.profileId, request.command, 0, false, true).await?,
                    },
                    Operation::Next { handle } => TerminalObservationResultData::Next {
                        event: JsNullable::from_option(self.next(&owner, &handle).await?),
                    },
                    Operation::Close { handle } => {
                        self.close(&owner, &handle)?;
                        TerminalObservationResultData::Close {}
                    }
                };
                Ok(ToolResultData::TerminalObservationResultData(result))
            },
        ))
    }
}

/// Coordinates conditional close with admission so queued executions cannot lose their session.
pub(crate) struct TerminalCloseExecutor(pub Arc<TerminalStreamExecutor>);

impl TerminalStreamExecutor {
    async fn closeSession(&self, sessionId: &str, sessionName: &str, idleOnly: bool) -> Result<ToolResultData, String> {
        let _admission = self.managedAdmission.lock().await;
        if idleOnly {
            let registry = self.observations.lock().map_err(|_| "Terminal observation registry is poisoned")?;
            for observation in registry.values().filter(|value| value.sessionId == sessionId) {
                if observation.inner.lock().map_err(|_| "Terminal observation is poisoned")?.result.is_none() {
                    return Err(format!("Error closing terminal session {sessionId}: terminal command is queued or running"));
                }
            }
        }
        let host = self.host.as_ref().ok_or("Terminal host is unavailable")?;
        let result = if idleOnly { host.closeSessionIfIdle(sessionId, sessionName) } else { host.closeSession(sessionId) }
            .map_err(|error| format!("Error closing terminal session {sessionId}: {}", error.message))?;
        Ok(ToolResultData::TerminalSessionCloseResultData(
            operit_plugin_sdk::js_sdk::results::TerminalSessionCloseResultData {
                sessionId: result.sessionId, success: result.success, message: result.message,
            },
        ))
    }
}

impl SharedToolExecutor for TerminalCloseExecutor {
    fn prepare<'a>(&'a self, request: &'a ToolCall, _context: &'a ToolInvocationContext, checkAccess: bool) -> Result<PreparedToolInvocation<'a>, String> {
        let text = |key: &str| -> Result<Option<String>, String> {
            match request.parameters.get(key) {
                None | Some(serde_json::Value::Null) => Ok(None),
                Some(serde_json::Value::String(value)) => Ok(Some(value.clone())),
                Some(serde_json::Value::Bool(value)) if key == "if_idle" => Ok(Some(value.to_string())),
                Some(_) => Err(format!("{key} must be text")),
            }
        };
        let sessionId = text("session_id")?.unwrap_or_default();
        if sessionId.is_empty() { return Err("session_id is required.".into()); }
        let sessionName = text("session_name")?.unwrap_or_default();
        let idleOnly = match text("if_idle")?.as_deref() {
            None | Some("false") => false,
            Some("true") if !sessionName.is_empty() => true,
            Some("true") => return Err("session_name is required for idle close.".into()),
            _ => return Err("if_idle must be true or false.".into()),
        };
        Ok(prepareInvocation(request, checkAccess, ToolEffect::WRITE, async move {
            self.0.closeSession(&sessionId, &sessionName, idleOnly).await
        }))
    }
}

/// Uses the authenticated plugin identity or the host's direct invocation authority.
fn terminalOwner(context: &ToolInvocationContext) -> Result<String, String> {
    match &context.host().authenticatedExtensionOwner {
        Some(_) => context
            .requirePluginOwner()
            .map(|owner| format!("plugin:{owner}")),
        None => Ok("host-terminal".to_string()),
    }
}

/// Routes the original public execution tools through session-owned command management.
pub(crate) struct TerminalExecutionExecutor(pub Arc<TerminalStreamExecutor>);

impl SharedToolExecutor for TerminalExecutionExecutor {
    /// Validates the existing public argument contract before starting managed execution.
    fn prepare<'a>(
        &'a self,
        request: &'a ToolCall,
        context: &'a ToolInvocationContext,
        checkAccess: bool,
    ) -> Result<PreparedToolInvocation<'a>, String> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ExecArguments {
            session_id: String,
            command: String,
            timeout_ms: Option<serde_json::Value>,
        }
        let args: ExecArguments = decodeArguments(request)?;
        let waitMs = match args.timeout_ms {
            None => DEFAULT_WAIT_MS,
            Some(serde_json::Value::Number(value)) => value
                .as_u64()
                .ok_or("timeout_ms must be a nonnegative integer")?,
            Some(serde_json::Value::String(value)) => value
                .parse::<u64>()
                .map_err(|_| "timeout_ms must be a nonnegative integer")?,
            Some(_) => return Err("timeout_ms must be a nonnegative integer".into()),
        };
        let owner = terminalOwner(context)?;
        Ok(prepareInvocation(
            request,
            checkAccess,
            ToolEffect::WRITE,
            async move {
                Ok(ToolResultData::TerminalCommandResultData(
                    self.0
                        .runExisting(
                            owner,
                            TerminalExecRequest {
                                sessionId: args.session_id,
                                command: args.command,
                                timeoutMs: Some(waitMs),
                            },
                        )
                        .await?,
                ))
            },
        ))
    }
}

/// Holds split UTF-8 and private completion markers until their boundaries are known.
#[derive(Default)]
struct OutputDecoder {
    pending: Vec<u8>,
    utf8: Vec<u8>,
}
impl OutputDecoder {
    fn feed(&mut self, bytes: &[u8], finalFrame: bool) -> HostResult<String> {
        const PREFIX: &[u8] = b"\x1b]133;OperitPrompt=";
        self.pending.extend_from_slice(bytes);
        let mut used = 0;
        while used < self.pending.len() {
            let tail = &self.pending[used..];
            if tail.starts_with(PREFIX) {
                if let Some(length) = tail[PREFIX.len()..].iter().position(|byte| *byte == 7) {
                    used += PREFIX.len() + length + 1;
                    continue;
                }
                if tail.len() > 4096 {
                    return Err(HostError::new(
                        "Terminal completion marker exceeds its size limit",
                    ));
                }
                if finalFrame {
                    used = self.pending.len();
                }
                break;
            }
            if !finalFrame && PREFIX.starts_with(tail) {
                break;
            }
            self.utf8.push(self.pending[used]);
            used += 1;
        }
        self.pending.drain(..used);
        let mut text = String::new();
        let mut used = 0;
        while used < self.utf8.len() {
            match std::str::from_utf8(&self.utf8[used..]) {
                Ok(value) => {
                    text.push_str(value);
                    used = self.utf8.len();
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    text.push_str(
                        std::str::from_utf8(&self.utf8[used..used + valid])
                            .expect("validated UTF-8 prefix"),
                    );
                    used += valid;
                    match error.error_len() {
                        Some(length) => {
                            text.push('\u{fffd}');
                            used += length;
                        }
                        None => {
                            if finalFrame {
                                text.push_str(&String::from_utf8_lossy(&self.utf8[used..]));
                                used = self.utf8.len();
                            }
                            break;
                        }
                    }
                }
            }
        }
        self.utf8.drain(..used);
        Ok(text)
    }
}

#[cfg(all(test, not(target_arch = "wasm32"), not(target_os = "windows")))]
mod tests {
    use super::*;
    use operit_host_native_terminal::NativePtyTerminalHost;
    use std::time::Duration;

    /// Holds blocking command jobs while keeping the real Host clock and async executor available.
    #[derive(Default)]
    struct QueuedScheduler {
        clock: operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost,
        jobs: Mutex<Vec<operit_host_api::HostRuntimeTask>>,
        holdStartup: AtomicBool,
    }
    impl HostRuntimeTaskSchedulerHost for QueuedScheduler {
        /// Reads the production Host clock.
        fn monotonicTimeMillis(&self) -> HostResult<u64> {
            self.clock.monotonicTimeMillis()
        }
        /// Models a saturated blocking pool without starting its queued commands.
        fn scheduleHostRuntimeTask(
            &self,
            name: &str,
            task: operit_host_api::HostRuntimeTask,
        ) -> HostResult<()> {
            if name == "terminal-start" && !self.holdStartup.load(Ordering::Acquire) {
                return self.clock.scheduleHostRuntimeTask(name, task);
            }
            self.jobs.lock().unwrap().push(task);
            Ok(())
        }
        /// Keeps async coordination on its normal executor.
        fn scheduleHostRuntimeAsyncTask(
            &self,
            name: &str,
            task: operit_host_api::HostRuntimeAsyncTask,
        ) -> HostResult<()> {
            self.clock.scheduleHostRuntimeAsyncTask(name, task)
        }
        /// Holds command submission before its observer owns any running work.
        fn scheduleHostRuntimeCooperativeAsyncTask(
            &self,
            name: &str,
            task: operit_host_api::HostRuntimeAsyncTask,
        ) -> HostResult<()> {
            let (clock, name) = (self.clock, name.to_owned());
            self.jobs.lock().unwrap().push(Box::new(move || { clock.scheduleHostRuntimeCooperativeAsyncTask(&name, task).unwrap(); }));
            Ok(())
        }
        /// Preserves production delayed task behavior.
        fn scheduleDelayedHostRuntimeTask(
            &self,
            name: &str,
            delay: u64,
            task: operit_host_api::HostRuntimeTask,
        ) -> HostResult<()> {
            self.clock.scheduleDelayedHostRuntimeTask(name, delay, task)
        }
        /// Preserves production nonblocking delays.
        fn waitForHostRuntimeDelay(&self, delay: u64) -> operit_host_api::HostRuntimeTurnFuture {
            self.clock.waitForHostRuntimeDelay(delay)
        }
    }

    /// Startup must yield to its blocking worker while shared admission retains queued sessions.
    #[tokio::test]
    async fn handlers_sharing_a_host_share_terminal_admission() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let scheduler = Arc::new(QueuedScheduler::default());
        let first = TerminalStreamExecutor::shared(StandardTerminalTools::new(Some(host.clone())), Some(scheduler.clone()));
        let second = TerminalStreamExecutor::shared(StandardTerminalTools::new(Some(host.clone())), Some(scheduler.clone()));
        assert!(Arc::ptr_eq(&first, &second));
        let name = uuid::Uuid::new_v4().to_string();
        scheduler.holdStartup.store(true, Ordering::Release);
        let mut opening = Box::pin(first.startNamed("owner".into(), name.clone(), Some("bash".into()), None, "printf queued".into(), 1000, true, false));
        assert!(futures_util::FutureExt::now_or_never(opening.as_mut()).is_none());
        assert!(host.listSessions().unwrap().is_empty(), "session creation ran before its blocking worker");
        let start = scheduler.jobs.lock().unwrap().pop().unwrap();
        start();
        let handle = opening.await.unwrap();
        let session = first.owned("owner", &handle).unwrap().sessionId.clone();
        drop(first);
        drop(second);
        let later = TerminalStreamExecutor::shared(StandardTerminalTools::new(Some(host)), Some(scheduler.clone()));
        assert!(later.closeSession(&session, &name, true).await.is_err());
        assert!(later.closeSession(&session, "", false).await.is_ok());
        scheduler.jobs.lock().unwrap().clear();
    }

    /// A continuous package stream must survive arbitrarily late output without a foreground deadline.
    #[tokio::test]
    async fn continuous_named_stream_retains_session_and_delivers_late_chunks_until_completion() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let scheduler = Arc::new(QueuedScheduler::default());
        let tools = TerminalStreamExecutor::new(StandardTerminalTools::new(Some(host.clone())), Some(scheduler.clone()));
        let name = uuid::Uuid::new_v4().to_string();
        let handle = tools.startNamed("owner".into(), name.clone(), Some("bash".into()), None,
            "printf queued".into(), 0, false, true).await.unwrap();
        let observation = tools.owned("owner", &handle).unwrap();
        assert_eq!(observation.deadline, None);
        assert!(matches!(tools.next("owner", &handle).await.unwrap(), Some(TerminalExecEvent::Start { .. })));
        {
            let mut state = observation.inner.lock().unwrap();
            state.events.push_back(TerminalExecEvent::Chunk {
                sessionId: observation.sessionId.clone(), command: "printf queued".into(), platform: "linux".into(),
                terminal: observation.metadata.terminal.clone(), terminalType: observation.metadata.terminalType.clone(),
                chunk: "late output".into(), chunkIndex: 0, receivedChars: 11,
            });
            state.queuedBytes = state.events.iter().map(chunkCost).sum();
        }
        assert!(matches!(tools.next("owner", &handle).await.unwrap(), Some(TerminalExecEvent::Chunk { chunk, .. }) if chunk == "late output"));
        let mut receipt = observation.metadata.clone();
        receipt.timedOut = false;
        receipt.exitCode = 0;
        {
            let mut state = observation.inner.lock().unwrap();
            state.result = Some(Ok(()));
            state.events.push_back(TerminalExecEvent::Completed { result: receipt });
        }
        assert!(matches!(tools.next("owner", &handle).await.unwrap(), Some(TerminalExecEvent::Completed { result }) if !result.timedOut));
        tools.close("owner", &handle).unwrap();
        assert!(host.listSessions().unwrap().iter().any(|session| session.sessionId == observation.sessionId));
        scheduler.jobs.lock().unwrap().clear();
        host.closeSession(&observation.sessionId).unwrap();
    }

    #[tokio::test]
    async fn idle_close_rejects_queued_execution_and_preserves_force_close() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let scheduler = Arc::new(QueuedScheduler::default());
        let tools = TerminalStreamExecutor::new(StandardTerminalTools::new(Some(host.clone())), Some(scheduler));
        let name = uuid::Uuid::new_v4().to_string();
        let handle = tools.startNamed("owner".into(), name.clone(), Some("bash".into()), None, "printf queued".into(), 1000, true, false).await.unwrap();
        let session = tools.owned("owner", &handle).unwrap().sessionId.clone();
        let error = match tools.closeSession(&session, &name, true).await { Err(error) => error, Ok(_) => panic!("queued session was closed") };
        assert!(error.contains("queued or running"));
        assert!(host.listSessions().unwrap().iter().any(|entry| entry.sessionId == session));
        assert!(tools.closeSession(&session, "", false).await.is_ok());
        let idle = host.createOrGetTypedSession(&format!("{name}_idle"), "bash").unwrap();
        assert!(tools.closeSession(&idle.sessionId, "wrong-owner", true).await.is_err());
        assert!(tools.closeSession(&idle.sessionId, &idle.sessionName, true).await.is_ok());
    }

    /// Queued jobs must return receipts and reserve separate sessions even before Host execution starts.
    #[tokio::test]
    async fn pending_commands_do_not_block_foreground_waits_or_reuse_pending_sessions() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let scheduler = Arc::new(QueuedScheduler::default());
        let tools = TerminalStreamExecutor::new(
            StandardTerminalTools::new(Some(host.clone())),
            Some(scheduler.clone()),
        );
        let name = uuid::Uuid::new_v4().to_string();
        let (first, second) = tokio::time::timeout(Duration::from_secs(5), async {
            let first = tools
                .runManaged(
                    "owner".into(),
                    name.clone(),
                    Some("bash".into()), None,
                    "printf first".into(),
                    20,
                )
                .await
                .unwrap();
            let second = tools
                .runManaged(
                    "owner".into(),
                    name,
                    Some("bash".into()), None,
                    "printf second".into(),
                    0,
                )
                .await
                .unwrap();
            (first, second)
        })
        .await
        .expect("A queued Host job must not prevent returning a foreground receipt");
        assert!(first.timedOut && second.timedOut);
        assert_ne!(first.sessionId, second.sessionId);
        assert_eq!(scheduler.jobs.lock().unwrap().len(), 2);
        scheduler.jobs.lock().unwrap().clear();
        assert_eq!(tools.workers.load(Ordering::Acquire), 0);
        host.closeSession(&first.sessionId).unwrap();
        host.closeSession(&second.sessionId).unwrap();
    }

    #[test]
    fn output_decoder_preserves_split_unicode_and_hides_split_native_markers() {
        let mut decoder = OutputDecoder::default();
        let input = "雪🌍\x1b]133;OperitPrompt=cwd:0\x07done!";
        let mut output = String::new();
        for byte in input.bytes() {
            output.push_str(&decoder.feed(&[byte], false).unwrap());
        }
        output.push_str(&decoder.feed(&[], true).unwrap());
        assert_eq!(output, "雪🌍done!");
        assert!(decoder.pending.is_empty() && decoder.utf8.is_empty());
    }
    /// Opens a native terminal with the same host-owned scheduler used in production.
    fn fixture() -> (Arc<NativePtyTerminalHost>, Arc<TerminalStreamExecutor>, String) {
        let host = Arc::new(NativePtyTerminalHost::default());
        let session = host
            .createOrGetSession(&uuid::Uuid::new_v4().to_string())
            .unwrap();
        let tools = TerminalStreamExecutor::new(
            StandardTerminalTools::new(Some(host.clone())),
            Some(Arc::new(
                operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
            )),
        );
        (host, tools, session.sessionId)
    }

    /// Checks yielding, session rotation, non-destructive waits and completion against a real PTY.
    #[tokio::test]
    async fn managed_commands_yield_without_interrupting_and_rotate_the_default_session() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let tools = TerminalStreamExecutor::new(
            StandardTerminalTools::new(Some(host.clone())),
            Some(Arc::new(
                operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
            )),
        );
        let name = uuid::Uuid::new_v4().to_string();
        let first = tools
            .runManaged(
                "owner".into(),
                name.clone(),
                Some("bash".into()), None,
                "printf 'starting\\n'; sleep 1; printf 'finished\\n'; (exit 7)".into(),
                20,
            )
            .await
            .unwrap();
        assert!(first.timedOut);
        assert!(
            host.getSessionScreen(&first.sessionId)
                .unwrap()
                .commandRunning
        );
        assert!(tools
            .waitManaged("other", &first.sessionId, 0)
            .await
            .is_err());
        assert!(
            tools
                .waitManaged("owner", &first.sessionId, 20)
                .await
                .unwrap()
                .timedOut
        );
        let second = tools
            .runManaged(
                "owner".into(),
                name.clone(),
                Some("bash".into()), None,
                "printf independent".into(),
                3000,
            )
            .await
            .unwrap();
        assert_ne!(first.sessionId, second.sessionId);
        assert!(!second.timedOut);
        assert!(second.output.contains("independent"));
        let completed = tools
            .waitManaged("owner", &first.sessionId, 3000)
            .await
            .unwrap();
        assert!(!completed.timedOut);
        assert_eq!(completed.exitCode, 7);
        assert!(completed.output.contains("finished"));
        let third = tools
            .runManaged(
                "owner".into(),
                name,
                Some("bash".into()), None,
                "printf reused".into(),
                3000,
            )
            .await
            .unwrap();
        assert_eq!(second.sessionId, third.sessionId);
        host.closeSession(&first.sessionId).unwrap();
        host.closeSession(&second.sessionId).unwrap();
    }

    /// Completed foreground yields reuse one shell and preserve its state after explicit acquisition.
    #[tokio::test]
    async fn repeated_yields_reuse_sessions_without_closing_acquired_shells() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let tools = TerminalStreamExecutor::new(
            StandardTerminalTools::new(Some(host.clone())),
            Some(Arc::new(operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new())),
        );
        let name = uuid::Uuid::new_v4().to_string();
        let mut sessionId = None;
        // More than the 16 service slots must reuse one persistent generation.
        for round in 0..20 {
            let command = if round == 0 {
                "export OPERIT_REUSE_STATE=kept; printf initialized".to_string()
            } else {
                "printf 'state:%s' \"$OPERIT_REUSE_STATE\"".to_string()
            };
            let yielded = tools.runManaged("owner".into(), name.clone(), Some("bash".into()), None, command, 0).await.unwrap();
            let result = tools.waitManaged("owner", &yielded.sessionId, 8000).await.unwrap();
            assert!(!result.timedOut);
            assert_eq!(result.exitCode, 0);
            if let Some(sessionId) = &sessionId {
                assert_eq!(&result.sessionId, sessionId);
                assert!(result.output.contains("state:kept"), "{}", result.output);
            }
            sessionId = Some(result.sessionId.clone());
            let acquired = tools.tools.availableSession(&name, "bash", &Default::default()).unwrap();
            assert_eq!(acquired.sessionId, result.sessionId);
            assert_eq!(host.listSessions().unwrap().len(), 1);
            assert!(tools.waitManaged("other", &result.sessionId, 0).await.is_err());
        }
        host.closeSession(&sessionId.unwrap()).unwrap();
    }

    /// Reuses an older idle generation while the newest one is reserved for queued work.
    #[test]
    fn older_idle_generation_is_reused_before_creating_another_shell() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let tools = StandardTerminalTools::new(Some(host.clone()));
        let name = uuid::Uuid::new_v4().to_string();
        let first = tools.availableSession(&name, "bash", &Default::default()).unwrap();
        let reserved = std::collections::BTreeSet::from([first.sessionId.clone()]);
        let second = tools.availableSession(&name, "bash", &reserved).unwrap();
        assert_ne!(first.sessionId, second.sessionId);
        let reserved = std::collections::BTreeSet::from([second.sessionId.clone()]);
        let reused = tools.availableSession(&name, "bash", &reserved).unwrap();
        assert_eq!(reused.sessionId, first.sessionId);
        assert!(!reused.isNewSession);
        host.closeSession(&first.sessionId).unwrap();
        host.closeSession(&second.sessionId).unwrap();
    }

    /// A closed background shell must release its host worker instead of waiting forever on silence.
    #[tokio::test]
    async fn managed_background_command_stops_when_its_session_is_closed() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let tools = TerminalStreamExecutor::new(
            StandardTerminalTools::new(Some(host.clone())),
            Some(Arc::new(
                operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
            )),
        );
        let run = tools
            .runManaged(
                "owner".into(),
                uuid::Uuid::new_v4().to_string(),
                Some("bash".into()), None,
                "sleep 30".into(),
                0,
            )
            .await
            .unwrap();
        assert!(run.timedOut);
        host.closeSession(&run.sessionId).unwrap();
        assert!(tools
            .waitManaged("owner", &run.sessionId, 3000)
            .await
            .is_err());
        assert_eq!(tools.workers.load(Ordering::Acquire), 0);
    }
    async fn idle(host: &dyn TerminalHost, session: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while host.getSessionScreen(session).unwrap().commandRunning {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("canceled command must release its session");
    }
    #[tokio::test]
    async fn terminal_stream_delivers_output_before_exit_and_preserves_session_state() {
        let (host, tools, session) = fixture();
        let command = "printf 'first\\n'; sleep 0.4; cd /tmp; export OPERIT_STREAM_STATE=kept; printf '雪🌍\\n'; (exit 7)";
        let handle = tools
            .open(
                "owner".into(),
                TerminalExecRequest {
                    sessionId: session.clone(),
                    command: command.into(),
                    timeoutMs: Some(5000),
                },
            )
            .unwrap();
        assert!(tools.owned("other", &handle).is_err());
        let mut sawLive = false;
        let mut chunks = String::new();
        let result = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match tools
                    .next("owner", &handle)
                    .await
                    .unwrap()
                    .expect("completion receipt")
                {
                    TerminalExecEvent::Start {
                        platform,
                        terminal,
                        terminalType,
                        ..
                    } => {
                        assert!(!platform.is_empty());
                        assert_eq!(terminal.as_str(), "native");
                        assert_eq!(terminalType.as_str(), "bash");
                    }
                    TerminalExecEvent::Chunk {
                        chunk,
                        platform,
                        terminal,
                        terminalType,
                        ..
                    } => {
                        assert!(!platform.is_empty());
                        assert_eq!(terminal.as_str(), "native");
                        assert_eq!(terminalType.as_str(), "bash");
                        if chunk.contains("first\r\n") || chunk.contains("first\n") {
                            assert!(tools
                                .owned("owner", &handle)
                                .unwrap()
                                .inner
                                .lock()
                                .unwrap()
                                .result
                                .is_none());
                            sawLive = true;
                        }
                        chunks.push_str(&chunk);
                    }
                    TerminalExecEvent::Completed { result } => break result,
                }
            }
        })
        .await
        .unwrap();
        assert!(sawLive, "actual output must precede process completion");
        assert!(chunks.contains("雪🌍"));
        assert!(!chunks.contains("\x1b]133;OperitPrompt="));
        assert_eq!(result.exitCode, 7);
        assert!(result.output.contains("雪🌍"));
        tools.close("owner", &handle).unwrap();
        let next = host
            .executeInSession(
                &session,
                "printf '%s:%s' \"$PWD\" \"$OPERIT_STREAM_STATE\"",
                3000,
            )
            .unwrap();
        assert!(next.output.contains("/tmp:kept"), "{}", next.output);
        host.closeSession(&session).unwrap();
    }
    #[tokio::test]
    async fn terminal_stream_detach_keeps_commands_running_and_wakes_pending_next() {
        let (host, tools, session) = fixture();
        let handle = tools
            .open(
                "owner".into(),
                TerminalExecRequest {
                    sessionId: session.clone(),
                    command: "sleep 10".into(),
                    timeoutMs: Some(15000),
                },
            )
            .unwrap();
        let observation = tools.owned("owner", &handle).unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while !host.getSessionScreen(&session).unwrap().commandRunning {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(tools.close("other", &handle).is_err());
        assert!(host
            .executeInSession(&session, "printf conflicting", 1000)
            .is_err());
        loop {
            let pending = tools.next("owner", &handle);
            tokio::pin!(pending);
            tokio::select! {
                event = &mut pending => { assert!(event.unwrap().is_some()); }
                _ = tokio::time::sleep(Duration::from_millis(30)) => {
                    tools.close("owner", &handle).unwrap();
                    assert!(pending.await.unwrap().is_none());
                    break;
                }
            }
        }
        assert!(observation.detached.load(Ordering::Acquire));
        assert!(host.getSessionScreen(&session).unwrap().commandRunning);
        host.closeSession(&session).unwrap();
    }
    #[tokio::test]
    async fn abandoned_terminal_output_is_bounded_without_interrupting_the_command() {
        let (host, tools, session) = fixture();
        let handle = tools
            .open(
                "owner".into(),
                TerminalExecRequest {
                    sessionId: session.clone(),
                    command: "yes stream-limit | head -c 2097152".into(),
                    timeoutMs: Some(5000),
                },
            )
            .unwrap();
        let observation = tools.owned("owner", &handle).unwrap();
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if observation.inner.lock().unwrap().result.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        {
            let inner = observation.inner.lock().unwrap();
            assert!(inner.queuedBytes <= MAX_QUEUED_BYTES);
            assert!(inner.result.as_ref().unwrap().is_ok());
            assert_eq!(inner.receipt.as_ref().unwrap().exitCode, 0);
        }
        tools.close("owner", &handle).unwrap();
        idle(host.as_ref(), &session).await;
        assert!(
            !host
                .executeInSession(&session, "printf after-overflow", 3000)
                .unwrap()
                .timedOut
        );
        host.closeSession(&session).unwrap();
    }

    #[tokio::test]
    async fn terminal_stream_timeout_returns_its_receipt_and_keeps_the_session() {
        let (host, tools, session) = fixture();
        let handle = tools
            .open(
                "owner".into(),
                TerminalExecRequest {
                    sessionId: session.clone(),
                    command: "sleep 5".into(),
                    timeoutMs: Some(100),
                },
            )
            .unwrap();
        let receipt = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(TerminalExecEvent::Completed { result }) =
                    tools.next("owner", &handle).await.unwrap()
                {
                    break result;
                }
            }
        })
        .await
        .unwrap();
        assert!(receipt.timedOut);
        tools.close("owner", &handle).unwrap();
        assert!(host.getSessionScreen(&session).unwrap().commandRunning);
        let completed = tools.waitManaged("owner", &session, 6000).await.unwrap();
        assert!(!completed.timedOut);
        assert_eq!(completed.exitCode, 0);
        host.closeSession(&session).unwrap();
    }
}
