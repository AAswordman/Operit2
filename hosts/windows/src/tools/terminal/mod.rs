use operit_host_api::terminalSessionKey as sessionKey;
use operit_host_api::TerminalText::{appendPtyOutput, csiParams, csiValue, nonBlank, parseExitCode, rfindBytes, stripPtyDeviceStatusReports, stripPtyPromptMarkers, scanPtyOutput, updatePtyCursor, PtyCursor};
#[path = "../../../../common/terminal_command.rs"]
mod terminal_command;
mod conpty;
mod powershell_runtime;
mod wsl;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::process::Command as CommandBuilder;
#[cfg(test)]
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput, TerminalSessionInfo,
    TerminalSessionListEntry, TerminalTypeInfo,
};
use operit_host_api::HostRuntimeTaskSchedulerHost;
use uuid::Uuid;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
const PTY_OUTPUT_LIMIT: usize = 1024 * 1024;
const PTY_PROMPT_MARKER_PREFIX: &[u8] = b"\x1b]133;OperitPrompt=";
const PTY_PROMPT_MARKER_END: u8 = 7;
const PLATFORM: &str = "windows";
const TERMINAL: &str = "native";
const PRIMARY_TERMINAL_TYPE: &str = "powershell";

#[derive(Clone, Default)]
pub struct WindowsTerminalHost {
    state: Arc<Mutex<TerminalState>>,
    powershell: PowerShellSource,
    profile: Option<operit_host_api::TerminalProfile>,
}

#[derive(Clone, Default)]
enum PowerShellSource {
    #[default]
    Bundled,
    OnDemand(PathBuf),
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
    kind: TerminalKind,
    workingDir: String,
    process: conpty::Process,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: PtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    commandRunning: bool,
    processOwned: bool,
    exitCode: Option<i32>,
    scriptFile: Option<tempfile::TempPath>,
}

type PtyCommandOutput = Arc<operit_host_api::TerminalCommand::TerminalCommandBuffer>;

type PtyCursorState = Arc<Mutex<PtyCursor>>;

impl Drop for PtySession {
    /// Reaps the interpreter before releasing a session's workspace and runtime files.
    fn drop(&mut self) {
        self.commandOutput.fail(HostError::new("Terminal session was closed"));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalKind {
    Bash,
    PowerShell,
}

impl WindowsTerminalHost {
    /// Creates a terminal host for a standalone package with a bundled interpreter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an application terminal host with an on-demand interpreter cache.
    pub fn with_on_demand_powershell(runtime_root: &Path) -> Self {
        Self {
            state: Arc::default(),
            powershell: PowerShellSource::OnDemand(runtime_root.join("cache/powershell")),
            profile: None,
        }
    }
}

impl TerminalHost for WindowsTerminalHost {
    /// Starts a complete command directly through ConPTY without typing into a line editor.
    fn openTerminalSession(&self, request: &operit_host_api::TerminalSessionRequest) -> HostResult<TerminalSessionInfo> {
        requireNativeTerminal(&request.terminal)?;
        let (terminalType, kind) = normalizeTerminalType(&request.terminalType)?;
        let name = nonBlank(&request.sessionName, "session_name")?;
        let directory = match &request.workingDir { Some(directory) => directory.clone(), None => std::env::current_dir()?.to_string_lossy().into_owned() };
        let session = createPtyProcess(&self.powershell, self.profile.as_ref(), name.clone(), terminalType.clone(), kind, directory, 24, 80, request.command.as_deref(), false)?;
        let id = nextSessionId();
        self.lockState()?.ptySessions.insert(id.clone(), session);
        Ok(TerminalSessionInfo { sessionId: id, sessionName: name, platform: PLATFORM.into(), terminal: TERMINAL.into(), terminalType, isNewSession: true })
    }

    /// Registers configurable interpreter executables and launcher argument prefixes.
    fn terminalProfileSupport(&self) -> HostResult<Vec<operit_host_api::TerminalProfileSupport>> {
        let distributions = wsl::registered_distributions()?;
        Ok(self
            .terminalInfo()?
            .types
            .into_iter()
            .map(|entry| operit_host_api::TerminalProfileSupport {
                terminal: entry.terminal,
                terminalType: entry.terminalType.clone(),
                fields: if entry.terminalType == "bash" {
                    vec![
                        "executable".into(),
                        "arguments".into(),
                        "wslDistribution".into(),
                    ]
                } else {
                    vec!["executable".into(), "arguments".into()]
                },
                fieldChoices: (entry.terminalType == "bash").then(|| {
                    BTreeMap::from([("wslDistribution".into(), distributions.clone())])
                }),
            })
            .collect())
    }

    /// Uses an explicit interpreter without downloading or selecting another executable.
    fn configuredTerminal(
        &self,
        profile: &operit_host_api::TerminalProfile,
    ) -> HostResult<Arc<dyn TerminalHost>> {
        requireNativeTerminal(&profile.terminal)?;
        normalizeTerminalType(&profile.terminalType)?;
        operit_host_api::TerminalProfiles::profileArguments(profile)?;
        if let Some(distribution) = profile.options.get("wslDistribution") {
            if profile.terminalType != "bash"
                || distribution.trim().is_empty()
                || distribution.starts_with('-')
            {
                return Err(HostError::new(
                    "WSL requires an explicit distribution name and Bash semantics",
                ));
            }
        }
        let executable = configuredExecutable(profile)?;
        if !executable.is_absolute() || !executable.is_file() {
            return Err(HostError::new(format!(
                "Terminal executable must be an existing absolute file path: {}",
                executable.display()
            )));
        }
        Ok(Arc::new(Self {
            state: Arc::default(),
            powershell: self.powershell.clone(),
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
        let native = resolveHostDirectory(workingDir)?;
        match &self.profile {
            Some(profile) if profile.options.contains_key("wslDistribution") => {
                wslDirectory(profile, &native)
            }
            _ => Ok(native),
        }
    }

    /// Lists supported interpreters without initiating any download.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        let (powershell_available, powershell_description) = match &self.powershell {
            PowerShellSource::OnDemand(_) => (true, "PowerShell 7 · downloads on first use".to_string()),
            PowerShellSource::Bundled => match bundledPowerShellExecutable() {
                Ok(_) => (true, "Bundled PowerShell 7 terminal".to_string()),
                Err(error) => (false, error.message),
            },
        };
        let bash = bundledBashExecutable();
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
            types: vec![
                TerminalTypeInfo {
                    terminal: TERMINAL.to_string(),
                    terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
                    available: powershell_available,
                    description: powershell_description,
                    nameKey: powershell_available.then(|| {
                        match &self.powershell {
                            PowerShellSource::OnDemand(_) => "terminalBuiltinPowerShellDownload",
                            PowerShellSource::Bundled => "terminalBuiltinPowerShell",
                        }
                        .into()
                    }),
                },
                TerminalTypeInfo {
                    terminal: TERMINAL.to_string(),
                    terminalType: "bash".to_string(),
                    available: bash.is_ok(),
                    nameKey: bash.is_ok().then(|| "terminalBuiltinWindowsBash".into()),
                    description: match bash {
                        Ok(_) => "Bundled Windows Bash terminal".to_string(),
                        Err(error) => format!(
                            "Bundled Windows Bash unavailable: {}. Reinstall the complete Operit Windows package.",
                            error.message,
                        ),
                    },
                },
            ],
        })
    }

    /// Opens the requested interpreter for a manual terminal session.
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
        let (normalizedTerminalType, kind) = normalizeTerminalType(terminalType)?;
        let workDir = nonBlank(workingDir, "working_directory")?;
        let session = createPtySession(
            &self.powershell,
            self.profile.as_ref(),
            normalizedSessionName,
            normalizedTerminalType,
            kind,
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
        let mut state = self.lockState()?;
        let session = state
            .ptySessions
            .get_mut(sessionId)
            .ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?;
        session.process.resize(rows, cols)
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
        match session.process.try_wait()? {
            Some(code) => {
                session.exitCode = Some(code);
                session.commandRunning = false;
                session.scriptFile.take();
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
                if let Some(code) = session.process.try_wait()? {
                    session.exitCode = Some(code);
                }
            }
            if session.exitCode.is_some() { session.commandRunning = false; session.scriptFile.take(); }
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
        let (normalizedTerminalType, kind) = normalizeTerminalType(terminalType)?;
        let key = sessionKey(&normalizedTerminalType, &normalizedSessionName);
        {
            let mut state = self.lockState()?;
            if let Some(sessionId) = state.sessionNameToId.get(&key).cloned() {
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
                state.sessionNameToId.remove(&key);
            }
        }

        let workingDir = std::env::current_dir()?.display().to_string();
        let session = createPtySession(
            &self.powershell,
            self.profile.as_ref(),
            normalizedSessionName.clone(),
            normalizedTerminalType.clone(),
            kind,
            workingDir,
            24,
            80,
        )?;
        let sessionId = nextSessionId();
        let mut state = self.lockState()?;
        state.sessionNameToId.insert(key, sessionId.clone());
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
        let kind = TerminalKind::PowerShell;
        let normalizedExecutorKey = match executorKey.trim() {
            "" => "default".to_string(),
            value => value.to_string(),
        };
        let key = sessionKey(&normalizedTerminalType, &normalizedExecutorKey);
        let sessionId = hiddenPtySessionId(
            self,
            &key,
            &normalizedExecutorKey,
            &normalizedTerminalType,
            kind,
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
            session.exitCode = session.process.try_wait()?;
            if session.exitCode.is_some() { session.commandRunning = false; session.scriptFile.take(); }
        }
        let terminalType = session.terminalType.clone();
        let commandRunning = session.commandRunning;
        let screenOutput = session.screenOutput.clone();
        let (screenRows, screenCols) = (session.process.rows, session.process.cols);
        drop(state);
        let content = ptyScreenContent(&screenOutput, screenRows, screenCols)?;
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

impl WindowsTerminalHost {
    #[allow(non_snake_case)]
    fn lockState(&self) -> HostResult<std::sync::MutexGuard<'_, TerminalState>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("terminal state mutex poisoned"))
    }
}

#[allow(non_snake_case)]
fn hiddenPtySessionId(
    host: &WindowsTerminalHost,
    executorKey: &str,
    executorLabel: &str,
    terminalType: &str,
    kind: TerminalKind,
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
        &host.powershell,
        host.profile.as_ref(),
        format!("hidden:{executorLabel}"),
        terminalType.to_string(),
        kind,
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
    host: WindowsTerminalHost,
    sessionId: String,
    command: String,
    terminalType: String,
    commandEcho: String,
    commandOutput: PtyCommandOutput,
    timeoutMs: u64,
    collected: Vec<u8>,
    workingDir: Option<String>,
    kind: TerminalKind,
    renderer: TerminalTextRenderer,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl WindowsTerminalHost {
    /// Submits one command and reserves its session before asynchronous observation begins.
    fn beginCommand(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<CooperativeCommand> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let normalizedCommand = nonBlank(command, "command")?;
        let (terminalType, kind, commandOutput, renderer, commandEcho, writer) = {
            let mut state = self.lockState()?;
            let session = state
                .ptySessions
                .get_mut(&normalizedSessionId)
                .ok_or_else(|| {
                    HostError::new(format!("Terminal session does not exist: {sessionId}"))
                })?;
            let renderer = ptyScreenRenderer(session)?;
            if session.commandRunning {
                return Err(HostError::new(
                    "Terminal session already has a command in progress",
                ));
            }
            clearPtyCommandOutput(&session.commandOutput)?;
            let commandEcho = ptyCommandLine(&normalizedCommand, session.kind);
            let commandInput = format!("{commandEcho}\r");
            let mut writer = session
                .writer
                .lock()
                .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
            writer.write_all(commandInput.as_bytes())?;
            writer.flush()?;
            session.commandRunning = true;
            (
                session.terminalType.clone(),
                session.kind,
                session.commandOutput.clone(),
                renderer,
                commandEcho,
                session.writer.clone(),
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
            kind,
            renderer,
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
                if let Some((marker, output)) = completedPtyPrompt(
                    &self.collected, self.kind, self.renderer.clone(), Some(&self.commandEcho),
                )? {
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
                        dropCommandEcho(renderPtyOutput(&self.collected, self.renderer.clone()), &self.commandEcho),
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
            if let Some((marker, _)) = completedPtyPrompt(
                &self.collected, self.kind, TerminalTextRenderer::new(self.renderer.rows, self.renderer.cols), None,
            )? {
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
/// Prepares the selected interpreter before allocating its PTY.
fn createPtySession(
    powershell: &PowerShellSource,
    profile: Option<&operit_host_api::TerminalProfile>,
    sessionName: String,
    terminalType: String,
    kind: TerminalKind,
    workingDir: String,
    rows: u16,
    cols: u16,
) -> HostResult<PtySession> {
    createPtyProcess(powershell, profile, sessionName, terminalType, kind, workingDir, rows, cols, None, true)
}

/// Reuses ConPTY readers and input while letting the child process own completion.
fn createPtyProcess(powershell: &PowerShellSource, profile: Option<&operit_host_api::TerminalProfile>,
    sessionName: String, terminalType: String, kind: TerminalKind, workingDir: String,
    rows: u16, cols: u16, script: Option<&str>, managedPrompt: bool) -> HostResult<PtySession> {
    let scriptFile = script.map(|script| -> HostResult<tempfile::TempPath> {
        let extension = match kind { TerminalKind::Bash => ".sh", TerminalKind::PowerShell => ".ps1" };
        let mut file = tempfile::Builder::new().prefix("operit-command-").suffix(extension).tempfile()?;
        file.write_all(script.as_bytes())?;
        file.flush()?;
        Ok(file.into_temp_path())
    }).transpose()?;
    let command = if managedPrompt {
        match profile {
            Some(profile) => configuredPtyCommand(profile, kind, &workingDir)?,
            None => ptyCommand(powershell, kind, &workingDir)?,
        }
    } else { processCommand(powershell, profile, kind, &workingDir, scriptFile.as_deref())? };
    let (process, writer, reader) = conpty::Process::spawn(command, rows, cols)?;
    let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(Box::new(writer)));
    let output = Arc::new(Mutex::new(VecDeque::new()));
    let commandOutput: PtyCommandOutput = Arc::new(operit_host_api::TerminalCommand::TerminalCommandBuffer::new(PTY_OUTPUT_LIMIT));
    let screenOutput = Arc::new(Mutex::new(VecDeque::new()));
    let cursorState: PtyCursorState = Arc::new(Mutex::new(PtyCursor { row: 1, col: 1 }));
    let outputForThread = output.clone();
    let commandOutputForThread = commandOutput.clone();
    let screenOutputForThread = screenOutput.clone();
    let writerForThread = writer.clone();
    let cursorStateForThread = cursorState.clone();
    operit_host_native_common::NativeHostRuntimeTaskSchedulerHost::new()
        .scheduleHostRuntimeCooperativeAsyncTask("operit-pty-reader", Box::new(move || Box::pin(async move {
            use std::os::windows::io::IntoRawHandle;
            use tokio::io::AsyncReadExt;
            let result: HostResult<()> = async {
                let mut reader = unsafe { tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle(reader.into_raw_handle()) }?;
                reader.connect().await?;
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    match reader.read(&mut buffer).await? {
                    0 => return Err(HostError::new("PTY reader reached EOF")),
                    count => {
                        let chunk = &buffer[..count];
                        processPtyTerminalQueries(&writerForThread, &cursorStateForThread, chunk)?;
                        let commandChunk = stripPtyDeviceStatusReports(chunk);
                        let visibleChunk = stripPtyPromptMarkers(&commandChunk);
                        appendPtyOutput(&outputForThread, &visibleChunk, PTY_OUTPUT_LIMIT);
                        appendPtyOutput(&screenOutputForThread, &visibleChunk, PTY_OUTPUT_LIMIT);
                        if managedPrompt { appendPtyCommandOutput(&commandOutputForThread, &commandChunk); }
                    }
                    }
                }
            }.await;
            commandOutputForThread.fail(result.unwrap_err());
        })))?;
    if managedPrompt {
    operit_host_native_common::waitForTerminalPrompt(
        &commandOutput,
        10000,
        |bytes| completedPtyPrompt(bytes, kind, TerminalTextRenderer::new(usize::from(rows), usize::from(cols)), None)
            .map(|prompt| prompt.is_some()),
        |_| HostError::new("Timed out waiting for terminal prompt"),
    )?;
    clearPtyCommandOutput(&commandOutput)?;
    }
    Ok(PtySession {
        sessionName,
        terminalType,
        kind,
        workingDir,
        process,
        writer,
        output,
        commandOutput,
        screenOutput,
        commandRunning: !managedPrompt,
        processOwned: !managedPrompt,
        exitCode: None,
        scriptFile,
    })
}

/// Builds raw interpreter arguments while preserving the selected runtime and launcher environment.
fn processCommand(powershell: &PowerShellSource, profile: Option<&operit_host_api::TerminalProfile>,
    kind: TerminalKind, directory: &str, script: Option<&Path>) -> HostResult<CommandBuilder> {
    if let Some(profile) = profile {
        if let Some(distribution) = profile.options.get("wslDistribution") {
            let mut command = CommandBuilder::new(configuredExecutable(profile)?);
            command.args(operit_host_api::TerminalProfiles::profileArguments(profile)?);
            command.args(["--distribution", distribution, "--cd", directory, "--exec", "/usr/bin/env",
                "TERM=xterm-256color", "COLORTERM=truecolor", "LANG=C.UTF-8", "/bin/bash", "--noprofile", "--norc"]);
            match script { Some(script) => { command.arg(wslDirectory(profile, &script.to_string_lossy())?); }, None => { command.arg("-i"); } }
            return Ok(command);
        }
    }
    let original = match profile {
        Some(profile) => configuredPtyCommand(profile, kind, directory)?,
        None => ptyCommand(powershell, kind, directory)?,
    };
    let mut command = CommandBuilder::new(original.get_program());
    // Only replace the interpreter suffix we own; profile arguments are literal launcher prefixes.
    let args = original.get_args().collect::<Vec<_>>();
    let suffix = match kind { TerminalKind::Bash => 1, TerminalKind::PowerShell => 3 };
    command.args(&args[..args.len() - suffix]);
    for (key, value) in original.get_envs() {
        match value { Some(value) => { command.env(key, value); }, None => { command.env_remove(key); } }
    }
    command.env_remove("PS1");
    command.env_remove("PROMPT_COMMAND");
    command.current_dir(directory);
    match (kind, script) {
        (TerminalKind::Bash, Some(script)) => { command.arg(script.to_string_lossy().replace('\\', "/")); },
        (TerminalKind::Bash, None) => { command.arg("-i"); },
        (TerminalKind::PowerShell, Some(script)) => { command.arg("-File").arg(script); },
        (TerminalKind::PowerShell, None) => {},
    }
    Ok(command)
}

/// Encodes scripts that interactive line editors cannot accept losslessly as one UTF-8 input line.
#[allow(non_snake_case)]
fn ptyCommandLine(command: &str, kind: TerminalKind) -> String {
    match kind {
        TerminalKind::Bash => terminal_command::posixCommandLine(command),
        TerminalKind::PowerShell => {
            if command.is_ascii() && !command.chars().any(char::is_control) {
                return command.to_string();
            }
            let encoded = BASE64_STANDARD.encode(command.as_bytes());
            format!("Invoke-Expression ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{encoded}')))")
        }
    }
}

#[allow(non_snake_case)]
/// Builds the PTY command for the selected Windows terminal type.
fn ptyCommand(powershell: &PowerShellSource, kind: TerminalKind, workingDir: &str) -> HostResult<CommandBuilder> {
    match kind {
        TerminalKind::PowerShell => windowsPtyCommand(powershell, workingDir),
        TerminalKind::Bash => {
            let executable = bundledBashExecutable()?;
            let root = executable
                .parent()
                .and_then(|path| path.parent())
                .and_then(|path| path.parent())
                .ok_or_else(|| HostError::new("Bundled Bash executable has no runtime root"))?;
            let mut paths = vec![root.join("bin"), root.join("usr/bin")];
            if let Some(inherited) = std::env::var_os("PATH") {
                paths.extend(std::env::split_paths(&inherited));
            }
            let mut command = CommandBuilder::new(&executable);
            command.env("PATH", std::env::join_paths(paths).map_err(toHostError)?);
            command.arg("--noprofile");
            command.arg("--norc");
            command.arg("-i");
            command.current_dir(workingDir);
            command.env("TERM", "xterm-256color");
            command.env("COLORTERM", "truecolor");
            command.env("LANG", "C.UTF-8");
            command.env("PS1", "$PWD $ ");
            command.env(
                "PROMPT_COMMAND",
                r#"__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$__operit_status" "$PWD""#,
            );
            Ok(command)
        }
    }
}

/// Applies managed prompt instrumentation to the exact user-selected interpreter.
fn configuredPtyCommand(
    profile: &operit_host_api::TerminalProfile,
    kind: TerminalKind,
    workingDir: &str,
) -> HostResult<CommandBuilder> {
    if let Some(distribution) = profile.options.get("wslDistribution") {
        return wslPtyCommand(profile, distribution, workingDir);
    }
    let mut command = CommandBuilder::new(configuredExecutable(profile)?);
    for argument in operit_host_api::TerminalProfiles::profileArguments(profile)? {
        command.arg(argument);
    }
    match kind {
        TerminalKind::PowerShell => {
            for argument in [
                "-NoLogo",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-NoExit",
                "-Command",
                powershellPtyInitScript(),
            ] {
                command.arg(argument);
            }
        }
        TerminalKind::Bash => {
            for argument in ["--noprofile", "--norc", "-i"] {
                command.arg(argument);
            }
            command.env("PS1", "$PWD $ ");
            command.env("PROMPT_COMMAND", r#"__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$__operit_status" "$PWD""#);
        }
    }
    command.current_dir(workingDir);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("LANG", "C.UTF-8");
    Ok(command)
}

/// Resolves the explicit custom launcher or the system launcher for a WSL profile.
fn configuredExecutable(profile: &operit_host_api::TerminalProfile) -> HostResult<PathBuf> {
    match (profile.options.get("executable"), profile.options.get("wslDistribution")) {
        (Some(executable), _) => Ok(PathBuf::from(executable)),
        (None, Some(_)) => wsl::system_executable(),
        (None, None) => Err(HostError::new("Custom terminal requires executable")),
    }
}

/// Converts the host-resolved workspace path using the exact selected WSL distribution.
fn wslDirectory(profile: &operit_host_api::TerminalProfile, native: &str) -> HostResult<String> {
    let distribution = profile
        .options
        .get("wslDistribution")
        .ok_or_else(|| HostError::new("WSL distribution is required"))?;
    let output = crate::process::background_command(configuredExecutable(profile)?)
        .args(operit_host_api::TerminalProfiles::profileArguments(
            profile,
        )?)
        .args([
            "--distribution",
            distribution,
            "--exec",
            "wslpath",
            "-a",
            "-u",
            native,
        ])
        .output()?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "WSL workspace mapping failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let path = String::from_utf8(output.stdout)
        .map_err(toHostError)?
        .trim()
        .to_string();
    if !path.starts_with('/') || path.contains(['\0', '\r', '\n']) {
        return Err(HostError::new("WSL returned an invalid guest directory"));
    }
    Ok(path)
}

/// Launches WSL Bash with explicit guest environment and managed prompt instrumentation.
fn wslPtyCommand(
    profile: &operit_host_api::TerminalProfile,
    distribution: &str,
    workingDir: &str,
) -> HostResult<CommandBuilder> {
    let directory = if workingDir.starts_with('/') {
        workingDir.to_string()
    } else {
        wslDirectory(profile, workingDir)?
    };
    let mut command = CommandBuilder::new(configuredExecutable(profile)?);
    command.args(operit_host_api::TerminalProfiles::profileArguments(
        profile,
    )?);
    command.args(["--distribution", distribution, "--cd", &directory, "--exec", "/usr/bin/env",
        "TERM=xterm-256color", "COLORTERM=truecolor", "LANG=C.UTF-8", "PS1=$PWD $ ",
        r#"PROMPT_COMMAND=__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$__operit_status" "$PWD""#,
        "/bin/bash", "--noprofile", "--norc", "-i"]);
    Ok(command)
}

#[allow(non_snake_case)]
/// Resolves only the Bash runtime shipped beside the running application.
fn bundledBashExecutable() -> HostResult<PathBuf> {
    let executable = std::env::current_exe()?;
    let root = executable
        .parent()
        .ok_or_else(|| HostError::new("Operit executable has no parent directory"))?;
    // Invoke the packaged interpreter directly, without the compatibility launcher.
    let bashExecutable = root.join("bash/usr/bin/bash.exe");
    if !bashExecutable.is_file() {
        return Err(HostError::new(format!(
            "Bundled Bash executable does not exist: {}",
            bashExecutable.display()
        )));
    }
    Ok(bashExecutable)
}

#[allow(non_snake_case)]
/// Resolves the PowerShell 7 runtime shipped beside the running application.
fn bundledPowerShellExecutable() -> HostResult<PathBuf> {
    let executable = std::env::current_exe()?;
    let root = executable
        .parent()
        .ok_or_else(|| HostError::new("Operit executable has no parent directory"))?;
    let powershellExecutable = root.join("powershell/pwsh.exe");
    if !powershellExecutable.is_file() {
        return Err(HostError::new(format!(
            "Bundled PowerShell 7 executable does not exist: {}",
            powershellExecutable.display()
        )));
    }
    Ok(powershellExecutable)
}

#[allow(non_snake_case)]
/// Builds the PowerShell 7 PTY command using the package's selected runtime source.
fn windowsPtyCommand(powershell: &PowerShellSource, workingDir: &str) -> HostResult<CommandBuilder> {
    let executable = match powershell {
        PowerShellSource::Bundled => bundledPowerShellExecutable()?,
        PowerShellSource::OnDemand(cache) => powershell_runtime::executable(cache)?,
    };
    let mut paths = vec![executable.parent().unwrap().to_path_buf()];
    if let Some(inherited) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&inherited));
    }
    let mut command = CommandBuilder::new(&executable);
    command.env("PATH", std::env::join_paths(paths).map_err(toHostError)?);
    command.arg("-NoLogo");
    command.arg("-NoProfile");
    command.arg("-ExecutionPolicy");
    command.arg("Bypass");
    command.arg("-NoExit");
    command.arg("-Command");
    command.arg(powershellPtyInitScript());
    command.current_dir(workingDir);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("LANG", "C.UTF-8");
    Ok(command)
}

#[allow(non_snake_case)]
/// Initializes UTF-8 input/output and the session completion prompt.
fn powershellPtyInitScript() -> &'static str {
    "$global:__operit_error_count = $global:error.Count; [Console]::InputEncoding = [Text.UTF8Encoding]::new($false); [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); $OutputEncoding = [Text.UTF8Encoding]::new($false); function global:prompt { $__operit_success = $?; $__operit_error_changed = $global:error.Count -gt $global:__operit_error_count; $global:__operit_error_count = $global:error.Count; $__operit_location = (Get-Location).Path; $__operit_exit_code = if ($__operit_success -and -not $__operit_error_changed) { 0 } elseif ($__operit_error_changed) { 1 } elseif ($LASTEXITCODE -is [int] -and $LASTEXITCODE -ne 0) { [int]$LASTEXITCODE } else { 1 }; ([char]27) + ']133;OperitPrompt=' + $__operit_exit_code + ':' + $__operit_location + ([char]7) + 'PS ' + $__operit_location + '> ' }"
}

#[allow(non_snake_case)]
/// Uses the session-owned event buffer for command output.
fn appendPtyCommandOutput(output: &PtyCommandOutput, data: &[u8]) { output.append(data); }

#[allow(non_snake_case)]
/// Keeps finite cursor replies off the cooperative output reader's IOCP executor.
fn processPtyTerminalQueries(
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    cursorState: &PtyCursorState,
    data: &[u8],
) -> HostResult<()> {
    scanPtyOutput(data, || {
        let cursor = cursorState.lock().map_err(|_| HostError::new("PTY cursor state is poisoned"))?;
        let response = format!("\x1b[{};{}R", cursor.row, cursor.col);
        let writer = writer.clone();
        operit_host_native_common::scheduleNativeBlockingTask("terminal-cursor-response", move || {
            let mut writer = writer.lock().map_err(|_| HostError::new("PTY writer is poisoned"))?;
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
    end: usize,
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
    // Windows directory names exclude BEL; putting the status first preserves drive colons.
    let (exitCodeText, workingDir) = payload
        .split_once(':')
        .ok_or_else(|| HostError::new(format!("Invalid PTY prompt marker '{payload}'")))?;
    let workingDir = workingDir.to_string();
    let exitCode = parseExitCode(exitCodeText)?;
    Ok(Some(PtyPromptMarker {
        end: payloadEnd + 1,
        workingDir,
        exitCode,
    }))
}

#[allow(non_snake_case)]
fn findBytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[allow(non_snake_case)]
/// Renders bytes from the captured viewport without assuming ConPTY has flushed its final frame.
fn renderPtyOutput(raw: &[u8], mut renderer: TerminalTextRenderer) -> String {
    let startRow = renderer.logicalLineStart();
    renderer.write(&String::from_utf8_lossy(raw));
    renderer.textFrom(startRow)
}

#[allow(non_snake_case)]
/// Waits for both OSC metadata and its rendered prompt because ConPTY forwards OSC before flushing text.
fn completedPtyPrompt(raw: &[u8], kind: TerminalKind, renderer: TerminalTextRenderer, command: Option<&str>)
    -> HostResult<Option<(PtyPromptMarker, String)>> {
    let Some(marker) = findLastPtyPromptMarker(raw)? else {
        return Ok(None);
    };
    let prompt = match kind {
        TerminalKind::PowerShell => format!("PS {}>", marker.workingDir),
        TerminalKind::Bash => format!("{} $", marker.workingDir),
    };
    let flushed = renderPtyOutput(&raw[marker.end..], TerminalTextRenderer::new(renderer.rows, renderer.cols));
    if dropTrailingPtyPrompt(&flushed, &prompt).is_none() { return Ok(None); }
    let mut rendered = renderPtyOutput(raw, renderer);
    if let Some(command) = command {
        rendered = dropCommandEcho(rendered, command);
    }
    Ok(dropTrailingPtyPrompt(&rendered, &prompt).map(|output| (marker, output)))
}

#[allow(non_snake_case)]
/// Reconstructs the terminal viewport and scrollback using the session's actual dimensions.
fn ptyScreenRenderer(session: &PtySession) -> HostResult<TerminalTextRenderer> {
    renderScreenBuffer(&session.screenOutput, session.process.rows, session.process.cols)
}

/// Copies terminal bytes before rendering, without retaining the session or output registry lock.
fn renderScreenBuffer(output: &Arc<Mutex<VecDeque<u8>>>, rows: u16, cols: u16) -> HostResult<TerminalTextRenderer> {
    let bytes = output.lock().map_err(|_| HostError::new("PTY screen output mutex poisoned"))?
        .iter().copied().collect::<Vec<_>>();
    let mut renderer = TerminalTextRenderer::new(usize::from(rows), usize::from(cols));
    renderer.write(&String::from_utf8_lossy(&bytes));
    Ok(renderer)
}

/// Renders a copied screen independently of the terminal registry used by input and close.
fn ptyScreenContent(output: &Arc<Mutex<VecDeque<u8>>>, rows: u16, cols: u16) -> HostResult<String> {
    Ok(renderScreenBuffer(output, rows, cols)?.textFrom(0))
}

#[derive(Clone)]
struct TerminalTextRenderer {
    lines: Vec<Vec<char>>,
    wrappedRows: BTreeSet<usize>,
    viewportStart: usize,
    rows: usize,
    cols: usize,
    row: usize,
    col: usize,
    savedRow: usize,
    savedCol: usize,
}

impl TerminalTextRenderer {
    /// Starts a viewport whose absolute cursor coordinates remain separate from scrollback.
    fn new(rows: usize, cols: usize) -> Self {
        Self {
            lines: vec![Vec::new()],
            wrappedRows: BTreeSet::new(),
            viewportStart: 0,
            rows,
            cols,
            row: 0,
            col: 0,
            savedRow: 0,
            savedCol: 0,
        }
    }

    /// Applies terminal output while retaining logical lines that have scrolled out of view.
    fn write(&mut self, value: &str) {
        let chars = value.chars().collect::<Vec<_>>();
        let mut index = 0usize;
        while index < chars.len() {
            let ch = chars[index];
            if ch == '\x1b' {
                index = self.consumeEscape(&chars, index);
                continue;
            }
            self.writeChar(ch);
            index += 1;
        }
    }

    /// Locates the logical start of the current line across automatic terminal wrapping.
    fn logicalLineStart(&self) -> usize {
        let mut row = self.row;
        while row > 0 && self.wrappedRows.contains(&(row - 1)) {
            row -= 1;
        }
        row
    }

    /// Joins soft-wrapped rows and retains explicit newlines from the requested transcript row.
    fn textFrom(&self, startRow: usize) -> String {
        let mut text = String::new();
        for (row, line) in self.lines.iter().enumerate().skip(startRow) {
            let content = line.iter().collect::<String>();
            if self.wrappedRows.contains(&row) || row + 1 == self.lines.len() {
                text.push_str(&content);
            } else {
                text.push_str(content.trim_end());
                if row + 1 < self.lines.len() {
                    text.push('\n');
                }
            }
        }
        text
    }

    fn consumeEscape(&mut self, chars: &[char], index: usize) -> usize {
        if index + 1 >= chars.len() {
            return index + 1;
        }
        match chars[index + 1] {
            '[' => self.consumeCsi(chars, index + 2),
            ']' => self.consumeOsc(chars, index + 2),
            '7' => {
                self.savedRow = self.row;
                self.savedCol = self.col;
                index + 2
            }
            '8' => {
                self.row = self.savedRow;
                self.col = self.savedCol;
                self.ensureCursorLine();
                index + 2
            }
            _ => index + 2,
        }
    }

    fn consumeOsc(&mut self, chars: &[char], mut index: usize) -> usize {
        while index < chars.len() {
            let ch = chars[index];
            index += 1;
            if ch == '\x07' {
                break;
            }
            if ch == '\x1b' && index < chars.len() && chars[index] == '\\' {
                index += 1;
                break;
            }
        }
        index
    }

    fn consumeCsi(&mut self, chars: &[char], mut index: usize) -> usize {
        let start = index;
        while index < chars.len() {
            let ch = chars[index];
            index += 1;
            if ch >= '@' && ch <= '~' {
                let params = chars[start..index - 1].iter().collect::<String>();
                self.applyCsi(&params, ch);
                break;
            }
        }
        index
    }

    /// Resolves absolute cursor addresses against the visible viewport rather than scrollback.
    fn applyCsi(&mut self, params: &str, command: char) {
        let values = csiParams(params);
        match command {
            'A' => self.moveRowUp(csiValue(&values, 0, 1)),
            'B' => self.moveRowDown(csiValue(&values, 0, 1)),
            'C' => self.col = (self.col + csiValue(&values, 0, 1)).min(self.cols - 1),
            'D' => self.col = self.col.saturating_sub(csiValue(&values, 0, 1)),
            'G' => self.col = csiValue(&values, 0, 1).saturating_sub(1).min(self.cols - 1),
            'H' | 'f' => {
                self.row = self.viewportStart
                    + csiValue(&values, 0, 1).saturating_sub(1).min(self.rows - 1);
                self.col = csiValue(&values, 1, 1).saturating_sub(1).min(self.cols - 1);
                self.ensureCursorLine();
            }
            'J' => self.eraseDisplay(csiValue(&values, 0, 0)),
            'K' => self.eraseLine(csiValue(&values, 0, 0)),
            'X' => self.eraseChars(csiValue(&values, 0, 1)),
            's' => {
                self.savedRow = self.row;
                self.savedCol = self.col;
            }
            'u' => {
                self.row = self.savedRow;
                self.col = self.savedCol;
                self.ensureCursorLine();
            }
            _ => {}
        }
    }

    /// Distinguishes explicit line feeds from terminal column wrapping.
    fn writeChar(&mut self, ch: char) {
        match ch {
            '\r' => self.col = 0,
            '\n' => {
                self.wrappedRows.remove(&self.row);
                self.advanceLine();
                self.col = 0;
            }
            '\x08' => {
                self.col = self.col.saturating_sub(1);
            }
            '\t' => {
                let spaces = 8 - self.col % 8;
                for _ in 0..spaces {
                    self.putChar(' ');
                }
            }
            _ if ch.is_control() => {}
            _ => self.putChar(ch),
        }
    }

    /// Wraps at the right margin and records continuation rows before writing a cell.
    fn putChar(&mut self, ch: char) {
        if self.col >= self.cols {
            self.wrappedRows.insert(self.row);
            self.advanceLine();
            self.col = 0;
        }
        self.ensureCursorLine();
        let line = &mut self.lines[self.row];
        while line.len() < self.col {
            line.push(' ');
        }
        if line.len() == self.col {
            line.push(ch);
        } else {
            line[self.col] = ch;
        }
        self.col += 1;
    }

    /// Moves upward within the visible viewport without changing scrollback.
    fn moveRowUp(&mut self, count: usize) {
        self.row = self.row.saturating_sub(count).max(self.viewportStart);
        self.ensureCursorLine();
    }

    /// Moves downward within the visible viewport without scrolling.
    fn moveRowDown(&mut self, count: usize) {
        self.row = (self.row + count).min(self.viewportStart + self.rows - 1);
        self.ensureCursorLine();
    }

    /// Advances output and moves the viewport when a line passes its bottom margin.
    fn advanceLine(&mut self) {
        self.row += 1;
        if self.row >= self.viewportStart + self.rows {
            self.viewportStart = self.row + 1 - self.rows;
        }
        self.ensureCursorLine();
    }

    /// Erases the requested screen region while preserving its cursor and scrollback coordinates.
    fn eraseDisplay(&mut self, mode: usize) {
        match mode {
            0 => {
                self.eraseLine(0);
                self.lines.truncate(self.row + 1);
                self.wrappedRows.retain(|row| *row < self.row);
            }
            1 => {
                for row in self.viewportStart..self.row {
                    self.lines[row].clear();
                }
                self.eraseLine(1);
            }
            2 | 3 => {
                let start = if mode == 3 { 0 } else { self.viewportStart };
                for row in start..self.lines.len() {
                    self.lines[row].clear();
                    self.wrappedRows.remove(&row);
                }
            }
            _ => {}
        }
    }

    fn eraseLine(&mut self, mode: usize) {
        self.ensureCursorLine();
        let line = &mut self.lines[self.row];
        match mode {
            0 => {
                if self.col < line.len() {
                    line.truncate(self.col);
                }
            }
            1 => {
                let end = self.col.min(line.len());
                for cell in &mut line[..end] {
                    *cell = ' ';
                }
            }
            2 => line.clear(),
            _ => {}
        }
    }

    fn eraseChars(&mut self, count: usize) {
        self.ensureCursorLine();
        let line = &mut self.lines[self.row];
        let start = self.col.min(line.len());
        let end = start.saturating_add(count).min(line.len());
        for cell in &mut line[start..end] {
            *cell = ' ';
        }
    }

    fn ensureCursorLine(&mut self) {
        while self.lines.len() <= self.row {
            self.lines.push(Vec::new());
        }
    }
}

/// Removes the submitted command echo across physical terminal line wraps.
#[allow(non_snake_case)]
fn dropCommandEcho(value: String, command: &str) -> String {
    let mut lines = value.lines().map(str::to_string).collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    let command = command.trim();
    let flattened = lines.join("");
    if let Some(start) = flattened.find(command) {
        if lines.first().is_some_and(|line| start < line.len()) {
            let commandEnd = start + command.len();
            let mut offset = 0;
            let echoEnd = lines.iter().position(|line| {
                offset += line.len();
                offset == commandEnd
            });
            if let Some(index) = echoEnd {
                lines.drain(..=index);
            }
        }
    }
    lines.join("\n").trim().to_string()
}

/// Recognizes the complete rendered prompt across physical ConPTY rows without altering output newlines.
#[allow(non_snake_case)]
fn dropTrailingPtyPrompt(value: &str, prompt: &str) -> Option<String> {
    let value = value.trim_end();
    let mut cells = value
        .char_indices()
        .rev()
        .filter(|(_, ch)| *ch != '\r' && *ch != '\n');
    let mut start = value.len();
    for expected in prompt.chars().rev() {
        match cells.next() {
            Some((index, actual)) if actual == expected => start = index,
            _ => return None,
        }
    }
    Some(value[..start].trim().to_string())
}

#[allow(non_snake_case)]
fn toHostError(error: impl std::fmt::Display) -> HostError {
    HostError::new(error.to_string())
}



#[allow(non_snake_case)]
fn normalizeTerminalType(terminalType: &str) -> HostResult<(String, TerminalKind)> {
    match terminalType.trim() {
        "powershell" => Ok(("powershell".to_string(), TerminalKind::PowerShell)),
        "bash" => Ok(("bash".to_string(), TerminalKind::Bash)),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for windows host: {value}"
        ))),
    }
}

/// Validates the terminal implementation owned by the Windows PTY host.
fn requireNativeTerminal(terminal: &str) -> HostResult<()> {
    match terminal.trim() {
        TERMINAL => Ok(()),
        value => Err(HostError::new(format!(
            "Unsupported terminal implementation for Windows PTY host: {value}"
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

    /// Keeps more silent ConPTY sessions than service workers live, including Unicode output and teardown.
    #[test]
    fn iocp_terminal_sessions_do_not_consume_service_capacity() {
        let host = WindowsTerminalHost::new();
        let powershell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let host = host.configuredTerminal(&operit_host_api::TerminalProfile {
            id: "iocp".into(), name: "IOCP".into(), nameKey: None,
            terminal: TERMINAL.into(), terminalType: "powershell".into(),
            options: BTreeMap::from([("executable".into(), powershell.display().to_string())]),
        }).unwrap();
        let mut sessions = Vec::new();
        for index in 0..32 {
            sessions.push(host.createOrGetSession(&format!("iocp-{index}")).unwrap().sessionId);
        }
        assert_eq!(host.listSessions().unwrap().len(), sessions.len());
        host.resizePtySession(&sessions[0], 30, 100).unwrap();
        let result = host.executeInSession(&sessions[0], "Write-Output '雪🌍'; Write-Output 'ready'", 5000).unwrap();
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "雪🌍\nready");

        for (command, expected, code) in [
            ("$null", String::new(), 0),
            ("cmd /c exit 7", String::new(), 7),
            ("1..80 | ForEach-Object { 'line-' + $_ }", (1..=80).map(|index| format!("line-{index}")).collect::<Vec<_>>().join("\n"), 0),
            ("Write-Output ready", "ready".into(), 0),
        ] {
            let result = host.executeInSession(&sessions[0], command, 5000).unwrap();
            assert!(!result.timedOut, "{command}");
            assert_eq!(result.exitCode, code, "{command}");
            assert_eq!(result.output, expected, "{command}");
        }

        let mut streamed = Vec::new();
        let mut beforeLast = false;
        let result = host.executeInSessionStreaming(&sessions[0],
            "Write-Output (('first','chunk') -join '-'); Start-Sleep -Milliseconds 300; Write-Output (('last','chunk') -join '-')",
            5000, &mut |bytes| {
                streamed.extend_from_slice(bytes);
                beforeLast |= findBytes(&streamed, b"first-chunk").is_some()
                    && findBytes(&streamed, b"last-chunk").is_none();
                Ok(())
            }).unwrap();
        assert!(beforeLast, "first output was not streamed before the final output");
        assert_eq!(result.output, "first-chunk\nlast-chunk");

        let timed = host.executeInSession(&sessions[0], "Start-Sleep -Seconds 30", 300).unwrap();
        assert!(timed.timedOut);
        assert_eq!(host.executeInSession(&sessions[0], "Write-Output recovered", 5000).unwrap().output, "recovered");
        assert!(!host.listSessions().unwrap().iter().find(|session| session.sessionId == sessions[0]).unwrap().commandRunning);

        let active = host.clone().beginInSessionStreaming(sessions[1].clone(), "Start-Sleep -Seconds 30".into(), 0).unwrap();
        assert!(host.clone().beginInSessionStreaming(sessions[1].clone(), "Write-Output concurrent".into(), 5000).is_err());
        let (completed, closed) = std::sync::mpsc::channel();
        let waiter = std::thread::spawn(move || {
            completed.send(operit_host_native_common::runTerminalCommandSynchronously(active, &mut |_| Ok(()))).unwrap();
        });
        host.closeSession(&sessions.remove(1)).unwrap();
        assert!(closed.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
        waiter.join().unwrap();
        for session in sessions { host.closeSession(&session).unwrap(); }
        assert!(host.listSessions().unwrap().is_empty());
    }

    /// Retains pending state through fragmented OSC, old prompt frames, and the final ConPTY text flush.
    #[test]
    fn iocp_completion_requires_newly_flushed_prompt() {
        let command = "Write-Output done";
        let marker = "\x1b]133;OperitPrompt=0:C:\\work\x07";
        let mut initial = TerminalTextRenderer::new(3, 80);
        initial.write("PS C:\\work> ");
        let complete = |raw: &str| completedPtyPrompt(raw.as_bytes(), TerminalKind::PowerShell, initial.clone(), Some(command)).unwrap();
        assert!(complete(marker).is_none());
        assert!(complete(&format!("{command}\r\n{}", &marker[..marker.len() - 1])).is_none());
        assert!(complete(&format!("{command}\r\n{marker}")).is_none());
        assert!(complete(&format!("{command}\r\n{marker}done\r\nPS C:\\wo")).is_none());
        assert_eq!(complete(&format!("{command}\r\n{marker}done\r\nPS C:\\work> ")).unwrap().1, "done");
        assert_eq!(completedPtyPrompt(format!("truncated-output\r\n{marker}PS C:\\work> ").as_bytes(),
            TerminalKind::PowerShell, TerminalTextRenderer::new(3, 80), Some(command)).unwrap().unwrap().1, "truncated-output");
    }

    /// Verifies WSL receives explicit guest environment values and never a Windows guest cwd.
    #[test]
    fn configured_profile_wsl_uses_selected_distribution_and_guest_directory() {
        let profile = operit_host_api::TerminalProfile {
            id: "ubuntu_work".into(),
            name: "Ubuntu work".into(),
            nameKey: None,
            terminal: "native".into(),
            terminalType: "bash".into(),
            options: BTreeMap::from([("wslDistribution".into(), "Ubuntu-24.04".into())]),
        };
        let command =
            configuredPtyCommand(&profile, TerminalKind::Bash, "/mnt/c/work space").unwrap();
        assert_eq!(command.get_program(), wsl::system_executable().unwrap().as_os_str());
        let argv: Vec<_> = command
            .get_args()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        assert!(argv.windows(4).any(|args| args
            == [
                "--distribution",
                "Ubuntu-24.04",
                "--cd",
                "/mnt/c/work space"
            ]));
        assert!(argv
            .iter()
            .any(|argument| argument.starts_with("PROMPT_COMMAND=")));
        assert_eq!(
            &argv[argv.len() - 4..],
            ["/bin/bash", "--noprofile", "--norc", "-i"]
        );
        assert!(command.get_current_dir().is_none());
    }

    /// Preserves exact PowerShell 5 or 7 executable selection and argv boundaries.
    #[test]
    fn configured_profile_powershell_preserves_version_and_structured_arguments() {
        for executable in [
            "C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe",
            "C:/Program Files/PowerShell/7/pwsh.exe",
        ] {
            let profile = operit_host_api::TerminalProfile {
                id: "powershell".into(),
                name: "PowerShell".into(),
                nameKey: None,
                terminal: "native".into(),
                terminalType: "powershell".into(),
                options: BTreeMap::from([
                    ("executable".into(), executable.into()),
                    (
                        "arguments".into(),
                        r#"["-SettingsFile","C:/settings file.json"]"#.into(),
                    ),
                ]),
            };
            let command =
                configuredPtyCommand(&profile, TerminalKind::PowerShell, "C:/work").unwrap();
            let argv: Vec<_> = std::iter::once(command.get_program()).chain(command.get_args())
                .map(|value| value.to_string_lossy().into_owned())
                .collect();
            assert_eq!(
                &argv[..3],
                [executable, "-SettingsFile", "C:/settings file.json"]
            );
            assert_eq!(argv.last().unwrap(), powershellPtyInitScript());
        }
    }

    /// Preserves Unicode workspaces and drive colons without spawning an encoder in Bash.
    #[test]
    fn terminal_marker_preserves_status_and_native_directory() {
        for directory in [
            r"E:\中文 工作区\chat_outputs",
            "/e/中文 工作区/chat_outputs",
        ] {
            let raw = format!("\x1b]133;OperitPrompt=7:{directory}\x07");
            let marker = findLastPtyPromptMarker(raw.as_bytes()).unwrap().unwrap();
            assert_eq!(marker.exitCode, 7);
            assert_eq!(marker.workingDir, directory);
        }
    }

    /// Accepts explicit ConPTY row breaks inside the exact prompt while preserving output newlines.
    #[test]
    fn terminal_prompt_matches_physical_rows() {
        let directory = r"E:\operitRuntime\identities\identity-1794309438693-7bda0f99\data\chat_outputs\2026-11-10_20-41-54-933";
        for prompt in [format!("PS {directory}>"), format!("{directory} $")] {
            for cols in [20, 40, 80] {
                let wrapped = prompt
                    .as_bytes()
                    .chunks(cols)
                    .map(|chunk| std::str::from_utf8(chunk).unwrap())
                    .collect::<Vec<_>>()
                    .join("\r\n");
                assert_eq!(
                    dropTrailingPtyPrompt(&format!("first\r\nsecond\r\n{wrapped} "), &prompt).unwrap(),
                    "first\r\nsecond"
                );
            }
        }
        assert!(dropTrailingPtyPrompt("hello\nPS wrong>", "PS expected>")
            .is_none());
    }

    /// Exercises the reported long workspace prompt across narrow ConPTY viewports.
    #[test]
    fn powershell_long_workspace_prompt_preserves_output() {
        let directory = std::env::temp_dir()
            .join(format!("operit-terminal-{}", Uuid::new_v4()))
            .join(
                "identities/identity-1794309438693-7bda0f99/data/chat_outputs/2026-11-10_20-41-54-933",
            );
        std::fs::create_dir_all(&directory).unwrap();
        let host = WindowsTerminalHost::new();
        for cols in [40, 80, 120] {
            let sessionId = host
                .startPtySession(
                    "long-workspace",
                    TERMINAL,
                    "powershell",
                    &directory.display().to_string(),
                    24,
                    cols,
                )
                .unwrap();
            for command in ["Write-Host 'hello'", "echo test123"] {
                let result = host.executeInSession(&sessionId, command, 5000).unwrap();
                assert_eq!(result.exitCode, 0);
                assert!(!result.timedOut);
                assert_eq!(
                    result.output,
                    if command.starts_with("Write-Host") {
                        "hello"
                    } else {
                        "test123"
                    }
                );
            }
            host.closePtySession(&sessionId).unwrap();
        }
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn powershell_command_block_completes() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetSession("terminal_powershell_marker")
            .expect("create terminal session");
        let result = host
            .executeInSession(&session.sessionId, "Write-Output hello", 3000)
            .expect("execute terminal command");

        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "hello");
    }

    /// Rejects malformed syntax on PowerShell 7 without waiting for another input line.
    #[test]
    fn powershell_parse_error_returns_without_timeout() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetSession("terminal_powershell_parse_error")
            .expect("create terminal session");
        let result = host
            .executeInSession(&session.sessionId, "Write-Output hello && && Write-Output world", 3000)
            .expect("execute terminal command");

        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 1);
        assert!(!result.output.trim().is_empty());
        assert!(!result.output.trim_start().starts_with("try {"));
    }

    #[test]
    fn powershell_screen_records_prompt_command_output_prompt() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetSession("terminal_powershell_screen")
            .expect("create terminal session");
        let command = "Write-Output hello";
        let prompt = format!("PS {}> ", std::env::current_dir().unwrap().display());

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
        assert_eq!(result.output, "hello");
        assert_eq!(
            screen.content.split('\n').collect::<Vec<_>>(),
            vec![format!("{prompt}{command}"), "hello".to_string(), prompt]
        );
        assert!(!screen.commandRunning);
    }

    #[test]
    fn windows_default_terminal_is_powershell() {
        let host = WindowsTerminalHost::new();
        let info = host.terminalInfo().expect("terminal info");
        assert_eq!(info.terminalType, "powershell");
        assert_eq!(info.terminal, "native");
    }

    /// Runs PowerShell 7 syntax in the packaged interpreter and preserves Unicode output.
    #[test]
    fn powershell_7_version_and_pipeline_chains() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetSession("terminal_powershell_7")
            .expect("create bundled PowerShell 7 session");
        let result = host
            .executeInSession(
                &session.sessionId,
                "Write-Output $PSVersionTable.PSVersion.Major; Write-Output $PSVersionTable.PSEdition; Write-Output '中文' && Write-Output 'chain-ok'",
                5000,
            )
            .expect("execute PowerShell 7 pipeline chain");
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "7\nCore\n中文\nchain-ok");
        let nativeError = host
            .executeInSession(&session.sessionId, "cmd /c exit 7", 3000)
            .unwrap();
        assert_eq!(nativeError.exitCode, 7);
        let parseError = host
            .executeInSession(&session.sessionId, "Write-Output hello && && Write-Output world", 3000)
            .unwrap();
        assert!(!parseError.timedOut);
        assert_eq!(parseError.exitCode, 1);
        host.closePtySession(&session.sessionId).unwrap();
    }

    /// Verifies that Windows Git Bash sessions execute through the typed host path.
    #[test]
    fn git_bash_command_block_completes() {
        let host = WindowsTerminalHost::new();
        let sessionId = host
            .startPtySession(
                "terminal_git_bash_marker",
                TERMINAL,
                "bash",
                &std::env::current_dir().unwrap().display().to_string(),
                24,
                80,
            )
            .expect("create Git Bash terminal session");
        let result = host
            .executeInSession(&sessionId, "printf git-bash-ok", 3000)
            .expect("execute Git Bash terminal command");

        assert_eq!(result.terminalType, "bash");
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "git-bash-ok");
    }

    /// Keeps viewport cursor corrections from overwriting rows retained in scrollback.
    #[test]
    fn terminal_cursor_addresses_are_relative_to_the_viewport() {
        let mut renderer = TerminalTextRenderer::new(3, 8);
        renderer.write("one\r\ntwo\r\n1234567\r\n\x1b[2;8H89abc\r\nnext");
        assert_eq!(renderer.textFrom(0), "one\ntwo\n123456789abc\nnext");
    }

    /// Expands a tab at a pending wrap without looping across the terminal margin.
    #[test]
    fn terminal_tab_wraps_at_the_right_margin() {
        let mut renderer = TerminalTextRenderer::new(3, 8);
        renderer.write("12345678\tend");
        assert_eq!(renderer.textFrom(0), "12345678        end");
    }

    /// ECH past the end of a short line must not panic, even with an overflowing count.
    #[test]
    fn terminal_erase_chars_tolerates_cursor_past_line_end() {
        let mut renderer = TerminalTextRenderer::new(3, 8);
        renderer.write("ab\x1b[1;8H\x1b[3X");
        assert_eq!(renderer.textFrom(0), "ab");
        let mut renderer = TerminalTextRenderer::new(3, 8);
        renderer.write("ab\x1b[1;8H\x1b[18446744073709551615X");
        assert_eq!(renderer.textFrom(0), "ab");
    }

    /// Preserves PowerShell pipeline output when the completion marker precedes its final display writes.
    #[test]
    fn powershell_scrolled_output_preserves_wrapped_lines() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetSession("terminal_powershell_scrolled")
            .expect("create PowerShell terminal session");
        let warmup = host
            .executeInSession(
                &session.sessionId,
                "1..40 | ForEach-Object { \"warmup-$_\" }",
                5000,
            )
            .expect("scroll the PowerShell viewport");
        assert_eq!(warmup.exitCode, 0);
        let result = host
            .executeInSession(
                &session.sessionId,
                "1..8 | ForEach-Object { ('x' * 110) + $_ }",
                5000,
            )
            .expect("parse wrapped PowerShell pipeline output");
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(
            result.output,
            (1..=8)
                .map(|index| format!("{}{index}", "x".repeat(110)))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// Preserves all wrapped output after scrolling and keeps subsequent commands usable.
    #[test]
    fn git_bash_scrolled_output_preserves_wrapped_lines_and_session() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetTypedSession("terminal_git_bash_scrolled", "bash")
            .expect("create Git Bash terminal session");
        let warmup = host
            .executeInSession(&session.sessionId, "printf 'warmup-%s\\n' {1..40}", 5000)
            .expect("scroll the terminal viewport");
        assert_eq!(warmup.exitCode, 0);
        assert_eq!(
            warmup.output,
            (1..=40)
                .map(|index| format!("warmup-{index}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let result = host
            .executeInSession(&session.sessionId, "printf '%0120d\\n' {1..8}", 5000)
            .expect("parse wrapped output in a scrolled terminal");
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(
            result.output,
            (1..=8)
                .map(|index| format!("{index:0120}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let failure = host
            .executeInSession(&session.sessionId, "printf problem; (exit 7)", 5000)
            .expect("retain command output and its real nonzero status");
        assert_eq!(failure.output, "problem");
        assert_eq!(failure.exitCode, 7);
        let next = host
            .executeInSession(&session.sessionId, "printf ready", 5000)
            .expect("execute a subsequent command in the same session");
        assert_eq!(next.output, "ready");
        assert_eq!(next.exitCode, 0);
        assert!(
            !host
                .getSessionScreen(&session.sessionId)
                .unwrap()
                .commandRunning
        );
    }

    /// Submits a multiline script as one command and waits for its final statement.
    #[test]
    fn git_bash_multiline_command_waits_for_all_statements() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetTypedSession("terminal_multiline", "bash")
            .unwrap();
        let result = host
            .executeInSession(
                &session.sessionId,
                "OPERIT_V=preserved\nprintf 'first\\n'\nsleep 1\nprintf '%s' \"$OPERIT_V\"",
                5000,
            )
            .unwrap();
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "first\npreserved");
        let next = host
            .executeInSession(&session.sessionId, "printf '%s' \"$OPERIT_V\"", 3000)
            .unwrap();
        assert_eq!(next.output, "preserved");
    }

    /// Preserves heredocs, Unicode, quotes, and backslashes in one shell submission.
    #[test]
    fn git_bash_multiline_heredoc_preserves_script_bytes() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetTypedSession("terminal_heredoc", "bash")
            .unwrap();
        let result = host.executeInSession(
            &session.sessionId,
            "OPERIT_V=$(cat <<'EOF'\n中文 'quoted' \\012 $literal\nEOF\n)\nprintf '%s\\n' \"$OPERIT_V\"",
            5000,
        ).unwrap();
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "中文 'quoted' \\012 $literal");
    }

    /// Prevents literal script controls from triggering interactive completion or key bindings.
    #[test]
    fn git_bash_script_control_characters_reach_the_shell_unchanged() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetTypedSession("terminal_control_characters", "bash")
            .unwrap();
        let result = host
            .executeInSession(
                &session.sessionId,
                "OPERIT_V='a\tb\x1b\u{85}'\nprintf '%s' \"$OPERIT_V\" | base64",
                5000,
            )
            .unwrap();
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, BASE64_STANDARD.encode("a\tb\x1b\u{85}"));
    }

    /// Cancels the complete script at its deadline and leaves its session usable.
    #[test]
    fn git_bash_multiline_timeout_cancels_remaining_statements() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetTypedSession("terminal_multiline_timeout", "bash")
            .unwrap();
        let started = Instant::now();
        let result = host
            .executeInSession(
                &session.sessionId,
                "printf 'started\\n'\nsleep 8\nprintf forbidden",
                800,
            )
            .unwrap();
        assert!(result.timedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
        let next = host
            .executeInSession(&session.sessionId, "printf ready", 3000)
            .unwrap();
        assert!(!next.timedOut);
        assert_eq!(next.output, "ready");
    }

    /// Keeps a quiet child process under the command deadline after multiline submission.
    #[test]
    fn git_bash_nested_powershell_timeout_keeps_both_terminal_types_usable() {
        let host = WindowsTerminalHost::new();
        let bash = host
            .createOrGetTypedSession("terminal_nested_powershell", "bash")
            .unwrap();
        host.executeInSession(
            &bash.sessionId,
            "OPERIT_V=$(cat <<'EOF'\nprepared\nEOF\n)\nprintf '%s\\n' \"$OPERIT_V\"",
            5000,
        )
        .unwrap();
        let result = host
            .executeInSession(
                &bash.sessionId,
                "powershell -NoProfile -Command 'Start-Sleep -Seconds 8' > /dev/null 2>&1",
                1800,
            )
            .unwrap();
        assert!(result.timedOut);
        let next = host
            .executeInSession(&bash.sessionId, "printf ready", 3000)
            .unwrap();
        assert_eq!(next.output, "ready");
        let powershell = host
            .createOrGetSession("terminal_after_nested_timeout")
            .unwrap();
        let next = host
            .executeInSession(&powershell.sessionId, "Write-Output ready", 3000)
            .unwrap();
        assert_eq!(next.output, "ready");
    }

    /// Reports the exit status of the last statement in a multiline script.
    #[test]
    fn git_bash_multiline_returns_last_exit_status() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetTypedSession("terminal_multiline_status", "bash")
            .unwrap();
        let result = host
            .executeInSession(&session.sessionId, "printf problem\n(exit 7)", 5000)
            .unwrap();
        assert!(!result.timedOut);
        assert_eq!(result.output, "problem");
        assert_eq!(result.exitCode, 7);
    }

    /// Executes every PowerShell statement and retains variables in the session scope.
    #[test]
    fn powershell_multiline_command_preserves_session_scope() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetSession("terminal_powershell_multiline")
            .unwrap();
        let result = host.executeInSession(
            &session.sessionId,
            "$operit_v = '中文';\nWrite-Output first\nStart-Sleep -Milliseconds 400\nWrite-Output $operit_v",
            5000,
        ).unwrap();
        assert!(!result.timedOut);
        assert_eq!(result.exitCode, 0);
        assert_eq!(result.output, "first\n中文");
        let next = host
            .executeInSession(&session.sessionId, "Write-Output $operit_v", 3000)
            .unwrap();
        assert_eq!(next.output, "中文");
    }

    #[test]
    fn visible_powershell_sessions_are_listed_as_pty() {
        let host = WindowsTerminalHost::new();
        let created = host
            .createOrGetSession("terminal_visible_ai")
            .expect("create visible terminal session");
        let manual = host
            .startPtySession(
                "terminal_visible_manual",
                TERMINAL,
                "powershell",
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
        assert_eq!(createdEntry.terminalType, "powershell");
        assert_eq!(manualEntry.sessionKind, "pty");
        assert_eq!(manualEntry.terminalType, "powershell");
    }
}
