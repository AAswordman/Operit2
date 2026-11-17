//! Concurrent, owner-scoped terminal observations over the common tool transport.
#![allow(non_snake_case)]

use super::StandardTerminalTools::{terminalCommandResultData, StandardTerminalTools};
use super::ToolArguments::decodeArguments;
use crate::tools::ToolInvocation::prepareInvocation;
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};
use operit_host_api::{HostError, HostResult, HostRuntimeTaskSchedulerHost, TerminalHost};
use operit_plugin_sdk::js_sdk::results::{
    JsNullable, TerminalCommandResultData, TerminalImplementation, TerminalObservationResultData,
    TerminalType, ToolResultData,
};
use operit_plugin_sdk::js_sdk::system::{TerminalExecEvent, TerminalExecRequest};
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
        terminalType: String,
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
    managedAdmission: tokio::sync::Mutex<()>,
    host: Option<Arc<dyn TerminalHost>>,
    scheduler: Option<Arc<dyn HostRuntimeTaskSchedulerHost>>,
    observations: Mutex<HashMap<String, Arc<Observation>>>,
    receipts: Arc<Mutex<ReceiptLog>>,
    workers: Arc<AtomicUsize>,
}
// A disposed handle still counts until its native command actually stops.
struct WorkerPermit(Arc<AtomicUsize>);
impl Drop for WorkerPermit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
/// Completion receipts retained after their observation or session is retired.
#[derive(Default)]
struct ReceiptLog {
    entries: VecDeque<ReceiptEntry>,
    evicted: VecDeque<String>,
}
struct ReceiptEntry {
    key: String,
    receipt: TerminalCommandResultData,
}
/// Combines the owner scope with the session a receipt was issued for.
fn completionKey(owner: &str, sessionId: &str) -> String {
    format!("{owner}\u{1}{sessionId}")
}
/// Stores one completion receipt, evicting the oldest entry beyond the bound.
fn recordCompletion(
    log: &Mutex<ReceiptLog>,
    owner: &str,
    sessionId: &str,
    receipt: TerminalCommandResultData,
) {
    let Ok(mut log) = log.lock() else {
        return;
    };
    let key = completionKey(owner, sessionId);
    log.entries.retain(|entry| entry.key != key);
    log.entries.push_back(ReceiptEntry { key, receipt });
    while log.entries.len() > MAX_OBSERVATIONS {
        let Some(evicted) = log.entries.pop_front() else {
            break;
        };
        log.evicted.push_back(evicted.key);
        while log.evicted.len() > MAX_OBSERVATIONS {
            log.evicted.pop_front();
        }
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
    /// The managed tool lineage created this session generation and may retire it once idle.
    autoOwned: bool,
    detached: AtomicBool,
    deadline: Option<u64>,
    metadata: TerminalCommandResultData,
    yielded: AtomicBool,
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
        let host = self.host.as_ref().ok_or("Terminal host is unavailable")?;
        let session = host
            .listSessions()
            .map_err(|error| error.message)?
            .into_iter()
            .find(|session| session.sessionId == request.sessionId)
            .ok_or("Terminal session does not exist")?;
        self.startNamed(
            owner,
            session.sessionName,
            session.terminalType,
            request.command,
            request.timeoutMs.unwrap_or(DEFAULT_WAIT_MS),
            false,
        )
        .await
    }

    /// Binds terminal execution to the host-owned blocking task scheduler.
    pub(crate) fn new(
        tools: StandardTerminalTools,
        scheduler: Option<Arc<dyn HostRuntimeTaskSchedulerHost>>,
    ) -> Self {
        Self {
            host: tools.terminalHost.clone(),
            tools,
            managedAdmission: tokio::sync::Mutex::new(()),
            scheduler,
            observations: Mutex::new(HashMap::new()),
            receipts: Arc::new(Mutex::new(ReceiptLog::default())),
            workers: Arc::new(AtomicUsize::new(0)),
        }
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
        autoOwned: bool,
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
            autoOwned,
            detached: AtomicBool::new(false),
            deadline: if receiptOnly {
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
            yielded: AtomicBool::new(false),
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
        let now = scheduler
            .monotonicTimeMillis()
            .map_err(|error| error.message)?;
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
        let outputHost = host.clone();
        let outputRequest = request.clone();
        let outputObservation = observation.clone();
        let outputScheduler = scheduler.clone();
        let onOutput = {
            let mut decoder = OutputDecoder::default();
            let mut chunkIndex = 0;
            let mut receivedChars = 0;
            let mut lastSessionCheck = now;
            let host = outputHost;
            let request = outputRequest;
            let observation = outputObservation;
            Box::new(move |data: &[u8], finalFrame: bool| -> HostResult<()> {
                let at = outputScheduler.monotonicTimeMillis()?;
                if at.saturating_sub(lastSessionCheck) >= 1000 {
                    if !host
                        .listSessions()?
                        .iter()
                        .any(|session| session.sessionId == request.sessionId)
                    {
                        return Err(HostError::new("Terminal session was closed"));
                    }
                    lastSessionCheck = at;
                }
                observation.ready.notify_waiters();
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
                    let cost = std::mem::size_of::<TerminalExecEvent>()
                        + request.sessionId.len()
                        + request.command.len()
                        + session.platform.len()
                        + text.len();
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
                    receivedChars += text.encode_utf16().count() as u64;
                    inner.queuedBytes += cost;
                    inner.events.push_back(TerminalExecEvent::Chunk {
                        sessionId: request.sessionId.clone(),
                        command: request.command.clone(),
                        platform: session.platform.clone(),
                        terminal: terminal.clone(),
                        terminalType: terminalType.clone(),
                        chunk: text,
                        chunkIndex,
                        receivedChars,
                    });
                    chunkIndex += 1;
                    observation.ready.notify_one();
                }
                Ok(())
            }) as operit_host_api::TerminalOutputCallback
        };
        let completionReceipts = self.receipts.clone();
        let scheduled = host.scheduleInSessionStreaming(
            scheduler.clone(),
            request.sessionId,
            request.command,
            0,
            onOutput,
            Box::new(move |result| {
                drop(permit);
                if let Ok(mut inner) = observation.inner.lock() {
                    let result = result.and_then(|receipt| terminalCommandResultData(&receipt));
                    inner.result = Some(match result {
                        Ok(receipt) => {
                            // The bounded cache keeps this receipt retrievable after the
                            // observation or its whole session generation is retired.
                            recordCompletion(
                                &completionReceipts,
                                &observation.owner,
                                &observation.sessionId,
                                receipt.clone(),
                            );
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
        terminalType: String,
        command: String,
        waitMs: u64,
    ) -> Result<TerminalCommandResultData, String> {
        let handle = self
            .startNamed(owner.clone(), name, terminalType, command, waitMs, true)
            .await?;
        let observation = self.owned(&owner, &handle)?;
        self.waitManaged(&owner, &observation.sessionId, waitMs)
            .await
    }

    /// Retires tool-created generations whose command completed after the caller
    /// yielded or detached. Their receipts stay available through the bounded cache,
    /// while running commands and user-held sessions are never touched here.
    fn retireCompletedToolSessions(&self) -> Result<(), String> {
        let retired: Vec<String> = {
            let mut registry = self
                .observations
                .lock()
                .map_err(|_| "Terminal observation registry is poisoned")?;
            let mut retired = Vec::new();
            registry.retain(|_, value| {
                let stale = value.autoOwned
                    && (value.yielded.load(Ordering::Acquire)
                        || value.detached.load(Ordering::Acquire))
                    && value
                        .inner
                        .lock()
                        .map(|inner| inner.result.is_some())
                        .unwrap_or(false);
                if stale {
                    retired.push(value.sessionId.clone());
                }
                !stale
            });
            retired
        };
        // Closing a session terminates its process; that must stay outside every registry lock.
        for sessionId in &retired {
            if let Some(host) = self.host.as_ref() {
                let _ = host.closeSession(sessionId);
            }
            self.tools.releaseManagedSession(sessionId);
        }
        Ok(())
    }

    /// Reserves pending and running commands before releasing admission to the next caller.
    async fn startNamed(
        &self,
        owner: String,
        name: String,
        terminalType: String,
        command: String,
        waitMs: u64,
        receiptOnly: bool,
    ) -> Result<String, String> {
        let _admission = self.managedAdmission.lock().await;
        self.retireCompletedToolSessions()?;
        let tools = &self.tools;
        let mut reserved = std::collections::BTreeSet::new();
        for value in self
            .observations
            .lock()
            .map_err(|_| "Terminal observation registry is poisoned")?
            .values()
        {
            // Only a pending or running command keeps its session reserved; a
            // completed one leaves the generation idle and reusable.
            if value
                .inner
                .lock()
                .map_err(|_| "Terminal observation is poisoned")?
                .result
                .is_none()
            {
                reserved.insert(value.sessionId.clone());
            }
        }
        let selection = tools
            .availableSession(&name, &terminalType, &reserved, true)
            .map_err(|error| error.message)?;
        let session = selection.session;
        let opened = self.openExecution(
            owner.clone(),
            TerminalExecRequest {
                sessionId: session.sessionId.clone(),
                command,
                timeoutMs: Some(waitMs),
            },
            receiptOnly,
            selection.toolOwned,
        );
        let handle = match opened {
            Ok(handle) => handle,
            Err(error) => {
                if session.isNewSession {
                    self.host
                        .as_ref()
                        .ok_or("Terminal host is unavailable")?
                        .closeSession(&session.sessionId)
                        .map_err(|close| {
                            format!("{error}; closing unused session failed: {}", close.message)
                        })?;
                    self.tools.releaseManagedSession(&session.sessionId);
                }
                return Err(error);
            }
        };
        Ok(handle)
    }

    /// Runs an existing session through the same managed admission and generation rules.
    async fn runExisting(
        &self,
        owner: String,
        request: TerminalExecRequest,
    ) -> Result<TerminalCommandResultData, String> {
        let session = self
            .host
            .as_ref()
            .ok_or("Terminal host is unavailable")?
            .listSessions()
            .map_err(|error| error.message)?
            .into_iter()
            .find(|session| session.sessionId == request.sessionId)
            .ok_or("Terminal session does not exist")?;
        self.runManaged(
            owner,
            session.sessionName,
            session.terminalType,
            request.command,
            request.timeoutMs.unwrap_or(DEFAULT_WAIT_MS),
        )
        .await
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
            .cloned();
        let Some(observation) = observation else {
            // The generation may already be retired; its receipt survives in the cache.
            return self.cachedReceipt(owner, sessionId);
        };
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or("Runtime task scheduler host is unavailable")?;
        let deadline = scheduler
            .monotonicTimeMillis()
            .map_err(|error| error.message)?
            .checked_add(waitMs)
            .ok_or("Terminal wait deadline overflow")?;
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
                if scheduler
                    .monotonicTimeMillis()
                    .map_err(|error| error.message)?
                    >= deadline
                {
                    observation.yielded.store(true, Ordering::Release);
                    let mut result = observation.metadata.clone();
                    result.output = inner.output.clone();
                    return Ok(result);
                }
            }
            tokio::select! {
                _ = ready => {},
                result = scheduler.waitForHostRuntimeDelay(50) => { result.map_err(|error| error.message)?; }
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
    /// Returns a retired command's receipt, or a precise error once it is gone.
    fn cachedReceipt(&self, owner: &str, sessionId: &str) -> Result<TerminalCommandResultData, String> {
        let key = completionKey(owner, sessionId);
        let log = self
            .receipts
            .lock()
            .map_err(|_| "Terminal completion log is poisoned")?;
        if let Some(entry) = log.entries.iter().rev().find(|entry| entry.key == key) {
            return Ok(entry.receipt.clone());
        }
        if log.evicted.iter().any(|evicted| evicted == &key) {
            return Err(format!(
                "Terminal result for session {sessionId} has been evicted from the completion cache; only the {MAX_OBSERVATIONS} most recent results are kept. Rerun the command to capture fresh output"
            ));
        }
        Err(format!(
            "No completed terminal command is recorded for session {sessionId}"
        ))
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
        loop {
            let notified = observation.ready.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let scheduler = self
                .scheduler
                .as_ref()
                .ok_or("Runtime task scheduler host is unavailable")?;
            let deadline = observation
                .deadline
                .ok_or("Terminal stream has no foreground deadline")?;
            let expired = scheduler
                .monotonicTimeMillis()
                .map_err(|error| error.message)?
                >= deadline;
            if expired
                && observation
                    .inner
                    .lock()
                    .map_err(|_| "Terminal observation is poisoned")?
                    .result
                    .is_none()
            {
                observation.yielded.store(true, Ordering::Release);
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
            tokio::select! {
                _ = notified => {},
                result = scheduler.waitForHostRuntimeDelay(50) => { result.map_err(|error| error.message)?; }
            }
        }
    }
    fn close(&self, owner: &str, handle: &str) -> Result<(), String> {
        let observation = self.owned(owner, handle)?;
        observation.detached.store(true, Ordering::Release);
        if observation
            .inner
            .lock()
            .map_err(|_| "Terminal observation is poisoned")?
            .result
            .is_none()
        {
            observation.yielded.store(true, Ordering::Release);
        }
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
                        command,
                        waitMs,
                    } => TerminalObservationResultData::Run {
                        result: self
                            .runManaged(owner, sessionName, terminalType, command, waitMs)
                            .await?,
                    },
                    Operation::Wait { sessionId, waitMs } => TerminalObservationResultData::Run {
                        result: self.waitManaged(&owner, &sessionId, waitMs).await?,
                    },
                    Operation::Open { request } => TerminalObservationResultData::Open {
                        handle: self.openStream(owner, request).await?,
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
    }
    impl HostRuntimeTaskSchedulerHost for QueuedScheduler {
        /// Reads the production Host clock.
        fn monotonicTimeMillis(&self) -> HostResult<u64> {
            self.clock.monotonicTimeMillis()
        }
        /// Models a saturated blocking pool without starting its queued commands.
        fn scheduleHostRuntimeTask(
            &self,
            _: &str,
            task: operit_host_api::HostRuntimeTask,
        ) -> HostResult<()> {
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
        /// Keeps cooperative coordination on its normal executor.
        fn scheduleHostRuntimeCooperativeAsyncTask(
            &self,
            name: &str,
            task: operit_host_api::HostRuntimeAsyncTask,
        ) -> HostResult<()> {
            self.clock
                .scheduleHostRuntimeCooperativeAsyncTask(name, task)
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
        /// Preserves production event-loop turns.
        fn waitForHostRuntimeTaskTurn(&self) -> operit_host_api::HostRuntimeTurnFuture {
            self.clock.waitForHostRuntimeTaskTurn()
        }
        /// Preserves production nonblocking delays.
        fn waitForHostRuntimeDelay(&self, delay: u64) -> operit_host_api::HostRuntimeTurnFuture {
            self.clock.waitForHostRuntimeDelay(delay)
        }
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
                    "bash".into(),
                    "printf first".into(),
                    20,
                )
                .await
                .unwrap();
            let second = tools
                .runManaged(
                    "owner".into(),
                    name,
                    "bash".into(),
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
    fn fixture() -> (Arc<NativePtyTerminalHost>, TerminalStreamExecutor, String) {
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
                "bash".into(),
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
                "bash".into(),
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
                "bash".into(),
                "printf reused".into(),
                3000,
            )
            .await
            .unwrap();
        assert_eq!(second.sessionId, third.sessionId);
        // The yielded generation finished before this admission, so the managed
        // path retired and closed it itself; its receipt survives in the cache.
        assert!(!host
            .listSessions()
            .unwrap()
            .iter()
            .any(|session| session.sessionId == first.sessionId));
        let retired = tools.waitManaged("owner", &first.sessionId, 0).await.unwrap();
        assert_eq!(retired.exitCode, completed.exitCode);
        host.closeSession(&second.sessionId).unwrap();
    }

    /// Repeated foreground yields must not accumulate idle tool generations, while
    /// retired results stay retrievable through the bounded completion cache.
    #[tokio::test]
    async fn repeated_foreground_yields_retire_generations_and_keep_bounded_receipts() {
        let host = Arc::new(NativePtyTerminalHost::default());
        let tools = TerminalStreamExecutor::new(
            StandardTerminalTools::new(Some(host.clone())),
            Some(Arc::new(
                operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
            )),
        );
        let name = uuid::Uuid::new_v4().to_string();
        let manual = host.createOrGetSession(&format!("{name}-manual")).unwrap();
        let prefix = format!("{name}_generation_");
        let mut previous: Option<TerminalCommandResultData> = None;
        // More rounds than the shared Host service capacity (16) proves retirement,
        // not an observation bound, keeps the idle generation count flat.
        for round in 0..20 {
            let exitCode = (round % 5) as i32;
            let command = format!("printf round{round}; sleep 0.05; (exit {exitCode})");
            let yielded = tools
                .runManaged("owner".into(), name.clone(), "bash".into(), command, 20)
                .await
                .unwrap();
            assert!(yielded.timedOut, "round {round} must yield its foreground wait");
            tokio::time::timeout(Duration::from_secs(2), async {
                while !host
                    .getSessionScreen(&yielded.sessionId)
                    .unwrap()
                    .commandRunning
                {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .expect("scheduled command must start before its session is reused");
            let completed = tools
                .waitManaged("owner", &yielded.sessionId, 8000)
                .await
                .unwrap();
            assert!(!completed.timedOut);
            assert_eq!(completed.exitCode, exitCode, "round {round}");
            if let Some(previous) = &previous {
                let cached = tools
                    .waitManaged("owner", &previous.sessionId, 0)
                    .await
                    .unwrap();
                assert_eq!(cached.exitCode, previous.exitCode);
                let isolated = tools
                    .waitManaged("other", &previous.sessionId, 0)
                    .await
                    .unwrap_err();
                assert!(
                    isolated.contains("No completed terminal command"),
                    "{isolated}"
                );
            }
            previous = Some(completed);
            let generations = host
                .listSessions()
                .unwrap()
                .into_iter()
                .filter(|session| {
                    session.sessionName == name || session.sessionName.starts_with(&prefix)
                })
                .count();
            assert!(
                generations <= 2,
                "round {round} left {generations} idle generations"
            );
            assert!(host
                .listSessions()
                .unwrap()
                .iter()
                .any(|session| session.sessionId == manual.sessionId));
        }
        // A manually held PTY is never auto-managed and must outlive every round.
        assert!(host
            .listSessions()
            .unwrap()
            .iter()
            .any(|session| session.sessionId == manual.sessionId));
        host.closeSession(&manual.sessionId).unwrap();
    }

    /// The completion cache keeps only the newest bounded receipts and reports eviction.
    #[tokio::test]
    async fn completion_receipts_are_bounded_owner_scoped_and_report_eviction() {
        let tools = TerminalStreamExecutor::new(StandardTerminalTools::new(None), None);
        let terminal = TerminalImplementation::try_from("native").unwrap();
        let terminalType = TerminalType::try_from("bash").unwrap();
        for index in 0..(MAX_OBSERVATIONS + 2) {
            let sessionId = format!("session-{index}");
            recordCompletion(
                &tools.receipts,
                "owner",
                &sessionId,
                TerminalCommandResultData {
                    sessionId: sessionId.clone(),
                    command: format!("printf {index}"),
                    platform: "posix".into(),
                    terminal: terminal.clone(),
                    terminalType: terminalType.clone(),
                    output: format!("round {index}"),
                    exitCode: index as i32,
                    timedOut: false,
                },
            );
        }
        let latest = tools
            .waitManaged("owner", &format!("session-{}", MAX_OBSERVATIONS + 1), 0)
            .await
            .unwrap();
        assert_eq!(latest.exitCode, (MAX_OBSERVATIONS + 1) as i32);
        let retained = tools
            .waitManaged("owner", &format!("session-{}", MAX_OBSERVATIONS - 1), 0)
            .await
            .unwrap();
        assert_eq!(retained.exitCode, (MAX_OBSERVATIONS - 1) as i32);
        let evicted = tools.waitManaged("owner", "session-0", 0).await.unwrap_err();
        assert!(evicted.contains("evicted"), "{evicted}");
        let isolated = tools
            .waitManaged("other", &format!("session-{}", MAX_OBSERVATIONS + 1), 0)
            .await
            .unwrap_err();
        assert!(
            isolated.contains("No completed terminal command"),
            "{isolated}"
        );
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
                "bash".into(),
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
