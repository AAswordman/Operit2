use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use operit_host_api::{TerminalHost, TerminalSessionListEntry};
use operit_store::{
    PreferencesDataStore::{mutableStateFlow, MutableStateFlow, StateFlow},
    RuntimeStorePaths::RuntimeStorePaths,
};
use serde::{Deserialize, Serialize};

use operit_host_api::HostManager::HostManager;
use operit_tools::files::PathMapper::PathMapper;
use operit_tools::files::VisualFileSystem::VisualFileSystem;
use operit_util::stream::HotStream::MutableSharedStreamImpl;
use operit_util::stream::Stream::Stream;

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

    fn collect(&mut self, collector: &mut dyn FnMut(Self::Item)) {
        self.upstream.collect(collector);
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
    sessionId: String,
    stream: MutableSharedStreamImpl<String>,
) {
    thread::spawn(move || loop {
        match terminalHost.readPtySession(&sessionId) {
            Ok(data) => {
                if !data.is_empty() {
                    stream.emit(STANDARD.encode(data));
                }
            }
            Err(_) => {
                close_terminal_pty_output_stream(&sessionId);
                break;
            }
        }

        match terminalHost.pollPtyExitCode(&sessionId) {
            Ok(Some(_)) => {
                publish_terminal_sessions(&terminalHost)
                    .expect("TerminalHost.listSessions must succeed after PTY exit");
                close_terminal_pty_output_stream(&sessionId);
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(_) => {
                close_terminal_pty_output_stream(&sessionId);
                break;
            }
        }
    });
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
        start_terminal_pty_output_reader(self.terminalHost.clone(), sessionId, stream.clone());
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
    let runtimeStoreRoot = context
        .runtimeStorageHost
        .as_ref()
        .and_then(|host| host.rootDir())
        .ok_or_else(|| {
            "RuntimeStorageHost root is not configured for terminal working directory".to_string()
        })?;
    let runtimeStorePaths = RuntimeStorePaths::new(runtimeStoreRoot.clone());
    Ok(VisualFileSystem::new(
        context.fileSystemHost.clone().ok_or_else(|| {
            "FileSystemHost is not registered for terminal working directory".to_string()
        })?,
        PathMapper::new(
            runtimeStoreRoot,
            context.appFilesRoot.clone(),
            runtimeStorePaths.workspace_dir(),
        ),
    ))
}
