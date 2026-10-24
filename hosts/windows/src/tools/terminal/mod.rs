use crate::process::background_command;

#[path = "../../../../common/terminal_command.rs"]
mod terminal_command;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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
const COMMAND_CANCEL_SETTLE_TIMEOUT_MS: u64 = 3000;
const PLATFORM: &str = "windows";
const TERMINAL: &str = "native";
const PRIMARY_TERMINAL_TYPE: &str = "powershell";

#[derive(Clone, Default)]
pub struct WindowsTerminalHost {
    state: Arc<Mutex<TerminalState>>,
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
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: PtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    commandRunning: bool,
    exitCode: Option<i32>,
}

type PtyCommandOutput = Arc<(Mutex<VecDeque<u8>>, Condvar)>;

#[derive(Debug)]
struct PtyCursor {
    row: usize,
    col: usize,
}

type PtyCursorState = Arc<Mutex<PtyCursor>>;

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalKind {
    Bash,
    PowerShell,
}

impl WindowsTerminalHost {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TerminalHost for WindowsTerminalHost {
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
            types: vec![
                TerminalTypeInfo {
                    terminal: TERMINAL.to_string(),
                    terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
                    available: commandStarts(
                        "powershell.exe",
                        &["-NoProfile", "-Command", "$PSVersionTable.PSVersion"],
                    ),
                    description: "Windows PowerShell terminal".to_string(),
                },
                TerminalTypeInfo {
                    terminal: TERMINAL.to_string(),
                    terminalType: "bash".to_string(),
                    available: gitBashExecutable().is_ok(),
                    description: "Windows Git Bash terminal".to_string(),
                },
            ],
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
        let (normalizedTerminalType, kind) = normalizeTerminalType(terminalType)?;
        let workDir = nonBlank(workingDir, "working_directory")?;
        let session = createPtySession(
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
        let mut state = self.lockState()?;
        let session = state
            .ptySessions
            .get_mut(sessionId)
            .ok_or_else(|| HostError::new(format!("PTY session does not exist: {sessionId}")))?;
        let mut writer = session
            .writer
            .lock()
            .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
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
                Ok(Some(code))
            }
            None => Ok(None),
        }
    }

    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        let mut state = self.lockState()?;
        let removed = state.ptySessions.remove(sessionId);
        match removed {
            Some(_) => {
                removeSessionMappings(&mut state, sessionId);
                Ok(())
            }
            None => Err(HostError::new(format!(
                "PTY session does not exist: {sessionId}"
            ))),
        }
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
            if session.exitCode.is_some() {
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

    fn executeInSessionStreaming(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let normalizedCommand = nonBlank(command, "command")?;
        let (terminalType, kind, commandOutput, renderer, commandEcho) = {
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
            )
        };

        let result = executePtyCommandInSession(
            commandOutput,
            kind,
            &commandEcho,
            timeoutMs,
            renderer,
            onOutput,
        );
        {
            let mut state = self.lockState()?;
            if let Some(session) = state.ptySessions.get_mut(&normalizedSessionId) {
                // Release the busy flag even if interrupting a broken PTY fails.
                // The state lock still excludes new commands during cancellation.
                session.commandRunning = false;
                if result.as_ref().map_or(true, |receipt| receipt.timedOut) {
                    if let Some(workingDir) = cancelTimedOutPtyCommand(session)? {
                        session.workingDir = workingDir;
                    }
                } else if let Some(workingDir) = result
                    .as_ref()
                    .ok()
                    .and_then(|receipt| receipt.workingDir.clone())
                {
                    session.workingDir = workingDir;
                }
            }
        }
        let result = result?;
        Ok(TerminalCommandOutput {
            command: normalizedCommand,
            output: result.output,
            exitCode: result.exitCode,
            sessionId: normalizedSessionId,
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType,
            timedOut: result.timedOut,
        })
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
        if input.is_none() && control.and_then(normalizeControl).is_none() {
            return Err(HostError::new(
                "At least one of input or control is required",
            ));
        }
        let mut state = self.lockState()?;
        let session = state
            .ptySessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let acceptedChars =
            applyPtyTerminalInput(session, input, control.and_then(normalizeControl))?;
        Ok(TerminalInputOutput {
            sessionId: normalizedSessionId,
            acceptedChars,
        })
    }

    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let mut state = self.lockState()?;
        if state.ptySessions.remove(&normalizedSessionId).is_none() {
            return Err(HostError::new(format!(
                "Terminal session does not exist: {sessionId}"
            )));
        }
        removeSessionMappings(&mut state, &normalizedSessionId);
        Ok(TerminalCloseOutput {
            sessionId: normalizedSessionId.clone(),
            success: true,
            message: format!("Terminal session closed: {normalizedSessionId}"),
        })
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
        let content = ptyScreenContent(session)?;
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
            terminalType: session.terminalType.clone(),
            rows,
            cols,
            content,
            commandRunning: session.commandRunning,
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

struct SessionCommandResult {
    output: String,
    exitCode: i32,
    timedOut: bool,
    workingDir: Option<String>,
}

#[allow(non_snake_case)]
fn createPtySession(
    sessionName: String,
    terminalType: String,
    kind: TerminalKind,
    workingDir: String,
    rows: u16,
    cols: u16,
) -> HostResult<PtySession> {
    let ptySystem = native_pty_system();
    let pair = ptySystem
        .openpty(ptySize(rows, cols))
        .map_err(toHostError)?;
    let command = ptyCommand(kind, &workingDir)?;
    let mut child = pair.slave.spawn_command(command).map_err(toHostError)?;
    let mut reader = pair.master.try_clone_reader().map_err(toHostError)?;
    let writer = Arc::new(Mutex::new(pair.master.take_writer().map_err(toHostError)?));
    let output = Arc::new(Mutex::new(VecDeque::new()));
    let commandOutput: PtyCommandOutput = Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
    let screenOutput = Arc::new(Mutex::new(VecDeque::new()));
    let cursorState: PtyCursorState = Arc::new(Mutex::new(PtyCursor { row: 1, col: 1 }));
    let outputForThread = output.clone();
    let commandOutputForThread = commandOutput.clone();
    let screenOutputForThread = screenOutput.clone();
    let writerForThread = writer.clone();
    let cursorStateForThread = cursorState.clone();
    let reader_task =
        operit_host_native_common::scheduleNativeServiceTask("operit-pty-reader", move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let chunk = &buffer[..count];
                        processPtyTerminalQueries(&writerForThread, &cursorStateForThread, chunk);
                        let commandChunk = stripPtyDeviceStatusReports(chunk);
                        let visibleChunk = stripPtyPromptMarkers(&commandChunk);
                        appendPtyOutput(&outputForThread, &visibleChunk);
                        appendPtyOutput(&screenOutputForThread, &visibleChunk);
                        appendPtyCommandOutput(&commandOutputForThread, &commandChunk);
                    }
                    Err(_) => break,
                }
            }
        });
    if let Err(error) = reader_task {
        let _ = child.kill();
        return Err(error);
    }
    if let Err(error) = waitForInitialPtyPrompt(commandOutput.clone(), Duration::from_millis(10000))
    {
        let _ = child.kill();
        return Err(error);
    }
    clearPtyCommandOutput(&commandOutput)?;
    Ok(PtySession {
        sessionName,
        terminalType,
        kind,
        workingDir,
        child,
        master: pair.master,
        writer,
        output,
        commandOutput,
        screenOutput,
        commandRunning: false,
        exitCode: None,
    })
}

/// Encodes a multiline script as one input line for its selected interactive interpreter.
#[allow(non_snake_case)]
fn ptyCommandLine(command: &str, kind: TerminalKind) -> String {
    match kind {
        TerminalKind::Bash => terminal_command::posixCommandLine(command),
        TerminalKind::PowerShell => {
            if !command.chars().any(char::is_control) {
                return command.to_string();
            }
            let encoded = BASE64_STANDARD.encode(command.as_bytes());
            format!("Invoke-Expression ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{encoded}')))")
        }
    }
}

#[allow(non_snake_case)]
/// Builds the PTY command for the selected Windows terminal type.
fn ptyCommand(kind: TerminalKind, workingDir: &str) -> HostResult<CommandBuilder> {
    match kind {
        TerminalKind::PowerShell => Ok(windowsPtyCommand(workingDir)),
        TerminalKind::Bash => {
            let mut command = CommandBuilder::new(gitBashExecutable()?);
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
            Ok(command)
        }
    }
}

#[allow(non_snake_case)]
/// Resolves the Bash executable from the active Git for Windows installation.
fn gitBashExecutable() -> HostResult<PathBuf> {
    let output = background_command("git")
        .arg("--exec-path")
        .output()
        .map_err(|error| {
            HostError::new(format!(
                "Failed to resolve Git for Windows exec path: {error}"
            ))
        })?;
    if !output.status.success() {
        return Err(HostError::new(format!(
            "git --exec-path failed with status {}",
            output.status
        )));
    }
    let execPathText = String::from_utf8(output.stdout)
        .map_err(|error| HostError::new(format!("Git exec path is not UTF-8: {error}")))?;
    let execPath = PathBuf::from(execPathText.trim());
    let gitRoot = execPath
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .ok_or_else(|| {
            HostError::new(format!(
                "Unexpected Git for Windows exec path: {}",
                execPath.display()
            ))
        })?;
    let bashExecutable = gitRoot.join("bin").join("bash.exe");
    if !bashExecutable.is_file() {
        return Err(HostError::new(format!(
            "Git Bash executable does not exist: {}",
            bashExecutable.display()
        )));
    }
    Ok(bashExecutable)
}

#[allow(non_snake_case)]
/// Builds the Windows PowerShell PTY command.
fn windowsPtyCommand(workingDir: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("powershell.exe");
    command.arg("-NoLogo");
    command.arg("-NoProfile");
    command.arg("-ExecutionPolicy");
    command.arg("Bypass");
    command.arg("-NoExit");
    command.arg("-Command");
    command.arg(powershellPtyInitScript());
    command.cwd(workingDir);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("LANG", "C.UTF-8");
    command
}

#[allow(non_snake_case)]
fn powershellPtyInitScript() -> &'static str {
    "$global:__operit_error_count = $global:error.Count; [Console]::OutputEncoding = [Text.Encoding]::UTF8; $OutputEncoding = [Text.Encoding]::UTF8; function global:prompt { $__operit_success = $?; $__operit_error_changed = $global:error.Count -gt $global:__operit_error_count; $global:__operit_error_count = $global:error.Count; $__operit_location = (Get-Location).Path; $__operit_pwd = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($__operit_location)); $__operit_exit_code = if ($__operit_success -and -not $__operit_error_changed) { 0 } elseif ($LASTEXITCODE -is [int] -and $LASTEXITCODE -ne 0) { [int]$LASTEXITCODE } else { 1 }; ([char]27) + ']133;OperitPrompt=' + $__operit_pwd + ':' + $__operit_exit_code + ([char]7) + 'PS ' + $__operit_location + '> ' }"
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
fn appendPtyOutput(output: &Arc<Mutex<VecDeque<u8>>>, data: &[u8]) {
    if let Ok(mut buffer) = output.lock() {
        buffer.extend(data.iter().copied());
        while buffer.len() > PTY_OUTPUT_LIMIT {
            buffer.pop_front();
        }
    }
}

#[allow(non_snake_case)]
fn appendPtyCommandOutput(output: &PtyCommandOutput, data: &[u8]) {
    let (lock, condvar) = &**output;
    if let Ok(mut buffer) = lock.lock() {
        buffer.extend(data.iter().copied());
        while buffer.len() > PTY_OUTPUT_LIMIT {
            buffer.pop_front();
        }
        condvar.notify_all();
    }
}

#[allow(non_snake_case)]
fn processPtyTerminalQueries(
    writer: &Arc<Mutex<Box<dyn Write + Send>>>,
    cursorState: &PtyCursorState,
    data: &[u8],
) {
    let mut index = 0usize;
    while index < data.len() {
        if data[index..].starts_with(b"\x1b[6n") {
            if let Ok(cursor) = cursorState.lock() {
                let response = format!("\x1b[{};{}R", cursor.row, cursor.col);
                if let Ok(mut writer) = writer.lock() {
                    let _ = writer.write_all(response.as_bytes());
                    let _ = writer.flush();
                }
            }
            index += 4;
            continue;
        }
        if data[index..].starts_with(PTY_PROMPT_MARKER_PREFIX) {
            let payloadStart = index + PTY_PROMPT_MARKER_PREFIX.len();
            if let Some(relativeEnd) = data[payloadStart..]
                .iter()
                .position(|value| *value == PTY_PROMPT_MARKER_END)
            {
                index = payloadStart + relativeEnd + 1;
                continue;
            }
        }
        if data[index] == b'\x1b' {
            if index + 1 < data.len() && data[index + 1] == b'[' {
                index += 2;
                while index < data.len() {
                    let value = data[index];
                    index += 1;
                    if value >= b'@' && value <= b'~' {
                        break;
                    }
                }
                continue;
            }
            if index + 1 < data.len() && data[index + 1] == b']' {
                index += 2;
                while index < data.len() {
                    let value = data[index];
                    index += 1;
                    if value == PTY_PROMPT_MARKER_END {
                        break;
                    }
                }
                continue;
            }
        }
        updatePtyCursor(cursorState, data[index]);
        index += 1;
    }
}

#[allow(non_snake_case)]
fn updatePtyCursor(cursorState: &PtyCursorState, value: u8) {
    if let Ok(mut cursor) = cursorState.lock() {
        match value {
            b'\r' => cursor.col = 1,
            b'\n' => {
                cursor.row += 1;
                cursor.col = 1;
            }
            8 => {
                if cursor.col > 1 {
                    cursor.col -= 1;
                }
            }
            0x20..=0x7e => cursor.col += 1,
            _ => {}
        }
    }
}

#[allow(non_snake_case)]
fn clearPtyCommandOutput(output: &PtyCommandOutput) -> HostResult<()> {
    let (lock, _) = &**output;
    let mut buffer = lock
        .lock()
        .map_err(|_| HostError::new("pty command output mutex poisoned"))?;
    buffer.clear();
    Ok(())
}

#[allow(non_snake_case)]
fn waitForInitialPtyPrompt(output: PtyCommandOutput, timeout: Duration) -> HostResult<()> {
    let deadline = Instant::now() + timeout;
    let mut collected = Vec::new();
    let mut promptSeenAt = None;
    let quietPeriod = Duration::from_millis(150);
    loop {
        let drained = drainPtyCommandOutput(&output, &mut collected, deadline)?;
        if drained > 0 {
            promptSeenAt = None;
        }
        if findLastPtyPromptMarker(&collected)?.is_some() && promptSeenAt.is_none() {
            promptSeenAt = Some(Instant::now());
        }
        if promptSeenAt.is_some_and(|seenAt| seenAt.elapsed() >= quietPeriod) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(HostError::new("Timed out waiting for terminal prompt"));
        }
    }
}

#[allow(non_snake_case)]
/// Waits for a typed PTY command to complete and extracts its output.
fn executePtyCommandInSession(
    output: PtyCommandOutput,
    kind: TerminalKind,
    command: &str,
    timeoutMs: u64,
    renderer: TerminalTextRenderer,
    onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
) -> HostResult<SessionCommandResult> {
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    let mut collected = Vec::new();
    let mut promptSeenAt = None;
    let quietPeriod = Duration::from_millis(150);
    loop {
        let previousLength = collected.len();
        let drained = drainPtyCommandOutput(&output, &mut collected, deadline)?;
        onOutput(&collected[previousLength..])?;
        if drained > 0 {
            promptSeenAt = None;
        }
        if let Some(marker) = findLastPtyPromptMarker(&collected)? {
            if promptSeenAt.is_none() {
                promptSeenAt = Some(Instant::now());
            }
            if !promptSeenAt.is_some_and(|seenAt| seenAt.elapsed() >= quietPeriod) {
                if Instant::now() < deadline {
                    continue;
                }
            }
            let output = ptyCommandOutputText(
                &collected,
                kind,
                command,
                Some(&marker.workingDir),
                renderer,
            )?;
            return Ok(SessionCommandResult {
                output,
                exitCode: marker.exitCode,
                timedOut: false,
                workingDir: Some(marker.workingDir),
            });
        }
        if Instant::now() >= deadline {
            let output = ptyCommandOutputText(&collected, kind, command, None, renderer)?;
            return Ok(SessionCommandResult {
                output,
                exitCode: -1,
                timedOut: true,
                workingDir: None,
            });
        }
    }
}

#[allow(non_snake_case)]
fn cancelTimedOutPtyCommand(session: &mut PtySession) -> HostResult<Option<String>> {
    {
        let mut writer = session
            .writer
            .lock()
            .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
        writer.write_all(b"\x03")?;
        writer.flush()?;
    }

    let deadline = Instant::now() + Duration::from_millis(COMMAND_CANCEL_SETTLE_TIMEOUT_MS);
    let mut collected = Vec::new();
    while Instant::now() < deadline {
        let drained = drainPtyCommandOutput(&session.commandOutput, &mut collected, deadline)?;
        if drained == 0 {
            continue;
        }
        if let Some(marker) = findLastPtyPromptMarker(&collected)? {
            return Ok(Some(marker.workingDir));
        }
    }
    Ok(None)
}

#[allow(non_snake_case)]
fn drainPtyCommandOutput(
    output: &PtyCommandOutput,
    collected: &mut Vec<u8>,
    deadline: Instant,
) -> HostResult<usize> {
    let (lock, condvar) = &**output;
    let mut buffer = lock
        .lock()
        .map_err(|_| HostError::new("pty command output mutex poisoned"))?;
    if buffer.is_empty() {
        let now = Instant::now();
        if now < deadline {
            let wait = (deadline - now).min(Duration::from_millis(100));
            let result = condvar
                .wait_timeout(buffer, wait)
                .map_err(|_| HostError::new("pty command output mutex poisoned"))?;
            buffer = result.0;
        }
    }
    let mut drained = 0usize;
    while let Some(value) = buffer.pop_front() {
        collected.push(value);
        drained += 1;
    }
    Ok(drained)
}

#[derive(Clone, Debug)]
struct PtyPromptMarker {
    start: usize,
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
        start,
        workingDir,
        exitCode,
    }))
}

#[allow(non_snake_case)]
fn rfindBytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
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
/// Renders command output from the captured viewport and removes the shell-owned echo and prompt.
fn ptyCommandOutputText(
    raw: &[u8],
    kind: TerminalKind,
    command: &str,
    workingDir: Option<&str>,
    mut renderer: TerminalTextRenderer,
) -> HostResult<String> {
    let startRow = renderer.logicalLineStart();
    renderer.write(&String::from_utf8_lossy(raw));
    let withoutEcho = dropCommandEcho(renderer.textFrom(startRow), command);
    let Some(workingDir) = workingDir else {
        return Ok(withoutEcho.trim().to_string());
    };
    match kind {
        TerminalKind::PowerShell => dropTrailingPowerShellPrompt(&withoutEcho, workingDir),
        TerminalKind::Bash => dropTrailingBashPrompt(&withoutEcho, workingDir),
    }
}

#[allow(non_snake_case)]
/// Reconstructs the terminal viewport and scrollback using the session's actual dimensions.
fn ptyScreenRenderer(session: &PtySession) -> HostResult<TerminalTextRenderer> {
    let size = session.master.get_size().map_err(toHostError)?;
    let buffer = session
        .screenOutput
        .lock()
        .map_err(|_| HostError::new("pty screen output mutex poisoned"))?;
    let bytes = buffer.iter().copied().collect::<Vec<_>>();
    let mut renderer = TerminalTextRenderer::new(usize::from(size.rows), usize::from(size.cols));
    renderer.write(&String::from_utf8_lossy(&bytes));
    Ok(renderer)
}

#[allow(non_snake_case)]
/// Returns the rendered session transcript without exposing internal completion markers.
fn ptyScreenContent(session: &PtySession) -> HostResult<String> {
    Ok(ptyScreenRenderer(session)?.textFrom(0))
}

#[allow(non_snake_case)]
fn stripPtyPromptMarkers(raw: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(raw.len());
    let mut index = 0usize;
    while index < raw.len() {
        if raw[index..].starts_with(PTY_PROMPT_MARKER_PREFIX) {
            let payloadStart = index + PTY_PROMPT_MARKER_PREFIX.len();
            if let Some(relativeEnd) = raw[payloadStart..]
                .iter()
                .position(|value| *value == PTY_PROMPT_MARKER_END)
            {
                index = payloadStart + relativeEnd + 1;
                continue;
            }
        }
        output.push(raw[index]);
        index += 1;
    }
    output
}

#[allow(non_snake_case)]
fn stripPtyDeviceStatusReports(raw: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(raw.len());
    let mut index = 0usize;
    while index < raw.len() {
        if raw[index..].starts_with(b"\x1b[6n") {
            index += 4;
            continue;
        }
        output.push(raw[index]);
        index += 1;
    }
    output
}

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
        let end = (self.col + count).min(line.len());
        for cell in &mut line[self.col..end] {
            *cell = ' ';
        }
    }

    fn ensureCursorLine(&mut self) {
        while self.lines.len() <= self.row {
            self.lines.push(Vec::new());
        }
    }
}

#[allow(non_snake_case)]
fn csiParams(params: &str) -> Vec<Option<usize>> {
    let params = params
        .trim_start_matches('?')
        .trim_start_matches('>')
        .trim_start_matches('!');
    params
        .split(';')
        .map(|value| {
            if value.is_empty() {
                None
            } else {
                value.parse::<usize>().ok()
            }
        })
        .collect()
}

#[allow(non_snake_case)]
fn csiValue(values: &[Option<usize>], index: usize, defaultValue: usize) -> usize {
    values
        .get(index)
        .and_then(|value| *value)
        .unwrap_or(defaultValue)
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

#[allow(non_snake_case)]
/// Removes the trailing PowerShell prompt from command output.
fn dropTrailingPowerShellPrompt(value: &str, workingDir: &str) -> HostResult<String> {
    let prompt = format!("PS {workingDir}>");
    let output = value
        .trim_end()
        .strip_suffix(&prompt)
        .ok_or_else(|| HostError::new(format!("Missing PowerShell prompt suffix: {prompt}")))?;
    Ok(output.trim().to_string())
}

#[allow(non_snake_case)]
/// Removes the trailing Git Bash prompt from command output.
fn dropTrailingBashPrompt(value: &str, workingDir: &str) -> HostResult<String> {
    let prompt = format!("{workingDir} $");
    let output = value
        .trim_end()
        .strip_suffix(&prompt)
        .ok_or_else(|| HostError::new(format!("Missing Git Bash prompt suffix: {prompt}")))?;
    Ok(output.trim().to_string())
}

#[allow(non_snake_case)]
fn toHostError(error: impl std::fmt::Display) -> HostError {
    HostError::new(error.to_string())
}

#[allow(non_snake_case)]
fn parseExitCode(exitCodeText: &str) -> HostResult<i32> {
    exitCodeText.trim().parse::<i32>().map_err(|error| {
        HostError::new(format!(
            "Invalid terminal exit code marker '{exitCodeText}': {error}"
        ))
    })
}

#[allow(non_snake_case)]
fn applyPtyTerminalInput(
    session: &mut PtySession,
    input: Option<&str>,
    control: Option<&str>,
) -> HostResult<usize> {
    let mut acceptedChars = 0;
    if let Some(input) = input {
        let mut writer = session
            .writer
            .lock()
            .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
        writer.write_all(input.as_bytes())?;
        writer.flush()?;
        acceptedChars += input.chars().count();
    }
    if let Some(control) = control {
        let sequence = match control {
            "enter" => "\r".to_string(),
            value => controlToSequence(value, input)?,
        };
        let mut writer = session
            .writer
            .lock()
            .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
        writer.write_all(sequence.as_bytes())?;
        writer.flush()?;
        acceptedChars += sequence.chars().count();
    }
    Ok(acceptedChars)
}

#[allow(non_snake_case)]
fn controlToSequence(control: &str, input: Option<&str>) -> HostResult<String> {
    match control {
        "enter" => Ok("\n".to_string()),
        "tab" => Ok("\t".to_string()),
        "esc" => Ok("\x1b".to_string()),
        "up" => Ok("\x1b[A".to_string()),
        "down" => Ok("\x1b[B".to_string()),
        "right" => Ok("\x1b[C".to_string()),
        "left" => Ok("\x1b[D".to_string()),
        "home" => Ok("\x1b[H".to_string()),
        "end" => Ok("\x1b[F".to_string()),
        "pageup" => Ok("\x1b[5~".to_string()),
        "pagedown" => Ok("\x1b[6~".to_string()),
        "delete" => Ok("\x1b[3~".to_string()),
        "backspace" => Ok("\x7f".to_string()),
        "ctrl" | "control" => ctrlSequence(input),
        "alt" | "meta" | "cmd" => Ok(format!("\x1b{}", input.unwrap_or(""))),
        "shift" => Ok(input.unwrap_or("").to_uppercase()),
        other => Err(HostError::new(format!(
            "Unsupported terminal control: {other}"
        ))),
    }
}

#[allow(non_snake_case)]
fn ctrlSequence(input: Option<&str>) -> HostResult<String> {
    let input = input.ok_or_else(|| HostError::new("ctrl control requires input"))?;
    let mut chars = input.chars();
    let value = chars
        .next()
        .ok_or_else(|| HostError::new("ctrl control requires input"))?;
    if chars.next().is_some() {
        return Err(HostError::new(
            "ctrl control input must be a single character",
        ));
    }
    let code = match value.to_ascii_uppercase() {
        'A'..='Z' => value.to_ascii_uppercase() as u8 - b'A' + 1,
        '[' => 27,
        '\\' => 28,
        ']' => 29,
        '^' => 30,
        '_' => 31,
        '?' => 127,
        other => {
            return Err(HostError::new(format!(
                "Unsupported ctrl control input: {other}"
            )))
        }
    };
    Ok((code as char).to_string())
}

#[allow(non_snake_case)]
fn normalizeControl(rawControl: &str) -> Option<&'static str> {
    match rawControl.trim().to_ascii_lowercase().as_str() {
        "" => None,
        "return" => Some("enter"),
        "escape" => Some("esc"),
        "arrowup" => Some("up"),
        "arrowdown" => Some("down"),
        "arrowleft" => Some("left"),
        "arrowright" => Some("right"),
        "pgup" | "page_up" => Some("pageup"),
        "pgdn" | "page_down" => Some("pagedown"),
        "del" => Some("delete"),
        "enter" => Some("enter"),
        "tab" => Some("tab"),
        "esc" => Some("esc"),
        "up" => Some("up"),
        "down" => Some("down"),
        "left" => Some("left"),
        "right" => Some("right"),
        "home" => Some("home"),
        "end" => Some("end"),
        "pageup" => Some("pageup"),
        "pagedown" => Some("pagedown"),
        "delete" => Some("delete"),
        "backspace" => Some("backspace"),
        "ctrl" | "control" => Some("ctrl"),
        "alt" => Some("alt"),
        "shift" => Some("shift"),
        "meta" => Some("meta"),
        "cmd" => Some("cmd"),
        _ => None,
    }
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
fn sessionKey(terminalType: &str, name: &str) -> String {
    format!("{terminalType}:{name}")
}

#[allow(non_snake_case)]
fn removeSessionMappings(state: &mut TerminalState, sessionId: &str) {
    state.sessionNameToId.retain(|_, value| value != sessionId);
    state
        .hiddenExecutorKeyToSessionId
        .retain(|_, value| value != sessionId);
}

#[allow(non_snake_case)]
fn nonBlank(value: &str, paramName: &str) -> HostResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HostError::new(format!("{paramName} parameter is required")));
    }
    Ok(trimmed.to_string())
}

#[allow(non_snake_case)]
fn commandStarts(program: &str, args: &[&str]) -> bool {
    background_command(program).args(args).output().is_ok()
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

    #[test]
    fn powershell_parse_error_returns_without_timeout() {
        let host = WindowsTerminalHost::new();
        let session = host
            .createOrGetSession("terminal_powershell_parse_error")
            .expect("create terminal session");
        let result = host
            .executeInSession(&session.sessionId, "date /t && time /t", 3000)
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
