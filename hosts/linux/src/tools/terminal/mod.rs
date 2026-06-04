use std::collections::{BTreeMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, TerminalCloseOutput, TerminalCommandOutput,
    TerminalHost, TerminalInfo, TerminalInputOutput, TerminalScreenOutput,
    TerminalSessionInfo, TerminalSessionListEntry, TerminalTypeInfo,
};
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use uuid::Uuid;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
const PTY_OUTPUT_LIMIT: usize = 1024 * 1024;

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
    workingDir: String,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output: Arc<Mutex<VecDeque<u8>>>,
    exitCode: Option<i32>,
}

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
                description: "Linux sh terminal".to_string(),
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
        let ptySystem = native_pty_system();
        let pair = ptySystem
            .openpty(ptySize(rows, cols))
            .map_err(toHostError)?;
        let command = linuxPtyCommand(&workDir);
        let child = pair.slave.spawn_command(command).map_err(toHostError)?;
        let mut reader = pair.master.try_clone_reader().map_err(toHostError)?;
        let writer = pair.master.take_writer().map_err(toHostError)?;
        let output = Arc::new(Mutex::new(VecDeque::new()));
        let outputForThread = output.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => appendPtyOutput(&outputForThread, &buffer[..count]),
                    Err(_) => break,
                }
            }
        });
        let sessionId = nextSessionId();
        let mut state = self.lockState()?;
        state.ptySessions.insert(
            sessionId.clone(),
            PtySession {
                sessionName: normalizedSessionName,
                workingDir: workDir,
                child,
                master: pair.master,
                writer,
                output,
                exitCode: None,
            },
        );
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
        session.writer.write_all(data)?;
        session.writer.flush()?;
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
                terminalType: "pty".to_string(),
                sessionKind: "pty".to_string(),
                workingDir: session.workingDir.clone(),
                commandRunning: true,
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
        let mut state = self.lockState()?;
        if let Some(sessionId) = state.sessionNameToId.get(&sessionKey).cloned() {
            if state.sessions.contains_key(&sessionId) {
                return Ok(TerminalSessionInfo {
                    sessionId,
                    sessionName: normalizedSessionName,
                    terminalType: normalizedTerminalType,
                    isNewSession: false,
                });
            }
            state.sessionNameToId.remove(&sessionKey);
        }

        let session = createShellSession(
            normalizedSessionName.clone(),
            normalizedTerminalType.clone(),
        )?;
        let sessionId = session.id.clone();
        state.sessionNameToId.insert(sessionKey, sessionId.clone());
        state.sessions.insert(sessionId.clone(), session);
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
        let session = state.sessions.get_mut(&normalizedSessionId).ok_or_else(|| {
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
fn linuxPtyCommand(workingDir: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new("sh");
    command.cwd(workingDir);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("LANG", "C.UTF-8");
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
                if sawStart && line.starts_with(&endMarkerPrefix) {
                    session.commandRunning = false;
                    let exitCode = line[endMarkerPrefix.len()..]
                        .trim()
                        .parse::<i32>()
                        .unwrap_or(-1);
                    let output = joinOutput(outputLines, drainStderr(session)?);
                    appendScreenLines(session, &output);
                    return Ok(SessionCommandResult {
                        output,
                        exitCode,
                        timedOut: false,
                    });
                }
                if sawStart {
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
