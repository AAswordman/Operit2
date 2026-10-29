use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::ffi::CString;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
use uuid::Uuid;

use crate::runtime_common::requiredAndroidRuntimePath;

#[cfg(target_os = "android")]
use std::ptr;

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);
const PTY_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
const PTY_PROMPT_MARKER_PREFIX: &[u8] = b"\x1b]133;OperitPrompt=";
const PTY_PROMPT_MARKER_END: u8 = 7;
const COMMAND_CANCEL_SETTLE_TIMEOUT_MS: u64 = 3000;
const PLATFORM: &str = "android";
const PROOT_TERMINAL: &str = "proot";
const ANDROID_SYSTEM_TERMINAL: &str = "android-system";
const PRIMARY_TERMINAL_TYPE: &str = "bash";
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
    sessionNameToId: BTreeMap<String, String>,
    hiddenExecutorKeyToSessionId: BTreeMap<String, String>,
    ptySessions: BTreeMap<String, AndroidPtySession>,
}

struct AndroidPtySession {
    sessionName: String,
    terminalType: String,
    workingDir: String,
    pid: AndroidPtyProcess,
    masterFd: RawFd,
    writer: Arc<Mutex<RawFd>>,
    readerStop: Arc<AtomicBool>,
    output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: AndroidPtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    commandRunning: bool,
    exitCode: Option<i32>,
}

type AndroidPtyCommandOutput = Arc<(Mutex<VecDeque<u8>>, Condvar)>;

/// Keeps remote process ownership distinct from local child PIDs.
#[derive(Clone, Copy, Debug)]
enum AndroidPtyProcess {
    Local(AndroidPid),
    Shizuku(i32),
}

#[derive(Debug)]
struct AndroidPtyCursor {
    row: usize,
    col: usize,
}

type AndroidPtyCursorState = Arc<Mutex<AndroidPtyCursor>>;

impl Drop for AndroidPtySession {
    /// Stops the reader before releasing its separately owned write handle.
    fn drop(&mut self) {
        self.readerStop.store(true, Ordering::Release);
        #[cfg(target_os = "android")]
        unsafe {
            terminateAndroidPtyProcess(self.pid);
            let mut writerFd = self
                .writer
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let fd = *writerFd;
            *writerFd = -1;
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

    /// Starts an Android PTY session for one exact terminal implementation and type.
    pub fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let normalizedSessionName = nonBlank(sessionName, "session_name")?;
        let normalizedTerminalType = requireAndroidTerminal(terminal, terminalType)?;
        let workDir = nonBlank(workingDir, "working_directory")?;
        let session = createAndroidPtySession(
            normalizedSessionName,
            normalizedTerminalType,
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
        resizeAndroidPtyProcess(session.pid, session.masterFd, rows, cols)
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
        removeAndroidSessionMappings(&mut state, sessionId);
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
    /// Resolves proot guest paths separately from Android system-shell host paths.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        match requireAndroidTerminal(terminal, terminalType)?.as_str() {
            "bash" => operit_host_api::TerminalWorkingDirectory::resolveLinuxGuestDirectory(
                workingDir,
                resolveHostDirectory,
            ),
            "shell" => resolveHostDirectory(workingDir),
            value => Err(HostError::new(format!(
                "Unsupported Android terminal type: {value}"
            ))),
        }
    }

    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: PROOT_TERMINAL.to_string(),
            terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
            types: vec![
                TerminalTypeInfo {
                    terminal: PROOT_TERMINAL.to_string(),
                    terminalType: PRIMARY_TERMINAL_TYPE.to_string(),
                    available: true,
                    description: "Android proot Linux bash terminal".to_string(),
                },
                TerminalTypeInfo {
                    terminal: ANDROID_SYSTEM_TERMINAL.to_string(),
                    terminalType: "shell".to_string(),
                    available: true,
                    description: "Android system shell terminal".to_string(),
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
        AndroidTerminalHost::startPtySession(
            self,
            sessionName,
            terminal,
            terminalType,
            workingDir,
            rows,
            cols,
        )
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
                platform: PLATFORM.to_string(),
                terminal: androidTerminalName(&session.terminalType)?.to_string(),
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
        let normalizedTerminalType = nonBlank(terminalType, "type")?;
        let terminal = androidTerminalName(&normalizedTerminalType)?;
        let key = sessionKey(&normalizedTerminalType, &normalizedSessionName);
        {
            let mut state = self.lockState()?;
            if let Some(sessionId) = state.sessionNameToId.get(&key).cloned() {
                if state.ptySessions.contains_key(&sessionId) {
                    return Ok(TerminalSessionInfo {
                        sessionId,
                        sessionName: normalizedSessionName,
                        platform: PLATFORM.to_string(),
                        terminal: terminal.to_string(),
                        terminalType: normalizedTerminalType,
                        isNewSession: false,
                    });
                }
                state.sessionNameToId.remove(&key);
            }
        }

        let workingDir = initialAndroidWorkingDir(&normalizedTerminalType)?;
        let session = createAndroidPtySession(
            normalizedSessionName.clone(),
            normalizedTerminalType.clone(),
            workingDir,
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
            platform: PLATFORM.to_string(),
            terminal: terminal.to_string(),
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
        let (terminalType, commandOutput) = {
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
            clearAndroidPtyCommandOutput(&session.commandOutput)?;
            let commandInput = format!("{normalizedCommand}\r");
            writeAndroidPtyBytes(&session.writer, commandInput.as_bytes())?;
            session.commandRunning = true;
            (session.terminalType.clone(), session.commandOutput.clone())
        };
        let result = executeAndroidPtyCommandInSession(
            commandOutput,
            &normalizedCommand,
            timeoutMs,
            onOutput,
        );
        {
            let mut state = self.lockState()?;
            if let Some(session) = state.ptySessions.get_mut(&normalizedSessionId) {
                // Release the busy flag even if interrupting a broken PTY fails.
                // The state lock still excludes new commands during cancellation.
                session.commandRunning = false;
                if result.as_ref().map_or(true, |receipt| receipt.timedOut) {
                    if let Some(workingDir) = cancelTimedOutAndroidPtyCommand(session)? {
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
            terminal: androidTerminalName(&terminalType)?.to_string(),
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
        let normalizedExecutorKey = match executorKey.trim() {
            "" => "default".to_string(),
            value => value.to_string(),
        };
        let key = sessionKey(&normalizedTerminalType, &normalizedExecutorKey);
        let sessionId =
            hiddenAndroidPtySessionId(self, &key, &normalizedExecutorKey, &normalizedTerminalType)?;
        let output = self.executeInSession(&sessionId, &normalizedCommand, timeoutMs)?;
        Ok(HiddenTerminalCommandOutput {
            command: normalizedCommand,
            output: output.output,
            exitCode: output.exitCode,
            executorKey: normalizedExecutorKey,
            platform: PLATFORM.to_string(),
            terminal: PROOT_TERMINAL.to_string(),
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
            platform: PLATFORM.to_string(),
            terminal: androidTerminalName(&session.terminalType)?.to_string(),
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

fn hiddenAndroidPtySessionId(
    host: &AndroidTerminalHost,
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
            removeAndroidSessionMappings(&mut state, &sessionId);
        }
    }

    let session = createAndroidPtySession(
        format!("hidden:{executorLabel}"),
        terminalType.to_string(),
        initialAndroidWorkingDir(terminalType)?,
        24,
        80,
    )?;
    let sessionId = nextTerminalId();
    let mut state = host.lockState()?;
    state
        .hiddenExecutorKeyToSessionId
        .insert(executorKey.to_string(), sessionId.clone());
    state.ptySessions.insert(sessionId.clone(), session);
    Ok(sessionId)
}

fn createAndroidPtySession(
    sessionName: String,
    terminalType: String,
    workingDir: String,
    rows: u16,
    cols: u16,
) -> HostResult<AndroidPtySession> {
    androidLogInfo(&format!(
        "createAndroidPtySession start name={sessionName} type={terminalType} workingDir={workingDir} size={rows}x{cols}"
    ));
    let command = buildAndroidPtyCommand(&terminalType, &workingDir)?;
    let (pid, readerFd, masterFd) = startAndroidPtyProcess(&command, rows, cols)?;
    androidLogInfo(&format!(
        "createAndroidPtySession forked pid={pid:?} masterFd={masterFd} name={sessionName}"
    ));
    let writer = Arc::new(Mutex::new(masterFd));
    let readerStop = Arc::new(AtomicBool::new(false));
    let output = Arc::new(Mutex::new(VecDeque::new()));
    let commandOutput: AndroidPtyCommandOutput =
        Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
    let screenOutput = Arc::new(Mutex::new(VecDeque::new()));
    let cursorState: AndroidPtyCursorState =
        Arc::new(Mutex::new(AndroidPtyCursor { row: 1, col: 1 }));
    let session = AndroidPtySession {
        sessionName,
        terminalType,
        workingDir: command.workingDir.unwrap_or(workingDir),
        pid,
        masterFd,
        writer: writer.clone(),
        readerStop: readerStop.clone(),
        output: output.clone(),
        commandOutput: commandOutput.clone(),
        screenOutput: screenOutput.clone(),
        commandRunning: false,
        exitCode: None,
    };
    scheduleAndroidPtyReader(
        readerFd,
        readerStop,
        writer.clone(),
        output.clone(),
        commandOutput.clone(),
        screenOutput.clone(),
        cursorState,
    )?;
    if let Err(error) =
        waitForInitialAndroidPtyPrompt(commandOutput.clone(), Duration::from_millis(10000))
    {
        androidLogError(&format!(
            "createAndroidPtySession initial prompt failed pid={pid:?} masterFd={masterFd} name={}: {error}", session.sessionName
        ));
        return Err(error);
    }
    clearAndroidPtyCommandOutput(&commandOutput)?;
    androidLogInfo(&format!(
        "createAndroidPtySession ready pid={pid:?} masterFd={masterFd} name={}",
        session.sessionName
    ));
    Ok(session)
}

/// Gives the reader task exclusive descriptor ownership even if scheduling fails.
struct AndroidPtyReaderFd(RawFd);

impl Drop for AndroidPtyReaderFd {
    /// Releases the read handle only after the reader has stopped using it.
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        unsafe {
            libc::close(self.0);
        }
    }
}

/// Schedules the PTY reader on bounded Host service capacity.
fn scheduleAndroidPtyReader(
    masterFd: RawFd,
    stopped: Arc<AtomicBool>,
    writer: Arc<Mutex<RawFd>>,
    output: Arc<Mutex<VecDeque<u8>>>,
    commandOutput: AndroidPtyCommandOutput,
    screenOutput: Arc<Mutex<VecDeque<u8>>>,
    cursorState: AndroidPtyCursorState,
) -> HostResult<()> {
    androidLogInfo(&format!("Android PTY reader thread start fd={masterFd}"));
    let descriptor = AndroidPtyReaderFd(masterFd);
    operit_host_native_common::scheduleNativeServiceTask("android-pty-reader", move || {
        let descriptor = descriptor;
        while !stopped.load(Ordering::Acquire) {
            match readPtyFd(descriptor.0) {
                Ok(data) if data.is_empty() => thread::sleep(Duration::from_millis(20)),
                Ok(data) => {
                    processAndroidPtyTerminalQueries(&writer, &cursorState, &data);
                    let commandChunk = stripAndroidPtyDeviceStatusReports(&data);
                    let visibleChunk = stripAndroidPtyPromptMarkers(&commandChunk);
                    appendAndroidPtyOutput(&output, &visibleChunk);
                    appendAndroidPtyOutput(&screenOutput, &visibleChunk);
                    appendAndroidPtyCommandOutput(&commandOutput, &commandChunk);
                }
                Err(error) => {
                    androidLogError(&format!(
                        "Android PTY reader thread exit fd={masterFd}: {error}"
                    ));
                    break;
                }
            }
        }
    })
    .map(|_| ())
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
    androidLogInfo(&format!(
        "waitForInitialAndroidPtyPrompt start timeoutMs={}",
        timeout.as_millis()
    ));
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
            androidLogInfo(&format!(
                "waitForInitialAndroidPtyPrompt marker seen bytes={}",
                collected.len()
            ));
            promptSeenAt = Some(Instant::now());
        }
        if promptSeenAt.is_some_and(|seenAt| seenAt.elapsed() >= quietPeriod) {
            androidLogInfo(&format!(
                "waitForInitialAndroidPtyPrompt ready bytes={}",
                collected.len()
            ));
            return Ok(());
        }
        if Instant::now() >= deadline {
            androidLogError(&format!(
                "Timed out waiting for Android terminal prompt bytes={} output={}",
                collected.len(),
                androidPtyLogSnippet(&collected)
            ));
            return Err(androidPtyStartupError(&collected));
        }
    }
}

/// Preserves startup diagnostics in the error returned to terminal callers and tools.
fn androidPtyStartupError(data: &[u8]) -> HostError {
    let visibleBytes = stripAndroidPtyPromptMarkers(data);
    let visibleOutput = renderTerminalText(&String::from_utf8_lossy(&visibleBytes));
    HostError::new(format!(
        "Timed out waiting for Android terminal prompt\nStartup output:\n{}",
        visibleOutput.trim(),
    ))
}

/// Produces a compact terminal-output snippet for Android diagnostic logs.
fn androidPtyLogSnippet(data: &[u8]) -> String {
    const MAX_LOG_BYTES: usize = 4096;
    let start = data.len().saturating_sub(MAX_LOG_BYTES);
    String::from_utf8_lossy(&data[start..])
        .replace('\x1b', "\\x1b")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}

fn executeAndroidPtyCommandInSession(
    output: AndroidPtyCommandOutput,
    command: &str,
    timeoutMs: u64,
    onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
) -> HostResult<AndroidSessionCommandResult> {
    let deadline = Instant::now() + Duration::from_millis(timeoutMs);
    let mut collected = Vec::new();
    let mut promptSeenAt = None;
    let quietPeriod = Duration::from_millis(150);
    loop {
        let previousLength = collected.len();
        let drained = drainAndroidPtyCommandOutput(&output, &mut collected, deadline)?;
        onOutput(&collected[previousLength..])?;
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

fn cancelTimedOutAndroidPtyCommand(session: &mut AndroidPtySession) -> HostResult<Option<String>> {
    writeAndroidPtyBytes(&session.writer, b"\x03")?;

    let deadline = Instant::now() + Duration::from_millis(COMMAND_CANCEL_SETTLE_TIMEOUT_MS);
    let mut collected = Vec::new();
    while Instant::now() < deadline {
        let drained =
            drainAndroidPtyCommandOutput(&session.commandOutput, &mut collected, deadline)?;
        if drained == 0 {
            continue;
        }
        if let Some(marker) = findLastAndroidPtyPromptMarker(&collected)? {
            return Ok(Some(marker.workingDir));
        }
    }
    Ok(None)
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
    let fd = writer
        .lock()
        .map_err(|_| HostError::new("android pty writer mutex poisoned"))?;
    writePtyFd(*fd, data)
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

/// Normalizes Android terminal type names accepted by the host.
fn normalizeAndroidTerminalType(terminalType: &str) -> HostResult<String> {
    match terminalType.trim() {
        "bash" => Ok("bash".to_string()),
        "shell" => Ok("shell".to_string()),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for android host: {value}"
        ))),
    }
}

/// Validates one Android terminal implementation and shell-type pair.
fn requireAndroidTerminal(terminal: &str, terminalType: &str) -> HostResult<String> {
    let normalizedTerminalType = normalizeAndroidTerminalType(terminalType)?;
    match (terminal.trim(), normalizedTerminalType.as_str()) {
        (PROOT_TERMINAL, "bash") | (ANDROID_SYSTEM_TERMINAL, "shell") => Ok(normalizedTerminalType),
        (implementation, terminalType) => Err(HostError::new(format!(
            "Unsupported Android terminal implementation and type: {implementation}/{terminalType}"
        ))),
    }
}

/// Returns the Android terminal implementation registered for one shell semantic type.
fn androidTerminalName(terminalType: &str) -> HostResult<&'static str> {
    match terminalType {
        "bash" => Ok(PROOT_TERMINAL),
        "shell" => Ok(ANDROID_SYSTEM_TERMINAL),
        value => Err(HostError::new(format!(
            "Unsupported Android terminal type: {value}"
        ))),
    }
}

/// Returns the initial working directory for an Android terminal type.
fn initialAndroidWorkingDir(terminalType: &str) -> HostResult<String> {
    match terminalType {
        "bash" => Ok("/root".to_string()),
        "shell" => requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")
            .map(|path| path.to_string_lossy().to_string()),
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
fn androidLogInfo(message: &str) {
    androidLogWrite(4, message);
}

#[cfg(not(target_os = "android"))]
fn androidLogInfo(_message: &str) {}

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

fn removeAndroidSessionMappings(state: &mut AndroidTerminalState, sessionId: &str) {
    state.sessionNameToId.retain(|_, value| value != sessionId);
    state
        .hiddenExecutorKeyToSessionId
        .retain(|_, value| value != sessionId);
}

fn nextTerminalId() -> String {
    Uuid::new_v4().to_string()
}

struct AndroidPtyCommand {
    executable: CString,
    argv: Vec<CString>,
    envp: Vec<CString>,
    cwd: CString,
    shizuku: bool,
    workingDir: Option<String>,
}

/// Builds the PTY startup command for an Android terminal type.
fn buildAndroidPtyCommand(terminalType: &str, workingDir: &str) -> HostResult<AndroidPtyCommand> {
    match terminalType {
        "bash" => buildAndroidBashPtyCommand(workingDir),
        "shell" => buildAndroidShellPtyCommand(workingDir),
        value => Err(HostError::new(format!(
            "Unsupported terminal type for android host: {value}"
        ))),
    }
}

/// Builds the PTY startup command for Android proot Linux bash.
fn buildAndroidBashPtyCommand(workingDir: &str) -> HostResult<AndroidPtyCommand> {
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
    let commandText = "source $HOME/common.sh && start_shell";
    let argv = vec![cstringPath(&bash)?, cstring("-c")?, cstring(commandText)?];
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
    androidLogInfo(&format!(
        "buildAndroidPtyCommand type=bash workingDir={workDir} runtimeDir={} internalRoot={} tmpDir={} bash={} loader={} nativeLibraryDir={}",
        runtimeDir.to_string_lossy(),
        internalRoot.to_string_lossy(),
        tmpDir.to_string_lossy(),
        bash.to_string_lossy(),
        loader.to_string_lossy(),
        nativeLibraryDir.to_string_lossy()
    ));
    Ok(AndroidPtyCommand {
        executable: cstringPath(&bash)?,
        argv,
        envp,
        cwd: cstringPath(&internalRoot)?,
        shizuku: false,
        workingDir: None,
    })
}

/// Builds the PTY startup command for Android system shell.
fn buildAndroidShellPtyCommand(workingDir: &str) -> HostResult<AndroidPtyCommand> {
    let transport = crate::terminal_privilege::privileges()?.select()?;
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let internalRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_INTERNAL_ROOT")?;
    let tmpDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_TMP")?;
    let nativeLibraryDir = requiredAndroidRuntimePath("OPERIT_ANDROID_NATIVE_LIBRARY_DIR")?;

    std::fs::create_dir_all(&tmpDir)?;

    let systemPath = env::var("PATH")
        .map_err(|error| HostError::new(format!("Android terminal PATH is required: {error}")))?;
    buildAndroidSystemShellCommand(
        transport,
        workingDir,
        &runtimeDir,
        &internalRoot,
        &tmpDir,
        &nativeLibraryDir,
        &systemPath,
    )
}

/// Builds the selected persistent shell with literal argv and transport-specific paths.
fn buildAndroidSystemShellCommand(
    transport: crate::terminal_privilege::ShellTransport,
    workingDir: &str,
    runtimeDir: &Path,
    internalRoot: &Path,
    tmpDir: &Path,
    nativeLibraryDir: &Path,
    systemPath: &str,
) -> HostResult<AndroidPtyCommand> {
    use crate::terminal_privilege::ShellTransport;

    let mut workDir = nonBlank(workingDir, "working_directory")?;
    // The ADB shell UID cannot enter the app's private default directory.
    if transport == ShellTransport::Shizuku && Path::new(&workDir) == internalRoot {
        workDir = "/sdcard".to_string();
    }
    let ldLibraryPath = format!(
        "{}:{}",
        nativeLibraryDir.to_string_lossy(),
        runtimeDir.to_string_lossy()
    );
    let commandText = r#"while true; do __operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$(printf '%s' "$PWD" | base64 | tr -d '\n')" "$__operit_status"; IFS= read -r __operit_command || exit 0; eval "$__operit_command"; done"#;
    let shell = Path::new("/system/bin/sh");
    let environment = if transport == ShellTransport::Shizuku {
        vec![
            "PATH=/system/bin:/system/xbin".to_string(),
            "HOME=/sdcard".to_string(),
            "TMPDIR=/data/local/tmp".to_string(),
            "TERM=xterm-256color".to_string(),
            "LANG=en_US.UTF-8".to_string(),
        ]
    } else {
        vec![
            format!("PATH={}:{}", runtimeDir.to_string_lossy(), systemPath),
            format!("HOME={}", internalRoot.to_string_lossy()),
            format!("LD_LIBRARY_PATH={ldLibraryPath}"),
            format!("TMPDIR={}", tmpDir.to_string_lossy()),
            "TERM=xterm-256color".to_string(),
            "LANG=en_US.UTF-8".to_string(),
        ]
    };
    let exports = environment
        .iter()
        .map(|value| format!("export {}", quoteAndroidShellArgument(value)))
        .collect::<Vec<_>>()
        .join("; ");
    let guard = if matches!(transport, ShellTransport::Root(_)) {
        "[ \"$(/system/bin/id -u)\" = 0 ] || exit 126; "
    } else {
        ""
    };
    let script = format!(
        "{guard}{exports}; cd {} || exit; {commandText}",
        quoteAndroidShellArgument(&workDir)
    );
    let argv = match &transport {
        ShellTransport::Root(arguments) => {
            let mut argv = vec![
                cstringPath(shell)?,
                cstring("-c")?,
                cstring("exec \"$@\"")?,
                cstring("operit-root-shell")?,
            ];
            for argument in arguments {
                argv.push(cstring(argument)?);
            }
            argv.extend([cstring("-c")?, cstring(&script)?]);
            argv
        }
        _ => vec![cstringPath(shell)?, cstring("-c")?, cstring(&script)?],
    };
    let envp = environment
        .iter()
        .map(|value| cstring(value))
        .collect::<HostResult<Vec<_>>>()?;
    androidLogInfo(&format!(
        "buildAndroidPtyCommand type=shell transport={transport:?} workingDir={workDir} runtimeDir={} internalRoot={} tmpDir={} nativeLibraryDir={}",
        runtimeDir.to_string_lossy(),
        internalRoot.to_string_lossy(),
        tmpDir.to_string_lossy(),
        nativeLibraryDir.to_string_lossy()
    ));
    Ok(AndroidPtyCommand {
        executable: cstringPath(shell)?,
        argv,
        envp,
        cwd: if transport == ShellTransport::Shizuku {
            cstring("/")?
        } else {
            cstringPath(&internalRoot)?
        },
        shizuku: transport == ShellTransport::Shizuku,
        workingDir: Some(workDir),
    })
}

/// Quotes literal environment values and paths without interpreting shell expressions.
fn quoteAndroidShellArgument(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn androidPtyPromptCommand() -> String {
    r#"__operit_status=$?; printf '\033]133;OperitPrompt=%s:%s\007' "$(printf '%s' "$PWD" | base64 | tr -d '\n')" "$__operit_status""#.to_string()
}

fn androidTerminalDebugInfo(workingDir: &str) -> HostResult<BTreeMap<String, String>> {
    let runtimeDir = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_DIR")?;
    let rootfsDir = requiredAndroidRuntimePath("OPERIT_ANDROID_ROOTFS_DIR")?;
    let runtimeRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_RUNTIME_ROOT")?;
    let workspaceRoot = requiredAndroidRuntimePath("OPERIT_ANDROID_WORKSPACE_ROOT")?;
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
    insertPathDebug(&mut info, "runtimeRoot", &runtimeRoot);
    insertPathDebug(&mut info, "workspaceRoot", &workspaceRoot);
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
    info.insert("bashTerminalType".to_string(), "bash".to_string());
    info.insert("shellTerminalType".to_string(), "shell".to_string());
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

/// Starts a local child or a remote Shizuku PTY exactly once.
fn startAndroidPtyProcess(
    command: &AndroidPtyCommand,
    rows: u16,
    cols: u16,
) -> HostResult<(AndroidPtyProcess, RawFd, RawFd)> {
    if command.shizuku {
        let request = serde_json::json!({
            "operation": "start", "argv": command.argv.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>(),
            "env": command.envp.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>(),
            "cwd": command.cwd.to_string_lossy(), "rows": rows, "cols": cols,
        });
        let response = crate::terminal_privilege::call(request)?;
        let pid = requiredPtyInteger(&response, "pid")?;
        let reader = requiredPtyInteger(&response, "readFd")?;
        let writer = requiredPtyInteger(&response, "writeFd")?;
        return Ok((AndroidPtyProcess::Shizuku(pid), reader, writer));
    }
    let (pid, fd) = forkPtyExecve(command, rows, cols)?;
    #[cfg(target_os = "android")]
    {
        let writer = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
        if writer < 0 {
            terminateAndroidPtyProcess(AndroidPtyProcess::Local(pid));
            unsafe {
                libc::close(fd);
            }
            return Err(HostError::new("Could not duplicate Android PTY writer"));
        }
        Ok((AndroidPtyProcess::Local(pid), fd, writer))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (pid, fd);
        unreachable!("forkPtyExecve rejects non-Android hosts")
    }
}

/// Rejects malformed process handles before they reach POSIX or Binder operations.
fn requiredPtyInteger(value: &serde_json::Value, key: &str) -> HostResult<i32> {
    value
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .and_then(|v| i32::try_from(v).ok())
        .filter(|v| *v >= 0)
        .ok_or_else(|| HostError::new(format!("Invalid Android PTY {key}")))
}

/// Stops a session through the same security context that created it.
fn terminateAndroidPtyProcess(process: AndroidPtyProcess) {
    match process {
        AndroidPtyProcess::Shizuku(pid) => {
            if let Err(error) = crate::terminal_privilege::call(
                serde_json::json!({"operation": "close", "pid": pid}),
            ) {
                androidLogError(&format!("Shizuku terminal close failed: {error}"));
            }
        }
        AndroidPtyProcess::Local(pid) => {
            #[cfg(target_os = "android")]
            unsafe {
                let mut status = 0;
                // A reaped PID can already belong to an unrelated process.
                if libc::waitpid(pid, &mut status, libc::WNOHANG) != 0 {
                    return;
                }
                libc::kill(pid, libc::SIGHUP);
                libc::kill(pid, libc::SIGKILL);
                while libc::waitpid(pid, &mut status, 0) < 0
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR)
                {
                }
            }
            #[cfg(not(target_os = "android"))]
            let _ = pid;
        }
    }
}

/// Resizes the remote controlling PTY rather than assuming a local child.
fn resizeAndroidPtyProcess(
    process: AndroidPtyProcess,
    fd: RawFd,
    rows: u16,
    cols: u16,
) -> HostResult<()> {
    match process {
        AndroidPtyProcess::Local(_) => setPtyWindowSize(fd, rows, cols),
        AndroidPtyProcess::Shizuku(pid) => crate::terminal_privilege::call(
            serde_json::json!({"operation": "resize", "pid": pid, "rows": rows, "cols": cols}),
        )
        .map(|_| ()),
    }
}

/// Forks the PTY child in the current process's Android security context.
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
        if unsafe { libc::fcntl(masterFd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            terminateAndroidPtyProcess(AndroidPtyProcess::Local(pid));
            unsafe {
                libc::close(masterFd);
            }
            return Err(HostError::new("Could not mark Android PTY close-on-exec"));
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
        let mut descriptor = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut descriptor, 1, 20) };
        if ready < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                return Ok(Vec::new());
            }
            return Err(HostError::new("poll failed for Android PTY"));
        }
        if ready == 0 {
            return Ok(Vec::new());
        }
        let mut output = vec![0u8; 64 * 1024];
        let count = unsafe { libc::read(fd, output.as_mut_ptr().cast(), output.len()) };
        if count < 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                return Ok(Vec::new());
            }
            return Err(HostError::new("read failed for Android PTY"));
        }
        if count == 0 {
            return Err(HostError::new("Android PTY reached EOF"));
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

/// Polls locally owned children or the Shizuku-owned remote process.
fn pollPidExitCode(process: AndroidPtyProcess) -> HostResult<Option<i32>> {
    let pid = match process {
        AndroidPtyProcess::Shizuku(pid) => {
            let response = crate::terminal_privilege::call(
                serde_json::json!({"operation": "poll", "pid": pid}),
            )?;
            return match response.get("exitCode") {
                Some(serde_json::Value::Null) => Ok(None),
                Some(value) => value
                    .as_i64()
                    .and_then(|v| i32::try_from(v).ok())
                    .map(Some)
                    .ok_or_else(|| HostError::new("Invalid Shizuku terminal exit code")),
                None => Err(HostError::new("Missing Shizuku terminal exit code")),
            };
        }
        AndroidPtyProcess::Local(pid) => pid,
    };
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

/// Provides PTY primitives to the privileged Binder service without invoking the app runtime.
#[cfg(target_os = "android")]
pub fn androidPrivilegedPtyOperation(request: &str) -> HostResult<String> {
    use serde_json::{json, Value};
    let request: Value =
        serde_json::from_str(request).map_err(|e| HostError::new(e.to_string()))?;
    if request.get("operation").and_then(Value::as_str) != Some("start")
        && requiredPtyInteger(&request, "pid")? == 0
    {
        return Err(HostError::new("Android PTY PID must be positive"));
    }
    let response = match request.get("operation").and_then(Value::as_str) {
        Some("start") => {
            let strings = |key: &str| -> HostResult<Vec<CString>> {
                request
                    .get(key)
                    .and_then(Value::as_array)
                    .ok_or_else(|| HostError::new(format!("Missing PTY {key}")))?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| HostError::new("Invalid PTY argument"))
                            .and_then(cstring)
                    })
                    .collect()
            };
            let argv = strings("argv")?;
            let executable = argv
                .first()
                .ok_or_else(|| HostError::new("Empty PTY argv"))?
                .clone();
            let cwd = request
                .get("cwd")
                .and_then(Value::as_str)
                .ok_or_else(|| HostError::new("Missing PTY cwd"))?;
            let command = AndroidPtyCommand {
                executable,
                argv,
                envp: strings("env")?,
                cwd: cstring(cwd)?,
                shizuku: false,
                workingDir: None,
            };
            let rows = u16::try_from(requiredPtyInteger(&request, "rows")?)
                .map_err(|e| HostError::new(e.to_string()))?;
            let cols = u16::try_from(requiredPtyInteger(&request, "cols")?)
                .map_err(|e| HostError::new(e.to_string()))?;
            let (pid, fd) = forkPtyExecve(&command, rows, cols)?;
            json!({"pid": pid, "fd": fd})
        }
        Some("poll") => {
            json!({"exitCode": pollPidExitCode(AndroidPtyProcess::Local(requiredPtyInteger(&request, "pid")?))?})
        }
        Some("close") => {
            terminateAndroidPtyProcess(AndroidPtyProcess::Local(requiredPtyInteger(
                &request, "pid",
            )?));
            json!({})
        }
        Some("resize") => {
            let rows = u16::try_from(requiredPtyInteger(&request, "rows")?)
                .map_err(|e| HostError::new(e.to_string()))?;
            let cols = u16::try_from(requiredPtyInteger(&request, "cols")?)
                .map_err(|e| HostError::new(e.to_string()))?;
            setPtyWindowSize(requiredPtyInteger(&request, "fd")?, rows, cols)?;
            json!({})
        }
        _ => return Err(HostError::new("Unsupported privileged PTY operation")),
    };
    Ok(response.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds production shell argv without requiring an Android runtime or device.
    fn system_shell(
        transport: crate::terminal_privilege::ShellTransport,
        directory: &str,
    ) -> AndroidPtyCommand {
        buildAndroidSystemShellCommand(
            transport,
            directory,
            Path::new("/private/runtime"),
            Path::new("/private/app"),
            Path::new("/private/tmp"),
            Path::new("/private/lib"),
            "/system/bin",
        )
        .unwrap()
    }

    /// A Root session launches su once and verifies its identity before accepting commands.
    #[test]
    fn root_system_shell_uses_verified_su_and_persistent_prompt() {
        let command = system_shell(
            crate::terminal_privilege::ShellTransport::Root(vec![
                "/custom path/su".into(),
                "--flag".into(),
            ]),
            "/private/app",
        );
        let argv = command
            .argv
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            &argv[..7],
            &[
                "/system/bin/sh",
                "-c",
                "exec \"$@\"",
                "operit-root-shell",
                "/custom path/su",
                "--flag",
                "-c"
            ]
        );
        assert!(argv[7].starts_with("[ \"$(/system/bin/id -u)\" = 0 ] || exit 126;"));
        assert!(argv[7].contains("eval \"$__operit_command\""));
        assert!(argv[7].contains("OperitPrompt="));
        assert!(!command.shizuku);
    }

    /// Shizuku must not inherit private app paths that the ADB shell UID cannot access.
    #[test]
    fn shizuku_system_shell_has_accessible_environment_and_default_directory() {
        let command = system_shell(
            crate::terminal_privilege::ShellTransport::Shizuku,
            "/private/app",
        );
        assert!(command.shizuku);
        assert_eq!(command.cwd.to_str().unwrap(), "/");
        assert_eq!(command.workingDir.as_deref(), Some("/sdcard"));
        for value in command.envp {
            assert!(!value.to_str().unwrap().contains("/private/"));
        }
        let explicit = system_shell(
            crate::terminal_privilege::ShellTransport::Shizuku,
            "/sdcard/custom",
        );
        assert_eq!(explicit.workingDir.as_deref(), Some("/sdcard/custom"));
    }

    /// Literal paths cannot inject commands and the app shell keeps its original private directory.
    #[test]
    fn app_system_shell_preserves_directory_and_quotes_literals() {
        let directory = "/private/app/a'b; touch bad";
        let command = system_shell(crate::terminal_privilege::ShellTransport::App, directory);
        assert!(!command.shizuku);
        assert_eq!(command.workingDir.as_deref(), Some(directory));
        assert!(command.argv[2]
            .to_str()
            .unwrap()
            .contains("cd '/private/app/a'\\''b; touch bad' || exit;"));
        assert_eq!(command.argv[0].to_str().unwrap(), "/system/bin/sh");
        assert_eq!(quoteAndroidShellArgument("$(id)"), "'$(id)'");
    }

    /// Resolves proot guest paths without invoking the Android host VFS mapper.
    #[test]
    fn manual_proot_directory_uses_guest_namespace() {
        let host = AndroidTerminalHost::new();
        let resolveHost = |_: &str| Err(HostError::new("/mnt/linux is not mounted"));
        assert_eq!(
            host.resolveWorkingDirectory("proot", "bash", "/mnt/linux/root", &resolveHost)
                .unwrap(),
            "/root",
        );
    }

    /// Leaves Android system-shell mount errors intact instead of selecting proot.
    #[test]
    fn manual_system_shell_preserves_host_mount_failure() {
        let host = AndroidTerminalHost::new();
        let resolveHost = |_: &str| Err(HostError::new("/mnt/linux is not mounted"));
        assert_eq!(
            host.resolveWorkingDirectory(
                "android-system",
                "shell",
                "/mnt/linux/root",
                &resolveHost
            )
            .unwrap_err()
            .message,
            "/mnt/linux is not mounted",
        );
    }

    /// Retains the actual proot startup failure in the returned terminal error.
    #[test]
    fn startup_errors_include_process_output() {
        let error = androidPtyStartupError(b"proot: execve(/bin/bash): Permission denied\r\n");
        assert_eq!(
            error.message,
            "Timed out waiting for Android terminal prompt\nStartup output:\nproot: execve(/bin/bash): Permission denied",
        );
    }

    /// Ensures the native system shell and proot Bash have distinct implementation identities.
    #[test]
    fn interpreter_types_select_exact_android_implementations() {
        assert_eq!(
            androidTerminalName("shell").unwrap(),
            ANDROID_SYSTEM_TERMINAL
        );
        assert_eq!(androidTerminalName("bash").unwrap(), PROOT_TERMINAL);
        assert_ne!(sessionKey("shell", "chat"), sessionKey("bash", "chat"));
    }

    /// Rejects an unsupported explicit interpreter before creating any PTY or accessing runtime paths.
    #[test]
    fn typed_session_rejects_unsupported_interpreter() {
        let host = AndroidTerminalHost::new();
        let error = host
            .createOrGetTypedSession("chat", "powershell")
            .unwrap_err();
        assert_eq!(
            error.message,
            "Unsupported Android terminal type: powershell"
        );
        assert!(host.lockState().unwrap().ptySessions.is_empty());
    }

    /// Rejects an empty explicit interpreter instead of creating the default proot session.
    #[test]
    fn typed_session_rejects_empty_interpreter() {
        let host = AndroidTerminalHost::new();
        let error = host.createOrGetTypedSession("chat", " ").unwrap_err();
        assert_eq!(error.message, "type parameter is required");
        assert!(host.lockState().unwrap().ptySessions.is_empty());
    }
}
