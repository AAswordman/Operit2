use operit_host_api::TerminalPtyOutput::TerminalPtyOutputBuffer;
#[path = "../../terminal_command.rs"]
mod terminal_command;

use operit_host_api::terminalSessionKey as sessionKey;
use operit_host_api::TerminalText::{appendPtyOutput, dropCommandEcho, nonBlank, parseExitCode, renderTerminalText, rfindBytes, stripPtyDeviceStatusReports, stripPtyPromptMarkers, scanPtyOutput, updatePtyCursor, PtyCursor};
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use uuid::Uuid;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
const PTY_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
const PTY_PROMPT_MARKER_PREFIX: &[u8] = b"\x1b]133;OperitPrompt=";
const PTY_PROMPT_MARKER_END: u8 = 7;
const PLATFORM: &str = "posix";
const TERMINAL: &str = "native";
const PRIMARY_TERMINAL_TYPE: &str = "bash";

#[derive(Clone)]
pub struct NativePtyTerminalHost {
    state: Arc<Mutex<TerminalState>>,
    commandSpec: Arc<PtyCommandSpec>,
}

#[derive(Clone)]
struct PtyCommandSpec {
    program: String,
    controllingTty: bool,
    args: Vec<String>,
    terminalType: String,
    description: String,
    nameKey: Option<String>,
    commandProtocol: PtyCommandProtocol,
    processWorkingDirectory: PtyProcessWorkingDirectory,
    initialWorkingDirectory: PtyInitialWorkingDirectory,
    environment: Vec<(String, String)>,
    sessionWorkingDirectoryEnvironment: PtySessionWorkingDirectoryEnvironment,
}

/// Describes an explicitly configured interactive POSIX shell process.
#[derive(Clone)]
pub struct NativePtyShellCommand {
    pub program: String,
    pub arguments: Vec<String>,
    pub description: String,
    pub processWorkingDirectory: String,
    pub defaultSessionWorkingDirectory: String,
    pub environment: Vec<(String, String)>,
    pub sessionWorkingDirectoryEnvironment: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PtyCommandProtocol {
    BashPromptCommand,
    ExplicitMarker,
}

#[derive(Clone)]
enum PtyProcessWorkingDirectory {
    Session,
    Fixed(String),
}

#[derive(Clone)]
enum PtyInitialWorkingDirectory {
    ProcessCurrent,
    Fixed(String),
}

#[derive(Clone)]
enum PtySessionWorkingDirectoryEnvironment {
    None,
    Name(String),
}

impl Default for NativePtyTerminalHost {
    /// Creates the shared POSIX host configured for its native bash terminal.
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(TerminalState::default())),
            commandSpec: Arc::new(PtyCommandSpec::nativeBash()),
        }
    }
}

impl PtyCommandSpec {
    /// Builds the default interactive bash command configuration.
    fn nativeBash() -> Self {
        Self {
            program: if cfg!(target_os = "macos") {
                "/bin/bash"
            } else {
                "bash"
            }
            .to_string(),
            controllingTty: !cfg!(target_os = "macos"),
            args: vec![
                "--noprofile".to_string(),
                "--norc".to_string(),
                "-i".to_string(),
            ],
            terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
            description: "POSIX bash terminal".to_string(),
            nameKey: Some("terminalBuiltinPosix".into()),
            commandProtocol: PtyCommandProtocol::BashPromptCommand,
            processWorkingDirectory: PtyProcessWorkingDirectory::Session,
            initialWorkingDirectory: PtyInitialWorkingDirectory::ProcessCurrent,
            environment: Vec::new(),
            sessionWorkingDirectoryEnvironment: PtySessionWorkingDirectoryEnvironment::None,
        }
    }

    /// Builds the privileged system POSIX sh command configuration.
    fn systemShell() -> Self {
        Self {
            program: "/bin/sh".to_string(),
            controllingTty: !cfg!(target_os = "macos"),
            args: vec!["-i".to_string()],
            terminalType: "shell".to_string(),
            description: "Privileged system /bin/sh terminal".to_string(),
            nameKey: Some("terminalBuiltinPrivileged".into()),
            commandProtocol: PtyCommandProtocol::ExplicitMarker,
            processWorkingDirectory: PtyProcessWorkingDirectory::Session,
            initialWorkingDirectory: PtyInitialWorkingDirectory::ProcessCurrent,
            environment: Vec::new(),
            sessionWorkingDirectoryEnvironment: PtySessionWorkingDirectoryEnvironment::None,
        }
    }

    /// Builds an explicitly configured POSIX shell command specification.
    fn customShell(command: NativePtyShellCommand) -> HostResult<Self> {
        let program = nonBlank(&command.program, "program")?;
        let description = nonBlank(&command.description, "description")?;
        let processWorkingDirectory = nonBlank(
            &command.processWorkingDirectory,
            "process_working_directory",
        )?;
        let defaultSessionWorkingDirectory = nonBlank(
            &command.defaultSessionWorkingDirectory,
            "default_session_working_directory",
        )?;
        let sessionWorkingDirectoryEnvironment = nonBlank(
            &command.sessionWorkingDirectoryEnvironment,
            "session_working_directory_environment",
        )?;
        if !program.starts_with('/') {
            return Err(HostError::new(format!(
                "Custom POSIX shell program must be absolute: {program}"
            )));
        }
        Ok(Self {
            program,
            controllingTty: !cfg!(target_os = "macos"),
            args: command.arguments,
            terminalType: "shell".to_string(),
            description,
            nameKey: None,
            commandProtocol: PtyCommandProtocol::ExplicitMarker,
            processWorkingDirectory: PtyProcessWorkingDirectory::Fixed(processWorkingDirectory),
            initialWorkingDirectory: PtyInitialWorkingDirectory::Fixed(
                defaultSessionWorkingDirectory,
            ),
            environment: command.environment,
            sessionWorkingDirectoryEnvironment: PtySessionWorkingDirectoryEnvironment::Name(
                sessionWorkingDirectoryEnvironment,
            ),
        })
    }
}

#[derive(Default)]
struct TerminalState {
    sessionNameToId: BTreeMap<String, String>,
    hiddenExecutorKeyToSessionId: BTreeMap<String, String>,
    ptySessions: BTreeMap<String, PtySession>,
}

struct PtySession {
    sessionName: String,
    terminalType: String,
    workingDir: String,
    process: Option<PtyProcess>,
    output: Arc<TerminalPtyOutputBuffer>,
    commandOutput: PtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    commandRunning: bool,
    processOwned: bool,
    exitCode: Option<i32>,
    commandProtocol: PtyCommandProtocol,
}

type PtyCommandOutput = Arc<operit_host_api::TerminalCommand::TerminalCommandBuffer>;

impl Drop for PtySession {
    /// Wakes silent command observers before releasing the process and PTY handles.
    fn drop(&mut self) {
        self.output.close();
        self.commandOutput.fail(HostError::new("Terminal session was closed"));
    }
}

type PtyCursorState = Arc<Mutex<PtyCursor>>;

/// Owns every live PTY resource separately from retained session output and status.
struct PtyProcess {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    readerTask: Option<Box<dyn FnOnce() + Send>>,
}

impl Drop for PtyProcess {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            if self.child.kill().is_ok() {
                // portable-pty escalates SIGHUP to SIGKILL; reap the shell as well.
                let _ = self.child.wait();
            }
        }
        if let Some(close) = self.readerTask.take() { close(); }
    }
}

impl PtySession {
    fn process(&self) -> HostResult<&PtyProcess> {
        self.process
            .as_ref()
            .ok_or_else(|| HostError::new("PTY session has exited"))
    }

    /// Reclaims handles on observed exit while preserving output and the exit receipt.
    fn pollExitCode(&mut self) -> HostResult<Option<i32>> {
        if self.exitCode.is_none() {
            if let Some(process) = self.process.as_mut() {
                if let Some(status) = process.child.try_wait()? {
                    self.exitCode = Some(status.exit_code() as i32);
                    self.commandRunning = false;
                    if !self.processOwned { self.process.take(); }
                }
            }
        }
        Ok(self.exitCode)
    }
}

impl NativePtyTerminalHost {
    /// Creates a separate configured shell while preserving this host's PTY and command protocol.
    pub fn withProfile(&self, profile: &operit_host_api::TerminalProfile) -> HostResult<Self> {
        let program = profile.options.get("executable").ok_or_else(|| HostError::new("Custom terminal requires executable"))?;
        if !std::path::Path::new(program).is_absolute() || !std::path::Path::new(program).is_file() {
            return Err(HostError::new("Terminal executable must be an existing absolute file path"));
        }
        let mut spec = (*self.commandSpec).clone();
        spec.program = program.clone();
        let mut arguments = operit_host_api::TerminalProfiles::profileArguments(profile)?;
        arguments.extend(spec.args);
        spec.args = arguments;
        Ok(Self { state: Arc::default(), commandSpec: Arc::new(spec) })
    }
    /// Creates the shared POSIX host configured for its native bash terminal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a host that starts the actual system /bin/sh process in a PTY.
    pub fn systemShell() -> Self {
        Self {
            state: Arc::new(Mutex::new(TerminalState::default())),
            commandSpec: Arc::new(PtyCommandSpec::systemShell()),
        }
    }

    /// Creates a host that starts one explicitly configured POSIX shell process in a PTY.
    pub fn customShell(command: NativePtyShellCommand) -> HostResult<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(TerminalState::default())),
            commandSpec: Arc::new(PtyCommandSpec::customShell(command)?),
        })
    }

    /// Retains ownership after shell exit, until the caller closes the session.
    pub fn ownsSession(&self, sessionId: &str) -> HostResult<bool> {
        Ok(self.lockState()?.ptySessions.contains_key(sessionId))
    }

    /// Selects whether the real PTY becomes the child process's controlling TTY.
    /// Apple sandbox hosts must disable TIOCSCTTY; unrestricted hosts keep job control.
    pub fn withControllingTty(mut self, enabled: bool) -> Self {
        Arc::make_mut(&mut self.commandSpec).controllingTty = enabled;
        self
    }

    /// Creates a native login shell with its normal startup files and environment.
    pub fn loginShell(program: &str, terminalType: &str, home: &str) -> HostResult<Self> {
        if !std::path::Path::new(program).is_absolute() || !std::path::Path::new(program).is_file() {
            return Err(HostError::new(format!("Login shell is unavailable: {program}")));
        }
        let home = nonBlank(home, "home_directory")?;
        let mut spec = PtyCommandSpec::nativeBash();
        spec.program = program.to_string();
        spec.controllingTty = true;
        spec.args = vec!["-l".to_string(), "-i".to_string()];
        spec.terminalType = nonBlank(terminalType, "terminal_type")?;
        spec.description = format!("Native {program} login shell");
        spec.commandProtocol = PtyCommandProtocol::ExplicitMarker;
        spec.initialWorkingDirectory = PtyInitialWorkingDirectory::Fixed(home);
        spec.environment = vec![("SHELL".to_string(), program.to_string())];
        Ok(Self {
            state: Arc::new(Mutex::new(TerminalState::default())),
            commandSpec: Arc::new(spec),
        })
    }

    /// Verifies that this host can start and close its configured PTY shell.
    pub fn probe(&self, workingDir: &str) -> HostResult<()> {
        let sessionId = self.startPtySession(
            "operit-terminal-capability-probe",
            TERMINAL,
            &self.commandSpec.terminalType,
            workingDir,
            1,
            1,
        )?;
        self.closePtySession(&sessionId)
    }

    /// Validates the shell type configured for this native PTY host.
    fn normalizeTerminalType(&self, terminalType: &str) -> HostResult<String> {
        match terminalType.trim() {
            value if value == self.commandSpec.terminalType => Ok(value.to_string()),
            value => Err(HostError::new(format!(
                "Unsupported terminal type for shared POSIX PTY host: {value}"
            ))),
        }
    }

    /// Returns the working directory used for automatically created terminal sessions.
    fn initialWorkingDirectory(&self) -> HostResult<String> {
        match &self.commandSpec.initialWorkingDirectory {
            PtyInitialWorkingDirectory::ProcessCurrent => {
                Ok(std::env::current_dir()?.display().to_string())
            }
            PtyInitialWorkingDirectory::Fixed(directory) => Ok(directory.clone()),
        }
    }
}

impl TerminalHost for NativePtyTerminalHost {
    /// Starts a fresh process in a PTY; completion is its operating-system exit status.
    fn openTerminalSession(&self, request: &operit_host_api::TerminalSessionRequest) -> HostResult<TerminalSessionInfo> {
        requireNativeTerminal(&request.terminal)?;
        let kind = self.normalizeTerminalType(&request.terminalType)?;
        let directory = match &request.workingDir { Some(path) => path.clone(), None => self.initialWorkingDirectory()? };
        let session = createPtySessionForProcess(request.sessionName.clone(), kind.clone(), directory, 24, 80,
            &self.commandSpec, request.command.as_deref(), false)?;
        let id = nextSessionId();
        self.lockState()?.ptySessions.insert(id.clone(), session);
        Ok(TerminalSessionInfo { sessionId: id, sessionName: request.sessionName.clone(), platform: PLATFORM.into(),
            terminal: TERMINAL.into(), terminalType: kind, isNewSession: true })
    }

    /// Resolves native terminal directories through the runtime's host VFS mapper.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        requireNativeTerminal(terminal)?;
        self.normalizeTerminalType(terminalType)?;
        resolveHostDirectory(workingDir)
    }

    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: self.commandSpec.terminalType.clone(),
            types: vec![TerminalTypeInfo {
                terminal: TERMINAL.to_string(),
                terminalType: self.commandSpec.terminalType.clone(),
                available: true,
                description: self.commandSpec.description.clone(),
                nameKey: self.commandSpec.nameKey.clone(),
            }],
        })
    }

    fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        requireNativeTerminal(terminal)?;
        let normalizedTerminalType = self.normalizeTerminalType(terminalType)?;
        let workDir = nonBlank(workingDir, "working_directory")?;
        let session = createPtySession(
            normalizedSessionName,
            normalizedTerminalType,
            workDir,
            rows,
            cols,
            self.commandSpec.as_ref(),
        )?;
        let sessionId = nextSessionId();
        let mut state = self.lockState()?;
        state.ptySessions.insert(sessionId.clone(), session);
        Ok(sessionId)
    }

    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        let mut state = self.lockState()?;
        let session = state
            .ptySessions
            .get_mut(sessionId)
            .ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?;
        session.output.read()
    }

    /// Waits on the PTY reader's producer notification, including final EOF.
    fn nextPtySessionOutput<'a>(&'a self, sessionId: &'a str) -> operit_host_api::TerminalPtyOutputFuture<'a> {
        Box::pin(async move {
            let output = {
                let state = self.lockState()?;
                state.ptySessions.get(sessionId)
                    .ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?.output.clone()
            };
            output.next().await
        })
    }

    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        let writer = {
            let state = self.lockState()?;
            let session = state.ptySessions.get(sessionId)
                .ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?;
            session.process()?.writer.clone()
        };
        let mut writer = writer.lock().map_err(|_| HostError::new("PTY writer mutex poisoned"))?;
        writer.write_all(data)?;
        writer.flush()?;
        Ok(data.len())
    }

    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        let state = self.lockState()?;
        let session = state
            .ptySessions
            .get(sessionId)
            .ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?;
        session
            .process()?
            .master
            .resize(ptySize(rows, cols))
            .map_err(toHostError)
    }

    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        let mut state = self.lockState()?;
        let session = state
            .ptySessions
            .get_mut(sessionId)
            .ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?;
        session.pollExitCode()
    }

    /// Removes registry ownership before tearing down a potentially blocking OS process.
    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        let removed = {
            let mut state = self.lockState()?;
            let removed = state.ptySessions.remove(sessionId).ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?;
            removeSessionMappings(&mut state, sessionId);
            removed
        };
        drop(removed);
        Ok(())
    }

    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let mut state = self.lockState()?;
        let mut entries = Vec::new();
        for (sessionId, session) in state.ptySessions.iter_mut() {
            if session.pollExitCode()?.is_some() && !session.processOwned {
                continue;
            }
            entries.push(TerminalSessionListEntry {
                sessionId: sessionId.clone(),
                sessionName: session.sessionName.clone(),
                platform: PLATFORM.to_string(),
                terminal: TERMINAL.to_string(),
                terminalType: session.terminalType.clone(),
                sessionKind: "pty".to_string(),
                workingDir: session.workingDir.clone(),
                commandRunning: session.commandRunning,
            });
        }
        Ok(entries)
    }

    /// Creates or reuses a named session in the configured default terminal.
    fn createOrGetSession(&self, sessionName: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetTypedSession(sessionName, &self.commandSpec.terminalType)
    }

    /// Creates or reuses a named session of exactly the requested interpreter type.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        let normalizedTerminalType = self.normalizeTerminalType(terminalType)?;
        let sessionKey = sessionKey(&normalizedTerminalType, &normalizedSessionName);
        {
            let mut state = self.lockState()?;
            if let Some(sessionId) = state.sessionNameToId.get(&sessionKey).cloned() {
                if state.ptySessions.contains_key(&sessionId) {
                    return Ok(TerminalSessionInfo {
                        sessionId,
                        sessionName: normalizedSessionName,
                        platform: PLATFORM.to_string(),
                        terminal: TERMINAL.to_string(),
                        terminalType: normalizedTerminalType,
                        isNewSession: false,
                    });
                }
                state.sessionNameToId.remove(&sessionKey);
            }
        }

        let workingDir = self.initialWorkingDirectory()?;
        let session = createPtySession(
            normalizedSessionName.clone(),
            normalizedTerminalType.clone(),
            workingDir,
            24,
            80,
            self.commandSpec.as_ref(),
        )?;
        let sessionId = nextSessionId();
        let mut state = self.lockState()?;
        state.sessionNameToId.insert(sessionKey, sessionId.clone());
        state.ptySessions.insert(sessionId.clone(), session);
        Ok(TerminalSessionInfo {
            sessionId,
            sessionName: normalizedSessionName,
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: normalizedTerminalType,
            isNewSession: true,
        })
    }

    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        self.executeInSessionStreaming(sessionId, command, timeoutMs, &mut |_| Ok(()))
    }

    /// Runs explicit synchronous callers through the same event-driven command state.
    fn executeInSessionStreaming(
        &self, sessionId: &str, command: &str, timeoutMs: u64,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        operit_host_native_scheduler::runTerminalCommandSynchronously(
            Box::new(self.beginCommand(sessionId, command, timeoutMs)?), onOutput,
        )
    }

    /// Returns command state whose output waits never occupy a host worker.
    fn beginInSessionStreaming(self: Arc<Self>, sessionId: String, command: String, timeoutMs: u64)
        -> HostResult<Box<dyn operit_host_api::TerminalCommand::TerminalCommandExecution>> {
        Ok(Box::new(self.beginCommand(&sessionId, &command, timeoutMs)?))
    }

    fn executeHiddenCommand(
        &self,
        command: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let normalizedCommand = nonBlank(command, "command")?;
        let normalizedTerminalType = self.commandSpec.terminalType.clone();
        let normalizedExecutorKey = match executorKey.trim() {
            "" => "default".to_string(),
            value => value.to_string(),
        };
        let executorKey = sessionKey(&normalizedTerminalType, &normalizedExecutorKey);
        let sessionId = hiddenPtySessionId(
            self,
            &executorKey,
            &normalizedExecutorKey,
            &normalizedTerminalType,
        )?;
        let output = self.executeInSession(&sessionId, &normalizedCommand, timeoutMs)?;
        Ok(HiddenTerminalCommandOutput {
            command: normalizedCommand,
            output: output.output,
            exitCode: output.exitCode,
            executorKey: normalizedExecutorKey,
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: normalizedTerminalType,
            timedOut: output.timedOut,
        })
    }

    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let data = operit_host_api::TerminalControl::terminalInput(input, control)?;
        self.writePtySession(&normalizedSessionId, data.as_bytes())?;
        let acceptedChars = data.chars().count();
        Ok(TerminalInputOutput {
            sessionId: normalizedSessionId,
            acceptedChars,
        })
    }

    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        self.closePtySession(&normalizedSessionId)?;
        Ok(TerminalCloseOutput {
            sessionId: normalizedSessionId.clone(),
            success: true,
            message: format!("Terminal session closed: {normalizedSessionId}"),
        })
    }

    fn closeSessionIfIdle(&self, sessionId: &str, sessionName: &str) -> HostResult<TerminalCloseOutput> {
        let id = nonBlank(sessionId, "session_id")?;
        let name = nonBlank(sessionName, "session_name")?;
        let mut state = self.lockState()?;
        let session = state.ptySessions.get(&id)
            .ok_or_else(|| HostError::new(format!("Terminal session does not exist: {id}")))?;
        if !operit_host_api::terminalSessionNameMatches(&session.sessionName, &name) {
            return Err(HostError::new("Terminal session name does not match"));
        }
        if session.commandRunning {
            return Err(HostError::new("Terminal session has a command in progress"));
        }
        let removed = state.ptySessions.remove(&id);
        removeSessionMappings(&mut state, &id);
        drop(state);
        drop(removed);
        Ok(TerminalCloseOutput { sessionId: id.clone(), success: true, message: format!("Idle terminal session closed: {id}") })
    }

    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let mut state = self.lockState()?;
        let session = state
            .ptySessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        if session.processOwned { session.pollExitCode()?; }
        let terminalType = session.terminalType.clone();
        let commandRunning = session.commandRunning;
        let screenOutput = session.screenOutput.clone();
        drop(state);
        let content = ptyScreenContent(&screenOutput)?;
        let rows = content.lines().count();
        let cols = content
            .lines()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        Ok(TerminalScreenOutput {
            sessionId: normalizedSessionId,
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType,
            rows,
            cols,
            content,
            commandRunning,
        })
    }
}

impl NativePtyTerminalHost {
    #[allow(non_snake_case)]
    fn lockState(&self) -> HostResult<std::sync::MutexGuard<'_, TerminalState>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("terminal state mutex poisoned"))
    }
}

#[allow(non_snake_case)]
fn hiddenPtySessionId(
    host: &NativePtyTerminalHost,
    executorKey: &str,
    executorLabel: &str,
    terminalType: &str,
) -> HostResult<String> {
    {
        let mut state = host.lockState()?;
        if let Some(sessionId) = liveHiddenPtySessionId(&mut state, executorKey)? {
            return Ok(sessionId);
        }
    }

    let workingDir = host.initialWorkingDirectory()?;
    let session = createPtySession(
        format!("hidden:{executorLabel}"),
        terminalType.to_string(),
        workingDir,
        24,
        80,
        host.commandSpec.as_ref(),
    )?;
    let sessionId = nextSessionId();
    let mut state = host.lockState()?;
    // Another caller may have installed a session while this shell was starting.
    // Keep one executor per key and drop the unused shell instead of orphaning it.
    if let Some(existing) = liveHiddenPtySessionId(&mut state, executorKey)? {
        drop(state);
        drop(session);
        return Ok(existing);
    }
    state
        .hiddenExecutorKeyToSessionId
        .insert(executorKey.to_string(), sessionId.clone());
    state.ptySessions.insert(sessionId.clone(), session);
    Ok(sessionId)
}

/// Only replace an executor observed to have exited before submitting a command.
/// Once a command has been submitted its outcome may be unknown, so never replay it.
#[allow(non_snake_case)]
fn liveHiddenPtySessionId(state: &mut TerminalState, executorKey: &str) -> HostResult<Option<String>> {
    let Some(sessionId) = state.hiddenExecutorKeyToSessionId.get(executorKey).cloned() else {
        return Ok(None);
    };
    let alive = match state.ptySessions.get_mut(&sessionId) {
        Some(session) if session.commandRunning => true,
        Some(session) => session.pollExitCode()?.is_none(),
        None => false,
    };
    if alive {
        return Ok(Some(sessionId));
    }
    state.ptySessions.remove(&sessionId);
    removeSessionMappings(state, &sessionId);
    Ok(None)
}

/// Owns command parsing independently of the executor thread awaiting terminal events.
struct CooperativeCommand {
    host: NativePtyTerminalHost,
    sessionId: String,
    command: String,
    terminalType: String,
    commandEcho: String,
    commandOutput: PtyCommandOutput,
    timeoutMs: u64,
    collected: Vec<u8>,
    workingDir: Option<String>,
    commandProtocol: PtyCommandProtocol,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl NativePtyTerminalHost {
    /// Submits one command and reserves its session before asynchronous observation begins.
    fn beginCommand(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<CooperativeCommand> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let normalizedCommand = nonBlank(command, "command")?;
        let (terminalType, commandOutput, commandProtocol, commandEcho, writer) = {
            let mut state = self.lockState()?;
            let session = state
                .ptySessions
                .get_mut(&normalizedSessionId)
                .ok_or_else(|| {
                    HostError::new(format!("Terminal session does not exist: {sessionId}"))
                })?;
            if session.commandRunning {
                return Err(HostError::new(
                    "Terminal session already has a command in progress",
                ));
            }
            clearPtyCommandOutput(&session.commandOutput)?;
            let commandInput = ptyCommandInput(&normalizedCommand, session.commandProtocol);
            let commandEcho = commandInput.trim_end().to_string();
            let mut writer = session
                .process()?
                .writer
                .lock()
                .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
            writer.write_all(commandInput.as_bytes())?;
            writer.flush()?;
            drop(writer);
            session.commandRunning = true;
            (
                session.terminalType.clone(),
                session.commandOutput.clone(),
                session.commandProtocol,
                commandEcho,
                session.process()?.writer.clone(),
            )
        };

        Ok(CooperativeCommand {
            host: self.clone(),
            commandEcho: commandEcho,
            sessionId: normalizedSessionId,
            command: normalizedCommand,
            terminalType,
            commandOutput,
            timeoutMs,
            collected: Vec::new(),
            workingDir: None,
            writer,
            commandProtocol,
        })
    }
}

impl operit_host_api::TerminalCommand::TerminalCommandExecution for CooperativeCommand {
    /// Registers a wakeup and parses available protocol records without waiting.
    fn pollOutput(
        &mut self,
        context: &mut std::task::Context<'_>,
        expired: bool,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> std::task::Poll<HostResult<TerminalCommandOutput>> {
        let result = (|| -> HostResult<Option<TerminalCommandOutput>> {
            let previousLength = self.collected.len();
            let readerError = self.commandOutput.pollDrain(context, &mut self.collected)?;
            if self.collected.len() > previousLength {
                onOutput(&self.collected[previousLength..])?;
            }
            if self.timeoutMs == 0 && self.collected.len() > PTY_OUTPUT_LIMIT {
                self.collected
                    .drain(..self.collected.len() - PTY_OUTPUT_LIMIT);
            }
            let (output, exitCode, timedOut) =
                if let Some(marker) = findLastPtyPromptMarker(&self.collected)? {
                    let output = ptyCommandOutputText(
                        &self.collected,
                        &self.commandEcho,
                        Some(&marker.workingDir),
                        self.commandProtocol,
                    );
                    self.workingDir = Some(marker.workingDir);
                    (output, marker.exitCode, false)
                } else {
                    if let Some(error) = readerError {
                        return Err(error);
                    }
                    if !expired {
                        return Ok(None);
                    }
                    (
                        ptyCommandOutputText(
                            &self.collected,
                            &self.commandEcho,
                            None,
                            self.commandProtocol,
                        ),
                        -1,
                        true,
                    )
                };
            Ok(Some(TerminalCommandOutput {
                command: self.command.clone(),
                output,
                exitCode,
                sessionId: self.sessionId.clone(),
                platform: PLATFORM.to_string(),
                terminal: TERMINAL.to_string(),
                terminalType: self.terminalType.clone(),
                timedOut,
            }))
        })();
        match result {
            Ok(Some(receipt)) => std::task::Poll::Ready(Ok(receipt)),
            Ok(None) => std::task::Poll::Pending,
            Err(error) => std::task::Poll::Ready(Err(error)),
        }
    }

    /// Keeps execution lifetime independent from the foreground observation deadline.
    fn timeoutMs(&self) -> u64 {
        self.timeoutMs
    }

    /// Sends Ctrl+C on the finite OS boundary without retaining the session registry lock.
    fn beginInterrupt(&mut self) -> HostResult<()> {
        self.commandOutput.clear()?;
        self.collected.clear();
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
        writer.write_all(b"\x03")?;
        if self.commandProtocol == PtyCommandProtocol::ExplicitMarker {
            writer.write_all(b"\r")?;
            writer.write_all(
                format!("__operit_status=$?; {}\r", ptyPromptMarkerCommand()).as_bytes(),
            )?;
        }
        writer.flush()?;
        Ok(())
    }

    /// Awaits the shell readiness marker through output notifications, including silent close.
    fn pollInterrupt(
        &mut self,
        context: &mut std::task::Context<'_>,
        expired: bool,
    ) -> std::task::Poll<HostResult<()>> {
        let result = (|| -> HostResult<Option<()>> {
            let readerError = self.commandOutput.pollDrain(context, &mut self.collected)?;
            if let Some(marker) = findLastPtyPromptMarker(&self.collected)? {
                self.workingDir = Some(marker.workingDir);
                return Ok(Some(()));
            }
            if let Some(error) = readerError {
                return Err(error);
            }
            if expired {
                return Err(HostError::new(
                    "Timed out waiting for the interrupted terminal to become ready",
                ));
            }
            Ok(None)
        })();
        match result {
            Ok(Some(())) => std::task::Poll::Ready(Ok(())),
            Ok(None) => std::task::Poll::Pending,
            Err(error) => std::task::Poll::Ready(Err(error)),
        }
    }

    /// Releases the busy flag only after completion, or closes an unresponsive session.
    fn finish(
        self: Box<Self>,
        result: HostResult<TerminalCommandOutput>,
        settled: HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        let mut state = self.host.lockState()?;
        if let Err(error) = settled {
            removeSessionMappings(&mut state, &self.sessionId);
            let removed = state.ptySessions.remove(&self.sessionId);
            drop(state);
            drop(removed);
            return Err(HostError::new(format!(
                "Terminal session {} closed because its command could not be interrupted: {}",
                self.sessionId, error.message
            )));
        }
        if let Some(session) = state.ptySessions.get_mut(&self.sessionId) {
            session.commandRunning = false;
            if let Some(workingDir) = &self.workingDir {
                session.workingDir = workingDir.clone();
            }
        }
        result
    }
}


#[allow(non_snake_case)]
fn createPtySession(
    sessionName: String,
    terminalType: String,
    workingDir: String,
    rows: u16,
    cols: u16,
    commandSpec: &PtyCommandSpec,
) -> HostResult<PtySession> {
    createPtySessionForProcess(sessionName, terminalType, workingDir, rows, cols, commandSpec, None, true)
}

/// Builds either an uninstrumented interactive shell or a complete command process.
fn createPtySessionForProcess(
    sessionName: String, terminalType: String, workingDir: String, rows: u16, cols: u16,
    commandSpec: &PtyCommandSpec, script: Option<&str>, managedPrompt: bool,
) -> HostResult<PtySession> {
    let ptySystem = native_pty_system();
    let pair = ptySystem
        .openpty(ptySize(rows, cols))
        .map_err(|error| HostError::new(format!("PTY open failed: {error}")))?;
    #[cfg(unix)]
    let (reader, writer) = {
        use std::os::fd::BorrowedFd;
        let fd = pair.master.as_raw_fd().ok_or_else(|| HostError::new("PTY master has no file descriptor"))?;
        let reader = unsafe { BorrowedFd::borrow_raw(fd) }.try_clone_to_owned()?;
        let writer = unsafe { BorrowedFd::borrow_raw(fd) }.try_clone_to_owned()?;
        (reader, Box::new(operit_host_native_scheduler::NativeFdWriter(std::fs::File::from(writer))) as Box<dyn Write + Send>)
    };
    #[cfg(not(unix))]
    let (mut reader, writer) = (pair.master.try_clone_reader().map_err(|error| HostError::new(error.to_string()))?,
        pair.master.take_writer().map_err(|error| HostError::new(error.to_string()))?);
    let writer = Arc::new(Mutex::new(writer));
    let mut command = posixPtyCommand(&workingDir, commandSpec);
    if !managedPrompt {
        command.env_remove("PROMPT_COMMAND");
        command.env_remove("PS1");
        if let Some(script) = script {
            command.get_argv_mut().pop();
            command.args(["-c", script]);
        }
    }
    let child = pair.slave.spawn_command(command).map_err(|error| {
        HostError::new(format!(
            "PTY shell spawn failed (program={}, cwd={}): {error}",
            commandSpec.program, workingDir
        ))
    })?;
    // The parent must never retain the slave after spawn, including setup failures.
    drop(pair.slave);
    let mut process = PtyProcess {
        child,
        master: pair.master,
        writer: writer.clone(),
        readerTask: None,
    };
    let output = Arc::new(TerminalPtyOutputBuffer::new(PTY_OUTPUT_LIMIT));
    let commandOutput: PtyCommandOutput = Arc::new(operit_host_api::TerminalCommand::TerminalCommandBuffer::new(PTY_OUTPUT_LIMIT));
    let screenOutput = Arc::new(Mutex::new(VecDeque::new()));
    let cursorState: PtyCursorState = Arc::new(Mutex::new(PtyCursor { row: 1, col: 1 }));
    let outputForThread = output.clone();
    let commandOutputForThread = commandOutput.clone();
    let screenOutputForThread = screenOutput.clone();
    let writerForThread = writer.clone();
    let cursorStateForThread = cursorState.clone();
    #[cfg(unix)]
    {
        let closed = commandOutput.clone();
        let outputClosed = output.clone();
        let readerTask = operit_host_native_scheduler::scheduleNativeFdReader(
            "operit-pty-reader", reader, move |chunk| {
                processPtyTerminalQueries(&writerForThread, &cursorStateForThread, chunk)?;
                let commandChunk = stripPtyDeviceStatusReports(chunk);
                let visibleChunk = stripPtyPromptMarkers(&commandChunk);
                outputForThread.append(&visibleChunk);
                appendPtyOutput(&screenOutputForThread, &visibleChunk, PTY_OUTPUT_LIMIT);
                if managedPrompt { appendPtyCommandOutput(&commandOutputForThread, &commandChunk); }
                Ok(())
            }, move |error| { outputClosed.close(); closed.fail(error); },
        )?;
        process.readerTask = Some(Box::new(move || {
            if let Err(error) = readerTask.close() { operit_host_api::logHostError("terminal-reader-close", &error.message); }
        }));
    }
    #[cfg(not(unix))]
    {
        let readerTask = operit_host_native_scheduler::scheduleNativeServiceTask("operit-pty-reader", move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let chunk = &buffer[..count];
                        if processPtyTerminalQueries(&writerForThread, &cursorStateForThread, chunk).is_err() { break; }
                        let commandChunk = stripPtyDeviceStatusReports(chunk);
                        let visibleChunk = stripPtyPromptMarkers(&commandChunk);
                        outputForThread.append(&visibleChunk);
                        appendPtyOutput(&screenOutputForThread, &visibleChunk, PTY_OUTPUT_LIMIT);
                        if managedPrompt { appendPtyCommandOutput(&commandOutputForThread, &commandChunk); }
                    },
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            outputForThread.close();
            commandOutputForThread.fail(HostError::new("PTY reader reached EOF or stopped"));
        })?;
        process.readerTask = Some(Box::new(move || {
            if let Err(error) = readerTask.join_timeout(Duration::from_secs(1)) {
                operit_host_api::logHostError("terminal-reader-close", &error.to_string());
            }
        }));
    }
    if managedPrompt && commandSpec.commandProtocol == PtyCommandProtocol::ExplicitMarker {
        writePtyPromptMarker(&writer)?;
    }
    if managedPrompt {
    operit_host_native_scheduler::waitForTerminalPrompt(
        &commandOutput,
        10000,
        |bytes| findLastPtyPromptMarker(bytes).map(|marker| marker.is_some()),
        |_| HostError::new("Timed out waiting for terminal prompt"),
    )?;
    clearPtyCommandOutput(&commandOutput)?;
    if commandSpec.commandProtocol == PtyCommandProtocol::ExplicitMarker {
        output.clear()?;
        clearPtyOutput(&screenOutput)?;
    }
    }
    Ok(PtySession {
        sessionName,
        terminalType,
        workingDir,
        process: Some(process),
        output,
        commandOutput,
        screenOutput,
        commandRunning: !managedPrompt,
        processOwned: !managedPrompt,
        exitCode: None,
        commandProtocol: commandSpec.commandProtocol,
    })
}

/// Produces the shell input that emits the command-completion marker for one protocol.
fn ptyCommandInput(command: &str, commandProtocol: PtyCommandProtocol) -> String {
    let command = terminal_command::posixCommandLine(command);
    match commandProtocol {
        PtyCommandProtocol::BashPromptCommand => format!("{command}\r"),
        PtyCommandProtocol::ExplicitMarker => format!(
            "{command}; __operit_status=$?; {}\r",
            ptyPromptMarkerCommand()
        ),
    }
}

/// Returns the POSIX sh command that writes one machine-readable completion marker.
fn ptyPromptMarkerCommand() -> &'static str {
    "printf '\\033]133;OperitPrompt=plain:%s:%s\\007' \"$PWD\" \"$__operit_status\""
}

/// Writes one command-completion marker command into an interactive shell PTY.
fn writePtyPromptMarker(writer: &Arc<Mutex<Box<dyn Write + Send>>>) -> HostResult<()> {
    let markerCommand = format!("__operit_status=$?; {}\r", ptyPromptMarkerCommand());
    let mut writer = writer
        .lock()
        .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
    writer.write_all(markerCommand.as_bytes())?;
    writer.flush()?;
    Ok(())
}

/// Builds an interactive PTY command for one configured POSIX shell.
fn posixPtyCommand(workingDir: &str, commandSpec: &PtyCommandSpec) -> CommandBuilder {
    let mut command = CommandBuilder::new(&commandSpec.program);
    for argument in &commandSpec.args {
        command.arg(argument);
    }
    match &commandSpec.processWorkingDirectory {
        PtyProcessWorkingDirectory::Session => command.cwd(workingDir),
        PtyProcessWorkingDirectory::Fixed(directory) => command.cwd(directory),
    }
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("LANG", "C.UTF-8");
    for (key, value) in &commandSpec.environment {
        command.env(key, value);
    }
    match &commandSpec.sessionWorkingDirectoryEnvironment {
        PtySessionWorkingDirectoryEnvironment::None => {}
        PtySessionWorkingDirectoryEnvironment::Name(key) => command.env(key, workingDir),
    }
    // Isolation is selected by the owning Host from its actual signature.
    // Unrestricted macOS shells retain controlling-terminal signals and job control.
    command.set_controlling_tty(commandSpec.controllingTty);

    command.env(
        "PS1",
        match commandSpec.commandProtocol {
            PtyCommandProtocol::BashPromptCommand => "$PWD $ ",
            PtyCommandProtocol::ExplicitMarker => "sh$ ",
        },
    );
    if commandSpec.commandProtocol == PtyCommandProtocol::BashPromptCommand {
        command.env(
            "PROMPT_COMMAND",
            r#"__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$(printf '%s' "$PWD" | base64 | tr -d '\n')" "$__operit_status""#,
        );
    }
    command
}

#[allow(non_snake_case)]
fn ptySize(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

#[allow(non_snake_case)]
/// Uses the session-owned event buffer for command output.
fn appendPtyCommandOutput(output: &PtyCommandOutput, data: &[u8]) { output.append(data); }

#[allow(non_snake_case)]
/// Answers terminal cursor queries on the finite OS boundary without blocking output observation.
fn processPtyTerminalQueries(
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    cursorState: &PtyCursorState,
    data: &[u8],
) -> HostResult<()> {
    scanPtyOutput(data, || {
        let response = {
            let cursor = cursorState.lock().map_err(|_| HostError::new("PTY cursor state poisoned"))?;
            format!("\x1b[{};{}R", cursor.row, cursor.col)
        };
        let writer = writer.clone();
        operit_host_native_scheduler::scheduleNativeBlockingTask("terminal-cursor-response", move || {
            let mut writer = writer.lock().map_err(|_| HostError::new("PTY writer state poisoned"))?;
            writer.write_all(response.as_bytes())?;
            writer.flush().map_err(HostError::from)
        })?;
        Ok(())
    }, |value| updatePtyCursor(cursorState, value))
}

#[allow(non_snake_case)]
/// Uses the session-owned event buffer for command output.
fn clearPtyCommandOutput(output: &PtyCommandOutput) -> HostResult<()> { output.clear() }



#[derive(Clone, Debug)]
struct PtyPromptMarker {
    workingDir: String,
    exitCode: i32,
}

#[allow(non_snake_case)]
fn findLastPtyPromptMarker(data: &[u8]) -> HostResult<Option<PtyPromptMarker>> {
    let Some(start) = rfindBytes(data, PTY_PROMPT_MARKER_PREFIX) else {
        return Ok(None);
    };
    let payloadStart = start + PTY_PROMPT_MARKER_PREFIX.len();
    let Some(relativeEnd) = data[payloadStart..]
        .iter()
        .position(|value| *value == PTY_PROMPT_MARKER_END)
    else {
        return Ok(None);
    };
    let payloadEnd = payloadStart + relativeEnd;
    let payload = std::str::from_utf8(&data[payloadStart..payloadEnd])
        .map_err(|error| HostError::new(format!("Invalid PTY prompt marker UTF-8: {error}")))?;
    let (workingDir, exitCodeText) = if let Some(plainPayload) = payload.strip_prefix("plain:") {
        let (workingDir, exitCode) = plainPayload
            .rsplit_once(':')
            .ok_or_else(|| HostError::new(format!("Invalid PTY prompt marker '{payload}'")))?;
        (workingDir.to_string(), exitCode)
    } else {
        let (workingDirText, exitCode) = payload
            .split_once(':')
            .ok_or_else(|| HostError::new(format!("Invalid PTY prompt marker '{payload}'")))?;
        let workingDirBytes = BASE64_STANDARD
            .decode(workingDirText.as_bytes())
            .map_err(|error| HostError::new(format!("Invalid PTY prompt cwd marker: {error}")))?;
        let workingDir = String::from_utf8(workingDirBytes)
            .map_err(|error| HostError::new(format!("Invalid PTY prompt cwd UTF-8: {error}")))?;
        (workingDir, exitCode)
    };
    let exitCode = parseExitCode(exitCodeText)?;
    Ok(Some(PtyPromptMarker {
        workingDir,
        exitCode,
    }))
}

#[allow(non_snake_case)]
fn ptyCommandOutputText(
    raw: &[u8],
    command: &str,
    workingDir: Option<&str>,
    commandProtocol: PtyCommandProtocol,
) -> String {
    let visibleBytes = stripPtyPromptMarkers(raw);
    let text = String::from_utf8_lossy(&visibleBytes);
    let clean = renderTerminalText(&text);
    let withoutEcho = dropCommandEcho(clean, command);
    dropTrailingPtyPrompt(withoutEcho, workingDir, commandProtocol)
}

#[allow(non_snake_case)]
/// Renders a copied screen without retaining the terminal output lock.
fn ptyScreenContent(output: &Arc<Mutex<VecDeque<u8>>>) -> HostResult<String> {
    let buffer = output.lock()
        .map_err(|_| HostError::new("pty screen output mutex poisoned"))?;
    let bytes = buffer.iter().copied().collect::<Vec<_>>();
    drop(buffer);
    let visibleBytes = stripPtyPromptMarkers(&bytes);
    let text = String::from_utf8_lossy(&visibleBytes);
    Ok(renderTerminalText(&text))
}

#[allow(non_snake_case)]
fn dropTrailingPtyPrompt(
    value: String,
    workingDir: Option<&str>,
    commandProtocol: PtyCommandProtocol,
) -> String {
    match commandProtocol {
        PtyCommandProtocol::BashPromptCommand => {
            let Some(workingDir) = workingDir else {
                return value.trim().to_string();
            };
            let prompt = format!("{workingDir} $");
            let mut lines = value.lines().map(str::to_string).collect::<Vec<_>>();
            while lines.last().is_some_and(|line| line.trim().is_empty()) {
                lines.pop();
            }
            if lines
                .last()
                .is_some_and(|line| line.trim_end() == prompt.as_str())
            {
                lines.pop();
            }
            lines.join("\n").trim().to_string()
        }
        PtyCommandProtocol::ExplicitMarker => value
            .trim_end()
            .strip_suffix("sh$")
            .unwrap_or(value.trim_end())
            .trim()
            .to_string(),
    }
}

#[allow(non_snake_case)]
fn toHostError(error: impl std::fmt::Display) -> HostError {
    HostError::new(error.to_string())
}

/// Validates the terminal implementation owned by the shared POSIX PTY host.
fn requireNativeTerminal(terminal: &str) -> HostResult<()> {
    match terminal.trim() {
        TERMINAL => Ok(()),
        value => Err(HostError::new(format!(
            "Unsupported terminal implementation for shared POSIX PTY host: {value}"
        ))),
    }
}

/// Clears one retained PTY output buffer after protocol initialization.
fn clearPtyOutput(output: &Arc<Mutex<VecDeque<u8>>>) -> HostResult<()> {
    let mut buffer = output
        .lock()
        .map_err(|_| HostError::new("pty output mutex poisoned"))?;
    buffer.clear();
    Ok(())
}

#[allow(non_snake_case)]
fn removeSessionMappings(state: &mut TerminalState, sessionId: &str) {
    state.sessionNameToId.retain(|_, value| value != sessionId);
    state
        .hiddenExecutorKeyToSessionId
        .retain(|_, value| value != sessionId);
}



#[allow(non_snake_case)]
fn nextSessionId() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use operit_host_api::TerminalHost;

    /// ECH with the cursor past the end of a short line must not panic, even
    /// when the erase count would overflow usize.
    #[test]
    fn renderer_erase_chars_tolerates_cursor_past_line_end() {
        assert_eq!(renderTerminalText("ab\x1b[1;10H\x1b[3X"), "ab");
        assert_eq!(
            renderTerminalText("ab\x1b[1;10H\x1b[18446744073709551615X"),
            "ab"
        );
    }

    /// An exited hidden shell is replaced before a new command is submitted.
    #[test]
    fn hidden_executor_replaces_an_exited_shell() {
        let host = NativePtyTerminalHost::new();
        let first = host.executeHiddenCommand("printf 'first\\n'", "recover", 3000).unwrap();
        assert!(first.output.contains("first"));
        let key = sessionKey(&host.commandSpec.terminalType, "recover");
        let old = {
            let mut state = host.lockState().unwrap();
            let id = state.hiddenExecutorKeyToSessionId[&key].clone();
            let process = state.ptySessions.get_mut(&id).unwrap().process.as_mut().unwrap();
            process.child.kill().unwrap();
            process.child.wait().unwrap();
            // Do not discard a shell while its previous command is still being observed.
            state.ptySessions.get_mut(&id).unwrap().commandRunning = true;
            assert_eq!(liveHiddenPtySessionId(&mut state, &key).unwrap(), Some(id.clone()));
            state.ptySessions.get_mut(&id).unwrap().commandRunning = false;
            id
        };
        let second = host.executeHiddenCommand("printf 'second\\n'", "recover", 3000).unwrap();
        assert!(second.output.contains("second"));
        let state = host.lockState().unwrap();
        let new = &state.hiddenExecutorKeyToSessionId[&key];
        assert_ne!(new, &old);
        assert!(!state.ptySessions.contains_key(&old));
    }

    /// Concurrent first use keeps only one mapped shell for the executor key.
    #[test]
    fn concurrent_hidden_executor_creation_does_not_orphan_sessions() {
        let host = NativePtyTerminalHost::new();
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let threads: Vec<_> = (0..4).map(|_| {
            let host = host.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let kind = &host.commandSpec.terminalType;
                hiddenPtySessionId(&host, &sessionKey(kind, "concurrent"), "concurrent", kind).unwrap()
            })
        }).collect();
        let ids: Vec<_> = threads.into_iter().map(|thread| thread.join().unwrap()).collect();
        assert!(ids.iter().all(|id| id == &ids[0]));
        let state = host.lockState().unwrap();
        assert_eq!(state.ptySessions.len(), 1);
        assert_eq!(state.hiddenExecutorKeyToSessionId.len(), 1);
    }

    /// The name and command state are checked under the same lock as removal.
    #[test]
    fn idle_close_rejects_wrong_owner_and_running_command() {
        let host = NativePtyTerminalHost::new();
        let name = "maintenance_child";
        let session = host.createOrGetSession(name).unwrap();
        assert!(host.closeSessionIfIdle(&session.sessionId, "other_child").is_err());
        let command = host.beginCommand(&session.sessionId, "printf busy", 5000).unwrap();
        assert!(host.closeSessionIfIdle(&session.sessionId, name).is_err());
        assert!(!operit_host_native_scheduler::runTerminalCommandSynchronously(Box::new(command), &mut |_| Ok(())).unwrap().timedOut);
        assert!(host.closeSessionIfIdle(&session.sessionId, name).unwrap().success);
        assert!(!host.ownsSession(&session.sessionId).unwrap());
        let generated = host.createOrGetSession("maintenance_child_generation_2").unwrap();
        assert!(host.closeSessionIfIdle(&generated.sessionId, name).unwrap().success);
        let other = host.createOrGetSession("maintenance_child_generation_2_extra").unwrap();
        assert!(host.closeSessionIfIdle(&other.sessionId, name).is_err());
        host.closeSession(&other.sessionId).unwrap();
    }

    /// Runs every statement before completing and keeps changes in the existing shell.
    #[test]
    fn posix_multiline_command_preserves_shell_state() {
        let host = NativePtyTerminalHost::new();
        let session = host.createOrGetSession("terminal_multiline").unwrap();
        let result = host.executeInSession(
            &session.sessionId,
            "OPERIT_V=$(cat <<'EOF'\n中文 'quoted' \\012 $literal\nEOF\n)\nsleep 1\nprintf '%s\\n' \"$OPERIT_V\"",
            5000,
        ).unwrap();
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "中文 'quoted' \\012 $literal");
        let next = host
            .executeInSession(&session.sessionId, "printf '%s\\n' \"$OPERIT_V\"", 3000)
            .unwrap();
        assert_eq!(next.output, "中文 'quoted' \\012 $literal");
    }

    /// Submits literal history characters through both interactive Bash protocols.
    #[test]
    fn posix_history_characters_execute_without_expansion() {
        for protocol in [
            PtyCommandProtocol::BashPromptCommand,
            PtyCommandProtocol::ExplicitMarker,
        ] {
            let mut host = NativePtyTerminalHost::new();
            Arc::make_mut(&mut host.commandSpec).commandProtocol = protocol;
            let session = host.createOrGetSession("terminal_history").unwrap();
            let result = host
                .executeInSession(
                    &session.sessionId,
                    "#!/bin/bash\nOPERIT_HISTORY_LITERAL='中文 ! !! !$ !123'\nprintf '%s\\n' \"$OPERIT_HISTORY_LITERAL\"",
                    3000,
                )
                .unwrap();
            assert!(!result.timedOut);
            assert_eq!(result.exitCode, 0);
            assert_eq!(result.output.lines().last(), Some("中文 ! !! !$ !123"));

            let next = host
                .executeInSession(
                    &session.sessionId,
                    "printf '%s\\n' \"$OPERIT_HISTORY_LITERAL\" 'single line!'",
                    3000,
                )
                .unwrap();
            assert!(!next.timedOut);
            assert_eq!(next.exitCode, 0);
            assert!(next.output.ends_with("中文 ! !! !$ !123\nsingle line!"));
            host.closePtySession(&session.sessionId).unwrap();
        }
    }

    /// A quote followed by an octal digit must not merge into the printf escape.
    #[test]
    fn posix_quote_before_octal_digit_is_not_swallowed() {
        let host = NativePtyTerminalHost::new();
        let session = host.createOrGetSession("terminal_quote_octal").unwrap();
        let result = host
            .executeInSession(
                &session.sessionId,
                "OPERIT_HASHES='327429f1 5c8a733d'\nprintf '%s\\n' \"$OPERIT_HASHES\"",
                3000,
            )
            .unwrap();
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "327429f1 5c8a733d");
    }

    /// Applies the timeout to the complete multiline script and keeps the shell responsive.
    #[test]
    fn posix_multiline_timeout_keeps_session_usable() {
        let host = NativePtyTerminalHost::new();
        let session = host
            .createOrGetSession("terminal_multiline_timeout")
            .unwrap();
        let result = host
            .executeInSession(
                &session.sessionId,
                "printf 'started\\n'\nsleep 8\nprintf forbidden",
                800,
            )
            .unwrap();
        assert!(result.timedOut);
        let next = host
            .executeInSession(&session.sessionId, "printf 'ready\\n'", 3000)
            .unwrap();
        assert!(!next.timedOut);
        assert_eq!(next.output, "ready");
    }

    #[test]
    fn linux_command_block_completes_in_pty() {
        let host = NativePtyTerminalHost::new();
        let session = host
            .createOrGetSession("linux_terminal_marker")
            .expect("create terminal session");
        let result = host
            .executeInSession(
                &session.sessionId,
                "printf 'hello\\n'; [ -t 0 ] && echo tty=yes || echo tty=no",
                3000,
            )
            .expect("execute terminal command");

        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "hello\ntty=yes");
    }

    #[test]
    fn linux_screen_records_prompt_command_output_prompt() {
        let host = NativePtyTerminalHost::new();
        let session = host
            .createOrGetSession("linux_terminal_screen")
            .expect("create terminal session");
        let workingDir = std::env::current_dir().unwrap().display().to_string();
        let prompt = format!("{workingDir} $ ");
        let command = "printf 'screen-ok\\n'";

        let initialScreen = host
            .getSessionScreen(&session.sessionId)
            .expect("get initial screen");
        assert_eq!(initialScreen.content, prompt);

        let result = host
            .executeInSession(&session.sessionId, command, 3000)
            .expect("execute terminal command");
        let screen = host
            .getSessionScreen(&session.sessionId)
            .expect("get terminal screen");

        assert!(!result.timedOut);
        assert_eq!(result.output, "screen-ok");
        assert_eq!(
            screen.content.split('\n').collect::<Vec<_>>(),
            vec![
                format!("{prompt}{command}"),
                "screen-ok".to_string(),
                prompt
            ]
        );
        assert!(!screen.commandRunning);
    }

    #[test]
    fn visible_linux_sessions_are_listed_as_pty() {
        let host = NativePtyTerminalHost::new();
        let created = host
            .createOrGetSession("linux_visible_ai")
            .expect("create visible terminal session");
        let manual = host
            .startPtySession(
                "linux_visible_manual",
                TERMINAL,
                PRIMARY_TERMINAL_TYPE,
                &std::env::current_dir().unwrap().display().to_string(),
                24,
                80,
            )
            .expect("start manual terminal session");

        let sessions = host.listSessions().expect("list terminal sessions");
        let createdEntry = sessions
            .iter()
            .find(|entry| entry.sessionId == created.sessionId)
            .expect("created terminal listed");
        let manualEntry = sessions
            .iter()
            .find(|entry| entry.sessionId == manual)
            .expect("manual terminal listed");

        assert_eq!(createdEntry.sessionKind, "pty");
        assert_eq!(createdEntry.terminalType, PRIMARY_TERMINAL_TYPE);
        assert_eq!(manualEntry.sessionKind, "pty");
        assert_eq!(manualEntry.terminalType, PRIMARY_TERMINAL_TYPE);
    }

    #[test]
    fn linux_pty_session_preserves_working_directory() {
        let host = NativePtyTerminalHost::new();
        let session = host
            .createOrGetSession("linux_terminal_cwd")
            .expect("create terminal session");

        let cdResult = host
            .executeInSession(&session.sessionId, "cd /tmp", 3000)
            .expect("cd in terminal session");
        let pwdResult = host
            .executeInSession(&session.sessionId, "pwd", 3000)
            .expect("pwd in terminal session");

        assert!(!cdResult.timedOut);
        assert_eq!(cdResult.exitCode, 0);
        assert_eq!(pwdResult.output, "/tmp");
        assert_eq!(pwdResult.exitCode, 0);
    }
}
