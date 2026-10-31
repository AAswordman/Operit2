//! Concurrent, owner-scoped terminal observations over the common tool transport.
#![allow(non_snake_case)]

use super::StandardTerminalTools::terminalCommandResultData;
use super::ToolArguments::decodeArguments;
use crate::tools::ToolInvocation::prepareInvocation;
use crate::ToolExecutionManager::{
    PreparedToolInvocation, SharedToolExecutor, ToolCall, ToolEffect, ToolInvocationContext,
};
use operit_host_api::{HostError, HostResult, HostRuntimeTaskSchedulerHost, TerminalHost};
use operit_plugin_sdk::js_sdk::results::{
    JsNullable, TerminalImplementation, TerminalObservationResultData, TerminalType, ToolResultData,
};
use operit_plugin_sdk::js_sdk::system::{TerminalExecEvent, TerminalExecRequest};
use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

const MAX_OBSERVATIONS: usize = 32;
const MAX_QUEUED_BYTES: usize = 1024 * 1024;
const MAX_TOTAL_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

#[derive(serde::Deserialize)]
#[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)]
enum Operation {
    Open { request: TerminalExecRequest },
    Next { handle: String },
    Close { handle: String },
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    observation: Operation,
}

pub(crate) struct TerminalStreamExecutor {
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
    canceled: AtomicBool,
    pulling: AtomicBool,
    ready: tokio::sync::Notify,
    inner: Mutex<ObservationState>,
}
struct ObservationState {
    events: VecDeque<TerminalExecEvent>,
    queuedBytes: usize,
    result: Option<Result<(), String>>,
    completedAt: Option<u64>,
}
impl TerminalStreamExecutor {
    /// Binds terminal execution to the host-owned blocking task scheduler.
    pub(crate) fn new(
        host: Option<Arc<dyn TerminalHost>>,
        scheduler: Option<Arc<dyn HostRuntimeTaskSchedulerHost>>,
    ) -> Self {
        Self {
            host,
            scheduler,
            observations: Mutex::new(HashMap::new()),
            workers: Arc::new(AtomicUsize::new(0)),
        }
    }
    /// Validates one observation and schedules its command through the registered host.
    fn open(&self, owner: String, request: TerminalExecRequest) -> Result<String, String> {
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
        let session = host
            .listSessions()
            .map_err(|error| error.message)?
            .into_iter()
            .find(|session| session.sessionId == request.sessionId)
            .ok_or("Terminal session does not exist")?;
        let terminal = TerminalImplementation::try_from(session.terminal.as_str())?;
        let terminalType = TerminalType::try_from(session.terminalType.as_str())?;
        let observation = Arc::new(Observation {
            owner,
            canceled: AtomicBool::new(false),
            pulling: AtomicBool::new(false),
            ready: tokio::sync::Notify::new(),
            inner: Mutex::new(ObservationState {
                events: VecDeque::from([TerminalExecEvent::Start {
                    sessionId: request.sessionId.clone(),
                    command: request.command.clone(),
                    platform: session.platform.clone(),
                    terminal: terminal.clone(),
                    terminalType: terminalType.clone(),
                }]),
                queuedBytes: 0,
                result: None,
                completedAt: None,
            }),
        });
        let handle = uuid::Uuid::new_v4().to_string();
        let now = scheduler.monotonicTimeMillis().map_err(|error| error.message)?;
        let permit = {
            let mut registry = self
                .observations
                .lock()
                .map_err(|_| "Terminal observation registry is poisoned")?;
            registry.retain(|_, value| {
                value.inner.lock().map_or(false, |inner| {
                    !inner
                        .completedAt
                        .is_some_and(|at| now.saturating_sub(at) > 60_000)
                })
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
        let taskScheduler = Arc::clone(scheduler);
        let scheduled = scheduler.scheduleHostRuntimeTask(
            "terminal-command-stream",
            Box::new(move || {
                let _permit = permit;
                let mut totalBytes = 0usize;
                let mut decoder = OutputDecoder::default();
                let mut chunkIndex = 0;
                let mut receivedChars = 0;
                let mut deliver = |data: &[u8], finalFrame: bool| -> HostResult<()> {
                    if observation.canceled.load(Ordering::Acquire) {
                        return Err(HostError::new("Terminal execution canceled"));
                    }
                    totalBytes = totalBytes.saturating_add(data.len());
                    if totalBytes > MAX_TOTAL_OUTPUT_BYTES {
                        return Err(HostError::new(
                            "Terminal stream total output limit exceeded",
                        ));
                    }
                    let text = decoder.feed(data, finalFrame)?;
                    if !text.is_empty() {
                        let mut inner = observation
                            .inner
                            .lock()
                            .map_err(|_| HostError::new("Terminal observation is poisoned"))?;
                        let cost = std::mem::size_of::<TerminalExecEvent>()
                            + request.sessionId.len()
                            + request.command.len()
                            + session.platform.len()
                            + text.len();
                        if inner.queuedBytes + cost > MAX_QUEUED_BYTES {
                            return Err(HostError::new(
                                "Terminal stream output buffer limit exceeded",
                            ));
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
                };
                let result = host.executeInSessionStreaming(
                    &request.sessionId,
                    &request.command,
                    request.timeoutMs.unwrap_or(1_800_000),
                    &mut |data| deliver(data, false),
                );
                let result = result.and_then(|receipt| {
                    deliver(&[], true)?;
                    Ok(receipt)
                });
                if let Ok(mut inner) = observation.inner.lock() {
                    let result = taskScheduler.monotonicTimeMillis().and_then(|at| {
                        inner.completedAt = Some(at);
                        result.and_then(|receipt| terminalCommandResultData(&receipt))
                    });
                    inner.result = Some(match result {
                        Ok(receipt) => {
                            inner.events.push_back(TerminalExecEvent::Completed {
                                result: receipt,
                            });
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
        loop {
            let notified = observation.ready.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut inner = observation
                    .inner
                    .lock()
                    .map_err(|_| "Terminal observation is poisoned")?;
                if observation.canceled.load(Ordering::Acquire) {
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
            notified.await;
        }
    }
    fn close(&self, owner: &str, handle: &str) -> Result<(), String> {
        let observation = self.owned(owner, handle)?;
        observation.canceled.store(true, Ordering::Release);
        observation.ready.notify_waiters();
        self.observations
            .lock()
            .map_err(|_| "Terminal observation registry is poisoned")?
            .remove(handle);
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
        let owner = context.requirePluginOwner()?;
        let effect = if matches!(&arguments.observation, Operation::Next { .. }) {
            ToolEffect::READ
        } else {
            // Closing an unfinished stream interrupts its command.
            ToolEffect::WRITE
        };
        Ok(prepareInvocation(
            request,
            checkAccess,
            effect,
            async move {
                let result = match arguments.observation {
                    Operation::Open { request } => TerminalObservationResultData::Open {
                        handle: self.open(owner, request)?,
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

/// Holds split UTF-8 and private completion markers until their boundaries are known.
#[derive(Default)]
struct OutputDecoder {
    pending: Vec<u8>,
    utf8: Vec<u8>,
}
impl OutputDecoder {
    fn feed(&mut self, bytes: &[u8], finalFrame: bool) -> HostResult<String> {
        const MARKERS: &[(&[u8], u8)] =
            &[(b"\x1b]133;OperitPrompt=", 7), (b"\x1e__OPERIT_ISH_", 31)];
        self.pending.extend_from_slice(bytes);
        let mut used = 0;
        while used < self.pending.len() {
            let tail = &self.pending[used..];
            if let Some((prefix, end)) = MARKERS.iter().find(|(prefix, _)| tail.starts_with(prefix))
            {
                if let Some(length) = tail[prefix.len()..].iter().position(|byte| byte == end) {
                    used += prefix.len() + length + 1;
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
            if !finalFrame && MARKERS.iter().any(|(prefix, _)| prefix.starts_with(tail)) {
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

    #[test]
    fn output_decoder_preserves_split_unicode_and_hides_split_native_markers() {
        let mut decoder = OutputDecoder::default();
        let input = "雪🌍\x1b]133;OperitPrompt=cwd:0\x07done\x1e__OPERIT_ISH_uuid__:0\x1f!";
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
            Some(host.clone()),
            Some(Arc::new(
                operit_host_native_scheduler::NativeHostRuntimeTaskSchedulerHost::new(),
            )),
        );
        (host, tools, session.sessionId)
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
    async fn terminal_stream_close_interrupts_quiet_commands_and_wakes_pending_next() {
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
        idle(host.as_ref(), &session).await;
        assert!(observation.canceled.load(Ordering::Acquire));
        assert!(
            !host
                .executeInSession(&session, "printf reused", 3000)
                .unwrap()
                .timedOut
        );
        host.closeSession(&session).unwrap();
    }
    #[tokio::test]
    async fn abandoned_terminal_output_is_bounded_and_interrupts_the_command() {
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
            assert!(inner
                .result
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap_err()
                .contains("buffer limit"));
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
        assert!(host
            .executeInSession(&session, "printf after-timeout", 3000)
            .unwrap()
            .output
            .contains("after-timeout"));
        host.closeSession(&session).unwrap();
    }
}
