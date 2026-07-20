use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::{
    HostRuntimeTaskSchedulerHost, TerminalHost, TerminalInfo, TerminalSessionListEntry,
    TerminalTypeInfo,
};
use operit_store::{
    PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow},
    RuntimeStorePaths::RuntimeStorePaths,
};
use serde::{Deserialize, Serialize};

use operit_host_api::HostManager::HostManager;
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::files::VisualFileSystem::VisualFileSystem;
use operit_util::stream::HotStream::MutableSharedStreamImpl;
use operit_util::stream::Stream::{CollectFuture, Stream};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Published terminal session metadata for UI session lists.
pub struct RuntimeTerminalSessionInfo {
    pub sessionId: String,
    pub sessionName: String,
    pub terminalType: String,
    pub sessionKind: String,
    pub workingDir: String,
    pub commandRunning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Current terminal screen snapshot returned by the host.
pub struct RuntimeTerminalScreen {
    pub sessionId: String,
    pub terminalType: String,
    pub rows: i32,
    pub cols: i32,
    pub content: String,
    pub commandRunning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Describes one terminal type that may be created by the active host.
pub struct RuntimeTerminalTypeInfo {
    pub terminalType: String,
    pub available: bool,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// Describes all terminal types exposed by the active host.
pub struct RuntimeTerminalInfo {
    pub platform: String,
    pub defaultType: String,
    pub types: Vec<RuntimeTerminalTypeInfo>,
}

/// Facade over host terminal APIs and shared terminal output streams.
pub struct RuntimeTerminalService {
    terminalHost: Arc<dyn TerminalHost>,
    context: HostManager,
}

#[derive(Clone, Debug)]
/// Stream wrapper exposing PTY output to Kotlin-style collectors.
pub struct RuntimeTerminalPtyOutputStream {
    upstream: MutableSharedStreamImpl<String>,
}

impl RuntimeTerminalPtyOutputStream {
    /// Wraps a shared PTY output stream.
    pub fn new(upstream: MutableSharedStreamImpl<String>) -> Self {
        Self { upstream }
    }
}

impl Stream for RuntimeTerminalPtyOutputStream {
    type Item = String;

    /// Collects PTY output asynchronously until the host closes the stream.
    fn collect<'a>(&'a mut self, collector: &'a mut dyn FnMut(Self::Item)) -> CollectFuture<'a> {
        self.upstream.collect(collector)
    }
}

#[derive(Clone)]
struct TerminalPtyOutputEntry {
    stream: MutableSharedStreamImpl<String>,
}

static TERMINAL_PTY_OUTPUT_STREAMS: OnceLock<Mutex<HashMap<String, TerminalPtyOutputEntry>>> =
    OnceLock::new();
static TERMINAL_SESSIONS_FLOW: OnceLock<MutableStateFlow<Vec<RuntimeTerminalSessionInfo>>> =
    OnceLock::new();

fn terminal_pty_output_streams() -> &'static Mutex<HashMap<String, TerminalPtyOutputEntry>> {
    TERMINAL_PTY_OUTPUT_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn terminal_sessions_flow() -> &'static MutableStateFlow<Vec<RuntimeTerminalSessionInfo>> {
    TERMINAL_SESSIONS_FLOW.get_or_init(|| mutableStateFlow(Vec::new()))
}

fn runtime_terminal_session_info(session: TerminalSessionListEntry) -> RuntimeTerminalSessionInfo {
    RuntimeTerminalSessionInfo {
        sessionId: session.sessionId,
        sessionName: session.sessionName,
        terminalType: session.terminalType,
        sessionKind: session.sessionKind,
        workingDir: session.workingDir,
        commandRunning: session.commandRunning,
    }
}

/// Converts one host terminal type descriptor into the Core proxy model.
fn runtime_terminal_type_info(terminal_type: TerminalTypeInfo) -> RuntimeTerminalTypeInfo {
    RuntimeTerminalTypeInfo {
        terminalType: terminal_type.terminalType,
        available: terminal_type.available,
        description: terminal_type.description,
    }
}

/// Converts one host terminal capability descriptor into the Core proxy model.
fn runtime_terminal_info(info: TerminalInfo) -> RuntimeTerminalInfo {
    RuntimeTerminalInfo {
        platform: info.platform,
        defaultType: info.defaultType,
        types: info
            .types
            .into_iter()
            .map(runtime_terminal_type_info)
            .collect(),
    }
}

fn load_terminal_sessions(
    terminalHost: &Arc<dyn TerminalHost>,
) -> Result<Vec<RuntimeTerminalSessionInfo>, String> {
    terminalHost
        .listSessions()
        .map_err(|error| error.message)
        .map(|sessions| {
            sessions
                .into_iter()
                .map(runtime_terminal_session_info)
                .collect()
        })
}

fn publish_terminal_sessions(
    terminalHost: &Arc<dyn TerminalHost>,
) -> Result<Vec<RuntimeTerminalSessionInfo>, String> {
    let sessions = load_terminal_sessions(terminalHost)?;
    terminal_sessions_flow().set_value(sessions.clone());
    Ok(sessions)
}

fn close_terminal_pty_output_stream(sessionId: &str) {
    let entry = terminal_pty_output_streams()
        .lock()
        .expect("terminal pty output streams mutex poisoned")
        .remove(sessionId);
    if let Some(entry) = entry {
        entry.stream.close();
    }
}

fn start_terminal_pty_output_reader(
    terminalHost: Arc<dyn TerminalHost>,
    taskScheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    sessionId: String,
    stream: MutableSharedStreamImpl<String>,
) {
    let scheduledTaskScheduler = taskScheduler.clone();
    taskScheduler
        .scheduleHostRuntimeTask(
            "operit-terminal-pty-output",
            Box::new(move || {
                poll_terminal_pty_output(terminalHost, scheduledTaskScheduler, sessionId, stream);
            }),
        )
        .expect("terminal PTY output task must be scheduled");
}

/// Reads one PTY output batch and schedules the next host-owned polling turn.
fn poll_terminal_pty_output(
    terminalHost: Arc<dyn TerminalHost>,
    taskScheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    sessionId: String,
    stream: MutableSharedStreamImpl<String>,
) {
    match terminalHost.readPtySession(&sessionId) {
        Ok(data) if !data.is_empty() => stream.emit(STANDARD.encode(data)),
        Ok(_) => {}
        Err(_) => {
            close_terminal_pty_output_stream(&sessionId);
            return;
        }
    }

    match terminalHost.pollPtyExitCode(&sessionId) {
        Ok(Some(_)) => {
            publish_terminal_sessions(&terminalHost)
                .expect("TerminalHost.listSessions must succeed after PTY exit");
            close_terminal_pty_output_stream(&sessionId);
        }
        Ok(None) => {
            let nextTerminalHost = terminalHost.clone();
            let nextTaskScheduler = taskScheduler.clone();
            let nextSessionId = sessionId.clone();
            let nextStream = stream.clone();
            taskScheduler
                .scheduleDelayedHostRuntimeTask(
                    "operit-terminal-pty-output",
                    40,
                    Box::new(move || {
                        poll_terminal_pty_output(
                            nextTerminalHost,
                            nextTaskScheduler,
                            nextSessionId,
                            nextStream,
                        );
                    }),
                )
                .expect("terminal PTY output delay must be scheduled");
        }
        Err(_) => close_terminal_pty_output_stream(&sessionId),
    }
}

impl RuntimeTerminalService {
    #[allow(non_snake_case)]
    /// Creates a terminal service from application host context.
    pub fn getInstance(context: &HostManager) -> Self {
        Self {
            terminalHost: context
                .terminalHost
                .clone()
                .expect("TerminalHost must be configured for RuntimeTerminalService"),
            context: context.clone(),
        }
    }

    #[allow(non_snake_case)]
    /// Lists terminal sessions currently known by the host.
    pub fn listTerminalSessions(&self) -> Result<Vec<RuntimeTerminalSessionInfo>, String> {
        publish_terminal_sessions(&self.terminalHost)
    }

    #[allow(non_snake_case)]
    /// Returns a state flow of published terminal sessions.
    pub fn terminalSessionsFlow(
        &self,
    ) -> Result<StateFlow<Vec<RuntimeTerminalSessionInfo>>, String> {
        publish_terminal_sessions(&self.terminalHost)?;
        Ok(terminal_sessions_flow().asStateFlow())
    }

    #[allow(non_snake_case)]
    /// Returns the host-declared terminal type for manual PTY creation.
    pub fn defaultTerminalType(&self) -> Result<String, String> {
        self.terminalHost
            .terminalInfo()
            .map(|info| info.defaultType)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Returns every terminal type that the active host exposes to users.
    pub fn terminalInfo(&self) -> Result<RuntimeTerminalInfo, String> {
        self.terminalHost
            .terminalInfo()
            .map(runtime_terminal_info)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Starts a PTY terminal session and attaches its output stream.
    pub fn startTerminalPty(
        &self,
        sessionName: String,
        terminalType: String,
        workingDir: String,
        rows: i32,
        cols: i32,
    ) -> Result<String, String> {
        let resolvedWorkingDir = resolve_terminal_working_dir(&self.context, &workingDir)?;
        let sessionId = self
            .terminalHost
            .startPtySession(
                &sessionName,
                &terminalType,
                &resolvedWorkingDir,
                rows as u16,
                cols as u16,
            )
            .map_err(|error| error.message)?;
        self.ensureTerminalPtyOutputStream(sessionId.clone());
        publish_terminal_sessions(&self.terminalHost)?;
        Ok(sessionId)
    }

    #[allow(non_snake_case)]
    /// Returns the shared output stream for a PTY session.
    pub fn terminalPtyOutput(&self, sessionId: String) -> RuntimeTerminalPtyOutputStream {
        RuntimeTerminalPtyOutputStream::new(self.ensureTerminalPtyOutputStream(sessionId))
    }

    #[allow(non_snake_case)]
    /// Writes base64-encoded bytes to a PTY session.
    pub fn writeTerminalPty(&self, sessionId: String, dataBase64: String) -> Result<i32, String> {
        let data = STANDARD
            .decode(dataBase64.as_bytes())
            .map_err(|error| error.to_string())?;
        self.terminalHost
            .writePtySession(&sessionId, &data)
            .map(|count| count as i32)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Resizes a PTY session.
    pub fn resizeTerminalPty(&self, sessionId: String, rows: i32, cols: i32) -> Result<(), String> {
        self.terminalHost
            .resizePtySession(&sessionId, rows as u16, cols as u16)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Polls the exit code for a PTY session.
    pub fn pollTerminalPtyExit(&self, sessionId: String) -> Result<Option<i32>, String> {
        self.terminalHost
            .pollPtyExitCode(&sessionId)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Closes a PTY session and removes its output stream.
    pub fn closeTerminalPty(&self, sessionId: String) -> Result<(), String> {
        close_terminal_pty_output_stream(&sessionId);
        self.terminalHost
            .closePtySession(&sessionId)
            .map_err(|error| error.message)?;
        publish_terminal_sessions(&self.terminalHost)?;
        Ok(())
    }

    #[allow(non_snake_case)]
    /// Sends text input to a terminal session.
    pub fn inputTerminalSession(&self, sessionId: String, input: String) -> Result<i32, String> {
        self.terminalHost
            .inputInSession(&sessionId, Some(&input), None)
            .map(|output| output.acceptedChars as i32)
            .map_err(|error| error.message)
    }

    #[allow(non_snake_case)]
    /// Reads the current screen contents for a terminal session.
    pub fn getTerminalSessionScreen(
        &self,
        sessionId: String,
    ) -> Result<RuntimeTerminalScreen, String> {
        self.terminalHost
            .getSessionScreen(&sessionId)
            .map_err(|error| error.message)
            .map(|screen| RuntimeTerminalScreen {
                sessionId: screen.sessionId,
                terminalType: screen.terminalType,
                rows: screen.rows as i32,
                cols: screen.cols as i32,
                content: screen.content,
                commandRunning: screen.commandRunning,
            })
    }

    #[allow(non_snake_case)]
    fn ensureTerminalPtyOutputStream(&self, sessionId: String) -> MutableSharedStreamImpl<String> {
        let mut streams = terminal_pty_output_streams()
            .lock()
            .expect("terminal pty output streams mutex poisoned");
        if let Some(entry) = streams.get(&sessionId) {
            return entry.stream.clone();
        }
        let stream = MutableSharedStreamImpl::new(512);
        streams.insert(
            sessionId.clone(),
            TerminalPtyOutputEntry {
                stream: stream.clone(),
            },
        );
        let taskScheduler = self
            .context
            .hostRuntimeTaskSchedulerHost
            .clone()
            .expect("RuntimeTerminalService requires a HostRuntimeTaskSchedulerHost");
        start_terminal_pty_output_reader(
            self.terminalHost.clone(),
            taskScheduler,
            sessionId,
            stream.clone(),
        );
        stream
    }
}

fn resolve_terminal_working_dir(context: &HostManager, workingDir: &str) -> Result<String, String> {
    let trimmed = workingDir.trim();
    if trimmed.starts_with("/app/") || trimmed == "/app" {
        return terminal_vfs(context)?
            .resolvePath(trimmed)
            .map(|path| path.physicalPath);
    }
    Ok(trimmed.to_string())
}

fn terminal_vfs(context: &HostManager) -> Result<VisualFileSystem, String> {
    let runtimeStorageHost = context.runtimeStorageHost.as_ref().ok_or_else(|| {
        "RuntimeStorageHost is not configured for terminal working directory".to_string()
    })?;
    let runtimeStoreRoot = runtimeStorageHost.runtimeRootDir().ok_or_else(|| {
        "RuntimeStorageHost runtime root is not configured for terminal working directory"
            .to_string()
    })?;
    let workspaceCollectionRoot = runtimeStorageHost.workspaceRootDir().ok_or_else(|| {
        "RuntimeStorageHost workspace root is not configured for terminal working directory"
            .to_string()
    })?;
    Ok(VisualFileSystem::new(
        context.fileSystemHost.clone().ok_or_else(|| {
            "FileSystemHost is not registered for terminal working directory".to_string()
        })?,
        PathMapper::new(runtimeStoreRoot, workspaceCollectionRoot),
    ))
}
