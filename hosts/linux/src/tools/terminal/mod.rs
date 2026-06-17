use std::collections::{BTreeMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

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

#[derive(Clone, Default)]
pub struct LinuxTerminalHost {
    state: Arc<Mutex<TerminalState>>,
}

#[derive(Default)]
struct TerminalState {
    sessions: BTreeMap<String, TerminalSession>,
    sessionNameToId: BTreeMap<String, String>,
    hiddenExecutorKeyToSessionId: BTreeMap<String, String>,
    ptySessions: BTreeMap<String, PtySession>,
}

struct TerminalSession {
    id: String,
    name: String,
    terminalType: String,
    child: Child,
    stdin: ChildStdin,
    stdoutRx: Receiver<String>,
    stderrLines: Arc<Mutex<VecDeque<String>>>,
    screenLines: VecDeque<String>,
    commandRunning: bool,
}

struct PtySession {
    sessionName: String,
    terminalType: String,
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

impl LinuxTerminalHost {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TerminalHost for LinuxTerminalHost {
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "linux".to_string(),
            defaultType: "linux".to_string(),
            types: vec![TerminalTypeInfo {
                terminalType: "linux".to_string(),
                available: true,
                description: "Linux bash terminal".to_string(),
            }],
        })
    }

    fn startPtySession(
        &self,
        sessionName: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        let workDir = nonBlank(workingDir, "working_directory")?;
        let session = createPtySession(
            normalizedSessionName,
            "linux".to_string(),
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
            Some(_) => Ok(()),
            None => Err(HostError::new(format!(
                "PTY session does not exist: {sessionId}"
            ))),
        }
    }

    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let mut state = self.lockState()?;
        let mut entries = Vec::new();
        for session in state.sessions.values() {
            entries.push(TerminalSessionListEntry {
                sessionId: session.id.clone(),
                sessionName: session.name.clone(),
                terminalType: session.terminalType.clone(),
                sessionKind: "shell".to_string(),
                workingDir: String::new(),
                commandRunning: session.commandRunning,
            });
        }
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
                terminalType: session.terminalType.clone(),
                sessionKind: "pty".to_string(),
                workingDir: session.workingDir.clone(),
                commandRunning: session.commandRunning,
            });
        }
        Ok(entries)
    }

    fn createOrGetSession(
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
                if state.ptySessions.contains_key(&sessionId)
                    || state.sessions.contains_key(&sessionId)
                {
                    return Ok(TerminalSessionInfo {
                        sessionId,
                        sessionName: normalizedSessionName,
                        terminalType: normalizedTerminalType,
                        isNewSession: false,
                    });
                }
                state.sessionNameToId.remove(&sessionKey);
            }
        }

        let workingDir = std::env::current_dir()?.display().to_string();
        let session = createPtySession(
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
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let normalizedCommand = nonBlank(command, "command")?;
        let ptyExecution = {
            let mut state = self.lockState()?;
            if let Some(session) = state.ptySessions.get_mut(&normalizedSessionId) {
                session.commandRunning = true;
                clearPtyCommandOutput(&session.commandOutput)?;
                let commandInput = format!("{normalizedCommand}\r");
                let mut writer = session
                    .writer
                    .lock()
                    .map_err(|_| HostError::new("pty writer mutex poisoned"))?;
                writer.write_all(commandInput.as_bytes())?;
                writer.flush()?;
                Some((session.terminalType.clone(), session.commandOutput.clone()))
            } else {
                None
            }
        };

        if let Some((terminalType, commandOutput)) = ptyExecution {
            let result = executePtyCommandInSession(commandOutput, &normalizedCommand, timeoutMs)?;
            {
                let mut state = self.lockState()?;
                if let Some(session) = state.ptySessions.get_mut(&normalizedSessionId) {
                    session.commandRunning = false;
                    if !result.timedOut {
                        if let Some(workingDir) = result.workingDir.clone() {
                            session.workingDir = workingDir;
                        }
                    }
                }
            }
            return Ok(TerminalCommandOutput {
                command: normalizedCommand,
                output: result.output,
                exitCode: result.exitCode,
                sessionId: normalizedSessionId,
                terminalType,
                timedOut: result.timedOut,
            });
        }

        let mut state = self.lockState()?;
        let session = state
            .sessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let result = executeShellCommandInSession(session, &normalizedCommand, timeoutMs)?;
        Ok(TerminalCommandOutput {
            command: normalizedCommand,
            output: result.output,
            exitCode: result.exitCode,
            sessionId: normalizedSessionId,
            terminalType: session.terminalType.clone(),
            timedOut: result.timedOut,
        })
    }

    fn executeHiddenCommand(
        &self,
        command: &str,
        terminalType: &str,
        executorKey: &str,
        timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        let normalizedCommand = nonBlank(command, "command")?;
        let normalizedTerminalType = normalizeTerminalType(terminalType)?;
        let normalizedExecutorKey = match executorKey.trim() {
            "" => "default".to_string(),
            value => value.to_string(),
        };
        let executorKey = sessionKey(&normalizedTerminalType, &normalizedExecutorKey);
        let mut state = self.lockState()?;
        let sessionId = match state
            .hiddenExecutorKeyToSessionId
            .get(&executorKey)
            .cloned()
        {
            Some(sessionId) if state.sessions.contains_key(&sessionId) => sessionId,
            Some(sessionId) => {
                state.hiddenExecutorKeyToSessionId.remove(&executorKey);
                let _ = sessionId;
                let session = createShellSession(
                    format!("hidden:{normalizedExecutorKey}"),
                    normalizedTerminalType.clone(),
                )?;
                let sessionId = session.id.clone();
                state
                    .hiddenExecutorKeyToSessionId
                    .insert(executorKey.clone(), sessionId.clone());
                state.sessions.insert(sessionId.clone(), session);
                sessionId
            }
            None => {
                let session = createShellSession(
                    format!("hidden:{normalizedExecutorKey}"),
                    normalizedTerminalType.clone(),
                )?;
                let sessionId = session.id.clone();
                state
                    .hiddenExecutorKeyToSessionId
                    .insert(executorKey, sessionId.clone());
                state.sessions.insert(sessionId.clone(), session);
                sessionId
            }
        };
        let session = state.sessions.get_mut(&sessionId).ok_or_else(|| {
            HostError::new(format!("Hidden terminal session missing: {sessionId}"))
        })?;
        let result = executeShellCommandInSession(session, &normalizedCommand, timeoutMs)?;
        Ok(HiddenTerminalCommandOutput {
            command: normalizedCommand,
            output: result.output,
            exitCode: result.exitCode,
            executorKey: normalizedExecutorKey,
            terminalType: normalizedTerminalType,
            timedOut: result.timedOut,
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
        if let Some(session) = state.ptySessions.get_mut(&normalizedSessionId) {
            let acceptedChars =
                applyPtyTerminalInput(session, input, control.and_then(normalizeControl))?;
            return Ok(TerminalInputOutput {
                sessionId: normalizedSessionId,
                acceptedChars,
            });
        }
        let session = state
            .sessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let acceptedChars = applyTerminalInput(session, input, control.and_then(normalizeControl))?;
        Ok(TerminalInputOutput {
            sessionId: normalizedSessionId,
            acceptedChars,
        })
    }

    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let mut state = self.lockState()?;
        if state.ptySessions.remove(&normalizedSessionId).is_some() {
            state
                .sessionNameToId
                .retain(|_, value| value != &normalizedSessionId);
            return Ok(TerminalCloseOutput {
                sessionId: normalizedSessionId.clone(),
                success: true,
                message: format!("Terminal session closed: {normalizedSessionId}"),
            });
        }
        let mut session = state.sessions.remove(&normalizedSessionId).ok_or_else(|| {
            HostError::new(format!("Terminal session does not exist: {sessionId}"))
        })?;
        let _ = session.child.kill();
        state
            .sessionNameToId
            .retain(|_, value| value != &normalizedSessionId);
        state
            .hiddenExecutorKeyToSessionId
            .retain(|_, value| value != &normalizedSessionId);
        Ok(TerminalCloseOutput {
            sessionId: normalizedSessionId.clone(),
            success: true,
            message: format!("Terminal session closed: {normalizedSessionId}"),
        })
    }

    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let mut state = self.lockState()?;
        if let Some(session) = state.ptySessions.get_mut(&normalizedSessionId) {
            let content = ptyScreenContent(session)?;
            let rows = content.lines().count();
            let cols = content
                .lines()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0);
            return Ok(TerminalScreenOutput {
                sessionId: normalizedSessionId,
                terminalType: session.terminalType.clone(),
                rows,
                cols,
                content,
                commandRunning: session.commandRunning,
            });
        }
        let session = state
            .sessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        drainLiveShellOutputToScreen(session)?;
        let content = session
            .screenLines
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let rows = session.screenLines.len();
        let cols = session
            .screenLines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        Ok(TerminalScreenOutput {
            sessionId: normalizedSessionId,
            terminalType: session.terminalType.clone(),
            rows,
            cols,
            content,
            commandRunning: session.commandRunning,
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

struct SessionCommandResult {
    output: String,
    exitCode: i32,
    timedOut: bool,
    workingDir: Option<String>,
}

#[allow(non_snake_case)]
fn createShellSession(name: String, terminalType: String) -> HostResult<TerminalSession> {
    let mut child = Command::new("sh")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| HostError::new("terminal shell has no stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| HostError::new("terminal shell has no stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| HostError::new("terminal shell has no stderr"))?;
    let (stdoutTx, stdoutRx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().flatten() {
            let _ = stdoutTx.send(line);
        }
    });
    let stderrLines = Arc::new(Mutex::new(VecDeque::new()));
    let stderrLinesForThread = stderrLines.clone();
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines().flatten() {
            if let Ok(mut lines) = stderrLinesForThread.lock() {
                lines.push_back(line);
                while lines.len() > 400 {
                    lines.pop_front();
                }
            }
        }
    });
    Ok(TerminalSession {
        id: nextSessionId(),
        name,
        terminalType,
        child,
        stdin,
        stdoutRx,
        stderrLines,
        screenLines: VecDeque::new(),
        commandRunning: false,
    })
}

#[allow(non_snake_case)]
fn createPtySession(
    sessionName: String,
    terminalType: String,
    workingDir: String,
    rows: u16,
    cols: u16,
) -> HostResult<PtySession> {
    let ptySystem = native_pty_system();
    let pair = ptySystem
        .openpty(ptySize(rows, cols))
        .map_err(toHostError)?;
    let command = linuxPtyCommand(&workingDir);
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
    thread::spawn(move || {
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
    if let Err(error) = waitForInitialPtyPrompt(commandOutput.clone(), Duration::from_millis(10000))
    {
        let _ = child.kill();
        return Err(error);
    }
    clearPtyCommandOutput(&commandOutput)?;
    Ok(PtySession {
        sessionName,
        terminalType,
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
fn executePtyCommandInSession(
    output: PtyCommandOutput,
    command: &str,
    timeoutMs: u64,
) -> HostResult<SessionCommandResult> {
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    let mut collected = Vec::new();
    let mut promptSeenAt = None;
    let quietPeriod = Duration::from_millis(150);
    loop {
        let drained = drainPtyCommandOutput(&output, &mut collected, deadline)?;
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
            let output = ptyCommandOutputText(&collected, command, Some(&marker.workingDir));
            return Ok(SessionCommandResult {
                output,
                exitCode: marker.exitCode,
                timedOut: false,
                workingDir: Some(marker.workingDir),
            });
        }
        if Instant::now() >= deadline {
            let output = ptyCommandOutputText(&collected, command, None);
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
fn rfindBytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
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
fn ptyScreenContent(session: &PtySession) -> HostResult<String> {
    let buffer = session
        .screenOutput
        .lock()
        .map_err(|_| HostError::new("pty screen output mutex poisoned"))?;
    let bytes = buffer.iter().copied().collect::<Vec<_>>();
    let visibleBytes = stripPtyPromptMarkers(&bytes);
    let text = String::from_utf8_lossy(&visibleBytes);
    Ok(renderTerminalText(&text))
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

#[derive(Default)]
struct TerminalTextRenderer {
    lines: Vec<Vec<char>>,
    row: usize,
    col: usize,
    savedRow: usize,
    savedCol: usize,
}

impl TerminalTextRenderer {
    fn new() -> Self {
        Self {
            lines: vec![Vec::new()],
            row: 0,
            col: 0,
            savedRow: 0,
            savedCol: 0,
        }
    }

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

    fn text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
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

    fn applyCsi(&mut self, params: &str, command: char) {
        let values = csiParams(params);
        match command {
            'A' => self.moveRowUp(csiValue(&values, 0, 1)),
            'B' => self.moveRowDown(csiValue(&values, 0, 1)),
            'C' => self.col += csiValue(&values, 0, 1),
            'D' => self.col = self.col.saturating_sub(csiValue(&values, 0, 1)),
            'G' => self.col = csiValue(&values, 0, 1).saturating_sub(1),
            'H' | 'f' => {
                self.row = csiValue(&values, 0, 1).saturating_sub(1);
                self.col = csiValue(&values, 1, 1).saturating_sub(1);
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

    fn writeChar(&mut self, ch: char) {
        match ch {
            '\r' => self.col = 0,
            '\n' => {
                self.row += 1;
                self.col = 0;
                self.ensureCursorLine();
            }
            '\x08' => {
                self.col = self.col.saturating_sub(1);
            }
            '\t' => {
                let nextStop = ((self.col / 8) + 1) * 8;
                while self.col < nextStop {
                    self.putChar(' ');
                }
            }
            _ if ch.is_control() => {}
            _ => self.putChar(ch),
        }
    }

    fn putChar(&mut self, ch: char) {
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

    fn moveRowUp(&mut self, count: usize) {
        self.row = self.row.saturating_sub(count);
        self.ensureCursorLine();
    }

    fn moveRowDown(&mut self, count: usize) {
        self.row += count;
        self.ensureCursorLine();
    }

    fn eraseDisplay(&mut self, mode: usize) {
        match mode {
            0 => {
                self.eraseLine(0);
                self.lines.truncate(self.row + 1);
            }
            1 => {
                for row in 0..self.row {
                    self.lines[row].clear();
                }
                self.eraseLine(1);
            }
            2 | 3 => {
                self.lines.clear();
                self.lines.push(Vec::new());
                self.row = 0;
                self.col = 0;
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
fn renderTerminalText(value: &str) -> String {
    let mut renderer = TerminalTextRenderer::new();
    renderer.write(value);
    renderer.text()
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

#[allow(non_snake_case)]
fn dropCommandEcho(value: String, command: &str) -> String {
    let mut lines = value.lines().map(str::to_string).collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    if lines.first().is_some_and(|line| {
        let line = line.trim_end();
        let command = command.trim();
        line == command || line.ends_with(command)
    }) {
        lines.remove(0);
    }
    lines.join("\n").trim().to_string()
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
        "" | "linux" => Ok("linux".to_string()),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for linux host: {value}"
        ))),
    }
}

#[allow(non_snake_case)]
fn sessionKey(terminalType: &str, name: &str) -> String {
    format!("{terminalType}:{name}")
}

#[allow(non_snake_case)]
fn executeShellCommandInSession(
    session: &mut TerminalSession,
    command: &str,
    timeoutMs: u64,
) -> HostResult<SessionCommandResult> {
    let marker = format!(
        "__OPERIT_TERMINAL_{}__",
        NEXT_SESSION_ID.fetch_add(1, Ordering::SeqCst)
    );
    let endMarkerPrefix = format!("{marker}_END:");
    let script = format!(
        "printf '%s\\n' '{marker}_START'\n{{\n{command}\n}}\n__operit_exit_code=$?\nprintf '%s%s\\n' '{endMarkerPrefix}' \"$__operit_exit_code\"\n"
    );
    session.commandRunning = true;
    session.stdin.write_all(script.as_bytes())?;
    session.stdin.flush()?;

    let deadline = Duration::from_millis(timeoutMs);
    let start = SystemTime::now();
    let mut outputLines = Vec::new();
    let mut sawStart = false;
    loop {
        let elapsed = start.elapsed().unwrap_or(Duration::from_millis(timeoutMs));
        if elapsed >= deadline {
            session.commandRunning = false;
            let output = joinOutput(outputLines, drainStderr(session)?);
            appendScreenLines(session, &output);
            return Ok(SessionCommandResult {
                output,
                exitCode: -1,
                timedOut: true,
                workingDir: None,
            });
        }
        let remaining = deadline - elapsed;
        let wait = remaining.min(Duration::from_millis(100));
        match session.stdoutRx.recv_timeout(wait) {
            Ok(line) => {
                if line == format!("{marker}_START") {
                    sawStart = true;
                    continue;
                }
                if sawStart {
                    if let Some(endMarkerIndex) = line.rfind(&endMarkerPrefix) {
                        let outputBeforeEndMarker = &line[..endMarkerIndex];
                        if !outputBeforeEndMarker.is_empty() {
                            outputLines.push(outputBeforeEndMarker.to_string());
                        }
                        session.commandRunning = false;
                        let exitCodeText = line[endMarkerIndex + endMarkerPrefix.len()..].trim();
                        let exitCode = parseExitCode(exitCodeText)?;
                        let output = joinOutput(outputLines, drainStderr(session)?);
                        appendScreenLines(session, &output);
                        return Ok(SessionCommandResult {
                            output,
                            exitCode,
                            timedOut: false,
                            workingDir: None,
                        });
                    }
                    outputLines.push(line);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                session.commandRunning = false;
                return Err(HostError::new(format!(
                    "Terminal session '{}' closed while executing command",
                    session.name
                )));
            }
        }
    }
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
fn drainStderr(session: &TerminalSession) -> HostResult<Vec<String>> {
    let mut lines = session
        .stderrLines
        .lock()
        .map_err(|_| HostError::new("terminal stderr mutex poisoned"))?;
    let mut collected = Vec::new();
    while let Some(line) = lines.pop_front() {
        collected.push(line);
    }
    Ok(collected)
}

#[allow(non_snake_case)]
fn drainLiveShellOutputToScreen(session: &mut TerminalSession) -> HostResult<()> {
    while let Ok(line) = session.stdoutRx.try_recv() {
        appendScreenLines(session, &line);
    }
    let stderrLines = drainStderr(session)?;
    for line in stderrLines {
        appendScreenLines(session, &line);
    }
    Ok(())
}

#[allow(non_snake_case)]
fn joinOutput(mut stdoutLines: Vec<String>, stderrLines: Vec<String>) -> String {
    stdoutLines.extend(stderrLines);
    stdoutLines.join("\n")
}

#[allow(non_snake_case)]
fn appendScreenLines(session: &mut TerminalSession, output: &str) {
    for line in output.lines() {
        session.screenLines.push_back(line.to_string());
        while session.screenLines.len() > 200 {
            session.screenLines.pop_front();
        }
    }
}

#[allow(non_snake_case)]
fn applyTerminalInput(
    session: &mut TerminalSession,
    input: Option<&str>,
    control: Option<&str>,
) -> HostResult<usize> {
    let mut acceptedChars = 0;
    if let Some(input) = input {
        session.stdin.write_all(input.as_bytes())?;
        acceptedChars += input.chars().count();
    }
    if let Some(control) = control {
        let sequence = controlToSequence(control, input)?;
        session.stdin.write_all(sequence.as_bytes())?;
        acceptedChars += sequence.chars().count();
    }
    session.stdin.flush()?;
    Ok(acceptedChars)
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
fn nonBlank(value: &str, paramName: &str) -> HostResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HostError::new(format!("{paramName} parameter is required")));
    }
    Ok(trimmed.to_string())
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
            .createOrGetSession("linux_terminal_marker", "linux")
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
            .createOrGetSession("linux_terminal_screen", "linux")
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
            .createOrGetSession("linux_visible_ai", "linux")
            .expect("create visible terminal session");
        let manual = host
            .startPtySession(
                "linux_visible_manual",
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
        assert_eq!(createdEntry.terminalType, "linux");
        assert_eq!(manualEntry.sessionKind, "pty");
        assert_eq!(manualEntry.terminalType, "linux");
    }

    #[test]
    fn linux_pty_session_preserves_working_directory() {
        let host = LinuxTerminalHost::new();
        let session = host
            .createOrGetSession("linux_terminal_cwd", "linux")
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
