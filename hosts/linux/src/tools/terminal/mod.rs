#[path = "../../../../common/terminal_command.rs"]
mod terminal_command;

use operit_host_api::terminalSessionKey as sessionKey;
use operit_host_api::TerminalText::{appendPtyOutput, dropCommandEcho, nonBlank, parseExitCode, renderTerminalText, rfindBytes, stripPtyDeviceStatusReports, stripPtyPromptMarkers, scanPtyOutput, updatePtyCursor, PtyCursor};
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

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
const PTY_OUTPUT_LIMIT: usize = 1024 * 1024;
const PTY_PROMPT_MARKER_PREFIX: &[u8] = b"\x1b]133;OperitPrompt=";
const PTY_PROMPT_MARKER_END: u8 = 7;
const PLATFORM: &str = "linux";
const TERMINAL: &str = "native";
const PRIMARY_TERMINAL_TYPE: &str = "bash";

#[derive(Clone, Default)]
pub struct LinuxTerminalHost {
    state: Arc<Mutex<TerminalState>>,
    profile: Option<operit_host_api::TerminalProfile>,
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
    child: Box<dyn portable_pty::Child + Send + Sync>,
    readerTask: Option<operit_host_native_common::NativeFdReader>,
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: PtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    commandRunning: bool,
    processOwned: bool,
    exitCode: Option<i32>,
}

type PtyCommandOutput = Arc<operit_host_api::TerminalCommand::TerminalCommandBuffer>;

type PtyCursorState = Arc<Mutex<PtyCursor>>;

impl Drop for PtySession {
    /// Wakes command observers before terminating the owning interpreter.
    fn drop(&mut self) {
        self.commandOutput.fail(HostError::new("Terminal session was closed"));
        let _ = self.child.kill();
        if let Some(reader) = self.readerTask.take() {
            if let Err(error) = reader.close() { operit_host_api::logHostError("terminal-reader-close", &error.message); }
        }
    }
}

impl LinuxTerminalHost {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TerminalHost for LinuxTerminalHost {
    /// Starts a fresh PTY process with a literal script and an explicit initial directory.
    fn openTerminalSession(&self, request: &operit_host_api::TerminalSessionRequest) -> HostResult<TerminalSessionInfo> {
        requireNativeTerminal(&request.terminal)?;
        let kind = normalizeTerminalType(&request.terminalType)?;
        let name = nonBlank(&request.sessionName, "session_name")?;
        let directory = match &request.workingDir { Some(directory) => directory.clone(), None => std::env::current_dir()?.to_string_lossy().into_owned() };
        let session = createPtyProcess(self.profile.as_ref(), name.clone(), kind.clone(), directory, 24, 80, request.command.as_deref(), false)?;
        let id = nextSessionId();
        self.lockState()?.ptySessions.insert(id.clone(), session);
        Ok(TerminalSessionInfo { sessionId: id, sessionName: name, platform: PLATFORM.into(), terminal: TERMINAL.into(), terminalType: kind, isNewSession: true })
    }

    /// Registers custom bash executables and launcher argument prefixes on Linux.
    fn terminalProfileSupport(&self) -> HostResult<Vec<operit_host_api::TerminalProfileSupport>> {
        Ok(vec![operit_host_api::TerminalProfileSupport {
            terminal: TERMINAL.into(),
            terminalType: PRIMARY_TERMINAL_TYPE.into(),
            fields: vec!["executable".into(), "arguments".into()],
            fieldChoices: None,
        }])
    }
    /// Creates an isolated registry for one exact custom bash launcher.
    fn configuredTerminal(
        &self,
        profile: &operit_host_api::TerminalProfile,
    ) -> HostResult<Arc<dyn TerminalHost>> {
        requireNativeTerminal(&profile.terminal)?;
        normalizeTerminalType(&profile.terminalType)?;
        let executable = profile
            .options
            .get("executable")
            .ok_or_else(|| HostError::new("Custom terminal requires executable"))?;
        if !std::path::Path::new(executable).is_absolute()
            || !std::path::Path::new(executable).is_file()
        {
            return Err(HostError::new(
                "Terminal executable must be an existing absolute file path",
            ));
        }
        operit_host_api::TerminalProfiles::profileArguments(profile)?;
        Ok(Arc::new(Self {
            state: Arc::default(),
            profile: Some(profile.clone()),
        }))
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
        normalizeTerminalType(terminalType)?;
        resolveHostDirectory(workingDir)
    }

    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
            types: vec![TerminalTypeInfo {
                terminal: TERMINAL.to_string(),
                terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
                available: true,
                description: "Linux bash terminal".to_string(),
                nameKey: Some("terminalBuiltinLinux".into()),
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
        let normalizedTerminalType = normalizeTerminalType(terminalType)?;
        let workDir = nonBlank(workingDir, "working_directory")?;
        let session = createPtySession(
            self.profile.as_ref(),
            normalizedSessionName,
            normalizedTerminalType,
            workDir,
            rows,
            cols,
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
        let mut output = session
            .output
            .lock()
            .map_err(|_| HostError::new("pty output mutex poisoned"))?;
        Ok(output.drain(..).collect())
    }

    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        let writer = {
            let state = self.lockState()?;
            let session = state.ptySessions.get(sessionId)
                .ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?;
            session.writer.clone()
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
        if session.exitCode.is_some() {
            return Ok(session.exitCode);
        }
        match session.child.try_wait()? {
            Some(status) => {
                let code = status.exit_code() as i32;
                session.exitCode = Some(code);
                session.commandRunning = false;
                Ok(Some(code))
            }
            None => Ok(None),
        }
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
            if session.exitCode.is_none() {
                if let Some(status) = session.child.try_wait()? {
                    session.exitCode = Some(status.exit_code() as i32);
                }
            }
            if session.exitCode.is_some() { session.commandRunning = false; }
            if session.exitCode.is_some() && !session.processOwned {
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
        self.createOrGetTypedSession(sessionName, PRIMARY_TERMINAL_TYPE)
    }

    /// Creates or reuses a named session of exactly the requested interpreter type.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        let normalizedTerminalType = normalizeTerminalType(terminalType)?;
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

        let workingDir = std::env::current_dir()?.display().to_string();
        let session = createPtySession(
            self.profile.as_ref(),
            normalizedSessionName.clone(),
            normalizedTerminalType.clone(),
            workingDir,
            24,
            80,
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
        operit_host_native_common::runTerminalCommandSynchronously(
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
        let normalizedTerminalType = PRIMARY_TERMINAL_TYPE.to_string();
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
        if session.processOwned && session.exitCode.is_none() {
            session.exitCode = session.child.try_wait()?.map(|status| status.exit_code() as i32);
            if session.exitCode.is_some() { session.commandRunning = false; }
        }
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

impl LinuxTerminalHost {
    #[allow(non_snake_case)]
    fn lockState(&self) -> HostResult<std::sync::MutexGuard<'_, TerminalState>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("terminal state mutex poisoned"))
    }
}

#[allow(non_snake_case)]
fn hiddenPtySessionId(
    host: &LinuxTerminalHost,
    executorKey: &str,
    executorLabel: &str,
    terminalType: &str,
) -> HostResult<String> {
    {
        let mut state = host.lockState()?;
        if let Some(sessionId) = state.hiddenExecutorKeyToSessionId.get(executorKey).cloned() {
            if state.ptySessions.contains_key(&sessionId) {
                return Ok(sessionId);
            }
            state.hiddenExecutorKeyToSessionId.remove(executorKey);
            removeSessionMappings(&mut state, &sessionId);
        }
    }

    let workingDir = std::env::current_dir()?.display().to_string();
    let session = createPtySession(
        host.profile.as_ref(),
        format!("hidden:{executorLabel}"),
        terminalType.to_string(),
        workingDir,
        24,
        80,
    )?;
    let sessionId = nextSessionId();
    let mut state = host.lockState()?;
    state
        .hiddenExecutorKeyToSessionId
        .insert(executorKey.to_string(), sessionId.clone());
    state.ptySessions.insert(sessionId.clone(), session);
    Ok(sessionId)
}

/// Owns command parsing independently of the executor thread awaiting terminal events.
struct CooperativeCommand {
    host: LinuxTerminalHost,
    sessionId: String,
    command: String,
    terminalType: String,
    commandEcho: String,
    commandOutput: PtyCommandOutput,
    timeoutMs: u64,
    collected: Vec<u8>,
    workingDir: Option<String>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl LinuxTerminalHost {
    /// Submits one command and reserves its session before asynchronous observation begins.
    fn beginCommand(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<CooperativeCommand> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let normalizedCommand = nonBlank(command, "command")?;
        let (terminalType, commandOutput, writer) = {
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
            let commandLine = terminal_command::posixCommandLine(&normalizedCommand);
            let commandInput = format!("{commandLine}\r");
            let mut writer = session
                .writer
                .lock()
                .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
            writer.write_all(commandInput.as_bytes())?;
            writer.flush()?;
            session.commandRunning = true;
            (
                session.terminalType.clone(),
                session.commandOutput.clone(),
                session.writer.clone(),
            )
        };

        Ok(CooperativeCommand {
            host: self.clone(),
            commandEcho: terminal_command::posixCommandLine(&normalizedCommand),
            sessionId: normalizedSessionId,
            command: normalizedCommand,
            terminalType,
            commandOutput,
            timeoutMs,
            collected: Vec::new(),
            workingDir: None,
            writer,
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
                        ptyCommandOutputText(&self.collected, &self.commandEcho, None),
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
    profile: Option<&operit_host_api::TerminalProfile>,
    sessionName: String,
    terminalType: String,
    workingDir: String,
    rows: u16,
    cols: u16,
) -> HostResult<PtySession> {
    createPtyProcess(profile, sessionName, terminalType, workingDir, rows, cols, None, true)
}

/// Shares the PTY transport while separating legacy prompt tracking from process execution.
fn createPtyProcess(profile: Option<&operit_host_api::TerminalProfile>, sessionName: String, terminalType: String,
    workingDir: String, rows: u16, cols: u16, script: Option<&str>, managedPrompt: bool) -> HostResult<PtySession> {
    let ptySystem = native_pty_system();
    let pair = ptySystem
        .openpty(ptySize(rows, cols))
        .map_err(toHostError)?;
    let mut command = linuxProfilePtyCommand(profile, &workingDir)?;
    if !managedPrompt {
        command.env_remove("PS1");
        command.env_remove("PROMPT_COMMAND");
        if let Some(script) = script {
            command.get_argv_mut().pop();
            command.args(["-c", script]);
        }
    }
    let mut child = pair.slave.spawn_command(command).map_err(toHostError)?;
    drop(pair.slave);
    use std::os::fd::BorrowedFd;
    let fd = pair.master.as_raw_fd().ok_or_else(|| HostError::new("PTY master has no file descriptor"))?;
    let reader = unsafe { BorrowedFd::borrow_raw(fd) }.try_clone_to_owned()?;
    let writer = unsafe { BorrowedFd::borrow_raw(fd) }.try_clone_to_owned()?;
    let writer = Arc::new(Mutex::new(Box::new(operit_host_native_common::NativeFdWriter(std::fs::File::from(writer))) as Box<dyn Write + Send>));
    let output = Arc::new(Mutex::new(VecDeque::new()));
    let commandOutput: PtyCommandOutput = Arc::new(operit_host_api::TerminalCommand::TerminalCommandBuffer::new(PTY_OUTPUT_LIMIT));
    let screenOutput = Arc::new(Mutex::new(VecDeque::new()));
    let cursorState: PtyCursorState = Arc::new(Mutex::new(PtyCursor { row: 1, col: 1 }));
    let outputForThread = output.clone();
    let commandOutputForThread = commandOutput.clone();
    let screenOutputForThread = screenOutput.clone();
    let writerForThread = writer.clone();
    let cursorStateForThread = cursorState.clone();
    let closed = commandOutput.clone();
    let readerTask = operit_host_native_common::scheduleNativeFdReader("operit-pty-reader", reader, move |chunk| {
        processPtyTerminalQueries(&writerForThread, &cursorStateForThread, chunk)?;
        let commandChunk = stripPtyDeviceStatusReports(chunk);
        let visibleChunk = stripPtyPromptMarkers(&commandChunk);
        appendPtyOutput(&outputForThread, &visibleChunk, PTY_OUTPUT_LIMIT);
        appendPtyOutput(&screenOutputForThread, &visibleChunk, PTY_OUTPUT_LIMIT);
        if managedPrompt { appendPtyCommandOutput(&commandOutputForThread, &commandChunk); }
        Ok(())
    }, move |error| closed.fail(error));
    let readerTask = match readerTask {
        Ok(reader) => reader,
        Err(error) => { let _ = child.kill(); return Err(error); }
    };
    if managedPrompt {
    if let Err(error) = operit_host_native_common::waitForTerminalPrompt(
        &commandOutput,
        10000,
        |bytes| findLastPtyPromptMarker(bytes).map(|marker| marker.is_some()),
        |_| HostError::new("Timed out waiting for terminal prompt"),
    )
    {
        let _ = child.kill();
        return Err(error);
    }
    clearPtyCommandOutput(&commandOutput)?;
    }
    Ok(PtySession {
        sessionName,
        terminalType,
        workingDir,
        child,
        readerTask: Some(readerTask),
        master: pair.master,
        writer,
        output,
        commandOutput,
        screenOutput,
        commandRunning: !managedPrompt,
        processOwned: !managedPrompt,
        exitCode: None,
    })
}

fn linuxPtyCommand(workingDir: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("bash");
    command.arg("--noprofile");
    command.arg("--norc");
    command.arg("-i");
    command.cwd(workingDir);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("LANG", "C.UTF-8");
    command.env("PS1", "$PWD $ ");
    command.env(
        "PROMPT_COMMAND",
        r#"__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$(printf '%s' "$PWD" | base64 | tr -d '\n')" "$__operit_status""#,
    );
    command
}

/// Builds an explicit launcher while retaining the host's existing bash completion protocol.
fn linuxProfilePtyCommand(profile: Option<&operit_host_api::TerminalProfile>, workingDir: &str) -> HostResult<CommandBuilder> {
    let Some(profile) = profile else { return Ok(linuxPtyCommand(workingDir)); };
    let executable = profile.options.get("executable").ok_or_else(|| HostError::new("Terminal executable is required"))?;
    let mut command = CommandBuilder::new(executable);
    for argument in operit_host_api::TerminalProfiles::profileArguments(profile)? { command.arg(argument); }
    for argument in ["--noprofile", "--norc", "-i"] { command.arg(argument); }
    command.cwd(workingDir);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("LANG", "C.UTF-8");
    command.env("PS1", "$PWD $ ");
    command.env("PROMPT_COMMAND", r#"__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$(printf '%s' "$PWD" | base64 | tr -d '\n')" "$__operit_status""#);
    Ok(command)
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
        operit_host_native_common::scheduleNativeBlockingTask("terminal-cursor-response", move || {
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
    let (workingDirText, exitCodeText) = payload
        .split_once(':')
        .ok_or_else(|| HostError::new(format!("Invalid PTY prompt marker '{payload}'")))?;
    let workingDirBytes = BASE64_STANDARD
        .decode(workingDirText.as_bytes())
        .map_err(|error| HostError::new(format!("Invalid PTY prompt cwd marker: {error}")))?;
    let workingDir = String::from_utf8(workingDirBytes)
        .map_err(|error| HostError::new(format!("Invalid PTY prompt cwd UTF-8: {error}")))?;
    let exitCode = parseExitCode(exitCodeText)?;
    Ok(Some(PtyPromptMarker {
        workingDir,
        exitCode,
    }))
}

#[allow(non_snake_case)]
fn ptyCommandOutputText(raw: &[u8], command: &str, workingDir: Option<&str>) -> String {
    let visibleBytes = stripPtyPromptMarkers(raw);
    let text = String::from_utf8_lossy(&visibleBytes);
    let clean = renderTerminalText(&text);
    let withoutEcho = dropCommandEcho(clean, command);
    dropTrailingLinuxPrompt(withoutEcho, workingDir)
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
fn dropTrailingLinuxPrompt(value: String, workingDir: Option<&str>) -> String {
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

#[allow(non_snake_case)]
fn toHostError(error: impl std::fmt::Display) -> HostError {
    HostError::new(error.to_string())
}

#[allow(non_snake_case)]
fn normalizeTerminalType(terminalType: &str) -> HostResult<String> {
    match terminalType.trim() {
        PRIMARY_TERMINAL_TYPE => Ok(PRIMARY_TERMINAL_TYPE.to_string()),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for linux host: {value}"
        ))),
    }
}

/// Validates the terminal implementation owned by the Linux PTY host.
fn requireNativeTerminal(terminal: &str) -> HostResult<()> {
    match terminal.trim() {
        TERMINAL => Ok(()),
        value => Err(HostError::new(format!(
            "Unsupported terminal implementation for Linux PTY host: {value}"
        ))),
    }
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

    #[test]
    fn linux_command_block_completes_in_pty() {
        let host = LinuxTerminalHost::new();
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
        let host = LinuxTerminalHost::new();
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
        let host = LinuxTerminalHost::new();
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
        let host = LinuxTerminalHost::new();
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
