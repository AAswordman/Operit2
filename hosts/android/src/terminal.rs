use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::ffi::CString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
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
use uuid::Uuid;

use crate::runtime_common::{buildAndroidProotCommand, requiredAndroidRuntimePath};

#[cfg(target_os = "android")]
use std::ptr;

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);
const PTY_OUTPUT_LIMIT: usize = 1024 * 1024;
const PTY_PROMPT_MARKER_PREFIX: &[u8] = b"\x1b]133;OperitPrompt=";
const PTY_PROMPT_MARKER_END: u8 = 7;
type RawFd = i32;
#[cfg(target_os = "android")]
type AndroidPid = libc::pid_t;
#[cfg(not(target_os = "android"))]
type AndroidPid = i32;

#[cfg(target_os = "android")]
#[link(name = "log")]
extern "C" {
    fn __android_log_write(
        priority: libc::c_int,
        tag: *const libc::c_char,
        text: *const libc::c_char,
    ) -> libc::c_int;
}

#[derive(Clone, Default)]
pub struct AndroidTerminalHost {
    state: Arc<Mutex<AndroidTerminalState>>,
}

#[derive(Default)]
struct AndroidTerminalState {
    sessions: BTreeMap<String, AndroidTerminalSession>,
    sessionNameToId: BTreeMap<String, String>,
    hiddenExecutorKeyToSessionId: BTreeMap<String, String>,
    ptySessions: BTreeMap<String, AndroidPtySession>,
}

struct AndroidTerminalSession {
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

struct AndroidPtySession {
    sessionName: String,
    terminalType: String,
    workingDir: String,
    pid: AndroidPid,
    masterFd: RawFd,
    writer: Arc<Mutex<RawFd>>,
    output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: AndroidPtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    commandRunning: bool,
    exitCode: Option<i32>,
}

type AndroidPtyCommandOutput = Arc<(Mutex<VecDeque<u8>>, Condvar)>;

#[derive(Debug)]
struct AndroidPtyCursor {
    row: usize,
    col: usize,
}

type AndroidPtyCursorState = Arc<Mutex<AndroidPtyCursor>>;

impl Drop for AndroidPtySession {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        unsafe {
            libc::kill(self.pid, libc::SIGHUP);
            libc::kill(self.pid, libc::SIGKILL);
            let fd = match self.writer.lock() {
                Ok(mut writerFd) => {
                    let value = *writerFd;
                    *writerFd = -1;
                    value
                }
                Err(_) => self.masterFd,
            };
            if fd >= 0 {
                libc::close(fd);
            }
        }
    }
}

impl AndroidTerminalHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn startPtySession(
        &self,
        sessionName: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        let workDir = nonBlank(workingDir, "working_directory")?;
        let session = createAndroidPtySession(
            normalizedSessionName,
            "android".to_string(),
            workDir,
            rows,
            cols,
        )?;
        let sessionId = nextTerminalId();
        let mut state = self.lockState()?;
        state.ptySessions.insert(sessionId.clone(), session);
        Ok(sessionId)
    }

    pub fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        let mut state = self.lockState()?;
        let session = state.ptySessions.get_mut(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        let mut output = session
            .output
            .lock()
            .map_err(|_| HostError::new("android pty output mutex poisoned"))?;
        Ok(output.drain(..).collect())
    }

    pub fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        let state = self.lockState()?;
        let session = state.ptySessions.get(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        writeAndroidPtyBytes(&session.writer, data)
    }

    pub fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        let state = self.lockState()?;
        let session = state.ptySessions.get(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        setPtyWindowSize(session.masterFd, rows, cols)
    }

    pub fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        let mut state = self.lockState()?;
        let session = state.ptySessions.get_mut(sessionId).ok_or_else(|| {
            HostError::new(format!("Android PTY session does not exist: {sessionId}"))
        })?;
        if let Some(exitCode) = session.exitCode {
            return Ok(Some(exitCode));
        }
        let exitCode = pollPidExitCode(session.pid)?;
        if let Some(code) = exitCode {
            session.exitCode = Some(code);
        }
        Ok(exitCode)
    }

    pub fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        let mut state = self.lockState()?;
        let removed = state.ptySessions.remove(sessionId);
        if removed.is_none() {
            androidLogError(&format!("closePtySession missing sessionId={sessionId}"));
            return Err(HostError::new(format!(
                "Android PTY session does not exist: {sessionId}"
            )));
        }
        Ok(())
    }

    pub fn terminalDebugInfo(&self, workingDir: &str) -> HostResult<BTreeMap<String, String>> {
        androidTerminalDebugInfo(workingDir)
    }

    fn lockState(&self) -> HostResult<std::sync::MutexGuard<'_, AndroidTerminalState>> {
        self.state
            .lock()
            .map_err(|_| HostError::new("android terminal state mutex poisoned"))
    }
}

impl TerminalHost for AndroidTerminalHost {
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: "android".to_string(),
            defaultType: "android".to_string(),
            types: vec![TerminalTypeInfo {
                terminalType: "android".to_string(),
                available: true,
                description: "Android proot terminal".to_string(),
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
        AndroidTerminalHost::startPtySession(self, sessionName, workingDir, rows, cols)
    }

    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        AndroidTerminalHost::readPtySession(self, sessionId)
    }

    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        AndroidTerminalHost::writePtySession(self, sessionId, data)
    }

    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        AndroidTerminalHost::resizePtySession(self, sessionId, rows, cols)
    }

    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        AndroidTerminalHost::pollPtyExitCode(self, sessionId)
    }

    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        AndroidTerminalHost::closePtySession(self, sessionId)
    }

    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let mut state = self.lockState()?;
        let mut entries = Vec::new();
        for (sessionId, session) in state.ptySessions.iter_mut() {
            if session.exitCode.is_none() {
                if let Some(exitCode) = pollPidExitCode(session.pid)? {
                    session.exitCode = Some(exitCode);
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
        let normalizedTerminalType = normalizeAndroidTerminalType(terminalType)?;
        let key = sessionKey(&normalizedTerminalType, &normalizedSessionName);
        {
            let mut state = self.lockState()?;
            if let Some(sessionId) = state.sessionNameToId.get(&key).cloned() {
                if state.ptySessions.contains_key(&sessionId) {
                    return Ok(TerminalSessionInfo {
                        sessionId,
                        sessionName: normalizedSessionName,
                        terminalType: normalizedTerminalType,
                        isNewSession: false,
                    });
                }
                state.sessionNameToId.remove(&key);
            }
        }

        let session = createAndroidPtySession(
            normalizedSessionName.clone(),
            normalizedTerminalType.clone(),
            "/root".to_string(),
            24,
            80,
        )?;
        let sessionId = nextTerminalId();
        let mut state = self.lockState()?;
        state.sessionNameToId.insert(key, sessionId.clone());
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
        let (terminalType, commandOutput, writer) = {
            let mut state = self.lockState()?;
            let session = state
                .ptySessions
                .get_mut(&normalizedSessionId)
                .ok_or_else(|| {
                    HostError::new(format!("Terminal session does not exist: {sessionId}"))
                })?;
            session.commandRunning = true;
            clearAndroidPtyCommandOutput(&session.commandOutput)?;
            (
                session.terminalType.clone(),
                session.commandOutput.clone(),
                session.writer.clone(),
            )
        };
        let commandInput = format!("{normalizedCommand}\r");
        writeAndroidPtyBytes(&writer, commandInput.as_bytes())?;
        let result =
            executeAndroidPtyCommandInSession(commandOutput, &normalizedCommand, timeoutMs)?;
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
        Ok(TerminalCommandOutput {
            command: normalizedCommand,
            output: result.output,
            exitCode: result.exitCode,
            sessionId: normalizedSessionId,
            terminalType,
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
        let normalizedTerminalType = normalizeAndroidTerminalType(terminalType)?;
        let normalizedExecutorKey = match executorKey.trim() {
            "" => "default".to_string(),
            value => value.to_string(),
        };
        let key = sessionKey(&normalizedTerminalType, &normalizedExecutorKey);
        let mut state = self.lockState()?;
        let sessionId = match state.hiddenExecutorKeyToSessionId.get(&key).cloned() {
            Some(sessionId) if state.sessions.contains_key(&sessionId) => sessionId,
            Some(sessionId) => {
                state.hiddenExecutorKeyToSessionId.remove(&key);
                let _ = sessionId;
                let session = createAndroidShellSession(
                    format!("hidden:{normalizedExecutorKey}"),
                    normalizedTerminalType.clone(),
                )?;
                let sessionId = session.id.clone();
                state
                    .hiddenExecutorKeyToSessionId
                    .insert(key.clone(), sessionId.clone());
                state.sessions.insert(sessionId.clone(), session);
                sessionId
            }
            None => {
                let session = createAndroidShellSession(
                    format!("hidden:{normalizedExecutorKey}"),
                    normalizedTerminalType.clone(),
                )?;
                let sessionId = session.id.clone();
                state
                    .hiddenExecutorKeyToSessionId
                    .insert(key, sessionId.clone());
                state.sessions.insert(sessionId.clone(), session);
                sessionId
            }
        };
        let session = state.sessions.get_mut(&sessionId).ok_or_else(|| {
            HostError::new(format!("Hidden terminal session missing: {sessionId}"))
        })?;
        let result = executeAndroidShellCommandInSession(session, &normalizedCommand, timeoutMs)?;
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
            .ptySessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let acceptedChars =
            applyAndroidPtyTerminalInput(session, input, control.and_then(normalizeControl))?;
        Ok(TerminalInputOutput {
            sessionId: normalizedSessionId,
            acceptedChars,
        })
    }

    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        let normalizedSessionId = nonBlank(sessionId, "session_id")?;
        let mut state = self.lockState()?;
        state
            .ptySessions
            .remove(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        state
            .sessionNameToId
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
        let session = state
            .ptySessions
            .get_mut(&normalizedSessionId)
            .ok_or_else(|| {
                HostError::new(format!("Terminal session does not exist: {sessionId}"))
            })?;
        let content = androidPtyScreenContent(session)?;
        let rows = content.lines().count();
        let cols = content
            .lines()
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

struct AndroidSessionCommandResult {
    output: String,
    exitCode: i32,
    timedOut: bool,
    workingDir: Option<String>,
}

fn createAndroidPtySession(
    sessionName: String,
    terminalType: String,
    workingDir: String,
    rows: u16,
    cols: u16,
) -> HostResult<AndroidPtySession> {
    let command = buildAndroidPtyCommand(&workingDir)?;
    let (pid, masterFd) = forkPtyExecve(&command, rows, cols)?;
    let writer = Arc::new(Mutex::new(masterFd));
    let output = Arc::new(Mutex::new(VecDeque::new()));
    let commandOutput: AndroidPtyCommandOutput =
        Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
    let screenOutput = Arc::new(Mutex::new(VecDeque::new()));
    let cursorState: AndroidPtyCursorState =
        Arc::new(Mutex::new(AndroidPtyCursor { row: 1, col: 1 }));
    spawnAndroidPtyReaderThread(
        masterFd,
        writer.clone(),
        output.clone(),
        commandOutput.clone(),
        screenOutput.clone(),
        cursorState,
    );
    if let Err(error) =
        waitForInitialAndroidPtyPrompt(commandOutput.clone(), Duration::from_millis(10000))
    {
        closeAndroidPtyProcess(pid, masterFd);
        return Err(error);
    }
    clearAndroidPtyCommandOutput(&commandOutput)?;
    Ok(AndroidPtySession {
        sessionName,
        terminalType,
        workingDir,
        pid,
        masterFd,
        writer,
        output,
        commandOutput,
        screenOutput,
        commandRunning: false,
        exitCode: None,
    })
}

fn spawnAndroidPtyReaderThread(
    masterFd: RawFd,
    writer: Arc<Mutex<RawFd>>,
    output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: AndroidPtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    cursorState: AndroidPtyCursorState,
) {
    thread::spawn(move || loop {
        match readPtyFd(masterFd) {
            Ok(data) if data.is_empty() => thread::sleep(Duration::from_millis(20)),
            Ok(data) => {
                processAndroidPtyTerminalQueries(&writer, &cursorState, &data);
                let commandChunk = stripAndroidPtyDeviceStatusReports(&data);
                let visibleChunk = stripAndroidPtyPromptMarkers(&commandChunk);
                appendAndroidPtyOutput(&output, &visibleChunk);
                appendAndroidPtyOutput(&screenOutput, &visibleChunk);
                appendAndroidPtyCommandOutput(&commandOutput, &commandChunk);
            }
            Err(_) => break,
        }
    });
}

fn appendAndroidPtyOutput(output: &Arc<Mutex<VecDeque<u8>>>, data: &[u8]) {
    if let Ok(mut buffer) = output.lock() {
        buffer.extend(data.iter().copied());
        while buffer.len() > PTY_OUTPUT_LIMIT {
            buffer.pop_front();
        }
    }
}

fn appendAndroidPtyCommandOutput(output: &AndroidPtyCommandOutput, data: &[u8]) {
    let (lock, condvar) = &**output;
    if let Ok(mut buffer) = lock.lock() {
        buffer.extend(data.iter().copied());
        while buffer.len() > PTY_OUTPUT_LIMIT {
            buffer.pop_front();
        }
        condvar.notify_all();
    }
}

fn processAndroidPtyTerminalQueries(
    writer: &Arc<Mutex<RawFd>>,
    cursorState: &AndroidPtyCursorState,
    data: &[u8],
) {
    let mut index = 0usize;
    while index < data.len() {
        if data[index..].starts_with(b"\x1b[6n") {
            if let Ok(cursor) = cursorState.lock() {
                let response = format!("\x1b[{};{}R", cursor.row, cursor.col);
                let _ = writeAndroidPtyBytes(writer, response.as_bytes());
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
        updateAndroidPtyCursor(cursorState, data[index]);
        index += 1;
    }
}

fn updateAndroidPtyCursor(cursorState: &AndroidPtyCursorState, value: u8) {
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

fn clearAndroidPtyCommandOutput(output: &AndroidPtyCommandOutput) -> HostResult<()> {
    let (lock, _) = &**output;
    let mut buffer = lock
        .lock()
        .map_err(|_| HostError::new("android pty command output mutex poisoned"))?;
    buffer.clear();
    Ok(())
}

fn waitForInitialAndroidPtyPrompt(
    output: AndroidPtyCommandOutput,
    timeout: Duration,
) -> HostResult<()> {
    let deadline = Instant::now() + timeout;
    let mut collected = Vec::new();
    let mut promptSeenAt = None;
    let quietPeriod = Duration::from_millis(150);
    loop {
        let drained = drainAndroidPtyCommandOutput(&output, &mut collected, deadline)?;
        if drained > 0 {
            promptSeenAt = None;
        }
        if findLastAndroidPtyPromptMarker(&collected)?.is_some() && promptSeenAt.is_none() {
            promptSeenAt = Some(Instant::now());
        }
        if promptSeenAt.is_some_and(|seenAt| seenAt.elapsed() >= quietPeriod) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(HostError::new(
                "Timed out waiting for Android terminal prompt",
            ));
        }
    }
}

fn executeAndroidPtyCommandInSession(
    output: AndroidPtyCommandOutput,
    command: &str,
    timeoutMs: u64,
) -> HostResult<AndroidSessionCommandResult> {
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    let mut collected = Vec::new();
    let mut promptSeenAt = None;
    let quietPeriod = Duration::from_millis(150);
    loop {
        let drained = drainAndroidPtyCommandOutput(&output, &mut collected, deadline)?;
        if drained > 0 {
            promptSeenAt = None;
        }
        if let Some(marker) = findLastAndroidPtyPromptMarker(&collected)? {
            if promptSeenAt.is_none() {
                promptSeenAt = Some(Instant::now());
            }
            if !promptSeenAt.is_some_and(|seenAt| seenAt.elapsed() >= quietPeriod) {
                if Instant::now() < deadline {
                    continue;
                }
            }
            let output = androidPtyCompletedCommandOutputText(&collected, command, marker.start);
            return Ok(AndroidSessionCommandResult {
                output,
                exitCode: marker.exitCode,
                timedOut: false,
                workingDir: Some(marker.workingDir),
            });
        }
        if Instant::now() >= deadline {
            let output = androidPtyTimedOutCommandOutputText(&collected, command);
            return Ok(AndroidSessionCommandResult {
                output,
                exitCode: -1,
                timedOut: true,
                workingDir: None,
            });
        }
    }
}

fn drainAndroidPtyCommandOutput(
    output: &AndroidPtyCommandOutput,
    collected: &mut Vec<u8>,
    deadline: Instant,
) -> HostResult<usize> {
    let (lock, condvar) = &**output;
    let mut buffer = lock
        .lock()
        .map_err(|_| HostError::new("android pty command output mutex poisoned"))?;
    if buffer.is_empty() {
        let now = Instant::now();
        if now < deadline {
            let wait = (deadline - now).min(Duration::from_millis(100));
            let result = condvar
                .wait_timeout(buffer, wait)
                .map_err(|_| HostError::new("android pty command output mutex poisoned"))?;
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
struct AndroidPtyPromptMarker {
    start: usize,
    workingDir: String,
    exitCode: i32,
}

fn findLastAndroidPtyPromptMarker(data: &[u8]) -> HostResult<Option<AndroidPtyPromptMarker>> {
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
    let payload = std::str::from_utf8(&data[payloadStart..payloadEnd]).map_err(|error| {
        HostError::new(format!("Invalid Android PTY prompt marker UTF-8: {error}"))
    })?;
    let (workingDirText, exitCodeText) = payload
        .split_once(':')
        .ok_or_else(|| HostError::new(format!("Invalid Android PTY prompt marker '{payload}'")))?;
    let workingDirBytes = BASE64_STANDARD
        .decode(workingDirText.as_bytes())
        .map_err(|error| {
            HostError::new(format!("Invalid Android PTY prompt cwd marker: {error}"))
        })?;
    let workingDir = String::from_utf8(workingDirBytes).map_err(|error| {
        HostError::new(format!("Invalid Android PTY prompt cwd UTF-8: {error}"))
    })?;
    let exitCode = parseAndroidExitCode(exitCodeText)?;
    Ok(Some(AndroidPtyPromptMarker {
        start,
        workingDir,
        exitCode,
    }))
}

fn rfindBytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

fn androidPtyCompletedCommandOutputText(raw: &[u8], command: &str, promptStart: usize) -> String {
    let visibleBytes = stripAndroidPtyPromptMarkers(&raw[..promptStart]);
    let text = String::from_utf8_lossy(&visibleBytes);
    let clean = renderTerminalText(&text);
    dropCommandEcho(clean, command)
}

fn androidPtyTimedOutCommandOutputText(raw: &[u8], command: &str) -> String {
    let visibleBytes = stripAndroidPtyPromptMarkers(raw);
    let text = String::from_utf8_lossy(&visibleBytes);
    let clean = renderTerminalText(&text);
    dropCommandEcho(clean, command)
}

fn androidPtyScreenContent(session: &AndroidPtySession) -> HostResult<String> {
    let buffer = session
        .screenOutput
        .lock()
        .map_err(|_| HostError::new("android pty screen output mutex poisoned"))?;
    let bytes = buffer.iter().copied().collect::<Vec<_>>();
    let visibleBytes = stripAndroidPtyPromptMarkers(&bytes);
    let text = String::from_utf8_lossy(&visibleBytes);
    Ok(renderTerminalText(&text))
}

fn stripAndroidPtyPromptMarkers(raw: &[u8]) -> Vec<u8> {
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

fn stripAndroidPtyDeviceStatusReports(raw: &[u8]) -> Vec<u8> {
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

fn renderTerminalText(value: &str) -> String {
    let mut renderer = TerminalTextRenderer::new();
    renderer.write(value);
    renderer.text()
}

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

fn csiValue(values: &[Option<usize>], index: usize, defaultValue: usize) -> usize {
    values
        .get(index)
        .and_then(|value| *value)
        .unwrap_or(defaultValue)
}

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

fn parseAndroidExitCode(exitCodeText: &str) -> HostResult<i32> {
    exitCodeText.trim().parse::<i32>().map_err(|error| {
        HostError::new(format!(
            "Invalid terminal exit code marker '{exitCodeText}': {error}"
        ))
    })
}

fn writeAndroidPtyBytes(writer: &Arc<Mutex<RawFd>>, data: &[u8]) -> HostResult<usize> {
    let fd = *writer
        .lock()
        .map_err(|_| HostError::new("android pty writer mutex poisoned"))?;
    writePtyFd(fd, data)
}

fn closeAndroidPtyProcess(pid: AndroidPid, fd: RawFd) {
    #[cfg(target_os = "android")]
    unsafe {
        libc::kill(pid, libc::SIGHUP);
        libc::kill(pid, libc::SIGKILL);
        if fd >= 0 {
            libc::close(fd);
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = (pid, fd);
    }
}

fn createAndroidShellSession(
    name: String,
    terminalType: String,
) -> HostResult<AndroidTerminalSession> {
    let mut command = buildAndroidProotCommand("/bin/bash", Some("/home/operit"))?;
    command.arg("-l");
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.spawn()?;
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
    Ok(AndroidTerminalSession {
        id: nextTerminalId(),
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

fn executeAndroidShellCommandInSession(
    session: &mut AndroidTerminalSession,
    command: &str,
    timeoutMs: u64,
) -> HostResult<AndroidSessionCommandResult> {
    let marker = format!(
        "__OPERIT_TERMINAL_{}__",
        NEXT_TERMINAL_ID.fetch_add(1, Ordering::SeqCst)
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
        let elapsed = match start.elapsed() {
            Ok(value) => value,
            Err(_) => deadline,
        };
        if elapsed >= deadline {
            session.commandRunning = false;
            let output = joinOutput(outputLines, drainAndroidStderr(session)?);
            appendAndroidScreenLines(session, &output);
            return Ok(AndroidSessionCommandResult {
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
                        let exitCode = exitCodeText.parse::<i32>().map_err(|error| {
                            HostError::new(format!(
                                "Invalid terminal exit code marker '{exitCodeText}': {error}"
                            ))
                        })?;
                        let output = joinOutput(outputLines, drainAndroidStderr(session)?);
                        appendAndroidScreenLines(session, &output);
                        return Ok(AndroidSessionCommandResult {
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

fn drainAndroidStderr(session: &AndroidTerminalSession) -> HostResult<Vec<String>> {
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

fn drainLiveAndroidShellOutputToScreen(session: &mut AndroidTerminalSession) -> HostResult<()> {
    while let Ok(line) = session.stdoutRx.try_recv() {
        appendAndroidScreenLines(session, &line);
    }
    let stderrLines = drainAndroidStderr(session)?;
    for line in stderrLines {
        appendAndroidScreenLines(session, &line);
    }
    Ok(())
}

fn joinOutput(mut stdoutLines: Vec<String>, stderrLines: Vec<String>) -> String {
    stdoutLines.extend(stderrLines);
    stdoutLines.join("\n")
}

fn appendAndroidScreenLines(session: &mut AndroidTerminalSession, output: &str) {
    for line in output.lines() {
        session.screenLines.push_back(line.to_string());
        while session.screenLines.len() > 200 {
            session.screenLines.pop_front();
        }
    }
}

fn applyTerminalInput(
    session: &mut AndroidTerminalSession,
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

fn applyAndroidPtyTerminalInput(
    session: &mut AndroidPtySession,
    input: Option<&str>,
    control: Option<&str>,
) -> HostResult<usize> {
    let mut acceptedChars = 0;
    if let Some(input) = input {
        writeAndroidPtyBytes(&session.writer, input.as_bytes())?;
        acceptedChars += input.chars().count();
    }
    if let Some(control) = control {
        let sequence = match control {
            "enter" => "\r".to_string(),
            value => controlToSequence(value, input)?,
        };
        writeAndroidPtyBytes(&session.writer, sequence.as_bytes())?;
        acceptedChars += sequence.chars().count();
    }
    Ok(acceptedChars)
}

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

fn normalizeAndroidTerminalType(terminalType: &str) -> HostResult<String> {
    match terminalType.trim() {
        "" | "android" => Ok("android".to_string()),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for android host: {value}"
        ))),
    }
}

fn nonBlank(value: &str, paramName: &str) -> HostResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(HostError::new(format!("{paramName} parameter is required")));
    }
    Ok(trimmed.to_string())
}

#[cfg(target_os = "android")]
fn androidLogError(message: &str) {
    androidLogWrite(6, message);
}

#[cfg(not(target_os = "android"))]
fn androidLogError(_message: &str) {}

#[cfg(target_os = "android")]
fn androidLogWrite(priority: libc::c_int, message: &str) {
    if let (Ok(tag), Ok(text)) = (CString::new("OperitTerminal"), CString::new(message)) {
        unsafe {
            let _ = __android_log_write(priority, tag.as_ptr(), text.as_ptr());
        }
    }
}

fn sessionKey(terminalType: &str, name: &str) -> String {
    format!("{terminalType}:{name}")
}

fn nextTerminalId() -> String {
    Uuid::new_v4().to_string()
}

struct AndroidPtyCommand {
    executable: CString,
    argv: Vec<CString>,
    envp: Vec<CString>,
    cwd: CString,
}

fn buildAndroidPtyCommand(workingDir: &str) -> HostResult<AndroidPtyCommand> {
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
    let tmpDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_TMP")?;
    let bash = requiredAndroidRuntimePath("OPERIT_ANDROID_BASH")?;
    let loader = requiredAndroidRuntimePath("OPERIT_ANDROID_LOADER")?;
    let nativeLibraryDir = requiredAndroidRuntimePath("OPERIT_ANDROID_NATIVE_LIBRARY_DIR")?;

    std::fs::create_dir_all(&tmpDir)?;

    let workDir = nonBlank(workingDir, "working_directory")?;
    let ldLibraryPath = format!(
        "{}:{}",
        nativeLibraryDir.to_string_lossy(),
        runtimeDir.to_string_lossy()
    );
    let systemPath = env::var("PATH")
        .map_err(|error| HostError::new(format!("Android terminal PATH is required: {error}")))?;
    let promptCommand = androidPtyPromptCommand();
    let argv = vec![
        cstringPath(&bash)?,
        cstring("-c")?,
        cstring("source $HOME/common.sh && start_shell")?,
    ];
    let envp = vec![
        cstring(&format!(
            "PATH={}:{}",
            runtimeDir.to_string_lossy(),
            systemPath
        ))?,
        cstring(&format!("HOME={}", internalRoot.to_string_lossy()))?,
        cstring(&format!("PREFIX={}", runtimeDir.to_string_lossy()))?,
        cstring(&format!("TERMUX_PREFIX={}", runtimeDir.to_string_lossy()))?,
        cstring(&format!("LD_LIBRARY_PATH={ldLibraryPath}"))?,
        cstring(&format!("PROOT_LOADER={}", loader.to_string_lossy()))?,
        cstring(&format!("TMPDIR={}", tmpDir.to_string_lossy()))?,
        cstring(&format!("PROOT_TMP_DIR={}", tmpDir.to_string_lossy()))?,
        cstring(&format!("OPERIT_WORKING_DIR={workDir}"))?,
        cstring("TERM=xterm-256color")?,
        cstring("LANG=en_US.UTF-8")?,
        cstring("PS1=$PWD $ ")?,
        cstring(&format!("PROMPT_COMMAND={promptCommand}"))?,
    ];
    Ok(AndroidPtyCommand {
        executable: cstringPath(&bash)?,
        argv,
        envp,
        cwd: cstringPath(&internalRoot)?,
    })
}

fn androidPtyPromptCommand() -> String {
    r#"__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$(printf '%s' "$PWD" | base64 | tr -d '\n')" "$__operit_status""#.to_string()
}

fn androidTerminalDebugInfo(workingDir: &str) -> HostResult<BTreeMap<String, String>> {
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let rootfsDir = requiredAndroidRuntimePath("OPERIT_ANDROID_ROOTFS_DIR")?;
    let storageRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_STORAGE_ROOT")?;
    let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
    let tmpDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_TMP")?;
    let proot = requiredAndroidRuntimePath("OPERIT_ANDROID_PROOT")?;
    let bash = requiredAndroidRuntimePath("OPERIT_ANDROID_BASH")?;
    let loader = requiredAndroidRuntimePath("OPERIT_ANDROID_LOADER")?;
    let busybox = requiredAndroidRuntimePath("OPERIT_ANDROID_BUSYBOX")?;
    let nativeLibraryDir = requiredAndroidRuntimePath("OPERIT_ANDROID_NATIVE_LIBRARY_DIR")?;
    let rootfsBash = rootfsDir.join("bin/bash");
    let workDir = nonBlank(workingDir, "working_directory")?;
    let mut info = BTreeMap::new();
    insertPathDebug(&mut info, "runtimeDir", &runtimeDir);
    insertPathDebug(&mut info, "rootfsDir", &rootfsDir);
    insertPathDebug(&mut info, "storageRoot", &storageRoot);
    insertPathDebug(&mut info, "internalRoot", &internalRoot);
    insertPathDebug(&mut info, "tmpDir", &tmpDir);
    insertPathDebug(&mut info, "proot", &proot);
    insertPathDebug(&mut info, "bash", &bash);
    insertPathDebug(&mut info, "loader", &loader);
    insertPathDebug(&mut info, "busybox", &busybox);
    insertPathDebug(&mut info, "nativeLibraryDir", &nativeLibraryDir);
    insertPathDebug(&mut info, "rootfsBash", &rootfsBash);
    info.insert("workingDirectory".to_string(), workDir.clone());
    info.insert("promptCommand".to_string(), androidPtyPromptCommand());
    info.insert(
        "argv".to_string(),
        [
            bash.to_string_lossy().to_string(),
            "-c".to_string(),
            "source $HOME/common.sh && start_shell".to_string(),
        ]
        .join(" "),
    );
    Ok(info)
}

fn insertPathDebug(info: &mut BTreeMap<String, String>, key: &str, path: &Path) {
    info.insert(key.to_string(), path.to_string_lossy().to_string());
    info.insert(format!("{key}.exists"), path.exists().to_string());
    info.insert(format!("{key}.isFile"), path.is_file().to_string());
    info.insert(format!("{key}.isDir"), path.is_dir().to_string());
}

fn cstring(value: &str) -> HostResult<CString> {
    CString::new(value).map_err(|error| HostError::new(error.to_string()))
}

fn cstringPath(path: &Path) -> HostResult<CString> {
    CString::new(path.to_string_lossy().as_bytes())
        .map_err(|error| HostError::new(error.to_string()))
}

fn forkPtyExecve(
    command: &AndroidPtyCommand,
    rows: u16,
    cols: u16,
) -> HostResult<(AndroidPid, RawFd)> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = (command, rows, cols);
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let mut masterFd: libc::c_int = -1;
        let mut termios = operitTermios();
        let mut winsize = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let pid =
            unsafe { libc::forkpty(&mut masterFd, ptr::null_mut(), &mut termios, &mut winsize) };
        if pid < 0 {
            androidLogError("forkpty failed");
            return Err(HostError::new("forkpty failed"));
        }
        if pid == 0 {
            unsafe {
                if libc::chdir(command.cwd.as_ptr()) != 0 {
                    libc::write(
                        libc::STDERR_FILENO,
                        b"chdir failed\n".as_ptr().cast(),
                        b"chdir failed\n".len(),
                    );
                    libc::_exit(1);
                }
                let mut argv = command
                    .argv
                    .iter()
                    .map(|item| item.as_ptr())
                    .collect::<Vec<_>>();
                argv.push(ptr::null());
                let mut envp = command
                    .envp
                    .iter()
                    .map(|item| item.as_ptr())
                    .collect::<Vec<_>>();
                envp.push(ptr::null());
                libc::execve(command.executable.as_ptr(), argv.as_ptr(), envp.as_ptr());
                libc::write(
                    libc::STDERR_FILENO,
                    b"execve failed\n".as_ptr().cast(),
                    b"execve failed\n".len(),
                );
                libc::_exit(1);
            }
        }
        Ok((pid, masterFd))
    }
}

#[cfg(target_os = "android")]
fn operitTermios() -> libc::termios {
    let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
    termios.c_iflag = libc::ICRNL | libc::IXON | libc::IXANY;
    termios.c_oflag = libc::OPOST | libc::ONLCR;
    termios.c_lflag = libc::ISIG
        | libc::ICANON
        | libc::ECHO
        | libc::ECHOE
        | libc::ECHOK
        | libc::ECHONL
        | libc::IEXTEN;
    termios.c_cflag = libc::CS8 | libc::CREAD;
    termios.c_cc[libc::VINTR] = b'C' - b'@';
    termios.c_cc[libc::VQUIT] = b'\\' - b'@';
    termios.c_cc[libc::VERASE] = 0x7f;
    termios.c_cc[libc::VKILL] = b'U' - b'@';
    termios.c_cc[libc::VEOF] = b'D' - b'@';
    termios.c_cc[libc::VSTOP] = b'S' - b'@';
    termios.c_cc[libc::VSUSP] = b'Z' - b'@';
    termios.c_cc[libc::VSTART] = b'Q' - b'@';
    termios.c_cc[libc::VMIN] = 1;
    termios.c_cc[libc::VTIME] = 0;
    termios
}

fn readPtyFd(fd: RawFd) -> HostResult<Vec<u8>> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = fd;
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let mut available: libc::c_int = 0;
        let ioctlResult = unsafe { libc::ioctl(fd, libc::FIONREAD, &mut available) };
        if ioctlResult != 0 {
            androidLogError(&format!("ioctl FIONREAD failed fd={fd}"));
            return Err(HostError::new("ioctl FIONREAD failed for Android PTY"));
        }
        if available <= 0 {
            return Ok(Vec::new());
        }
        let mut output = vec![0u8; available as usize];
        let count = unsafe { libc::read(fd, output.as_mut_ptr().cast(), output.len()) };
        if count < 0 {
            androidLogError(&format!("read failed fd={fd} available={available}"));
            return Err(HostError::new("read failed for Android PTY"));
        }
        output.truncate(count as usize);
        Ok(output)
    }
}

fn writePtyFd(fd: RawFd, data: &[u8]) -> HostResult<usize> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = (fd, data);
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let count = unsafe { libc::write(fd, data.as_ptr().cast(), data.len()) };
        if count < 0 {
            androidLogError(&format!("write failed fd={fd} bytes={}", data.len()));
            return Err(HostError::new("write failed for Android PTY"));
        }
        Ok(count as usize)
    }
}

fn setPtyWindowSize(fd: RawFd, rows: u16, cols: u16) -> HostResult<()> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = (fd, rows, cols);
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let mut winsize = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let result = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &mut winsize) };
        if result != 0 {
            androidLogError(&format!(
                "ioctl TIOCSWINSZ failed fd={fd} rows={rows} cols={cols}"
            ));
            return Err(HostError::new("ioctl TIOCSWINSZ failed for Android PTY"));
        }
        Ok(())
    }
}

fn pollPidExitCode(pid: AndroidPid) -> HostResult<Option<i32>> {
    #[cfg(not(target_os = "android"))]
    {
        let _ = pid;
        return Err(HostError::new(
            "Android PTY is only available on Android target",
        ));
    }

    #[cfg(target_os = "android")]
    {
        let mut status: libc::c_int = 0;
        let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if result == 0 {
            return Ok(None);
        }
        if result < 0 {
            androidLogError(&format!("waitpid failed pid={pid}"));
            return Err(HostError::new("waitpid failed for Android PTY"));
        }
        if libc::WIFEXITED(status) {
            return Ok(Some(libc::WEXITSTATUS(status)));
        }
        if libc::WIFSIGNALED(status) {
            return Ok(Some(-libc::WTERMSIG(status)));
        }
        Ok(Some(-1))
    }
}
