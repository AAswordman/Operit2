use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::Arc;

use js_sys::Uint8Array;
use operit_host_api::{
    HiddenTerminalCommandOutput, HostError, HostResult, HostRuntimeTaskSchedulerHost,
    TerminalCloseOutput, TerminalCommandOutput, TerminalCompletionCallback, TerminalHost,
    TerminalInfo, TerminalInputOutput, TerminalOutputCallback, TerminalScreenOutput,
    TerminalSessionInfo, TerminalSessionListEntry, TerminalTypeInfo,
};
use wasm_bindgen::JsValue;

use crate::common::{
    bytes_to_js, call_terminal, js_i64, js_usize, read_bool_property, read_i32_property,
    read_string_property,
};

const PLATFORM: &str = "web-linux-vm";
const TERMINAL: &str = "v86";
const SHELL_TERMINAL_TYPE: &str = "shell";

#[derive(Clone)]
struct WebTerminalSession {
    sessionName: String,
    terminalType: String,
    workingDir: String,
    rows: u16,
    cols: u16,
}

thread_local! {
    static NEXT_TERMINAL_SESSION_ID: Cell<u64> = const { Cell::new(0) };
    static TERMINAL_SESSIONS: RefCell<BTreeMap<String, WebTerminalSession>> = const { RefCell::new(BTreeMap::new()) };
}

/// Hosts browser-local Linux VM terminal sessions.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebTerminalHost;

impl WebTerminalHost {
    /// Creates the browser-local Linux VM terminal host.
    pub fn new() -> Self {
        Self
    }
}

impl TerminalHost for WebTerminalHost {
    /// Starts isolated browser terminals at their only supported guest root.
    fn resolveWorkingDirectory(
        &self,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        resolveHostDirectory: &dyn Fn(&str) -> HostResult<String>,
    ) -> HostResult<String> {
        requireLinuxVmTerminalType(terminal, terminalType)?;
        operit_host_api::TerminalWorkingDirectory::resolveLinuxGuestDirectory(workingDir, &|path| {
            if path == "/" {
                return Ok("/".to_string());
            }
            let native = resolveHostDirectory(path)?;
            Ok(format!(
                "/mnt/operit-host/{}",
                native.trim_start_matches('/')
            ))
        })
    }

    /// Describes the Linux VM terminal available in browser builds.
    fn terminalInfo(&self) -> HostResult<TerminalInfo> {
        Ok(TerminalInfo {
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            types: vec![TerminalTypeInfo {
                terminal: TERMINAL.to_string(),
                terminalType: SHELL_TERMINAL_TYPE.to_string(),
                available: true,
                description: "Browser-local Buildroot VM with BusyBox, Node, and Python"
                    .to_string(),
            }],
        })
    }

    /// Starts one browser-local Linux virtual machine terminal session.
    fn startPtySession(
        &self,
        sessionName: &str,
        terminal: &str,
        terminalType: &str,
        workingDir: &str,
        rows: u16,
        cols: u16,
    ) -> HostResult<String> {
        let normalizedSessionName = requiredText(sessionName, "session_name")?;
        requireLinuxVmTerminalType(terminal, terminalType)?;
        let normalizedWorkingDir = requiredText(workingDir, "working_directory")?;
        if rows == 0 || cols == 0 {
            return Err(HostError::new("terminal dimensions must be positive"));
        }
        let sessionId = nextTerminalSessionId();
        call_terminal(
            "startPty",
            &[
                JsValue::from_str(&sessionId),
                JsValue::from_f64(rows as f64),
                JsValue::from_f64(cols as f64),
            ],
        )?;
        let session = WebTerminalSession {
            sessionName: normalizedSessionName,
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            workingDir: normalizedWorkingDir,
            rows,
            cols,
        };
        TERMINAL_SESSIONS.with(|sessions| {
            sessions.borrow_mut().insert(sessionId.clone(), session);
        });
        Ok(sessionId)
    }

    /// Drains raw virtual serial output emitted by one Linux VM session.
    fn readPtySession(&self, sessionId: &str) -> HostResult<Vec<u8>> {
        ensureTerminalSession(sessionId)?;
        let output = call_terminal("readPty", &[JsValue::from_str(sessionId)])?;
        Ok(Uint8Array::new(&output).to_vec())
    }

    /// Writes raw terminal input bytes to one Linux VM virtual serial console.
    fn writePtySession(&self, sessionId: &str, data: &[u8]) -> HostResult<usize> {
        ensureTerminalSession(sessionId)?;
        let accepted = call_terminal(
            "writePty",
            &[JsValue::from_str(sessionId), bytes_to_js(data)],
        )?;
        js_usize(accepted, "Linux VM terminal write")
    }

    /// Records the renderer dimensions for one Linux VM terminal session.
    fn resizePtySession(&self, sessionId: &str, rows: u16, cols: u16) -> HostResult<()> {
        if rows == 0 || cols == 0 {
            return Err(HostError::new("terminal dimensions must be positive"));
        }
        ensureTerminalSession(sessionId)?;
        call_terminal(
            "resizePty",
            &[
                JsValue::from_str(sessionId),
                JsValue::from_f64(rows as f64),
                JsValue::from_f64(cols as f64),
            ],
        )?;
        TERMINAL_SESSIONS.with(|sessions| {
            let mut sessions = sessions.borrow_mut();
            let session = terminalSession(&mut sessions, sessionId)?;
            session.rows = rows;
            session.cols = cols;
            Ok(())
        })
    }

    /// Returns the Linux VM exit code after the virtual machine has stopped.
    fn pollPtyExitCode(&self, sessionId: &str) -> HostResult<Option<i32>> {
        ensureTerminalSession(sessionId)?;
        let value = call_terminal("exitCode", &[JsValue::from_str(sessionId)])?;
        if value.is_null() || value.is_undefined() {
            return Ok(None);
        }
        i32::try_from(js_i64(value, "Linux VM terminal exit code")?)
            .map(Some)
            .map_err(|error| HostError::new(error.to_string()))
    }

    /// Stops one Linux VM terminal session and releases its browser resources.
    fn closePtySession(&self, sessionId: &str) -> HostResult<()> {
        ensureTerminalSession(sessionId)?;
        call_terminal("closePty", &[JsValue::from_str(sessionId)])?;
        removeTerminalSession(sessionId)
    }

    /// Lists Linux VM terminal sessions that have not exited.
    fn listSessions(&self) -> HostResult<Vec<TerminalSessionListEntry>> {
        let entries = TERMINAL_SESSIONS.with(|sessions| {
            sessions
                .borrow()
                .iter()
                .map(|(sessionId, session)| (sessionId.clone(), session.clone()))
                .collect::<Vec<_>>()
        });
        let mut active = Vec::new();
        for (sessionId, session) in entries {
            if self.pollPtyExitCode(&sessionId)?.is_none() {
                let state = call_terminal("sessionState", &[JsValue::from_str(&sessionId)])?;
                active.push(TerminalSessionListEntry {
                    sessionId,
                    sessionName: session.sessionName,
                    platform: PLATFORM.to_string(),
                    terminal: TERMINAL.to_string(),
                    terminalType: session.terminalType,
                    sessionKind: "linux-vm".to_string(),
                    workingDir: read_string_property(&state, "workingDir")?,
                    commandRunning: read_bool_property(&state, "commandRunning")?,
                });
            }
        }
        Ok(active)
    }

    /// Returns a named Linux VM terminal session or starts it at the guest root.
    fn createOrGetSession(&self, sessionName: &str) -> HostResult<TerminalSessionInfo> {
        self.createOrGetTypedSession(sessionName, SHELL_TERMINAL_TYPE)
    }

    /// Creates or reuses a browser VM session of exactly the requested interpreter type.
    fn createOrGetTypedSession(
        &self,
        sessionName: &str,
        terminalType: &str,
    ) -> HostResult<TerminalSessionInfo> {
        requireLinuxVmTerminalType(TERMINAL, terminalType)?;
        let normalizedSessionName = requiredText(sessionName, "session_name")?;
        let existing = TERMINAL_SESSIONS.with(|sessions| {
            sessions.borrow().iter().find_map(|(sessionId, session)| {
                (session.sessionName == normalizedSessionName).then(|| TerminalSessionInfo {
                    sessionId: sessionId.clone(),
                    sessionName: session.sessionName.clone(),
                    platform: PLATFORM.to_string(),
                    terminal: TERMINAL.to_string(),
                    terminalType: session.terminalType.clone(),
                    isNewSession: false,
                })
            })
        });
        if let Some(session) = existing {
            return Ok(session);
        }
        let sessionId = self.startPtySession(
            &normalizedSessionName,
            TERMINAL,
            SHELL_TERMINAL_TYPE,
            "/",
            24,
            80,
        )?;
        Ok(TerminalSessionInfo {
            sessionId,
            sessionName: normalizedSessionName,
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            isNewSession: true,
        })
    }

    /// Executes one shell command through the existing browser-local Linux VM terminal.
    fn executeInSession(
        &self,
        sessionId: &str,
        command: &str,
        timeoutMs: u64,
    ) -> HostResult<TerminalCommandOutput> {
        ensureTerminalSession(sessionId)?;
        let result = call_terminal(
            "executePtyCommand",
            &[
                JsValue::from_str(sessionId),
                JsValue::from_str(command),
                JsValue::from_f64(timeoutMs as f64),
            ],
        )?;
        let output = read_string_property(&result, "output")?;
        let exitCode = read_i32_property(&result, "exitCode")?;
        let timedOut = read_bool_property(&result, "timedOut")?;
        let workingDir = read_string_property(&result, "workingDir")?;
        if !timedOut {
            TERMINAL_SESSIONS.with(|sessions| -> HostResult<()> {
                let mut sessions = sessions.borrow_mut();
                let session = terminalSession(&mut sessions, sessionId)?;
                session.workingDir = workingDir;
                Ok(())
            })?;
        }
        Ok(TerminalCommandOutput {
            command: command.to_string(),
            output,
            exitCode,
            sessionId: sessionId.to_string(),
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: SHELL_TERMINAL_TYPE.to_string(),
            timedOut,
        })
    }

    /// Polls browser-owned execution while yielding the Rust worker between output frames.
    fn scheduleInSessionStreaming(
        self: Arc<Self>,
        scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
        sessionId: String,
        command: String,
        timeoutMs: u64,
        mut onOutput: TerminalOutputCallback,
        onComplete: TerminalCompletionCallback,
    ) -> HostResult<()> {
        if timeoutMs != 0 {
            return Err(HostError::new(
                "Browser scheduled commands require session-owned execution",
            ));
        }
        call_terminal(
            "startPtyCommand",
            &[JsValue::from_str(&sessionId), JsValue::from_str(&command)],
        )?;
        let delay = scheduler.clone();
        scheduler.scheduleHostRuntimeAsyncTask(
            "browser-terminal-command",
            Box::new(move || {
                Box::pin(async move {
                    let result = async {
                        loop {
                            let receipt = pollBrowserCommand(&sessionId, &command, &mut onOutput)?;
                            if let Some(receipt) = receipt {
                                return Ok(receipt);
                            }
                            delay.waitForHostRuntimeDelay(50).await?;
                        }
                    }
                    .await;
                    onComplete(result);
                })
            }),
        )
    }

    /// Rejects hidden command execution because the Linux guest exposes interactive sessions only.
    fn executeHiddenCommand(
        &self,
        _command: &str,
        _executorKey: &str,
        _timeoutMs: u64,
    ) -> HostResult<HiddenTerminalCommandOutput> {
        Err(HostError::new(
            "browser-local Linux VM terminals do not expose hidden command execution",
        ))
    }

    /// Sends textual or control input through one Linux VM terminal session.
    fn inputInSession(
        &self,
        sessionId: &str,
        input: Option<&str>,
        control: Option<&str>,
    ) -> HostResult<TerminalInputOutput> {
        let content = input.or(control).unwrap_or_default();
        let acceptedChars = self.writePtySession(sessionId, content.as_bytes())?;
        Ok(TerminalInputOutput {
            sessionId: sessionId.to_string(),
            acceptedChars,
        })
    }

    /// Stops one Linux VM terminal session and returns its close result.
    fn closeSession(&self, sessionId: &str) -> HostResult<TerminalCloseOutput> {
        self.closePtySession(sessionId)?;
        Ok(TerminalCloseOutput {
            sessionId: sessionId.to_string(),
            success: true,
            message: "Linux VM terminal session closed".to_string(),
        })
    }

    /// Renders the retained serial screen independently of the Flutter output consumer.
    fn getSessionScreen(&self, sessionId: &str) -> HostResult<TerminalScreenOutput> {
        let session = TERMINAL_SESSIONS
            .with(|sessions| sessions.borrow().get(sessionId).cloned())
            .ok_or_else(|| HostError::new("Terminal session does not exist"))?;
        let state = call_terminal("sessionState", &[JsValue::from_str(sessionId)])?;
        let mut screen = vt100::Parser::new(session.rows, session.cols, 0);
        screen.process(read_string_property(&state, "content")?.as_bytes());
        Ok(TerminalScreenOutput {
            sessionId: sessionId.to_string(),
            platform: PLATFORM.to_string(),
            terminal: TERMINAL.to_string(),
            terminalType: session.terminalType,
            rows: session.rows as usize,
            cols: session.cols as usize,
            content: screen.screen().contents(),
            commandRunning: read_bool_property(&state, "commandRunning")?,
        })
    }
}

/// Decodes a synchronous browser poll before the async task yields to its Host scheduler.
fn pollBrowserCommand(
    sessionId: &str,
    command: &str,
    onOutput: &mut TerminalOutputCallback,
) -> HostResult<Option<TerminalCommandOutput>> {
    let state = call_terminal("pollPtyCommand", &[JsValue::from_str(sessionId)])?;
    let error = js_sys::Reflect::get(&state, &JsValue::from_str("error"))
        .map_err(crate::common::js_error)?;
    if let Some(error) = error.as_string() {
        return Err(HostError::new(error));
    }
    let bytes = js_sys::Reflect::get(&state, &JsValue::from_str("output"))
        .map_err(crate::common::js_error)?;
    onOutput(&Uint8Array::new(&bytes).to_vec(), false)?;
    let result = js_sys::Reflect::get(&state, &JsValue::from_str("result"))
        .map_err(crate::common::js_error)?;
    if result.is_null() {
        return Ok(None);
    }
    onOutput(&[], true)?;
    Ok(Some(TerminalCommandOutput {
        command: command.to_string(),
        output: read_string_property(&result, "output")?,
        exitCode: read_i32_property(&result, "exitCode")?,
        timedOut: false,
        sessionId: sessionId.to_string(),
        platform: PLATFORM.to_string(),
        terminal: TERMINAL.to_string(),
        terminalType: SHELL_TERMINAL_TYPE.to_string(),
    }))
}

/// Allocates one unique browser-local Linux VM terminal session identifier.
fn nextTerminalSessionId() -> String {
    NEXT_TERMINAL_SESSION_ID.with(|next| {
        let value = next.get().saturating_add(1);
        next.set(value);
        format!("linux-vm-terminal-{value}")
    })
}

/// Returns one mutable terminal session entry or reports an unknown session identifier.
fn terminalSession<'a>(
    sessions: &'a mut BTreeMap<String, WebTerminalSession>,
    sessionId: &str,
) -> HostResult<&'a mut WebTerminalSession> {
    sessions.get_mut(sessionId).ok_or_else(|| {
        HostError::new(format!(
            "Linux VM terminal session does not exist: {sessionId}"
        ))
    })
}

/// Verifies that one browser-local Linux VM terminal session exists.
fn ensureTerminalSession(sessionId: &str) -> HostResult<()> {
    TERMINAL_SESSIONS.with(|sessions| {
        if sessions.borrow().contains_key(sessionId) {
            Ok(())
        } else {
            Err(HostError::new(format!(
                "Linux VM terminal session does not exist: {sessionId}"
            )))
        }
    })
}

/// Removes one Linux VM terminal session entry after its browser resources have stopped.
fn removeTerminalSession(sessionId: &str) -> HostResult<()> {
    TERMINAL_SESSIONS.with(|sessions| {
        sessions
            .borrow_mut()
            .remove(sessionId)
            .map(|_| ())
            .ok_or_else(|| {
                HostError::new(format!(
                    "Linux VM terminal session does not exist: {sessionId}"
                ))
            })
    })
}

/// Validates the only terminal type implemented by the browser-local Linux guest.
fn requireLinuxVmTerminalType(terminal: &str, terminalType: &str) -> HostResult<()> {
    if terminal.trim() == TERMINAL && terminalType.trim() == SHELL_TERMINAL_TYPE {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "browser-local Linux VM terminal implementation and type must be {TERMINAL}/{SHELL_TERMINAL_TYPE}: {terminal}/{terminalType}"
        )))
    }
}

/// Validates one required non-blank terminal value.
fn requiredText(value: &str, field: &str) -> HostResult<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        Err(HostError::new(format!(
            "terminal {field} must not be blank"
        )))
    } else {
        Ok(normalized.to_string())
    }
}
